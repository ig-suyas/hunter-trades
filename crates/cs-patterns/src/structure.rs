//! Market structure (knowledge S8, S9): HH/HL/LH/LL labels, trend state, break of structure
//! (BOS) and change of character (`CHoCH`).

use cs_core::{Candle, Event};

use crate::Detector;
use crate::swing::{Pivot, SwingParams, Swings};
use crate::util::Atr;

/// Market-structure parameters.
#[derive(Debug, Clone, Copy, Default)]
pub struct StructureParams {
    /// Swing pivots used for structure (default k = 3; sensible 2..=10).
    pub swing: SwingParams,
}

/// Labels every confirmed swing against the previous one of the same side:
/// `hh` / `lh` for highs, `hl` / `ll` for lows (`ax,ay` = previous pivot, `bx,by` = new pivot).
/// Trend (S8): up when the latest high is HH and the latest low HL, down when LH and LL, else range.
/// Also emits `trend_up` / `trend_down` / `trend_range` when that state changes.
/// Detector name: `structure`.
#[derive(Debug, Clone)]
pub struct Structure {
    pub swings: Swings,
    last_high_up: Option<bool>,
    last_low_up: Option<bool>,
    trend: i8,
}

impl Structure {
    pub fn new(p: StructureParams) -> Structure {
        Structure {
            swings: Swings::new(p.swing),
            last_high_up: None,
            last_low_up: None,
            trend: 0,
        }
    }

    /// +1 up, -1 down, 0 range (as of the latest confirmed swing).
    pub fn trend(&self) -> i8 {
        self.trend
    }

    fn label(&mut self, i: u32, p: Pivot, prev: Option<Pivot>, out: &mut Vec<Event>) {
        let Some(q) = prev else { return };
        let up = p.price > q.price;
        let (kind, dir) = match (p.is_high, up) {
            (true, true) => ("hh", 1),
            (true, false) => ("lh", -1),
            (false, true) => ("hl", 1),
            (false, false) => ("ll", -1),
        };
        if p.is_high {
            self.last_high_up = Some(up);
        } else {
            self.last_low_up = Some(up);
        }
        out.push(
            Event::new("structure", kind, dir, i)
                .with_formed(p.idx)
                .with_start(q.idx)
                .with_price(p.price)
                .with_a(q.idx, q.price)
                .with_b(p.idx, p.price),
        );
    }

    pub fn step(&mut self, i: u32, c: &Candle, out: &mut Vec<Event>) {
        self.swings.step(i, c);
        if let Some(p) = self.swings.new_high {
            let prev = self.swings.high(1);
            self.label(i, p, prev, out);
        }
        if let Some(p) = self.swings.new_low {
            let prev = self.swings.low(1);
            self.label(i, p, prev, out);
        }
        let t = match (self.last_high_up, self.last_low_up) {
            (Some(true), Some(true)) => 1,
            (Some(false), Some(false)) => -1,
            _ => 0,
        };
        if t != self.trend {
            self.trend = t;
            let kind = match t {
                1 => "trend_up",
                -1 => "trend_down",
                _ => "trend_range",
            };
            out.push(Event::new("structure", kind, t, i).with_price(c.close));
        }
    }
}

impl Detector for Structure {
    fn names(&self) -> &'static [&'static str] {
        &["structure"]
    }

    fn update(&mut self, i: u32, c: &Candle, out: &mut Vec<Event>) {
        self.step(i, c, out);
    }
}

/// BOS / `CHoCH` parameters.
#[derive(Debug, Clone, Copy)]
pub struct BreakParams {
    pub structure: StructureParams,
    /// The close must clear the swing level by this many ATR(14). Default 0.0 (any close beyond);
    /// sensible 0.0..=0.5.
    pub buffer_atr: f64,
}

impl Default for BreakParams {
    fn default() -> Self {
        BreakParams {
            structure: StructureParams::default(),
            buffer_atr: 0.0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Level {
    idx: u32,
    price: f64,
    broken: bool,
}

/// Break of structure (S9): a close above the most recent confirmed, not yet broken swing high
/// (bull) or below the latest swing low (bear). Each swing level breaks at most once.
/// If the structure trend at that moment is against the break, the event is a change of
/// character: detector `choch` (`choch_bull` / `choch_bear`); otherwise detector `bos`
/// (`bos_bull` / `bos_bear`). `price` = broken level, `ax,ay` = the swing, `value` = close
/// beyond the level in ATR.
#[derive(Debug, Clone)]
pub struct StructureBreak {
    p: BreakParams,
    st: Structure,
    atr: Atr,
    sh: Option<Level>,
    sl: Option<Level>,
    scratch: Vec<Event>,
}

impl StructureBreak {
    pub fn new(p: BreakParams) -> StructureBreak {
        StructureBreak {
            p,
            st: Structure::new(p.structure),
            atr: Atr::new(14),
            sh: None,
            sl: None,
            scratch: Vec::with_capacity(8),
        }
    }
}

impl Detector for StructureBreak {
    fn names(&self) -> &'static [&'static str] {
        &["bos", "choch"]
    }

    fn update(&mut self, i: u32, c: &Candle, out: &mut Vec<Event>) {
        let a = self.atr.update(c);
        // The trend that applies to a break on this bar is the one before this bar's swings.
        let trend = self.st.trend();
        self.scratch.clear();
        self.st.step(i, c, &mut self.scratch);
        if let Some(p) = self.st.swings.new_high {
            self.sh = Some(Level {
                idx: p.idx,
                price: p.price,
                broken: false,
            });
        }
        if let Some(p) = self.st.swings.new_low {
            self.sl = Some(Level {
                idx: p.idx,
                price: p.price,
                broken: false,
            });
        }
        let buf = self.p.buffer_atr * a;
        if let Some(l) = self.sh.as_mut()
            && !l.broken
            && c.close > l.price + buf
        {
            l.broken = true;
            let (det, kind) = if trend < 0 {
                ("choch", "choch_bull")
            } else {
                ("bos", "bos_bull")
            };
            out.push(
                Event::new(det, kind, 1, i)
                    .with_start(l.idx)
                    .with_price(l.price)
                    .with_a(l.idx, l.price)
                    .with_value((c.close - l.price) / a),
            );
        }
        if let Some(l) = self.sl.as_mut()
            && !l.broken
            && c.close < l.price - buf
        {
            l.broken = true;
            let (det, kind) = if trend > 0 {
                ("choch", "choch_bear")
            } else {
                ("bos", "bos_bear")
            };
            out.push(
                Event::new(det, kind, -1, i)
                    .with_start(l.idx)
                    .with_price(l.price)
                    .with_a(l.idx, l.price)
                    .with_value((l.price - c.close) / a),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::path_candles;

    #[test]
    fn hh_hl_then_bos_and_choch() {
        // Zig-zag up (HH/HL), then break the last swing low while trend is up -> choch_bear.
        let path = [
            10.0, 12.0, 14.0, 16.0, 14.0, 12.0, 13.0, 15.0, 17.0, 19.0, 17.0, 15.0, 16.0, 18.0,
            20.0, 22.0, 20.0, 18.0, 16.0, 14.0, 12.0, 10.0, 9.0, 8.0,
        ];
        let cs = path_candles(&path);
        let mut d = StructureBreak::new(BreakParams {
            structure: StructureParams {
                swing: SwingParams { k: 2 },
            },
            buffer_atr: 0.0,
        });
        let mut st = Structure::new(StructureParams {
            swing: SwingParams { k: 2 },
        });
        let mut out = Vec::new();
        let mut so = Vec::new();
        for (i, c) in cs.iter().enumerate() {
            d.update(i as u32, c, &mut out);
            st.update(i as u32, c, &mut so);
        }
        let kinds: Vec<_> = so.iter().map(|e| e.kind).collect();
        assert!(kinds.contains(&"hh") && kinds.contains(&"hl"), "{kinds:?}");
        assert!(kinds.contains(&"trend_up"), "{kinds:?}");
        let bk: Vec<_> = out.iter().map(|e| e.kind).collect();
        assert!(bk.contains(&"bos_bull"), "{bk:?}");
        assert!(bk.contains(&"choch_bear"), "{bk:?}");
    }
}
