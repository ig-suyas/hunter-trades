//! `candlescope detect`: run the tier-1 detector set over stored candles, write events and
//! per-candle context features to disk, print counts per detector, flag zero or absurd counts,
//! measure throughput, and write the data for the detector gallery.

// Counts and indices shown as f64 rates are far below 2^52; random picks are bounded by lengths.
#![allow(clippy::cast_precision_loss, clippy::cast_possible_truncation)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use cs_core::time::fmt_minute;
use cs_core::{Candle, Event, Timeframe};
use cs_data::{DataDir, Market, store};
use cs_patterns::context::{ContextParams, Features, SESSIONS};
use cs_patterns::pullback::HEALTHY;
use cs_patterns::util::Rng;
use cs_patterns::{Runner, default_set, registry};

pub struct DetectOpts {
    pub symbol: String,
    pub tf: String,
    pub market: Market,
    pub out_dir: PathBuf,
    pub report: PathBuf,
    pub gallery_n: usize,
    pub seed: u64,
    pub bench_reps: usize,
    pub prefix_checks: usize,
}

pub struct Count {
    pub det: &'static str,
    pub total: usize,
    pub kinds: BTreeMap<&'static str, usize>,
    pub flag: Option<String>,
}

pub struct DetectResult {
    pub candles: usize,
    pub counts: Vec<Count>,
    pub events_path: PathBuf,
    pub features_path: PathBuf,
    pub gallery_json: PathBuf,
    pub best_cps: f64,
    pub median_cps: f64,
    pub prefix_ok: Vec<(usize, bool)>,
    pub group_cps: Vec<(&'static str, f64)>,
}

fn load(dd: &DataDir, o: &DetectOpts) -> Result<Vec<Candle>, String> {
    let path = dd.resampled(o.market, &o.symbol, &o.tf);
    let t = store::read_file(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let (c, _) = store::table_to_candles(&t).map_err(|e| e.to_string())?;
    Ok(c)
}

fn run(candles: &[Candle], base_ms: i64) -> (Vec<Event>, Vec<Features>, Vec<&'static str>) {
    let mut r = Runner::new(default_set(base_ms), Some(ContextParams::for_base(base_ms)));
    let names = r.names();
    let mut ev = Vec::with_capacity(candles.len() * 2);
    let mut fs = Vec::with_capacity(candles.len());
    for c in candles {
        if let Some(f) = r.step(c, &mut ev) {
            fs.push(f);
        }
    }
    (ev, fs, names)
}

/// Candles per second for the full detector set plus context features (events into a reused
/// buffer, no I/O).
fn bench(candles: &[Candle], base_ms: i64, reps: usize) -> (f64, f64) {
    let mut v = Vec::new();
    let mut buf = Vec::with_capacity(4096);
    for _ in 0..reps.max(1) {
        let mut r = Runner::new(default_set(base_ms), Some(ContextParams::for_base(base_ms)));
        let t = Instant::now();
        let mut sink = 0usize;
        for c in candles {
            buf.clear();
            if let Some(f) = r.step(c, &mut buf) {
                sink = sink.wrapping_add(usize::from(f.session));
            }
            sink = sink.wrapping_add(buf.len());
        }
        let s = t.elapsed().as_secs_f64();
        std::hint::black_box(sink);
        v.push(candles.len() as f64 / s.max(1e-9));
    }
    v.sort_by(f64::total_cmp);
    (v[v.len() - 1], v[v.len() / 2])
}

/// Candles per second for each registry group alone (one run each), plus context alone.
fn bench_groups(candles: &[Candle], base_ms: i64) -> Vec<(&'static str, f64)> {
    let mut v = Vec::new();
    let mut buf = Vec::with_capacity(4096);
    let time = |r: &mut Runner, buf: &mut Vec<Event>| {
        let t = Instant::now();
        for c in candles {
            buf.clear();
            std::hint::black_box(r.step(c, buf));
        }
        candles.len() as f64 / t.elapsed().as_secs_f64().max(1e-9)
    };
    for e in registry() {
        let mut r = Runner::new(vec![(e.make)(base_ms)], None);
        v.push((e.group, time(&mut r, &mut buf)));
    }
    let mut r = Runner::new(vec![], Some(ContextParams::for_base(base_ms)));
    v.push(("context features", time(&mut r, &mut buf)));
    v
}

fn num(x: f64) -> String {
    if x.is_finite() {
        format!("{x}")
    } else {
        String::new()
    }
}

fn jnum(x: f64) -> String {
    if x.is_finite() {
        format!("{x}")
    } else {
        "null".to_string()
    }
}

fn idx(x: u32) -> String {
    if x == u32::MAX {
        String::new()
    } else {
        x.to_string()
    }
}

fn write_events(path: &Path, ev: &[Event]) -> Result<(), String> {
    let mut s = String::with_capacity(ev.len() * 160);
    s.push_str(
        "det,kind,dir,formed_utc,known_utc,formed_at,known_at,start_idx,formed_idx,known_idx,\
         price,lo,hi,ax,ay,bx,by,tags,value\n",
    );
    for e in ev {
        let _ = writeln!(
            s,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            e.det,
            e.kind,
            e.dir,
            fmt_minute(e.formed_at),
            fmt_minute(e.known_at),
            e.formed_at,
            e.known_at,
            e.start_idx,
            e.formed_idx,
            e.known_idx,
            num(e.price),
            num(e.lo),
            num(e.hi),
            idx(e.ax),
            num(e.ay),
            idx(e.bx),
            num(e.by),
            e.tags,
            num(e.value)
        );
    }
    std::fs::write(path, s).map_err(|e| format!("{}: {e}", path.display()))
}

fn write_features(path: &Path, fs: &[Features]) -> Result<(), String> {
    let mut s = String::with_capacity(fs.len() * 100);
    s.push_str("idx,open_time,utc,atr,adx,regime,vol_pct,hour,session,zone_dist_atr,vol_z");
    // Higher-time-frame columns, prefixed by the HTF length (e.g. h1_, h4_). Empty until that
    // HTF has a completed candle; `known_utc` = open of the base candle whose close completed it.
    let prefixes: Vec<String> = fs.first().map_or_else(Vec::new, |f| {
        f.htf
            .iter()
            .map(|h| format!("h{}", h.htf_ms / 3_600_000))
            .collect()
    });
    for p in &prefixes {
        let _ = write!(
            s,
            ",{p}_candle_utc,{p}_known_utc,{p}_ema_ready,{p}_stack,{p}_slope_fast,{p}_slope_mid,\
             {p}_slope_slow,{p}_structure"
        );
    }
    s.push('\n');
    for f in fs {
        let _ = write!(
            s,
            "{},{},{},{},{},{},{},{},{},{},{}",
            f.idx,
            f.open_time,
            fmt_minute(f.open_time),
            num(f.atr),
            num(f.adx),
            f.regime,
            num(f.vol_pct),
            f.hour,
            SESSIONS[usize::from(f.session)],
            num(f.zone_dist_atr),
            num(f.vol_z)
        );
        for h in &f.htf {
            if h.n == 0 {
                s.push_str(",,,,,,,,");
            } else {
                let _ = write!(
                    s,
                    ",{},{},{},{},{},{},{},{}",
                    fmt_minute(h.candle_open),
                    fmt_minute(h.known_at),
                    u8::from(h.ema.ready),
                    h.ema.stack,
                    num(h.ema.slope_fast),
                    num(h.ema.slope_mid),
                    num(h.ema.slope_slow),
                    h.structure
                );
            }
        }
        s.push('\n');
    }
    std::fs::write(path, s).map_err(|e| format!("{}: {e}", path.display()))
}

/// Gallery input: up to `n` random events per detector with a candle window around each.
fn write_gallery(
    path: &Path,
    o: &DetectOpts,
    candles: &[Candle],
    ev: &[Event],
    names: &[&'static str],
) -> Result<(), String> {
    let mut by: BTreeMap<&str, Vec<usize>> = BTreeMap::new();
    for (k, e) in ev.iter().enumerate() {
        by.entry(e.det).or_default().push(k);
    }
    let mut rng = Rng::new(o.seed);
    let mut s = String::new();
    let _ = write!(
        s,
        "{{\"symbol\":\"{}\",\"tf\":\"{}\",\"market\":\"{}\",\"detectors\":[",
        o.symbol,
        o.tf,
        o.market.dir()
    );
    for (dn, det) in names.iter().enumerate() {
        if dn > 0 {
            s.push(',');
        }
        let ids = by.get(det).cloned().unwrap_or_default();
        let total = ids.len();
        // Partial Fisher-Yates: n distinct random events, then sorted by time.
        let mut ids = ids;
        let n = o.gallery_n.min(ids.len());
        for k in 0..n {
            let j = k + rng.below((ids.len() - k) as u64) as usize;
            ids.swap(k, j);
        }
        let mut pick: Vec<usize> = ids[..n].to_vec();
        pick.sort_unstable();
        let _ = write!(s, "{{\"det\":\"{det}\",\"total\":{total},\"events\":[");
        for (pn, &k) in pick.iter().enumerate() {
            if pn > 0 {
                s.push(',');
            }
            let e = &ev[k];
            let mut first = e.start_idx.min(e.formed_idx);
            if e.ax != u32::MAX {
                first = first.min(e.ax);
            }
            let known = e.known_idx as usize;
            let w0 = (first as usize)
                .saturating_sub(15)
                .max(known.saturating_sub(220));
            let w1 = (known + 12).min(candles.len() - 1);
            let _ = write!(
                s,
                "{{\"kind\":\"{}\",\"dir\":{},\"formed_idx\":{},\"known_idx\":{},\"start_idx\":{},\
                 \"formed_utc\":\"{}\",\"known_utc\":\"{}\",\"price\":{},\"lo\":{},\"hi\":{},\
                 \"ax\":{},\"ay\":{},\"bx\":{},\"by\":{},\"tags\":{},\"value\":{},\"w0\":{w0},\"candles\":[",
                e.kind,
                e.dir,
                e.formed_idx,
                e.known_idx,
                e.start_idx,
                fmt_minute(e.formed_at),
                fmt_minute(e.known_at),
                jnum(e.price),
                jnum(e.lo),
                jnum(e.hi),
                if e.ax == u32::MAX {
                    "null".to_string()
                } else {
                    e.ax.to_string()
                },
                jnum(e.ay),
                if e.bx == u32::MAX {
                    "null".to_string()
                } else {
                    e.bx.to_string()
                },
                jnum(e.by),
                e.tags,
                jnum(e.value)
            );
            for (cn, c) in candles[w0..=w1].iter().enumerate() {
                if cn > 0 {
                    s.push(',');
                }
                let _ = write!(
                    s,
                    "[{},{},{},{},{}]",
                    c.open, c.high, c.low, c.close, c.volume
                );
            }
            s.push_str("]}");
        }
        s.push_str("]}");
    }
    s.push_str("]}");
    std::fs::write(path, s).map_err(|e| format!("{}: {e}", path.display()))
}

/// Summary of the healthy-pullback flag and the higher-time-frame state.
fn followup_md(ev: &[Event], fs: &[Features], base_ms: i64) -> String {
    let mut md = String::new();
    let pb: Vec<&Event> = ev.iter().filter(|e| e.det == "pullback").collect();
    let healthy = |dir: i8| {
        let all = pb.iter().filter(|e| e.dir == dir).count();
        let h = pb
            .iter()
            .filter(|e| e.dir == dir && e.tags & HEALTHY != 0)
            .count();
        (h, all)
    };
    let (hl, al) = healthy(1);
    let (hs, as_) = healthy(-1);
    let _ = writeln!(
        md,
        "Healthy pullbacks (smaller and slower than the impulse, no close beyond the last swing): \
         long {hl} of {al}, short {hs} of {as_}.\n"
    );
    md.push_str(
        "| HTF | candles with state | EMA ready | bull stack | bear stack | structure up | \
         structure down | mean lag (base candles since the HTF candle completed) |\n\
         |---|---:|---:|---:|---:|---:|---:|---:|\n",
    );
    let n = fs.len().max(1) as f64;
    for slot in 0..2 {
        let Some(f0) = fs.first() else { break };
        let label = format!("{}h", f0.htf[slot].htf_ms / 3_600_000);
        let with: Vec<_> = fs.iter().map(|f| (f.open_time, f.htf[slot])).collect();
        let have = with.iter().filter(|(_, h)| h.n > 0).count();
        let ready = with.iter().filter(|(_, h)| h.ema.ready).count();
        let bull = with
            .iter()
            .filter(|(_, h)| h.ema.ready && h.ema.stack > 0)
            .count();
        let bear = with
            .iter()
            .filter(|(_, h)| h.ema.ready && h.ema.stack < 0)
            .count();
        let up = with.iter().filter(|(_, h)| h.structure > 0).count();
        let dn = with.iter().filter(|(_, h)| h.structure < 0).count();
        let lag: f64 = with
            .iter()
            .filter(|(_, h)| h.n > 0)
            .map(|(t, h)| (t - h.known_at) as f64 / base_ms as f64)
            .sum::<f64>()
            / have.max(1) as f64;
        let pct = |x: usize| 100.0 * x as f64 / n;
        let _ = writeln!(
            md,
            "| {label} | {have} | {:.1} % | {:.1} % | {:.1} % | {:.1} % | {:.1} % | {lag:.2} |",
            pct(ready),
            pct(bull),
            pct(bear),
            pct(up),
            pct(dn)
        );
    }
    md.push('\n');
    md
}

/// Zero / absurd count rules (per detector, on the whole history).
fn flag(total: usize, candles: usize) -> Option<String> {
    let rate = total as f64 / candles.max(1) as f64;
    if total == 0 {
        Some("ZERO: no events".into())
    } else if total < 20 {
        Some(format!("RARE: only {total} events"))
    } else if rate > 1.0 / 3.0 {
        Some(format!("ABSURD: {:.1} % of candles", rate * 100.0))
    } else {
        None
    }
}

#[allow(clippy::too_many_lines)] // linear: run, check, write files, build the report
pub fn run_detect(dd: &DataDir, o: &DetectOpts) -> Result<DetectResult, String> {
    let candles = load(dd, o)?;
    if candles.len() < 300 {
        return Err(format!("only {} candles", candles.len()));
    }
    let base_ms = Timeframe::parse(&o.tf)
        .ok_or_else(|| format!("unknown time frame {}", o.tf))?
        .ms();
    let (ev, fs, names) = run(&candles, base_ms);
    let (best_cps, median_cps) = bench(&candles, base_ms, o.bench_reps);
    let group_cps = bench_groups(&candles, base_ms);

    // Prefix invariance on the real data for the full set (the per-detector tests live in
    // crates/cs-patterns/tests/prefix.rs).
    let mut rng = Rng::new(o.seed ^ 0x5EED);
    let mut prefix_ok = Vec::new();
    for _ in 0..o.prefix_checks {
        let cut = 300 + rng.below((candles.len() - 300) as u64) as usize;
        let (pe, pf, _) = run(&candles[..cut], base_ms);
        let want: Vec<&Event> = ev.iter().filter(|e| (e.known_idx as usize) < cut).collect();
        let same_ev = pe.len() == want.len() && pe.iter().zip(&want).all(|(a, b)| a.same(b));
        let same_f = pf.iter().zip(&fs).all(|(a, b)| a.same(b));
        prefix_ok.push((cut, same_ev && same_f));
    }

    let dir = o.out_dir.join(o.market.dir()).join(&o.symbol).join(&o.tf);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let events_path = dir.join("events.csv");
    let features_path = dir.join("features.csv");
    let gallery_json = dir.join("gallery.json");
    write_events(&events_path, &ev)?;
    write_features(&features_path, &fs)?;
    write_gallery(&gallery_json, o, &candles, &ev, &names)?;

    let mut counts: Vec<Count> = names
        .iter()
        .map(|d| Count {
            det: d,
            total: 0,
            kinds: BTreeMap::new(),
            flag: None,
        })
        .collect();
    for e in &ev {
        if let Some(c) = counts.iter_mut().find(|c| c.det == e.det) {
            c.total += 1;
            *c.kinds.entry(e.kind).or_default() += 1;
        }
    }
    for c in &mut counts {
        c.flag = flag(c.total, candles.len());
    }
    let followup = followup_md(&ev, &fs, base_ms);
    let first = candles.first().map_or(0, |c| c.open_time);
    let last = candles.last().map_or(0, |c| c.open_time);
    let mut md = String::new();
    let _ = writeln!(
        md,
        "# Detector run: {} {} {}\n\nCandles: {} ({} to {} UTC, open times). Events: {}. \
         Detectors: {}.\n\nThroughput (full set + context features, {} runs): best {:.0}, median \
         {:.0} candles/s.\n",
        o.market.dir(),
        o.symbol,
        o.tf,
        candles.len(),
        fmt_minute(first),
        fmt_minute(last),
        ev.len(),
        counts.len(),
        o.bench_reps.max(1),
        best_cps,
        median_cps
    );
    if !prefix_ok.is_empty() {
        let _ = writeln!(
            md,
            "Prefix invariance on this data (full set + features): {}\n",
            prefix_ok
                .iter()
                .map(|(c, ok)| format!("cut {c}: {}", if *ok { "PASS" } else { "FAIL" }))
                .collect::<Vec<_>>()
                .join(", ")
        );
    }
    md.push_str(&followup);
    md.push_str("| group | candles/s alone |\n|---|---:|\n");
    for (g, cps) in &group_cps {
        let _ = writeln!(md, "| {g} | {cps:.0} |");
    }
    md.push('\n');
    md.push_str(
        "| detector | events | per 1000 candles | kinds | flag |\n|---|---:|---:|---|---|\n",
    );
    for c in &counts {
        let kinds = c
            .kinds
            .iter()
            .map(|(k, n)| format!("{k} {n}"))
            .collect::<Vec<_>>()
            .join(", ");
        let _ = writeln!(
            md,
            "| {} | {} | {:.1} | {} | {} |",
            c.det,
            c.total,
            1000.0 * c.total as f64 / candles.len() as f64,
            kinds,
            c.flag.as_deref().unwrap_or("")
        );
    }
    let _ = writeln!(
        md,
        "\nFlags: ZERO = no events; RARE = fewer than 20 events in the whole history; ABSURD = more \
         than one event per three candles.\n\nFiles: `{}`, `{}`, `{}`.",
        events_path.display(),
        features_path.display(),
        gallery_json.display()
    );
    if let Some(d) = o.report.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    std::fs::write(&o.report, &md).map_err(|e| format!("{}: {e}", o.report.display()))?;
    Ok(DetectResult {
        candles: candles.len(),
        counts,
        events_path,
        features_path,
        gallery_json,
        best_cps,
        median_cps,
        prefix_ok,
        group_cps,
    })
}
