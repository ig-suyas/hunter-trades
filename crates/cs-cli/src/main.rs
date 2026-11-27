//! candlescope: command-line entry point.
//!
//! Advisory only: it never places orders and never needs broker or exchange keys. Network use is
//! limited to downloading public market data.

mod detect;

use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::Duration;

use cs_core::Timeframe;
use cs_core::time::parse_date;
use cs_data::fetch::{self, FetchOpts, SourceChoice};
use cs_data::record::{self, RecordOpts};
use cs_data::resample::{Policy, resample};
use cs_data::{DataDir, Market, clean, store, validate};

const USAGE: &str = "candlescope <command> [options]

Commands:
  fetch          download ~24 months of 1m candles (spot + USDS-M), funding and 5-min OI
                 --months N (24) | --start YYYY-MM-DD --end YYYY-MM-DD (end exclusive)
                 --symbols BTCUSDT,ETHUSDT  --markets spot,um  --no-funding  --no-metrics
                 --book-depth (also +-1..5% depth bands, ~0.7 GB)  --source auto|bybit
                 --gap-ms 60 (pause between requests)  --summary reports/fetch-summary.md
  validate       data-quality checks; writes --report reports/data-quality.md
  resample       write 5m/15m/1h/4h/1d candles to data/store/resampled  --policy strict|allow-gaps
  record         live recorder (runs until data/live/STOP exists)
                 --cap-gb 20  --depth-every 10  --bbo-every 1  --mark-every 5  --oi-every 30
                 --drop-after S (test reconnects)  --run-for S (stop after S seconds)
  detect         run the tier-1 pattern detectors over stored candles; writes events and
                 context features to data/events/<market>/<SYM>/<tf>/, prints counts per detector
                 and flags zero or absurd counts
                 --symbol BTCUSDT  --tf 15m  --market um|spot  --out-dir data/events
                 --report reports/detect-<SYM>-<tf>.md  --gallery-n 10  --seed 1
                 --bench-reps 3  --prefix-checks 3
                 (then: python scripts/gallery.py <gallery.json> reports/detector-gallery)
  record-status  print the recorder's status file
  record-stop    ask a running recorder to stop (creates data/live/STOP)

Common: --data-dir data (default ./data)";

struct Args(Vec<String>);

impl Args {
    fn get(&self, flag: &str) -> Option<&str> {
        self.0
            .iter()
            .position(|a| a == flag)
            .and_then(|i| self.0.get(i + 1))
            .map(String::as_str)
    }

    fn has(&self, flag: &str) -> bool {
        self.0.iter().any(|a| a == flag)
    }

    fn num<T: std::str::FromStr>(&self, flag: &str, default: T) -> Result<T, String> {
        match self.get(flag) {
            None => Ok(default),
            Some(v) => v.parse().map_err(|_| format!("{flag}: bad value {v:?}")),
        }
    }

    fn list(&self, flag: &str, default: &str) -> Vec<String> {
        self.get(flag)
            .unwrap_or(default)
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect()
    }

    fn symbols(&self) -> Vec<String> {
        self.list("--symbols", "BTCUSDT,ETHUSDT")
            .into_iter()
            .map(|s| s.to_uppercase())
            .collect()
    }

    fn markets(&self) -> Result<Vec<Market>, String> {
        self.list("--markets", "spot,um")
            .iter()
            .map(|m| Market::parse(m).ok_or_else(|| format!("unknown market {m}")))
            .collect()
    }

    fn data_dir(&self) -> DataDir {
        DataDir::new(self.get("--data-dir").unwrap_or("data"))
    }
}

fn out(s: &str) {
    let mut o = std::io::stdout().lock();
    let _ = writeln!(o, "{s}");
}

fn write_report(path: &str, body: &str) -> Result<(), String> {
    let p = PathBuf::from(path);
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    std::fs::write(&p, body).map_err(|e| format!("{path}: {e}"))
}

fn cmd_fetch(a: &Args) -> Result<(), String> {
    let (mut start, mut end) = FetchOpts::default_range(a.num("--months", 24)?);
    if let Some(s) = a.get("--start") {
        start = parse_date(s).ok_or("--start: expected YYYY-MM-DD")?;
    }
    if let Some(e) = a.get("--end") {
        end = parse_date(e).ok_or("--end: expected YYYY-MM-DD")?;
    }
    let source = match a.get("--source").unwrap_or("auto") {
        "auto" | "vision" => SourceChoice::Auto,
        "bybit" => SourceChoice::Bybit,
        s => return Err(format!("unknown source {s}")),
    };
    let o = FetchOpts {
        symbols: a.symbols(),
        markets: a.markets()?,
        start,
        end,
        funding: !a.has("--no-funding"),
        metrics: !a.has("--no-metrics"),
        book_depth: a.has("--book-depth"),
        source,
        min_gap: Duration::from_millis(a.num("--gap-ms", 60)?),
    };
    let dd = a.data_dir();
    let sum = fetch::run(&dd, &o).map_err(|e| e.to_string())?;
    let md = fetch::summary_md(&sum, &o);
    out(&md);
    let path = a.get("--summary").unwrap_or("reports/fetch-summary.md");
    write_report(
        path,
        &format!("# Fetch summary\n\nLast `candlescope fetch` run.\n\n{md}"),
    )?;
    if sum.failures.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "{} periods failed (see {path}); re-run to resume",
            sum.failures.len()
        ))
    }
}

fn cmd_validate(a: &Args) -> Result<(), String> {
    let dd = a.data_dir();
    let md = validate::run(&dd, &a.symbols(), &a.markets()?).map_err(|e| e.to_string())?;
    let path = a.get("--report").unwrap_or("reports/data-quality.md");
    write_report(path, &md)?;
    out(&md);
    out(&format!("wrote {path}"));
    Ok(())
}

fn cmd_resample(a: &Args) -> Result<(), String> {
    let dd = a.data_dir();
    let policy = match a.get("--policy").unwrap_or("strict") {
        "strict" => Policy::Strict,
        "allow-gaps" => Policy::AllowGaps,
        p => return Err(format!("unknown policy {p}")),
    };
    for sym in a.symbols() {
        for m in a.markets()? {
            let ck = clean::klines(&dd, m, &sym).map_err(|e| e.to_string())?;
            if ck.candles.is_empty() {
                continue;
            }
            for tf in &Timeframe::ALL[1..] {
                let (c, mins, st) = resample(&ck.candles, *tf, policy);
                let meta = vec![
                    ("source".to_string(), "resampled from clean 1m".to_string()),
                    ("timeframe".to_string(), tf.label().to_string()),
                    ("policy".to_string(), format!("{policy:?}")),
                ];
                let path = dd.resampled(m, &sym, tf.label());
                store::write_file(&path, &store::candles_to_table(&c, &mins, meta))
                    .map_err(|e| e.to_string())?;
                out(&format!(
                    "{} {sym} {}: {} candles, {} incomplete, {} edge-dropped -> {}",
                    m.dir(),
                    tf.label(),
                    st.emitted,
                    st.incomplete,
                    st.edge_dropped,
                    path.display()
                ));
            }
        }
    }
    Ok(())
}

fn cmd_record(a: &Args) -> Result<(), String> {
    let d = RecordOpts::default();
    let cap_gb: f64 = a.num("--cap-gb", 20.0)?;
    if cap_gb.is_nan() || cap_gb <= 0.0 {
        return Err("--cap-gb must be > 0".into());
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let cap_bytes = (cap_gb * 1024.0 * 1024.0 * 1024.0) as u64;
    let o = RecordOpts {
        symbols: a.symbols(),
        cap_bytes,
        bbo_every_s: a.num("--bbo-every", d.bbo_every_s)?,
        depth_every_s: a.num("--depth-every", d.depth_every_s)?,
        mark_every_s: a.num("--mark-every", d.mark_every_s)?,
        ticker_every_s: a.num("--ticker-every", d.ticker_every_s)?,
        oi_poll_s: a.num("--oi-every", d.oi_poll_s)?,
        drop_after_s: a
            .get("--drop-after")
            .map(str::parse)
            .transpose()
            .map_err(|_| "--drop-after")?,
        run_for_s: a
            .get("--run-for")
            .map(str::parse)
            .transpose()
            .map_err(|_| "--run-for")?,
    };
    record::run(&a.data_dir().live(), &o).map_err(|e| e.to_string())
}

#[allow(clippy::cast_precision_loss)] // event counts per 1000 candles, display only
fn cmd_detect(a: &Args) -> Result<(), String> {
    let symbol = a.get("--symbol").unwrap_or("BTCUSDT").to_uppercase();
    let tf = a.get("--tf").unwrap_or("15m").to_string();
    if Timeframe::parse(&tf).is_none() {
        return Err(format!("--tf: unknown time frame {tf}"));
    }
    let market = Market::parse(a.get("--market").unwrap_or("um")).ok_or("--market: spot|um")?;
    let default_report = format!("reports/detect-{symbol}-{tf}.md");
    let o = detect::DetectOpts {
        symbol,
        tf,
        market,
        out_dir: PathBuf::from(a.get("--out-dir").unwrap_or("data/events")),
        report: PathBuf::from(a.get("--report").unwrap_or(&default_report)),
        gallery_n: a.num("--gallery-n", 10)?,
        seed: a.num("--seed", 1)?,
        bench_reps: a.num("--bench-reps", 3)?,
        prefix_checks: a.num("--prefix-checks", 3)?,
    };
    let r = detect::run_detect(&a.data_dir(), &o)?;
    out(&format!(
        "{} candles, {} {} {}",
        r.candles,
        o.market.dir(),
        o.symbol,
        o.tf
    ));
    out(&format!(
        "{:<28} {:>8} {:>9}  flag",
        "detector", "events", "per 1k"
    ));
    for c in &r.counts {
        out(&format!(
            "{:<28} {:>8} {:>9.1}  {}",
            c.det,
            c.total,
            1000.0 * c.total as f64 / r.candles as f64,
            c.flag.as_deref().unwrap_or("")
        ));
    }
    let flagged: Vec<_> = r.counts.iter().filter(|c| c.flag.is_some()).collect();
    out(&format!(
        "flagged: {}",
        if flagged.is_empty() {
            "none".to_string()
        } else {
            flagged
                .iter()
                .map(|c| format!("{} ({})", c.det, c.flag.as_deref().unwrap_or("")))
                .collect::<Vec<_>>()
                .join("; ")
        }
    ));
    out(&format!(
        "throughput: best {:.0} candles/s, median {:.0} candles/s (all detectors + context)",
        r.best_cps, r.median_cps
    ));
    for (g, cps) in &r.group_cps {
        out(&format!("  {g:<18} alone: {cps:>10.0} candles/s"));
    }
    for (cut, ok) in &r.prefix_ok {
        out(&format!(
            "prefix check on real data, cut {cut}: {}",
            if *ok { "PASS" } else { "FAIL" }
        ));
    }
    out(&format!(
        "wrote {}, {}, {}, {}",
        r.events_path.display(),
        r.features_path.display(),
        r.gallery_json.display(),
        o.report.display()
    ));
    if r.prefix_ok.iter().any(|(_, ok)| !ok) {
        return Err("prefix invariance failed on real data".into());
    }
    Ok(())
}

fn main() -> ExitCode {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let Some(cmd) = argv.first().cloned() else {
        out(USAGE);
        return ExitCode::SUCCESS;
    };
    let a = Args(argv[1..].to_vec());
    let r = match cmd.as_str() {
        "fetch" => cmd_fetch(&a),
        "validate" => cmd_validate(&a),
        "resample" => cmd_resample(&a),
        "record" => cmd_record(&a),
        "detect" => cmd_detect(&a),
        "record-status" => record::status_text(&a.data_dir().live())
            .map(|s| out(&s))
            .map_err(|e| e.to_string()),
        "record-stop" => {
            let live = a.data_dir().live();
            std::fs::write(live.join("STOP"), b"stop\n")
                .map(|()| out("stop requested; the recorder exits within a few seconds"))
                .map_err(|e| e.to_string())
        }
        "help" | "--help" | "-h" => {
            out(USAGE);
            Ok(())
        }
        c => Err(format!("unknown command {c}\n\n{USAGE}")),
    };
    match r {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
