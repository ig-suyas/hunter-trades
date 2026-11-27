//! Hand-built scenarios for the structural detectors (sweep, breakout / retest / fakeout,
//! supply / demand, Fibonacci, trendline, role reversal).

// Test code: panics are the failure mode, sizes are tiny, printing is the report.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stdout,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::unreadable_literal
)]

use cs_core::{Candle, Event};
use cs_patterns::breakout::{BreakoutParams, Breakouts, TAG_RANGE};
use cs_patterns::fib::{Fib, FibParams};
use cs_patterns::sweep::{POOL_DAY, SweepParams, Sweeps};
use cs_patterns::testutil::{ohlcv, timed};
use cs_patterns::zones::{SdParams, SdZones, TAG_FRESH};
use cs_patterns::{Detector, run_all};

fn b(o: f64, h: f64, l: f64, c: f64) -> Candle {
    ohlcv(o, h, l, c, 1.0)
}

/// `n` bars ranging 100..110 (true range 10) with alternating bodies.
fn range_bars(n: usize) -> Vec<Candle> {
    (0..n)
        .map(|k| {
            if k % 2 == 0 {
                b(104.0, 110.0, 100.0, 106.0)
            } else {
                b(106.0, 110.0, 100.0, 104.0)
            }
        })
        .collect()
}

fn run(d: Box<dyn Detector>, cs: Vec<Candle>) -> (Vec<Event>, u32) {
    let cs = timed(cs);
    let last = (cs.len() - 1) as u32;
    (run_all(vec![d], &cs), last)
}

fn has(ev: &[Event], kind: &str, at: u32) -> bool {
    ev.iter().any(|e| e.kind == kind && e.known_idx == at)
}

#[test]
fn sweep_of_previous_day_low() {
    // Day 1 (96 x 15m) ranges 100..110; on day 2 a bar wicks to 97 and closes back at 103.
    let mut cs = range_bars(96);
    cs.extend(range_bars(4));
    cs.push(b(104.0, 105.0, 97.0, 103.0));
    let (ev, last) = run(Box::new(Sweeps::new(SweepParams::default())), cs);
    let e = ev
        .iter()
        .find(|e| e.kind == "sweep_low" && e.known_idx == last)
        .expect("sweep_low on the reclaim bar");
    assert!(e.tags & POOL_DAY != 0);
    assert!((e.price - 100.0).abs() < 1e-9 && (e.lo - 97.0).abs() < 1e-9);
}

#[test]
fn no_sweep_when_the_close_stays_below() {
    let mut cs = range_bars(100);
    cs.push(b(104.0, 105.0, 95.0, 96.0));
    cs.push(b(96.0, 97.0, 93.0, 94.0));
    cs.push(b(94.0, 96.0, 92.0, 95.0));
    cs.push(b(95.0, 101.0, 94.0, 100.5)); // too late: outside the 3-bar reclaim window
    let (ev, _) = run(Box::new(Sweeps::new(SweepParams::default())), cs);
    assert!(!ev.iter().any(|e| e.kind == "sweep_low"));
}

#[test]
fn range_breakout_then_retest() {
    let mut cs = range_bars(60);
    cs.push(ohlcv(106.0, 125.0, 105.0, 124.0, 5.0)); // range 20, volume 5x
    cs.push(b(124.0, 125.0, 110.5, 112.0)); // back to the level, holds above 110
    cs.push(b(112.0, 115.0, 111.0, 114.5)); // bull trigger
    let (ev, last) = run(Box::new(Breakouts::new(BreakoutParams::default())), cs);
    assert!(ev.iter().any(|e| e.kind == "range_formed"));
    let bo = ev
        .iter()
        .find(|e| e.kind == "breakout_up")
        .expect("breakout_up");
    assert_eq!(bo.known_idx, last - 2);
    assert!(bo.tags & TAG_RANGE != 0 && (bo.price - 110.0).abs() < 1e-9);
    assert!(has(&ev, "retest_long", last));
    assert!(!ev.iter().any(|e| e.kind == "fakeout_up_failed"));
}

#[test]
fn breakout_needs_expansion_but_fakeout_does_not() {
    let mut cs = range_bars(60);
    cs.push(b(106.0, 113.0, 105.0, 112.0)); // closes beyond, no volume / range expansion
    cs.push(b(112.0, 112.5, 104.0, 106.0)); // back inside next bar
    let (ev, last) = run(Box::new(Breakouts::new(BreakoutParams::default())), cs);
    assert!(!ev.iter().any(|e| e.kind == "breakout_up"));
    assert!(has(&ev, "fakeout_up_failed", last));
}

#[test]
fn demand_zone_formed_tested_broken() {
    let mut cs: Vec<Candle> = (0..30).map(|_| b(100.0, 105.0, 95.0, 100.0)).collect();
    cs.push(b(100.0, 102.0, 98.0, 101.0)); // base
    cs.push(b(101.0, 102.5, 99.0, 100.0));
    cs.push(b(100.0, 102.0, 98.5, 101.0));
    cs.push(b(101.0, 110.0, 100.5, 109.5)); // departure 1
    cs.push(b(109.5, 121.0, 109.0, 120.0)); // departure 2 -> formed
    let formed_at = cs.len() as u32 - 1;
    cs.push(b(120.0, 121.0, 108.0, 110.0));
    cs.push(b(110.0, 111.0, 100.5, 102.0)); // first test
    let test_at = cs.len() as u32 - 1;
    cs.push(b(102.0, 103.0, 96.0, 97.0)); // close below 98 -> broken
    let (ev, last) = run(Box::new(SdZones::new(SdParams::default())), cs);
    let f = ev
        .iter()
        .find(|e| e.kind == "demand_formed")
        .expect("demand_formed");
    assert_eq!(f.known_idx, formed_at);
    assert!(
        (f.lo - 98.0).abs() < 1e-9 && (f.hi - 101.0).abs() < 1e-9,
        "{f:?}"
    );
    let t = ev
        .iter()
        .find(|e| e.kind == "demand_test")
        .expect("demand_test");
    assert_eq!(t.known_idx, test_at);
    assert!(t.tags & TAG_FRESH != 0);
    assert!(has(&ev, "demand_broken", last));
}

#[test]
fn fib_leg_and_zone_touch() {
    let mut cs: Vec<Candle> = (0..30).map(|_| b(100.0, 105.0, 95.0, 100.0)).collect();
    for k in 0..10 {
        let p = 100.0 + 10.0 * f64::from(k);
        cs.push(b(p, p + 11.0, p - 1.0, p + 10.0));
    }
    cs.push(b(200.0, 200.5, 160.0, 165.0));
    let (ev, last) = run(Box::new(Fib::new(FibParams::default())), cs);
    let leg = ev
        .iter()
        .find(|e| e.kind == "fib_leg_up" && e.known_idx == last)
        .expect("fib_leg_up");
    assert!(
        (leg.ay - 95.0).abs() < 1e-9 && (leg.by - 201.0).abs() < 1e-9,
        "{leg:?}"
    );
    // zone 38.2-61.8 % of 95 -> 201
    assert!((leg.hi - (201.0 - 0.382 * 106.0)).abs() < 1e-9);
    assert!((leg.lo - (201.0 - 0.618 * 106.0)).abs() < 1e-9);
    assert!(has(&ev, "fib_zone_touch", last));
}

#[test]
fn trendline_three_touches_and_break() {
    use cs_patterns::trend::{Trendline, TrendlineParams};
    // 20 flat warm-up bars, then blocks of 6 bars whose single lowest bar lies on the line
    // low = 100 + 10 k at bar 20 + 6 k + 3 (so swing lows are on one straight rising line).
    let mut cs: Vec<Candle> = (0..20).map(|_| b(94.0, 98.0, 90.0, 94.0)).collect();
    for k in 0..4 {
        for off in [6.0, 4.0, 2.0, 0.0, 3.0, 5.0] {
            let l = 100.0 + 10.0 * f64::from(k) + off;
            cs.push(b(l + 4.0, l + 8.0, l, l + 4.0));
        }
    }
    // Collapse through the line.
    let mut p = cs.last().unwrap().close;
    for _ in 0..4 {
        cs.push(b(p, p + 1.0, p - 12.0, p - 11.0));
        p -= 11.0;
    }
    let (ev, _) = run(Box::new(Trendline::new(TrendlineParams::default())), cs);
    let kinds: Vec<&str> = ev.iter().map(|e| e.kind).collect();
    assert!(kinds.contains(&"tl_up_formed"), "{kinds:?}");
    assert!(kinds.contains(&"tl_up_touch"), "{kinds:?}");
    assert!(kinds.contains(&"tl_up_break"), "{kinds:?}");
    let t = ev.iter().find(|e| e.kind == "tl_up_touch").unwrap();
    assert!(t.value >= 3.0);
}
