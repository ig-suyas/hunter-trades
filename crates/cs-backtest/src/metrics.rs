//! Summary statistics of a run: counts, win rate, average win/loss, expectancy (net and gross of
//! costs), profit factor, drawdowns, streaks, exposure, time in trade, a block-bootstrap
//! confidence interval for the expectancy, breakdowns by regime, UTC hour and month, and sanity
//! flags that mark results too good to be real (a lookahead symptom).

use std::collections::BTreeMap;

use cs_core::time::split;
use cs_stats::bootstrap::{MeanCi, block_mean_ci};

use crate::sim::{ExitReason, Run, Trade};

/// Bootstrap settings.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BootCfg {
    /// Resamples. Default 2000.
    pub reps: usize,
    /// Block length in trades; 0 = `ceil(n^(1/3))`.
    pub block: usize,
    /// Confidence level. Default 0.95.
    pub level: f64,
    pub seed: u64,
}

impl Default for BootCfg {
    fn default() -> Self {
        BootCfg {
            reps: 2000,
            block: 0,
            level: 0.95,
            seed: 1,
        }
    }
}

/// One row of a breakdown.
#[derive(Debug, Clone, PartialEq)]
pub struct Group {
    pub key: String,
    pub n: usize,
    pub win_rate: f64,
    pub expectancy_r: f64,
    pub sum_r: f64,
}

#[derive(Debug, Clone)]
pub struct Summary {
    pub n: usize,
    pub wins: usize,
    pub losses: usize,
    pub win_rate: f64,
    pub avg_win_r: f64,
    pub avg_loss_r: f64,
    /// Mean net R per trade (after all costs).
    pub expectancy_r: f64,
    /// Mean R before costs.
    pub gross_expectancy_r: f64,
    /// Mean total cost per trade in R (fee + spread + slippage + funding).
    pub mean_cost_r: f64,
    pub mean_fee_r: f64,
    pub mean_spread_r: f64,
    pub mean_slip_r: f64,
    pub mean_funding_r: f64,
    pub sd_r: f64,
    /// `expectancy / (sd / sqrt(n))` (naive, ignores serial correlation; see `ci`).
    pub t_stat: f64,
    /// Sum of winning R / |sum of losing R| (`inf` without losses).
    pub profit_factor: f64,
    pub sum_r: f64,
    /// Largest peak-to-trough fall of cumulative net R, in exit order.
    pub max_dd_r: f64,
    /// Largest peak-to-trough fall of the compounded equity, as a share of the peak.
    pub max_dd_pct: f64,
    /// Final equity / starting equity - 1.
    pub total_return: f64,
    pub mean_ret: f64,
    pub longest_losing_streak: usize,
    /// Share of the market's time span with at least one open position.
    pub exposure: f64,
    pub mean_hold_bars: f64,
    pub median_hold_bars: f64,
    pub mean_hold_hours: f64,
    pub ci: Option<MeanCi>,
    pub exits: Vec<(&'static str, usize)>,
    pub by_regime: Vec<Group>,
    pub by_hour: Vec<Group>,
    pub by_month: Vec<Group>,
    pub flags: Vec<String>,
}

fn mean(v: impl Iterator<Item = f64>) -> f64 {
    let (s, n) = v.fold((0.0, 0usize), |(s, n), x| (s + x, n + 1));
    if n == 0 { f64::NAN } else { s / n as f64 }
}

fn groups<K: Ord + ToString>(trades: &[Trade], key: impl Fn(&Trade) -> K) -> Vec<Group> {
    let mut m: BTreeMap<K, Vec<f64>> = BTreeMap::new();
    for t in trades {
        m.entry(key(t)).or_default().push(t.net_r);
    }
    m.into_iter()
        .map(|(k, v)| Group {
            key: k.to_string(),
            n: v.len(),
            win_rate: v.iter().filter(|x| **x > 0.0).count() as f64 / v.len() as f64,
            expectancy_r: v.iter().sum::<f64>() / v.len() as f64,
            sum_r: v.iter().sum(),
        })
        .collect()
}

/// ADX regime of the signal candle relative to the trade direction.
pub fn regime_label(t: &Trade) -> &'static str {
    match t.regime {
        0 => "range",
        r if r == t.dir => "with_trend",
        _ => "against_trend",
    }
}

/// Summarise a run.
pub fn summarize(run: &Run, tf_ms: i64, boot: BootCfg) -> Summary {
    let tr = &run.trades;
    let n = tr.len();
    // Realised order: by exit time, then entry time, then signal order.
    let mut order: Vec<usize> = (0..n).collect();
    order.sort_by_key(|&i| (tr[i].exit_time, tr[i].entry_time, i));
    let rs: Vec<f64> = order.iter().map(|&i| tr[i].net_r).collect();

    let wins: Vec<f64> = rs.iter().copied().filter(|x| *x > 0.0).collect();
    let losses: Vec<f64> = rs.iter().copied().filter(|x| *x < 0.0).collect();
    let sum_r: f64 = rs.iter().sum();
    let expectancy = if n == 0 { f64::NAN } else { sum_r / n as f64 };
    let sd = if n > 1 {
        (rs.iter()
            .map(|x| (x - expectancy) * (x - expectancy))
            .sum::<f64>()
            / (n - 1) as f64)
            .sqrt()
    } else {
        f64::NAN
    };
    let t_stat = if sd > 0.0 {
        expectancy / (sd / (n as f64).sqrt())
    } else {
        f64::NAN
    };
    let sw: f64 = wins.iter().sum();
    let sl: f64 = losses.iter().sum();
    let profit_factor = if sl < 0.0 {
        sw / -sl
    } else if sw > 0.0 {
        f64::INFINITY
    } else {
        f64::NAN
    };

    let mut peak = 0.0f64;
    let mut cum = 0.0;
    let mut max_dd_r = 0.0f64;
    let mut streak = 0;
    let mut longest = 0;
    for &r in &rs {
        cum += r;
        peak = peak.max(cum);
        max_dd_r = max_dd_r.max(peak - cum);
        if r < 0.0 {
            streak += 1;
            longest = longest.max(streak);
        } else {
            streak = 0;
        }
    }
    let eq0 = run.sizing.equity0;
    let mut eq = eq0;
    let mut epeak = eq0;
    let mut max_dd_pct = 0.0f64;
    for &i in &order {
        eq += tr[i].pnl;
        epeak = epeak.max(eq);
        if epeak > 0.0 {
            max_dd_pct = max_dd_pct.max((epeak - eq) / epeak);
        }
    }

    // Exposure: union of [entry, exit) intervals over the market span.
    let mut iv: Vec<(i64, i64)> = tr
        .iter()
        .map(|t| (t.entry_time, t.exit_time.max(t.entry_time)))
        .collect();
    iv.sort_unstable();
    let mut covered = 0i64;
    let mut cur: Option<(i64, i64)> = None;
    for (a, b) in iv {
        cur = match cur {
            Some((s, e)) if a <= e => Some((s, e.max(b))),
            Some((s, e)) => {
                covered += e - s;
                Some((a, b))
            }
            None => Some((a, b)),
        };
    }
    if let Some((s, e)) = cur {
        covered += e - s;
    }
    let span = (run.span.1 - run.span.0).max(1);
    let mut holds: Vec<f64> = tr
        .iter()
        .map(|t| (t.exit_time - t.entry_time) as f64 / tf_ms as f64)
        .collect();
    holds.sort_by(f64::total_cmp);
    let median_hold = if holds.is_empty() {
        f64::NAN
    } else {
        holds[holds.len() / 2]
    };

    let exits = ExitReason::ALL
        .iter()
        .map(|r| (r.label(), tr.iter().filter(|t| t.reason == *r).count()))
        .collect();
    let ci = block_mean_ci(&rs, boot.block, boot.reps, boot.level, boot.seed);
    let win_rate = if n == 0 {
        f64::NAN
    } else {
        wins.len() as f64 / n as f64
    };
    let avg_win = mean(wins.iter().copied());
    let avg_loss = mean(losses.iter().copied());
    let mut s = Summary {
        n,
        wins: wins.len(),
        losses: losses.len(),
        win_rate,
        avg_win_r: avg_win,
        avg_loss_r: avg_loss,
        expectancy_r: expectancy,
        gross_expectancy_r: mean(tr.iter().map(|t| t.gross_r)),
        mean_cost_r: mean(tr.iter().map(|t| t.gross_r - t.net_r)),
        mean_fee_r: mean(tr.iter().map(|t| t.fee_r)),
        mean_spread_r: mean(tr.iter().map(|t| t.spread_r)),
        mean_slip_r: mean(tr.iter().map(|t| t.slip_r)),
        mean_funding_r: mean(tr.iter().map(|t| t.funding_r)),
        sd_r: sd,
        t_stat,
        profit_factor,
        sum_r,
        max_dd_r,
        max_dd_pct,
        total_return: eq / eq0 - 1.0,
        mean_ret: mean(tr.iter().map(|t| t.ret)),
        longest_losing_streak: longest,
        exposure: covered as f64 / span as f64,
        mean_hold_bars: mean(holds.iter().copied()),
        median_hold_bars: median_hold,
        mean_hold_hours: mean(
            tr.iter()
                .map(|t| (t.exit_time - t.entry_time) as f64 / 3_600_000.0),
        ),
        ci,
        exits,
        by_regime: groups(tr, regime_label),
        by_hour: groups(tr, |t| format!("{:02}", split(t.entry_time).3)),
        by_month: groups(tr, |t| {
            let (y, m, ..) = split(t.entry_time);
            format!("{y:04}-{m:02}")
        }),
        flags: Vec::new(),
    };
    s.flags = sanity_flags(&s, run);
    s
}

/// Flags for results that are too good to be real on market data (typical lookahead symptoms)
/// and for weak evidence. They do not change any number.
pub fn sanity_flags(s: &Summary, run: &Run) -> Vec<String> {
    let mut f = Vec::new();
    if s.n < 30 {
        f.push(format!("SMALL_SAMPLE: only {} trades", s.n));
        return f;
    }
    if s.t_stat > 8.0 {
        f.push(format!(
            "SUSPECT_LOOKAHEAD: t-stat {:.1} over {} trades (real edges are far below 8)",
            s.t_stat, s.n
        ));
    }
    if s.profit_factor > 5.0 {
        f.push(format!(
            "SUSPECT_LOOKAHEAD: profit factor {:.2}",
            s.profit_factor
        ));
    }
    if s.win_rate > 0.9 && s.avg_win_r >= 0.5 * s.avg_loss_r.abs() {
        f.push(format!(
            "SUSPECT_LOOKAHEAD: win rate {:.1} % with average win {:.2} R vs loss {:.2} R",
            100.0 * s.win_rate,
            s.avg_win_r,
            s.avg_loss_r
        ));
    }
    let fallback = run.fills.stop_first_no_1m + run.fills.stop_first_same_minute;
    if f64::from(fallback) > 0.02 * s.n as f64 {
        f.push(format!(
            "STOP_FIRST_ASSUMED: {fallback} exits ({:.1} %) assumed the stop came first",
            100.0 * f64::from(fallback) / s.n as f64
        ));
    }
    let eod = s
        .exits
        .iter()
        .find(|(k, _)| *k == "end_of_data")
        .map_or(0, |x| x.1);
    if eod > 0 {
        f.push(format!(
            "END_OF_DATA: {eod} trades closed at the last candle"
        ));
    }
    f
}
