//! Trading costs: taker fee per fill, half the quoted spread and slippage on every market fill,
//! and funding on perpetual positions held across funding times.
//!
//! Default numbers and where they come from are documented in `notes/data-sources.md`
//! (section "Costs used by the backtester"). Every fill is charged the **taker** fee, also limit
//! fills that would really pay the lower maker fee (pessimistic). Market fills (next-open entries,
//! stop, target, trailing, time-stop and end-of-data exits) also pay half the quoted spread plus
//! slippage `slip_bps` of the price plus `slip_atr` x ATR. Limit fills happen at the limit price
//! and pay the fee only.
//!
//! The stress multiplier `mult` scales fee, spread and slippage (not funding, which is a
//! historical cash flow that can be a credit; scaling a credit would make "higher costs" raise
//! expectancy for shorts). Prices never depend on the costs: stops, targets and R distances are
//! set from raw prices, so changing costs never changes which trades happen or when they exit.

/// Cost parameters. Basis points are of the fill price.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CostModel {
    /// Taker fee per fill (entry and exit), in bps of the fill notional.
    pub taker_fee_bps: f64,
    /// Full quoted spread in bps; a market fill pays half of it.
    pub spread_bps: f64,
    /// Fixed slippage per market fill, in bps of the price.
    pub slip_bps: f64,
    /// Volatility slippage per market fill, as a fraction of ATR(14) of the setup time frame at
    /// the last closed bar before the fill.
    pub slip_atr: f64,
    /// Stress multiplier for fee, spread and slippage (1 = base case, 2 = double costs).
    pub mult: f64,
    /// Charge funding at the historical rates for positions open across a funding time.
    pub funding: bool,
}

impl CostModel {
    /// Binance USDS-M perpetuals, VIP 0 taker without BNB discount, conservative spread and
    /// slippage (see `notes/data-sources.md`).
    pub const BINANCE_UM: CostModel = CostModel {
        taker_fee_bps: 5.0,
        spread_bps: 1.0,
        slip_bps: 1.0,
        slip_atr: 0.02,
        mult: 1.0,
        funding: true,
    };

    /// Binance spot, VIP 0 taker without BNB discount; no funding.
    pub const BINANCE_SPOT: CostModel = CostModel {
        taker_fee_bps: 10.0,
        spread_bps: 1.0,
        slip_bps: 1.0,
        slip_atr: 0.02,
        mult: 1.0,
        funding: false,
    };

    /// No costs at all (for tests and gross results).
    pub const ZERO: CostModel = CostModel {
        taker_fee_bps: 0.0,
        spread_bps: 0.0,
        slip_bps: 0.0,
        slip_atr: 0.0,
        mult: 1.0,
        funding: false,
    };

    /// Same model with the stress multiplier set.
    #[must_use]
    pub fn with_mult(mut self, m: f64) -> CostModel {
        self.mult = m;
        self
    }

    /// Price a fill. `side` +1 buy, -1 sell; `raw` the price before costs; `atr` the ATR used for
    /// volatility slippage; `market` false for a resting limit order filled at its limit.
    pub fn fill(&self, side: i8, raw: f64, atr: f64, market: bool) -> Fill {
        let m = self.mult.max(0.0);
        let (spread, slip) = if market {
            let a = if atr.is_finite() { atr } else { 0.0 };
            (
                raw * self.spread_bps * 0.5 * m / 1e4,
                raw * self.slip_bps * m / 1e4 + self.slip_atr * a * m,
            )
        } else {
            (0.0, 0.0)
        };
        let price = raw + f64::from(side) * (spread + slip);
        Fill {
            raw,
            price,
            spread,
            slip,
            fee: price.abs() * self.taker_fee_bps * m / 1e4,
            market,
        }
    }
}

/// One priced fill, per unit of the base asset. `price = raw + side * (spread + slip)`.
#[derive(Debug, Clone, Copy)]
pub struct Fill {
    pub raw: f64,
    pub price: f64,
    /// Half-spread paid (price units, >= 0).
    pub spread: f64,
    /// Slippage paid (price units, >= 0).
    pub slip: f64,
    /// Fee paid (price units, >= 0).
    pub fee: f64,
    pub market: bool,
}

/// One funding settlement: time (UTC ms) and rate (positive: longs pay shorts).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Funding {
    pub time: i64,
    pub rate: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fill_prices() {
        let c = CostModel {
            taker_fee_bps: 5.0,
            spread_bps: 2.0,
            slip_bps: 1.0,
            slip_atr: 0.1,
            mult: 1.0,
            funding: false,
        };
        let b = c.fill(1, 100.0, 0.5, true);
        // half spread 0.01, slip 0.01 + 0.05
        assert!((b.spread - 0.01).abs() < 1e-12);
        assert!((b.slip - 0.06).abs() < 1e-12);
        assert!((b.price - 100.07).abs() < 1e-12);
        assert!((b.fee - 100.07 * 5e-4).abs() < 1e-12);
        let s = c.fill(-1, 100.0, 0.5, true);
        assert!((s.price - 99.93).abs() < 1e-12);
        let l = c.fill(1, 100.0, 0.5, false);
        assert!((l.price - 100.0).abs() < 1e-12 && l.spread.abs() < 1e-15);
        assert!((l.fee - 0.05).abs() < 1e-12);
        let d = c.with_mult(2.0).fill(1, 100.0, 0.5, true);
        assert!((d.price - 100.14).abs() < 1e-12);
    }
}
