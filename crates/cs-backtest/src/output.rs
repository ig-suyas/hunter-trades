//! Trades CSV, summary JSON and hashes.
//!
//! * `inputs_hash`: SHA-256 over the market (candles, events, features, levels), the 1-minute
//!   and funding data, the rule, costs, sizing and bootstrap settings.
//! * `results_hash`: SHA-256 over every trade field (floats bit for bit), the skip and fill
//!   counters and the summary numbers (including the bootstrap interval). The same inputs and
//!   seed must give the same results hash.

use std::fmt::Write as _;

use cs_core::Candle;
use cs_core::time::fmt_second;
use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};

use crate::cost::Funding;
use crate::market::Market;
use crate::metrics::{BootCfg, Group, Summary};
use crate::sim::{Run, Trade};

fn hex(d: &[u8]) -> String {
    let mut s = String::with_capacity(d.len() * 2);
    for b in d {
        let _ = write!(s, "{b:02x}");
    }
    s
}

struct H(Sha256);

impl H {
    fn new() -> H {
        H(Sha256::new())
    }
    fn f(&mut self, x: f64) {
        self.0.update(x.to_bits().to_le_bytes());
    }
    fn i(&mut self, x: i64) {
        self.0.update(x.to_le_bytes());
    }
    fn u(&mut self, x: u64) {
        self.0.update(x.to_le_bytes());
    }
    fn s(&mut self, x: &str) {
        self.u(x.len() as u64);
        self.0.update(x.as_bytes());
    }
    fn done(self) -> String {
        hex(&self.0.finalize())
    }
}

fn candle(h: &mut H, c: &Candle) {
    h.i(c.open_time);
    for x in [c.open, c.high, c.low, c.close, c.volume] {
        h.f(x);
    }
}

/// Fingerprint of the prepared market (candles, events, features, levels).
pub fn market_fingerprint(m: &Market) -> String {
    let mut h = H::new();
    h.i(m.tf_ms);
    h.s(&m.source);
    h.u(m.bars.len() as u64);
    for c in &m.bars {
        candle(&mut h, c);
    }
    h.u(m.events.len() as u64);
    for e in &m.events {
        h.s(e.det);
        h.s(e.kind);
        h.i(i64::from(e.dir));
        for x in [e.start_idx, e.formed_idx, e.known_idx, e.ax, e.bx, e.tags] {
            h.u(u64::from(x));
        }
        h.i(e.formed_at);
        h.i(e.known_at);
        for x in [e.price, e.lo, e.hi, e.ay, e.by, e.value] {
            h.f(x);
        }
    }
    for f in &m.features {
        for x in [f.atr, f.adx, f.vol_pct, f.zone_dist_atr, f.vol_z] {
            h.f(x);
        }
        h.i(i64::from(f.regime));
        h.u(u64::from(f.hour));
        for s in &f.htf {
            h.u(u64::from(s.n));
            h.i(s.known_at);
            h.i(i64::from(s.structure));
            h.i(i64::from(s.ema.stack));
            h.u(u64::from(s.ema.ready));
        }
    }
    for l in &m.levels {
        for x in l.above.iter().chain(&l.below) {
            h.f(*x);
        }
    }
    h.done()
}

/// Fingerprint of the 1-minute candles and funding rows.
pub fn data_fingerprint(m1: &[Candle], funding: &[Funding]) -> String {
    let mut h = H::new();
    h.u(m1.len() as u64);
    for c in m1 {
        candle(&mut h, c);
    }
    h.u(funding.len() as u64);
    for f in funding {
        h.i(f.time);
        h.f(f.rate);
    }
    h.done()
}

/// Hash of everything that determines a run.
pub fn inputs_hash(market_fp: &str, data_fp: &str, run: &Run, boot: BootCfg) -> String {
    let mut h = H::new();
    h.s(market_fp);
    h.s(data_fp);
    h.s(&format!("{:?}", run.rule));
    h.s(&format!("{:?}", run.costs));
    h.s(&format!("{:?}", run.sizing));
    h.s(&format!("{boot:?}"));
    h.done()
}

fn trade_hash(h: &mut H, t: &Trade) {
    h.s(t.det);
    h.s(t.kind);
    h.i(i64::from(t.dir));
    for x in [t.signal_idx, t.entry_idx, t.exit_idx, t.funding_events] {
        h.u(u64::from(x));
    }
    for x in [t.known_at, t.decision_at, t.entry_time, t.exit_time] {
        h.i(x);
    }
    for x in [
        t.entry_raw,
        t.entry_fill,
        t.stop0,
        t.target,
        t.risk,
        t.exit_raw,
        t.exit_fill,
        t.gross_r,
        t.spread_r,
        t.slip_r,
        t.fee_r,
        t.funding_r,
        t.net_r,
        t.risk_frac,
        t.ret,
        t.equity_before,
        t.pnl,
    ] {
        h.f(x);
    }
    h.s(t.reason.label());
    for b in [t.entry_market, t.ambiguous, t.resolved_1m, t.stop_first] {
        h.u(u64::from(b));
    }
}

/// Hash of the results of a run.
pub fn results_hash(run: &Run, s: &Summary) -> String {
    let mut h = H::new();
    h.u(run.trades.len() as u64);
    for t in &run.trades {
        trade_hash(&mut h, t);
    }
    h.u(u64::from(run.signals));
    for (k, v) in run.skips.fields().iter().chain(run.fills.fields().iter()) {
        h.s(k);
        h.u(u64::from(*v));
    }
    for x in [
        s.win_rate,
        s.avg_win_r,
        s.avg_loss_r,
        s.expectancy_r,
        s.gross_expectancy_r,
        s.mean_cost_r,
        s.profit_factor,
        s.sum_r,
        s.max_dd_r,
        s.max_dd_pct,
        s.total_return,
        s.exposure,
        s.mean_hold_bars,
    ] {
        h.f(x);
    }
    h.u(s.longest_losing_streak as u64);
    if let Some(c) = s.ci {
        for x in [c.mean, c.lo, c.hi, c.se] {
            h.f(x);
        }
        h.u(c.block as u64);
    }
    for g in s.by_regime.iter().chain(&s.by_hour).chain(&s.by_month) {
        h.s(&g.key);
        h.u(g.n as u64);
        h.f(g.sum_r);
    }
    h.done()
}

fn num(x: f64) -> String {
    if x.is_finite() {
        format!("{x}")
    } else {
        String::new()
    }
}

/// Trades as CSV (one header line, one row per trade, times in UTC).
pub fn trades_csv(trades: &[Trade]) -> String {
    let mut s = String::from(
        "det,kind,dir,signal_idx,known_at,decision_at,entry_idx,entry_time,entry_market,entry_raw,\
         entry_fill,stop0,target,risk,exit_idx,exit_time,exit_raw,exit_fill,reason,gross_r,spread_r,\
         slip_r,fee_r,funding_r,net_r,funding_events,risk_frac,ret,equity_before,pnl,ambiguous,\
         resolved_1m,stop_first,regime,session\n",
    );
    for t in trades {
        let _ = writeln!(
            s,
            "{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{},{}",
            t.det,
            t.kind,
            t.dir,
            t.signal_idx,
            fmt_second(t.known_at),
            fmt_second(t.decision_at),
            t.entry_idx,
            fmt_second(t.entry_time),
            u8::from(t.entry_market),
            num(t.entry_raw),
            num(t.entry_fill),
            num(t.stop0),
            num(t.target),
            num(t.risk),
            t.exit_idx,
            fmt_second(t.exit_time),
            num(t.exit_raw),
            num(t.exit_fill),
            t.reason.label(),
            num(t.gross_r),
            num(t.spread_r),
            num(t.slip_r),
            num(t.fee_r),
            num(t.funding_r),
            num(t.net_r),
            t.funding_events,
            num(t.risk_frac),
            num(t.ret),
            num(t.equity_before),
            num(t.pnl),
            u8::from(t.ambiguous),
            u8::from(t.resolved_1m),
            u8::from(t.stop_first),
            t.regime,
            t.session
        );
    }
    s
}

fn groups_json(g: &[Group]) -> Value {
    Value::Array(
        g.iter()
            .map(|x| {
                json!({"key": x.key, "n": x.n, "win_rate": x.win_rate,
                       "expectancy_r": x.expectancy_r, "sum_r": x.sum_r})
            })
            .collect(),
    )
}

/// Summary as JSON (`NaN`/`inf` become `null`).
pub fn summary_json(
    run: &Run,
    s: &Summary,
    boot: BootCfg,
    inputs_hash: &str,
    results_hash: &str,
) -> Value {
    let mut skips = Map::new();
    for (k, v) in run.skips.fields() {
        skips.insert(k.into(), v.into());
    }
    let mut fills = Map::new();
    for (k, v) in run.fills.fields() {
        fills.insert(k.into(), v.into());
    }
    let mut exits = Map::new();
    for (k, v) in &s.exits {
        exits.insert((*k).into(), (*v).into());
    }
    let c = run.costs;
    let ci = s.ci.map_or(Value::Null, |c| {
        json!({"lo": c.lo, "hi": c.hi, "se": c.se, "level": c.level, "block": c.block,
               "reps": c.reps, "seed": boot.seed,
               "method": "circular block bootstrap of net R in exit order"})
    });
    let mut o = Map::new();
    let mut put = |k: &str, v: Value| {
        o.insert(k.to_string(), v);
    };
    put("rule", run.rule.name.clone().into());
    put("rule_detail", format!("{:?}", run.rule).into());
    put(
        "costs",
        json!({"taker_fee_bps": c.taker_fee_bps, "spread_bps": c.spread_bps,
               "slip_bps": c.slip_bps, "slip_atr": c.slip_atr, "mult": c.mult,
               "funding": c.funding}),
    );
    put(
        "sizing",
        json!({"equity0": run.sizing.equity0, "risk_frac": run.sizing.risk_frac,
               "max_leverage": run.sizing.max_leverage}),
    );
    put(
        "span",
        json!({"from": fmt_second(run.span.0), "to": fmt_second(run.span.1)}),
    );
    put("signals", run.signals.into());
    put("skips", Value::Object(skips));
    put("fills", Value::Object(fills));
    put("trades", s.n.into());
    put("wins", s.wins.into());
    put("losses", s.losses.into());
    for (k, v) in [
        ("win_rate", s.win_rate),
        ("avg_win_r", s.avg_win_r),
        ("avg_loss_r", s.avg_loss_r),
        ("expectancy_r", s.expectancy_r),
        ("gross_expectancy_r", s.gross_expectancy_r),
        ("mean_cost_r", s.mean_cost_r),
        ("mean_fee_r", s.mean_fee_r),
        ("mean_spread_r", s.mean_spread_r),
        ("mean_slip_r", s.mean_slip_r),
        ("mean_funding_r", s.mean_funding_r),
        ("sd_r", s.sd_r),
        ("t_stat", s.t_stat),
        ("profit_factor", s.profit_factor),
        ("sum_r", s.sum_r),
        ("max_drawdown_r", s.max_dd_r),
        ("max_drawdown_pct", 100.0 * s.max_dd_pct),
        ("total_return_pct", 100.0 * s.total_return),
        ("mean_return_pct", 100.0 * s.mean_ret),
        ("exposure", s.exposure),
        ("mean_hold_bars", s.mean_hold_bars),
        ("median_hold_bars", s.median_hold_bars),
        ("mean_hold_hours", s.mean_hold_hours),
    ] {
        put(k, v.into());
    }
    put("longest_losing_streak", s.longest_losing_streak.into());
    put("expectancy_ci", ci);
    put("exits", Value::Object(exits));
    put("by_regime", groups_json(&s.by_regime));
    put("by_hour_utc", groups_json(&s.by_hour));
    put("by_month", groups_json(&s.by_month));
    put("flags", s.flags.clone().into());
    put("inputs_hash", inputs_hash.into());
    put("results_hash", results_hash.into());
    Value::Object(o)
}
