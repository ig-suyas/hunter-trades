//! `fetch`: download ~24 months of history, verify, convert to the columnar store.
//!
//! Resumable: a period whose store file exists is skipped; a verified raw zip is reused; a
//! partial download (`*.part`) resumes with HTTP Range. Sequential and paced (see `http`).
//! Primary = Binance Vision archive. Fallback = Bybit REST, used only when the archive is
//! blocked (network failure after retries, 403 or 451) or a file older than 3 days is missing.

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use cs_core::time::{self, day_start, month_start, next_month};
use serde_json::Value;

use crate::http::Client;
use crate::store::{self, Table};
use crate::vision::{self, Dataset, Period};
use crate::{DataDir, Error, Market, Result, bybit, log, parse, zipread};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceChoice {
    /// Binance Vision, with automatic fallback to Bybit when blocked.
    Auto,
    /// Force the Bybit fallback (used to test that path).
    Bybit,
}

#[derive(Debug, Clone)]
pub struct FetchOpts {
    pub symbols: Vec<String>,
    pub markets: Vec<Market>,
    /// First UTC day (ms, midnight).
    pub start: i64,
    /// Exclusive end (ms, midnight); default = start of the current UTC day.
    pub end: i64,
    pub funding: bool,
    pub metrics: bool,
    pub book_depth: bool,
    pub source: SourceChoice,
    pub min_gap: Duration,
}

impl FetchOpts {
    /// `months` full calendar months before the current one, plus the current month to yesterday.
    pub fn default_range(months: i64) -> (i64, i64) {
        let now = time::now_ms();
        let (y, m, ..) = time::split(now);
        let total = y * 12 + (m - 1) - months;
        (
            month_start(total.div_euclid(12), total.rem_euclid(12) + 1),
            day_start(now),
        )
    }
}

#[derive(Debug, Clone, Default)]
pub struct Counter {
    pub units: u64,
    pub skipped_done: u64,
    pub stored_vision: u64,
    pub stored_fallback: u64,
    pub stored_rest: u64,
    pub not_published: u64,
    pub failed: u64,
    pub checksum_ok: u64,
    pub checksum_retry: u64,
    pub rows: u64,
}

#[derive(Debug, Default)]
pub struct FetchSummary {
    pub per: BTreeMap<String, Counter>,
    pub failures: Vec<String>,
    pub requests: u64,
    pub retries: u64,
    pub throttled: u64,
}

enum Outcome {
    Stored { rows: u64, retried_checksum: bool },
    NotFound,
}

/// Monthly periods for complete months inside the range, daily periods elsewhere.
pub fn periods(start: i64, end: i64, now: i64) -> Vec<Period> {
    let cur_month = {
        let (y, m, ..) = time::split(now);
        month_start(y, m)
    };
    let mut out = Vec::new();
    let (mut y, mut m, ..) = time::split(start);
    loop {
        let ms = month_start(y, m);
        if ms >= end {
            break;
        }
        let (ny, nm) = next_month(y, m);
        let me = month_start(ny, nm);
        if ms >= start && me <= end && me <= cur_month {
            out.push(Period::Month(y, m));
        } else {
            let mut d = ms.max(start);
            while d < me.min(end) {
                let (yy, mm, dd, ..) = time::split(d);
                out.push(Period::Day(yy, mm, dd));
                d += 86_400_000;
            }
        }
        (y, m) = (ny, nm);
    }
    out
}

/// Daily periods only (for daily-only datasets).
pub fn days(start: i64, end: i64) -> Vec<Period> {
    let mut out = Vec::new();
    let mut d = start;
    while d < end {
        let (y, m, dd, ..) = time::split(d);
        out.push(Period::Day(y, m, dd));
        d += 86_400_000;
    }
    out
}

fn is_blocked(e: &Error) -> bool {
    matches!(e, Error::Http(_) | Error::Status(403 | 451, _))
}

fn store_path(dd: &DataDir, ds: Dataset, sym: &str, p: &Period) -> PathBuf {
    let dir = match ds {
        Dataset::Klines1m(m) => dd.klines_1m(m, sym),
        Dataset::Funding => dd.funding(sym),
        Dataset::Metrics => dd.metrics(sym),
        Dataset::BookDepth => dd.book_depth(sym),
    };
    dir.join(format!("{}.csc", p.label()))
}

fn meta(pairs: &[(&str, String)]) -> Vec<(String, String)> {
    pairs
        .iter()
        .map(|(k, v)| ((*k).to_string(), v.clone()))
        .collect()
}

fn download_verified(c: &mut Client, rel: &str, zip: &Path) -> Result<Option<(String, bool)>> {
    let ck = zip.with_extension("zip.CHECKSUM");
    let url = vision::url(rel);
    if !ck.exists() && !c.download(&format!("{url}.CHECKSUM"), &ck)? {
        return Ok(None);
    }
    if !zip.exists() && !c.download(&url, zip)? {
        return Ok(None);
    }
    match vision::verify(zip) {
        Ok(h) => Ok(Some((h, false))),
        Err(Error::Checksum(msg)) => {
            log(&format!("{msg}; downloading again"));
            fs::remove_file(zip)?;
            fs::remove_file(&ck)?;
            if !c.download(&format!("{url}.CHECKSUM"), &ck)? || !c.download(&url, zip)? {
                return Ok(None);
            }
            vision::verify(zip).map(|h| Some((h, true)))
        }
        Err(e) => Err(e),
    }
}

fn vision_unit(
    c: &mut Client,
    dd: &DataDir,
    ds: Dataset,
    sym: &str,
    p: &Period,
    dest: &Path,
) -> Result<Outcome> {
    let rel = vision::rel_path(ds, sym, p);
    let zip = vision::raw_path(&dd.raw(), &rel);
    let Some((sha, retried)) = download_verified(c, &rel, &zip)? else {
        return Ok(Outcome::NotFound);
    };
    let (name, bytes) = zipread::first_entry(&fs::read(&zip)?)?;
    let text = String::from_utf8(bytes).map_err(|_| Error::Parse(format!("{name}: not UTF-8")))?;
    let parsed = match ds {
        Dataset::Klines1m(_) => parse::klines_csv(&text).map(|(k, n)| (k.into_table(vec![]), n)),
        Dataset::Funding => parse::funding_csv(&text),
        Dataset::Metrics => parse::metrics_csv(&text),
        Dataset::BookDepth => parse::book_depth_csv(&text),
    };
    let (table, notes) = parsed.map_err(|e| Error::Parse(format!("{name}: {e}")))?;
    let rows = table.nrows() as u64;
    let t = Table {
        meta: meta(&[
            ("source", "binance-vision".into()),
            ("file", rel.clone()),
            ("sha256", sha),
            ("header_lines", notes.header_lines.to_string()),
            ("blank_lines", notes.blank_lines.to_string()),
            ("period", p.label()),
        ]),
        cols: table.cols,
    };
    store::write_file(dest, &t)?;
    Ok(Outcome::Stored {
        rows,
        retried_checksum: retried,
    })
}

fn fallback_unit(c: &mut Client, ds: Dataset, sym: &str, p: &Period, dest: &Path) -> Result<u64> {
    let (s, e) = p.range();
    let m = |src: &str| {
        meta(&[
            ("source", src.to_string()),
            ("period", p.label()),
            ("partial", "0".into()),
        ])
    };
    let t = match ds {
        Dataset::Klines1m(mk) => bybit::klines(c, mk, sym, s, e)?.into_table(m("bybit")),
        Dataset::Funding => {
            let rows = bybit::funding(c, sym, s, e)?;
            let n = rows.len();
            let mut t = parse::funding_table(
                rows.iter().map(|r| r.0).collect(),
                vec![-1; n],
                rows.iter().map(|r| r.1).collect(),
            );
            t.meta = m("bybit");
            t
        }
        Dataset::Metrics => {
            let rows = bybit::open_interest(c, sym, s, e)?;
            let n = rows.len();
            let nan = || vec![f64::NAN; n];
            let mut t = parse::metrics_table(
                rows.iter().map(|r| r.0).collect(),
                [
                    rows.iter().map(|r| r.1).collect(),
                    nan(),
                    nan(),
                    nan(),
                    nan(),
                    nan(),
                ],
            );
            t.meta = m("bybit");
            t
        }
        Dataset::BookDepth => {
            return Err(Error::Other("no fallback for bookDepth".into()));
        }
    };
    let rows = t.nrows() as u64;
    store::write_file(dest, &t)?;
    Ok(rows)
}

/// Funding for the current (incomplete) month from Binance REST; refreshed on every run.
fn funding_rest(c: &mut Client, sym: &str, s: i64, e: i64, dest: &Path) -> Result<u64> {
    let url = format!(
        "https://fapi.binance.com/fapi/v1/fundingRate?symbol={sym}&startTime={s}&endTime={}&limit=1000",
        e - 1
    );
    let v = c.get_json(&url)?;
    let arr = v.as_array().cloned().unwrap_or_default();
    let (mut t, mut h, mut r) = (Vec::new(), Vec::new(), Vec::new());
    for x in arr {
        let ft = x.get("fundingTime").and_then(Value::as_i64);
        let fr = x
            .get("fundingRate")
            .and_then(Value::as_str)
            .and_then(|s| s.parse::<f64>().ok());
        if let (Some(ft), Some(fr)) = (ft, fr) {
            t.push(ft);
            h.push(-1);
            r.push(fr);
        }
    }
    let mut table = parse::funding_table(t, h, r);
    table.meta = meta(&[("source", "binance-rest".into()), ("partial", "1".into())]);
    let n = table.nrows() as u64;
    store::write_file(dest, &table)?;
    Ok(n)
}

fn is_done(dest: &Path) -> bool {
    // A store file marked partial (current month) is always refreshed.
    dest.exists() && store::read_file(dest).is_ok_and(|t| t.meta("partial") != Some("1"))
}

#[allow(clippy::too_many_lines)]
pub fn run(dd: &DataDir, o: &FetchOpts) -> Result<FetchSummary> {
    let mut c = Client::new(o.min_gap);
    let mut sum = FetchSummary::default();
    let now = time::now_ms();
    let recent = day_start(now) - 3 * 86_400_000;

    let mut jobs: Vec<(Dataset, String, Period)> = Vec::new();
    for sym in &o.symbols {
        for &m in &o.markets {
            for p in periods(o.start, o.end, now) {
                jobs.push((Dataset::Klines1m(m), sym.clone(), p));
            }
        }
        if o.funding {
            for p in periods(o.start, o.end, now) {
                // Funding has no daily archive: collapse days to their month.
                let pm = match p {
                    Period::Day(y, m, _) => Period::Month(y, m),
                    x @ Period::Month(..) => x,
                };
                if !jobs
                    .iter()
                    .any(|(d, s, pp)| *d == Dataset::Funding && s == sym && *pp == pm)
                {
                    jobs.push((Dataset::Funding, sym.clone(), pm));
                }
            }
        }
        if o.metrics {
            for p in days(o.start, o.end) {
                jobs.push((Dataset::Metrics, sym.clone(), p));
            }
        }
        if o.book_depth {
            for p in days(o.start, o.end) {
                jobs.push((Dataset::BookDepth, sym.clone(), p));
            }
        }
    }
    let total = jobs.len();
    log(&format!("fetch: {total} archive periods to check"));

    for (i, (ds, sym, p)) in jobs.into_iter().enumerate() {
        let key = format!("{} {sym}", ds.name());
        let dest = store_path(dd, ds, &sym, &p);
        let ctr = sum.per.entry(key.clone()).or_default();
        ctr.units += 1;
        if is_done(&dest) {
            ctr.skipped_done += 1;
            if let Ok(t) = store::read_file(&dest) {
                ctr.rows += t.nrows() as u64;
            }
            continue;
        }
        if i % 25 == 0 {
            log(&format!("[{i}/{total}] {key} {}", p.label()));
        }
        let (ps, pe) = p.range();
        let partial_month = matches!(p, Period::Month(..)) && pe > o.end;
        let res: Result<(u64, &str, bool)> = if ds == Dataset::Funding && partial_month {
            funding_rest(&mut c, &sym, ps.max(o.start), o.end, &dest).map(|n| (n, "rest", false))
        } else if o.source == SourceChoice::Bybit {
            fallback_unit(&mut c, ds, &sym, &p, &dest).map(|n| (n, "fallback", false))
        } else {
            match vision_unit(&mut c, dd, ds, &sym, &p, &dest) {
                Ok(Outcome::Stored {
                    rows,
                    retried_checksum,
                }) => Ok((rows, "vision", retried_checksum)),
                Ok(Outcome::NotFound) if ps >= recent => Ok((0, "unpublished", false)),
                Ok(Outcome::NotFound) => {
                    log(&format!(
                        "{key} {}: not in archive, using Bybit fallback",
                        p.label()
                    ));
                    fallback_unit(&mut c, ds, &sym, &p, &dest).map(|n| (n, "fallback", false))
                }
                Err(e) if is_blocked(&e) && ds != Dataset::BookDepth => {
                    log(&format!(
                        "{key} {}: archive blocked ({e}); using Bybit",
                        p.label()
                    ));
                    fallback_unit(&mut c, ds, &sym, &p, &dest).map(|n| (n, "fallback", false))
                }
                Err(e) => Err(e),
            }
        };
        let ctr = sum.per.entry(key.clone()).or_default();
        match res {
            Ok((n, how, retried)) => {
                ctr.rows += n;
                match how {
                    "vision" => {
                        ctr.stored_vision += 1;
                        ctr.checksum_ok += 1;
                    }
                    "fallback" => ctr.stored_fallback += 1,
                    "rest" => ctr.stored_rest += 1,
                    _ => ctr.not_published += 1,
                }
                if retried {
                    ctr.checksum_retry += 1;
                }
            }
            Err(e) => {
                ctr.failed += 1;
                let msg = format!("{key} {}: {e}", p.label());
                log(&format!("FAILED {msg}"));
                sum.failures.push(msg);
            }
        }
    }
    sum.requests = c.requests;
    sum.retries = c.retries;
    sum.throttled = c.throttled;
    Ok(sum)
}

/// Markdown summary of a fetch run.
pub fn summary_md(s: &FetchSummary, o: &FetchOpts) -> String {
    let mut out = String::new();
    let _ = writeln!(
        out,
        "Range: {} to {} (exclusive), symbols {:?}, markets {:?}, source {:?}\n",
        time::fmt_date(o.start),
        time::fmt_date(o.end),
        o.symbols,
        o.markets.iter().map(|m| m.dir()).collect::<Vec<_>>(),
        o.source
    );
    out.push_str("| Dataset | Periods | Already stored | From archive (SHA-256 OK) | Checksum re-downloads | Fallback (Bybit) | REST (current month) | Not yet published | Failed | Rows |\n|---|---|---|---|---|---|---|---|---|---|\n");
    for (k, c) in &s.per {
        let _ = writeln!(
            out,
            "| {k} | {} | {} | {} | {} | {} | {} | {} | {} | {} |",
            c.units,
            c.skipped_done,
            c.checksum_ok,
            c.checksum_retry,
            c.stored_fallback,
            c.stored_rest,
            c.not_published,
            c.failed,
            c.rows
        );
    }
    let _ = writeln!(
        out,
        "\nHTTP requests this run: {}, retries: {}, rate-limit pauses: {}",
        s.requests, s.retries, s.throttled
    );
    if !s.failures.is_empty() {
        out.push_str("\nFailures:\n");
        for f in &s.failures {
            let _ = writeln!(out, "- {f}");
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use cs_core::time::ymd_hms_to_ms;

    #[test]
    fn period_plan_mixes_months_and_days() {
        let now = ymd_hms_to_ms(2026, 11, 26, 21, 0, 0);
        let p = periods(
            ymd_hms_to_ms(2026, 9, 1, 0, 0, 0),
            ymd_hms_to_ms(2026, 11, 26, 0, 0, 0),
            now,
        );
        assert_eq!(p[0], Period::Month(2026, 9));
        assert_eq!(p[1], Period::Month(2026, 10));
        assert_eq!(p[2], Period::Day(2026, 11, 1));
        assert_eq!(p.last(), Some(&Period::Day(2026, 11, 25)));
        assert_eq!(p.len(), 2 + 25);
        // A range starting mid-month uses days for that month.
        let p = periods(
            ymd_hms_to_ms(2026, 10, 30, 0, 0, 0),
            ymd_hms_to_ms(2026, 11, 2, 0, 0, 0),
            now,
        );
        assert_eq!(p.len(), 3);
    }
}
