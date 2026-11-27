//! Position size for an alert: the quantity that loses `risk %` of the account if the price
//! moves from the entry to the stop (before costs), capped at 10x leverage, exactly as the
//! backtest sizes trades (`cs_backtest::sim::Sizing`: fixed fractional risk, leverage cap), and
//! the estimated round-trip cost in R at this distance with the Phase 3 cost model.

use cs_backtest::cost::CostModel;

/// Leverage cap (the backtest's default `max_leverage`).
pub const MAX_LEVERAGE: f64 = 10.0;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Size {
    /// Quantity of the base asset.
    pub qty: f64,
    /// Notional at the entry price.
    pub notional: f64,
    /// Share of the account at risk between entry and stop (lower than asked when capped).
    pub risk_frac: f64,
    pub capped: bool,
}

/// `None` for unusable inputs (non-positive or non-finite numbers, entry equal to stop, risk
/// above 100 %).
pub fn size(account: f64, risk_pct: f64, entry: f64, stop: f64) -> Option<Size> {
    let ok = |x: f64| x.is_finite() && x > 0.0;
    if !(ok(account) && ok(risk_pct) && ok(entry) && ok(stop)) || risk_pct > 100.0 {
        return None;
    }
    let dist = (entry - stop).abs();
    if dist <= entry * 1e-9 {
        return None;
    }
    let want = risk_pct / 100.0;
    let risk_frac = want.min(MAX_LEVERAGE * dist / entry);
    let qty = account * risk_frac / dist;
    Some(Size {
        qty,
        notional: qty * entry,
        risk_frac,
        capped: risk_frac < want - 1e-15,
    })
}

/// Estimated round-trip cost per unit in R for a market entry at `entry` and a market exit at
/// `stop` (fee, half spread and slippage on both fills; ATR slippage with the signal candle's
/// ATR; funding not included because the holding time is unknown).
pub fn cost_r(cost: CostModel, dir: i8, entry: f64, stop: f64, atr: f64) -> f64 {
    let risk = (entry - stop).abs();
    if !(risk > 0.0 && entry.is_finite() && stop.is_finite()) {
        return f64::NAN;
    }
    let a = if atr.is_finite() { atr } else { 0.0 };
    let ef = cost.fill(dir, entry, a, true);
    let xf = cost.fill(-dir, stop, a, true);
    (ef.fee + ef.spread + ef.slip + xf.fee + xf.spread + xf.slip) / risk
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64) -> bool {
        (a - b).abs() <= 1e-9 * b.abs().max(1.0)
    }

    /// Hand calculation: account 10,000 USDT, 1 %, entry 90,000, stop 89,100: risk 100 USDT over
    /// 900 per BTC = 0.111111 BTC, notional 10,000 (1.0x).
    #[test]
    fn matches_hand_calculation() {
        let s = size(10_000.0, 1.0, 90_000.0, 89_100.0).unwrap();
        assert!(close(s.qty, 100.0 / 900.0));
        assert!(close(s.notional, 10_000.0));
        assert!(close(s.risk_frac, 0.01));
        assert!(!s.capped);
        // Sell side: stop above the entry, same distance, same size.
        let t = size(10_000.0, 1.0, 90_000.0, 90_900.0).unwrap();
        assert!(close(t.qty, s.qty));
        // 25,000 at 0.5 %: 125 USDT over 300 = 0.416667 ETH at 3,000 (1,250 notional... 0.42x).
        let e = size(25_000.0, 0.5, 3_000.0, 2_700.0).unwrap();
        assert!(close(e.qty, 125.0 / 300.0));
        assert!(close(e.notional, 1_250.0));
        // Stop 45 away on 90,000 would need 20x: capped at 10x, risk 0.5 %.
        let c = size(10_000.0, 1.0, 90_000.0, 89_955.0).unwrap();
        assert!(c.capped);
        assert!(close(c.notional, 100_000.0));
        assert!(close(c.risk_frac, 0.005));
        assert!(close(c.qty, 100_000.0 / 90_000.0));
        assert!(size(10_000.0, 1.0, 90_000.0, 90_000.0).is_none());
        assert!(size(-1.0, 1.0, 90_000.0, 89_000.0).is_none());
        assert!(size(10_000.0, 150.0, 90_000.0, 89_000.0).is_none());
    }

    /// Same numbers as the old calculator's hand check (fee 5 bps, half spread 0.5 bp, slippage
    /// 1 bp + 0.02 ATR): entry 90,000, stop 89,100, ATR 450 -> 134.4150675 per BTC without
    /// funding = 0.149350075 R.
    #[test]
    fn cost_matches_hand_calculation() {
        let r = cost_r(CostModel::BINANCE_UM, 1, 90_000.0, 89_100.0, 450.0);
        assert!(close(r, 134.415_067_5 / 900.0), "{r}");
        let r2 = cost_r(
            CostModel::BINANCE_UM.with_mult(2.0),
            1,
            90_000.0,
            89_100.0,
            450.0,
        );
        assert!(r2 > 1.99 * r && r2 < 2.01 * r);
    }
}
