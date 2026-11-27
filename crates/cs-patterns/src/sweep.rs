//! Liquidity pools and sweeps (knowledge S5, T11): equal highs/lows, previous UTC day and week
//! high/low; a wick beyond the pool followed by a close back inside (reclaim).

use cs_core::time::day_start;
use cs_core::{Candle, Event};

use crate::Detector;
use crate::swing::{SwingParams, Swings};
use crate::util::{Atr, bounded_push};

/// Pool source flags (`tags`).
pub const POOL_EQ: u32 = 1;
pub const POOL_DAY: u32 = 2;
pub const POOL_WEEK: u32 = 4;

const DAY_MS: i64 = 86_400_000;
const WEEK_MS: i64 = 7 * DAY_MS;
/// 1970-01-05 (the first Monday after the epoch) in ms.
const MONDAY0: i64 = 4 * DAY_MS;

/// UTC week start (Monday 00:00) containing `t`.
pub fn week_start(t: i64) -> i64 {
    (t - MONDAY0).div_euclid(WEEK_MS) * WEEK_MS + MONDAY0
}

/// Sweep parameters.
#[derive(Debug, Clone, Copy)]
pub struct SweepParams {
    pub swing: SwingParams,
    /// Two swing points are "equal" within this many ATR (S5). Default 0.15; sensible 0.05..=0.3.
    pub eq_tol_atr: f64,
    /// Max bars between the two equal swings. Default 200; sensible 20..=500.
    pub eq_max_sep: u32,
    /// The wick must pass the pool by more than this many ATR. Default 0.02; sensible 0..=0.2.
    pub eps_atr: f64,
    /// Bars allowed for the reclaim close, counting the sweep bar. Default 3; sensible 1..=5.
    pub reclaim_bars: u32,
    pub use_day: bool,
    pub use_week: bool,
}

impl Default for SweepParams {
    fn default() -> Self {
        SweepParams {
            swing: SwingParams::default(),
            eq_tol_atr: 0.15,
            eq_max_sep: 200,
            eps_atr: 0.02,
            reclaim_bars: 3,
            use_day: true,
            use_week: true,
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Pool {
    price: f64,
    /// +1 = pool of highs (buy stops above), -1 = pool of lows.
    side: i8,
    tag: u32,
    created: u32,
    start: u32,
    /// Bar of the first pierce, `u32::MAX` if none yet.
    pierced: u32,
    extreme: f64,
}

#[derive(Debug, Clone, Copy, Default)]
struct Single {
    idx: u32,
    price: f64,
}

/// Liquidity pools and sweeps.
///
/// * Detector `eq_levels`: `eq_highs` / `eq_lows` when a new confirmed swing is within
///   `eq_tol_atr` of an earlier, still untouched swing on the same side (`ax`/`bx` = the two swings,
///   `price` = the pool: max of the highs / min of the lows).
/// * Detector `sweep`: `sweep_high` (dir -1) / `sweep_low` (dir +1): a bar trades beyond a live
///   pool by more than `eps_atr`, and the same bar or one of the next `reclaim_bars - 1` bars
///   closes back inside. If no reclaim comes in time the pool is treated as broken and dropped.
///   One event per side per bar; `tags` = pool sources swept, `price` = the furthest pool swept,
///   `lo` / `hi` = wick extreme, `value` = wick depth beyond the pool in ATR.
#[derive(Debug, Clone)]
pub struct Sweeps {
    p: SweepParams,
    swings: Swings,
    atr: Atr,
    highs: Vec<Single>,
    lows: Vec<Single>,
    pools: Vec<Pool>,
    day: i64,
    day_hl: (f64, f64, u32),
    week: i64,
    week_hl: (f64, f64, u32),
}

const MAX_SINGLES: usize = 16;
const MAX_POOLS: usize = 48;

impl Sweeps {
    pub fn new(p: SweepParams) -> Sweeps {
        Sweeps {
            p,
            swings: Swings::new(p.swing),
            atr: Atr::new(14),
            highs: Vec::with_capacity(MAX_SINGLES),
            lows: Vec::with_capacity(MAX_SINGLES),
            pools: Vec::with_capacity(MAX_POOLS),
            day: i64::MIN,
            day_hl: (f64::NAN, f64::NAN, 0),
            week: i64::MIN,
            week_hl: (f64::NAN, f64::NAN, 0),
        }
    }

    fn add_pool(&mut self, pool: Pool) {
        bounded_push(&mut self.pools, MAX_POOLS, pool, |v| {
            v.iter()
                .enumerate()
                .min_by_key(|(_, p)| p.created)
                .map_or(0, |(k, _)| k)
        });
    }

    fn periods(&mut self, i: u32, c: &Candle) {
        let d = day_start(c.open_time);
        if d == self.day {
            self.day_hl.0 = self.day_hl.0.max(c.high);
            self.day_hl.1 = self.day_hl.1.min(c.low);
        } else {
            if self.day != i64::MIN && self.p.use_day && !self.day_hl.0.is_nan() {
                self.pools.retain(|p| p.tag != POOL_DAY);
                let (h, l, s) = self.day_hl;
                self.add_pool(new_pool(h, 1, POOL_DAY, i.wrapping_sub(1), s));
                self.add_pool(new_pool(l, -1, POOL_DAY, i.wrapping_sub(1), s));
            }
            self.day = d;
            self.day_hl = (c.high, c.low, i);
        }
        let w = week_start(c.open_time);
        if w == self.week {
            self.week_hl.0 = self.week_hl.0.max(c.high);
            self.week_hl.1 = self.week_hl.1.min(c.low);
        } else {
            if self.week != i64::MIN && self.p.use_week && !self.week_hl.0.is_nan() {
                self.pools.retain(|p| p.tag != POOL_WEEK);
                let (h, l, s) = self.week_hl;
                self.add_pool(new_pool(h, 1, POOL_WEEK, i.wrapping_sub(1), s));
                self.add_pool(new_pool(l, -1, POOL_WEEK, i.wrapping_sub(1), s));
            }
            self.week = w;
            self.week_hl = (c.high, c.low, i);
        }
    }

    fn equal_levels(&mut self, i: u32, c: &Candle, a: f64, out: &mut Vec<Event>) {
        // Singles that price has traded through are no longer liquidity.
        self.highs.retain(|s| c.high <= s.price);
        self.lows.retain(|s| c.low >= s.price);
        let tol = self.p.eq_tol_atr * a;
        let sep = self.p.eq_max_sep;
        for high in [true, false] {
            let pv = if high {
                self.swings.new_high
            } else {
                self.swings.new_low
            };
            let Some(pv) = pv else { continue };
            let list = if high {
                &mut self.highs
            } else {
                &mut self.lows
            };
            let hit = list
                .iter()
                .rev()
                .find(|s| pv.idx - s.idx <= sep && (s.price - pv.price).abs() <= tol)
                .copied();
            bounded_push(
                list,
                MAX_SINGLES,
                Single {
                    idx: pv.idx,
                    price: pv.price,
                },
                |_| 0,
            );
            if let Some(s) = hit {
                let (price, side, kind) = if high {
                    (s.price.max(pv.price), 1, "eq_highs")
                } else {
                    (s.price.min(pv.price), -1, "eq_lows")
                };
                out.push(
                    Event::new("eq_levels", kind, -side, i)
                        .with_formed(pv.idx)
                        .with_start(s.idx)
                        .with_price(price)
                        .with_a(s.idx, s.price)
                        .with_b(pv.idx, pv.price),
                );
                self.add_pool(new_pool(price, side, POOL_EQ, i, s.idx));
            }
        }
    }

    fn sweeps(&mut self, i: u32, c: &Candle, a: f64, out: &mut Vec<Event>) {
        let eps = self.p.eps_atr * a;
        let window = self.p.reclaim_bars.max(1);
        // (tags, furthest pool price, start, extreme) per side: [highs, lows]
        let mut hit: [(u32, f64, u32, f64); 2] = [(0, f64::NAN, 0, f64::NAN); 2];
        self.pools.retain_mut(|p| {
            if i <= p.created {
                return true;
            }
            let up = p.side > 0;
            if p.pierced == u32::MAX {
                let beyond = if up {
                    c.high > p.price + eps
                } else {
                    c.low < p.price - eps
                };
                if !beyond {
                    return true;
                }
                p.pierced = i;
                p.extreme = if up { c.high } else { c.low };
            } else {
                p.extreme = if up {
                    p.extreme.max(c.high)
                } else {
                    p.extreme.min(c.low)
                };
            }
            let back = if up {
                c.close < p.price
            } else {
                c.close > p.price
            };
            if back {
                let s = usize::from(!up);
                let h = &mut hit[s];
                let further = h.1.is_nan() || (up && p.price > h.1) || (!up && p.price < h.1);
                h.0 |= p.tag;
                if further {
                    h.1 = p.price;
                    h.2 = p.start;
                }
                h.3 = if h.3.is_nan() {
                    p.extreme
                } else if up {
                    h.3.max(p.extreme)
                } else {
                    h.3.min(p.extreme)
                };
                return false;
            }
            // Not reclaimed yet: keep waiting inside the window, else the pool broke.
            i - p.pierced + 1 < window
        });
        for (s, h) in hit.iter().enumerate() {
            if h.0 == 0 {
                continue;
            }
            let up = s == 0;
            let (kind, dir, lo, hi) = if up {
                ("sweep_high", -1, h.1, h.3)
            } else {
                ("sweep_low", 1, h.3, h.1)
            };
            out.push(
                Event::new("sweep", kind, dir, i)
                    .with_start(h.2)
                    .with_price(h.1)
                    .with_zone(lo, hi)
                    .with_tags(h.0)
                    .with_value((h.3 - h.1).abs() / a),
            );
        }
    }

    pub fn step(&mut self, i: u32, c: &Candle, out: &mut Vec<Event>) {
        let a = self.atr.get();
        self.atr.update(c);
        self.swings.step(i, c);
        // The previous day's / week's high and low use only earlier bars, so the first bar of the
        // new period may already sweep them.
        self.periods(i, c);
        if !a.is_nan() {
            self.sweeps(i, c, a, out);
            self.equal_levels(i, c, a, out);
        }
    }
}

fn new_pool(price: f64, side: i8, tag: u32, created: u32, start: u32) -> Pool {
    Pool {
        price,
        side,
        tag,
        created,
        start,
        pierced: u32::MAX,
        extreme: f64::NAN,
    }
}

impl Detector for Sweeps {
    fn names(&self) -> &'static [&'static str] {
        &["eq_levels", "sweep"]
    }

    fn update(&mut self, i: u32, c: &Candle, out: &mut Vec<Event>) {
        self.step(i, c, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cs_core::time::ymd_hms_to_ms;

    #[test]
    fn week_starts_monday() {
        // 2026-11-27 is a Friday; its week starts Monday 2026-11-23.
        let t = ymd_hms_to_ms(2026, 11, 27, 13, 0, 0);
        assert_eq!(week_start(t), ymd_hms_to_ms(2026, 11, 23, 0, 0, 0));
        let m = ymd_hms_to_ms(2026, 11, 23, 0, 0, 0);
        assert_eq!(week_start(m), m);
    }
}
