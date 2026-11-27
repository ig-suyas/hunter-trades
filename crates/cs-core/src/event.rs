//! Pattern events produced by the detectors in `cs-patterns`.
//!
//! The no-lookahead contract lives here:
//!
//! * `formed_idx` / `formed_at`: the candle where the pattern completes (for a swing high, the
//!   pivot candle itself).
//! * `known_idx` / `known_at`: the first candle whose **close** reveals the pattern (for a swing
//!   high of strength `k`, the candle `k` bars after the pivot). An event may be acted on only
//!   after that candle has closed, i.e. at the next candle's open or later.
//!
//! Indices count candles from the start of the stream fed to a detector (0-based). Times are the
//! candles' open times in UTC milliseconds (candle time = open time), so the moment an event
//! becomes usable is `known_at + timeframe length`.
//!
//! Floating fields that a detector does not use are `NaN`. Compare events with
//! [`Event::same`], which compares floats bit for bit (so `NaN == NaN`).

/// One detector event. `Copy` and allocation free, so detectors can emit them in steady state
/// without touching the heap (the caller owns and reuses the output buffer).
#[derive(Debug, Clone, Copy)]
pub struct Event {
    /// Detector name (stable, used in reports and files), e.g. `"swing"`, `"cdl_hammer"`.
    pub det: &'static str,
    /// Event kind within the detector, e.g. `"swing_high"`, `"bos_bull"`.
    pub kind: &'static str,
    /// +1 bullish, -1 bearish, 0 neutral / not directional.
    pub dir: i8,
    /// First candle of the pattern (anchor), for drawing. May be far in the past.
    pub start_idx: u32,
    /// Candle where the pattern completes.
    pub formed_idx: u32,
    /// First candle at whose close the pattern is known. Always the candle being processed when
    /// the event is emitted.
    pub known_idx: u32,
    /// Open time of `formed_idx` (UTC ms). Filled by the runner.
    pub formed_at: i64,
    /// Open time of `known_idx` (UTC ms). Filled by the runner.
    pub known_at: i64,
    /// Key price (level, pivot price, trigger close ...).
    pub price: f64,
    /// Lower bound of a zone or pattern extent (stop reference for long setups).
    pub lo: f64,
    /// Upper bound of a zone or pattern extent.
    pub hi: f64,
    /// Optional anchor point A (index, price) for lines and legs.
    pub ax: u32,
    pub ay: f64,
    /// Optional anchor point B (index, price).
    pub bx: u32,
    pub by: f64,
    /// Detector-specific flags (documented per detector).
    pub tags: u32,
    /// Detector-specific magnitude (documented per detector), e.g. strength in ATR.
    pub value: f64,
}

impl Event {
    /// A new event known (and, unless changed, formed) at candle `i`; all optional fields empty.
    pub fn new(det: &'static str, kind: &'static str, dir: i8, i: u32) -> Event {
        Event {
            det,
            kind,
            dir,
            start_idx: i,
            formed_idx: i,
            known_idx: i,
            formed_at: 0,
            known_at: 0,
            price: f64::NAN,
            lo: f64::NAN,
            hi: f64::NAN,
            ax: u32::MAX,
            ay: f64::NAN,
            bx: u32::MAX,
            by: f64::NAN,
            tags: 0,
            value: f64::NAN,
        }
    }

    /// Exact equality with floats compared bit for bit.
    pub fn same(&self, o: &Event) -> bool {
        let f = |a: f64, b: f64| a.to_bits() == b.to_bits();
        self.det == o.det
            && self.kind == o.kind
            && self.dir == o.dir
            && self.start_idx == o.start_idx
            && self.formed_idx == o.formed_idx
            && self.known_idx == o.known_idx
            && self.formed_at == o.formed_at
            && self.known_at == o.known_at
            && f(self.price, o.price)
            && f(self.lo, o.lo)
            && f(self.hi, o.hi)
            && self.ax == o.ax
            && f(self.ay, o.ay)
            && self.bx == o.bx
            && f(self.by, o.by)
            && self.tags == o.tags
            && f(self.value, o.value)
    }

    #[must_use]
    pub fn with_price(mut self, p: f64) -> Event {
        self.price = p;
        self
    }

    #[must_use]
    pub fn with_zone(mut self, lo: f64, hi: f64) -> Event {
        self.lo = lo;
        self.hi = hi;
        self
    }

    #[must_use]
    pub fn with_formed(mut self, formed_idx: u32) -> Event {
        self.formed_idx = formed_idx;
        self
    }

    #[must_use]
    pub fn with_start(mut self, start_idx: u32) -> Event {
        self.start_idx = start_idx;
        self
    }

    #[must_use]
    pub fn with_a(mut self, x: u32, y: f64) -> Event {
        self.ax = x;
        self.ay = y;
        self
    }

    #[must_use]
    pub fn with_b(mut self, x: u32, y: f64) -> Event {
        self.bx = x;
        self.by = y;
        self
    }

    #[must_use]
    pub fn with_tags(mut self, t: u32) -> Event {
        self.tags = t;
        self
    }

    #[must_use]
    pub fn with_value(mut self, v: f64) -> Event {
        self.value = v;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_treats_nan_as_equal() {
        let a = Event::new("x", "y", 1, 5).with_price(1.5);
        let mut b = a;
        assert!(a.same(&b));
        b.lo = 0.0;
        assert!(!a.same(&b));
    }
}
