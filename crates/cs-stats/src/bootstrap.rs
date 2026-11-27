//! Circular block bootstrap for the mean of a time-ordered series (e.g. trade results in R).
//!
//! Trades are resampled in blocks of `block` consecutive trades (wrapping around the end), so
//! winning and losing streaks inside a block survive the resampling; an i.i.d. bootstrap would
//! break them up and understate the uncertainty when results are serially correlated. The
//! confidence interval is the percentile interval of the resampled means. Deterministic per seed.

use crate::Rng;

/// Result of [`block_mean_ci`].
#[derive(Debug, Clone, Copy)]
pub struct MeanCi {
    /// Sample mean of the input.
    pub mean: f64,
    /// Lower and upper percentile bounds of the resampled means.
    pub lo: f64,
    pub hi: f64,
    /// Standard deviation of the resampled means (bootstrap standard error).
    pub se: f64,
    pub level: f64,
    pub block: usize,
    pub reps: usize,
}

/// Default block length: `ceil(n^(1/3))`, at least 1 (the usual rate for block bootstraps of the
/// mean).
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn default_block(n: usize) -> usize {
    ((n as f64).cbrt().ceil() as usize).max(1)
}

/// Circular block bootstrap confidence interval for the mean of `xs` (kept in time order).
///
/// `block` 0 means [`default_block`]. Returns `None` for an empty input or `reps == 0`.
/// With one value, the interval is that value.
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn block_mean_ci(
    xs: &[f64],
    block: usize,
    reps: usize,
    level: f64,
    seed: u64,
) -> Option<MeanCi> {
    let n = xs.len();
    if n == 0 || reps == 0 {
        return None;
    }
    let mean = xs.iter().sum::<f64>() / n as f64;
    let b = if block == 0 { default_block(n) } else { block }.min(n);
    // Prefix sums over the series extended circularly by b values: the sum of the block that
    // starts at s is pre[s + b] - pre[s].
    let mut pre = Vec::with_capacity(n + b + 1);
    pre.push(0.0);
    let mut acc = 0.0;
    for k in 0..n + b {
        acc += xs[k % n];
        pre.push(acc);
    }
    let blocks = n.div_ceil(b);
    let mut rng = Rng::new(seed);
    let mut means = Vec::with_capacity(reps);
    for _ in 0..reps {
        let mut sum = 0.0;
        let mut left = n;
        for _ in 0..blocks {
            let s = rng.below(n as u64) as usize;
            let take = left.min(b);
            sum += pre[s + take] - pre[s];
            left -= take;
        }
        means.push(sum / n as f64);
    }
    means.sort_by(f64::total_cmp);
    let m = means.iter().sum::<f64>() / reps as f64;
    let var =
        means.iter().map(|x| (x - m) * (x - m)).sum::<f64>() / reps.max(2).saturating_sub(1) as f64;
    let a = ((1.0 - level) / 2.0).clamp(0.0, 0.5);
    let q = |p: f64| {
        let k = (p * (reps - 1) as f64).round() as usize;
        means[k.min(reps - 1)]
    };
    Some(MeanCi {
        mean,
        lo: q(a),
        hi: q(1.0 - a),
        se: var.sqrt(),
        level,
        block: b,
        reps,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deterministic_and_brackets_the_mean() {
        let xs: Vec<f64> = (0..500)
            .map(|i| if i % 3 == 0 { 2.0 } else { -1.0 })
            .collect();
        let a = block_mean_ci(&xs, 0, 2000, 0.95, 7).unwrap();
        let b = block_mean_ci(&xs, 0, 2000, 0.95, 7).unwrap();
        assert_eq!(a.lo.to_bits(), b.lo.to_bits());
        assert_eq!(a.hi.to_bits(), b.hi.to_bits());
        assert!(a.lo <= a.mean && a.mean <= a.hi, "{a:?}");
        assert_eq!(a.block, 8);
        let c = block_mean_ci(&xs, 0, 2000, 0.95, 8).unwrap();
        assert_ne!(a.lo.to_bits(), c.lo.to_bits());
    }

    #[test]
    fn streaks_widen_the_interval() {
        // Same values, once alternating and once in long streaks: the block bootstrap must give
        // the streaky series a wider interval (an i.i.d. bootstrap would give both the same).
        let alt: Vec<f64> = (0..1000)
            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        let streaky: Vec<f64> = (0..1000)
            .map(|i| if (i / 50) % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        let a = block_mean_ci(&alt, 50, 2000, 0.95, 1).unwrap();
        let s = block_mean_ci(&streaky, 50, 2000, 0.95, 1).unwrap();
        assert!(s.hi - s.lo > 3.0 * (a.hi - a.lo), "{a:?} {s:?}");
    }

    #[test]
    fn constant_series_has_zero_width() {
        let xs = vec![0.5; 40];
        let c = block_mean_ci(&xs, 0, 100, 0.9, 3).unwrap();
        assert!((c.lo - 0.5).abs() < 1e-12 && (c.hi - 0.5).abs() < 1e-12);
        assert!(block_mean_ci(&[], 0, 100, 0.9, 3).is_none());
    }
}
