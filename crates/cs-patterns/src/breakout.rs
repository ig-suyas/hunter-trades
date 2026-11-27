//! Ranges (knowledge S10), breakouts with expansion (T8), breakout retests (T9) and fakeouts
//! (T10). Levels come from range boxes and from S/R zones.

use cs_core::{Candle, Event};

use crate::Detector;
use crate::util::{Atr, Ring, RollStats, bounded_push};
use crate::zones::{SrParams, SrZones};

/// `tags` bit: the level came from a range box.
pub const TAG_RANGE: u32 = 1;
/// `tags` bit: the level came from an S/R zone.
pub const TAG_SR: u32 = 2;

/// Range / breakout parameters.
#[derive(Debug, Clone, Copy)]
pub struct BreakoutParams {
    /// Range window in bars. Default 40; sensible 20..=100.
    pub range_bars: usize,
    /// Range height limit in ATR. Default 5.0; sensible 3..=8.
    pub range_max_atr: f64,
    /// Touch tolerance for range edges in ATR. Default 0.25; sensible 0.1..=0.5.
    pub touch_atr: f64,
    /// Separate touches needed at each edge. Default 2; sensible 2..=4.
    pub min_touches: usize,
    /// A box expires this many bars after it formed. Default 200.
    pub box_max_age: u32,
    /// Close must clear the level by this many ATR. Default 0.1; sensible 0.0..=0.5.
    pub clear_atr: f64,
    /// Breakout bar range and volume vs the 20-bar means. Defaults 1.5 / 1.5; sensible 1.2..=3.
    pub range_mult: f64,
    pub vol_mult: f64,
    /// Retest window and tolerance (T9). Defaults 12 bars / 0.25 ATR.
    pub retest_bars: u32,
    pub retest_tol_atr: f64,
    /// Fakeout window (T10): close back inside within this many bars. Default 3; sensible 1..=5.
    pub fake_bars: u32,
    pub sr: SrParams,
}

impl Default for BreakoutParams {
    fn default() -> Self {
        BreakoutParams {
            range_bars: 40,
            range_max_atr: 5.0,
            touch_atr: 0.25,
            min_touches: 2,
            box_max_age: 200,
            clear_atr: 0.1,
            range_mult: 1.5,
            vol_mult: 1.5,
            retest_bars: 12,
            retest_tol_atr: 0.25,
            fake_bars: 3,
            sr: SrParams::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Hb {
    idx: u32,
    h: f64,
    l: f64,
}

#[derive(Debug, Clone, Copy, Default)]
struct BoxState {
    top: f64,
    bot: f64,
    start: u32,
    formed: u32,
    active: bool,
}

#[derive(Debug, Clone, Copy, Default)]
struct Pending {
    dir: i8,
    level: f64,
    lo: f64,
    hi: f64,
    brk_idx: u32,
    start: u32,
    tags: u32,
    touched: bool,
    extreme: f64,
}

/// Ranges, breakouts, retests and fakeouts.
///
/// * Detector `range`: `range_formed` when the last `range_bars` bars fit in `range_max_atr` x ATR
///   with `min_touches` separate touches (>= 3 bars apart) of the top and of the bottom, no box is
///   active and the window shares no bar with the previous box.
///   `lo..hi` = box.
/// * Detector `breakout`: `breakout_up` / `breakout_down`: close beyond a level (active box edge,
///   or an S/R zone breaking against its role) by `clear_atr`, on a bar whose range is
///   `range_mult` x and volume `vol_mult` x their 20-bar means. `price` = level, `value` =
///   volume ratio, `tags` = level source.
/// * Detector `retest`: `retest_long` / `retest_short`: within `retest_bars` after a breakout,
///   price comes back within tolerance of the level without closing back through it, then a bar
///   closes in the breakout direction beyond the level.
/// * Detector `fakeout`: `fakeout_up_failed` (dir -1) / `fakeout_down_failed` (dir +1): any close
///   beyond a level (no expansion needed) followed within `fake_bars` by a close back inside.
///   `hi` / `lo` = extreme reached outside.
#[derive(Debug, Clone)]
pub struct Breakouts {
    p: BreakoutParams,
    atr: Atr,
    bars: Ring<Hb>,
    rng_stats: RollStats,
    vol_stats: RollStats,
    bx: BoxState,
    sr: SrZones,
    scratch: Vec<Event>,
    retests: Vec<Pending>,
    fakes: Vec<Pending>,
}

/// Broken-level candidate: (level, zone lo, zone hi, start, tags).
type Cand = (f64, f64, f64, u32, u32);

const MAX_PENDING: usize = 16;
/// Minimum bars between two counted touches of a range edge.
const TOUCH_GAP: u32 = 3;

impl Breakouts {
    pub fn new(p: BreakoutParams) -> Breakouts {
        Breakouts {
            p,
            atr: Atr::new(14),
            bars: Ring::new(p.range_bars.max(4)),
            rng_stats: RollStats::new(20),
            vol_stats: RollStats::new(20),
            bx: BoxState::default(),
            sr: SrZones::new(p.sr),
            scratch: Vec::with_capacity(16),
            retests: Vec::with_capacity(MAX_PENDING),
            fakes: Vec::with_capacity(MAX_PENDING),
        }
    }

    /// Separate touches of an edge: touching bars counted greedily at least `TOUCH_GAP` bars
    /// apart (so one wide bar or a short run counts once, a long flat edge several times).
    fn touches(&self, top: bool, edge: f64, tol: f64) -> usize {
        let mut n = 0;
        let mut last: Option<u32> = None;
        for b in self.bars.iter() {
            let t = if top {
                b.h >= edge - tol
            } else {
                b.l <= edge + tol
            };
            if t && last.is_none_or(|l| b.idx - l >= TOUCH_GAP) {
                n += 1;
                last = Some(b.idx);
            }
        }
        n
    }

    fn find_box(&mut self, i: u32, a: f64, out: &mut Vec<Event>) {
        if self.bx.active || !self.bars.is_full() {
            return;
        }
        // A new box may not reuse bars of the previous one.
        let Some(first) = self.bars.get(0) else {
            return;
        };
        if self.bx.formed != 0 && first.idx <= self.bx.formed {
            return;
        }
        let top = self
            .bars
            .iter()
            .map(|b| b.h)
            .fold(f64::NEG_INFINITY, f64::max);
        let bot = self.bars.iter().map(|b| b.l).fold(f64::INFINITY, f64::min);
        if top - bot > self.p.range_max_atr * a {
            return;
        }
        let tol = self.p.touch_atr * a;
        if self.touches(true, top, tol) < self.p.min_touches
            || self.touches(false, bot, tol) < self.p.min_touches
        {
            return;
        }
        self.bx = BoxState {
            top,
            bot,
            start: first.idx,
            formed: i,
            active: true,
        };
        out.push(
            Event::new("range", "range_formed", 0, i)
                .with_start(first.idx)
                .with_price(f64::midpoint(top, bot))
                .with_zone(bot, top)
                .with_value((top - bot) / a),
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn on_break(
        &mut self,
        i: u32,
        c: &Candle,
        dir: i8,
        level: f64,
        zone: (f64, f64),
        start: u32,
        tags: u32,
        expansion: Option<f64>,
        out: &mut Vec<Event>,
    ) {
        let pend = Pending {
            dir,
            level,
            lo: zone.0,
            hi: zone.1,
            brk_idx: i,
            start,
            tags,
            touched: false,
            extreme: if dir > 0 { c.high } else { c.low },
        };
        bounded_push(&mut self.fakes, MAX_PENDING, pend, |_| 0);
        if let Some(vr) = expansion {
            let kind = if dir > 0 {
                "breakout_up"
            } else {
                "breakout_down"
            };
            out.push(
                Event::new("breakout", kind, dir, i)
                    .with_start(start)
                    .with_price(level)
                    .with_zone(zone.0, zone.1)
                    .with_tags(tags)
                    .with_value(vr),
            );
            bounded_push(&mut self.retests, MAX_PENDING, pend, |_| 0);
        }
    }

    fn track_pending(&mut self, i: u32, c: &Candle, a: f64, out: &mut Vec<Event>) {
        let tol = self.p.retest_tol_atr * a;
        let rb = self.p.retest_bars;
        self.retests.retain_mut(|r| {
            if i <= r.brk_idx {
                return true;
            }
            if i - r.brk_idx > rb {
                return false;
            }
            let up = r.dir > 0;
            let back_inside = if up {
                c.close < r.level
            } else {
                c.close > r.level
            };
            if back_inside {
                return false;
            }
            if (up && c.low <= r.level + tol) || (!up && c.high >= r.level - tol) {
                r.touched = true;
            }
            let trigger = if up {
                c.close > c.open && c.close > r.level
            } else {
                c.close < c.open && c.close < r.level
            };
            if r.touched && trigger {
                let kind = if up { "retest_long" } else { "retest_short" };
                out.push(
                    Event::new("retest", kind, r.dir, i)
                        .with_start(r.start)
                        .with_price(r.level)
                        .with_zone(r.lo, r.hi)
                        .with_a(r.brk_idx, r.level)
                        .with_tags(r.tags),
                );
                return false;
            }
            true
        });
        let fb = self.p.fake_bars;
        self.fakes.retain_mut(|f| {
            if i <= f.brk_idx {
                return true;
            }
            if i - f.brk_idx > fb {
                return false;
            }
            let up = f.dir > 0;
            f.extreme = if up {
                f.extreme.max(c.high)
            } else {
                f.extreme.min(c.low)
            };
            let back_inside = if up {
                c.close < f.level
            } else {
                c.close > f.level
            };
            if back_inside {
                let kind = if up {
                    "fakeout_up_failed"
                } else {
                    "fakeout_down_failed"
                };
                let (lo, hi) = if up {
                    (f.level, f.extreme)
                } else {
                    (f.extreme, f.level)
                };
                out.push(
                    Event::new("fakeout", kind, -f.dir, i)
                        .with_start(f.start)
                        .with_price(f.level)
                        .with_zone(lo, hi)
                        .with_a(f.brk_idx, f.level)
                        .with_tags(f.tags),
                );
                return false;
            }
            true
        });
    }

    pub fn step(&mut self, i: u32, c: &Candle, out: &mut Vec<Event>) {
        let a_prev = self.atr.get();
        let a = self.atr.update(c);
        let mean_rng = self.rng_stats.mean();
        let mean_vol = self.vol_stats.mean();
        self.rng_stats.push(c.high - c.low);
        self.vol_stats.push(c.volume);
        self.scratch.clear();
        self.sr.step(i, c, &mut self.scratch);
        if a_prev.is_nan() {
            self.bars.push(Hb {
                idx: i,
                h: c.high,
                l: c.low,
            });
            return;
        }
        self.track_pending(i, c, a, out);
        let expansion = (mean_rng > 0.0
            && mean_vol > 0.0
            && c.high - c.low >= self.p.range_mult * mean_rng
            && c.volume >= self.p.vol_mult * mean_vol)
            .then(|| c.volume / mean_vol);
        let clear = self.p.clear_atr * a_prev;
        // Candidate levels broken on this bar, one per direction: the outermost level cleared
        // (highest for up, lowest for down); sources are OR-ed into `tags`.
        // (level, zone lo, zone hi, start, tags)
        let mut cand: [Option<Cand>; 2] = [None, None];
        let mut offer = |dir: i8, level: f64, lo: f64, hi: f64, start: u32, tag: u32| {
            let slot = usize::from(dir < 0);
            let c = &mut cand[slot];
            match c {
                Some(x) => {
                    let further = if dir > 0 { level > x.0 } else { level < x.0 };
                    x.4 |= tag;
                    if further {
                        *x = (level, lo, hi, start, x.4);
                    }
                }
                None => *c = Some((level, lo, hi, start, tag)),
            }
        };
        // Range box edges.
        if self.bx.active {
            let b = self.bx;
            if c.close > b.top + clear {
                self.bx.active = false;
                offer(1, b.top, b.bot, b.top, b.start, TAG_RANGE);
            } else if c.close < b.bot - clear {
                self.bx.active = false;
                offer(-1, b.bot, b.bot, b.top, b.start, TAG_RANGE);
            } else if i - b.formed > self.p.box_max_age {
                self.bx.active = false;
            }
        }
        // S/R zone breaks (the zone detector already requires a close beyond by its own buffer).
        for e in &self.scratch {
            match e.kind {
                "zone_break_up" => offer(1, e.price, e.lo, e.hi, e.start_idx, TAG_SR),
                "zone_break_down" => offer(-1, e.price, e.lo, e.hi, e.start_idx, TAG_SR),
                _ => {}
            }
        }
        for (slot, x) in cand.iter().enumerate() {
            if let Some((level, lo, hi, start, tags)) = *x {
                let dir = if slot == 0 { 1 } else { -1 };
                self.on_break(i, c, dir, level, (lo, hi), start, tags, expansion, out);
            }
        }
        self.bars.push(Hb {
            idx: i,
            h: c.high,
            l: c.low,
        });
        self.find_box(i, a, out);
    }
}

impl Detector for Breakouts {
    fn names(&self) -> &'static [&'static str] {
        &["range", "breakout", "retest", "fakeout"]
    }

    fn update(&mut self, i: u32, c: &Candle, out: &mut Vec<Event>) {
        self.step(i, c, out);
    }
}
