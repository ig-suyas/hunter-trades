//! Hand-built candlestick fixtures: at least two positive and two negative cases per pattern.
//!
//! Each fixture is a 30-bar warm-up with constant true range 100 (so ATR(14) = 100 exactly) in a
//! steady down-trend or up-trend, followed by the pattern bars. Pattern bars are written in ATR
//! units relative to the last warm-up close (P + 100 x value). A positive case must fire the
//! pattern on the last bar; a negative case (a near miss on one rule, or the right shape in the
//! wrong trend) must not.

// Test code: panics are the failure mode, sizes are tiny, printing is the report.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stdout,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::unreadable_literal
)]

use cs_core::Candle;
use cs_patterns::candles::{CandleParams, Candles, PATTERNS};
use cs_patterns::testutil::{ohlc, timed};
use cs_patterns::{Detector, run_all};

#[derive(Clone, Copy, PartialEq)]
enum Ctx {
    Down,
    Up,
}
use Ctx::{Down, Up};

type B = (f64, f64, f64, f64);

fn warmup(ctx: Ctx) -> Vec<Candle> {
    let mut v = Vec::new();
    let mut c = 10_000.0;
    for _ in 0..30 {
        let o = c;
        let k = match ctx {
            Down => {
                c = o - 20.0;
                ohlc(o, o + 40.0, c - 40.0, c)
            }
            Up => {
                c = o + 20.0;
                ohlc(o, c + 40.0, o - 40.0, c)
            }
        };
        v.push(k);
    }
    v
}

fn fires(pattern: &str, ctx: Ctx, bars: &[B]) -> bool {
    let mut v = warmup(ctx);
    let p = v.last().unwrap().close;
    let a = 100.0;
    for &(o, h, l, c) in bars {
        assert!(
            h >= o.max(c) && l <= o.min(c),
            "{pattern}: bad fixture bar {o} {h} {l} {c}"
        );
        v.push(ohlc(p + a * o, p + a * h, p + a * l, p + a * c));
    }
    let v = timed(v);
    let last = (v.len() - 1) as u32;
    let det = Box::new(Candles::new(CandleParams::default())) as Box<dyn Detector>;
    run_all(vec![det], &v)
        .iter()
        .any(|e| e.kind == pattern && e.known_idx == last)
}

struct Fx {
    pattern: &'static str,
    ctx: Ctx,
    bars: Vec<B>,
    expect: bool,
}

fn fx(pattern: &'static str, ctx: Ctx, bars: &[B], expect: bool) -> Fx {
    Fx {
        pattern,
        ctx,
        bars: bars.to_vec(),
        expect,
    }
}

#[allow(clippy::too_many_lines)]
fn fixtures() -> Vec<Fx> {
    // Reused shapes.
    let hammer1 = [(0.0, 0.25, -0.8, 0.2)];
    let hammer2 = [(0.0, 0.02, -1.0, -0.25)];
    let inv1 = [(0.0, 0.8, -0.03, 0.2)];
    let inv2 = [(0.0, 1.0, -0.25, -0.2)];
    let beng1 = [(0.0, 0.1, -0.5, -0.4), (-0.4, 0.3, -0.5, 0.2)];
    let beng2 = [(0.0, 0.05, -0.3, -0.2), (-0.25, 0.6, -0.3, 0.5)];
    let seng1 = [(0.0, 0.5, -0.1, 0.4), (0.4, 0.5, -0.3, -0.2)];
    let seng2 = [(0.0, 0.3, -0.05, 0.2), (0.25, 0.3, -0.6, -0.5)];
    let bhar1 = [(0.0, 0.05, -0.85, -0.8), (-0.8, -0.5, -0.85, -0.6)];
    let bhar2 = [(0.0, 0.1, -1.0, -0.9), (-0.7, -0.4, -0.75, -0.45)];
    let shar1 = [(0.0, 0.85, -0.05, 0.8), (0.8, 0.85, 0.5, 0.6)];
    let shar2 = [(0.0, 1.0, -0.1, 0.9), (0.7, 0.75, 0.4, 0.45)];
    let ms1 = [
        (0.0, 0.05, -0.9, -0.8),
        (-0.8, -0.7, -1.0, -0.95),
        (-0.95, -0.3, -1.0, -0.35),
    ];
    let es1 = [
        (0.0, 0.9, -0.05, 0.8),
        (0.8, 1.0, 0.7, 0.95),
        (0.95, 1.0, 0.3, 0.35),
    ];
    let cat = |a: &[B], b: &[B]| -> Vec<B> { a.iter().chain(b).copied().collect() };
    vec![
        // ---------------- bullish
        fx("hammer", Down, &hammer1, true),
        fx("hammer", Down, &hammer2, true),
        fx("hammer", Up, &hammer1, false),
        fx("hammer", Down, &[(0.0, 0.25, -0.4, 0.2)], false),
        fx("inverted_hammer", Down, &inv1, true),
        fx("inverted_hammer", Down, &inv2, true),
        fx("inverted_hammer", Up, &inv1, false),
        fx("inverted_hammer", Down, &[(0.0, 0.8, -0.2, 0.2)], false),
        fx("bullish_engulfing", Down, &beng1, true),
        fx("bullish_engulfing", Down, &beng2, true),
        fx("bullish_engulfing", Up, &beng1, false),
        fx(
            "bullish_engulfing",
            Down,
            &[(0.0, 0.1, -0.5, -0.4), (-0.4, 0.0, -0.5, -0.05)],
            false,
        ),
        fx(
            "piercing_line",
            Down,
            &[(0.0, 0.1, -0.9, -0.8), (-0.8, -0.2, -0.9, -0.3)],
            true,
        ),
        fx(
            "piercing_line",
            Down,
            &[(0.0, 0.05, -1.05, -1.0), (-1.0, -0.1, -1.1, -0.2)],
            true,
        ),
        fx(
            "piercing_line",
            Up,
            &[(0.0, 0.1, -0.9, -0.8), (-0.8, -0.2, -0.9, -0.3)],
            false,
        ),
        fx(
            "piercing_line",
            Down,
            &[(0.0, 0.1, -0.9, -0.8), (-0.8, -0.4, -0.9, -0.5)],
            false,
        ),
        fx("morning_star", Down, &ms1, true),
        fx(
            "morning_star",
            Down,
            &[
                (0.0, 0.1, -1.0, -0.9),
                (-0.9, -0.85, -1.2, -1.1),
                (-1.1, -0.2, -1.15, -0.3),
            ],
            true,
        ),
        fx("morning_star", Up, &ms1, false),
        fx(
            "morning_star",
            Down,
            &[
                (0.0, 0.05, -0.9, -0.8),
                (-0.8, -0.7, -1.0, -0.95),
                (-0.95, -0.6, -1.0, -0.65),
            ],
            false,
        ),
        fx(
            "morning_doji_star",
            Down,
            &[
                (0.0, 0.05, -0.9, -0.8),
                (-0.8, -0.7, -0.95, -0.85),
                (-0.85, -0.3, -0.9, -0.35),
            ],
            true,
        ),
        fx(
            "morning_doji_star",
            Down,
            &[
                (0.0, 0.1, -1.0, -0.9),
                (-0.9, -0.8, -1.0, -0.9),
                (-0.9, -0.2, -0.95, -0.3),
            ],
            true,
        ),
        fx(
            "morning_doji_star",
            Up,
            &[
                (0.0, 0.05, -0.9, -0.8),
                (-0.8, -0.7, -0.95, -0.85),
                (-0.85, -0.3, -0.9, -0.35),
            ],
            false,
        ),
        fx("morning_doji_star", Down, &ms1, false),
        fx(
            "three_white_soldiers",
            Down,
            &[
                (0.0, 0.55, -0.05, 0.5),
                (0.3, 1.05, 0.25, 1.0),
                (0.8, 1.6, 0.75, 1.55),
            ],
            true,
        ),
        fx(
            "three_white_soldiers",
            Down,
            &[
                (0.0, 0.45, -0.1, 0.4),
                (0.4, 0.95, 0.35, 0.9),
                (0.9, 1.42, 0.85, 1.4),
            ],
            true,
        ),
        fx(
            "three_white_soldiers",
            Up,
            &[
                (0.0, 0.55, -0.05, 0.5),
                (0.3, 1.05, 0.25, 1.0),
                (0.8, 1.6, 0.75, 1.55),
            ],
            false,
        ),
        fx(
            "three_white_soldiers",
            Down,
            &[
                (0.0, 0.55, -0.05, 0.5),
                (0.3, 1.05, 0.25, 1.0),
                (0.8, 1.9, 0.75, 1.55),
            ],
            false,
        ),
        fx(
            "tweezer_bottom",
            Down,
            &[(0.0, 0.1, -0.6, -0.4), (-0.4, 0.0, -0.62, -0.1)],
            true,
        ),
        fx(
            "tweezer_bottom",
            Down,
            &[(0.0, 0.05, -0.5, -0.3), (-0.3, 0.1, -0.5, 0.05)],
            true,
        ),
        fx(
            "tweezer_bottom",
            Up,
            &[(0.0, 0.1, -0.6, -0.4), (-0.4, 0.0, -0.62, -0.1)],
            false,
        ),
        fx(
            "tweezer_bottom",
            Down,
            &[(0.0, 0.1, -0.6, -0.4), (-0.4, 0.0, -0.75, -0.1)],
            false,
        ),
        fx("bullish_harami", Down, &bhar1, true),
        fx("bullish_harami", Down, &bhar2, true),
        fx("bullish_harami", Up, &bhar1, false),
        fx(
            "bullish_harami",
            Down,
            &[(0.0, 0.05, -0.85, -0.8), (-0.85, -0.6, -0.9, -0.65)],
            false,
        ),
        fx(
            "bullish_harami_cross",
            Down,
            &[(0.0, 0.05, -0.85, -0.8), (-0.8, -0.6, -0.9, -0.75)],
            true,
        ),
        fx(
            "bullish_harami_cross",
            Down,
            &[(0.0, 0.1, -1.0, -0.9), (-0.5, -0.3, -0.7, -0.5)],
            true,
        ),
        fx(
            "bullish_harami_cross",
            Up,
            &[(0.0, 0.05, -0.85, -0.8), (-0.8, -0.6, -0.9, -0.75)],
            false,
        ),
        fx("bullish_harami_cross", Down, &bhar1, false),
        fx(
            "three_inside_up",
            Down,
            &cat(&bhar1, &[(-0.6, 0.15, -0.65, 0.1)]),
            true,
        ),
        fx(
            "three_inside_up",
            Down,
            &cat(&bhar2, &[(-0.45, 0.2, -0.5, 0.15)]),
            true,
        ),
        fx(
            "three_inside_up",
            Up,
            &cat(&bhar1, &[(-0.6, 0.15, -0.65, 0.1)]),
            false,
        ),
        fx(
            "three_inside_up",
            Down,
            &cat(&bhar1, &[(-0.6, -0.05, -0.65, -0.1)]),
            false,
        ),
        fx(
            "three_outside_up",
            Down,
            &cat(&beng1, &[(0.2, 0.7, 0.15, 0.6)]),
            true,
        ),
        fx(
            "three_outside_up",
            Down,
            &cat(&beng2, &[(0.5, 0.9, 0.45, 0.85)]),
            true,
        ),
        fx(
            "three_outside_up",
            Up,
            &cat(&beng1, &[(0.2, 0.7, 0.15, 0.6)]),
            false,
        ),
        fx(
            "three_outside_up",
            Down,
            &cat(&beng1, &[(0.2, 0.3, 0.0, 0.1)]),
            false,
        ),
        fx(
            "rising_three_methods",
            Up,
            &[
                (0.0, 0.85, -0.05, 0.8),
                (0.8, 0.82, 0.6, 0.65),
                (0.65, 0.7, 0.45, 0.5),
                (0.5, 0.62, 0.4, 0.55),
                (0.55, 1.3, 0.5, 1.25),
            ],
            true,
        ),
        fx(
            "rising_three_methods",
            Up,
            &[
                (0.0, 1.05, -0.02, 1.0),
                (1.0, 1.02, 0.75, 0.8),
                (0.8, 0.9, 0.6, 0.7),
                (0.7, 0.8, 0.55, 0.6),
                (0.6, 1.5, 0.55, 1.45),
            ],
            true,
        ),
        fx(
            "rising_three_methods",
            Down,
            &[
                (0.0, 0.85, -0.05, 0.8),
                (0.8, 0.82, 0.6, 0.65),
                (0.65, 0.7, 0.45, 0.5),
                (0.5, 0.62, 0.4, 0.55),
                (0.55, 1.3, 0.5, 1.25),
            ],
            false,
        ),
        fx(
            "rising_three_methods",
            Up,
            &[
                (0.0, 0.85, -0.05, 0.8),
                (0.8, 0.82, 0.6, 0.65),
                (0.65, 0.7, -0.2, 0.5),
                (0.5, 0.62, 0.4, 0.55),
                (0.55, 1.3, 0.5, 1.25),
            ],
            false,
        ),
        fx("bullish_marubozu", Down, &[(0.0, 0.82, -0.03, 0.8)], true),
        fx("bullish_marubozu", Up, &[(0.0, 1.2, 0.0, 1.2)], true),
        fx("bullish_marubozu", Down, &[(0.0, 0.95, -0.03, 0.8)], false),
        fx("bullish_marubozu", Down, &[(0.0, 0.42, -0.02, 0.4)], false),
        fx("dragonfly_doji", Down, &[(0.0, 0.05, -0.7, 0.03)], true),
        fx("dragonfly_doji", Down, &[(0.0, 0.0, -0.9, 0.0)], true),
        fx("dragonfly_doji", Up, &[(0.0, 0.05, -0.7, 0.03)], false),
        fx("dragonfly_doji", Down, &[(0.0, 0.3, -0.7, 0.03)], false),
        // ---------------- bearish
        fx("hanging_man", Up, &hammer1, true),
        fx("hanging_man", Up, &hammer2, true),
        fx("hanging_man", Down, &hammer1, false),
        fx("hanging_man", Up, &[(0.0, 0.5, -0.8, 0.2)], false),
        fx("shooting_star", Up, &inv1, true),
        fx("shooting_star", Up, &inv2, true),
        fx("shooting_star", Down, &inv1, false),
        fx("shooting_star", Up, &[(0.0, 0.4, -0.03, 0.2)], false),
        fx("bearish_engulfing", Up, &seng1, true),
        fx("bearish_engulfing", Up, &seng2, true),
        fx("bearish_engulfing", Down, &seng1, false),
        fx(
            "bearish_engulfing",
            Up,
            &[(0.0, 0.5, -0.1, 0.4), (0.4, 0.5, 0.0, 0.05)],
            false,
        ),
        fx(
            "dark_cloud_cover",
            Up,
            &[(0.0, 0.9, -0.1, 0.8), (0.8, 0.9, 0.2, 0.3)],
            true,
        ),
        fx(
            "dark_cloud_cover",
            Up,
            &[(0.0, 1.05, -0.05, 1.0), (1.0, 1.1, 0.15, 0.2)],
            true,
        ),
        fx(
            "dark_cloud_cover",
            Down,
            &[(0.0, 0.9, -0.1, 0.8), (0.8, 0.9, 0.2, 0.3)],
            false,
        ),
        fx(
            "dark_cloud_cover",
            Up,
            &[(0.0, 0.9, -0.1, 0.8), (0.8, 0.9, 0.45, 0.5)],
            false,
        ),
        fx("evening_star", Up, &es1, true),
        fx(
            "evening_star",
            Up,
            &[
                (0.0, 1.0, -0.1, 0.9),
                (0.9, 1.2, 0.85, 1.1),
                (1.1, 1.15, 0.2, 0.3),
            ],
            true,
        ),
        fx("evening_star", Down, &es1, false),
        fx(
            "evening_star",
            Up,
            &[
                (0.0, 0.9, -0.05, 0.8),
                (0.8, 1.0, 0.7, 0.95),
                (0.95, 1.0, 0.6, 0.65),
            ],
            false,
        ),
        fx(
            "evening_doji_star",
            Up,
            &[
                (0.0, 0.9, -0.05, 0.8),
                (0.8, 0.95, 0.7, 0.85),
                (0.85, 0.9, 0.3, 0.35),
            ],
            true,
        ),
        fx(
            "evening_doji_star",
            Up,
            &[
                (0.0, 1.0, -0.1, 0.9),
                (0.9, 1.0, 0.8, 0.9),
                (0.9, 0.95, 0.2, 0.3),
            ],
            true,
        ),
        fx(
            "evening_doji_star",
            Down,
            &[
                (0.0, 0.9, -0.05, 0.8),
                (0.8, 0.95, 0.7, 0.85),
                (0.85, 0.9, 0.3, 0.35),
            ],
            false,
        ),
        fx("evening_doji_star", Up, &es1, false),
        fx(
            "three_black_crows",
            Up,
            &[
                (0.0, 0.05, -0.55, -0.5),
                (-0.3, -0.25, -1.05, -1.0),
                (-0.8, -0.75, -1.6, -1.55),
            ],
            true,
        ),
        fx(
            "three_black_crows",
            Up,
            &[
                (0.0, 0.1, -0.45, -0.4),
                (-0.4, -0.35, -0.95, -0.9),
                (-0.9, -0.85, -1.42, -1.4),
            ],
            true,
        ),
        fx(
            "three_black_crows",
            Down,
            &[
                (0.0, 0.05, -0.55, -0.5),
                (-0.3, -0.25, -1.05, -1.0),
                (-0.8, -0.75, -1.6, -1.55),
            ],
            false,
        ),
        fx(
            "three_black_crows",
            Up,
            &[
                (0.0, 0.05, -0.55, -0.5),
                (-0.3, -0.25, -1.05, -1.0),
                (-0.8, -0.75, -1.9, -1.55),
            ],
            false,
        ),
        fx(
            "tweezer_top",
            Up,
            &[(0.0, 0.6, -0.1, 0.4), (0.4, 0.62, 0.0, 0.1)],
            true,
        ),
        fx(
            "tweezer_top",
            Up,
            &[(0.0, 0.5, -0.05, 0.3), (0.3, 0.5, -0.1, -0.05)],
            true,
        ),
        fx(
            "tweezer_top",
            Down,
            &[(0.0, 0.6, -0.1, 0.4), (0.4, 0.62, 0.0, 0.1)],
            false,
        ),
        fx(
            "tweezer_top",
            Up,
            &[(0.0, 0.6, -0.1, 0.4), (0.4, 0.75, 0.0, 0.1)],
            false,
        ),
        fx("bearish_harami", Up, &shar1, true),
        fx("bearish_harami", Up, &shar2, true),
        fx("bearish_harami", Down, &shar1, false),
        fx(
            "bearish_harami",
            Up,
            &[(0.0, 0.85, -0.05, 0.8), (0.85, 0.9, 0.6, 0.65)],
            false,
        ),
        fx(
            "bearish_harami_cross",
            Up,
            &[(0.0, 0.85, -0.05, 0.8), (0.8, 0.9, 0.6, 0.75)],
            true,
        ),
        fx(
            "bearish_harami_cross",
            Up,
            &[(0.0, 1.0, -0.1, 0.9), (0.5, 0.7, 0.3, 0.5)],
            true,
        ),
        fx(
            "bearish_harami_cross",
            Down,
            &[(0.0, 0.85, -0.05, 0.8), (0.8, 0.9, 0.6, 0.75)],
            false,
        ),
        fx("bearish_harami_cross", Up, &shar1, false),
        fx(
            "three_inside_down",
            Up,
            &cat(&shar1, &[(0.6, 0.65, -0.15, -0.1)]),
            true,
        ),
        fx(
            "three_inside_down",
            Up,
            &cat(&shar2, &[(0.45, 0.5, -0.2, -0.15)]),
            true,
        ),
        fx(
            "three_inside_down",
            Down,
            &cat(&shar1, &[(0.6, 0.65, -0.15, -0.1)]),
            false,
        ),
        fx(
            "three_inside_down",
            Up,
            &cat(&shar1, &[(0.6, 0.65, 0.05, 0.1)]),
            false,
        ),
        fx(
            "three_outside_down",
            Up,
            &cat(&seng1, &[(-0.2, -0.15, -0.7, -0.6)]),
            true,
        ),
        fx(
            "three_outside_down",
            Up,
            &cat(&seng2, &[(-0.5, -0.45, -0.9, -0.85)]),
            true,
        ),
        fx(
            "three_outside_down",
            Down,
            &cat(&seng1, &[(-0.2, -0.15, -0.7, -0.6)]),
            false,
        ),
        fx(
            "three_outside_down",
            Up,
            &cat(&seng1, &[(-0.2, 0.0, -0.3, -0.1)]),
            false,
        ),
        fx(
            "falling_three_methods",
            Down,
            &[
                (0.0, 0.05, -0.85, -0.8),
                (-0.8, -0.6, -0.82, -0.65),
                (-0.65, -0.45, -0.7, -0.5),
                (-0.5, -0.4, -0.62, -0.55),
                (-0.55, -0.5, -1.3, -1.25),
            ],
            true,
        ),
        fx(
            "falling_three_methods",
            Down,
            &[
                (0.0, 0.02, -1.05, -1.0),
                (-1.0, -0.75, -1.02, -0.8),
                (-0.8, -0.6, -0.9, -0.7),
                (-0.7, -0.55, -0.8, -0.6),
                (-0.6, -0.55, -1.5, -1.45),
            ],
            true,
        ),
        fx(
            "falling_three_methods",
            Up,
            &[
                (0.0, 0.05, -0.85, -0.8),
                (-0.8, -0.6, -0.82, -0.65),
                (-0.65, -0.45, -0.7, -0.5),
                (-0.5, -0.4, -0.62, -0.55),
                (-0.55, -0.5, -1.3, -1.25),
            ],
            false,
        ),
        fx(
            "falling_three_methods",
            Down,
            &[
                (0.0, 0.05, -0.85, -0.8),
                (-0.8, -0.6, -0.82, -0.65),
                (-0.65, 0.2, -0.7, -0.5),
                (-0.5, -0.4, -0.62, -0.55),
                (-0.55, -0.5, -1.3, -1.25),
            ],
            false,
        ),
        fx("bearish_marubozu", Up, &[(0.0, 0.03, -0.82, -0.8)], true),
        fx("bearish_marubozu", Down, &[(0.0, 0.0, -1.2, -1.2)], true),
        fx("bearish_marubozu", Up, &[(0.0, 0.03, -0.95, -0.8)], false),
        fx("bearish_marubozu", Up, &[(0.0, 0.02, -0.42, -0.4)], false),
        fx("gravestone_doji", Up, &[(0.0, 0.7, -0.05, -0.03)], true),
        fx("gravestone_doji", Up, &[(0.0, 0.9, 0.0, 0.0)], true),
        fx("gravestone_doji", Down, &[(0.0, 0.7, -0.05, -0.03)], false),
        fx("gravestone_doji", Up, &[(0.0, 0.7, -0.33, -0.03)], false),
        // ---------------- neutral
        fx("doji", Down, &[(0.0, 0.3, -0.3, 0.02)], true),
        fx("doji", Up, &[(0.0, 0.1, -0.2, -0.05)], true),
        fx("doji", Down, &[(0.0, 0.3, -0.3, 0.15)], false),
        fx("doji", Down, &[(0.0, 0.1, -0.1, 0.0)], false),
        fx("long_legged_doji", Down, &[(0.0, 0.6, -0.6, 0.02)], true),
        fx("long_legged_doji", Up, &[(0.0, 0.4, -0.8, -0.05)], true),
        fx("long_legged_doji", Down, &[(0.0, 0.4, -0.4, 0.02)], false),
        fx("long_legged_doji", Down, &[(0.0, 0.2, -1.0, 0.02)], false),
        fx("spinning_top", Down, &[(0.0, 0.5, -0.3, 0.2)], true),
        fx("spinning_top", Up, &[(0.0, 0.3, -0.5, -0.15)], true),
        fx("spinning_top", Down, &[(0.0, 0.5, -0.3, 0.05)], false),
        fx("spinning_top", Down, &[(0.0, 0.5, -0.1, 0.2)], false),
        fx("high_wave", Down, &[(0.0, 1.0, -0.7, 0.2)], true),
        fx("high_wave", Up, &[(0.0, 0.9, -0.9, -0.25)], true),
        fx("high_wave", Down, &[(0.0, 0.6, -0.6, 0.2)], false),
        fx("high_wave", Down, &[(0.0, 1.4, -0.3, 0.2)], false),
        fx("rickshaw_man", Down, &[(0.0, 0.6, -0.6, 0.02)], true),
        fx("rickshaw_man", Up, &[(-0.02, 0.55, -0.55, 0.03)], true),
        fx("rickshaw_man", Down, &[(0.0, 0.4, -0.8, -0.05)], false),
        fx("rickshaw_man", Down, &[(0.0, 0.4, -0.4, 0.02)], false),
    ]
}

#[test]
fn candlestick_fixtures() {
    let fxs = fixtures();
    let mut failures = Vec::new();
    for (k, f) in fxs.iter().enumerate() {
        let got = fires(f.pattern, f.ctx, &f.bars);
        if got != f.expect {
            failures.push(format!(
                "#{k} {} ({}) expected {} got {}",
                f.pattern,
                if f.ctx == Down { "down" } else { "up" },
                f.expect,
                got
            ));
        }
    }
    // Coverage: every pattern has >= 2 positive and >= 2 negative fixtures.
    for (_, kind, _, _) in PATTERNS {
        let pos = fxs.iter().filter(|f| f.pattern == kind && f.expect).count();
        let neg = fxs
            .iter()
            .filter(|f| f.pattern == kind && !f.expect)
            .count();
        if pos < 2 || neg < 2 {
            failures.push(format!(
                "{kind}: only {pos} positive / {neg} negative fixtures"
            ));
        }
    }
    for f in &fxs {
        assert!(
            PATTERNS.iter().any(|p| p.1 == f.pattern),
            "unknown pattern {}",
            f.pattern
        );
    }
    println!(
        "candlestick fixtures: {} cases over {} patterns, {} failures",
        fxs.len(),
        PATTERNS.len(),
        failures.len()
    );
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
