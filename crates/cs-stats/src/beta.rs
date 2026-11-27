//! Beta-Binomial credible interval of a hit rate.
//!
//! With `h` hits in `n` trials and a uniform prior Beta(1, 1), the posterior of the rate is
//! Beta(1 + h, 1 + n - h). [`credible_interval`] returns its equal-tailed interval (for example
//! the 10th and 90th percentiles for an 80 % interval). The regularised incomplete beta function
//! uses the continued fraction of Numerical Recipes (Lentz's method) and the quantile is found
//! by bisection, which is exact to about 1e-12 and plenty fast for a handful of rules.

/// `ln Gamma(x)` for `x > 0` (Lanczos approximation, g = 7, n = 9; relative error < 1e-14).
pub fn ln_gamma(x: f64) -> f64 {
    const G: f64 = 7.0;
    const C: [f64; 9] = [
        0.999_999_999_999_809_9,
        676.520_368_121_885_1,
        -1_259.139_216_722_402_8,
        771.323_428_777_653_1,
        -176.615_029_162_140_6,
        12.507_343_278_686_905,
        -0.138_571_095_265_720_12,
        9.984_369_578_019_572e-6,
        1.505_632_735_149_311_6e-7,
    ];
    if x < 0.5 {
        // Reflection formula.
        let s = (std::f64::consts::PI * x).sin();
        return std::f64::consts::PI.ln() - s.abs().ln() - ln_gamma(1.0 - x);
    }
    let x = x - 1.0;
    let mut a = C[0];
    let t = x + G + 0.5;
    for (i, c) in C.iter().enumerate().skip(1) {
        #[allow(clippy::cast_precision_loss)]
        let k = i as f64;
        a += c / (x + k);
    }
    0.5 * (2.0 * std::f64::consts::PI).ln() + (x + 0.5) * t.ln() - t + a.ln()
}

/// Continued fraction for the incomplete beta function (Numerical Recipes `betacf`).
fn beta_cf(a: f64, b: f64, x: f64) -> f64 {
    const TINY: f64 = 1e-300;
    let qab = a + b;
    let qap = a + 1.0;
    let qam = a - 1.0;
    let mut c = 1.0;
    let mut d = 1.0 - qab * x / qap;
    if d.abs() < TINY {
        d = TINY;
    }
    d = 1.0 / d;
    let mut h = d;
    for m in 1..=400u32 {
        let m = f64::from(m);
        let m2 = 2.0 * m;
        let aa = m * (b - m) * x / ((qam + m2) * (a + m2));
        d = 1.0 + aa * d;
        if d.abs() < TINY {
            d = TINY;
        }
        c = 1.0 + aa / c;
        if c.abs() < TINY {
            c = TINY;
        }
        d = 1.0 / d;
        h *= d * c;
        let aa = -(a + m) * (qab + m) * x / ((a + m2) * (qap + m2));
        d = 1.0 + aa * d;
        if d.abs() < TINY {
            d = TINY;
        }
        c = 1.0 + aa / c;
        if c.abs() < TINY {
            c = TINY;
        }
        d = 1.0 / d;
        let del = d * c;
        h *= del;
        if (del - 1.0).abs() < 1e-15 {
            break;
        }
    }
    h
}

/// Regularised incomplete beta function `I_x(a, b)` (the Beta(a, b) distribution function).
pub fn inc_beta(a: f64, b: f64, x: f64) -> f64 {
    if !(a > 0.0 && b > 0.0) || x.is_nan() {
        return f64::NAN;
    }
    if x <= 0.0 {
        return 0.0;
    }
    if x >= 1.0 {
        return 1.0;
    }
    let ln_front = ln_gamma(a + b) - ln_gamma(a) - ln_gamma(b) + a * x.ln() + b * (1.0 - x).ln();
    let front = ln_front.exp();
    if x < (a + 1.0) / (a + b + 2.0) {
        front * beta_cf(a, b, x) / a
    } else {
        1.0 - front * beta_cf(b, a, 1.0 - x) / b
    }
}

/// Quantile of Beta(a, b): the `x` with `I_x(a, b) = p`, by bisection.
pub fn beta_quantile(a: f64, b: f64, p: f64) -> f64 {
    if a <= 0.0 || b <= 0.0 || a.is_nan() || b.is_nan() || !(0.0..=1.0).contains(&p) {
        return f64::NAN;
    }
    let (mut lo, mut hi) = (0.0f64, 1.0f64);
    for _ in 0..200 {
        let mid = f64::midpoint(lo, hi);
        if inc_beta(a, b, mid) < p {
            lo = mid;
        } else {
            hi = mid;
        }
        if hi - lo < 1e-14 {
            break;
        }
    }
    f64::midpoint(lo, hi)
}

/// Equal-tailed credible interval of a hit rate with `hits` of `n` and a uniform prior:
/// posterior Beta(1 + hits, 1 + n - hits), interval `level` (0.8 = 10th to 90th percentile).
/// `None` when `hits > n` or the level is not in (0, 1).
pub fn credible_interval(hits: u64, n: u64, level: f64) -> Option<(f64, f64)> {
    if hits > n || !(level > 0.0 && level < 1.0) {
        return None;
    }
    #[allow(clippy::cast_precision_loss)]
    let (a, b) = (1.0 + hits as f64, 1.0 + (n - hits) as f64);
    let tail = (1.0 - level) / 2.0;
    Some((beta_quantile(a, b, tail), beta_quantile(a, b, 1.0 - tail)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: f64, b: f64, tol: f64) -> bool {
        (a - b).abs() <= tol
    }

    #[test]
    fn gamma_and_beta_known_values() {
        // ln Gamma(n) = ln (n-1)!
        assert!(close(ln_gamma(1.0), 0.0, 1e-13));
        assert!(close(ln_gamma(5.0), 24f64.ln(), 1e-12));
        assert!(close(
            ln_gamma(0.5),
            std::f64::consts::PI.sqrt().ln(),
            1e-12
        ));
        // Beta(1, 1) is uniform: I_x = x.
        for x in [0.1, 0.37, 0.5, 0.9] {
            assert!(close(inc_beta(1.0, 1.0, x), x, 1e-12));
        }
        // Beta(a, 1): I_x = x^a; Beta(1, b): I_x = 1 - (1 - x)^b.
        assert!(close(inc_beta(3.0, 1.0, 0.6), 0.6f64.powi(3), 1e-12));
        assert!(close(inc_beta(1.0, 4.0, 0.3), 1.0 - 0.7f64.powi(4), 1e-12));
        // Beta(2, 2): I_x = 3x^2 - 2x^3.
        let x: f64 = 0.27;
        assert!(close(
            inc_beta(2.0, 2.0, x),
            3.0 * x * x - 2.0 * x * x * x,
            1e-12
        ));
        // Symmetry I_x(a, b) = 1 - I_{1-x}(b, a) at a large, skewed case.
        assert!(close(
            inc_beta(120.0, 35.0, 0.8),
            1.0 - inc_beta(35.0, 120.0, 0.2),
            1e-11
        ));
    }

    #[test]
    fn quantiles_invert_the_distribution() {
        assert!(close(beta_quantile(1.0, 1.0, 0.1), 0.1, 1e-12));
        assert!(close(beta_quantile(2.0, 2.0, 0.5), 0.5, 1e-12));
        // Beta(a, 1): quantile p^(1/a).
        assert!(close(
            beta_quantile(4.0, 1.0, 0.9),
            0.9f64.powf(0.25),
            1e-12
        ));
        for (a, b, p) in [(31.0, 71.0, 0.1), (31.0, 71.0, 0.9), (150.0, 85.0, 0.25)] {
            let q = beta_quantile(a, b, p);
            assert!(close(inc_beta(a, b, q), p, 1e-10), "{a} {b} {p}");
        }
    }

    #[test]
    fn credible_interval_hand_values() {
        // 0 of 0: the prior itself, Uniform(0, 1): 10th and 90th percentiles 0.1 and 0.9.
        let (lo, hi) = credible_interval(0, 0, 0.8).unwrap_or((f64::NAN, f64::NAN));
        assert!(close(lo, 0.1, 1e-12) && close(hi, 0.9, 1e-12));
        // 0 of 1: Beta(1, 2), CDF 1 - (1 - x)^2 -> quantile 1 - sqrt(1 - p).
        let (lo, hi) = credible_interval(0, 1, 0.8).unwrap_or((f64::NAN, f64::NAN));
        assert!(close(lo, 1.0 - 0.9f64.sqrt(), 1e-12));
        assert!(close(hi, 1.0 - 0.1f64.sqrt(), 1e-12));
        // 40 of 100: interval around the posterior mean 41/102, inside (0.34, 0.47).
        let (lo, hi) = credible_interval(40, 100, 0.8).unwrap_or((f64::NAN, f64::NAN));
        assert!(
            lo > 0.34 && lo < 0.36 && hi > 0.45 && hi < 0.47,
            "{lo} {hi}"
        );
        assert!(credible_interval(5, 4, 0.8).is_none());
        assert!(credible_interval(1, 4, 1.0).is_none());
    }
}
