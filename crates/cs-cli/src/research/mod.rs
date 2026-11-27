//! `candlescope research`: the Phase 4 research protocol (`reports/protocol.md`).
//!
//! Stages, in order:
//! * `--seal`: write `reports/holdout-seal.json` (boundary, holdout data hashes, protocol hashes)
//!   before any result is computed; refuses to overwrite an existing seal.
//! * `--stage discovery`: every pre-registered trial (`space::enumerate`) on BTCUSDT DISCOVERY data,
//!   Benjamini-Hochberg FDR, parameter plateau, finalists frozen in `reports/research/finalists.json`.
//! * `--stage validate`: frozen finalists on VALIDATION data, ETH check, walk-forward, random
//!   baselines, regimes; decides which finalists may enter the holdout.
//! * `--final`: creates `reports/holdout.lock` (refuses if it exists), then opens the holdout once
//!   for the finalists that passed validation.
//!
//! No stage except `--final` can load candles at or after the seal boundary ([`Access`]).
//! The engine (`cs-backtest`) is used as is.

// Counts shown as rates are far below 2^52; candle indices fit u32 (event contract).
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap,
    clippy::too_many_lines,
    clippy::similar_names
)]

pub mod discovery;
pub mod holdout;
pub mod space;
pub mod validate;

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use cs_backtest::cost::{CostModel, Funding};
use cs_backtest::market::{
    DetectorSource, Market, RandomSource, SignalSource, audit_cuts, audit_prefix,
};
use cs_backtest::rule::TradeRule;
use cs_backtest::sim::{Inputs, Run, Sizing, Trade, simulate};
use cs_core::time::{fmt_minute, now_ms, split, ymd_hms_to_ms};
use cs_core::{Candle, Event, Timeframe};
use cs_data::{DataDir, Market as DataMarket, clean, store};
use cs_patterns::context::SESSIONS;
use cs_stats::bootstrap::block_mean_ci;
use cs_stats::hyp::mean_t;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

use space::{Dir, SETUPS, Setup, TFS, Trial};

/// DISCOVERY start (data start).
pub const DISC_START: i64 = ymd_hms_to_ms(2024, 11, 1, 0, 0, 0);
/// DISCOVERY end = VALIDATION start.
pub const DISC_END: i64 = ymd_hms_to_ms(2025, 11, 1, 0, 0, 0);
/// VALIDATION end = HOLDOUT start (the seal boundary: last 183 days of the data).
pub const VAL_END: i64 = ymd_hms_to_ms(2026, 5, 27, 0, 0, 0);
/// End of the data (close of the last 1m candle, 2026-11-25 23:59).
pub const DATA_END: i64 = ymd_hms_to_ms(2026, 11, 26, 0, 0, 0);

pub const SYMBOLS: [&str; 2] = ["BTCUSDT", "ETHUSDT"];
pub const SEED: u64 = 1;
pub const AUDIT_CUTS: usize = 3;
/// Minimum trades for a p-value (protocol section 4).
pub const MIN_TRADES_P: usize = 30;
pub const FDR_Q: f64 = 0.10;

pub struct Opts {
    pub data: DataDir,
    pub reports: PathBuf,
    pub threads: usize,
}

impl Opts {
    pub fn research_dir(&self) -> PathBuf {
        self.reports.join("research")
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let d = Sha256::digest(bytes);
    d.iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

pub fn file_sha256(p: &Path) -> Result<String, String> {
    std::fs::read(p)
        .map(|b| sha256_hex(&b))
        .map_err(|e| format!("{}: {e}", p.display()))
}

pub fn write_file(p: &Path, body: &str) -> Result<(), String> {
    if let Some(d) = p.parent() {
        std::fs::create_dir_all(d).map_err(|e| e.to_string())?;
    }
    std::fs::write(p, body).map_err(|e| format!("{}: {e}", p.display()))
}

pub fn read_json(p: &Path) -> Result<Value, String> {
    let s = std::fs::read_to_string(p).map_err(|e| format!("{}: {e}", p.display()))?;
    serde_json::from_str(&s).map_err(|e| format!("{}: {e}", p.display()))
}

pub fn write_json(p: &Path, v: &Value) -> Result<(), String> {
    write_file(
        p,
        &serde_json::to_string_pretty(v).map_err(|e| e.to_string())?,
    )
}

/// Number formatting for CSV and Markdown (`-` for NaN, `inf` for infinities).
pub fn f(x: f64, d: usize) -> String {
    if x.is_finite() {
        format!("{x:.d$}")
    } else if x.is_nan() {
        String::new()
    } else if x > 0.0 {
        "inf".into()
    } else {
        "-inf".into()
    }
}

/// JSON number or null.
pub fn jn(x: f64) -> Value {
    if x.is_finite() { json!(x) } else { Value::Null }
}

// ---------------------------------------------------------------- seal and lock

pub fn seal_path(reports: &Path) -> PathBuf {
    reports.join("holdout-seal.json")
}

pub fn lock_path(reports: &Path) -> PathBuf {
    reports.join("holdout.lock")
}

/// Seal boundary from `reports/holdout-seal.json`, if the seal exists.
pub fn seal_boundary(reports: &Path) -> Result<Option<i64>, String> {
    let p = seal_path(reports);
    if !p.exists() {
        return Ok(None);
    }
    let v = read_json(&p)?;
    v["boundary_ms"]
        .as_i64()
        .map(Some)
        .ok_or_else(|| format!("{}: no boundary_ms", p.display()))
}

/// Who may load data at or after the seal boundary.
#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Access {
    /// Everything before the boundary only.
    Sealed,
    /// `--final` after the lock file was created.
    Final,
    /// `--seal` hashing the holdout bytes (no simulation).
    SealHash,
}

/// Candles of one symbol: 1m candles, funding and the 5m/15m/1h series, all ending before the
/// `end` given to [`load_symbol`].
pub struct SymData {
    pub symbol: String,
    pub m1: Vec<Candle>,
    pub funding: Vec<Funding>,
    pub bars: BTreeMap<Timeframe, Vec<Candle>>,
}

fn candle_bytes(cs: &[Candle], h: &mut Sha256) {
    for c in cs {
        h.update(c.open_time.to_le_bytes());
        for x in [c.open, c.high, c.low, c.close, c.volume] {
            h.update(x.to_bits().to_le_bytes());
        }
    }
}

pub fn candles_sha256(cs: &[Candle]) -> String {
    let mut h = Sha256::new();
    candle_bytes(cs, &mut h);
    h.finalize().iter().fold(String::new(), |mut s, b| {
        let _ = write!(s, "{b:02x}");
        s
    })
}

/// Load one symbol up to `end` (exclusive: candles must close by `end`, 1m candles and funding
/// before it). Refuses `end > boundary` unless the access allows it.
pub fn load_symbol(
    dd: &DataDir,
    sym: &str,
    end: i64,
    boundary: i64,
    access: Access,
) -> Result<SymData, String> {
    if end > boundary && access == Access::Sealed {
        return Err(format!(
            "refused: loading data up to {} would read the sealed holdout (from {}); only --final may",
            fmt_minute(end),
            fmt_minute(boundary)
        ));
    }
    let mut bars = BTreeMap::new();
    for tf in TFS {
        let path = dd.resampled(DataMarket::Um, sym, tf.label());
        let tb = store::read_file(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        let (b, _) = store::table_to_candles(&tb).map_err(|e| e.to_string())?;
        let ms = tf.ms();
        bars.insert(
            tf,
            b.into_iter()
                .filter(|c| c.open_time + ms <= end)
                .collect::<Vec<_>>(),
        );
    }
    let m1: Vec<Candle> = clean::klines(dd, DataMarket::Um, sym)
        .map_err(|e| e.to_string())?
        .candles
        .into_iter()
        .filter(|c| c.open_time < end)
        .collect();
    let (ft, _, _) = clean::series(&dd.funding(sym), None).map_err(|e| e.to_string())?;
    let mut funding = Vec::new();
    if ft.nrows() > 0 {
        let ts = ft.i64("time").map_err(|e| e.to_string())?;
        let rs = ft.f64("rate").map_err(|e| e.to_string())?;
        funding = ts
            .iter()
            .zip(rs)
            .filter(|(t, _)| **t < end)
            .map(|(&time, &rate)| Funding { time, rate })
            .collect();
    }
    Ok(SymData {
        symbol: sym.to_string(),
        m1,
        funding,
        bars,
    })
}

/// `--seal`: write the seal before any result exists.
pub fn seal(o: &Opts, out: &dyn Fn(&str)) -> Result<(), String> {
    let p = seal_path(&o.reports);
    if p.exists() {
        return Err(format!(
            "{} already exists; the seal is never overwritten",
            p.display()
        ));
    }
    let mut hashes = serde_json::Map::new();
    for sym in SYMBOLS {
        let d = load_symbol(&o.data, sym, i64::MAX, VAL_END, Access::SealHash)?;
        let last = d.m1.last().map_or(0, |c| c.open_time + 60_000);
        if last != DATA_END {
            return Err(format!(
                "{sym}: data end {} differs from the protocol's {}",
                fmt_minute(last),
                fmt_minute(DATA_END)
            ));
        }
        let mut m = serde_json::Map::new();
        let h1: Vec<Candle> =
            d.m1.iter()
                .copied()
                .filter(|c| c.open_time >= VAL_END)
                .collect();
        m.insert(
            "1m".into(),
            json!({"candles": h1.len(), "sha256": candles_sha256(&h1)}),
        );
        for (tf, b) in &d.bars {
            let h: Vec<Candle> = b
                .iter()
                .copied()
                .filter(|c| c.open_time >= VAL_END)
                .collect();
            m.insert(
                tf.label().into(),
                json!({"candles": h.len(), "sha256": candles_sha256(&h)}),
            );
        }
        hashes.insert(sym.into(), Value::Object(m));
    }
    let v = json!({
        "boundary_utc": fmt_minute(VAL_END),
        "boundary_ms": VAL_END,
        "data_end_utc": fmt_minute(DATA_END),
        "discovery_utc": [fmt_minute(DISC_START), fmt_minute(DISC_END)],
        "validation_utc": [fmt_minute(DISC_END), fmt_minute(VAL_END)],
        "holdout_utc": [fmt_minute(VAL_END), fmt_minute(DATA_END)],
        "created_utc": cs_core::time::fmt_second(now_ms()),
        "protocol_sha256": file_sha256(&o.reports.join("protocol.md"))?,
        "search_space_sha256": file_sha256(&o.reports.join("search-space.md"))?,
        "holdout_data_sha256": Value::Object(hashes),
        "rule": "Only `candlescope research --final` may load data from the boundary on. It creates reports/holdout.lock first and refuses to run while that file exists; only the user deletes it.",
    });
    write_json(&p, &v)?;
    out(&format!(
        "sealed: holdout from {} written to {}",
        fmt_minute(VAL_END),
        p.display()
    ));
    Ok(())
}

/// Read the seal and warn if the protocol files changed since sealing. Returns the seal JSON.
pub fn check_seal(o: &Opts, out: &dyn Fn(&str)) -> Result<Value, String> {
    let p = seal_path(&o.reports);
    if !p.exists() {
        return Err("no holdout seal: run `candlescope research --seal` first".into());
    }
    let v = read_json(&p)?;
    if v["boundary_ms"].as_i64() != Some(VAL_END) {
        return Err("seal boundary differs from the protocol's validation end".into());
    }
    for (k, file) in [
        ("protocol_sha256", "protocol.md"),
        ("search_space_sha256", "search-space.md"),
    ] {
        let now = file_sha256(&o.reports.join(file))?;
        if v[k].as_str() != Some(now.as_str()) {
            out(&format!(
                "WARNING: reports/{file} changed since the seal (allowed only with a written reason in protocol.md section 12)"
            ));
        }
    }
    Ok(v)
}

// ---------------------------------------------------------------- markets and events

/// Detector market for one time frame, with the engine's prefix audit.
pub fn build_market(bars: &[Candle], tf: Timeframe) -> Result<(Market, usize), String> {
    let src = DetectorSource::default_set();
    let m = Market::prepare(bars.to_vec(), tf.ms(), &src).map_err(|e| e.to_string())?;
    let cuts = audit_cuts(m.bars.len(), AUDIT_CUTS, SEED);
    let a = audit_prefix(&m, &src, &cuts).map_err(|e| format!("{}: {e}", tf.label()))?;
    Ok((m, a.events_compared))
}

/// Random-entry events (seed 1, p = 0.5) for the same candles, audited.
pub fn random_events(m: &Market) -> Result<Vec<Event>, String> {
    let src = RandomSource {
        seed: SEED,
        prob: 0.5,
    };
    let mut rm = m.clone();
    rm.events = src.events(&m.bars, m.tf_ms);
    let cuts = audit_cuts(rm.bars.len(), AUDIT_CUTS, SEED);
    audit_prefix(&rm, &src, &cuts).map_err(|e| e.to_string())?;
    Ok(rm.events)
}

/// Directional events of a setup, with directions flipped for `Reversed`.
pub fn setup_events(full: &Market, st: &Setup, dir: Dir) -> Vec<Event> {
    full.events
        .iter()
        .filter(|e| st.matches(e))
        .map(|e| {
            let mut x = *e;
            if dir == Dir::Reversed {
                x.dir = -x.dir;
            }
            x
        })
        .collect()
}

/// Confluence count of an event (with its trade direction) at its signal candle, protocol
/// section 9. Uses only the features, levels and events known at that candle.
pub fn conf_count(full: &Market, tf: Timeframe, e: &Event) -> u8 {
    let k = e.known_idx as usize;
    let d = e.dir;
    let fe = &full.features[k];
    let h4 = &fe.htf[1];
    let c1 = h4.n > 0 && h4.ema.ready && h4.ema.stack == d;
    let hs = if tf == Timeframe::H1 {
        &fe.htf[1]
    } else {
        &fe.htf[0]
    };
    let c2 = hs.n > 0 && hs.structure == d;
    let close = full.bars[k].close;
    let lv = &full.levels[k];
    let inside = fe.zone_dist_atr.is_finite() && fe.zone_dist_atr < 1e-9;
    let near = fe.atr.is_finite()
        && if d > 0 {
            lv.below[0].is_finite() && close - lv.below[0] <= 0.5 * fe.atr
        } else {
            lv.above[0].is_finite() && lv.above[0] - close <= 0.5 * fe.atr
        };
    let c3 = inside || near;
    let lo = k.saturating_sub(2) as u32;
    let a = full.events.partition_point(|x| x.known_idx < lo);
    let c4 = full.events[a..]
        .iter()
        .take_while(|x| x.known_idx as usize <= k)
        .any(|x| x.det != e.det && x.dir == d);
    u8::from(c1) + u8::from(c2) + u8::from(c3) + u8::from(c4)
}

/// Events of a trial: setup events, flipped, confluence-filtered.
pub fn trial_events(full: &Market, t: &Trial) -> Vec<Event> {
    let ev = setup_events(full, &SETUPS[t.setup], t.dir);
    if t.conf_min == 0 {
        return ev;
    }
    ev.into_iter()
        .filter(|e| conf_count(full, t.tf, e) >= t.conf_min)
        .collect()
}

pub fn trial_rule(t: &Trial) -> TradeRule {
    t.rule.trade_rule(&t.label(), space::filters_of(t.filter))
}

// ---------------------------------------------------------------- simulation and statistics

pub fn costs(mult: f64) -> CostModel {
    CostModel::BINANCE_UM.with_mult(mult)
}

/// Run a rule at 1x and 2x costs (the same trades; checked).
pub fn run_pair(m: &Market, d: &SymData, rule: &TradeRule) -> Result<(Run, Run), String> {
    run_pair_with(m, d, rule, costs(1.0), costs(2.0))
}

pub fn run_pair_with(
    m: &Market,
    d: &SymData,
    rule: &TradeRule,
    c1: CostModel,
    c2: CostModel,
) -> Result<(Run, Run), String> {
    let inp = |c| Inputs {
        market: m,
        m1: &d.m1,
        funding: &d.funding,
        costs: c,
        sizing: Sizing::default(),
    };
    let r1 = simulate(&inp(c1), rule).map_err(|e| format!("{}: {e}", rule.name))?;
    let r2 = simulate(&inp(c2), rule).map_err(|e| format!("{}: {e}", rule.name))?;
    let same = r1.trades.len() == r2.trades.len()
        && r1.trades.iter().zip(&r2.trades).all(|(a, b)| {
            a.signal_idx == b.signal_idx
                && a.entry_time == b.entry_time
                && a.exit_time == b.exit_time
        });
    if !same {
        return Err(format!(
            "ENGINE CHECK FAILED: costs changed the trade path of {} (stop and report)",
            rule.name
        ));
    }
    Ok((r1, r2))
}

/// Statistics of the trades whose decision time lies in `[a, b)`.
#[derive(Debug, Clone, Copy)]
pub struct Stats {
    pub signals: u32,
    pub n: usize,
    pub win_rate: f64,
    pub exp1: f64,
    pub exp2: f64,
    pub gross: f64,
    pub cost1: f64,
    pub sd: f64,
    pub t: f64,
    pub p: f64,
    pub max_dd: f64,
    pub streak: usize,
    pub worst: f64,
    pub pf: f64,
    pub sum1: f64,
    pub eod: usize,
}

impl Default for Stats {
    fn default() -> Self {
        Stats {
            signals: 0,
            n: 0,
            win_rate: f64::NAN,
            exp1: f64::NAN,
            exp2: f64::NAN,
            gross: f64::NAN,
            cost1: f64::NAN,
            sd: f64::NAN,
            t: f64::NAN,
            p: 1.0,
            max_dd: 0.0,
            streak: 0,
            worst: f64::NAN,
            pf: f64::NAN,
            sum1: 0.0,
            eod: 0,
        }
    }
}

/// Indices of the trades in `[a, b)` (by decision time), in realised order (exit time, entry
/// time, signal order), as the engine's metrics order them.
pub fn window_order(tr: &[Trade], a: i64, b: i64) -> Vec<usize> {
    let mut o: Vec<usize> = (0..tr.len())
        .filter(|&i| tr[i].decision_at >= a && tr[i].decision_at < b)
        .collect();
    o.sort_by_key(|&i| (tr[i].exit_time, tr[i].entry_time, i));
    o
}

pub fn stats(r1: &Run, r2: &Run, a: i64, b: i64) -> Stats {
    let o = window_order(&r1.trades, a, b);
    let n = o.len();
    let mut s = Stats {
        signals: r1.signals,
        n,
        ..Stats::default()
    };
    if n == 0 {
        return s;
    }
    let x1: Vec<f64> = o.iter().map(|&i| r1.trades[i].net_r).collect();
    let x2: Vec<f64> = o.iter().map(|&i| r2.trades[i].net_r).collect();
    let nf = n as f64;
    s.sum1 = x1.iter().sum();
    s.exp1 = s.sum1 / nf;
    s.exp2 = x2.iter().sum::<f64>() / nf;
    s.gross = o.iter().map(|&i| r1.trades[i].gross_r).sum::<f64>() / nf;
    s.cost1 = s.gross - s.exp1;
    s.win_rate = x1.iter().filter(|x| **x > 0.0).count() as f64 / nf;
    let mt = mean_t(&x1);
    s.sd = mt.sd;
    s.t = mt.t;
    s.p = if n < MIN_TRADES_P { 1.0 } else { mt.p };
    let (mut cum, mut peak, mut dd, mut run, mut longest) =
        (0.0f64, 0.0f64, 0.0f64, 0usize, 0usize);
    for &x in &x1 {
        cum += x;
        peak = peak.max(cum);
        dd = dd.max(peak - cum);
        if x < 0.0 {
            run += 1;
            longest = longest.max(run);
        } else {
            run = 0;
        }
    }
    s.max_dd = dd;
    s.streak = longest;
    s.worst = x1.iter().copied().fold(f64::INFINITY, f64::min);
    let w: f64 = x1.iter().filter(|x| **x > 0.0).sum();
    let l: f64 = x1.iter().filter(|x| **x < 0.0).sum();
    s.pf = if l < 0.0 {
        w / -l
    } else if w > 0.0 {
        f64::INFINITY
    } else {
        f64::NAN
    };
    s.eod = o
        .iter()
        .filter(|&&i| r1.trades[i].reason == cs_backtest::sim::ExitReason::EndOfData)
        .count();
    s
}

/// Net R at 1x of the trades in `[a, b)` in realised order.
pub fn window_r(r: &Run, a: i64, b: i64) -> Vec<f64> {
    window_order(&r.trades, a, b)
        .iter()
        .map(|&i| r.trades[i].net_r)
        .collect()
}

/// Block-bootstrap interval of the mean (protocol section 4).
pub fn boot(xs: &[f64], level: f64) -> (f64, f64) {
    block_mean_ci(xs, 0, 2000, level, SEED).map_or((f64::NAN, f64::NAN), |c| (c.lo, c.hi))
}

pub fn stats_json(s: &Stats) -> Value {
    json!({
        "trades": s.n,
        "win_rate": jn(s.win_rate),
        "exp_net_1x": jn(s.exp1),
        "exp_net_2x": jn(s.exp2),
        "exp_gross": jn(s.gross),
        "cost_1x": jn(s.cost1),
        "t": jn(s.t),
        "p": jn(s.p),
        "max_dd_r": jn(s.max_dd),
        "longest_losing_streak": s.streak,
        "worst_trade_r": jn(s.worst),
        "profit_factor": jn(s.pf),
        "sum_r_1x": jn(s.sum1),
        "end_of_data": s.eod,
    })
}

// ---------------------------------------------------------------- ledger

pub const LEDGER_HEADER: &str = "stage,trial_id,family,symbol,tf,setup,direction,rule,filter,conf_min,period,signals,trades,win_rate,exp_net_1x,exp_net_2x,exp_gross,cost_1x,sd_1x,t_stat,p_value,q_value,bh_pass,plateau,max_dd_r,longest_losing_streak,worst_trade_r,profit_factor,end_of_data\n";

/// Identity columns of a ledger line.
pub struct Desc {
    pub id: String,
    pub family: String,
    pub tf: String,
    pub setup: String,
    pub dir: String,
    pub rule: String,
    pub filter: String,
    pub conf: String,
}

impl Desc {
    pub fn of(t: &Trial) -> Desc {
        Desc {
            id: t.id.to_string(),
            family: t.family.label().to_string(),
            tf: t.tf.label().to_string(),
            setup: SETUPS[t.setup].name.to_string(),
            dir: t.dir.label().to_string(),
            rule: t.rule.name(),
            filter: space::FILTER_IDS[t.filter].to_string(),
            conf: t.conf_min.to_string(),
        }
    }

    /// A run that is not a trial (baselines, matched random controls).
    pub fn other(setup: &str, tf: Timeframe, rule: &str, family: &str) -> Desc {
        Desc {
            id: String::new(),
            family: family.to_string(),
            tf: tf.label().to_string(),
            setup: setup.to_string(),
            dir: String::new(),
            rule: rule.to_string(),
            filter: String::new(),
            conf: String::new(),
        }
    }
}

/// One ledger line.
#[allow(clippy::too_many_arguments)]
pub fn ledger_line(
    stage: &str,
    d: &Desc,
    symbol: &str,
    period: &str,
    s: &Stats,
    q: f64,
    bh: Option<bool>,
    plateau: Option<bool>,
) -> String {
    let Desc {
        id,
        family: fam,
        tf,
        setup,
        dir,
        rule,
        filter: filt,
        conf,
    } = d;
    let b = |x: Option<bool>| x.map_or(String::new(), |v| u8::from(v).to_string());
    let pe = format!("{:.3e}", s.p);
    format!(
        "{stage},{id},{fam},{symbol},{tf},{setup},{dir},{rule},{filt},{conf},{period},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}\n",
        s.signals,
        s.n,
        f(s.win_rate, 4),
        f(s.exp1, 4),
        f(s.exp2, 4),
        f(s.gross, 4),
        f(s.cost1, 4),
        f(s.sd, 4),
        f(s.t, 3),
        pe,
        if q.is_nan() {
            String::new()
        } else {
            format!("{q:.3e}")
        },
        b(bh),
        b(plateau),
        f(s.max_dd, 2),
        s.streak,
        f(s.worst, 3),
        f(s.pf, 3),
        s.eod
    )
}

/// Rebuild `reports/trial-ledger.csv` from the stage files that exist.
pub fn rebuild_ledger(o: &Opts) -> Result<usize, String> {
    let mut body = String::from(LEDGER_HEADER);
    let mut rows = 0;
    for stage in ["discovery", "validation", "holdout"] {
        let p = o.research_dir().join(format!("ledger-{stage}.csv"));
        if let Ok(s) = std::fs::read_to_string(&p) {
            for line in s.lines().skip(1) {
                body.push_str(line);
                body.push('\n');
                rows += 1;
            }
        }
    }
    write_file(&o.reports.join("trial-ledger.csv"), &body)?;
    Ok(rows)
}

// ---------------------------------------------------------------- diagnostics helpers

/// Calendar month index since 2024-11 (0) of a time.
pub fn month_index(t: i64) -> usize {
    let (y, m, ..) = split(t);
    ((y - 2024) * 12 + (m - 11)).max(0) as usize
}

pub fn month_label(i: usize) -> String {
    let y = 2024 + (10 + i as i64) / 12;
    let m = (10 + i as i64) % 12 + 1;
    format!("{y:04}-{m:02}")
}

/// Breakdown rows: (key, n, expectancy 1x, sum 1x).
pub fn breakdown(
    m: &Market,
    r1: &Run,
    a: i64,
    b: i64,
    key: impl Fn(&Market, &Trade) -> String,
) -> Vec<(String, usize, f64, f64)> {
    let mut g: BTreeMap<String, Vec<f64>> = BTreeMap::new();
    for i in window_order(&r1.trades, a, b) {
        let t = &r1.trades[i];
        g.entry(key(m, t)).or_default().push(t.net_r);
    }
    g.into_iter()
        .map(|(k, v)| {
            let s: f64 = v.iter().sum();
            (k, v.len(), s / v.len() as f64, s)
        })
        .collect()
}

pub fn regime_key(_: &Market, t: &Trade) -> String {
    match t.regime {
        0 => "range".into(),
        r if r == t.dir => "with_trend".into(),
        _ => "against_trend".into(),
    }
}

pub fn vol_key(m: &Market, t: &Trade) -> String {
    let v = m.features[t.signal_idx as usize].vol_pct;
    if !v.is_finite() {
        "n/a".into()
    } else if v < 1.0 / 3.0 {
        "low".into()
    } else if v <= 2.0 / 3.0 {
        "mid".into()
    } else {
        "high".into()
    }
}

pub fn session_key(_: &Market, t: &Trade) -> String {
    SESSIONS
        .get(t.session as usize)
        .map_or("?", |s| *s)
        .to_string()
}

pub fn hour_key(m: &Market, t: &Trade) -> String {
    format!("{:02}", m.features[t.signal_idx as usize].hour)
}

pub fn breakdown_json(rows: &[(String, usize, f64, f64)]) -> Value {
    Value::Array(
        rows.iter()
            .map(|(k, n, e, s)| json!({"key": k, "trades": n, "exp_net_1x": jn(*e), "sum_r": jn(*s)}))
            .collect(),
    )
}

/// Share of the total net R that comes from the best three UTC hours (`NaN` if the total is not
/// positive).
pub fn top3_hour_share(rows: &[(String, usize, f64, f64)]) -> f64 {
    let total: f64 = rows.iter().map(|r| r.3).sum();
    if total <= 0.0 {
        return f64::NAN;
    }
    let mut s: Vec<f64> = rows.iter().map(|r| r.3).collect();
    s.sort_by(|a, b| b.total_cmp(a));
    s.iter().take(3).sum::<f64>() / total
}

// ---------------------------------------------------------------- parallel map

/// Map `items` over `threads` workers, each with its own state from `init`; results in input
/// order. The first error stops the run.
pub fn par_map<T, S, R>(
    items: &[T],
    threads: usize,
    init: impl Fn() -> S + Sync,
    work: impl Fn(&mut S, &T) -> Result<R, String> + Sync,
) -> Result<Vec<R>, String>
where
    T: Sync,
    R: Send,
{
    let next = AtomicUsize::new(0);
    let results: Mutex<Vec<(usize, R)>> = Mutex::new(Vec::with_capacity(items.len()));
    let err: Mutex<Option<String>> = Mutex::new(None);
    std::thread::scope(|sc| {
        for _ in 0..threads.max(1) {
            sc.spawn(|| {
                let mut st = init();
                loop {
                    if err.lock().map_or(true, |e| e.is_some()) {
                        return;
                    }
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(item) = items.get(i) else { return };
                    match work(&mut st, item) {
                        Ok(r) => {
                            if let Ok(mut v) = results.lock() {
                                v.push((i, r));
                            }
                        }
                        Err(e) => {
                            if let Ok(mut x) = err.lock() {
                                x.get_or_insert(e);
                            }
                            return;
                        }
                    }
                }
            });
        }
    });
    if let Some(e) = err.into_inner().map_err(|e| e.to_string())? {
        return Err(e);
    }
    let mut v = results.into_inner().map_err(|e| e.to_string())?;
    if v.len() != items.len() {
        return Err(format!(
            "parallel map: {} of {} results",
            v.len(),
            items.len()
        ));
    }
    v.sort_by_key(|x| x.0);
    Ok(v.into_iter().map(|x| x.1).collect())
}

/// Phase 3 random-entry control rule (stop 1 ATR, target 1 R).
pub fn random_control_rule() -> TradeRule {
    crate::backtest::random_rule()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cs_backtest::testkit::{aggregate, regime_walk_1m};

    fn temp_dir(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cs-research-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        std::fs::create_dir_all(d.join("research")).unwrap();
        d
    }

    fn opts(reports: &Path) -> Opts {
        Opts {
            data: DataDir::new(reports.join("no-data")),
            reports: reports.to_path_buf(),
            threads: 1,
        }
    }

    fn fake_seal(d: &Path) {
        std::fs::write(d.join("protocol.md"), "p").unwrap();
        std::fs::write(d.join("search-space.md"), "s").unwrap();
        let v = json!({"boundary_ms": VAL_END, "created_utc": "test"});
        std::fs::write(seal_path(d), v.to_string()).unwrap();
    }

    #[test]
    fn sealed_access_refuses_holdout_before_reading_anything() {
        let dd = DataDir::new("definitely-missing-data-dir");
        let e = load_symbol(&dd, "BTCUSDT", VAL_END + 1, VAL_END, Access::Sealed)
            .err()
            .unwrap();
        assert!(e.contains("refused"), "{e}");
        // Up to the boundary is allowed (fails here only because the data dir is missing).
        let e = load_symbol(&dd, "BTCUSDT", VAL_END, VAL_END, Access::Sealed)
            .err()
            .unwrap();
        assert!(!e.contains("refused"), "{e}");
    }

    #[test]
    fn final_refuses_when_the_lock_exists() {
        let d = temp_dir("lock");
        fake_seal(&d);
        std::fs::write(lock_path(&d), "{}").unwrap();
        let e = holdout::run(&opts(&d), &|_| {}).err().unwrap();
        assert!(e.contains("already been opened"), "{e}");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn final_without_passed_finalists_keeps_the_seal_and_creates_no_lock() {
        let d = temp_dir("nopass");
        fake_seal(&d);
        std::fs::write(
            d.join("research").join("validation.json"),
            json!({"passed": []}).to_string(),
        )
        .unwrap();
        let e = holdout::run(&opts(&d), &|_| {}).err().unwrap();
        assert!(e.contains("stays sealed"), "{e}");
        assert!(!lock_path(&d).exists());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn seal_is_never_overwritten() {
        let d = temp_dir("seal");
        fake_seal(&d);
        let e = seal(&opts(&d), &|_| {}).err().unwrap();
        assert!(e.contains("never overwritten"), "{e}");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn confluence_and_reversal_use_only_the_past() {
        let m1 = regime_walk_1m(60 * 24 * 40, 50_000.0, 0.0008, 0.00004, 7);
        let bars = aggregate(&m1, Timeframe::M15.ms());
        let (full, _) = build_market(&bars, Timeframe::M15).unwrap();
        for cut in [900, 2000, bars.len() - 5] {
            let (part, _) = build_market(&bars[..cut], Timeframe::M15).unwrap();
            for st in &SETUPS {
                for dir in Dir::ALL {
                    let a: Vec<Event> = setup_events(&full, st, dir)
                        .into_iter()
                        .filter(|e| (e.known_idx as usize) < cut)
                        .collect();
                    let b = setup_events(&part, st, dir);
                    assert_eq!(a.len(), b.len(), "{} {cut}", st.name);
                    for (x, y) in a.iter().zip(&b) {
                        assert!(x.same(y));
                        assert_eq!(
                            conf_count(&full, Timeframe::M15, x),
                            conf_count(&part, Timeframe::M15, y),
                            "{} at {}",
                            st.name,
                            x.known_idx
                        );
                    }
                }
            }
        }
        // Reversal flips only the direction.
        let st = &SETUPS[0];
        let n = setup_events(&full, st, Dir::Native);
        let r = setup_events(&full, st, Dir::Reversed);
        assert!(!n.is_empty());
        assert!(
            n.iter()
                .zip(&r)
                .all(|(a, b)| a.dir == -b.dir && a.known_idx == b.known_idx)
        );
    }
}
