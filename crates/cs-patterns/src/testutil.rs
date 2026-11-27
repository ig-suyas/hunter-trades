//! Candle builders for tests and a synthetic market generator for the prefix-invariance tests.
//! Library code (not `cfg(test)`) so integration tests and benches can use it.

use cs_core::Candle;

use crate::util::Rng;

/// 15 minutes in ms; synthetic candles are spaced like the main study time frame.
pub const STEP_MS: i64 = 15 * 60_000;
/// 2024-11-01 00:00 UTC.
pub const T0: i64 = 1_730_419_200_000;

/// A candle with the given OHLC, unit volume, open time 0.
pub fn ohlc(o: f64, h: f64, l: f64, c: f64) -> Candle {
    ohlcv(o, h, l, c, 1.0)
}

pub fn ohlcv(o: f64, h: f64, l: f64, c: f64, v: f64) -> Candle {
    Candle {
        open_time: 0,
        open: o,
        high: h,
        low: l,
        close: c,
        volume: v,
        quote_volume: v * c,
        trades: 1,
        taker_buy_base: v * 0.5,
        taker_buy_quote: v * c * 0.5,
    }
}

/// Assign consecutive open times starting at [`T0`].
pub fn timed(mut cs: Vec<Candle>) -> Vec<Candle> {
    for (i, c) in cs.iter_mut().enumerate() {
        c.open_time = T0 + i64::try_from(i).unwrap_or(0) * STEP_MS;
    }
    cs
}

/// Candles that walk through `path` (open = previous point, close = point, wicks 0.5 beyond).
pub fn path_candles(path: &[f64]) -> Vec<Candle> {
    let mut v = Vec::with_capacity(path.len());
    let mut prev = path.first().copied().unwrap_or(0.0);
    for &p in path {
        let (o, c) = (prev, p);
        v.push(ohlc(o, o.max(c) + 0.5, o.min(c) - 0.5, c));
        prev = p;
    }
    timed(v)
}

/// Synthetic market: a regime-switching random walk (trends, ranges, volatility bursts,
/// spikes and volume surges) so that every detector gets exercised. Deterministic per seed.
pub fn synthetic(n: usize, seed: u64) -> Vec<Candle> {
    let mut r = Rng::new(seed);
    let mut v = Vec::with_capacity(n);
    let mut px = 30_000.0 + r.f64() * 40_000.0;
    let mut drift = 0.0;
    let mut vol = 0.002;
    let mut regime_left = 0usize;
    let mut range_mid = px;
    let mut ranging = false;
    for i in 0..n {
        if regime_left == 0 {
            regime_left = 30 + r.below(300) as usize;
            match r.below(4) {
                0 => {
                    ranging = true;
                    range_mid = px;
                    drift = 0.0;
                }
                1 => {
                    ranging = false;
                    drift = 0.0008 * (0.5 + r.f64());
                }
                2 => {
                    ranging = false;
                    drift = -0.0008 * (0.5 + r.f64());
                }
                _ => {
                    ranging = false;
                    drift = 0.0;
                }
            }
            vol = 0.001 + r.f64() * 0.004;
        }
        regime_left -= 1;
        let o = px;
        let mut ret = drift + vol * r.normal();
        if ranging {
            ret += -0.08 * (px - range_mid) / px;
        }
        if r.below(80) == 0 {
            ret += vol * 4.0 * r.normal();
        }
        let c = (o * (1.0 + ret)).max(1.0);
        let w = vol * o;
        let mut h = o.max(c) + w * r.f64() * r.f64() * 2.0;
        let mut l = o.min(c) - w * r.f64() * r.f64() * 2.0;
        if r.below(40) == 0 {
            // wick spike (sweep-like)
            if r.below(2) == 0 {
                h += w * 3.0 * r.f64();
            } else {
                l -= w * 3.0 * r.f64();
            }
        }
        // Occasionally emit an exact doji or a tiny inside bar.
        let (o, c) = if r.below(25) == 0 { (o, o) } else { (o, c) };
        let base_vol = 100.0 * (1.0 + (c - o).abs() / w.max(1e-9));
        let volume = base_vol * (0.5 + r.f64()) * if r.below(30) == 0 { 4.0 } else { 1.0 };
        let mut k = ohlcv(o, h, l.max(0.5), c, volume);
        k.open_time = T0 + i64::try_from(i).unwrap_or(0) * STEP_MS;
        v.push(k);
        px = c;
    }
    v
}
