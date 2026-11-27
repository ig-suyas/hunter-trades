//! Candlestick patterns (knowledge K1-K32): every candlestick pattern named in
//! `technical_analysis_patterns.txt` (15 bullish, 15 bearish, 5 neutral).
//!
//! All size tests are relative to ATR(14), taken at the bar *before* the pattern starts (so the
//! pattern bars do not scale themselves). The constants follow TA-Lib's candle settings with its
//! "average range" replaced by ATR and its "average body" by 0.5 x ATR:
//!
//! | name | default | meaning | sensible range |
//! |---|---|---|---|
//! | `doji_body` | 0.1 | doji: body <= 0.1 ATR | 0.05..=0.15 |
//! | `short_body` | 0.3 | short body: body <= 0.3 ATR | 0.2..=0.5 |
//! | `long_body` | 0.6 | long body: body >= 0.6 ATR | 0.5..=1.0 |
//! | `very_short_shadow` | 0.1 | "no" shadow: <= 0.1 ATR | 0.05..=0.2 |
//! | `marubozu_shadow` | 0.05 | marubozu shadows <= 0.05 ATR | 0.0..=0.1 |
//! | `long_shadow` | 0.5 | hammer-type shadow >= max(2 x body, 0.5 ATR) | 0.3..=1.0 |
//! | `equal` | 0.05 | "equal" prices (tweezers, star gaps) within 0.05 ATR | 0.02..=0.15 |
//! | `star_penetration` | 0.3 | star bar 3 closes >= 30 % into bar 1's body (TA-Lib) | 0.3..=0.5 |
//! | `doji_min_range` | 0.25 | a plain doji needs range >= 0.25 ATR (no flat bars) | 0.1..=0.5 |
//!
//! Prior trend (S7): down before bar `j` if `C[j] < SMA10[j]` and `SMA10[j] < SMA10[j-5]`, where
//! `j` is the bar right before the pattern; up mirrored. Bullish reversal patterns need a prior
//! down-trend and bearish ones a prior up-trend. Rising/falling three methods are continuation
//! patterns and need a trend in their own direction. Marubozus and the five neutral patterns need
//! no trend. Crypto trades 24/7, so the gap conditions of the classic definitions are replaced by
//! the body relations below (recorded as a parameter choice in `notes/knowledge.md`). Doji
//! variants are kept separate: a star or harami with a doji middle bar is only the doji version.
//!
//! Every event: detector `cdl_<name>`, kind = name, `start_idx` = first pattern bar, `price` =
//! last close, `lo..hi` = pattern low..high, `value` = ATR used.

use cs_core::{Candle, Event};

use crate::Detector;
use crate::util::{Atr, Ring, RollStats};

/// Candlestick thresholds (see the module table).
#[derive(Debug, Clone, Copy)]
pub struct CandleParams {
    pub doji_body: f64,
    pub short_body: f64,
    pub long_body: f64,
    pub very_short_shadow: f64,
    pub marubozu_shadow: f64,
    pub long_shadow: f64,
    pub equal: f64,
    pub star_penetration: f64,
    pub doji_min_range: f64,
    /// Prior-trend SMA length and slope bars. Defaults 10 / 5.
    pub trend_sma: usize,
    pub trend_slope: usize,
}

impl Default for CandleParams {
    fn default() -> Self {
        CandleParams {
            doji_body: 0.1,
            short_body: 0.3,
            long_body: 0.6,
            very_short_shadow: 0.1,
            marubozu_shadow: 0.05,
            long_shadow: 0.5,
            equal: 0.05,
            star_penetration: 0.3,
            doji_min_range: 0.25,
            trend_sma: 10,
            trend_slope: 5,
        }
    }
}

/// All pattern names with their direction: (detector name, kind, dir, bars).
pub const PATTERNS: [(&str, &str, i8, usize); 35] = [
    ("cdl_hammer", "hammer", 1, 1),
    ("cdl_inverted_hammer", "inverted_hammer", 1, 1),
    ("cdl_bullish_engulfing", "bullish_engulfing", 1, 2),
    ("cdl_piercing_line", "piercing_line", 1, 2),
    ("cdl_morning_star", "morning_star", 1, 3),
    ("cdl_morning_doji_star", "morning_doji_star", 1, 3),
    ("cdl_three_white_soldiers", "three_white_soldiers", 1, 3),
    ("cdl_tweezer_bottom", "tweezer_bottom", 1, 2),
    ("cdl_bullish_harami", "bullish_harami", 1, 2),
    ("cdl_bullish_harami_cross", "bullish_harami_cross", 1, 2),
    ("cdl_three_inside_up", "three_inside_up", 1, 3),
    ("cdl_three_outside_up", "three_outside_up", 1, 3),
    ("cdl_rising_three_methods", "rising_three_methods", 1, 5),
    ("cdl_bullish_marubozu", "bullish_marubozu", 1, 1),
    ("cdl_dragonfly_doji", "dragonfly_doji", 1, 1),
    ("cdl_hanging_man", "hanging_man", -1, 1),
    ("cdl_shooting_star", "shooting_star", -1, 1),
    ("cdl_bearish_engulfing", "bearish_engulfing", -1, 2),
    ("cdl_dark_cloud_cover", "dark_cloud_cover", -1, 2),
    ("cdl_evening_star", "evening_star", -1, 3),
    ("cdl_evening_doji_star", "evening_doji_star", -1, 3),
    ("cdl_three_black_crows", "three_black_crows", -1, 3),
    ("cdl_tweezer_top", "tweezer_top", -1, 2),
    ("cdl_bearish_harami", "bearish_harami", -1, 2),
    ("cdl_bearish_harami_cross", "bearish_harami_cross", -1, 2),
    ("cdl_three_inside_down", "three_inside_down", -1, 3),
    ("cdl_three_outside_down", "three_outside_down", -1, 3),
    ("cdl_falling_three_methods", "falling_three_methods", -1, 5),
    ("cdl_bearish_marubozu", "bearish_marubozu", -1, 1),
    ("cdl_gravestone_doji", "gravestone_doji", -1, 1),
    ("cdl_doji", "doji", 0, 1),
    ("cdl_long_legged_doji", "long_legged_doji", 0, 1),
    ("cdl_spinning_top", "spinning_top", 0, 1),
    ("cdl_high_wave", "high_wave", 0, 1),
    ("cdl_rickshaw_man", "rickshaw_man", 0, 1),
];

const NAMES: [&str; 35] = {
    let mut n = [""; 35];
    let mut k = 0;
    while k < 35 {
        n[k] = PATTERNS[k].0;
        k += 1;
    }
    n
};

#[derive(Debug, Clone, Copy, Default)]
struct K {
    o: f64,
    h: f64,
    l: f64,
    c: f64,
    /// ATR after this bar.
    atr: f64,
    down: bool,
    up: bool,
}

impl K {
    fn body(&self) -> f64 {
        (self.c - self.o).abs()
    }
    fn rng(&self) -> f64 {
        self.h - self.l
    }
    fn top(&self) -> f64 {
        self.o.max(self.c)
    }
    fn bot(&self) -> f64 {
        self.o.min(self.c)
    }
    fn ush(&self) -> f64 {
        self.h - self.top()
    }
    fn lsh(&self) -> f64 {
        self.bot() - self.l
    }
    fn bull(&self) -> bool {
        self.c > self.o
    }
    fn bear(&self) -> bool {
        self.c < self.o
    }
}

/// All 35 candlestick patterns as one streaming detector.
#[derive(Debug, Clone)]
pub struct Candles {
    p: CandleParams,
    atr: Atr,
    sma: RollStats,
    sma_hist: Ring<f64>,
    bars: Ring<K>,
}

impl Candles {
    pub fn new(p: CandleParams) -> Candles {
        Candles {
            p,
            atr: Atr::new(14),
            sma: RollStats::new(p.trend_sma.max(1)),
            sma_hist: Ring::new(p.trend_slope.max(1) + 1),
            bars: Ring::new(7),
        }
    }

    /// Evaluate all patterns ending at the newest bar; calls `hit(index into PATTERNS)`.
    fn eval(&self, mut hit: impl FnMut(usize)) {
        let p = &self.p;
        let b = |k: usize| self.bars.back(k);
        // ctx(n): the bar right before an n-bar pattern.
        let ctx = |n: usize| b(n).filter(|k| !k.atr.is_nan());
        let mut fire = |name: &str, ok: bool| {
            if ok && let Some(k) = PATTERNS.iter().position(|x| x.1 == name) {
                hit(k);
            }
        };
        // --- one bar
        if let (Some(x), Some(cx)) = (b(0), ctx(1)) {
            let a = cx.atr;
            let doji = x.body() <= p.doji_body * a;
            let short = x.body() <= p.short_body * a;
            let long_sh = (2.0 * x.body()).max(p.long_shadow * a);
            let hammer = short && x.lsh() >= long_sh && x.ush() <= p.very_short_shadow * a;
            let inv = short && x.ush() >= long_sh && x.lsh() <= p.very_short_shadow * a;
            fire("hammer", cx.down && hammer);
            fire("hanging_man", cx.up && hammer);
            fire("inverted_hammer", cx.down && inv);
            fire("shooting_star", cx.up && inv);
            let maru = x.body() >= p.long_body * a
                && x.ush() <= p.marubozu_shadow * a
                && x.lsh() <= p.marubozu_shadow * a;
            fire("bullish_marubozu", maru && x.bull());
            fire("bearish_marubozu", maru && x.bear());
            fire(
                "dragonfly_doji",
                cx.down
                    && doji
                    && x.ush() <= p.very_short_shadow * a
                    && x.lsh() >= p.long_shadow * a,
            );
            fire(
                "gravestone_doji",
                cx.up && doji && x.lsh() <= p.very_short_shadow * a && x.ush() >= p.long_shadow * a,
            );
            fire("doji", doji && x.rng() >= p.doji_min_range * a);
            let ll = doji && x.ush() >= 0.3 * x.rng() && x.lsh() >= 0.3 * x.rng() && x.rng() >= a;
            fire("long_legged_doji", ll);
            let mid = f64::midpoint(x.o, x.c);
            fire(
                "rickshaw_man",
                ll && (mid - f64::midpoint(x.h, x.l)).abs() <= 0.1 * x.rng(),
            );
            let real_small = !doji && short;
            fire(
                "spinning_top",
                real_small && x.ush() > x.body() && x.lsh() > x.body(),
            );
            fire(
                "high_wave",
                real_small
                    && x.ush() >= 2.0 * x.body()
                    && x.lsh() >= 2.0 * x.body()
                    && x.rng() >= 1.5 * a,
            );
        }
        // --- two bars
        if let (Some(x1), Some(x2), Some(cx)) = (b(1), b(0), ctx(2)) {
            let a = cx.atr;
            let doji = |k: &K| k.body() <= p.doji_body * a;
            let short = |k: &K| k.body() <= p.short_body * a;
            let long = |k: &K| k.body() >= p.long_body * a;
            let bull_eng = x1.bear()
                && !doji(&x1)
                && x2.bull()
                && x2.o <= x1.c
                && x2.c >= x1.o
                && x2.body() > x1.body();
            let bear_eng = x1.bull()
                && !doji(&x1)
                && x2.bear()
                && x2.o >= x1.c
                && x2.c <= x1.o
                && x2.body() > x1.body();
            fire("bullish_engulfing", cx.down && bull_eng);
            fire("bearish_engulfing", cx.up && bear_eng);
            fire(
                "piercing_line",
                cx.down
                    && x1.bear()
                    && long(&x1)
                    && x2.bull()
                    && x2.o <= x1.c
                    && x2.c > x1.c + 0.5 * x1.body()
                    && x2.c < x1.o,
            );
            fire(
                "dark_cloud_cover",
                cx.up
                    && x1.bull()
                    && long(&x1)
                    && x2.bear()
                    && x2.o >= x1.c
                    && x2.c < x1.c - 0.5 * x1.body()
                    && x2.c > x1.o,
            );
            let eq = p.equal * a;
            fire(
                "tweezer_bottom",
                cx.down
                    && x1.bear()
                    && !doji(&x1)
                    && x2.bull()
                    && !doji(&x2)
                    && (x1.l - x2.l).abs() <= eq,
            );
            fire(
                "tweezer_top",
                cx.up
                    && x1.bull()
                    && !doji(&x1)
                    && x2.bear()
                    && !doji(&x2)
                    && (x1.h - x2.h).abs() <= eq,
            );
            let in_bear_body = x1.bear() && long(&x1) && x2.top() <= x1.o && x2.bot() >= x1.c;
            let in_bull_body = x1.bull() && long(&x1) && x2.top() <= x1.c && x2.bot() >= x1.o;
            fire(
                "bullish_harami",
                cx.down && in_bear_body && short(&x2) && !doji(&x2),
            );
            fire("bullish_harami_cross", cx.down && in_bear_body && doji(&x2));
            fire(
                "bearish_harami",
                cx.up && in_bull_body && short(&x2) && !doji(&x2),
            );
            fire("bearish_harami_cross", cx.up && in_bull_body && doji(&x2));
        }
        // --- three bars
        if let (Some(x1), Some(x2), Some(x3), Some(cx)) = (b(2), b(1), b(0), ctx(3)) {
            let a = cx.atr;
            let doji = |k: &K| k.body() <= p.doji_body * a;
            let short = |k: &K| k.body() <= p.short_body * a;
            let long = |k: &K| k.body() >= p.long_body * a;
            let eq = p.equal * a;
            let pen = p.star_penetration;
            let ms_base = x1.bear()
                && long(&x1)
                && short(&x2)
                && x2.top() <= x1.c + eq
                && x3.bull()
                && !short(&x3)
                && x3.c > x1.c + pen * x1.body();
            fire("morning_star", cx.down && ms_base && !doji(&x2));
            fire("morning_doji_star", cx.down && ms_base && doji(&x2));
            let es_base = x1.bull()
                && long(&x1)
                && short(&x2)
                && x2.bot() >= x1.c - eq
                && x3.bear()
                && !short(&x3)
                && x3.c < x1.c - pen * x1.body();
            fire("evening_star", cx.up && es_base && !doji(&x2));
            fire("evening_doji_star", cx.up && es_base && doji(&x2));
            let vs = p.very_short_shadow * a;
            let soldiers = [x1, x2, x3]
                .iter()
                .all(|k| k.bull() && !short(k) && k.ush() <= vs)
                && x2.c > x1.c
                && x3.c > x2.c
                && x2.o > x1.o
                && x2.o <= x1.c + eq
                && x3.o > x2.o
                && x3.o <= x2.c + eq
                && x3.body() >= 0.5 * x2.body();
            fire("three_white_soldiers", cx.down && soldiers);
            let crows = [x1, x2, x3]
                .iter()
                .all(|k| k.bear() && !short(k) && k.lsh() <= vs)
                && x2.c < x1.c
                && x3.c < x2.c
                && x2.o < x1.o
                && x2.o >= x1.c - eq
                && x3.o < x2.o
                && x3.o >= x2.c - eq
                && x3.body() >= 0.5 * x2.body();
            fire("three_black_crows", cx.up && crows);
            fire(
                "three_inside_up",
                cx.down
                    && x1.bear()
                    && long(&x1)
                    && short(&x2)
                    && x2.top() <= x1.o
                    && x2.bot() >= x1.c
                    && x3.bull()
                    && x3.c > x1.o,
            );
            fire(
                "three_inside_down",
                cx.up
                    && x1.bull()
                    && long(&x1)
                    && short(&x2)
                    && x2.top() <= x1.c
                    && x2.bot() >= x1.o
                    && x3.bear()
                    && x3.c < x1.o,
            );
            let bull_eng = x1.bear()
                && !doji(&x1)
                && x2.bull()
                && x2.o <= x1.c
                && x2.c >= x1.o
                && x2.body() > x1.body();
            let bear_eng = x1.bull()
                && !doji(&x1)
                && x2.bear()
                && x2.o >= x1.c
                && x2.c <= x1.o
                && x2.body() > x1.body();
            fire(
                "three_outside_up",
                cx.down && bull_eng && x3.bull() && x3.c > x2.c,
            );
            fire(
                "three_outside_down",
                cx.up && bear_eng && x3.bear() && x3.c < x2.c,
            );
        }
        // --- five bars
        if let (Some(x1), Some(x5), Some(cx)) = (b(4), b(0), ctx(5)) {
            let a = cx.atr;
            let short = |k: &K| k.body() <= p.short_body * a;
            let long = |k: &K| k.body() >= p.long_body * a;
            let mids = [b(3), b(2), b(1)];
            let inside = mids
                .iter()
                .all(|m| m.is_some_and(|m| short(&m) && m.h <= x1.h && m.l >= x1.l));
            let n_bear = mids.iter().filter(|m| m.is_some_and(|m| m.bear())).count();
            let n_bull = mids.iter().filter(|m| m.is_some_and(|m| m.bull())).count();
            fire(
                "rising_three_methods",
                cx.up
                    && x1.bull()
                    && long(&x1)
                    && inside
                    && n_bear >= 2
                    && x5.bull()
                    && long(&x5)
                    && x5.c > x1.c,
            );
            fire(
                "falling_three_methods",
                cx.down
                    && x1.bear()
                    && long(&x1)
                    && inside
                    && n_bull >= 2
                    && x5.bear()
                    && long(&x5)
                    && x5.c < x1.c,
            );
        }
    }
}

impl Detector for Candles {
    fn names(&self) -> &'static [&'static str] {
        &NAMES
    }

    fn update(&mut self, i: u32, c: &Candle, out: &mut Vec<Event>) {
        let a = self.atr.update(c);
        self.sma.push(c.close);
        let s = self.sma.mean();
        self.sma_hist.push(s);
        let old = self.sma_hist.back(self.p.trend_slope.max(1));
        let (down, up) = match old {
            Some(o) if !o.is_nan() && !s.is_nan() => (c.close < s && s < o, c.close > s && s > o),
            _ => (false, false),
        };
        self.bars.push(K {
            o: c.open,
            h: c.high,
            l: c.low,
            c: c.close,
            atr: if self.atr.ready() { a } else { f64::NAN },
            down,
            up,
        });
        let bars = &self.bars;
        let mut hits = [false; 35];
        self.eval(|k| hits[k] = true);
        for (k, &(det, kind, dir, n)) in PATTERNS.iter().enumerate() {
            if !hits[k] {
                continue;
            }
            let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
            for j in 0..n {
                if let Some(x) = bars.back(j) {
                    lo = lo.min(x.l);
                    hi = hi.max(x.h);
                }
            }
            let atr_used = bars.back(n).map_or(f64::NAN, |x| x.atr);
            out.push(
                Event::new(det, kind, dir, i)
                    .with_start(i + 1 - n as u32)
                    .with_price(c.close)
                    .with_zone(lo, hi)
                    .with_value(atr_used),
            );
        }
    }
}
