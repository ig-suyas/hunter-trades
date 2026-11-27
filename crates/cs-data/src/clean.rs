//! Loading stored series and applying the documented repair rules, counting every row touched.
//!
//! Store files are a faithful copy of the source. Repairs happen only here, at load time, so
//! the raw data stays intact and each run reports exactly what it changed:
//!
//! * **R1 timestamp unit**: values >= 1e14 are microseconds (Binance spot files from 2025-01-01)
//!   and are divided by 1000. Applies to `open_time` and `close_time`.
//! * **R2 misaligned rows dropped**: when a grid is given (klines: whole UTC minutes), rows whose
//!   time is not on the grid after R1 are dropped.
//! * **R3 out-of-order rows sorted**: stable sort by time; the count is the number of rows
//!   that were earlier than their predecessor.
//! * **R4 exact duplicates dropped**: same time and all values equal (within 1e-12).
//! * **R5 conflicting duplicates**: same time, different values; the first row in file order is
//!   kept and the others are dropped.
//!
//! Nothing else is changed: gaps are not filled, zero-volume candles and outlier wicks are kept
//! and only flagged by `validate`.

use std::collections::BTreeMap;
use std::path::Path;

use cs_core::{Candle, approx_eq};

use crate::store::{self, Column, RawKlines, Table};
use crate::{DataDir, Error, Market, Result, list_csc};

pub const US_THRESHOLD: i64 = 100_000_000_000_000;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Repairs {
    pub rows_in: u64,
    pub r1_unit_us_to_ms: u64,
    pub r2_misaligned_dropped: u64,
    pub r3_out_of_order: u64,
    pub r4_exact_dupes: u64,
    pub r5_conflicting_dupes: u64,
    pub rows_out: u64,
}

impl Repairs {
    pub fn add(&mut self, o: &Repairs) {
        self.rows_in += o.rows_in;
        self.r1_unit_us_to_ms += o.r1_unit_us_to_ms;
        self.r2_misaligned_dropped += o.r2_misaligned_dropped;
        self.r3_out_of_order += o.r3_out_of_order;
        self.r4_exact_dupes += o.r4_exact_dupes;
        self.r5_conflicting_dupes += o.r5_conflicting_dupes;
        self.rows_out += o.rows_out;
    }
}

/// Concatenated store files of one directory, plus row counts per `source` and file metadata.
#[derive(Debug, Default)]
pub struct Loaded {
    pub table: Table,
    pub sources: BTreeMap<String, u64>,
    pub files: u64,
    pub header_lines: u64,
}

pub fn load_dir(dir: &Path) -> Result<Loaded> {
    let mut out = Loaded::default();
    for p in list_csc(dir)? {
        let t = store::read_file(&p)?;
        out.files += 1;
        *out.sources
            .entry(t.meta("source").unwrap_or("unknown").to_string())
            .or_default() += t.nrows() as u64;
        out.header_lines += t
            .meta("header_lines")
            .and_then(|v| v.parse().ok())
            .unwrap_or(0);
        if out.table.cols.is_empty() {
            out.table.cols = t.cols;
            continue;
        }
        for ((name, acc), (n2, col)) in out.table.cols.iter_mut().zip(t.cols) {
            if *name != n2 {
                return Err(Error::Format(format!(
                    "{}: column {n2} != {name}",
                    p.display()
                )));
            }
            match (acc, col) {
                (Column::I64(a), Column::I64(b)) => a.extend(b),
                (Column::F64(a), Column::F64(b)) => a.extend(b),
                _ => return Err(Error::Format(format!("{}: column type", p.display()))),
            }
        }
    }
    Ok(out)
}

fn rows_equal(t: &Table, i: usize, j: usize) -> bool {
    t.cols.iter().all(|(_, c)| match c {
        Column::I64(v) => v[i] == v[j],
        Column::F64(v) => approx_eq(v[i], v[j], 1e-12, 1e-12),
    })
}

/// Apply R1-R5 to a table keyed by `time_col`. `unit_cols` get the R1 conversion.
pub fn clean_table(
    t: &Table,
    time_col: &str,
    unit_cols: &[&str],
    grid_ms: Option<i64>,
) -> Result<(Table, Repairs)> {
    let mut t = t.clone();
    let n = t.nrows();
    let mut rep = Repairs {
        rows_in: n as u64,
        ..Repairs::default()
    };
    // R1
    for (name, col) in &mut t.cols {
        if unit_cols.contains(&name.as_str())
            && let Column::I64(v) = col
        {
            for x in v.iter_mut() {
                if *x >= US_THRESHOLD {
                    *x /= 1000;
                    if name == time_col {
                        rep.r1_unit_us_to_ms += 1;
                    }
                }
            }
        }
    }
    let times = t.i64(time_col)?.to_vec();
    // R2
    let mut keep: Vec<usize> = (0..n)
        .filter(|&i| grid_ms.is_none_or(|g| times[i].rem_euclid(g) == 0))
        .collect();
    rep.r2_misaligned_dropped = (n - keep.len()) as u64;
    // R3
    rep.r3_out_of_order = keep
        .windows(2)
        .filter(|w| times[w[1]] < times[w[0]])
        .count() as u64;
    keep.sort_by_key(|&i| times[i]);
    // R4 / R5
    let mut out_idx: Vec<usize> = Vec::with_capacity(keep.len());
    for i in keep {
        if let Some(&last) = out_idx.last()
            && times[last] == times[i]
        {
            if rows_equal(&t, last, i) {
                rep.r4_exact_dupes += 1;
            } else {
                rep.r5_conflicting_dupes += 1;
            }
            continue;
        }
        out_idx.push(i);
    }
    rep.rows_out = out_idx.len() as u64;
    let cols = t
        .cols
        .iter()
        .map(|(name, c)| {
            let c2 = match c {
                Column::I64(v) => Column::I64(out_idx.iter().map(|&i| v[i]).collect()),
                Column::F64(v) => Column::F64(out_idx.iter().map(|&i| v[i]).collect()),
            };
            (name.clone(), c2)
        })
        .collect();
    Ok((Table { meta: t.meta, cols }, rep))
}

/// Clean 1m klines of one market/symbol.
#[derive(Debug, Default)]
pub struct CleanKlines {
    pub candles: Vec<Candle>,
    pub repairs: Repairs,
    pub sources: BTreeMap<String, u64>,
    pub files: u64,
    pub header_lines: u64,
    /// Flag only: rows whose `close_time` != `open_time` + 59 999 ms after R1.
    pub close_time_mismatch: u64,
}

pub fn klines_from_table(t: &Table) -> Result<(CleanKlines, Repairs)> {
    let (ct, rep) = clean_table(t, "open_time", &["open_time", "close_time"], Some(60_000))?;
    let raw = RawKlines::from_table(&ct)?;
    let mut ck = CleanKlines {
        close_time_mismatch: (0..raw.len())
            .filter(|&i| raw.close_time[i] != raw.open_time[i] + 59_999)
            .count() as u64,
        ..CleanKlines::default()
    };
    ck.candles = (0..raw.len()).map(|i| raw.row(i)).collect();
    ck.repairs = rep.clone();
    Ok((ck, rep))
}

pub fn klines(dd: &DataDir, m: Market, sym: &str) -> Result<CleanKlines> {
    let l = load_dir(&dd.klines_1m(m, sym))?;
    if l.table.cols.is_empty() {
        return Ok(CleanKlines::default());
    }
    let (mut ck, _) = klines_from_table(&l.table)?;
    ck.sources = l.sources;
    ck.files = l.files;
    ck.header_lines = l.header_lines;
    Ok(ck)
}

/// Clean a time-keyed series directory (funding, metrics).
pub fn series(dir: &Path, grid_ms: Option<i64>) -> Result<(Table, Repairs, Loaded)> {
    let l = load_dir(dir)?;
    if l.table.cols.is_empty() {
        return Ok((Table::default(), Repairs::default(), l));
    }
    let (t, r) = clean_table(&l.table, "time", &[], grid_ms)?;
    Ok((t, r, l))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn table(times: Vec<i64>, vals: Vec<f64>) -> Table {
        Table {
            meta: vec![],
            cols: vec![
                ("time".into(), Column::I64(times)),
                ("v".into(), Column::F64(vals)),
            ],
        }
    }

    #[test]
    fn rules_are_counted() {
        let m = 60_000;
        let b = 1_730_419_200_000; // 2024-11-01 00:00 UTC in ms
        let t = table(
            vec![
                b + 2 * m,
                b + m,
                b + m,
                b + 3 * m,
                b + 3 * m,
                b + 4 * m + 5,
                (b + 5 * m) * 1000,
            ],
            vec![2.0, 1.0, 1.0, 3.0, 3.5, 9.0, 5.0],
        );
        let (c, r) = clean_table(&t, "time", &["time"], Some(m)).unwrap();
        assert_eq!(r.r1_unit_us_to_ms, 1);
        assert_eq!(r.r2_misaligned_dropped, 1);
        assert_eq!(r.r3_out_of_order, 1);
        assert_eq!(r.r4_exact_dupes, 1);
        assert_eq!(r.r5_conflicting_dupes, 1);
        assert_eq!(
            c.i64("time").unwrap(),
            &[b + m, b + 2 * m, b + 3 * m, b + 5 * m]
        );
        // first occurrence kept for the conflicting duplicate
        assert!(cs_core::price_eq(c.f64("v").unwrap()[2], 3.0));
        assert_eq!(r.rows_out, 4);
    }
}
