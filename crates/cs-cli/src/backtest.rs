//! `candlescope backtest`: run trade rules over stored candles with the Phase 3 engine.
//!
//! * The last part of the history is a **sealed holdout** (default: the last 183 days of the
//!   setup candles). Runs stop before it unless `--open-holdout` is given (Phase 4, once).
//!   `--holdout-from YYYY-MM-DD` moves the boundary.
//! * `--rules random` (default) runs the random-entry baseline (a control, not a tactic).
//!   `--rules all` or `--rules <det>,<det>` runs the fixed rule templates per detector and
//!   writes trades and summaries; every rule run counts toward the multiple-testing budget.
//! * `--bench` times a full run (all history, all detectors, all templates, every cost
//!   multiplier) and prints only timings, counts and a combined results hash: no outcome
//!   numbers are printed or written, so it does not spend the multiple-testing budget.

// Counts shown as rates and timings are far below 2^52.
#![allow(clippy::cast_precision_loss, clippy::too_many_lines)]

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::time::Instant;

use cs_backtest::cost::{CostModel, Funding};
use cs_backtest::market::{
    DetectorSource, Market, RandomSource, SignalSource, audit_cuts, audit_prefix,
};
use cs_backtest::metrics::BootCfg;
use cs_backtest::output::{data_fingerprint, market_fingerprint, summary_json, trades_csv};
use cs_backtest::rule::{Entry, LimitAt, Overlap, Select, StopRule, Target, TradeRule, Trail};
use cs_backtest::sim::{Inputs, Sizing};
use cs_backtest::{Outcome, backtest};
use cs_core::time::{fmt_minute, parse_date};
use cs_core::{Candle, Timeframe};
use cs_data::{DataDir, Market as DataMarket, clean, store};
use sha2::{Digest, Sha256};

pub struct BacktestOpts {
    pub symbol: String,
    pub tf: Timeframe,
    pub market: DataMarket,
    pub rules: String,
    pub cost_mults: Vec<f64>,
    pub seed: u64,
    pub audit_cuts: usize,
    pub holdout_from: Option<i64>,
    pub open_holdout: bool,
    pub bench: bool,
    pub sizing: Sizing,
    pub out_dir: PathBuf,
    pub report: Option<PathBuf>,
}

/// The fixed rule templates per detector (chosen before any outcome was seen; not tuned).
pub fn templates(det: &str) -> Vec<TradeRule> {
    let sel = Select::det(det);
    let mut v = Vec::new();
    for (tag, x) in [("1r", 1.0), ("1.5r", 1.5), ("2r", 2.0), ("3r", 3.0)] {
        v.push(TradeRule::basic(
            &format!("{det}-{tag}"),
            sel.clone(),
            Target::R(x),
        ));
    }
    v.push(TradeRule::basic(
        &format!("{det}-lvl"),
        sel.clone(),
        Target::NextLevel { min_r: 1.0 },
    ));
    let mut l = TradeRule::basic(&format!("{det}-lim2r"), sel.clone(), Target::R(2.0));
    l.entry = Entry::Limit {
        at: LimitAt::ZoneNear,
        timeout_bars: 10,
    };
    v.push(l);
    let mut t = TradeRule::basic(&format!("{det}-trail"), sel, Target::None);
    t.trail = Some(Trail {
        activate_r: 1.0,
        atr_mult: 2.0,
    });
    t.time_stop_bars = Some(96);
    v.push(t);
    v
}

/// Random-entry baseline: random direction at random candles (p = 0.5), stop 1 ATR, target 1 R.
pub fn random_rule() -> TradeRule {
    TradeRule {
        name: "random-1r".into(),
        select: Select::det("random"),
        entry: Entry::NextOpen,
        stop: StopRule::Atr { mult: 1.0 },
        target: Target::R(1.0),
        time_stop_bars: None,
        trail: None,
        filters: Vec::new(),
        overlap: Overlap::OnePerSetup,
        min_stop_atr: 0.0,
    }
}

struct Data {
    bars: Vec<Candle>,
    m1: Vec<Candle>,
    funding: Vec<Funding>,
    t_load: [f64; 3],
}

fn load(dd: &DataDir, o: &BacktestOpts) -> Result<Data, String> {
    let t = Instant::now();
    let path = dd.resampled(o.market, &o.symbol, o.tf.label());
    let tb = store::read_file(&path).map_err(|e| format!("{}: {e}", path.display()))?;
    let (bars, _) = store::table_to_candles(&tb).map_err(|e| e.to_string())?;
    let t1 = t.elapsed().as_secs_f64();
    let t = Instant::now();
    let m1 = clean::klines(dd, o.market, &o.symbol)
        .map_err(|e| e.to_string())?
        .candles;
    let t2 = t.elapsed().as_secs_f64();
    let t = Instant::now();
    let mut funding = Vec::new();
    if o.market == DataMarket::Um {
        let (ft, _, _) = clean::series(&dd.funding(&o.symbol), None).map_err(|e| e.to_string())?;
        if ft.nrows() > 0 {
            let ts = ft.i64("time").map_err(|e| e.to_string())?;
            let rs = ft.f64("rate").map_err(|e| e.to_string())?;
            funding = ts
                .iter()
                .zip(rs)
                .map(|(&time, &rate)| Funding { time, rate })
                .collect();
        }
    }
    let t3 = t.elapsed().as_secs_f64();
    if bars.is_empty() {
        return Err(format!("no candles in {}", path.display()));
    }
    Ok(Data {
        bars,
        m1,
        funding,
        t_load: [t1, t2, t3],
    })
}

fn write(path: &Path, body: &str) -> Result<(), String> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    std::fs::write(path, body).map_err(|e| format!("{}: {e}", path.display()))
}

fn f(x: f64, d: usize) -> String {
    if x.is_finite() {
        format!("{x:.d$}")
    } else if x.is_nan() {
        "-".into()
    } else {
        "inf".into()
    }
}

pub fn run(dd: &DataDir, o: &BacktestOpts, out: &dyn Fn(&str)) -> Result<(), String> {
    let t_all = Instant::now();
    let data = load(dd, o)?;
    let tf_ms = o.tf.ms();
    let first = data.bars[0].open_time;
    let last_close = data.bars[data.bars.len() - 1].open_time + tf_ms;
    let holdout = o.holdout_from.unwrap_or_else(|| {
        let d = 86_400_000;
        (last_close - 183 * d).div_euclid(d) * d
    });
    let sealed = !(o.bench || o.open_holdout);
    let end = if sealed { holdout } else { i64::MAX };
    let bars: Vec<Candle> = data
        .bars
        .iter()
        .copied()
        .filter(|c| c.open_time + tf_ms <= end)
        .collect();
    let m1: Vec<Candle> = data
        .m1
        .iter()
        .copied()
        .filter(|c| c.open_time < end)
        .collect();
    let funding: Vec<Funding> = data
        .funding
        .iter()
        .copied()
        .filter(|x| x.time < end)
        .collect();
    out(&format!(
        "{} {} {}: {} candles {} .. {} (all data: {} .. {}), {} 1m candles, {} funding rows",
        o.market.dir(),
        o.symbol,
        o.tf.label(),
        bars.len(),
        fmt_minute(first),
        fmt_minute(bars.last().map_or(first, |c| c.open_time + tf_ms)),
        fmt_minute(first),
        fmt_minute(last_close),
        m1.len(),
        funding.len()
    ));
    out(&format!(
        "holdout from {}: {}",
        fmt_minute(holdout),
        if o.bench {
            "included for timing only (bench prints no outcomes)"
        } else if o.open_holdout {
            "OPENED (--open-holdout)"
        } else {
            "sealed (not loaded into the run)"
        }
    ));

    let random = o.rules == "random" && !o.bench;
    let src: Box<dyn SignalSource> = if random {
        Box::new(RandomSource {
            seed: o.seed,
            prob: 0.5,
        })
    } else {
        Box::new(DetectorSource::default_set())
    };
    let t = Instant::now();
    let market = Market::prepare(bars, tf_ms, src.as_ref()).map_err(|e| e.to_string())?;
    let t_prep = t.elapsed().as_secs_f64();
    let t = Instant::now();
    let cuts = audit_cuts(market.bars.len(), o.audit_cuts, o.seed);
    let audit = audit_prefix(&market, src.as_ref(), &cuts).map_err(|e| e.to_string())?;
    let t_audit = t.elapsed().as_secs_f64();
    out(&format!(
        "lookahead audit: PASS ({} cuts, {} events compared)",
        audit.cuts.len(),
        audit.events_compared
    ));
    let t = Instant::now();
    let mfp = market_fingerprint(&market);
    let dfp = data_fingerprint(&m1, &funding);
    let t_fp = t.elapsed().as_secs_f64();

    let rules: Vec<TradeRule> = if random {
        vec![random_rule()]
    } else {
        let dets: BTreeSet<&str> = market
            .events
            .iter()
            .filter(|e| e.dir != 0)
            .map(|e| e.det)
            .collect();
        let wanted: Vec<String> = if o.rules == "all" || o.bench {
            dets.iter().map(|s| (*s).to_string()).collect()
        } else {
            o.rules.split(',').map(|s| s.trim().to_string()).collect()
        };
        for w in &wanted {
            if !dets.contains(w.as_str()) {
                return Err(format!("no directional events for detector {w}"));
            }
        }
        wanted.iter().flat_map(|d| templates(d)).collect()
    };
    let base = if o.market == DataMarket::Um {
        CostModel::BINANCE_UM
    } else {
        CostModel::BINANCE_SPOT
    };
    let boot = BootCfg {
        seed: o.seed,
        ..BootCfg::default()
    };
    let t = Instant::now();
    let mut outcomes: Vec<Outcome> = Vec::with_capacity(rules.len() * o.cost_mults.len());
    for r in &rules {
        for &x in &o.cost_mults {
            let inp = Inputs {
                market: &market,
                m1: &m1,
                funding: &funding,
                costs: base.with_mult(x),
                sizing: o.sizing,
            };
            outcomes.push(backtest(&inp, r, boot, &mfp, &dfp).map_err(|e| e.to_string())?);
        }
    }
    let t_sim = t.elapsed().as_secs_f64();
    let trades: usize = outcomes.iter().map(|x| x.run.trades.len()).sum();
    let mut h = Sha256::new();
    for x in &outcomes {
        h.update(x.results_hash.as_bytes());
    }
    let combined = h.finalize().iter().fold(String::new(), |mut acc, b| {
        let _ = write!(acc, "{b:02x}");
        acc
    });

    let mut md = String::new();
    let _ = writeln!(
        md,
        "| step | seconds |\n|---|---|\n| load {} candles | {:.2} |\n| load 1m candles | {:.2} |\n| load funding | {:.2} |\n| prepare: detectors + context + levels | {:.2} |\n| lookahead audit ({} cuts) | {:.2} |\n| fingerprints | {:.2} |\n| {} rule runs ({} rules x {} cost levels), {} trades, metrics + bootstrap | {:.2} |\n| **total** | **{:.2}** |",
        o.tf.label(),
        data.t_load[0],
        data.t_load[1],
        data.t_load[2],
        t_prep,
        cuts.len(),
        t_audit,
        t_fp,
        outcomes.len(),
        rules.len(),
        o.cost_mults.len(),
        trades,
        t_sim,
        t_all.elapsed().as_secs_f64()
    );
    out(&md);
    out(&format!("combined results hash: {combined}"));
    if o.bench {
        out(
            "bench: outcome numbers are not printed or written (multiple-testing budget untouched)",
        );
        if let Some(p) = &o.report {
            let body = format!(
                "# Backtest benchmark: {} {} {}\n\nFull run over all stored history (holdout included for timing only; no outcome numbers were printed or written). Release build, this laptop, single thread.\n\nCandles: {} ({} .. {}), 1m candles: {}, funding rows: {}, events: {}.\n\n{md}\nCombined results hash: `{combined}`\n",
                o.market.dir(),
                o.symbol,
                o.tf.label(),
                market.bars.len(),
                fmt_minute(first),
                fmt_minute(last_close),
                m1.len(),
                funding.len(),
                market.events.len()
            );
            write(p, &body)?;
            out(&format!("wrote {}", p.display()));
        }
        return Ok(());
    }

    let dir = o
        .out_dir
        .join(o.market.dir())
        .join(&o.symbol)
        .join(o.tf.label());
    let mut table = String::from(
        "rule,cost_mult,signals,trades,win_rate,avg_win_r,avg_loss_r,expectancy_r,ci_lo,ci_hi,gross_r,cost_r,profit_factor,max_dd_r,max_dd_pct,longest_losing_streak,exposure,mean_hold_bars,results_hash\n",
    );
    for x in &outcomes {
        let s = &x.summary;
        let name = format!("{}-x{}", x.run.rule.name, x.run.costs.mult);
        let d = dir.join(&name);
        write(&d.join("trades.csv"), &trades_csv(&x.run.trades))?;
        let js = summary_json(&x.run, s, boot, &x.inputs_hash, &x.results_hash);
        write(
            &d.join("summary.json"),
            &serde_json::to_string_pretty(&js).map_err(|e| e.to_string())?,
        )?;
        let (lo, hi) = s.ci.map_or((f64::NAN, f64::NAN), |c| (c.lo, c.hi));
        let _ = writeln!(
            table,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            x.run.rule.name,
            x.run.costs.mult,
            x.run.signals,
            s.n,
            f(s.win_rate, 4),
            f(s.avg_win_r, 4),
            f(s.avg_loss_r, 4),
            f(s.expectancy_r, 4),
            f(lo, 4),
            f(hi, 4),
            f(s.gross_expectancy_r, 4),
            f(s.mean_cost_r, 4),
            f(s.profit_factor, 3),
            f(s.max_dd_r, 2),
            f(100.0 * s.max_dd_pct, 2),
            s.longest_losing_streak,
            f(s.exposure, 3),
            f(s.mean_hold_bars, 2),
            x.results_hash
        );
        if random {
            out(&format!(
                "{name}: {} trades, win rate {} %, avg win {} R, avg loss {} R, expectancy {} R \
                 (95% block-bootstrap CI {} .. {}), gross {} R, mean cost {} R (fee {}, spread {}, \
                 slippage {}, funding {}), PF {}, max DD {} R / {} %, longest losing streak {}, \
                 exposure {} %, mean hold {} candles, flags {:?}, results hash {}",
                s.n,
                f(100.0 * s.win_rate, 1),
                f(s.avg_win_r, 3),
                f(s.avg_loss_r, 3),
                f(s.expectancy_r, 4),
                f(lo, 4),
                f(hi, 4),
                f(s.gross_expectancy_r, 4),
                f(s.mean_cost_r, 4),
                f(s.mean_fee_r, 4),
                f(s.mean_spread_r, 4),
                f(s.mean_slip_r, 4),
                f(s.mean_funding_r, 5),
                f(s.profit_factor, 3),
                f(s.max_dd_r, 1),
                f(100.0 * s.max_dd_pct, 1),
                s.longest_losing_streak,
                f(100.0 * s.exposure, 1),
                f(s.mean_hold_bars, 2),
                s.flags,
                x.results_hash
            ));
        }
    }
    write(&dir.join("runs.csv"), &table)?;
    out(&format!(
        "wrote {} rule runs to {} (runs.csv + <rule>-x<mult>/trades.csv, summary.json)",
        outcomes.len(),
        dir.display()
    ));
    if let Some(p) = &o.report {
        let body = format!(
            "# Backtest: {} {} {} (rules: {})\n\nIn-sample candles {} .. {}; holdout from {} {}.\nCosts: {:?}; sizing: {:?}.\nLookahead audit: PASS ({} cuts).\n\n```\n{table}```\n\n{md}\nCombined results hash: `{combined}`\n",
            o.market.dir(),
            o.symbol,
            o.tf.label(),
            o.rules,
            fmt_minute(first),
            fmt_minute(market.close_at(market.bars.len() - 1)),
            fmt_minute(holdout),
            if sealed { "(sealed)" } else { "(opened)" },
            base,
            o.sizing,
            cuts.len()
        );
        write(p, &body)?;
        out(&format!("wrote {}", p.display()));
    }
    Ok(())
}

/// Parse `--holdout-from`.
pub fn parse_holdout(s: Option<&str>) -> Result<Option<i64>, String> {
    s.map(|x| parse_date(x).ok_or_else(|| "--holdout-from: expected YYYY-MM-DD".to_string()))
        .transpose()
}
