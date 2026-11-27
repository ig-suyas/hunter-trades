//! Price handshake: the price on screen must agree with the exchange feed before the overlay
//! shows context for it. A platform index (`TradingView`'s BTCUSD) and a perpetual (Binance
//! BTCUSDT) differ by a few hundredths of a percent; a wrong symbol or a stale feed differs by
//! far more.

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Handshake {
    /// Within tolerance; `diff_pct` = (screen - feed) / feed x 100.
    Match { diff_pct: f64 },
    /// Outside tolerance.
    Mismatch {
        screen: f64,
        feed: f64,
        diff_pct: f64,
    },
    /// No price read from the screen (context is shown, marked unchecked).
    NoScreenPrice,
    /// The feed has no price yet (warming up).
    NoFeedPrice,
}

impl Handshake {
    pub fn is_mismatch(&self) -> bool {
        matches!(self, Handshake::Mismatch { .. })
    }
}

/// Compares a screen price with the feed price; `tol_pct` in percent (0.3 = 0.3 %).
pub fn check_price(screen: Option<f64>, feed: f64, tol_pct: f64) -> Handshake {
    let Some(s) = screen.filter(|s| s.is_finite() && *s > 0.0) else {
        return Handshake::NoScreenPrice;
    };
    if !(feed.is_finite() && feed > 0.0) {
        return Handshake::NoFeedPrice;
    }
    let diff_pct = (s - feed) / feed * 100.0;
    if diff_pct.abs() <= tol_pct {
        Handshake::Match { diff_pct }
    } else {
        Handshake::Mismatch {
            screen: s,
            feed,
            diff_pct,
        }
    }
}

/// Debounce: one misread label must not flash SOURCE MISMATCH, so a mismatch is reported only
/// after `need` mismatching readings in a row; one matching reading clears it.
#[derive(Debug, Clone, Copy, Default)]
pub struct MismatchGate {
    streak: u32,
    pub need: u32,
}

impl MismatchGate {
    pub fn new(need: u32) -> MismatchGate {
        MismatchGate {
            streak: 0,
            need: need.max(1),
        }
    }

    /// Feeds one handshake result; returns true while SOURCE MISMATCH should be shown.
    pub fn update(&mut self, h: Handshake) -> bool {
        match h {
            Handshake::Mismatch { .. } => self.streak = self.streak.saturating_add(1),
            Handshake::Match { .. } => self.streak = 0,
            // No information: keep the current state.
            Handshake::NoScreenPrice | Handshake::NoFeedPrice => {}
        }
        self.streak >= self.need
    }

    pub fn active(&self) -> bool {
        self.streak >= self.need
    }

    pub fn reset(&mut self) {
        self.streak = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tolerance_edges() {
        assert!(matches!(
            check_price(Some(60_626.47), 60_659.2, 0.3),
            Handshake::Match { .. }
        ));
        // 0.29 % inside, 0.31 % outside.
        assert!(matches!(
            check_price(Some(100.29), 100.0, 0.3),
            Handshake::Match { .. }
        ));
        assert!(check_price(Some(100.31), 100.0, 0.3).is_mismatch());
        assert!(check_price(Some(99.69), 100.0, 0.3).is_mismatch());
        assert_eq!(check_price(None, 100.0, 0.3), Handshake::NoScreenPrice);
        assert_eq!(
            check_price(Some(1.0), f64::NAN, 0.3),
            Handshake::NoFeedPrice
        );
        // ETH price against the BTC feed.
        let h = check_price(Some(1_704.15), 60_659.2, 0.3);
        assert!(h.is_mismatch());
    }

    #[test]
    fn gate_needs_two_in_a_row() {
        let mut g = MismatchGate::new(2);
        let bad = check_price(Some(110.0), 100.0, 0.3);
        let good = check_price(Some(100.0), 100.0, 0.3);
        assert!(!g.update(bad));
        assert!(!g.update(good));
        assert!(!g.update(bad));
        assert!(g.update(bad));
        assert!(
            g.update(Handshake::NoScreenPrice),
            "no information keeps the state"
        );
        assert!(!g.update(good));
    }
}
