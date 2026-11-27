//! Synthetic data for tests and checks: 1-minute random walks (a martingale in price, so any
//! entry rule without information has zero expected gross P&L), a regime-switching variant that
//! makes the detectors fire, strict aggregation to a higher time frame, and funding rows.
//! Library code (not `cfg(test)`) so integration tests and the CLI can use it.

use cs_core::Candle;
use cs_stats::Rng;

use crate::cost::Funding;

/// 2024-11-01 00:00 UTC.
pub const T0: i64 = 1_730_419_200_000;
pub const MINUTE_MS: i64 = 60_000;

fn normal(r: &mut Rng) -> f64 {
    let u1 = r.f64().max(1e-300);
    let u2 = r.f64();
    (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos()
}

/// A 1-minute candle (volume 1).
pub fn m1(t: i64, o: f64, h: f64, l: f64, c: f64) -> Candle {
    Candle {
        open_time: t,
        open: o,
        high: h,
        low: l,
        close: c,
        volume: 1.0,
        quote_volume: c,
        trades: 1,
        taker_buy_base: 0.5,
        taker_buy_quote: 0.5 * c,
    }
}

/// Driftless 1-minute random walk starting at `T0`: each minute is 6 sub-steps
/// `p *= 1 + s * z` (z standard normal, `s = sigma_1m / sqrt(6)`), so price is a martingale;
/// open = previous close (no gaps, like a 24/7 market); high/low are the path extremes.
pub fn random_walk_1m(n: usize, p0: f64, sigma_1m: f64, seed: u64) -> Vec<Candle> {
    regime_walk_1m(n, p0, sigma_1m, 0.0, seed)
}

/// Like [`random_walk_1m`], plus regimes so that detectors see trends, ranges and volatility
/// bursts: every 300-3000 minutes a new drift in `[-trend, trend]` per minute (as a share of
/// price) and a volatility factor in {0.6, 1, 2} are drawn. With `trend = 0` it is a martingale.
pub fn regime_walk_1m(n: usize, p0: f64, sigma_1m: f64, trend: f64, seed: u64) -> Vec<Candle> {
    const SUB: usize = 6;
    let mut r = Rng::new(seed);
    let mut v = Vec::with_capacity(n);
    let mut p = p0;
    let mut left = 0usize;
    let mut drift = 0.0;
    let mut volf = 1.0;
    let s = sigma_1m / (SUB as f64).sqrt();
    for i in 0..n {
        if left == 0 {
            left = 300 + r.below(2700) as usize;
            if trend > 0.0 {
                drift = (2.0 * r.f64() - 1.0) * trend;
                volf = [0.6, 1.0, 2.0][r.below(3) as usize];
            }
        }
        left -= 1;
        let o = p;
        let (mut h, mut l) = (p, p);
        for _ in 0..SUB {
            p *= 1.0 + drift / SUB as f64 + s * volf * normal(&mut r);
            h = h.max(p);
            l = l.min(p);
        }
        v.push(m1(T0 + i as i64 * MINUTE_MS, o, h, l, p));
    }
    v
}

/// Strict aggregation of 1-minute candles into UTC-aligned candles `tf_ms` long: a candle is
/// emitted only if all its minutes exist.
pub fn aggregate(m: &[Candle], tf_ms: i64) -> Vec<Candle> {
    let need = (tf_ms / MINUTE_MS) as usize;
    let mut out = Vec::with_capacity(m.len() / need.max(1) + 1);
    let mut i = 0;
    while i < m.len() {
        let b = m[i].open_time.div_euclid(tf_ms) * tf_ms;
        let mut j = i;
        while j < m.len() && m[j].open_time < b + tf_ms {
            j += 1;
        }
        let s = &m[i..j];
        if s.len() == need && s[0].open_time == b {
            let mut c = s[0];
            c.open_time = b;
            for x in &s[1..] {
                c.high = c.high.max(x.high);
                c.low = c.low.min(x.low);
                c.close = x.close;
                c.volume += x.volume;
                c.quote_volume += x.quote_volume;
                c.taker_buy_base += x.taker_buy_base;
                c.taker_buy_quote += x.taker_buy_quote;
                c.trades += x.trades;
            }
            out.push(c);
        }
        i = j;
    }
    out
}

/// Funding every 8 hours (00/08/16 UTC) at a constant rate between `from` and `to`.
pub fn funding_8h(from: i64, to: i64, rate: f64) -> Vec<Funding> {
    let step = 8 * 3_600_000;
    let mut t = from.div_euclid(step) * step;
    let mut v = Vec::new();
    while t <= to {
        if t >= from {
            v.push(Funding { time: t, rate });
        }
        t += step;
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aggregation_is_consistent() {
        let m = random_walk_1m(15 * 40 + 7, 100.0, 0.001, 3);
        let b = aggregate(&m, 15 * MINUTE_MS);
        assert_eq!(b.len(), 40);
        for (k, c) in b.iter().enumerate() {
            let s = &m[k * 15..k * 15 + 15];
            assert_eq!(c.open_time, s[0].open_time);
            assert!(cs_core::price_eq(c.open, s[0].open));
            assert!(cs_core::price_eq(c.close, s[14].close));
            let h = s.iter().map(|x| x.high).fold(f64::MIN, f64::max);
            assert!(cs_core::price_eq(c.high, h));
        }
        for w in m.windows(2) {
            assert!(cs_core::price_eq(w[1].open, w[0].close));
        }
    }
}
