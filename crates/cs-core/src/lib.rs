//! cs-core: shared types (Candle, Timeframe) and small maths/time helpers.
//!
//! Advisory only: nothing in this workspace places orders.

pub mod event;
pub mod time;

pub use event::Event;

/// Milliseconds in one minute.
pub const MINUTE_MS: i64 = 60_000;

/// One OHLCV candle. `open_time` is UTC milliseconds of the candle's open (candle time = open
/// time). Prices and volumes are `f64`; compare them with [`approx_eq`], never with `==`.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Candle {
    pub open_time: i64,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    /// Base-asset volume.
    pub volume: f64,
    /// Quote-asset volume (turnover).
    pub quote_volume: f64,
    /// Number of trades; `-1` when the source does not provide it.
    pub trades: i64,
    /// Taker-buy base volume (aggressive buys); `NaN` when the source does not provide it.
    pub taker_buy_base: f64,
    /// Taker-buy quote volume; `NaN` when the source does not provide it.
    pub taker_buy_quote: f64,
}

impl Candle {
    /// Close time in UTC ms for a candle of length `len_ms` (inclusive end, Binance style).
    pub fn close_time(&self, len_ms: i64) -> i64 {
        self.open_time + len_ms - 1
    }
}

/// Candle time frames used by the project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Timeframe {
    M1,
    M5,
    M15,
    H1,
    H4,
    D1,
}

impl Timeframe {
    pub const ALL: [Timeframe; 6] = [
        Timeframe::M1,
        Timeframe::M5,
        Timeframe::M15,
        Timeframe::H1,
        Timeframe::H4,
        Timeframe::D1,
    ];

    /// Length in milliseconds.
    pub const fn ms(self) -> i64 {
        match self {
            Timeframe::M1 => MINUTE_MS,
            Timeframe::M5 => 5 * MINUTE_MS,
            Timeframe::M15 => 15 * MINUTE_MS,
            Timeframe::H1 => 60 * MINUTE_MS,
            Timeframe::H4 => 240 * MINUTE_MS,
            Timeframe::D1 => 1440 * MINUTE_MS,
        }
    }

    /// Number of 1-minute candles in one candle of this time frame.
    pub const fn minutes(self) -> i64 {
        self.ms() / MINUTE_MS
    }

    pub const fn label(self) -> &'static str {
        match self {
            Timeframe::M1 => "1m",
            Timeframe::M5 => "5m",
            Timeframe::M15 => "15m",
            Timeframe::H1 => "1h",
            Timeframe::H4 => "4h",
            Timeframe::D1 => "1d",
        }
    }

    pub fn parse(s: &str) -> Option<Timeframe> {
        Timeframe::ALL.into_iter().find(|tf| tf.label() == s)
    }

    /// UTC-aligned bucket open time containing `t` (floor to a multiple of the length since
    /// the Unix epoch; for 4h that gives 00/04/08/12/16/20 UTC, for 1d 00:00 UTC).
    pub const fn bucket(self, t: i64) -> i64 {
        t.div_euclid(self.ms()) * self.ms()
    }
}

/// Relative + absolute float tolerance compare.
pub fn approx_eq(a: f64, b: f64, rel: f64, abs: f64) -> bool {
    if a.is_nan() || b.is_nan() {
        return a.is_nan() && b.is_nan();
    }
    let d = (a - b).abs();
    d <= abs || d <= rel * a.abs().max(b.abs())
}

/// Default tolerance used for prices and volumes parsed from decimal strings.
pub fn price_eq(a: f64, b: f64) -> bool {
    approx_eq(a, b, 1e-12, 1e-12)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buckets_are_utc_aligned() {
        // 2024-11-01 05:17:00 UTC
        let t = time::ymd_hms_to_ms(2024, 11, 1, 5, 17, 0);
        assert_eq!(
            Timeframe::H4.bucket(t),
            time::ymd_hms_to_ms(2024, 11, 1, 4, 0, 0)
        );
        assert_eq!(
            Timeframe::D1.bucket(t),
            time::ymd_hms_to_ms(2024, 11, 1, 0, 0, 0)
        );
        assert_eq!(
            Timeframe::M15.bucket(t),
            time::ymd_hms_to_ms(2024, 11, 1, 5, 15, 0)
        );
        assert_eq!(Timeframe::parse("4h"), Some(Timeframe::H4));
    }

    #[test]
    fn approx() {
        assert!(price_eq(0.1 + 0.2, 0.3));
        assert!(!price_eq(1.0, 1.0001));
        assert!(approx_eq(f64::NAN, f64::NAN, 0.0, 0.0));
    }
}
