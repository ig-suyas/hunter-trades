//! Risk calculator: position size from account, risk % and the user's own entry and stop, and
//! the round-trip cost of that position in R with the Phase 3 cost model
//! (`cs_backtest::cost::CostModel`, the same `fill` pricing the backtest uses). Sizing follows
//! the backtest: fixed fractional risk on the distance entry-stop before costs, capped at 10x
//! leverage. It only calculates; nothing is sent anywhere.

use cs_backtest::cost::CostModel;

/// Cost above this many R gets a warning.
pub const WARN_COST_R: f64 = 0.2;
/// Leverage cap (Phase 3 default `max_leverage`).
pub const MAX_LEVERAGE: f64 = 10.0;

#[derive(Debug, Clone, Copy)]
pub struct RiskIn {
    pub account: f64,
    /// Percent, e.g. 1.0 for 1 %.
    pub risk_pct: f64,
    pub entry: f64,
    pub stop: f64,
    /// ATR(14) of the chart time frame (volatility slippage); `NaN` if unknown (treated as 0).
    pub atr: f64,
    /// Funding rate per settlement (positive: positions that gain when price rises pay).
    pub funding_rate: f64,
    /// Settlements the position is expected to be held across.
    pub funding_n: f64,
    pub cost: CostModel,
}

#[derive(Debug, Clone, Copy)]
pub struct RiskOut {
    /// +1 when the stop is below the entry (the position loses if price falls), -1 above.
    pub side: i8,
    pub dist: f64,
    pub qty: f64,
    pub notional: f64,
    pub leverage: f64,
    /// Share of the account actually at risk at the stop before costs (lower than asked when
    /// the leverage cap binds).
    pub risk_frac: f64,
    pub capped: bool,
    /// Round-trip cost per unit split up (price units): fees, half-spreads, slippage, funding.
    pub fee: f64,
    pub spread: f64,
    pub slip: f64,
    pub funding: f64,
    /// Funding that would be received instead of paid (not counted as a cost reduction).
    pub funding_credit: bool,
    /// Total round-trip cost in R and in account currency.
    pub cost_r: f64,
    pub cost_cash: f64,
    pub warn: bool,
}

#[cfg(test)]
impl RiskOut {
    pub fn total_per_unit(&self) -> f64 {
        self.fee + self.spread + self.slip + self.funding
    }
}

/// Size the position and price its round trip (market entry at `entry`, market exit at the
/// stop). `None` for unusable inputs.
pub fn calc(i: &RiskIn) -> Option<RiskOut> {
    let ok = |x: f64| x.is_finite() && x > 0.0;
    if !(ok(i.account) && ok(i.risk_pct) && ok(i.entry) && ok(i.stop)) || i.risk_pct > 100.0 {
        return None;
    }
    let dist = (i.entry - i.stop).abs();
    if dist <= i.entry * 1e-9 {
        return None;
    }
    let side: i8 = if i.stop < i.entry { 1 } else { -1 };
    let want = i.risk_pct / 100.0;
    let risk_frac = want.min(MAX_LEVERAGE * dist / i.entry);
    let capped = risk_frac < want - 1e-15;
    let qty = i.account * risk_frac / dist;
    let notional = qty * i.entry;
    let atr = if i.atr.is_finite() { i.atr } else { 0.0 };
    let ef = i.cost.fill(side, i.entry, atr, true);
    let xf = i.cost.fill(-side, i.stop, atr, true);
    let rate = if i.funding_rate.is_finite() {
        i.funding_rate
    } else {
        0.0
    };
    let n = if i.funding_n.is_finite() {
        i.funding_n.max(0.0)
    } else {
        0.0
    };
    let pay = f64::from(side) * rate * i.entry * n;
    let funding = pay.max(0.0);
    let fee = ef.fee + xf.fee;
    let spread = ef.spread + xf.spread;
    let slip = ef.slip + xf.slip;
    let total = fee + spread + slip + funding;
    let cost_r = total / dist;
    Some(RiskOut {
        side,
        dist,
        qty,
        notional,
        leverage: notional / i.account,
        risk_frac,
        capped,
        fee,
        spread,
        slip,
        funding,
        funding_credit: pay < 0.0,
        cost_r,
        cost_cash: total * qty,
        warn: cost_r > WARN_COST_R,
    })
}

/// Parse a typed number (commas and spaces allowed).
pub fn parse(s: &str) -> Option<f64> {
    let t: String = s
        .chars()
        .filter(|c| !matches!(c, ',' | ' ' | '_'))
        .collect();
    t.parse::<f64>().ok().filter(|x| x.is_finite())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base() -> RiskIn {
        RiskIn {
            account: 10_000.0,
            risk_pct: 1.0,
            entry: 90_000.0,
            stop: 89_100.0,
            atr: 450.0,
            funding_rate: 0.0001,
            funding_n: 1.0,
            cost: CostModel::BINANCE_UM,
        }
    }

    /// Hand calculation (written out in the handoff):
    /// entry fill (buy, market) at 90,000: half spread 90,000 x 0.5 bp = 4.5, slippage
    /// 90,000 x 1 bp + 0.02 x 450 = 18, fill 90,022.5, fee 5 bp = 45.01125;
    /// exit at the stop (sell, market) 89,100: half spread 4.455, slippage 8.91 + 9 = 17.91,
    /// fill 89,077.635, fee 44.5388175; funding 1 x 0.01 % x 90,000 = 9.
    /// Total 143.4150675 per BTC = 0.159350075 R on a 900 stop. Size 100 / 900 = 0.111111 BTC,
    /// notional 10,000, 1.0x; cost 15.935 USDT; no warning.
    #[test]
    fn matches_hand_calculation() {
        let r = calc(&base()).unwrap_or_else(|| panic!("calc"));
        assert_eq!(r.side, 1);
        assert!((r.qty - 0.111_111_111_111).abs() < 1e-9);
        assert!((r.notional - 10_000.0).abs() < 1e-6);
        assert!((r.leverage - 1.0).abs() < 1e-12);
        assert!((r.fee - (45.011_25 + 44.538_817_5)).abs() < 1e-9);
        assert!((r.spread - (4.5 + 4.455)).abs() < 1e-9);
        assert!((r.slip - (18.0 + 17.91)).abs() < 1e-9);
        assert!((r.funding - 9.0).abs() < 1e-9);
        assert!((r.total_per_unit() - 143.415_067_5).abs() < 1e-9);
        assert!((r.cost_r - 0.159_350_075).abs() < 1e-12);
        assert!((r.cost_cash - 15.935_007_5).abs() < 1e-6);
        assert!(!r.warn && !r.capped);
    }

    /// Tight stop: 90,000 / 89,700. Exit fill: half spread 4.485, slippage 8.97 + 9 = 17.97,
    /// fill 89,677.545, fee 44.8387725; total 143.8050225 = 0.479350075 R -> warning.
    /// Size 0.333333 BTC, notional 30,000, 3x.
    #[test]
    fn tight_stop_warns() {
        let r = calc(&RiskIn {
            stop: 89_700.0,
            ..base()
        })
        .unwrap_or_else(|| panic!("calc"));
        assert!((r.cost_r - 0.479_350_075).abs() < 1e-12);
        assert!((r.leverage - 3.0).abs() < 1e-9);
        assert!(r.warn);
    }

    #[test]
    fn stop_above_entry_funding_credit_and_cap() {
        // Stop above the entry: funding at a positive rate is received, so it is not counted.
        let r = calc(&RiskIn {
            stop: 90_900.0,
            ..base()
        })
        .unwrap_or_else(|| panic!("calc"));
        assert_eq!(r.side, -1);
        assert!(r.funding_credit);
        assert!(r.funding.abs() < 1e-12);
        // A 0.05 % stop at 1 % risk needs 20x: capped at 10x, risk halves.
        let c = calc(&RiskIn {
            stop: 89_955.0,
            ..base()
        })
        .unwrap_or_else(|| panic!("calc"));
        assert!(c.capped);
        assert!((c.leverage - 10.0).abs() < 1e-9);
        assert!((c.risk_frac - 0.005).abs() < 1e-12);
        assert!(
            calc(&RiskIn {
                stop: 90_000.0,
                ..base()
            })
            .is_none()
        );
        assert!(
            calc(&RiskIn {
                account: -1.0,
                ..base()
            })
            .is_none()
        );
        assert_eq!(parse("10,000"), Some(10_000.0));
        assert_eq!(parse("abc"), None);
    }
}
