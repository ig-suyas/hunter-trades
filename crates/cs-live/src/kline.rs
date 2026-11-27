//! Binance USDS-M public market-data messages: kline and mark-price websocket frames (raw or
//! combined-stream form) and REST kline arrays. Public endpoints only; nothing here is signed.

use cs_core::Candle;
use serde_json::Value;

/// One kline frame from the websocket.
#[derive(Debug, Clone, Copy)]
pub struct KlineMsg {
    /// Exchange event time (UTC ms).
    pub event_time: i64,
    pub candle: Candle,
    /// Inclusive close time (Binance style: open + length - 1).
    pub close_time: i64,
    /// `true` only on the final frame of a candle (`"x": true`).
    pub closed: bool,
}

/// Mark price frame (`<sym>@markPrice@1s`).
#[derive(Debug, Clone, Copy)]
pub struct MarkMsg {
    pub event_time: i64,
    pub mark: f64,
    /// Current funding rate estimate (positive: longs pay shorts).
    pub funding_rate: f64,
    pub next_funding: i64,
}

/// A parsed websocket frame.
#[derive(Debug, Clone)]
pub enum WsMsg {
    Kline {
        symbol: String,
        interval: String,
        k: KlineMsg,
    },
    Mark {
        symbol: String,
        m: MarkMsg,
    },
    Other,
}

fn num(v: &Value) -> f64 {
    match v {
        Value::String(s) => s.parse().unwrap_or(f64::NAN),
        Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        _ => f64::NAN,
    }
}

fn int(v: &Value) -> Option<i64> {
    match v {
        Value::Number(n) => n.as_i64(),
        Value::String(s) => s.parse().ok(),
        _ => None,
    }
}

/// Parse one websocket text frame. Malformed frames give `None`.
pub fn parse_ws(txt: &str) -> Option<WsMsg> {
    let v: Value = serde_json::from_str(txt).ok()?;
    let d = v.get("data").unwrap_or(&v);
    match d.get("e")?.as_str()? {
        "kline" => {
            let k = d.get("k")?;
            let candle = Candle {
                open_time: int(k.get("t")?)?,
                open: num(k.get("o")?),
                high: num(k.get("h")?),
                low: num(k.get("l")?),
                close: num(k.get("c")?),
                volume: num(k.get("v")?),
                quote_volume: k.get("q").map_or(f64::NAN, num),
                trades: k.get("n").and_then(int).unwrap_or(-1),
                taker_buy_base: k.get("V").map_or(f64::NAN, num),
                taker_buy_quote: k.get("Q").map_or(f64::NAN, num),
            };
            Some(WsMsg::Kline {
                symbol: d.get("s")?.as_str()?.to_string(),
                interval: k.get("i")?.as_str()?.to_string(),
                k: KlineMsg {
                    event_time: d.get("E").and_then(int).unwrap_or(0),
                    candle,
                    close_time: int(k.get("T")?)?,
                    closed: k.get("x")?.as_bool()?,
                },
            })
        }
        "markPriceUpdate" => Some(WsMsg::Mark {
            symbol: d.get("s")?.as_str()?.to_string(),
            m: MarkMsg {
                event_time: d.get("E").and_then(int).unwrap_or(0),
                mark: num(d.get("p")?),
                funding_rate: d.get("r").map_or(f64::NAN, num),
                next_funding: d.get("T").and_then(int).unwrap_or(0),
            },
        }),
        _ => Some(WsMsg::Other),
    }
}

/// Parse a REST `/fapi/v1/klines` response into `(candle, inclusive close time)` rows.
pub fn parse_rest_klines(v: &Value) -> Vec<(Candle, i64)> {
    let Some(rows) = v.as_array() else {
        return Vec::new();
    };
    rows.iter()
        .filter_map(|r| {
            let a = r.as_array()?;
            if a.len() < 11 {
                return None;
            }
            Some((
                Candle {
                    open_time: int(&a[0])?,
                    open: num(&a[1]),
                    high: num(&a[2]),
                    low: num(&a[3]),
                    close: num(&a[4]),
                    volume: num(&a[5]),
                    quote_volume: num(&a[7]),
                    trades: int(&a[8]).unwrap_or(-1),
                    taker_buy_base: num(&a[9]),
                    taker_buy_quote: num(&a[10]),
                },
                int(&a[6])?,
            ))
        })
        .collect()
}

/// Build a kline websocket frame (combined-stream form) for a candle. Used by the replay and
/// parity tests to drive the live path with historical candles exactly as the exchange would.
pub fn kline_frame(
    symbol: &str,
    interval: &str,
    tf_ms: i64,
    c: &Candle,
    closed: bool,
    event_time: i64,
) -> String {
    let d = serde_json::json!({
        "e": "kline",
        "E": event_time,
        "s": symbol,
        "k": {
            "t": c.open_time,
            "T": c.open_time + tf_ms - 1,
            "s": symbol,
            "i": interval,
            "o": format!("{}", c.open),
            "c": format!("{}", c.close),
            "h": format!("{}", c.high),
            "l": format!("{}", c.low),
            "v": format!("{}", c.volume),
            "n": c.trades,
            "x": closed,
            "q": format!("{}", c.quote_volume),
            "V": format!("{}", c.taker_buy_base),
            "Q": format!("{}", c.taker_buy_quote),
        }
    });
    serde_json::json!({
        "stream": format!("{}@kline_{interval}", symbol.to_lowercase()),
        "data": d
    })
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_kline_frame() {
        let c = Candle {
            open_time: 1_700_000_100_000,
            open: 91_234.5,
            high: 91_400.1,
            low: 91_100.0,
            close: 91_300.25,
            volume: 123.456,
            quote_volume: 11_270_000.5,
            trades: 4242,
            taker_buy_base: 60.1,
            taker_buy_quote: 5_480_000.25,
        };
        let f = kline_frame("BTCUSDT", "15m", 900_000, &c, true, 1_700_000_999_999);
        let Some(WsMsg::Kline {
            symbol,
            interval,
            k,
        }) = parse_ws(&f)
        else {
            panic!("not a kline: {f}");
        };
        assert_eq!(symbol, "BTCUSDT");
        assert_eq!(interval, "15m");
        assert!(k.closed);
        assert_eq!(k.close_time, c.open_time + 899_999);
        assert_eq!(format!("{:?}", k.candle), format!("{c:?}"));
    }

    #[test]
    fn mark_and_rest() {
        let m = r#"{"stream":"btcusdt@markPrice@1s","data":{"e":"markPriceUpdate","E":1,"s":"BTCUSDT","p":"91000.5","i":"91001","P":"91002","r":"0.00010000","T":1700006400000}}"#;
        let Some(WsMsg::Mark { m, .. }) = parse_ws(m) else {
            panic!("no mark");
        };
        assert!((m.funding_rate - 1e-4).abs() < 1e-15);
        let v: Value = serde_json::from_str(
            r#"[[1700000000000,"1","2","0.5","1.5","10",1700000899999,"15",7,"4","6","0"]]"#,
        )
        .unwrap_or(Value::Null);
        let r = parse_rest_klines(&v);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].1, 1_700_000_899_999);
        assert_eq!(r[0].0.trades, 7);
        assert!(parse_ws("not json").is_none());
    }
}
