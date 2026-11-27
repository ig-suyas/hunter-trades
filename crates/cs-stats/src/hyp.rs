//! Hypothesis tests for the research phase: a one-sided t-test of a mean with a
//! heteroscedasticity- and autocorrelation-consistent (Newey-West) standard error, the normal tail
//! probability it needs, and Benjamini-Hochberg false-discovery-rate control.

/// Complementary error function, Numerical Recipes `erfcc` (Chebyshev fit, fractional error below
/// 1.2e-7 everywhere, so tail probabilities keep their relative accuracy).
pub fn erfc(x: f64) -> f64 {
    let z = x.abs();
    let t = 1.0 / (1.0 + 0.5 * z);
    let poly = -z * z - 1.265_512_23
        + t * (1.000_023_68
            + t * (0.374_091_96
                + t * (0.096_784_18
                    + t * (-0.186_288_06
                        + t * (0.278_868_07
                            + t * (-1.135_203_98
                                + t * (1.488_515_87 + t * (-0.822_152_23 + t * 0.170_872_77))))))));
    let r = t * poly.exp();
    if x >= 0.0 { r } else { 2.0 - r }
}

/// Upper tail of the standard normal distribution, `P(Z > z)`.
pub fn normal_sf(z: f64) -> f64 {
    if z.is_nan() {
        return f64::NAN;
    }
    0.5 * erfc(z / std::f64::consts::SQRT_2)
}

/// Newey-West lag `floor(4 (n / 100)^(2/9))` (the usual automatic choice).
#[allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
pub fn nw_lag(n: usize) -> usize {
    (4.0 * (n as f64 / 100.0).powf(2.0 / 9.0)).floor() as usize
}

/// Result of [`mean_t`].
#[derive(Debug, Clone, Copy)]
pub struct MeanT {
    pub n: usize,
    pub mean: f64,
    /// Sample standard deviation (n - 1 denominator).
    pub sd: f64,
    /// i.i.d. standard error of the mean, `sd / sqrt(n)`.
    pub se_iid: f64,
    /// Newey-West (Bartlett kernel) standard error of the mean.
    pub se_hac: f64,
    /// The standard error used: `max(se_iid, se_hac)`.
    pub se: f64,
    pub lag: usize,
    pub t: f64,
    /// One-sided p-value of H0 `mean <= 0` against `mean > 0` (normal approximation).
    pub p: f64,
}

/// One-sided t-test of `mean > 0` for a time-ordered series, with the conservative standard
/// error `max(i.i.d., Newey-West)`. Fewer than two values give `NaN` statistics and `p = 1`.
#[allow(clippy::cast_precision_loss)]
pub fn mean_t(xs: &[f64]) -> MeanT {
    let n = xs.len();
    let nan = MeanT {
        n,
        mean: f64::NAN,
        sd: f64::NAN,
        se_iid: f64::NAN,
        se_hac: f64::NAN,
        se: f64::NAN,
        lag: 0,
        t: f64::NAN,
        p: 1.0,
    };
    if n < 2 {
        return MeanT {
            mean: xs.first().copied().unwrap_or(f64::NAN),
            ..nan
        };
    }
    let nf = n as f64;
    let mean = xs.iter().sum::<f64>() / nf;
    let ss: f64 = xs.iter().map(|x| (x - mean) * (x - mean)).sum();
    let sd = (ss / (nf - 1.0)).sqrt();
    let se_iid = sd / nf.sqrt();
    let lag = nw_lag(n).min(n - 1);
    let gamma = |l: usize| -> f64 {
        xs[l..]
            .iter()
            .zip(xs)
            .map(|(a, b)| (a - mean) * (b - mean))
            .sum::<f64>()
            / nf
    };
    let mut var = gamma(0);
    for l in 1..=lag {
        let w = 1.0 - l as f64 / (lag as f64 + 1.0);
        var += 2.0 * w * gamma(l);
    }
    let se_hac = (var.max(0.0) / nf).sqrt();
    let se = se_iid.max(se_hac);
    let t = if se > 0.0 { mean / se } else { f64::NAN };
    let p = if t.is_nan() { 1.0 } else { normal_sf(t) };
    MeanT {
        n,
        mean,
        sd,
        se_iid,
        se_hac,
        se,
        lag,
        t,
        p,
    }
}

/// Result of [`benjamini_hochberg`].
#[derive(Debug, Clone)]
pub struct Bh {
    /// Rejected (discovery) per input, in input order.
    pub reject: Vec<bool>,
    /// BH-adjusted p-values (q-values) per input, capped at 1.
    pub qvalue: Vec<f64>,
    /// Number of rejections.
    pub k: usize,
    /// The p-value threshold actually used (`k q / m`; 0 when nothing is rejected).
    pub threshold: f64,
}

/// Benjamini-Hochberg step-up procedure at level `q` over all `p` (`m = p.len()`). `NaN`
/// p-values are treated as 1.
#[allow(clippy::cast_precision_loss)]
pub fn benjamini_hochberg(p: &[f64], q: f64) -> Bh {
    let m = p.len();
    let clean = |x: f64| if x.is_nan() { 1.0 } else { x.clamp(0.0, 1.0) };
    let mut idx: Vec<usize> = (0..m).collect();
    idx.sort_by(|&a, &b| clean(p[a]).total_cmp(&clean(p[b])).then(a.cmp(&b)));
    let mut k = 0;
    for (r, &i) in idx.iter().enumerate() {
        if clean(p[i]) <= (r + 1) as f64 * q / m as f64 {
            k = r + 1;
        }
    }
    let mut reject = vec![false; m];
    for &i in &idx[..k] {
        reject[i] = true;
    }
    let mut qvalue = vec![1.0; m];
    let mut run = 1.0f64;
    for r in (0..m).rev() {
        let i = idx[r];
        run = run.min(clean(p[i]) * m as f64 / (r + 1) as f64);
        qvalue[i] = run.min(1.0);
    }
    Bh {
        reject,
        qvalue,
        k,
        threshold: if m == 0 { 0.0 } else { k as f64 * q / m as f64 },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, rel: f64) -> bool {
        (a - b).abs() <= rel * b.abs().max(1e-300)
    }

    #[test]
    fn normal_tail_values() {
        assert!(close(normal_sf(0.0), 0.5, 1e-7));
        assert!(close(normal_sf(1.644_853_627), 0.05, 1e-6));
        assert!(close(normal_sf(1.959_963_985), 0.025, 1e-6));
        assert!(close(normal_sf(3.0), 1.349_898_032e-3, 1e-6));
        assert!(close(normal_sf(5.0), 2.866_515_719e-7, 1e-6));
        assert!(close(normal_sf(-1.0), 0.841_344_746, 1e-6));
        assert!(normal_sf(40.0) >= 0.0 && normal_sf(40.0) < 1e-300);
    }

    #[test]
    fn lag_rule() {
        assert_eq!(nw_lag(100), 4);
        assert_eq!(nw_lag(1000), 6);
        assert_eq!(nw_lag(10), 2);
    }

    #[test]
    fn t_test_iid_and_autocorrelated() {
        // Alternating series: negative autocorrelation, HAC SE < iid SE, so iid is used.
        let alt: Vec<f64> = (0..200)
            .map(|i| if i % 2 == 0 { 1.1 } else { -0.9 })
            .collect();
        let r = mean_t(&alt);
        assert!(close(r.mean, 0.1, 1e-12));
        assert!(r.se_hac < r.se_iid);
        assert!(close(r.se, r.se_iid, 1e-12));
        // Long runs: strong positive autocorrelation, HAC SE > iid SE.
        let runs: Vec<f64> = (0..400)
            .map(|i| if (i / 20) % 2 == 0 { 1.2 } else { -1.0 })
            .collect();
        let r = mean_t(&runs);
        assert!(r.se_hac > 1.5 * r.se_iid);
        assert!(close(r.se, r.se_hac, 1e-12));
        assert!(r.p > 0.0 && r.p < 1.0);
        // Hand check of the iid part: [1, 2, 3] -> mean 2, sd 1, se 1/sqrt(3).
        let r = mean_t(&[1.0, 2.0, 3.0]);
        assert!(close(r.sd, 1.0, 1e-12));
        assert!(close(r.se_iid, 1.0 / 3f64.sqrt(), 1e-12));
        // Degenerate inputs.
        assert!((mean_t(&[]).p - 1.0).abs() < 1e-15);
        assert!((mean_t(&[0.5]).p - 1.0).abs() < 1e-15);
        assert!((mean_t(&[0.5, 0.5]).p - 1.0).abs() < 1e-15);
    }

    #[test]
    #[allow(clippy::cast_precision_loss)]
    fn hac_matches_hand_formula() {
        let xs = [0.3, -1.0, 2.0, 0.5, -0.2, 1.5, -1.1, 0.4];
        let n = xs.len() as f64;
        let m = xs.iter().sum::<f64>() / n;
        let g = |l: usize| {
            (l..xs.len())
                .map(|t| (xs[t] - m) * (xs[t - l] - m))
                .sum::<f64>()
                / n
        };
        let lag = nw_lag(xs.len());
        assert_eq!(lag, 2);
        let var = g(0) + 2.0 * (2.0 / 3.0) * g(1) + 2.0 * (1.0 / 3.0) * g(2);
        let r = mean_t(&xs);
        assert!(close(r.se_hac, (var / n).sqrt(), 1e-12));
    }

    #[test]
    fn bh_textbook_example() {
        // Benjamini & Hochberg (1995) example: 15 p-values, q = 0.05 rejects the 4 smallest.
        let p = [
            0.0001, 0.0004, 0.0019, 0.0095, 0.0201, 0.0278, 0.0298, 0.0344, 0.0459, 0.3240, 0.4262,
            0.5719, 0.6528, 0.7590, 1.0000,
        ];
        let r = benjamini_hochberg(&p, 0.05);
        assert_eq!(r.k, 4);
        assert_eq!(r.reject.iter().filter(|x| **x).count(), 4);
        assert!(r.reject[..4].iter().all(|x| *x));
        assert!(close(r.threshold, 4.0 * 0.05 / 15.0, 1e-12));
        // q-values: monotone in p, smallest = 0.0001 * 15 / 1 = 0.0015.
        assert!(close(r.qvalue[0], 0.0015, 1e-12));
        for w in 1..p.len() {
            assert!(r.qvalue[w] >= r.qvalue[w - 1] - 1e-15);
        }
        assert!(r.qvalue.iter().all(|q| *q <= 1.0));
        // Order of inputs does not matter; NaN counts as 1.
        let mut shuffled = p.to_vec();
        shuffled.reverse();
        shuffled.push(f64::NAN);
        let s = benjamini_hochberg(&shuffled, 0.05);
        assert_eq!(s.k, 4);
        assert!(!s.reject[15]);
        // Step-up: a p above its own line is still rejected when a larger rank passes.
        let r = benjamini_hochberg(&[0.04, 0.041, 0.042], 0.05);
        assert_eq!(r.k, 3);
        assert_eq!(benjamini_hochberg(&[], 0.1).k, 0);
    }
}
