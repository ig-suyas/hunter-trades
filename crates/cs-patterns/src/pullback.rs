//! Pullback to value with a trigger candle (knowledge T6).

use cs_core::{Candle, Event};

use crate::Detector;
use crate::fib::{Fib, FibParams};
use crate::swing::{SwingParams, Swings};
use crate::trend::{EmaParams, EmaState};
use crate::util::{Atr, Ring};
use crate::zones::{SdParams, SdZones, SrParams, SrZones, band_touch};

/// Value-type flags (`tags`).
pub const VAL_EMA: u32 = 1;
pub const VAL_FIB: u32 = 2;
pub const VAL_FLIP: u32 = 4;
pub const VAL_DEMAND: u32 = 8;
/// Flag (`tags` bit): the pullback is "healthy" (smaller and slower than the impulse before it and
/// no close beyond the last swing). See [`Pullback`].
pub const HEALTHY: u32 = 16;

/// Pullback parameters.
#[derive(Debug, Clone, Copy)]
pub struct PullbackParams {
    pub ema: EmaParams,
    pub fib: FibParams,
    pub sr: SrParams,
    pub sd: SdParams,
    /// Touch tolerance in ATR. Default 0.25; sensible 0.1..=0.5.
    pub tol_atr: f64,
    /// A value touch counts for this many bars (touch bar included). Default 4; sensible 1..=8.
    pub touch_bars: usize,
    /// Minimum pullback depth: highest high of the last `swing_bars` bars minus the lowest low of
    /// the last `touch_bars` bars, in ATR. Defaults 1.0 ATR / 20 bars.
    pub min_depth_atr: f64,
    pub swing_bars: usize,
    /// Bars between two signals in the same direction. Default 10.
    pub cooldown: u32,
    /// Healthy-pullback rule: the impulse origin is the lowest low (long) within this many bars
    /// before the pullback top. Default 40; sensible 20..=100.
    pub impulse_bars: usize,
    /// A healthy pullback must be smaller than this share of the impulse. Default 1.0 ("smaller
    /// than the preceding move"); sensible 0.5..=1.0.
    pub healthy_max_ratio: f64,
    /// Swings used for "does not close beyond the last swing". Default k = 3.
    pub swing: SwingParams,
}

impl Default for PullbackParams {
    fn default() -> Self {
        PullbackParams {
            ema: EmaParams::default(),
            fib: FibParams::default(),
            sr: SrParams::default(),
            sd: SdParams::default(),
            tol_atr: 0.25,
            touch_bars: 4,
            min_depth_atr: 1.0,
            swing_bars: 20,
            cooldown: 10,
            impulse_bars: 40,
            healthy_max_ratio: 1.0,
            swing: SwingParams::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Hb {
    idx: u32,
    h: f64,
    l: f64,
    c: f64,
    long_tags: u32,
    short_tags: u32,
}

/// Pullback continuation (T6), long side (short mirrored):
/// 1. trend: EMA20 > EMA50 and EMA50 rising (slope over 5 bars > 0);
/// 2. value touched within the last `touch_bars` bars: the EMA20-EMA50 band, the 38.2-61.8 %
///    zone of the active Fibonacci up-leg, an S/R zone that flipped from resistance to support,
///    or a live demand zone tested at most once (all with `tol_atr`);
/// 3. a real pullback: depth >= `min_depth_atr`;
/// 4. trigger on this bar: bull bar closing above the previous high, or a bull rejection bar
///    (lower wick >= half the range and close in the top third).
///
/// Healthy-pullback flag (`tags` bit [`HEALTHY`], T6 condition 3), long side: top `B` = highest
/// high of the `swing_bars - 1` bars before the trigger bar (latest if tied); impulse origin `A` =
/// lowest low of the `impulse_bars` bars before `B`; pullback = from `B` to the trigger bar, depth
/// `P` = high of `B` minus the lowest low since `B`. Healthy when
/// (a) smaller: `P < healthy_max_ratio x (B - A)`,
/// (b) slower: `P / bars(B..trigger) < (B - A) / bars(A..B)` (price moved per bar),
/// (c) no close since `B` below the last confirmed swing low (strength `swing.k`) that lies
///     before `B`; with no such swing the flag is not set.
/// `ax,ay` = A and `bx,by` = B whenever the impulse could be measured (else unset).
///
/// Detector `pullback`: `pullback_long` / `pullback_short`. `price` = trigger close, `lo` =
/// pullback low (stop reference), `hi` = recent swing high (target reference), `tags` = value
/// types touched, `value` = depth in ATR.
#[derive(Debug, Clone)]
pub struct Pullback {
    p: PullbackParams,
    atr: Atr,
    ema: EmaState,
    fib: Fib,
    sr: SrZones,
    sd: SdZones,
    scratch: Vec<Event>,
    bars: Ring<Hb>,
    swings: Swings,
    last: [u32; 2],
}

/// Impulse / pullback measurement for the healthy flag.
#[derive(Debug, Clone, Copy)]
struct Impulse {
    a_idx: u32,
    a: f64,
    b_idx: u32,
    b: f64,
    healthy: bool,
}

impl Pullback {
    pub fn new(p: PullbackParams) -> Pullback {
        Pullback {
            p,
            atr: Atr::new(14),
            ema: EmaState::new(p.ema),
            fib: Fib::new(p.fib),
            sr: SrZones::new(p.sr),
            sd: SdZones::new(p.sd),
            scratch: Vec::with_capacity(32),
            bars: Ring::new(p.swing_bars.max(p.touch_bars).max(2) + p.impulse_bars + 2),
            swings: Swings::new(p.swing),
            last: [u32::MAX; 2],
        }
    }

    fn tags(&self, c: &Candle, tol: f64) -> (u32, u32) {
        let e = self.ema.snap();
        let (mut lt, mut st) = (0, 0);
        if e.ready {
            let (lo, hi) = (e.fast.min(e.mid), e.fast.max(e.mid));
            if band_touch(c.low, lo, hi, tol) {
                lt |= VAL_EMA;
            }
            if band_touch(c.high, lo, hi, tol) {
                st |= VAL_EMA;
            }
        }
        if let Some((lo, hi)) = self.fib.zone_up()
            && band_touch(c.low, lo, hi, tol)
        {
            lt |= VAL_FIB;
        }
        if let Some((lo, hi)) = self.fib.zone_down()
            && band_touch(c.high, lo, hi, tol)
        {
            st |= VAL_FIB;
        }
        for z in self.sr.zones().filter(|z| z.flips > 0) {
            if z.role > 0 && band_touch(c.low, z.lo, z.hi, tol) {
                lt |= VAL_FLIP;
            }
            if z.role < 0 && band_touch(c.high, z.lo, z.hi, tol) {
                st |= VAL_FLIP;
            }
        }
        for z in self.sd.zones().iter().filter(|z| z.tests <= 1) {
            if z.dir > 0 && band_touch(c.low, z.lo, z.hi, tol) {
                lt |= VAL_DEMAND;
            }
            if z.dir < 0 && band_touch(c.high, z.lo, z.hi, tol) {
                st |= VAL_DEMAND;
            }
        }
        (lt, st)
    }

    /// Healthy-pullback test at trigger bar `i` (the newest bar in the ring). `up` = long side.
    fn impulse(&self, i: u32, up: bool) -> Option<Impulse> {
        let sb = self.p.swing_bars.max(2);
        // Top B (bottom for shorts) among the bars before the trigger bar.
        let mut best: Option<Hb> = None;
        for k in 1..sb.min(self.bars.len()) {
            let b = self.bars.back(k)?;
            if best.is_none_or(|x| if up { b.h > x.h } else { b.l < x.l }) {
                best = Some(b);
            }
        }
        let top = best?;
        let kb = (i - top.idx) as usize;
        // Origin A: opposite extreme of the `impulse_bars` bars before B.
        let mut org: Option<Hb> = None;
        for k in kb + 1..=kb + self.p.impulse_bars {
            let Some(b) = self.bars.back(k) else { break };
            if org.is_none_or(|x| if up { b.l < x.l } else { b.h > x.h }) {
                org = Some(b);
            }
        }
        let org = org?;
        let (a, b) = if up { (org.l, top.h) } else { (org.h, top.l) };
        let imp = (b - a).abs();
        let imp_bars = f64::from(top.idx - org.idx);
        // Pullback from B to the trigger bar (inclusive).
        let (mut ext, mut worst_close) = if up {
            (f64::INFINITY, f64::INFINITY)
        } else {
            (f64::NEG_INFINITY, f64::NEG_INFINITY)
        };
        for k in 0..=kb {
            let x = self.bars.back(k)?;
            if up {
                ext = ext.min(x.l);
                worst_close = worst_close.min(x.c);
            } else {
                ext = ext.max(x.h);
                worst_close = worst_close.max(x.c);
            }
        }
        let pb = (b - ext).abs();
        let pb_bars = f64::from(i - top.idx);
        let smaller = imp > 0.0 && pb < self.p.healthy_max_ratio * imp;
        let slower = pb_bars > 0.0 && imp_bars > 0.0 && pb / pb_bars < imp / imp_bars;
        // Last confirmed swing on the protective side that lies before B.
        let swing = (0..16)
            .map_while(|n| {
                if up {
                    self.swings.low(n)
                } else {
                    self.swings.high(n)
                }
            })
            .find(|p| p.idx < top.idx);
        let holds = swing.is_some_and(|s| {
            if up {
                worst_close >= s.price
            } else {
                worst_close <= s.price
            }
        });
        Some(Impulse {
            a_idx: org.idx,
            a,
            b_idx: top.idx,
            b,
            healthy: smaller && slower && holds,
        })
    }

    pub fn step(&mut self, i: u32, c: &Candle, out: &mut Vec<Event>) {
        let a = self.atr.update(c);
        self.scratch.clear();
        self.ema.step(i, c, &mut self.scratch);
        self.fib.step(i, c, &mut self.scratch);
        self.sr.step(i, c, &mut self.scratch);
        self.sd.step(i, c, &mut self.scratch);
        self.swings.step(i, c);
        let prev = self.bars.back(0);
        let tol = self.p.tol_atr * a;
        let (lt, st) = self.tags(c, tol);
        self.bars.push(Hb {
            idx: i,
            h: c.high,
            l: c.low,
            c: c.close,
            long_tags: lt,
            short_tags: st,
        });
        let e = self.ema.snap();
        let Some(prev) = prev else { return };
        if !e.ready || !self.atr.ready() {
            return;
        }
        let tb = self.p.touch_bars.max(1);
        let recent = |k: usize| k < tb;
        let mut ltags = 0;
        let mut stags = 0;
        let (mut pb_lo, mut pb_hi) = (f64::INFINITY, f64::NEG_INFINITY);
        let (mut sw_hi, mut sw_lo) = (f64::NEG_INFINITY, f64::INFINITY);
        for k in 0..self.bars.len().min(tb.max(self.p.swing_bars)) {
            let Some(b) = self.bars.back(k) else { break };
            if recent(k) {
                ltags |= b.long_tags;
                stags |= b.short_tags;
                pb_lo = pb_lo.min(b.l);
                pb_hi = pb_hi.max(b.h);
            }
            if k < self.p.swing_bars {
                sw_hi = sw_hi.max(b.h);
                sw_lo = sw_lo.min(b.l);
            }
        }
        let rng = c.high - c.low;
        let cd = self.p.cooldown;
        let last = self.last;
        let ready = |slot: usize| last[slot] == u32::MAX || i - last[slot] >= cd;
        // Long.
        let trend_up = e.fast > e.mid && e.slope_mid > 0.0;
        let bull_trigger = c.close > c.open
            && (c.close > prev.h
                || (rng > 0.0
                    && c.open.min(c.close) - c.low >= 0.5 * rng
                    && c.close >= c.low + rng * 2.0 / 3.0));
        let depth_l = (sw_hi - pb_lo) / a;
        if trend_up && ltags != 0 && bull_trigger && depth_l >= self.p.min_depth_atr && ready(0) {
            self.last[0] = i;
            out.push(self.event(i, 1, tb, c.close, (pb_lo, sw_hi), ltags, depth_l));
        }
        // Short.
        let trend_dn = e.fast < e.mid && e.slope_mid < 0.0;
        let bear_trigger = c.close < c.open
            && (c.close < prev.l
                || (rng > 0.0
                    && c.high - c.open.max(c.close) >= 0.5 * rng
                    && c.close <= c.high - rng * 2.0 / 3.0));
        let depth_s = (pb_hi - sw_lo) / a;
        if trend_dn && stags != 0 && bear_trigger && depth_s >= self.p.min_depth_atr && ready(1) {
            self.last[1] = i;
            out.push(self.event(i, -1, tb, c.close, (sw_lo, pb_hi), stags, depth_s));
        }
    }
}

impl Pullback {
    #[allow(clippy::too_many_arguments)]
    fn event(
        &self,
        i: u32,
        dir: i8,
        tb: usize,
        close: f64,
        zone: (f64, f64),
        tags: u32,
        depth: f64,
    ) -> Event {
        let kind = if dir > 0 {
            "pullback_long"
        } else {
            "pullback_short"
        };
        let mut e = Event::new("pullback", kind, dir, i)
            .with_start(i + 1 - tb.min(self.bars.len()) as u32)
            .with_price(close)
            .with_zone(zone.0, zone.1)
            .with_tags(tags)
            .with_value(depth);
        if let Some(m) = self.impulse(i, dir > 0) {
            e = e.with_a(m.a_idx, m.a).with_b(m.b_idx, m.b);
            if m.healthy {
                e.tags |= HEALTHY;
            }
        }
        e
    }
}

impl Detector for Pullback {
    fn names(&self) -> &'static [&'static str] {
        &["pullback"]
    }

    fn update(&mut self, i: u32, c: &Candle, out: &mut Vec<Event>) {
        self.step(i, c, out);
    }
}
