//! Trend tools: EMA 20/50/200 state and slope (knowledge T1), trendlines from swing points (T5).

use cs_core::{Candle, Event};

use crate::Detector;
use crate::swing::{Pivot, SwingParams, Swings};
use crate::util::{Atr, Ema, Ring};

/// EMA state parameters.
#[derive(Debug, Clone, Copy)]
pub struct EmaParams {
    /// Fast / mid / slow periods. Defaults 20 / 50 / 200 (the guide's values).
    pub fast: usize,
    pub mid: usize,
    pub slow: usize,
    /// Slope = change over this many bars, in ATR per bar. Default 5; sensible 3..=20.
    pub slope_bars: usize,
}

impl Default for EmaParams {
    fn default() -> Self {
        EmaParams {
            fast: 20,
            mid: 50,
            slow: 200,
            slope_bars: 5,
        }
    }
}

/// Snapshot of the EMA state after a bar.
#[derive(Debug, Clone, Copy, Default)]
pub struct EmaSnap {
    pub fast: f64,
    pub mid: f64,
    pub slow: f64,
    /// Slopes in ATR per bar over `slope_bars`.
    pub slope_fast: f64,
    pub slope_mid: f64,
    pub slope_slow: f64,
    /// +1 bull stack (close > fast > mid > slow, all rising), -1 bear stack, 0 neither.
    pub stack: i8,
    pub ready: bool,
}

/// EMA 20/50/200 with slopes. Detector `ema`: `cross_up_fast_mid` / `cross_down_fast_mid`
/// (20 over/under 50), `cross_up_mid_slow` / `cross_down_mid_slow` (50 over/under 200, i.e.
/// golden / death cross), `bull_stack` / `bear_stack` when the stack state starts.
#[derive(Debug, Clone)]
pub struct EmaState {
    p: EmaParams,
    atr: Atr,
    f: Ema,
    m: Ema,
    s: Ema,
    hist: Ring<(f64, f64, f64)>,
    snap: EmaSnap,
}

impl EmaState {
    pub fn new(p: EmaParams) -> EmaState {
        EmaState {
            p,
            atr: Atr::new(14),
            f: Ema::new(p.fast),
            m: Ema::new(p.mid),
            s: Ema::new(p.slow),
            hist: Ring::new(p.slope_bars.max(1) + 1),
            snap: EmaSnap::default(),
        }
    }

    pub fn snap(&self) -> EmaSnap {
        self.snap
    }

    pub fn step(&mut self, i: u32, c: &Candle, out: &mut Vec<Event>) {
        let a = self.atr.update(c);
        let (f, m, s) = (
            self.f.update(c.close),
            self.m.update(c.close),
            self.s.update(c.close),
        );
        self.hist.push((f, m, s));
        let prev = self.snap;
        let n = self.p.slope_bars.max(1);
        let old = self.hist.back(n);
        let slope = |x: f64, y: Option<f64>| y.map_or(f64::NAN, |y| (x - y) / (n as f64 * a));
        let snap = EmaSnap {
            fast: f,
            mid: m,
            slow: s,
            slope_fast: slope(f, old.map(|o| o.0)),
            slope_mid: slope(m, old.map(|o| o.1)),
            slope_slow: slope(s, old.map(|o| o.2)),
            stack: 0,
            ready: self.s.ready() && old.is_some_and(|o| !o.2.is_nan()),
        };
        let mut snap = snap;
        if snap.ready {
            if c.close > f
                && f > m
                && m > s
                && snap.slope_fast > 0.0
                && snap.slope_mid > 0.0
                && snap.slope_slow > 0.0
            {
                snap.stack = 1;
            } else if c.close < f
                && f < m
                && m < s
                && snap.slope_fast < 0.0
                && snap.slope_mid < 0.0
                && snap.slope_slow < 0.0
            {
                snap.stack = -1;
            }
        }
        self.snap = snap;
        if !(snap.ready && prev.ready) {
            return;
        }
        let ev = |kind, dir: i8, price: f64| {
            Event::new("ema", kind, dir, i)
                .with_price(price)
                .with_zone(f.min(m).min(s), f.max(m).max(s))
        };
        if prev.fast <= prev.mid && f > m {
            out.push(ev("cross_up_fast_mid", 1, f));
        } else if prev.fast >= prev.mid && f < m {
            out.push(ev("cross_down_fast_mid", -1, f));
        }
        if prev.mid <= prev.slow && m > s {
            out.push(ev("cross_up_mid_slow", 1, m));
        } else if prev.mid >= prev.slow && m < s {
            out.push(ev("cross_down_mid_slow", -1, m));
        }
        if snap.stack != prev.stack && snap.stack != 0 {
            let kind = if snap.stack > 0 {
                "bull_stack"
            } else {
                "bear_stack"
            };
            out.push(ev(kind, snap.stack, c.close));
        }
    }
}

impl Detector for EmaState {
    fn names(&self) -> &'static [&'static str] {
        &["ema"]
    }

    fn update(&mut self, i: u32, c: &Candle, out: &mut Vec<Event>) {
        self.step(i, c, out);
    }
}

/// Trendline parameters (T5).
#[derive(Debug, Clone, Copy)]
pub struct TrendlineParams {
    pub swing: SwingParams,
    /// Touch / break tolerance in ATR. Default 0.25; sensible 0.1..=0.5.
    pub tol_atr: f64,
    /// Minimum and maximum bars between the two anchor swings. Defaults 5 / 300.
    pub min_sep: u32,
    pub max_span: u32,
    /// A line is retired this many bars after its second anchor. Default 400; sensible 100..=1000.
    pub max_age: u32,
}

impl Default for TrendlineParams {
    fn default() -> Self {
        TrendlineParams {
            swing: SwingParams::default(),
            tol_atr: 0.25,
            min_sep: 5,
            max_span: 300,
            max_age: 400,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Line {
    a_idx: u32,
    a: f64,
    b_idx: u32,
    b: f64,
    touches: u16,
    away: bool,
    alive: bool,
}

impl Line {
    fn at(&self, t: u32) -> f64 {
        let dx = f64::from(self.b_idx) - f64::from(self.a_idx);
        self.a + (self.b - self.a) * (f64::from(t) - f64::from(self.a_idx)) / dx
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Hb {
    idx: u32,
    h: f64,
    l: f64,
    c: f64,
}

/// Trendlines from swing points (T5). Up line: through the last two confirmed swing lows when
/// the second is higher, `min_sep..=max_span` bars apart, with no close more than `tol` below
/// the line in between. Down line mirrored on swing highs.
///
/// Detector `trendline`: `tl_up_formed` / `tl_down_formed` (when the second anchor is confirmed),
/// `tl_up_touch` / `tl_down_touch` (third and later touches: a bar reaches within `tol` of the line
/// after having been away from it and closes on the right side; `value` = touch number),
/// `tl_up_break` / `tl_down_break` (close beyond the line by `tol`). `ax,ay`-`bx,by` = anchors,
/// `price` = line value at the event bar.
#[derive(Debug, Clone)]
pub struct Trendline {
    p: TrendlineParams,
    swings: Swings,
    atr: Atr,
    bars: Ring<Hb>,
    up: Line,
    down: Line,
}

impl Trendline {
    pub fn new(p: TrendlineParams) -> Trendline {
        Trendline {
            p,
            swings: Swings::new(p.swing),
            atr: Atr::new(14),
            bars: Ring::new(p.max_span as usize + 4 * p.swing.k + 8),
            up: Line::default(),
            down: Line::default(),
        }
    }

    fn valid(&self, l: &Line, up: bool, tol: f64) -> bool {
        self.bars.iter().filter(|b| b.idx >= l.a_idx).all(|b| {
            if up {
                b.c >= l.at(b.idx) - tol
            } else {
                b.c <= l.at(b.idx) + tol
            }
        })
    }

    fn try_new(&mut self, i: u32, p: Pivot, q: Option<Pivot>, tol: f64, out: &mut Vec<Event>) {
        let Some(q) = q else { return };
        let up = !p.is_high;
        let rising = p.price > q.price;
        if up != rising {
            return;
        }
        let sep = p.idx - q.idx;
        if sep < self.p.min_sep || sep > self.p.max_span {
            return;
        }
        let Some(oldest) = self.bars.get(0) else {
            return;
        };
        if oldest.idx > q.idx {
            return;
        }
        let mut l = Line {
            a_idx: q.idx,
            a: q.price,
            b_idx: p.idx,
            b: p.price,
            touches: 2,
            away: false,
            alive: true,
        };
        if !self.valid(&l, up, tol) {
            return;
        }
        if let Some(b) = self.bars.back(0) {
            l.away = if up {
                b.l > l.at(i) + tol
            } else {
                b.h < l.at(i) - tol
            };
        }
        let (kind, dir) = if up {
            ("tl_up_formed", 1)
        } else {
            ("tl_down_formed", -1)
        };
        out.push(
            Event::new("trendline", kind, dir, i)
                .with_formed(p.idx)
                .with_start(q.idx)
                .with_price(l.at(i))
                .with_a(q.idx, q.price)
                .with_b(p.idx, p.price),
        );
        if up {
            self.up = l;
        } else {
            self.down = l;
        }
    }

    fn track(l: &mut Line, up: bool, i: u32, c: &Candle, tol: f64, age: u32, out: &mut Vec<Event>) {
        if !l.alive {
            return;
        }
        if i - l.b_idx > age {
            l.alive = false;
            return;
        }
        let v = l.at(i);
        let ev = |kind, dir: i8, l: &Line| {
            Event::new("trendline", kind, dir, i)
                .with_start(l.a_idx)
                .with_price(v)
                .with_a(l.a_idx, l.a)
                .with_b(l.b_idx, l.b)
                .with_value(f64::from(l.touches))
        };
        if up {
            if c.close < v - tol {
                l.alive = false;
                out.push(ev("tl_up_break", -1, l));
            } else if c.low <= v + tol {
                if l.away {
                    l.touches += 1;
                    l.away = false;
                    out.push(ev("tl_up_touch", 1, l));
                }
            } else {
                l.away = true;
            }
        } else if c.close > v + tol {
            l.alive = false;
            out.push(ev("tl_down_break", 1, l));
        } else if c.high >= v - tol {
            if l.away {
                l.touches += 1;
                l.away = false;
                out.push(ev("tl_down_touch", -1, l));
            }
        } else {
            l.away = true;
        }
    }

    pub fn step(&mut self, i: u32, c: &Candle, out: &mut Vec<Event>) {
        let a = self.atr.update(c);
        self.swings.step(i, c);
        self.bars.push(Hb {
            idx: i,
            h: c.high,
            l: c.low,
            c: c.close,
        });
        if !self.atr.ready() {
            return;
        }
        let tol = self.p.tol_atr * a;
        let age = self.p.max_age;
        Self::track(&mut self.up, true, i, c, tol, age, out);
        Self::track(&mut self.down, false, i, c, tol, age, out);
        if let Some(p) = self.swings.new_low {
            let q = self.swings.low(1);
            self.try_new(i, p, q, tol, out);
        }
        if let Some(p) = self.swings.new_high {
            let q = self.swings.high(1);
            self.try_new(i, p, q, tol, out);
        }
    }
}

impl Detector for Trendline {
    fn names(&self) -> &'static [&'static str] {
        &["trendline"]
    }

    fn update(&mut self, i: u32, c: &Candle, out: &mut Vec<Event>) {
        self.step(i, c, out);
    }
}
