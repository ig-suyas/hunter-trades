//! Zones: support/resistance zones with role reversal (knowledge S11, T3) and supply/demand
//! zones (T4). Zones are price bands, not lines.

use cs_core::{Candle, Event};

use crate::Detector;
use crate::swing::{Pivot, SwingParams, Swings};
use crate::util::{Atr, Ring, bounded_push};

// ---------------------------------------------------------------------------------------------
// Support / resistance
// ---------------------------------------------------------------------------------------------

/// S/R zone parameters.
#[derive(Debug, Clone, Copy)]
pub struct SrParams {
    /// Swing pivots that build the zones (default k = 3; sensible 2..=10).
    pub swing: SwingParams,
    /// A pivot joins a zone if its band is within this many ATR of the zone. Default 0.25;
    /// sensible 0.1..=0.5.
    pub tol_atr: f64,
    /// A zone may not be wider than this many ATR. Default 1.0; sensible 0.5..=2.0.
    pub max_width_atr: f64,
    /// Pivots needed before a candidate becomes a zone. Default 2; sensible 2..=4.
    pub min_touches: u16,
    /// A close beyond the zone by this many ATR breaks it (and flips its role). Default 0.1;
    /// sensible 0.0..=0.5.
    pub break_atr: f64,
    /// After a break, a retest that holds within this many bars is a role reversal. Default 60;
    /// sensible 10..=200.
    pub flip_window: u32,
    /// "Later revisited": after the break, a close at least this many ATR beyond the zone must
    /// come before the retest counts (otherwise it is chop around the level). Default 0.5;
    /// sensible 0.0..=2.0.
    pub flip_away_atr: f64,
    /// Zones with no new pivot touch for this many bars are dropped (unless a retest is
    /// pending). Default 1500; sensible 300..=5000.
    pub expire_bars: u32,
    /// A zone broken more than this many times is retired: a level that price keeps slicing
    /// through is not acting as support or resistance. Default 2 (one flip and one flip back);
    /// sensible 1..=4.
    pub max_flips: u16,
    /// Maximum zones (candidates included) tracked at once. Default 40.
    pub max_zones: usize,
}

impl Default for SrParams {
    fn default() -> Self {
        SrParams {
            swing: SwingParams::default(),
            tol_atr: 0.25,
            max_width_atr: 1.0,
            min_touches: 2,
            break_atr: 0.1,
            flip_window: 60,
            flip_away_atr: 0.5,
            expire_bars: 1500,
            max_flips: 2,
            max_zones: 40,
        }
    }
}

/// One S/R zone (or candidate).
#[derive(Debug, Clone, Copy, Default)]
pub struct SrZone {
    pub lo: f64,
    pub hi: f64,
    pub touches: u16,
    pub first_idx: u32,
    pub last_idx: u32,
    pub formed: bool,
    /// +1 support (price above), -1 resistance (price below), 0 not yet known.
    pub role: i8,
    /// Number of role flips so far (breaks through the zone).
    pub flips: u16,
    /// Bar of the latest break; a pending retest is checked after it.
    pub break_idx: u32,
    pub retest_pending: bool,
    /// Price has moved `flip_away_atr` beyond the zone since the latest break.
    pub left: bool,
}

/// Support/resistance zones from clustered swing pivots (S11). A pivot's band is the pivot bar's
/// wick..body extreme; pivots within `tol_atr` merge into one zone (width capped); a zone exists
/// once it has `min_touches` pivots.
///
/// Detector `sr_zone`: `zone_formed` (dir = role), `zone_break_up` / `zone_break_down` (close
/// beyond by `break_atr`; the role flips). Overlapping zones are merged; a zone broken more
/// than `max_flips` times is retired. Detector `role_reversal`: `flip_support` /
/// `flip_resistance` when, within `flip_window` bars of a break, price first closes
/// `flip_away_atr` beyond the zone, then comes back to it and the bar closes on the new side (T3). `lo..hi` = zone, `value` = touches.
#[derive(Debug, Clone)]
pub struct SrZones {
    p: SrParams,
    swings: Swings,
    atr: Atr,
    zones: Vec<SrZone>,
}

impl SrZones {
    pub fn new(p: SrParams) -> SrZones {
        SrZones {
            p,
            swings: Swings::new(p.swing),
            atr: Atr::new(14),
            zones: Vec::with_capacity(p.max_zones.max(1)),
        }
    }

    /// Formed zones.
    pub fn zones(&self) -> impl Iterator<Item = &SrZone> {
        self.zones.iter().filter(|z| z.formed)
    }

    fn add_pivot(&mut self, i: u32, pv: Pivot, close: f64, a: f64, out: &mut Vec<Event>) {
        let (blo, bhi) = if pv.is_high {
            (pv.body, pv.price)
        } else {
            (pv.price, pv.body)
        };
        let tol = self.p.tol_atr * a;
        let maxw = self.p.max_width_atr * a;
        let mut best: Option<(usize, f64)> = None;
        for (k, z) in self.zones.iter().enumerate() {
            if bhi < z.lo - tol || blo > z.hi + tol {
                continue;
            }
            if z.hi.max(bhi) - z.lo.min(blo) > maxw {
                continue;
            }
            let d = ((blo + bhi) - (z.lo + z.hi)).abs();
            if best.is_none_or(|(_, bd)| d < bd) {
                best = Some((k, d));
            }
        }
        if let Some((k, _)) = best {
            let was_formed = self.zones[k].formed;
            {
                let z = &mut self.zones[k];
                z.lo = z.lo.min(blo);
                z.hi = z.hi.max(bhi);
                z.touches += 1;
                z.last_idx = pv.idx;
            }
            let k = self.merge_overlaps(k, maxw);
            let z = &mut self.zones[k];
            if !was_formed && !z.formed && z.touches >= self.p.min_touches {
                z.formed = true;
                z.role = if close > z.hi {
                    1
                } else if close < z.lo || pv.is_high {
                    -1
                } else {
                    1
                };
                out.push(
                    Event::new("sr_zone", "zone_formed", z.role, i)
                        .with_formed(pv.idx)
                        .with_start(z.first_idx)
                        .with_price(f64::midpoint(z.lo, z.hi))
                        .with_zone(z.lo, z.hi)
                        .with_value(f64::from(z.touches)),
                );
            }
            return;
        }
        let z = SrZone {
            lo: blo,
            hi: bhi,
            touches: 1,
            first_idx: pv.idx,
            last_idx: pv.idx,
            ..SrZone::default()
        };
        // Evict the stalest candidate first, else the stalest zone.
        bounded_push(&mut self.zones, self.p.max_zones.max(1), z, |v| {
            v.iter()
                .enumerate()
                .min_by_key(|(_, z)| (z.formed, z.last_idx))
                .map_or(0, |(k, _)| k)
        });
    }

    /// Merge zones that now overlap zone `k` (if the union stays within `maxw`). Returns the new
    /// index of zone `k`.
    fn merge_overlaps(&mut self, mut k: usize, maxw: f64) -> usize {
        let mut j = 0;
        while j < self.zones.len() {
            if j == k {
                j += 1;
                continue;
            }
            let (a, b) = (self.zones[k], self.zones[j]);
            let overlap = b.lo <= a.hi && a.lo <= b.hi;
            if !overlap || a.hi.max(b.hi) - a.lo.min(b.lo) > maxw {
                j += 1;
                continue;
            }
            let z = &mut self.zones[k];
            z.lo = a.lo.min(b.lo);
            z.hi = a.hi.max(b.hi);
            z.touches = a.touches.saturating_add(b.touches);
            z.first_idx = a.first_idx.min(b.first_idx);
            z.last_idx = a.last_idx.max(b.last_idx);
            z.flips = a.flips.max(b.flips);
            if !a.formed && b.formed {
                z.formed = true;
                z.role = b.role;
                z.break_idx = b.break_idx;
                z.retest_pending = b.retest_pending;
                z.left = b.left;
            }
            self.zones.remove(j);
            if j < k {
                k -= 1;
            }
            j = 0;
        }
        k
    }

    pub fn step(&mut self, i: u32, c: &Candle, out: &mut Vec<Event>) {
        let a = self.atr.update(c);
        self.swings.step(i, c);
        if !self.atr.ready() {
            return;
        }
        // Existing zones react to this bar first (pivots confirmed now belong to older bars,
        // but a zone created now should not "break" on the bar that confirmed it).
        let tol = self.p.tol_atr * a;
        let brk = self.p.break_atr * a;
        let away = self.p.flip_away_atr * a;
        for z in self.zones.iter_mut().filter(|z| z.formed) {
            if z.retest_pending && i > z.break_idx {
                if i - z.break_idx > self.p.flip_window {
                    z.retest_pending = false;
                } else if !z.left {
                    z.left = (z.role > 0 && c.close >= z.hi + away)
                        || (z.role < 0 && c.close <= z.lo - away);
                } else if z.role > 0 && c.low <= z.hi + tol && c.close > z.hi {
                    z.retest_pending = false;
                    out.push(
                        Event::new("role_reversal", "flip_support", 1, i)
                            .with_start(z.first_idx)
                            .with_price(z.hi)
                            .with_zone(z.lo, z.hi)
                            .with_a(z.break_idx, z.hi)
                            .with_value(f64::from(z.touches)),
                    );
                } else if z.role < 0 && c.high >= z.lo - tol && c.close < z.lo {
                    z.retest_pending = false;
                    out.push(
                        Event::new("role_reversal", "flip_resistance", -1, i)
                            .with_start(z.first_idx)
                            .with_price(z.lo)
                            .with_zone(z.lo, z.hi)
                            .with_a(z.break_idx, z.lo)
                            .with_value(f64::from(z.touches)),
                    );
                }
            }
            if z.role <= 0 && c.close > z.hi + brk {
                if z.role < 0 {
                    z.flips += 1;
                    z.break_idx = i;
                    z.retest_pending = true;
                    z.left = c.close >= z.hi + away;
                    out.push(
                        Event::new("sr_zone", "zone_break_up", 1, i)
                            .with_start(z.first_idx)
                            .with_price(z.hi)
                            .with_zone(z.lo, z.hi)
                            .with_value(f64::from(z.touches)),
                    );
                }
                z.role = 1;
            } else if z.role >= 0 && c.close < z.lo - brk {
                if z.role > 0 {
                    z.flips += 1;
                    z.break_idx = i;
                    z.retest_pending = true;
                    z.left = c.close <= z.lo - away;
                    out.push(
                        Event::new("sr_zone", "zone_break_down", -1, i)
                            .with_start(z.first_idx)
                            .with_price(z.lo)
                            .with_zone(z.lo, z.hi)
                            .with_value(f64::from(z.touches)),
                    );
                }
                z.role = -1;
            }
        }
        let close = c.close;
        if let Some(pv) = self.swings.new_high {
            self.add_pivot(i, pv, close, a, out);
        }
        if let Some(pv) = self.swings.new_low {
            self.add_pivot(i, pv, close, a, out);
        }
        let exp = self.p.expire_bars;
        let max_flips = self.p.max_flips;
        self.zones.retain(|z| {
            z.flips <= max_flips && (i.saturating_sub(z.last_idx) <= exp || z.retest_pending)
        });
    }
}

impl Detector for SrZones {
    fn names(&self) -> &'static [&'static str] {
        &["sr_zone", "role_reversal"]
    }

    fn update(&mut self, i: u32, c: &Candle, out: &mut Vec<Event>) {
        self.step(i, c, out);
    }
}

// ---------------------------------------------------------------------------------------------
// Supply / demand
// ---------------------------------------------------------------------------------------------

/// Supply/demand parameters (T4).
#[derive(Debug, Clone, Copy)]
pub struct SdParams {
    /// Base: 1..=`base_max` consecutive bars each with range <= `base_range_atr` x ATR.
    /// Default 6; sensible 3..=10.
    pub base_max: usize,
    /// Default 0.6; sensible 0.4..=1.0.
    pub base_range_atr: f64,
    /// Departure: `depart_min`..=`depart_max` consecutive same-direction bars with
    /// body >= `depart_body_ratio` x range. Defaults 2 / 5; sensible 1..=3 / 3..=8.
    pub depart_min: usize,
    pub depart_max: usize,
    /// Default 0.5; sensible 0.4..=0.8.
    pub depart_body_ratio: f64,
    /// Departure close must be this many ATR beyond the base zone. Default 2.0; sensible 1.5..=4.
    pub depart_atr: f64,
    /// A zone retires after this many tests or bars. Defaults 3 / 2000.
    pub max_tests: u16,
    pub max_age: u32,
    pub max_zones: usize,
}

impl Default for SdParams {
    fn default() -> Self {
        SdParams {
            base_max: 6,
            base_range_atr: 0.6,
            depart_min: 2,
            depart_max: 5,
            depart_body_ratio: 0.5,
            depart_atr: 2.0,
            max_tests: 3,
            max_age: 2000,
            max_zones: 40,
        }
    }
}

/// One supply or demand zone.
#[derive(Debug, Clone, Copy, Default)]
pub struct SdZone {
    pub lo: f64,
    pub hi: f64,
    /// +1 demand, -1 supply.
    pub dir: i8,
    pub base_start: u32,
    pub formed_idx: u32,
    pub tests: u16,
    in_touch: bool,
}

impl SdZone {
    /// Fresh = not yet revisited.
    pub fn fresh(&self) -> bool {
        self.tests == 0
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct B {
    o: f64,
    h: f64,
    l: f64,
    c: f64,
    idx: u32,
}

/// Supply and demand zones (T4): a base of small bars followed by a strong departure.
/// Demand zone = [lowest wick of base, highest body of base] (supply mirrored).
///
/// Detector `sd_zone`: `demand_formed` / `supply_formed` (at the departure bar that first
/// clears `depart_atr`), `demand_test` / `supply_test` (each new revisit; `tags` bit 0 = fresh,
/// i.e. the first test), `demand_broken` / `supply_broken` (close beyond the far side).
/// `value` = departure size in ATR for `*_formed`, test number for `*_test`.
#[derive(Debug, Clone)]
pub struct SdZones {
    p: SdParams,
    atr: Atr,
    bars: Ring<B>,
    zones: Vec<SdZone>,
    last_base: [u32; 2],
}

/// `tags` bit for a fresh (first) test.
pub const TAG_FRESH: u32 = 1;

impl SdZones {
    pub fn new(p: SdParams) -> SdZones {
        SdZones {
            p,
            atr: Atr::new(14),
            bars: Ring::new(p.depart_max + p.base_max + 2),
            zones: Vec::with_capacity(p.max_zones.max(1)),
            last_base: [u32::MAX; 2],
        }
    }

    pub fn zones(&self) -> &[SdZone] {
        &self.zones
    }

    fn detect(&mut self, i: u32, dir: i8, a: f64, out: &mut Vec<Event>) {
        let p = self.p;
        // Departure run ending at the newest bar.
        let mut d = 0;
        while let Some(b) = self.bars.back(d) {
            let rng = b.h - b.l;
            let directional = if dir > 0 { b.c > b.o } else { b.c < b.o };
            if directional && rng > 0.0 && (b.c - b.o).abs() >= p.depart_body_ratio * rng {
                d += 1;
            } else {
                break;
            }
        }
        if d < p.depart_min || d > p.depart_max {
            return;
        }
        // Base right before it.
        let mut nb = 0;
        let (mut lo, mut hi) = (f64::INFINITY, f64::NEG_INFINITY);
        let mut start = 0;
        while nb < p.base_max {
            let Some(b) = self.bars.back(d + nb) else {
                break;
            };
            if b.h - b.l > p.base_range_atr * a {
                break;
            }
            if dir > 0 {
                lo = lo.min(b.l);
                hi = hi.max(b.o.max(b.c));
            } else {
                lo = lo.min(b.o.min(b.c));
                hi = hi.max(b.h);
            }
            start = b.idx;
            nb += 1;
        }
        if nb == 0 {
            return;
        }
        let slot = usize::from(dir < 0);
        if self.last_base[slot] == start {
            return;
        }
        let Some(last) = self.bars.back(0) else {
            return;
        };
        let dist = if dir > 0 { last.c - hi } else { lo - last.c };
        if dist < p.depart_atr * a {
            return;
        }
        self.last_base[slot] = start;
        let z = SdZone {
            lo,
            hi,
            dir,
            base_start: start,
            formed_idx: i,
            tests: 0,
            in_touch: false,
        };
        bounded_push(&mut self.zones, p.max_zones.max(1), z, |_| 0);
        let kind = if dir > 0 {
            "demand_formed"
        } else {
            "supply_formed"
        };
        out.push(
            Event::new("sd_zone", kind, dir, i)
                .with_start(start)
                .with_price(if dir > 0 { hi } else { lo })
                .with_zone(lo, hi)
                .with_value(dist / a),
        );
    }

    pub fn step(&mut self, i: u32, c: &Candle, out: &mut Vec<Event>) {
        let a = self.atr.get();
        self.atr.update(c);
        // Existing zones react to this bar.
        let p = self.p;
        self.zones.retain_mut(|z| {
            if i <= z.formed_idx {
                return true;
            }
            if i - z.formed_idx > p.max_age {
                return false;
            }
            let (broken, touching) = if z.dir > 0 {
                (c.close < z.lo, c.low <= z.hi)
            } else {
                (c.close > z.hi, c.high >= z.lo)
            };
            if broken {
                let kind = if z.dir > 0 {
                    "demand_broken"
                } else {
                    "supply_broken"
                };
                out.push(
                    Event::new("sd_zone", kind, -z.dir, i)
                        .with_start(z.base_start)
                        .with_price(if z.dir > 0 { z.lo } else { z.hi })
                        .with_zone(z.lo, z.hi)
                        .with_value(f64::from(z.tests)),
                );
                return false;
            }
            if touching && !z.in_touch {
                z.tests += 1;
                let kind = if z.dir > 0 {
                    "demand_test"
                } else {
                    "supply_test"
                };
                out.push(
                    Event::new("sd_zone", kind, z.dir, i)
                        .with_start(z.base_start)
                        .with_price(if z.dir > 0 { z.hi } else { z.lo })
                        .with_zone(z.lo, z.hi)
                        .with_tags(if z.tests == 1 { TAG_FRESH } else { 0 })
                        .with_value(f64::from(z.tests)),
                );
            }
            z.in_touch = touching;
            z.tests <= p.max_tests || z.in_touch
        });
        self.bars.push(B {
            o: c.open,
            h: c.high,
            l: c.low,
            c: c.close,
            idx: i,
        });
        if a.is_nan() {
            return;
        }
        self.detect(i, 1, a, out);
        self.detect(i, -1, a, out);
    }
}

impl Detector for SdZones {
    fn names(&self) -> &'static [&'static str] {
        &["sd_zone"]
    }

    fn update(&mut self, i: u32, c: &Candle, out: &mut Vec<Event>) {
        self.step(i, c, out);
    }
}

/// Helpers shared with the pullback detector.
pub fn band_touch(low_or_high: f64, lo: f64, hi: f64, tol: f64) -> bool {
    low_or_high <= hi + tol && low_or_high >= lo - tol
}
