//! Swing pivots: fractal pivots (knowledge S2) and ATR `ZigZag` pivots (S3).

use cs_core::{Candle, Event};

use crate::Detector;
use crate::util::{Atr, Ring};

/// A confirmed swing pivot.
#[derive(Debug, Clone, Copy, Default)]
pub struct Pivot {
    /// Pivot candle index.
    pub idx: u32,
    /// Index of the candle whose close confirmed it.
    pub known: u32,
    /// High (swing high) or low (swing low) of the pivot candle.
    pub price: f64,
    /// Body extreme of the pivot candle: `max(O,C)` for highs, `min(O,C)` for lows.
    pub body: f64,
    pub is_high: bool,
}

/// Fractal swing parameters.
#[derive(Debug, Clone, Copy)]
pub struct SwingParams {
    /// Pivot strength `k`: the pivot's high must beat the `k` candles on each side.
    /// Default 3; sensible range 2..=10 (larger = fewer, more significant swings, later
    /// confirmation). A pivot is known only `k` candles after it.
    pub k: usize,
}

impl Default for SwingParams {
    fn default() -> Self {
        SwingParams { k: 3 }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Bar {
    idx: u32,
    o: f64,
    h: f64,
    l: f64,
    c: f64,
}

/// Fractal swing detector (S2). Bar `i` is a swing high of strength `k` if `H[i] > H[j]` for the
/// `k` bars before it and `H[i] >= H[j]` for the `k` bars after it (ties: the earliest bar wins).
/// Known at the close of bar `i + k`. Emits `swing` / `swing_high` | `swing_low`
/// (`price` = pivot price, `lo..hi` = wick..body band).
#[derive(Debug, Clone)]
pub struct Swings {
    k: usize,
    win: Ring<Bar>,
    highs: Ring<Pivot>,
    lows: Ring<Pivot>,
    /// Pivots confirmed on the latest update (at most one of each).
    pub new_high: Option<Pivot>,
    pub new_low: Option<Pivot>,
}

impl Swings {
    pub fn new(p: SwingParams) -> Swings {
        let k = p.k.max(1);
        Swings {
            k,
            win: Ring::new(2 * k + 1),
            highs: Ring::new(16),
            lows: Ring::new(16),
            new_high: None,
            new_low: None,
        }
    }

    pub fn k(&self) -> usize {
        self.k
    }

    /// `n`-th most recent confirmed swing high (0 = latest).
    pub fn high(&self, n: usize) -> Option<Pivot> {
        self.highs.back(n)
    }

    pub fn low(&self, n: usize) -> Option<Pivot> {
        self.lows.back(n)
    }

    /// Feed bar `i`; updates `new_high` / `new_low`.
    pub fn step(&mut self, i: u32, c: &Candle) {
        self.new_high = None;
        self.new_low = None;
        self.win.push(Bar {
            idx: i,
            o: c.open,
            h: c.high,
            l: c.low,
            c: c.close,
        });
        if !self.win.is_full() {
            return;
        }
        let k = self.k;
        let Some(m) = self.win.get(k) else { return };
        let mut is_h = true;
        let mut is_l = true;
        for (j, b) in self.win.iter().enumerate() {
            if j == k {
                continue;
            }
            if j < k {
                is_h &= m.h > b.h;
                is_l &= m.l < b.l;
            } else {
                is_h &= m.h >= b.h;
                is_l &= m.l <= b.l;
            }
        }
        if is_h {
            let p = Pivot {
                idx: m.idx,
                known: i,
                price: m.h,
                body: m.o.max(m.c),
                is_high: true,
            };
            self.highs.push(p);
            self.new_high = Some(p);
        }
        if is_l {
            let p = Pivot {
                idx: m.idx,
                known: i,
                price: m.l,
                body: m.o.min(m.c),
                is_high: false,
            };
            self.lows.push(p);
            self.new_low = Some(p);
        }
    }
}

impl Detector for Swings {
    fn names(&self) -> &'static [&'static str] {
        &["swing"]
    }

    fn update(&mut self, i: u32, c: &Candle, out: &mut Vec<Event>) {
        self.step(i, c);
        if let Some(p) = self.new_high {
            out.push(
                Event::new("swing", "swing_high", -1, i)
                    .with_formed(p.idx)
                    .with_start(p.idx)
                    .with_price(p.price)
                    .with_zone(p.body, p.price),
            );
        }
        if let Some(p) = self.new_low {
            out.push(
                Event::new("swing", "swing_low", 1, i)
                    .with_formed(p.idx)
                    .with_start(p.idx)
                    .with_price(p.price)
                    .with_zone(p.price, p.body),
            );
        }
    }
}

/// `ZigZag` parameters (S3).
#[derive(Debug, Clone, Copy)]
pub struct ZigZagParams {
    /// Reversal threshold in ATR(14) multiples. Default 3.0; sensible range 1.5..=8.0
    /// (bigger = only "significant" swings).
    pub atr_mult: f64,
    pub atr_len: usize,
}

impl Default for ZigZagParams {
    fn default() -> Self {
        ZigZagParams {
            atr_mult: 3.0,
            atr_len: 14,
        }
    }
}

/// ATR `ZigZag`: a swing high is confirmed when a later low is at least `atr_mult x ATR` below the
/// running high (and vice versa). A bar that makes a new extreme never also confirms a reversal
/// (intrabar order is unknown). ATR is the value before the current bar.
#[derive(Debug, Clone)]
pub struct ZigZag {
    p: ZigZagParams,
    atr: Atr,
    /// 0 = not started, 1 = in an up-leg (tracking the high), -1 = in a down-leg.
    dir: i8,
    hi: f64,
    hi_idx: u32,
    lo: f64,
    lo_idx: u32,
    pivots: Ring<Pivot>,
    pub new_pivot: Option<Pivot>,
}

impl ZigZag {
    pub fn new(p: ZigZagParams) -> ZigZag {
        ZigZag {
            p,
            atr: Atr::new(p.atr_len),
            dir: 0,
            hi: f64::NAN,
            hi_idx: 0,
            lo: f64::NAN,
            lo_idx: 0,
            pivots: Ring::new(8),
            new_pivot: None,
        }
    }

    /// `n`-th most recent pivot (0 = latest).
    pub fn pivot(&self, n: usize) -> Option<Pivot> {
        self.pivots.back(n)
    }

    pub fn step(&mut self, i: u32, c: &Candle) {
        self.new_pivot = None;
        let a = self.atr.get();
        self.atr.update(c);
        if a.is_nan() {
            return;
        }
        let thr = self.p.atr_mult * a;
        match self.dir {
            0 => {
                if self.hi.is_nan() || c.high > self.hi {
                    self.hi = c.high;
                    self.hi_idx = i;
                }
                if self.lo.is_nan() || c.low < self.lo {
                    self.lo = c.low;
                    self.lo_idx = i;
                }
                if c.high >= self.lo + thr && self.lo_idx < i {
                    self.confirm(false, i);
                    self.dir = 1;
                    self.hi = c.high;
                    self.hi_idx = i;
                } else if c.low <= self.hi - thr && self.hi_idx < i {
                    self.confirm(true, i);
                    self.dir = -1;
                    self.lo = c.low;
                    self.lo_idx = i;
                }
            }
            1 => {
                if c.high > self.hi {
                    self.hi = c.high;
                    self.hi_idx = i;
                } else if c.low <= self.hi - thr {
                    self.confirm(true, i);
                    self.dir = -1;
                    self.lo = c.low;
                    self.lo_idx = i;
                }
            }
            _ => {
                if c.low < self.lo {
                    self.lo = c.low;
                    self.lo_idx = i;
                } else if c.high >= self.lo + thr {
                    self.confirm(false, i);
                    self.dir = 1;
                    self.hi = c.high;
                    self.hi_idx = i;
                }
            }
        }
    }

    fn confirm(&mut self, high: bool, i: u32) {
        let p = if high {
            Pivot {
                idx: self.hi_idx,
                known: i,
                price: self.hi,
                body: self.hi,
                is_high: true,
            }
        } else {
            Pivot {
                idx: self.lo_idx,
                known: i,
                price: self.lo,
                body: self.lo,
                is_high: false,
            }
        };
        self.pivots.push(p);
        self.new_pivot = Some(p);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::ohlc;

    #[test]
    fn fractal_pivot_known_k_bars_later() {
        // highs: 1 2 3 9 3 2 1 -> pivot at idx 3, known at idx 6 with k=3
        let hs = [1.0, 2.0, 3.0, 9.0, 3.0, 2.0, 1.0, 1.5];
        let mut s = Swings::new(SwingParams { k: 3 });
        let mut out = Vec::new();
        for (i, h) in hs.iter().enumerate() {
            s.update(i as u32, &ohlc(h - 0.5, *h, h - 1.0, h - 0.5), &mut out);
            if i < 6 {
                assert!(out.iter().all(|e| e.kind != "swing_high"), "early at {i}");
            }
        }
        let e: Vec<_> = out.iter().filter(|e| e.kind == "swing_high").collect();
        assert_eq!(e.len(), 1);
        assert_eq!((e[0].formed_idx, e[0].known_idx), (3, 6));
    }

    #[test]
    fn fractal_tie_earliest_wins() {
        let hs = [1.0, 2.0, 3.0, 9.0, 9.0, 2.0, 1.0, 0.5, 0.4];
        let mut s = Swings::new(SwingParams { k: 3 });
        let mut out = Vec::new();
        for (i, h) in hs.iter().enumerate() {
            s.update(i as u32, &ohlc(h - 0.5, *h, h - 1.0, h - 0.5), &mut out);
        }
        let e: Vec<_> = out.iter().filter(|e| e.kind == "swing_high").collect();
        assert_eq!(e.len(), 1);
        assert_eq!(e[0].formed_idx, 3);
    }
}
