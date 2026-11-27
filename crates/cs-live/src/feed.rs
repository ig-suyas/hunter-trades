//! The transport thread: Binance USDS-M public endpoints only (REST klines and server time, the
//! `/market` websocket kline and mark-price streams). No keys, no signed requests, no order
//! endpoints, nothing uploaded.
//!
//! 1. Measure the exchange clock (`/fapi/v1/time`), again every [`DRIFT_EVERY`].
//! 2. Warm up: page back through REST klines (closed candles only) and push them through the
//!    live [`Pipeline`].
//! 3. Websocket loop with reconnect and backoff (the recorder's rules, `cs_data::record`). After
//!    every (re)connect the closed candles missed meanwhile are fetched over REST first; a
//!    closed frame that still arrives after a gap triggers the same backfill.
//! 4. Read with a 1 s timeout so stop requests, the stale guard and drift checks run even when
//!    the exchange is silent; a feed silent for 3x the stale limit is reconnected.

use std::fmt::Write as _;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, MutexGuard};
use std::thread::{self, JoinHandle};
use std::time::{Duration, Instant};

use cs_core::{Candle, Timeframe};
use cs_data::http::Client;
use cs_data::record::{backoff, next_attempt};
use tungstenite::Message;
use tungstenite::stream::MaybeTlsStream;

use crate::kline::{MarkMsg, parse_rest_klines};
use crate::now_ms;
use crate::pipeline::{PipeStats, Pipeline};
use crate::snapshot::ContextSnapshot;
use crate::status::{CLOSE_GRACE_MS, Drift, FeedStatus, classify};

/// How often the exchange clock is re-measured.
pub const DRIFT_EVERY: Duration = Duration::from_secs(300);

/// Feed settings.
#[derive(Debug, Clone)]
pub struct FeedConfig {
    pub symbol: String,
    pub tf: Timeframe,
    /// No frame for this long means FEED STALE. Default 15 s (klines arrive every ~0.25 s
    /// while trading and the mark price every second).
    pub stale_after: Duration,
    /// Closed candles loaded before going live (EMA 200 on 4h and the zone memory need history).
    pub warmup_bars: usize,
    pub rest_base: String,
    pub ws_base: String,
}

impl FeedConfig {
    pub fn new(symbol: &str, tf: Timeframe) -> FeedConfig {
        FeedConfig {
            symbol: symbol.to_uppercase(),
            tf,
            stale_after: Duration::from_secs(15),
            warmup_bars: default_warmup(tf),
            rest_base: "https://fapi.binance.com".into(),
            ws_base: "wss://fstream.binance.com/market/stream".into(),
        }
    }
}

/// Warm-up length: 1.25x what the 4h EMA 200 needs, at least the 1,500-candle zone memory, at
/// most 15,000 candles (10 REST pages).
pub fn default_warmup(tf: Timeframe) -> usize {
    let per4h = usize::try_from((Timeframe::H4.ms() / tf.ms()).max(1)).unwrap_or(1);
    (per4h * 250).clamp(1500, 15_000)
}

/// State shared with the panel.
#[derive(Debug, Clone)]
pub struct Shared {
    pub connected: bool,
    pub ever_connected: bool,
    pub last_msg: Option<Instant>,
    pub snapshot: Option<ContextSnapshot>,
    pub last_price: f64,
    pub mark: Option<MarkMsg>,
    pub drift: Option<Drift>,
    pub reconnects: u64,
    pub warmup_done: bool,
    pub warmup_candles: usize,
    /// Latest problem or phase, in plain words.
    pub note: String,
    pub stats: PipeStats,
}

impl Default for Shared {
    fn default() -> Self {
        Shared {
            connected: false,
            ever_connected: false,
            last_msg: None,
            snapshot: None,
            last_price: f64::NAN,
            mark: None,
            drift: None,
            reconnects: 0,
            warmup_done: false,
            warmup_candles: 0,
            note: "starting".into(),
            stats: PipeStats::default(),
        }
    }
}

impl Shared {
    /// Current feed status.
    pub fn status(&self, stale_after: Duration, tf_ms: i64) -> FeedStatus {
        let offset = self.drift.map_or(0, |d| d.offset_ms);
        let due = self.snapshot.map(|s| s.close_at + tf_ms + CLOSE_GRACE_MS);
        classify(
            self.connected,
            self.ever_connected,
            self.last_msg.map(|t| t.elapsed()),
            stale_after,
            if self.warmup_done { due } else { None },
            now_ms() + offset,
        )
    }
}

/// Handle to a running feed. Dropping it asks the thread to stop (it exits within about a
/// second, or after an in-flight REST request).
pub struct FeedHandle {
    pub cfg: FeedConfig,
    shared: Arc<Mutex<Shared>>,
    stop: Arc<AtomicBool>,
    join: Option<JoinHandle<()>>,
}

fn lock(s: &Mutex<Shared>) -> MutexGuard<'_, Shared> {
    s.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

impl FeedHandle {
    /// Copy of the shared state.
    pub fn read(&self) -> Shared {
        lock(&self.shared).clone()
    }

    pub fn status(&self) -> FeedStatus {
        lock(&self.shared).status(self.cfg.stale_after, self.cfg.tf.ms())
    }

    pub fn stop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
    }

    /// Stop and wait for the thread.
    pub fn join(mut self) {
        self.stop();
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

impl Drop for FeedHandle {
    fn drop(&mut self) {
        self.stop();
    }
}

/// Start the feed thread.
pub fn spawn(cfg: FeedConfig) -> FeedHandle {
    let shared = Arc::new(Mutex::new(Shared::default()));
    let stop = Arc::new(AtomicBool::new(false));
    let (s2, st2, c2) = (Arc::clone(&shared), Arc::clone(&stop), cfg.clone());
    let join = thread::Builder::new()
        .name(format!("cs-live {} {}", cfg.symbol, cfg.tf.label()))
        .spawn(move || run(&c2, &s2, &st2))
        .ok();
    FeedHandle {
        cfg,
        shared,
        stop,
        join,
    }
}

/// `GET /fapi/v1/time` and the NTP-style offset.
///
/// # Errors
/// Network or parse failure, as text.
pub fn measure_drift(client: &mut Client, rest: &str) -> Result<Drift, String> {
    let sent = now_ms();
    let v = client
        .get_json(&format!("{rest}/fapi/v1/time"))
        .map_err(|e| e.to_string())?;
    let recv = now_ms();
    let server = v["serverTime"].as_i64().ok_or("no serverTime")?;
    Ok(Drift::from_times(sent, server, recv))
}

fn rest_page(
    client: &mut Client,
    cfg: &FeedConfig,
    start: Option<i64>,
    end: Option<i64>,
) -> Result<Vec<(Candle, i64)>, String> {
    let mut url = format!(
        "{}/fapi/v1/klines?symbol={}&interval={}&limit=1500",
        cfg.rest_base,
        cfg.symbol,
        cfg.tf.label()
    );
    if let Some(s) = start {
        let _ = write!(url, "&startTime={s}");
    }
    if let Some(e) = end {
        let _ = write!(url, "&endTime={e}");
    }
    let v = client.get_json(&url).map_err(|e| e.to_string())?;
    if v.get("code").is_some() {
        return Err(format!("exchange error: {v}"));
    }
    Ok(parse_rest_klines(&v))
}

/// The newest `bars` closed candles (close time before `exchange_now`), oldest first.
///
/// # Errors
/// Network, exchange or parse failure, as text.
pub fn fetch_warmup(
    client: &mut Client,
    cfg: &FeedConfig,
    bars: usize,
    exchange_now: i64,
) -> Result<Vec<Candle>, String> {
    let mut out: Vec<Candle> = Vec::with_capacity(bars + 1500);
    let mut end: Option<i64> = None;
    while out.len() < bars {
        let page = rest_page(client, cfg, None, end)?;
        let mut got: Vec<Candle> = page
            .into_iter()
            .filter(|(_, ct)| *ct < exchange_now)
            .map(|(c, _)| c)
            .collect();
        if got.is_empty() {
            break;
        }
        let first = got[0].open_time;
        got.append(&mut out);
        out = got;
        end = Some(first - 1);
    }
    out.sort_by_key(|c| c.open_time);
    out.dedup_by_key(|c| c.open_time);
    let skip = out.len().saturating_sub(bars);
    Ok(out.split_off(skip))
}

/// Closed candles with `from <= open_time < to` (close before `exchange_now`), oldest first.
///
/// # Errors
/// Network, exchange or parse failure, as text.
pub fn fetch_range(
    client: &mut Client,
    cfg: &FeedConfig,
    from: i64,
    to: i64,
    exchange_now: i64,
) -> Result<Vec<Candle>, String> {
    let mut out = Vec::new();
    let mut start = from;
    while start < to {
        let page = rest_page(client, cfg, Some(start), None)?;
        let n = page.len();
        let mut last = start;
        for (c, ct) in page {
            last = last.max(c.open_time);
            if c.open_time >= from && c.open_time < to && ct < exchange_now {
                out.push(c);
            }
        }
        if n < 1500 || last + cfg.tf.ms() <= start {
            break;
        }
        start = last + cfg.tf.ms();
    }
    Ok(out)
}

fn set_read_timeout(
    ws: &mut tungstenite::WebSocket<MaybeTlsStream<std::net::TcpStream>>,
    d: Duration,
) {
    let r = match ws.get_mut() {
        MaybeTlsStream::Plain(s) => s.set_read_timeout(Some(d)),
        MaybeTlsStream::NativeTls(s) => s.get_ref().set_read_timeout(Some(d)),
        _ => Ok(()),
    };
    let _ = r;
}

fn sleep_until(stop: &AtomicBool, d: Duration) {
    let until = Instant::now() + d;
    while Instant::now() < until && !stop.load(Ordering::Relaxed) {
        thread::sleep(Duration::from_millis(100));
    }
}

fn publish(shared: &Mutex<Shared>, p: &Pipeline) {
    let mut s = lock(shared);
    s.snapshot = p.snapshot().copied();
    s.stats = p.stats;
    s.last_price = p.last_price;
    s.mark = p.mark;
}

// One loop with its reconnect state in locals reads better than splitting it up.
#[allow(clippy::too_many_lines)]
fn run(cfg: &FeedConfig, shared: &Mutex<Shared>, stop: &AtomicBool) {
    let mut client = Client::new(Duration::from_millis(250));
    client.max_retries = 1;
    // Clock checks get their own client, so the polite gap after a kline request is never
    // timed as part of the round trip (it would skew the offset estimate).
    let mut tclient = Client::new(Duration::from_millis(250));
    tclient.max_retries = 0;
    let mut pipe = Pipeline::new(&cfg.symbol, cfg.tf.label(), cfg.tf.ms());
    let mut offset = 0i64;
    let mut last_drift = Instant::now();
    let mut attempt = 0u32;

    // Clock and warm-up (retried with backoff until they work or a stop is requested).
    while !stop.load(Ordering::Relaxed) {
        match measure_drift(&mut tclient, &cfg.rest_base) {
            Ok(d) => {
                offset = d.offset_ms;
                lock(shared).drift = Some(d);
            }
            Err(e) => lock(shared).note = format!("exchange time: {e}"),
        }
        lock(shared).note = "warming up from REST klines".into();
        match fetch_warmup(&mut client, cfg, cfg.warmup_bars, now_ms() + offset) {
            Ok(c) if !c.is_empty() => {
                pipe.push_closed(&c);
                publish(shared, &pipe);
                let mut s = lock(shared);
                s.warmup_done = true;
                s.warmup_candles = c.len();
                s.note = String::new();
                break;
            }
            Ok(_) => lock(shared).note = "warm-up: no candles returned".into(),
            Err(e) => lock(shared).note = format!("warm-up failed: {e}"),
        }
        attempt = attempt.saturating_add(1);
        sleep_until(stop, backoff(attempt.min(6)));
    }
    last_drift = last_drift.checked_sub(DRIFT_EVERY).unwrap_or(last_drift);
    attempt = 0;

    let sym = cfg.symbol.to_lowercase();
    let url = format!(
        "{}?streams={sym}@kline_{}/{sym}@markPrice@1s",
        cfg.ws_base,
        cfg.tf.label()
    );
    while !stop.load(Ordering::Relaxed) {
        let started = Instant::now();
        let mut got_data = false;
        let reason = match tungstenite::connect(url.as_str()) {
            Ok((mut ws, _)) => {
                set_read_timeout(&mut ws, Duration::from_secs(1));
                {
                    let mut s = lock(shared);
                    if s.ever_connected {
                        s.reconnects += 1;
                    }
                    s.connected = true;
                    s.ever_connected = true;
                    s.last_msg = Some(Instant::now());
                    s.note = String::new();
                }
                // Catch up on candles that closed while we were away.
                if let Some(last) = pipe.engine().last_open() {
                    let now_x = now_ms() + offset;
                    let from = last + cfg.tf.ms();
                    if from + cfg.tf.ms() <= now_x {
                        match fetch_range(&mut client, cfg, from, now_x, now_x) {
                            Ok(c) => {
                                pipe.push_closed(&c);
                                publish(shared, &pipe);
                            }
                            Err(e) => lock(shared).note = format!("catch-up failed: {e}"),
                        }
                    }
                }
                let reason = loop {
                    if stop.load(Ordering::Relaxed) {
                        let _ = ws.close(None);
                        break "stop requested".to_string();
                    }
                    if last_drift.elapsed() >= DRIFT_EVERY {
                        last_drift = Instant::now();
                        if let Ok(d) = measure_drift(&mut tclient, &cfg.rest_base) {
                            offset = d.offset_ms;
                            lock(shared).drift = Some(d);
                        }
                    }
                    match ws.read() {
                        Ok(Message::Text(t)) => {
                            got_data = true;
                            let applied = {
                                let c = &mut client;
                                let mut bf = |from: i64, to: i64| {
                                    let nx = now_ms() + offset;
                                    fetch_range(c, cfg, from, to, nx).unwrap_or_default()
                                };
                                pipe.on_text(t.as_str(), &mut bf)
                            };
                            {
                                let mut s = lock(shared);
                                s.last_msg = Some(Instant::now());
                                s.last_price = pipe.last_price;
                                s.mark = pipe.mark;
                                s.stats = pipe.stats;
                            }
                            if !applied.is_empty() {
                                publish(shared, &pipe);
                            }
                        }
                        Ok(Message::Close(f)) => break format!("server closed: {f:?}"),
                        Ok(_) => {}
                        Err(tungstenite::Error::Io(e))
                            if matches!(
                                e.kind(),
                                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                            ) =>
                        {
                            let silent = lock(shared)
                                .last_msg
                                .is_none_or(|t| t.elapsed() > cfg.stale_after * 3);
                            if silent {
                                let _ = ws.close(None);
                                break "no data for 3x the stale limit: reconnecting".to_string();
                            }
                        }
                        Err(e) => break format!("read: {e}"),
                    }
                };
                lock(shared).connected = false;
                reason
            }
            Err(e) => format!("connect: {e}"),
        };
        if stop.load(Ordering::Relaxed) {
            break;
        }
        attempt = next_attempt(attempt, started.elapsed(), got_data);
        let wait = backoff(attempt.saturating_sub(1));
        lock(shared).note = format!("{reason}; retry in {} s", wait.as_secs());
        sleep_until(stop, wait);
    }
}
