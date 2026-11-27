//! Bybit v5 public REST: fallback for candles, funding and open interest.
//!
//! Bybit klines carry no trade count and no taker-buy volume, so those columns are stored as
//! `-1` / `NaN` and the file is tagged `source=bybit`.

use serde_json::Value;

use crate::http::Client;
use crate::store::RawKlines;
use crate::{Error, Market, Result};

pub const BASE: &str = "https://api.bybit.com";

fn category(m: Market) -> &'static str {
    match m {
        Market::Spot => "spot",
        Market::Um => "linear",
    }
}

fn list(v: &Value, url: &str) -> Result<Vec<Value>> {
    if v.get("retCode").and_then(Value::as_i64) != Some(0) {
        return Err(Error::Http(format!(
            "{url}: bybit error {}",
            v.get("retMsg").unwrap_or(v)
        )));
    }
    Ok(v.pointer("/result/list")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default())
}

fn f(v: &Value) -> f64 {
    v.as_str()
        .and_then(|s| s.parse().ok())
        .or_else(|| v.as_f64())
        .unwrap_or(f64::NAN)
}

fn i(v: &Value) -> Option<i64> {
    v.as_str()
        .and_then(|s| s.parse().ok())
        .or_else(|| v.as_i64())
}

/// 1m klines in [start, end).
pub fn klines(c: &mut Client, m: Market, sym: &str, start: i64, end: i64) -> Result<RawKlines> {
    let mut rows: Vec<(i64, [f64; 6])> = Vec::new();
    let mut s = start;
    while s < end {
        let e = (s + 1000 * 60_000).min(end) - 1;
        let url = format!(
            "{BASE}/v5/market/kline?category={}&symbol={sym}&interval=1&start={s}&end={e}&limit=1000",
            category(m)
        );
        for r in list(&c.get_json(&url)?, &url)? {
            let a = r.as_array().cloned().unwrap_or_default();
            if a.len() < 7 {
                continue;
            }
            let Some(t) = i(&a[0]) else { continue };
            if (start..end).contains(&t) {
                rows.push((
                    t,
                    [f(&a[1]), f(&a[2]), f(&a[3]), f(&a[4]), f(&a[5]), f(&a[6])],
                ));
            }
        }
        s = e + 1;
    }
    rows.sort_by_key(|r| r.0);
    let mut k = RawKlines::default();
    for (t, [o, h, l, cl, v, q]) in rows {
        k.open_time.push(t);
        k.open.push(o);
        k.high.push(h);
        k.low.push(l);
        k.close.push(cl);
        k.volume.push(v);
        k.close_time.push(t + 59_999);
        k.quote_volume.push(q);
        k.trades.push(-1);
        k.taker_buy_base.push(f64::NAN);
        k.taker_buy_quote.push(f64::NAN);
    }
    Ok(k)
}

/// Funding history in [start, end): (time, rate).
pub fn funding(c: &mut Client, sym: &str, start: i64, end: i64) -> Result<Vec<(i64, f64)>> {
    let mut out = Vec::new();
    let step = 200 * 8 * 3_600_000;
    let mut s = start;
    while s < end {
        let e = (s + step).min(end) - 1;
        let url = format!(
            "{BASE}/v5/market/funding/history?category=linear&symbol={sym}&startTime={s}&endTime={e}&limit=200"
        );
        for r in list(&c.get_json(&url)?, &url)? {
            if let Some(t) = r.get("fundingRateTimestamp").and_then(i) {
                out.push((t, r.get("fundingRate").map_or(f64::NAN, f)));
            }
        }
        s = e + 1;
    }
    out.sort_by_key(|r| r.0);
    out.dedup_by_key(|r| r.0);
    Ok(out)
}

/// 5-minute open interest in [start, end): (time, oi in base units).
pub fn open_interest(c: &mut Client, sym: &str, start: i64, end: i64) -> Result<Vec<(i64, f64)>> {
    let mut out = Vec::new();
    let step = 200 * 5 * 60_000;
    let mut s = start;
    while s < end {
        let e = (s + step).min(end) - 1;
        let url = format!(
            "{BASE}/v5/market/open-interest?category=linear&symbol={sym}&intervalTime=5min&startTime={s}&endTime={e}&limit=200"
        );
        for r in list(&c.get_json(&url)?, &url)? {
            if let Some(t) = r.get("timestamp").and_then(i) {
                out.push((t, r.get("openInterest").map_or(f64::NAN, f)));
            }
        }
        s = e + 1;
    }
    out.sort_by_key(|r| r.0);
    out.dedup_by_key(|r| r.0);
    Ok(out)
}
