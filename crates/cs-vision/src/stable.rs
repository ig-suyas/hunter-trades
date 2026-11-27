//! Stability rule for screen readings, so one bad read never flips what the panel shows:
//! - a field missing from a read (symbol, time frame, price, or a whole read that saw no chart)
//!   is filled from the last good value for up to [`HOLD`] (3 s), marked as held;
//! - the feed may switch only after [`CONFIRM`] (2) consecutive reads agree on symbol and time
//!   frame;
//! - a platform window that leaves the front for less than [`HOLD`] (a popup, a recording
//!   toolbar, a click elsewhere) does not change the status: nothing is captured meanwhile, the
//!   last reading stays.
//!
//! Pure logic: time is passed in, so the rule is unit-tested.

use std::time::{Duration, Instant};

use crate::parse::TfRead;
use crate::read::{PageKind, PriceFrom, Reading, SymbolRead};

pub const HOLD: Duration = Duration::from_secs(3);
pub const CONFIRM: u32 = 2;

#[derive(Debug, Clone, Default)]
pub struct Stable {
    symbol: Option<(SymbolRead, &'static str, Instant)>,
    tf: Option<(TfRead, &'static str, Instant)>,
    price: Option<((f64, PriceFrom), Instant)>,
    chart_at: Option<Instant>,
    platform_at: Option<Instant>,
    key: Option<(SymbolRead, String)>,
    streak: u32,
}

/// The key two reads must share to count as consistent.
fn key_of(r: &Reading) -> Option<(SymbolRead, String)> {
    if r.page != PageKind::Chart || r.symbol == SymbolRead::Unknown {
        return None;
    }
    let tf = r.tf.as_ref()?;
    let canon = match tf.raw.as_str() {
        "D" | "W" | "M" => format!("1{}", tf.raw),
        _ => tf.label(),
    };
    Some((r.symbol.clone(), canon))
}

impl Stable {
    /// Takes a fresh read; returns the reading to show (held fields filled and listed in
    /// `held`).
    pub fn on_read(&mut self, raw: &Reading, now: Instant) -> Reading {
        self.platform_at = Some(now);
        let fresh = |t: Instant| now.saturating_duration_since(t) <= HOLD;
        let mut out = raw.clone();
        out.held.clear();
        if raw.page == PageKind::Chart {
            self.chart_at = Some(now);
        } else if self.chart_at.is_some_and(fresh) {
            // One read that saw no chart (a menu, a half-drawn page): keep the chart.
            out.page = PageKind::Chart;
            out.held.push("page");
        }
        if out.page == PageKind::Chart {
            match (&raw.symbol, &self.symbol) {
                (SymbolRead::Unknown, Some((s, from, t))) if fresh(*t) => {
                    out.symbol = s.clone();
                    out.symbol_from = from;
                    out.held.push("symbol");
                }
                (SymbolRead::Unknown, _) => {}
                (s, _) => self.symbol = Some((s.clone(), raw.symbol_from, now)),
            }
            match (&raw.tf, &self.tf) {
                (None, Some((tf, from, t))) if fresh(*t) => {
                    out.tf = Some(tf.clone());
                    out.tf_from = from;
                    out.held.push("time frame");
                }
                (None, _) => {}
                (Some(tf), _) => self.tf = Some((tf.clone(), raw.tf_from, now)),
            }
            match (raw.price, self.price) {
                (None, Some((p, t))) if fresh(t) => {
                    out.price = Some(p);
                    out.held.push("price");
                }
                (None, _) => {}
                (Some(p), _) => self.price = Some((p, now)),
            }
        }
        // Consistency counts fresh, complete reads only; an incomplete read neither confirms
        // nor breaks the streak.
        if let Some(k) = key_of(raw) {
            if self.key.as_ref() == Some(&k) {
                self.streak = self.streak.saturating_add(1);
            } else {
                self.key = Some(k);
                self.streak = 1;
            }
        }
        out
    }

    /// Symbol and time frame confirmed by [`CONFIRM`] consecutive complete reads.
    pub fn confirmed(&self) -> bool {
        self.streak >= CONFIRM
    }

    /// The platform window is not in front at `now`: true while the last platform read is
    /// recent enough to keep the status (no capture is made meanwhile).
    pub fn in_grace(&self, now: Instant) -> bool {
        self.platform_at
            .is_some_and(|t| now.saturating_duration_since(t) <= HOLD)
    }

    pub fn reset(&mut self) {
        *self = Stable::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::{parse_tf, parse_toolbar_tf};

    fn rd(sym: Option<&str>, tf: Option<&str>, price: Option<f64>) -> Reading {
        Reading {
            page: PageKind::Chart,
            symbol: sym.map_or(SymbolRead::Unknown, |s| SymbolRead::Supported {
                ticker: s.into(),
                exchange: format!("{s}T"),
            }),
            tf: tf.and_then(|t| parse_toolbar_tf(t).or_else(|| parse_tf(t))),
            tf_from: "toolbar",
            price: price.map(|p| (p, PriceFrom::AxisLabel)),
            source: None,
            symbol_from: "window title",
            engine: None,
            ocr_calls: 1,
            read_ms: 1.0,
            held: Vec::new(),
            tf_note: None,
        }
    }

    #[test]
    fn one_failed_read_does_not_flip_the_reading() {
        let t0 = Instant::now();
        let mut s = Stable::default();
        let good = s.on_read(&rd(Some("BTCUSD"), Some("5m"), Some(61_701.0)), t0);
        assert_eq!(good.held, Vec::<&str>::new());
        // 0.5 s later the time frame and price are not read: both held, marked.
        let r = s.on_read(
            &rd(Some("BTCUSD"), None, None),
            t0 + Duration::from_millis(500),
        );
        assert_eq!(r.tf.as_ref().map(TfRead::label), Some("5m".into()));
        assert_eq!(r.price.map(|p| p.0), Some(61_701.0));
        assert_eq!(r.held, ["time frame", "price"]);
        // A read that saw no chart at all: the chart and its fields stay.
        let mut other = rd(None, None, None);
        other.page = PageKind::Other;
        let r = s.on_read(&other, t0 + Duration::from_secs(1));
        assert_eq!(r.page, PageKind::Chart);
        assert_eq!(r.symbol.ticker(), Some("BTCUSD"));
        // After more than 3 s without a good value, nothing is held any more.
        let r = s.on_read(
            &rd(Some("BTCUSD"), None, None),
            t0 + Duration::from_millis(3_600),
        );
        assert!(r.tf.is_none() && r.price.is_none());
    }

    #[test]
    fn two_consistent_reads_before_a_switch() {
        let t0 = Instant::now();
        let ms = |m: u64| t0 + Duration::from_millis(m);
        let mut s = Stable::default();
        s.on_read(&rd(Some("BTCUSD"), Some("5m"), None), ms(0));
        assert!(!s.confirmed(), "one read is not enough");
        s.on_read(&rd(Some("BTCUSD"), Some("5"), None), ms(500));
        assert!(
            s.confirmed(),
            "toolbar 5m and legend 5 are the same interval"
        );
        // A different interval once: not confirmed until it is read again.
        s.on_read(&rd(Some("BTCUSD"), Some("15m"), None), ms(1_000));
        assert!(!s.confirmed());
        // An incomplete read in between neither confirms nor breaks the streak.
        s.on_read(&rd(Some("BTCUSD"), None, None), ms(1_500));
        assert!(!s.confirmed());
        s.on_read(&rd(Some("BTCUSD"), Some("15m"), None), ms(2_000));
        assert!(s.confirmed());
        s.on_read(&rd(Some("BTCUSD"), None, None), ms(2_500));
        assert!(s.confirmed(), "a failed read keeps the confirmed state");
        // D and 1D agree.
        s.on_read(&rd(Some("BTCUSD"), Some("D"), None), ms(3_000));
        s.on_read(&rd(Some("BTCUSD"), Some("1D"), None), ms(3_500));
        assert!(s.confirmed());
    }

    #[test]
    fn brief_foreground_change_keeps_the_status() {
        let t0 = Instant::now();
        let mut s = Stable::default();
        assert!(!s.in_grace(t0), "never read: no grace");
        s.on_read(&rd(Some("BTCUSD"), Some("5m"), None), t0);
        assert!(
            s.in_grace(t0 + Duration::from_millis(900)),
            "popup for 0.9 s"
        );
        assert!(s.in_grace(t0 + Duration::from_secs(3)));
        assert!(
            !s.in_grace(t0 + Duration::from_millis(3_100)),
            "gone for good"
        );
    }
}
