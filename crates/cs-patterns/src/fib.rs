//! Fibonacci retracement and extension levels from the latest significant swing (knowledge T7f).

use cs_core::{Candle, Event};

use crate::Detector;
use crate::swing::{ZigZag, ZigZagParams};

/// Retracement ratios reported for each leg.
pub const RETRACE: [f64; 6] = [0.236, 0.382, 0.5, 0.618, 0.65, 0.786];
/// Extension ratios (targets beyond B).
pub const EXTEND: [f64; 2] = [1.272, 1.618];

/// Fibonacci parameters.
#[derive(Debug, Clone, Copy)]
pub struct FibParams {
    /// "Significant swing" = ATR `ZigZag` pivot. Default 3 ATR; sensible 2..=6.
    pub zigzag: ZigZagParams,
    /// Value zone used for touches. Defaults 0.382 / 0.618 (the brief's zone); the golden
    /// pocket 0.618 / 0.65 is a variant.
    pub zone_lo: f64,
    pub zone_hi: f64,
}

impl Default for FibParams {
    fn default() -> Self {
        FibParams {
            zigzag: ZigZagParams::default(),
            zone_lo: 0.382,
            zone_hi: 0.618,
        }
    }
}

/// An impulse leg A -> B.
#[derive(Debug, Clone, Copy, Default)]
pub struct Leg {
    pub a_idx: u32,
    pub a: f64,
    pub b_idx: u32,
    pub b: f64,
    pub alive: bool,
    zone_hit: bool,
    ext_hit: [bool; 2],
}

impl Leg {
    /// Retracement level `x` (0.618 = 61.8 % back from B toward A).
    pub fn level(&self, x: f64) -> f64 {
        self.b - x * (self.b - self.a)
    }

    /// Extension level `x` (1.272 = 27.2 % beyond B).
    pub fn ext(&self, x: f64) -> f64 {
        self.b + (x - 1.0) * (self.b - self.a)
    }
}

/// Fibonacci tool. Keeps the latest up-leg (`ZigZag` low A -> high B) and the latest down-leg.
/// A leg is cancelled by a close beyond A.
///
/// Detector `fib`: `fib_leg_up` / `fib_leg_down` when a leg is confirmed (`ax,ay` = A,
/// `bx,by` = B, `lo..hi` = the 38.2-61.8 % zone); `fib_zone_touch` (first bar of the leg that
/// trades into the zone; `value` = retracement depth of that bar's extreme); `fib_ext_1272` /
/// `fib_ext_1618` (first bar that reaches the extension after the leg is known).
#[derive(Debug, Clone)]
pub struct Fib {
    p: FibParams,
    zz: ZigZag,
    up: Leg,
    down: Leg,
}

impl Fib {
    pub fn new(p: FibParams) -> Fib {
        Fib {
            p,
            zz: ZigZag::new(p.zigzag),
            up: Leg::default(),
            down: Leg::default(),
        }
    }

    /// Active up-leg value zone (lo, hi), if any.
    pub fn zone_up(&self) -> Option<(f64, f64)> {
        self.up
            .alive
            .then(|| (self.up.level(self.p.zone_hi), self.up.level(self.p.zone_lo)))
    }

    /// Active down-leg value zone (lo, hi), if any.
    pub fn zone_down(&self) -> Option<(f64, f64)> {
        self.down.alive.then(|| {
            (
                self.down.level(self.p.zone_lo),
                self.down.level(self.p.zone_hi),
            )
        })
    }

    pub fn up_leg(&self) -> Option<Leg> {
        self.up.alive.then_some(self.up)
    }

    pub fn down_leg(&self) -> Option<Leg> {
        self.down.alive.then_some(self.down)
    }

    fn zone(&self, l: &Leg) -> (f64, f64) {
        let (x, y) = (l.level(self.p.zone_lo), l.level(self.p.zone_hi));
        (x.min(y), x.max(y))
    }

    fn ev(&self, kind: &'static str, dir: i8, i: u32, l: &Leg) -> Event {
        let (lo, hi) = self.zone(l);
        Event::new("fib", kind, dir, i)
            .with_formed(i)
            .with_start(l.a_idx)
            .with_zone(lo, hi)
            .with_a(l.a_idx, l.a)
            .with_b(l.b_idx, l.b)
    }

    fn track(&mut self, up: bool, i: u32, c: &Candle, out: &mut Vec<Event>) {
        let mut l = if up { self.up } else { self.down };
        if !l.alive {
            return;
        }
        let dir: i8 = if up { 1 } else { -1 };
        if (up && c.close < l.a) || (!up && c.close > l.a) {
            l.alive = false;
        } else {
            let (zlo, zhi) = self.zone(&l);
            let r = (l.b - l.a).abs();
            if !l.zone_hit && ((up && c.low <= zhi) || (!up && c.high >= zlo)) {
                l.zone_hit = true;
                let depth = if up {
                    (l.b - c.low) / r
                } else {
                    (c.high - l.b) / r
                };
                out.push(
                    self.ev("fib_zone_touch", dir, i, &l)
                        .with_price(if up { zhi } else { zlo })
                        .with_value(depth),
                );
            }
            for (k, x) in EXTEND.iter().enumerate() {
                let e = l.ext(*x);
                if !l.ext_hit[k] && ((up && c.high >= e) || (!up && c.low <= e)) {
                    l.ext_hit[k] = true;
                    let kind = if k == 0 {
                        "fib_ext_1272"
                    } else {
                        "fib_ext_1618"
                    };
                    out.push(self.ev(kind, dir, i, &l).with_price(e).with_value(*x));
                }
            }
        }
        if up {
            self.up = l;
        } else {
            self.down = l;
        }
    }

    pub fn step(&mut self, i: u32, c: &Candle, out: &mut Vec<Event>) {
        self.zz.step(i, c);
        if let (Some(b), Some(a)) = (self.zz.new_pivot, self.zz.pivot(1)) {
            let l = Leg {
                a_idx: a.idx,
                a: a.price,
                b_idx: b.idx,
                b: b.price,
                alive: true,
                zone_hit: false,
                ext_hit: [false; 2],
            };
            let up = b.is_high;
            let kind = if up { "fib_leg_up" } else { "fib_leg_down" };
            out.push(
                self.ev(kind, if up { 1 } else { -1 }, i, &l)
                    .with_formed(b.idx)
                    .with_price(b.price),
            );
            if up {
                self.up = l;
            } else {
                self.down = l;
            }
        }
        self.track(true, i, c, out);
        self.track(false, i, c, out);
    }
}

impl Detector for Fib {
    fn names(&self) -> &'static [&'static str] {
        &["fib"]
    }

    fn update(&mut self, i: u32, c: &Candle, out: &mut Vec<Event>) {
        self.step(i, c, out);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels() {
        let l = Leg {
            a: 100.0,
            b: 200.0,
            alive: true,
            ..Leg::default()
        };
        assert!((l.level(0.618) - 138.2).abs() < 1e-9);
        assert!((l.ext(1.272) - 227.2).abs() < 1e-9);
        let d = Leg {
            a: 200.0,
            b: 100.0,
            alive: true,
            ..Leg::default()
        };
        assert!((d.level(0.382) - 138.2).abs() < 1e-9);
        assert!((d.ext(1.618) - 38.2).abs() < 1e-9);
    }
}
