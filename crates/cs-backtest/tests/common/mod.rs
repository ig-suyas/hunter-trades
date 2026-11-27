//! Shared helpers for the cs-backtest integration tests.
// Test code: panics are the failure mode, sizes are tiny, printing is the report.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stdout,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::needless_range_loop,
    clippy::too_many_lines,
    dead_code
)]

use cs_backtest::cost::{CostModel, Funding};
use cs_backtest::market::{FixedSource, Market};
use cs_backtest::rule::{Entry, Overlap, Select, StopRule, Target, TradeRule};
use cs_backtest::sim::{Inputs, Sizing};
use cs_backtest::testkit::{T0, m1};
use cs_core::{Candle, Event};

pub const TF: i64 = 15 * 60_000;
pub const MIN: i64 = 60_000;

/// 15m candles from (o, h, l, c) starting at T0.
pub fn bars(spec: &[(f64, f64, f64, f64)]) -> Vec<Candle> {
    spec.iter()
        .enumerate()
        .map(|(i, &(o, h, l, c))| m1(T0 + i as i64 * TF, o, h, l, c))
        .collect()
}

/// 15 one-minute candles that aggregate exactly to `b`, path open -> high -> low -> close.
pub fn split_hl(b: &Candle) -> Vec<Candle> {
    split_path(b, &[b.open, b.high, b.low, b.close])
}

/// 15 one-minute candles that aggregate exactly to `b`, path open -> low -> high -> close.
pub fn split_lh(b: &Candle) -> Vec<Candle> {
    split_path(b, &[b.open, b.low, b.high, b.close])
}

/// One-minute candles walking through `pts` (one minute per leg), then flat at the last point.
pub fn split_path(b: &Candle, pts: &[f64]) -> Vec<Candle> {
    let mut v = Vec::with_capacity(15);
    for w in pts.windows(2) {
        let (o, c) = (w[0], w[1]);
        v.push(m1(
            b.open_time + v.len() as i64 * MIN,
            o,
            o.max(c),
            o.min(c),
            c,
        ));
    }
    let last = *pts.last().unwrap_or(&b.close);
    while v.len() < 15 {
        v.push(m1(
            b.open_time + v.len() as i64 * MIN,
            last,
            last,
            last,
            last,
        ));
    }
    v
}

/// 1m data for all bars (path o-h-l-c), replacing bars listed in `custom` with their own minutes.
pub fn minutes(bs: &[Candle], custom: &[(usize, Vec<Candle>)]) -> Vec<Candle> {
    let mut v = Vec::new();
    for (i, b) in bs.iter().enumerate() {
        match custom.iter().find(|(k, _)| *k == i) {
            Some((_, ms)) => v.extend_from_slice(ms),
            None => v.extend(split_hl(b)),
        }
    }
    v
}

/// An event known at candle `k`.
pub fn ev(bs: &[Candle], k: usize, dir: i8, lo: f64, hi: f64) -> Event {
    let mut e = Event::new(
        "test",
        if dir > 0 { "long" } else { "short" },
        dir,
        k as u32,
    )
    .with_zone(lo, hi)
    .with_price(if dir > 0 { hi } else { lo });
    e.known_at = bs[k].open_time;
    e.formed_at = bs[k].open_time;
    e
}

pub fn market(bs: Vec<Candle>, events: Vec<Event>) -> Market {
    Market::prepare(bs, TF, &FixedSource(events)).unwrap()
}

pub fn inputs<'a>(m: &'a Market, m1s: &'a [Candle], f: &'a [Funding], c: CostModel) -> Inputs<'a> {
    Inputs {
        market: m,
        m1: m1s,
        funding: f,
        costs: c,
        sizing: Sizing {
            equity0: 10_000.0,
            risk_frac: 0.01,
            max_leverage: 100.0,
        },
    }
}

/// Next-open, structural stop with `buffer` ATR, fixed R target, any test event.
pub fn rule(buffer: f64, target: Target) -> TradeRule {
    TradeRule {
        name: "t".into(),
        select: Select::det("test"),
        entry: Entry::NextOpen,
        stop: StopRule::Structural { buffer_atr: buffer },
        target,
        time_stop_bars: None,
        trail: None,
        filters: vec![],
        overlap: Overlap::OnePerSetup,
        min_stop_atr: 0.0,
    }
}

/// Flat warm-up candles (O 100, H 101, L 99, C 100): ATR(14) is exactly 2.0 from candle 13 on.
pub fn flat(n: usize, p: f64) -> Vec<(f64, f64, f64, f64)> {
    vec![(p, p + 1.0, p - 1.0, p); n]
}

pub fn close(a: f64, b: f64) -> bool {
    (a - b).abs() <= 1e-9 * (1.0 + a.abs().max(b.abs()))
}
