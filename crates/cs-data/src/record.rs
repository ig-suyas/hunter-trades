//! `record`: live recorder for the data that has no free history (liquidations, order book) and
//! for live candles, trade flow, funding/mark price and open interest.
//!
//! One thread per WebSocket connection (blocking `tungstenite`, no async runtime), one REST
//! poller for open interest, one writer thread that owns all files.
//!
//! | Stream dir      | Source                                              | What is written                          |
//! |-----------------|-----------------------------------------------------|------------------------------------------|
//! | `kline_um`      | Binance futures `/market` `<sym>@kline_1m`           | each closed 1m candle (raw)              |
//! | `kline_spot`    | Binance spot `<sym>@kline_1m`                        | each closed 1m candle (raw)              |
//! | `trades_1s_um`  | Binance futures `/market` `<sym>@aggTrade`           | per-second buy/sell volume, count, OHLC  |
//! | `liq_binance`   | Binance futures `/market` `!forceOrder@arr`          | every event (raw)                        |
//! | `liq_bybit`     | Bybit `allLiquidation.<sym>`                         | every event (raw)                        |
//! | `mark_um`       | Binance futures `/market` `<sym>@markPrice@1s`       | mark, index, funding rate, sampled       |
//! | `ticker_bybit`  | Bybit `tickers.<sym>` (merged deltas)                | mark, funding, OI, best bid/ask, sampled |
//! | `bbo_um`        | Binance futures `/public` `<sym>@depth5@100ms`       | top of book (level 1), sampled           |
//! | `depth20_um`    | Binance futures `/public` `<sym>@depth20@500ms`      | 20-level book, sampled                   |
//! | `oi_um`         | Binance REST `/fapi/v1/openInterest` (polled)         | open interest                            |
//! | `events`        | the recorder                                        | connects, disconnects, errors, deletions |
//!
//! Files: `data/live/<stream>/<YYYY-MM-DD>/<stream>-<YYYY-MM-DD>T<HH>-<HHMMSS>.jsonl`, one per
//! stream per UTC hour, gzipped when the hour ends (or on stop / next start). Disk use is capped:
//! when the live folder exceeds the cap the oldest finished files are deleted (and logged).
//! Reconnects use exponential backoff with jitter (see [`next_attempt`]). Stop by creating
//! `data/live/STOP`.
//!
//! `bbo_um` is taken from level 1 of `depth5@100ms`, not from `bookTicker`: `bookTicker` sends
//! every top-of-book change (about 1,200-1,500 messages/s, ~300 kB/s for BTC + ETH) although only
//! one sample per second is kept, and that connection was reset by the server again and again
//! (2026-11-27). `depth5@100ms` gives the same sampled top of book at ~7 kB/s. Lines keep the
//! `bookTicker` keys (`s`, `b`, `B`, `a`, `A`, `E`, `T`, `u`) plus `"src": "depth5@100ms"`.

use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Read, Write};
use std::net::TcpStream;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread;
use std::time::{Duration, Instant};

use cs_core::time::{fmt_second, now_ms, split};
use flate2::Compression;
use flate2::write::GzEncoder;
use serde_json::{Map, Value, json};
use tungstenite::stream::MaybeTlsStream;
use tungstenite::{Message, WebSocket};

use crate::http::{Client, Fetched};
use crate::{Error, Result};

type Ws = WebSocket<MaybeTlsStream<TcpStream>>;

#[derive(Debug, Clone)]
pub struct RecordOpts {
    pub symbols: Vec<String>,
    pub cap_bytes: u64,
    pub bbo_every_s: i64,
    pub depth_every_s: i64,
    pub mark_every_s: i64,
    pub ticker_every_s: i64,
    pub oi_poll_s: u64,
    /// Testing: drop every connection after this many seconds (exercises reconnect).
    pub drop_after_s: Option<u64>,
    /// Testing: stop after this many seconds.
    pub run_for_s: Option<u64>,
}

impl Default for RecordOpts {
    fn default() -> Self {
        RecordOpts {
            symbols: vec!["BTCUSDT".into(), "ETHUSDT".into()],
            cap_bytes: 20 * 1024 * 1024 * 1024,
            bbo_every_s: 1,
            depth_every_s: 10,
            mark_every_s: 5,
            ticker_every_s: 5,
            oi_poll_s: 30,
            drop_after_s: None,
            run_for_s: None,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct ConnStatus {
    pub connected: bool,
    pub connects: u64,
    pub disconnects: u64,
    pub msgs: u64,
    pub last_msg: i64,
    pub last_error: String,
}

#[derive(Debug, Default)]
pub struct Status {
    pub started: i64,
    pub conns: HashMap<String, ConnStatus>,
    pub lines: HashMap<String, (u64, i64)>,
    pub agg_id_gaps: HashMap<String, u64>,
    pub disk_bytes: u64,
    pub deleted_files: u64,
    pub oi_polls_ok: u64,
    pub oi_polls_failed: u64,
}

type Shared = Arc<Mutex<Status>>;

fn lock(s: &Shared) -> std::sync::MutexGuard<'_, Status> {
    s.lock().unwrap_or_else(PoisonError::into_inner)
}

enum Msg {
    Line(&'static str, String),
    Shutdown,
}

/// Exponential backoff with jitter: 1, 2, 4 ... 60 s, plus up to 1 s.
pub fn backoff(attempt: u32) -> Duration {
    let base = 1u64 << attempt.min(6);
    let jitter = u64::from(
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(0, |d| d.subsec_millis()),
    );
    Duration::from_millis(base.min(60) * 1000 + jitter)
}

/// Number of consecutive failures after a connection ended; the wait is
/// `backoff(next_attempt(..) - 1)`.
///
/// - up for 60 s or more: back to 1 (wait 1 s);
/// - delivered data but dropped sooner: grow, but at most to 3 (wait 4 s), so a server that keeps
///   resetting a working connection does not push the wait to 60 s and lose most of the data;
/// - no data at all (connect failed, or closed before the first message): grow to the 60 s cap.
#[must_use]
pub fn next_attempt(attempt: u32, up: Duration, got_data: bool) -> u32 {
    if up >= Duration::from_secs(60) {
        1
    } else if got_data {
        attempt.min(2) + 1
    } else {
        attempt.saturating_add(1)
    }
}

/// Top of book from a partial-depth frame, in `bookTicker` field names.
fn bbo_from_depth(d: &Value) -> Option<Value> {
    let level = |side: &str| -> Option<(Value, Value)> {
        let l = d.get(side)?.as_array()?.first()?.as_array()?;
        Some((l.first()?.clone(), l.get(1)?.clone()))
    };
    let (b, bq) = level("b")?;
    let (a, aq) = level("a")?;
    Some(json!({
        "e": "bbo", "src": "depth5@100ms",
        "s": d.get("s")?, "E": d.get("E")?, "T": d.get("T"), "u": d.get("u"),
        "b": b, "B": bq, "a": a, "A": aq,
    }))
}

/// Emits at most once per bucket of `every_ms` per key.
#[derive(Default)]
struct Sampler(HashMap<String, i64>);

impl Sampler {
    fn due(&mut self, key: &str, t: i64, every_ms: i64) -> bool {
        let b = t.div_euclid(every_ms.max(1));
        match self.0.get_mut(key) {
            Some(last) if *last == b => false,
            Some(last) => {
                *last = b;
                true
            }
            None => {
                self.0.insert(key.to_string(), b);
                true
            }
        }
    }
}

fn num(v: &Value) -> f64 {
    v.as_str()
        .and_then(|s| s.parse().ok())
        .or_else(|| v.as_f64())
        .unwrap_or(f64::NAN)
}

/// Per-second aggregation of Binance aggTrades for one symbol.
#[derive(Debug, Clone, Default)]
pub struct SecondAgg {
    pub sec: i64,
    pub n: u64,
    pub trades: i64,
    pub buy_qty: f64,
    pub sell_qty: f64,
    pub buy_quote: f64,
    pub sell_quote: f64,
    pub o: f64,
    pub h: f64,
    pub l: f64,
    pub c: f64,
    pub first_id: i64,
    pub last_id: i64,
}

impl SecondAgg {
    fn start(sec: i64, px: f64, id: i64) -> SecondAgg {
        SecondAgg {
            sec,
            o: px,
            h: px,
            l: px,
            c: px,
            first_id: id,
            last_id: id,
            ..Default::default()
        }
    }

    /// Add one aggTrade. `buyer_maker` = the seller was the aggressor.
    pub fn add(&mut self, px: f64, qty: f64, buyer_maker: bool, id: i64, trades: i64) {
        self.n += 1;
        self.trades += trades;
        if buyer_maker {
            self.sell_qty += qty;
            self.sell_quote += px * qty;
        } else {
            self.buy_qty += qty;
            self.buy_quote += px * qty;
        }
        self.h = self.h.max(px);
        self.l = self.l.min(px);
        self.c = px;
        self.last_id = id;
    }

    pub fn to_json(&self, sym: &str, partial: bool) -> String {
        json!({"s": self.sec, "sym": sym, "n": self.n, "nt": self.trades,
               "bq": self.buy_qty, "sq": self.sell_qty, "bn": self.buy_quote, "sn": self.sell_quote,
               "o": self.o, "h": self.h, "l": self.l, "c": self.c,
               "a0": self.first_id, "a1": self.last_id, "partial": partial})
        .to_string()
    }
}

trait Handler: Send {
    fn name(&self) -> &'static str;
    fn url(&self) -> String;
    fn on_open(&mut self, _ws: &mut Ws) -> Result<()> {
        Ok(())
    }
    fn on_text(&mut self, txt: &str, now: i64, out: &Sender<Msg>, st: &Shared);
    fn on_tick(&mut self, _ws: &mut Ws, _now: i64) -> Result<()> {
        Ok(())
    }
    fn on_disconnect(&mut self, _out: &Sender<Msg>) {}
}

fn raw_line(now: i64, d: &Value) -> String {
    json!({"t": now, "d": d}).to_string()
}

fn send(out: &Sender<Msg>, stream: &'static str, line: String) {
    let _ = out.send(Msg::Line(stream, line));
}

fn combined(v: &Value) -> Option<(&str, &Value)> {
    Some((v.get("stream")?.as_str()?, v.get("data")?))
}

struct BinanceMarket {
    symbols: Vec<String>,
    mark_every_ms: i64,
    sampler: Sampler,
    secs: HashMap<String, SecondAgg>,
    last_agg_id: HashMap<String, i64>,
}

impl Handler for BinanceMarket {
    fn name(&self) -> &'static str {
        "binance_um_market"
    }

    fn url(&self) -> String {
        let mut s: Vec<String> = Vec::new();
        for sym in &self.symbols {
            let l = sym.to_lowercase();
            s.push(format!("{l}@kline_1m"));
            s.push(format!("{l}@aggTrade"));
            s.push(format!("{l}@markPrice@1s"));
        }
        s.push("!forceOrder@arr".into());
        format!(
            "wss://fstream.binance.com/market/stream?streams={}",
            s.join("/")
        )
    }

    fn on_text(&mut self, txt: &str, now: i64, out: &Sender<Msg>, st: &Shared) {
        let Ok(v) = serde_json::from_str::<Value>(txt) else {
            return;
        };
        let Some((stream, d)) = combined(&v) else {
            return;
        };
        if stream.ends_with("@kline_1m") {
            if d.pointer("/k/x").and_then(Value::as_bool) == Some(true) {
                send(out, "kline_um", raw_line(now, d));
            }
        } else if stream.ends_with("@aggTrade") {
            let sym = d
                .get("s")
                .and_then(Value::as_str)
                .unwrap_or("?")
                .to_string();
            let t = d.get("T").and_then(Value::as_i64).unwrap_or(now);
            let id = d.get("a").and_then(Value::as_i64).unwrap_or(-1);
            let px = d.get("p").map_or(f64::NAN, num);
            let qty = d.get("q").map_or(f64::NAN, num);
            let bm = d.get("m").and_then(Value::as_bool).unwrap_or(false);
            let nt = match (
                d.get("f").and_then(Value::as_i64),
                d.get("l").and_then(Value::as_i64),
            ) {
                (Some(f), Some(l)) => l - f + 1,
                _ => 1,
            };
            if let Some(&last) = self.last_agg_id.get(&sym)
                && id > last + 1
            {
                let gap = u64::try_from(id - last - 1).unwrap_or(0);
                *lock(st).agg_id_gaps.entry(sym.clone()).or_default() += gap;
            }
            self.last_agg_id.insert(sym.clone(), id);
            let sec = t.div_euclid(1000) * 1000;
            match self.secs.get_mut(&sym) {
                Some(a) if a.sec == sec => a.add(px, qty, bm, id, nt),
                _ => {
                    if let Some(prev) = self.secs.remove(&sym) {
                        send(out, "trades_1s_um", prev.to_json(&sym, false));
                    }
                    let mut a = SecondAgg::start(sec, px, id);
                    a.add(px, qty, bm, id, nt);
                    self.secs.insert(sym, a);
                }
            }
        } else if stream.contains("@markPrice") {
            let sym = d.get("s").and_then(Value::as_str).unwrap_or("?");
            if self.sampler.due(sym, now, self.mark_every_ms) {
                send(out, "mark_um", raw_line(now, d));
            }
        } else if stream == "!forceOrder@arr" {
            send(out, "liq_binance", raw_line(now, d));
        }
    }

    fn on_disconnect(&mut self, out: &Sender<Msg>) {
        for (sym, a) in self.secs.drain() {
            send(out, "trades_1s_um", a.to_json(&sym, true));
        }
    }
}

struct BinancePublic {
    symbols: Vec<String>,
    bbo_every_ms: i64,
    depth_every_ms: i64,
    sampler: Sampler,
}

impl Handler for BinancePublic {
    fn name(&self) -> &'static str {
        "binance_um_public"
    }

    fn url(&self) -> String {
        let mut s: Vec<String> = Vec::new();
        for sym in &self.symbols {
            let l = sym.to_lowercase();
            s.push(format!("{l}@depth5@100ms"));
            s.push(format!("{l}@depth20@500ms"));
        }
        format!(
            "wss://fstream.binance.com/public/stream?streams={}",
            s.join("/")
        )
    }

    fn on_text(&mut self, txt: &str, now: i64, out: &Sender<Msg>, _st: &Shared) {
        let Ok(v) = serde_json::from_str::<Value>(txt) else {
            return;
        };
        let Some((stream, d)) = combined(&v) else {
            return;
        };
        if stream.contains("@depth5@") {
            if self.sampler.due(stream, now, self.bbo_every_ms)
                && let Some(b) = bbo_from_depth(d)
            {
                send(out, "bbo_um", raw_line(now, &b));
            }
        } else if stream.contains("@depth20") && self.sampler.due(stream, now, self.depth_every_ms)
        {
            send(out, "depth20_um", raw_line(now, d));
        }
    }
}

struct BinanceSpot {
    symbols: Vec<String>,
}

impl Handler for BinanceSpot {
    fn name(&self) -> &'static str {
        "binance_spot"
    }

    fn url(&self) -> String {
        let s: Vec<String> = self
            .symbols
            .iter()
            .map(|x| format!("{}@kline_1m", x.to_lowercase()))
            .collect();
        format!(
            "wss://stream.binance.com:9443/stream?streams={}",
            s.join("/")
        )
    }

    fn on_text(&mut self, txt: &str, now: i64, out: &Sender<Msg>, _st: &Shared) {
        let Ok(v) = serde_json::from_str::<Value>(txt) else {
            return;
        };
        let Some((_, d)) = combined(&v) else { return };
        if d.pointer("/k/x").and_then(Value::as_bool) == Some(true) {
            send(out, "kline_spot", raw_line(now, d));
        }
    }
}

struct Bybit {
    symbols: Vec<String>,
    ticker_every_ms: i64,
    sampler: Sampler,
    state: HashMap<String, Map<String, Value>>,
    last_ping: Instant,
}

impl Handler for Bybit {
    fn name(&self) -> &'static str {
        "bybit_linear"
    }

    fn url(&self) -> String {
        "wss://stream.bybit.com/v5/public/linear".into()
    }

    fn on_open(&mut self, ws: &mut Ws) -> Result<()> {
        let mut args = Vec::new();
        for s in &self.symbols {
            args.push(format!("allLiquidation.{s}"));
            args.push(format!("tickers.{s}"));
        }
        let sub = json!({"op": "subscribe", "args": args}).to_string();
        ws.send(Message::text(sub))
            .map_err(|e| Error::Http(format!("bybit subscribe: {e}")))?;
        self.state.clear();
        self.last_ping = Instant::now();
        Ok(())
    }

    fn on_text(&mut self, txt: &str, now: i64, out: &Sender<Msg>, _st: &Shared) {
        let Ok(v) = serde_json::from_str::<Value>(txt) else {
            return;
        };
        if v.get("op").and_then(Value::as_str) == Some("subscribe") {
            event(out, "subscribe_ack", "bybit_linear", txt);
            return;
        }
        let Some(topic) = v.get("topic").and_then(Value::as_str) else {
            return;
        };
        if topic.starts_with("allLiquidation.") {
            send(out, "liq_bybit", raw_line(now, &v));
        } else if let Some(sym) = topic.strip_prefix("tickers.") {
            let Some(d) = v.get("data").and_then(Value::as_object) else {
                return;
            };
            let snapshot = v.get("type").and_then(Value::as_str) == Some("snapshot");
            let e = self.state.entry(sym.to_string()).or_default();
            if snapshot {
                e.clone_from(d);
            } else {
                for (k, val) in d {
                    e.insert(k.clone(), val.clone());
                }
            }
            if self.sampler.due(sym, now, self.ticker_every_ms) {
                let line = json!({"t": now, "ts": v.get("ts"), "sym": sym, "d": e}).to_string();
                send(out, "ticker_bybit", line);
            }
        }
    }

    fn on_tick(&mut self, ws: &mut Ws, _now: i64) -> Result<()> {
        if self.last_ping.elapsed() >= Duration::from_secs(20) {
            self.last_ping = Instant::now();
            ws.send(Message::text(r#"{"op":"ping"}"#))
                .map_err(|e| Error::Http(format!("bybit ping: {e}")))?;
        }
        Ok(())
    }
}

fn set_read_timeout(ws: &mut Ws, d: Duration) {
    let r = match ws.get_mut() {
        MaybeTlsStream::Plain(s) => s.set_read_timeout(Some(d)),
        MaybeTlsStream::NativeTls(s) => s.get_ref().set_read_timeout(Some(d)),
        _ => Ok(()),
    };
    let _ = r;
}

fn event(out: &Sender<Msg>, what: &str, conn: &str, detail: &str) {
    let line = json!({"t": now_ms(), "event": what, "conn": conn, "detail": detail}).to_string();
    send(out, "events", line);
}

/// Connection loop with reconnect and backoff. Returns when `stop` is set.
fn run_conn(
    mut h: Box<dyn Handler>,
    out: &Sender<Msg>,
    st: &Shared,
    stop: &AtomicBool,
    drop_after: Option<Duration>,
) {
    let name = h.name();
    let mut attempt = 0u32;
    while !stop.load(Ordering::Relaxed) {
        let url = h.url();
        let started = Instant::now();
        let mut got_data = false;
        let reason = match tungstenite::connect(url.as_str()) {
            Ok((mut ws, _)) => {
                set_read_timeout(&mut ws, Duration::from_secs(30));
                {
                    let mut s = lock(st);
                    let c = s.conns.entry(name.to_string()).or_default();
                    c.connected = true;
                    c.connects += 1;
                }
                event(out, "connected", name, &url);
                let reason = match h.on_open(&mut ws) {
                    Err(e) => e.to_string(),
                    Ok(()) => loop {
                        if stop.load(Ordering::Relaxed) {
                            let _ = ws.close(None);
                            break "stop requested".to_string();
                        }
                        if drop_after.is_some_and(|d| started.elapsed() >= d) {
                            let _ = ws.close(None);
                            break "test drop (--drop-after)".to_string();
                        }
                        let now = now_ms();
                        if let Err(e) = h.on_tick(&mut ws, now) {
                            break e.to_string();
                        }
                        match ws.read() {
                            Ok(Message::Text(t)) => {
                                got_data = true;
                                {
                                    let mut s = lock(st);
                                    let c = s.conns.entry(name.to_string()).or_default();
                                    c.msgs += 1;
                                    c.last_msg = now;
                                }
                                h.on_text(t.as_str(), now, out, st);
                            }
                            Ok(Message::Close(f)) => break format!("server closed: {f:?}"),
                            Ok(_) => {}
                            Err(e) => break format!("read: {e}"),
                        }
                    },
                };
                h.on_disconnect(out);
                {
                    let mut s = lock(st);
                    let c = s.conns.entry(name.to_string()).or_default();
                    c.connected = false;
                    c.disconnects += 1;
                    c.last_error.clone_from(&reason);
                }
                event(out, "disconnected", name, &reason);
                reason
            }
            Err(e) => {
                let r = format!("connect: {e}");
                lock(st)
                    .conns
                    .entry(name.to_string())
                    .or_default()
                    .last_error
                    .clone_from(&r);
                event(out, "connect_failed", name, &r);
                r
            }
        };
        if stop.load(Ordering::Relaxed) {
            break;
        }
        attempt = next_attempt(attempt, started.elapsed(), got_data);
        let wait = backoff(attempt.saturating_sub(1));
        event(
            out,
            "reconnect_wait",
            name,
            &format!("{} ms after: {reason}", wait.as_millis()),
        );
        let until = Instant::now() + wait;
        while Instant::now() < until && !stop.load(Ordering::Relaxed) {
            thread::sleep(Duration::from_millis(200));
        }
    }
}

fn run_oi_poller(
    symbols: &[String],
    every: Duration,
    out: &Sender<Msg>,
    st: &Shared,
    stop: &AtomicBool,
) {
    let mut c = Client::new(Duration::from_millis(250));
    c.max_retries = 2;
    let mut next = Instant::now();
    while !stop.load(Ordering::Relaxed) {
        if Instant::now() >= next {
            next += every;
            for sym in symbols {
                let url = format!("https://fapi.binance.com/fapi/v1/openInterest?symbol={sym}");
                let now = now_ms();
                match c.get(&url) {
                    Ok(Fetched::Body(b)) => match serde_json::from_slice::<Value>(&b) {
                        Ok(v) => {
                            send(out, "oi_um", raw_line(now, &v));
                            lock(st).oi_polls_ok += 1;
                        }
                        Err(e) => {
                            lock(st).oi_polls_failed += 1;
                            event(out, "oi_poll_failed", "binance_rest", &e.to_string());
                        }
                    },
                    Ok(Fetched::NotFound) => {
                        lock(st).oi_polls_failed += 1;
                    }
                    Err(e) => {
                        lock(st).oi_polls_failed += 1;
                        event(out, "oi_poll_failed", "binance_rest", &e.to_string());
                    }
                }
            }
        }
        thread::sleep(Duration::from_millis(500));
    }
}

// ---------------------------------------------------------------- writer

struct OpenFile {
    hour: i64,
    path: PathBuf,
    w: BufWriter<File>,
}

fn gzip_file(p: &Path) -> Result<()> {
    let mut data = Vec::new();
    File::open(p)?.read_to_end(&mut data)?;
    let gz = PathBuf::from(format!("{}.gz", p.display()));
    let tmp = PathBuf::from(format!("{}.gz.tmp", p.display()));
    {
        let mut e = GzEncoder::new(File::create(&tmp)?, Compression::new(6));
        e.write_all(&data)?;
        e.finish()?.sync_all()?;
    }
    fs::rename(&tmp, &gz)?;
    fs::remove_file(p)?;
    Ok(())
}

fn walk(dir: &Path, out: &mut Vec<(PathBuf, u64)>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    for e in rd.flatten() {
        let p = e.path();
        if p.is_dir() {
            walk(&p, out);
        } else if let Ok(m) = e.metadata() {
            out.push((p, m.len()));
        }
    }
}

struct Writer {
    root: PathBuf,
    files: HashMap<&'static str, OpenFile>,
    st: Shared,
    cap: u64,
    log: PathBuf,
}

impl Writer {
    fn write(&mut self, stream: &'static str, line: &str) -> Result<()> {
        let now = now_ms();
        let hour = now.div_euclid(3_600_000);
        if self.files.get(stream).is_some_and(|f| f.hour != hour)
            && let Some(f) = self.files.remove(stream)
        {
            Self::finish(f)?;
        }
        if !self.files.contains_key(stream) {
            let (y, m, d, h, mi, s) = split(now);
            let date = format!("{y:04}-{m:02}-{d:02}");
            let dir = self.root.join(stream).join(&date);
            fs::create_dir_all(&dir)?;
            let path = dir.join(format!("{stream}-{date}T{h:02}-{h:02}{mi:02}{s:02}.jsonl"));
            let f = OpenOptions::new().create(true).append(true).open(&path)?;
            self.files.insert(
                stream,
                OpenFile {
                    hour,
                    path,
                    w: BufWriter::new(f),
                },
            );
        }
        if let Some(f) = self.files.get_mut(stream) {
            f.w.write_all(line.as_bytes())?;
            f.w.write_all(b"\n")?;
        }
        let mut s = lock(&self.st);
        let e = s.lines.entry(stream.to_string()).or_default();
        e.0 += 1;
        e.1 = now;
        Ok(())
    }

    fn finish(mut f: OpenFile) -> Result<()> {
        f.w.flush()?;
        drop(f.w);
        gzip_file(&f.path)
    }

    fn flush_all(&mut self) {
        for f in self.files.values_mut() {
            let _ = f.w.flush();
        }
    }

    fn close_all(&mut self) {
        let files: Vec<OpenFile> = self.files.drain().map(|(_, f)| f).collect();
        for f in files {
            if let Err(e) = Self::finish(f) {
                self.log_line(&format!("gzip on close failed: {e}"));
            }
        }
    }

    fn log_line(&self, msg: &str) {
        if fs::metadata(&self.log).is_ok_and(|m| m.len() > 10 * 1024 * 1024) {
            let _ = fs::rename(&self.log, self.log.with_extension("log.old"));
        }
        if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&self.log) {
            let _ = writeln!(f, "[{}] {msg}", fmt_second(now_ms()));
        }
    }

    /// Measure disk use (every minute) and enforce the cap: delete the oldest `.jsonl.gz` files.
    fn enforce_cap(&mut self) {
        let mut all = Vec::new();
        walk(&self.root, &mut all);
        let mut total: u64 = all.iter().map(|x| x.1).sum();
        lock(&self.st).disk_bytes = total;
        if total <= self.cap {
            return;
        }
        let target = self.cap / 10 * 9;
        // Oldest first: sort by the date folder + file name (both UTC-ordered).
        let mut gz: Vec<(PathBuf, u64)> = all
            .into_iter()
            .filter(|(p, _)| p.to_string_lossy().ends_with(".jsonl.gz"))
            .collect();
        gz.sort_by_key(|(p, _)| {
            p.file_name().map(|n| {
                let n = n.to_string_lossy();
                // strip the stream prefix so all streams interleave by time
                n.split_once('-')
                    .map_or(n.to_string(), |(_, rest)| rest.to_string())
            })
        });
        for (p, n) in gz {
            if total <= target {
                break;
            }
            if fs::remove_file(&p).is_ok() {
                total -= n;
                lock(&self.st).deleted_files += 1;
                self.log_line(&format!("disk cap: deleted {}", p.display()));
            }
        }
        lock(&self.st).disk_bytes = total;
    }

    fn write_status(&self) {
        let s = lock(&self.st);
        let conns: Map<String, Value> = s
            .conns
            .iter()
            .map(|(k, c)| {
                (
                    k.clone(),
                    json!({"connected": c.connected, "connects": c.connects,
                           "disconnects": c.disconnects, "msgs": c.msgs,
                           "last_msg_utc": fmt_second(c.last_msg), "last_error": c.last_error}),
                )
            })
            .collect();
        let lines: Map<String, Value> = s
            .lines
            .iter()
            .map(|(k, (n, t))| (k.clone(), json!({"lines": n, "last_utc": fmt_second(*t)})))
            .collect();
        let v = json!({
            "pid": std::process::id(),
            "started_utc": fmt_second(s.started),
            "updated_utc": fmt_second(now_ms()),
            "updated_ms": now_ms(),
            "connections": conns,
            "streams": lines,
            "agg_trade_id_gaps": s.agg_id_gaps,
            "oi_polls_ok": s.oi_polls_ok,
            "oi_polls_failed": s.oi_polls_failed,
            "disk_bytes": s.disk_bytes,
            "cap_bytes": self.cap,
            "deleted_files_for_cap": s.deleted_files,
        });
        drop(s);
        let p = self.root.join("status.json");
        let tmp = self.root.join("status.json.tmp");
        if fs::write(&tmp, serde_json::to_string_pretty(&v).unwrap_or_default()).is_ok() {
            let _ = fs::rename(&tmp, &p);
        }
    }
}

fn writer_loop(mut w: Writer, rx: &Receiver<Msg>) {
    let mut last_flush = Instant::now();
    let mut last_status = Instant::now();
    let mut last_cap = Instant::now();
    w.enforce_cap();
    loop {
        match rx.recv_timeout(Duration::from_millis(500)) {
            Ok(Msg::Line(stream, line)) => {
                if stream == "events" {
                    w.log_line(&line);
                }
                if let Err(e) = w.write(stream, &line) {
                    w.log_line(&format!("write {stream} failed: {e}"));
                }
            }
            Ok(Msg::Shutdown) | Err(RecvTimeoutError::Disconnected) => break,
            Err(RecvTimeoutError::Timeout) => {}
        }
        if last_flush.elapsed() >= Duration::from_secs(5) {
            w.flush_all();
            last_flush = Instant::now();
        }
        if last_status.elapsed() >= Duration::from_secs(15) {
            w.write_status();
            last_status = Instant::now();
        }
        if last_cap.elapsed() >= Duration::from_secs(60) {
            w.enforce_cap();
            last_cap = Instant::now();
        }
    }
    // drain what is queued, then close
    while let Ok(Msg::Line(stream, line)) = rx.try_recv() {
        let _ = w.write(stream, &line);
    }
    w.close_all();
    w.enforce_cap();
    w.write_status();
    w.log_line("recorder stopped");
}

/// Gzip `.jsonl` files left behind by a previous run that was killed.
fn recover(root: &Path) -> u64 {
    let mut all = Vec::new();
    walk(root, &mut all);
    let mut n = 0;
    for (p, _) in all {
        if p.extension().is_some_and(|e| e == "jsonl") && gzip_file(&p).is_ok() {
            n += 1;
        }
    }
    n
}

/// Run the recorder until `data/live/STOP` appears (or `run_for_s` passes).
pub fn run(live_root: &Path, o: &RecordOpts) -> Result<()> {
    fs::create_dir_all(live_root)?;
    let lock_path = live_root.join("recorder.lock");
    let lock_file = OpenOptions::new()
        .create(true)
        .truncate(false)
        .write(true)
        .open(&lock_path)?;
    if lock_file.try_lock().is_err() {
        return Err(Error::Other(format!(
            "another recorder holds {}; stop it first",
            lock_path.display()
        )));
    }
    let stop_path = live_root.join("STOP");
    let _ = fs::remove_file(&stop_path);
    let recovered = recover(live_root);

    let st: Shared = Arc::new(Mutex::new(Status {
        started: now_ms(),
        ..Status::default()
    }));
    let stop = Arc::new(AtomicBool::new(false));
    let (tx, rx) = channel::<Msg>();
    let w = Writer {
        root: live_root.to_path_buf(),
        files: HashMap::new(),
        st: st.clone(),
        cap: o.cap_bytes,
        log: live_root.join("recorder.log"),
    };
    w.log_line(&format!(
        "recorder starting (pid {}), symbols {:?}, cap {} bytes, recovered {recovered} unfinished files",
        std::process::id(),
        o.symbols,
        o.cap_bytes
    ));
    let writer = thread::spawn(move || writer_loop(w, &rx));

    let drop_after = o.drop_after_s.map(Duration::from_secs);
    let handlers: Vec<Box<dyn Handler>> = vec![
        Box::new(BinanceMarket {
            symbols: o.symbols.clone(),
            mark_every_ms: o.mark_every_s * 1000,
            sampler: Sampler::default(),
            secs: HashMap::new(),
            last_agg_id: HashMap::new(),
        }),
        Box::new(BinancePublic {
            symbols: o.symbols.clone(),
            bbo_every_ms: o.bbo_every_s * 1000,
            depth_every_ms: o.depth_every_s * 1000,
            sampler: Sampler::default(),
        }),
        Box::new(BinanceSpot {
            symbols: o.symbols.clone(),
        }),
        Box::new(Bybit {
            symbols: o.symbols.clone(),
            ticker_every_ms: o.ticker_every_s * 1000,
            sampler: Sampler::default(),
            state: HashMap::new(),
            last_ping: Instant::now(),
        }),
    ];
    for h in handlers {
        let (tx, st, stop) = (tx.clone(), st.clone(), stop.clone());
        thread::spawn(move || run_conn(h, &tx, &st, &stop, drop_after));
    }
    {
        let (tx, st, stop) = (tx.clone(), st.clone(), stop.clone());
        let syms = o.symbols.clone();
        let every = Duration::from_secs(o.oi_poll_s.max(5));
        thread::spawn(move || run_oi_poller(&syms, every, &tx, &st, &stop));
    }

    let t0 = Instant::now();
    loop {
        thread::sleep(Duration::from_secs(1));
        if stop_path.exists()
            || o.run_for_s
                .is_some_and(|s| t0.elapsed() >= Duration::from_secs(s))
        {
            break;
        }
    }
    stop.store(true, Ordering::Relaxed);
    event(&tx, "stopping", "recorder", "stop requested");
    // Give connection threads a moment to flush partial seconds.
    thread::sleep(Duration::from_secs(2));
    let _ = tx.send(Msg::Shutdown);
    let _ = writer.join();
    let _ = fs::remove_file(&stop_path);
    drop(lock_file);
    Ok(())
}

/// Human-readable summary of `status.json` (for `record-status`).
pub fn status_text(live_root: &Path) -> Result<String> {
    let p = live_root.join("status.json");
    let v: Value = serde_json::from_str(&fs::read_to_string(&p)?)
        .map_err(|e| Error::Parse(format!("{}: {e}", p.display())))?;
    let age_s = (now_ms() - v.get("updated_ms").and_then(Value::as_i64).unwrap_or(0)) / 1000;
    let alive = live_root.join("recorder.lock").exists()
        && OpenOptions::new()
            .write(true)
            .open(live_root.join("recorder.lock"))
            .is_ok_and(|f| f.try_lock().is_err());
    let mut s = format!(
        "recorder {} (status updated {age_s} s ago, pid {})\n",
        if alive { "RUNNING" } else { "NOT running" },
        v.get("pid").unwrap_or(&Value::Null)
    );
    s.push_str(&serde_json::to_string_pretty(&v).unwrap_or_default());
    Ok(s)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sampler_once_per_bucket() {
        let mut s = Sampler::default();
        assert!(s.due("a", 1_000, 5_000));
        assert!(!s.due("a", 4_999, 5_000));
        assert!(s.due("a", 5_000, 5_000));
        assert!(s.due("b", 5_001, 5_000));
    }

    #[test]
    fn second_agg_splits_buy_sell() {
        let mut a = SecondAgg::start(1000, 100.0, 7);
        a.add(100.0, 2.0, false, 7, 3);
        a.add(99.0, 1.0, true, 8, 1);
        assert!(cs_core::price_eq(a.buy_qty, 2.0));
        assert!(cs_core::price_eq(a.sell_quote, 99.0));
        assert!(cs_core::price_eq(a.l, 99.0));
        assert_eq!((a.n, a.trades, a.last_id), (2, 4, 8));
        assert!(a.to_json("BTCUSDT", false).contains("\"bq\":2.0"));
    }

    #[test]
    fn backoff_caps() {
        assert!(backoff(0) >= Duration::from_secs(1));
        assert!(backoff(30) <= Duration::from_secs(61));
        assert!(backoff(3) >= Duration::from_secs(8));
    }

    #[test]
    fn next_attempt_rules() {
        let short = Duration::from_secs(12);
        let long = Duration::from_secs(61);
        // No data: grows without a low cap (backoff itself caps the wait at 60 s).
        let mut a = 0;
        for _ in 0..8 {
            a = next_attempt(a, short, false);
        }
        assert_eq!(a, 8);
        assert!(backoff(a - 1) >= Duration::from_secs(60));
        // Working connection that keeps being reset: the wait never exceeds ~4 s.
        let mut a = 0;
        for _ in 0..20 {
            a = next_attempt(a, short, true);
            assert!(backoff(a - 1) < Duration::from_secs(5));
        }
        assert_eq!(a, 3);
        // After a stable minute: back to the first step, even from the cap.
        assert_eq!(next_attempt(8, long, true), 1);
        assert_eq!(next_attempt(8, long, false), 1);
    }

    #[test]
    fn bbo_from_depth5_frame() {
        let d: Value = serde_json::from_str(
            r#"{"e":"depthUpdate","E":1795747309559,"T":1795747309557,"s":"BTCUSDT","ps":"BTCUSDT",
               "U":1,"u":12253259957116,"pu":0,
               "b":[["61582.30","1.487"],["61582.20","0.002"]],
               "a":[["61582.40","6.023"],["61582.50","0.217"]]}"#,
        )
        .unwrap();
        let b = bbo_from_depth(&d).unwrap();
        assert_eq!(b["s"], "BTCUSDT");
        assert_eq!(b["b"], "61582.30");
        assert_eq!(b["B"], "1.487");
        assert_eq!(b["a"], "61582.40");
        assert_eq!(b["A"], "6.023");
        assert_eq!(b["E"], 1_795_747_309_559_i64);
        assert_eq!(b["u"], 12_253_259_957_116_i64);
        assert_eq!(b["src"], "depth5@100ms");
        // An empty side gives no line instead of a half-filled one.
        let empty: Value =
            serde_json::from_str(r#"{"s":"BTCUSDT","E":1,"b":[],"a":[["1","1"]]}"#).unwrap();
        assert!(bbo_from_depth(&empty).is_none());
    }

    #[test]
    fn disk_cap_deletes_oldest_first() {
        let dir = std::env::temp_dir().join(format!("cs-cap-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        for (stream, day, hour) in [
            ("a", "2026-11-01", "00"),
            ("b", "2026-11-01", "05"),
            ("a", "2026-11-02", "00"),
        ] {
            let d = dir.join(stream).join(day);
            fs::create_dir_all(&d).unwrap();
            fs::write(
                d.join(format!("{stream}-{day}T{hour}-{hour}0000.jsonl.gz")),
                vec![0u8; 1000],
            )
            .unwrap();
        }
        let mut w = Writer {
            root: dir.clone(),
            files: HashMap::new(),
            st: Arc::new(Mutex::new(Status::default())),
            cap: 2500,
            log: dir.join("recorder.log"),
        };
        w.enforce_cap();
        // 3000 bytes > cap 2500 -> delete down to 90% (2250): only the oldest file goes.
        assert!(
            !dir.join("a/2026-11-01/a-2026-11-01T00-000000.jsonl.gz")
                .exists()
        );
        assert!(
            dir.join("b/2026-11-01/b-2026-11-01T05-050000.jsonl.gz")
                .exists()
        );
        assert!(
            dir.join("a/2026-11-02/a-2026-11-02T00-000000.jsonl.gz")
                .exists()
        );
        assert_eq!(lock(&w.st).deleted_files, 1);
        fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn gzip_and_recover() {
        let dir = std::env::temp_dir().join(format!("cs-rec-test-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("x")).unwrap();
        fs::write(dir.join("x").join("a.jsonl"), "{\"a\":1}\n").unwrap();
        assert_eq!(recover(&dir), 1);
        let gz = dir.join("x").join("a.jsonl.gz");
        let mut s = String::new();
        flate2::read::GzDecoder::new(File::open(&gz).unwrap())
            .read_to_string(&mut s)
            .unwrap();
        assert_eq!(s, "{\"a\":1}\n");
        fs::remove_dir_all(&dir).unwrap();
    }
}
