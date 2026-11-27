//! cs-live: the live context engine.
//!
//! Exchange klines in, context out, through the **same detector and context code as the
//! backtest** (`cs_patterns::Runner` + `cs_patterns::context::Context`, built exactly as
//! `cs_backtest::market::Market::prepare` builds them). Updates happen only on closed candles,
//! strictly forward; nothing already shown is ever repainted or removed.
//!
//! The GATE (Phase 4) found no setup that survives costs (0 of 59), so this crate produces
//! **context only**: trend state, regime, volatility, session, zones, the latest sweep / `BOS` /
//! `CHoCH` and the healthy-pullback flag. It has no direction call, score or probability, and it
//! never places orders: only public market-data endpoints are used.
//!
//! * [`kline`]: websocket and REST message parsing.
//! * [`engine`]: [`engine::LiveEngine`], one closed candle at a time.
//! * [`snapshot`]: [`snapshot::ContextSnapshot`], what the panel shows.
//! * [`pipeline`]: websocket text to context (closed frames only, replays dropped, gaps
//!   backfilled). The transport and the parity test drive this same code.
//! * [`status`]: live / stale / disconnected, and clock drift.
//! * [`feed`]: the transport thread (REST warm-up, websocket with reconnect and backoff).

pub mod engine;
pub mod feed;
pub mod kline;
pub mod pipeline;
pub mod snapshot;
pub mod status;

use cs_core::Timeframe;

/// Symbols the panel supports: the ones with downloaded history, tested detectors and a
/// measured cost model (Binance USDS-M perpetuals).
pub const SUPPORTED_SYMBOLS: [&str; 2] = ["BTCUSDT", "ETHUSDT"];

/// Time frames the panel supports (the ones Phase 2-4 ran on).
pub const SUPPORTED_TFS: [Timeframe; 3] = [Timeframe::M5, Timeframe::M15, Timeframe::H1];

pub fn is_supported(symbol: &str) -> bool {
    SUPPORTED_SYMBOLS.contains(&symbol.to_uppercase().as_str())
}

/// Local clock, UTC milliseconds.
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}
