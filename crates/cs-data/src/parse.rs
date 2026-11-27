//! CSV parsers for Binance Vision files. Values are converted to numbers only; no row is
//! dropped, reordered or changed here (repairs happen in `clean`, with counts).
//! A leading header line is skipped and reported.

use cs_core::time::parse_datetime;

use crate::store::{Column, RawKlines, Table};
use crate::{Error, Result};

/// Parse outcome notes for the data-quality report.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ParseNotes {
    pub header_lines: u64,
    pub blank_lines: u64,
}

fn num_f(s: &str, line: usize) -> Result<f64> {
    let s = s.trim();
    if s.is_empty() {
        return Ok(f64::NAN);
    }
    s.parse::<f64>()
        .map_err(|_| Error::Parse(format!("line {line}: bad number {s:?}")))
}

fn num_i(s: &str, line: usize) -> Result<i64> {
    let s = s.trim();
    s.parse::<i64>()
        .map_err(|_| Error::Parse(format!("line {line}: bad integer {s:?}")))
}

fn lines(text: &str) -> impl Iterator<Item = (usize, &str)> {
    text.lines()
        .enumerate()
        .map(|(i, l)| (i + 1, l.trim_end_matches('\r')))
}

fn is_header(l: &str) -> bool {
    l.split(',').next().is_some_and(|f| {
        let f = f.trim();
        !f.is_empty() && f.parse::<f64>().is_err() && parse_datetime(f).is_none()
    })
}

/// Binance kline CSV (12 columns; the last one is ignored).
pub fn klines_csv(text: &str) -> Result<(RawKlines, ParseNotes)> {
    let mut k = RawKlines::default();
    let mut notes = ParseNotes::default();
    for (ln, l) in lines(text) {
        if l.trim().is_empty() {
            notes.blank_lines += 1;
            continue;
        }
        if ln == 1 && is_header(l) {
            notes.header_lines += 1;
            continue;
        }
        let f: Vec<&str> = l.split(',').collect();
        if f.len() < 11 {
            return Err(Error::Parse(format!(
                "line {ln}: {} fields, expected 12",
                f.len()
            )));
        }
        k.open_time.push(num_i(f[0], ln)?);
        k.open.push(num_f(f[1], ln)?);
        k.high.push(num_f(f[2], ln)?);
        k.low.push(num_f(f[3], ln)?);
        k.close.push(num_f(f[4], ln)?);
        k.volume.push(num_f(f[5], ln)?);
        k.close_time.push(num_i(f[6], ln)?);
        k.quote_volume.push(num_f(f[7], ln)?);
        k.trades.push(num_i(f[8], ln)?);
        k.taker_buy_base.push(num_f(f[9], ln)?);
        k.taker_buy_quote.push(num_f(f[10], ln)?);
    }
    Ok((k, notes))
}

/// `calc_time,funding_interval_hours,last_funding_rate`
pub fn funding_csv(text: &str) -> Result<(Table, ParseNotes)> {
    let (mut t, mut h, mut r) = (Vec::new(), Vec::new(), Vec::new());
    let mut notes = ParseNotes::default();
    for (ln, l) in lines(text) {
        if l.trim().is_empty() {
            notes.blank_lines += 1;
            continue;
        }
        if ln == 1 && is_header(l) {
            notes.header_lines += 1;
            continue;
        }
        let f: Vec<&str> = l.split(',').collect();
        if f.len() < 3 {
            return Err(Error::Parse(format!(
                "line {ln}: funding row has {} fields",
                f.len()
            )));
        }
        t.push(num_i(f[0], ln)?);
        h.push(num_i(f[1], ln)?);
        r.push(num_f(f[2], ln)?);
    }
    Ok((funding_table(t, h, r), notes))
}

pub fn funding_table(t: Vec<i64>, h: Vec<i64>, r: Vec<f64>) -> Table {
    Table {
        meta: vec![],
        cols: vec![
            ("time".into(), Column::I64(t)),
            ("interval_h".into(), Column::I64(h)),
            ("rate".into(), Column::F64(r)),
        ],
    }
}

pub const METRICS_F64: [&str; 6] = [
    "oi",
    "oi_value",
    "top_ls_count",
    "top_ls_position",
    "ls_count",
    "taker_ls_vol",
];

/// Binance `metrics` daily CSV (5-minute OI and long/short ratios).
pub fn metrics_csv(text: &str) -> Result<(Table, ParseNotes)> {
    let mut t = Vec::new();
    let mut cols: [Vec<f64>; 6] = Default::default();
    let mut notes = ParseNotes::default();
    for (ln, l) in lines(text) {
        if l.trim().is_empty() {
            notes.blank_lines += 1;
            continue;
        }
        if ln == 1 && is_header(l) {
            notes.header_lines += 1;
            continue;
        }
        let f: Vec<&str> = l.split(',').collect();
        if f.len() < 8 {
            return Err(Error::Parse(format!(
                "line {ln}: metrics row has {} fields",
                f.len()
            )));
        }
        t.push(
            parse_datetime(f[0])
                .ok_or_else(|| Error::Parse(format!("line {ln}: bad time {:?}", f[0])))?,
        );
        for (j, c) in cols.iter_mut().enumerate() {
            c.push(num_f(f[2 + j], ln)?);
        }
    }
    Ok((metrics_table(t, cols), notes))
}

pub fn metrics_table(t: Vec<i64>, cols: [Vec<f64>; 6]) -> Table {
    let mut out = vec![("time".to_string(), Column::I64(t))];
    for (name, c) in METRICS_F64.iter().zip(cols) {
        out.push(((*name).to_string(), Column::F64(c)));
    }
    Table {
        meta: vec![],
        cols: out,
    }
}

/// Binance `bookDepth` daily CSV: `timestamp,percentage,depth,notional`.
pub fn book_depth_csv(text: &str) -> Result<(Table, ParseNotes)> {
    let (mut t, mut p, mut d, mut n) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    let mut notes = ParseNotes::default();
    for (ln, l) in lines(text) {
        if l.trim().is_empty() {
            notes.blank_lines += 1;
            continue;
        }
        if ln == 1 && is_header(l) {
            notes.header_lines += 1;
            continue;
        }
        let f: Vec<&str> = l.split(',').collect();
        if f.len() < 4 {
            return Err(Error::Parse(format!(
                "line {ln}: bookDepth row has {} fields",
                f.len()
            )));
        }
        t.push(
            parse_datetime(f[0])
                .ok_or_else(|| Error::Parse(format!("line {ln}: bad time {:?}", f[0])))?,
        );
        p.push(num_f(f[1], ln)?);
        d.push(num_f(f[2], ln)?);
        n.push(num_f(f[3], ln)?);
    }
    let table = Table {
        meta: vec![],
        cols: vec![
            ("time".into(), Column::I64(t)),
            ("pct".into(), Column::F64(p)),
            ("depth".into(), Column::F64(d)),
            ("notional".into(), Column::F64(n)),
        ],
    };
    Ok((table, notes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn kline_with_and_without_header() {
        let row =
            "1730419200000,70000.1,70010,69990,70005,1.5,1730419259999,105007.5,42,0.7,49003.5,0\n";
        let (k, n) = klines_csv(row).unwrap();
        assert_eq!((k.len(), n.header_lines), (1, 0));
        let with_h = format!(
            "open_time,open,high,low,close,volume,close_time,quote_volume,count,taker_buy_volume,taker_buy_quote_volume,ignore\r\n{row}"
        );
        let (k2, n2) = klines_csv(&with_h).unwrap();
        assert_eq!((k2.len(), n2.header_lines), (1, 1));
        assert_eq!(k2.trades[0], 42);
        assert!(klines_csv("1,2,3\n").is_err());
    }

    #[test]
    fn metrics_and_funding() {
        let m = "create_time,symbol,sum_open_interest,sum_open_interest_value,a,b,c,d\n2024-11-27 01:35:00,BTCUSDT,10,20,1,2,3,\n";
        let (t, _) = metrics_csv(m).unwrap();
        assert_eq!(
            t.i64("time").unwrap()[0],
            cs_core::time::ymd_hms_to_ms(2024, 11, 27, 1, 35, 0)
        );
        assert!(t.f64("taker_ls_vol").unwrap()[0].is_nan());
        let f = "calc_time,funding_interval_hours,last_funding_rate\n1730419200000,8,0.00010000\n";
        let (t, n) = funding_csv(f).unwrap();
        assert_eq!((t.nrows(), n.header_lines), (1, 1));
    }
}
