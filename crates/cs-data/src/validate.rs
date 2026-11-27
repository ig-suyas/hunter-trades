//! `validate`: data-quality checks and `reports/data-quality.md`.
//!
//! Checks (flag only unless a repair rule in `clean` applies): gaps, duplicates, out-of-order
//! rows, zero-volume candles, OHLC inconsistencies, outlier wicks, timestamp unit, minute
//! alignment, close-time mismatch and time-zone shifts (a file whose first/last row is not at
//! its UTC period boundary).

use std::fmt::Write as _;
use std::path::Path;

use cs_core::time::{fmt_date, fmt_minute, now_ms};
use cs_core::{Candle, Timeframe};

use crate::clean::{self, CleanKlines, Repairs};
use crate::resample::{Policy, ResampleStats, resample};
use crate::store::{self, RawKlines};
use crate::vision::Period;
use crate::{DataDir, Market, Result, list_csc};

/// Outlier wick: wick > `WICK_MULT` x the mean high-low range of the previous `WICK_LOOKBACK`
/// candles AND wick > `WICK_MIN_FRAC` of the close.
pub const WICK_LOOKBACK: usize = 60;
pub const WICK_MULT: f64 = 10.0;
pub const WICK_MIN_FRAC: f64 = 0.005;
/// Extreme wick: wick > 3% of close in a single minute.
pub const WICK_EXTREME_FRAC: f64 = 0.03;

#[derive(Debug, Clone, Default)]
pub struct KlineQuality {
    pub rows: u64,
    pub first: i64,
    pub last: i64,
    pub expected: u64,
    pub gaps: u64,
    pub missing_minutes: u64,
    pub largest_gaps: Vec<(i64, i64)>,
    pub zero_volume: u64,
    pub zero_trades: u64,
    pub ohlc_bad: u64,
    pub outlier_wicks: u64,
    pub extreme_wicks: u64,
    pub top_wicks: Vec<(i64, f64)>,
    pub taker_gt_volume: u64,
    pub taker_missing: u64,
    pub tz_shift_files: u64,
    /// Longest runs of consecutive zero-volume minutes: (start, length), top 5.
    pub zero_volume_runs: Vec<(i64, i64)>,
}

pub fn kline_quality(c: &[Candle]) -> KlineQuality {
    let mut q = KlineQuality {
        rows: c.len() as u64,
        ..KlineQuality::default()
    };
    let (Some(f), Some(l)) = (c.first(), c.last()) else {
        return q;
    };
    q.first = f.open_time;
    q.last = l.open_time;
    q.expected = u64::try_from((l.open_time - f.open_time) / 60_000 + 1).unwrap_or(0);
    let mut gaps: Vec<(i64, i64)> = Vec::new();
    for w in c.windows(2) {
        let d = (w[1].open_time - w[0].open_time) / 60_000;
        if d > 1 {
            q.gaps += 1;
            q.missing_minutes += u64::try_from(d - 1).unwrap_or(0);
            gaps.push((w[0].open_time + 60_000, d - 1));
        }
    }
    gaps.sort_by_key(|g| std::cmp::Reverse(g.1));
    gaps.truncate(5);
    q.largest_gaps = gaps;

    let mut runs: Vec<(i64, i64)> = Vec::new();
    for (i, k) in c.iter().enumerate() {
        if k.volume > 0.0 {
            continue;
        }
        match runs.last_mut() {
            Some((start, len))
                if i > 0 && c[i - 1].volume <= 0.0 && *start + *len * 60_000 == k.open_time =>
            {
                *len += 1;
            }
            _ => runs.push((k.open_time, 1)),
        }
    }
    runs.sort_by_key(|r| std::cmp::Reverse(r.1));
    runs.truncate(5);
    q.zero_volume_runs = runs;

    let mut range_sum = 0.0;
    let mut wicks: Vec<(i64, f64)> = Vec::new();
    for (i, k) in c.iter().enumerate() {
        if k.volume <= 0.0 {
            q.zero_volume += 1;
        }
        if k.trades == 0 {
            q.zero_trades += 1;
        }
        let tol = 1e-9 * k.close.abs().max(1.0);
        let bad = [k.open, k.high, k.low, k.close]
            .iter()
            .any(|x| !x.is_finite())
            || k.low <= 0.0
            || k.high + tol < k.open.max(k.close)
            || k.low - tol > k.open.min(k.close);
        if bad {
            q.ohlc_bad += 1;
        }
        if k.taker_buy_base.is_nan() {
            q.taker_missing += 1;
        } else if k.taker_buy_base > k.volume * (1.0 + 1e-9) + 1e-12 {
            q.taker_gt_volume += 1;
        }
        let wick = (k.high - k.open.max(k.close)).max(k.open.min(k.close) - k.low);
        if i >= WICK_LOOKBACK {
            #[allow(clippy::cast_precision_loss)]
            let mean = range_sum / WICK_LOOKBACK as f64;
            let frac = wick / k.close;
            if wick > WICK_MULT * mean && frac > WICK_MIN_FRAC {
                q.outlier_wicks += 1;
                wicks.push((k.open_time, frac));
            }
            if frac > WICK_EXTREME_FRAC {
                q.extreme_wicks += 1;
            }
            let old = &c[i - WICK_LOOKBACK];
            range_sum -= old.high - old.low;
        }
        range_sum += k.high - k.low;
    }
    wicks.sort_by(|a, b| b.1.total_cmp(&a.1));
    wicks.truncate(5);
    q.top_wicks = wicks;
    q
}

/// Files whose first/last row is not at the UTC period boundary although the period is fully
/// covered by row count; that would indicate a time-zone shift in the source.
fn tz_shift_files(dir: &Path) -> Result<u64> {
    let mut n = 0;
    for p in list_csc(dir)? {
        let t = store::read_file(&p)?;
        let raw = RawKlines::from_table(&t)?;
        let Some(label) = t.meta("period") else {
            continue;
        };
        let period = match label.len() {
            7 => label
                .split_once('-')
                .and_then(|(y, m)| Some(Period::Month(y.parse().ok()?, m.parse().ok()?))),
            10 => {
                let v: Vec<i64> = label.split('-').filter_map(|x| x.parse().ok()).collect();
                (v.len() == 3).then(|| Period::Day(v[0], v[1], v[2]))
            }
            _ => None,
        };
        let (Some(period), Some(&first), Some(&last)) =
            (period, raw.open_time.first(), raw.open_time.last())
        else {
            continue;
        };
        let norm = |x: i64| {
            if x >= clean::US_THRESHOLD {
                x / 1000
            } else {
                x
            }
        };
        let (s, e) = period.range();
        let expected_rows = usize::try_from((e - s) / 60_000).unwrap_or(0);
        if raw.len() == expected_rows && (norm(first) != s || norm(last) != e - 60_000) {
            n += 1;
        }
    }
    Ok(n)
}

pub struct SeriesReport {
    pub name: String,
    pub ck: CleanKlines,
    pub q: KlineQuality,
    pub resampled: Vec<(Timeframe, ResampleStats)>,
}

fn repairs_row(name: &str, r: &Repairs) -> String {
    format!(
        "| {name} | {} | {} | {} | {} | {} | {} | {} |\n",
        r.rows_in,
        r.r1_unit_us_to_ms,
        r.r2_misaligned_dropped,
        r.r3_out_of_order,
        r.r4_exact_dupes,
        r.r5_conflicting_dupes,
        r.rows_out
    )
}

#[allow(clippy::cast_precision_loss)]
fn mb(b: u64) -> f64 {
    b as f64 / 1e6
}

fn pct(a: u64, b: u64) -> f64 {
    #[allow(clippy::cast_precision_loss)]
    if b == 0 {
        0.0
    } else {
        100.0 * a as f64 / b as f64
    }
}

/// Count live liquidation lines already recorded (any `data/live/liquidations*` files).
fn recorded_liquidations(dd: &DataDir) -> (u64, u64) {
    let mut files = 0;
    let mut bytes = 0;
    let root = dd.live();
    for stream in ["liq_binance", "liq_bybit"] {
        let dir = root.join(stream);
        let Ok(days) = std::fs::read_dir(&dir) else {
            continue;
        };
        for d in days.flatten() {
            let Ok(fs) = std::fs::read_dir(d.path()) else {
                continue;
            };
            for f in fs.flatten() {
                files += 1;
                bytes += f.metadata().map_or(0, |m| m.len());
            }
        }
    }
    (files, bytes)
}

#[allow(clippy::too_many_lines)]
pub fn run(dd: &DataDir, symbols: &[String], markets: &[Market]) -> Result<String> {
    let mut md = String::new();
    let _ = writeln!(md, "# Data-quality report\n");
    let _ = writeln!(
        md,
        "Generated by `candlescope validate` at {} UTC from the store in `{}`. All timestamps are UTC milliseconds; candle time = open time. Store files are faithful copies of the source; the repair rules below are applied when data is loaded, and every row they touch is counted here. Nothing else is repaired: gaps stay gaps, and zero-volume candles and outlier wicks are kept and only flagged.\n",
        fmt_minute(now_ms()),
        dd.root.display()
    );

    let mut reports = Vec::new();
    for sym in symbols {
        for &m in markets {
            let ck = clean::klines(dd, m, sym)?;
            if ck.candles.is_empty() {
                continue;
            }
            let mut q = kline_quality(&ck.candles);
            q.tz_shift_files = tz_shift_files(&dd.klines_1m(m, sym))?;
            let resampled = Timeframe::ALL[1..]
                .iter()
                .map(|&tf| (tf, resample(&ck.candles, tf, Policy::Strict).2))
                .collect();
            reports.push(SeriesReport {
                name: format!("{} {sym}", m.dir()),
                ck,
                q,
                resampled,
            });
        }
    }

    md.push_str("## 1. Candles (1m) summary\n\n| Series | Rows | From (UTC) | To (UTC) | Coverage | Gaps | Missing minutes | Duplicates (exact / conflicting) | Out-of-order rows | Zero-volume | OHLC inconsistent | Outlier wicks | Extreme wicks (>3%) | Sources (rows) |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|---|\n");
    for r in &reports {
        let q = &r.q;
        let src: Vec<String> =
            r.ck.sources
                .iter()
                .map(|(k, v)| format!("{k}: {v}"))
                .collect();
        let _ = writeln!(
            md,
            "| {} | {} | {} | {} | {:.4}% | {} | {} | {} / {} | {} | {} | {} | {} | {} | {} |",
            r.name,
            q.rows,
            fmt_minute(q.first),
            fmt_minute(q.last),
            pct(q.rows, q.expected),
            q.gaps,
            q.missing_minutes,
            r.ck.repairs.r4_exact_dupes,
            r.ck.repairs.r5_conflicting_dupes,
            r.ck.repairs.r3_out_of_order,
            q.zero_volume,
            q.ohlc_bad,
            q.outlier_wicks,
            q.extreme_wicks,
            src.join(", ")
        );
    }

    md.push_str("\n## 2. Repair rules and rows touched\n\nRules (implemented in `crates/cs-data/src/clean.rs`, applied at load time, in this order):\n\n- **R1 timestamp unit**: a timestamp >= 1e14 is in microseconds (Binance spot archive files from 2025-01-01 onward) and is divided by 1000 (open_time and close_time).\n- **R2 misaligned rows dropped**: after R1, a candle whose open time is not a whole UTC minute is dropped.\n- **R3 out-of-order rows sorted**: stable sort by time; the count is rows that were earlier than the row before them.\n- **R4 exact duplicates dropped**: same time and identical values.\n- **R5 conflicting duplicates**: same time, different values; the first row in file order is kept.\n\n| Series | Rows in | R1 us->ms | R2 misaligned dropped | R3 out-of-order sorted | R4 exact dupes dropped | R5 conflicting dupes dropped | Rows out |\n|---|---|---|---|---|---|---|---|\n");
    for r in &reports {
        md.push_str(&repairs_row(&r.name, &r.ck.repairs));
    }

    md.push_str("\n## 3. Flags (not repaired)\n\n| Series | Zero-trade candles | Taker-buy > volume | Taker-buy missing (Bybit rows) | close_time != open+59 999 ms | Files shifted from UTC boundary | CSV header lines skipped |\n|---|---|---|---|---|---|---|\n");
    for r in &reports {
        let q = &r.q;
        let _ = writeln!(
            md,
            "| {} | {} | {} | {} | {} | {} | {} |",
            r.name,
            q.zero_trades,
            q.taker_gt_volume,
            q.taker_missing,
            r.ck.close_time_mismatch,
            q.tz_shift_files,
            r.ck.header_lines
        );
    }
    let _ = writeln!(
        md,
        "\nOutlier wick rule: the larger wick (high - max(open, close), or min(open, close) - low) is more than {WICK_MULT} x the mean high-low range of the previous {WICK_LOOKBACK} candles **and** more than {:.1}% of the close. These are kept: in crypto such wicks are usually real liquidation spikes, and the sweep tactics need them.\n",
        WICK_MIN_FRAC * 100.0
    );
    for r in &reports {
        let q = &r.q;
        let gaps: Vec<String> = q
            .largest_gaps
            .iter()
            .map(|(t, n)| format!("{} ({n} min)", fmt_minute(*t)))
            .collect();
        let wicks: Vec<String> = q
            .top_wicks
            .iter()
            .map(|(t, f)| format!("{} ({:.2}%)", fmt_minute(*t), f * 100.0))
            .collect();
        let zv: Vec<String> = q
            .zero_volume_runs
            .iter()
            .map(|(t, n)| format!("{} ({n} min)", fmt_minute(*t)))
            .collect();
        let list = |v: &[String]| {
            if v.is_empty() {
                "none".to_string()
            } else {
                v.join(", ")
            }
        };
        let _ = writeln!(
            md,
            "- **{}**: largest gaps: {}; longest zero-volume runs: {}; largest outlier wicks: {}",
            r.name,
            list(&gaps),
            list(&zv),
            list(&wicks)
        );
    }
    md.push_str("\nZero-volume candles that come in runs at the same minutes on both symbols of one market are most likely exchange outages that the archive fills with flat candles (open = high = low = close, no trades). They are kept as rows; later phases should treat them as \"no trading\", not as real quiet minutes.\n");

    // Funding
    md.push_str("\n## 4. Funding rate history (USDS-M)\n\n| Symbol | Rows | From | To | Sources (rows) | R3 out-of-order | R4/R5 dupes | Interval gaps (> interval + 1 min) | Non-8h intervals | calc_time not on the hour | abs(rate) > 0.3% |\n|---|---|---|---|---|---|---|---|---|---|---|\n");
    for sym in symbols {
        let (t, rep, l) = clean::series(&dd.funding(sym), None)?;
        if t.nrows() == 0 {
            continue;
        }
        let ts = t.i64("time")?;
        let ih = t.i64("interval_h")?;
        let rate = t.f64("rate")?;
        let mut gaps = 0;
        let mut non8 = 0;
        for i in 1..ts.len() {
            let iv = if ih[i] > 0 { ih[i] } else { 8 } * 3_600_000;
            if ts[i] - ts[i - 1] > iv + 60_000 {
                gaps += 1;
            }
            if ih[i] > 0 && ih[i] != 8 {
                non8 += 1;
            }
        }
        let off_hour = ts.iter().filter(|&&x| x % 3_600_000 != 0).count();
        let big = rate.iter().filter(|r| r.abs() > 0.003).count();
        let src: Vec<String> = l.sources.iter().map(|(k, v)| format!("{k}: {v}")).collect();
        let _ = writeln!(
            md,
            "| {sym} | {} | {} | {} | {} | {} | {} / {} | {gaps} | {non8} | {off_hour} | {big} |",
            ts.len(),
            fmt_minute(ts[0]),
            fmt_minute(ts[ts.len() - 1]),
            src.join(", "),
            rep.r3_out_of_order,
            rep.r4_exact_dupes,
            rep.r5_conflicting_dupes
        );
    }
    md.push_str("\n`calc_time` in the archive can carry a few milliseconds past the hour; that is flagged, not changed.\n");

    // Metrics
    md.push_str("\n## 5. Open interest (5-minute `metrics`, USDS-M)\n\n| Symbol | Rows | From | To | Expected 5-min rows | Coverage | Gaps | Missing 5-min slots | R3 out-of-order sorted | R4 / R5 dupes dropped | Off-grid rows | OI missing or <= 0 | Header lines | Sources (rows) |\n|---|---|---|---|---|---|---|---|---|---|---|---|---|---|\n");
    for sym in symbols {
        let (t, rep, l) = clean::series(&dd.metrics(sym), None)?;
        if t.nrows() == 0 {
            continue;
        }
        let ts = t.i64("time")?;
        let oi = t.f64("oi")?;
        let step = 300_000;
        let expected = u64::try_from((ts[ts.len() - 1] - ts[0]) / step + 1).unwrap_or(0);
        let mut gaps = 0u64;
        let mut missing = 0u64;
        for w in ts.windows(2) {
            let d = (w[1] - w[0]) / step;
            if d > 1 {
                gaps += 1;
                missing += u64::try_from(d - 1).unwrap_or(0);
            }
        }
        let off = ts.iter().filter(|&&x| x % step != 0).count();
        let bad = oi.iter().filter(|x| !(x.is_finite() && **x > 0.0)).count();
        let src: Vec<String> = l.sources.iter().map(|(k, v)| format!("{k}: {v}")).collect();
        let _ = writeln!(
            md,
            "| {sym} | {} | {} | {} | {expected} | {:.3}% | {gaps} | {missing} | {} | {} / {} | {off} | {bad} | {} | {} |",
            ts.len(),
            fmt_minute(ts[0]),
            fmt_minute(ts[ts.len() - 1]),
            pct(ts.len() as u64, expected),
            rep.r3_out_of_order,
            rep.r4_exact_dupes,
            rep.r5_conflicting_dupes,
            l.header_lines,
            src.join(", ")
        );
    }
    md.push_str("\nThe archive's daily metrics files are not sorted by time, so R3 (sort) touches many rows; that is expected and harmless.\n");

    // Resampling
    md.push_str("\n## 6. Resampling (strict: only complete buckets)\n\n| Series | TF | Candles emitted | Incomplete inside the data (dropped) | Partial at the edges (dropped) |\n|---|---|---|---|---|\n");
    for r in &reports {
        for (tf, st) in &r.resampled {
            let _ = writeln!(
                md,
                "| {} | {} | {} | {} | {} |",
                r.name,
                tf.label(),
                st.emitted,
                st.incomplete,
                st.edge_dropped
            );
        }
    }
    md.push_str("\nBuckets are UTC aligned and labelled by open time; volume is summed. A bucket with any missing minute is not emitted under the default policy (`Policy::Strict`); `Policy::AllowGaps` keeps internal ones and stores the minute count with each candle.\n");

    // Liquidations
    let (lf, lb) = recorded_liquidations(dd);
    let _ = writeln!(
        md,
        "\n## 7. Liquidation history\n\n**No usable free history for 2 years.** Checked in Phase 0 and re-checked in Phase 1 (see `notes/data-sources.md`): the Binance Vision `liquidationSnapshot` archive returns 404, Binance REST `allForceOrders` returns 404, CoinGlass needs a key, and Tardis.dev is free only for the first day of each month. The heatmap tactics (H1 liquidation-cluster sweep, H2 walls) therefore **cannot be backtested** on history. They can only be forward-tested on data recorded by `candlescope record`, which started in Phase 1. Recorded so far: {lf} liquidation files, {:.1} MB (Binance `!forceOrder@arr`, which sends only the largest liquidation per symbol per second, plus Bybit `allLiquidation`).\n",
        mb(lb)
    );
    let _ = writeln!(md, "Report date: {}.", fmt_date(now_ms()));
    Ok(md)
}
