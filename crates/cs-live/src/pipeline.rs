//! The live path from websocket text to context: parse the frame, keep only the final frame of
//! each candle (`"x": true`), drop replays, backfill missing candles before going on, and feed
//! the engine. The transport thread ([`crate::feed`]) and the parity test drive this same code.

use cs_core::Candle;

use crate::engine::LiveEngine;
use crate::kline::{MarkMsg, WsMsg, parse_ws};
use crate::snapshot::ContextSnapshot;

/// Counters of the live path.
#[derive(Debug, Clone, Copy, Default)]
pub struct PipeStats {
    pub frames: u64,
    /// In-progress kline frames (ignored for the context).
    pub partial: u64,
    /// Closed candles applied (warm-up and backfill included).
    pub applied: u64,
    /// Closed candles at or before the last applied one (reconnect replays): ignored.
    pub duplicates: u64,
    /// Candles filled from the backfill source after a gap.
    pub backfilled: u64,
    /// Gaps the backfill could not close (the engine then skips the missing candles, as the
    /// batch preparation does).
    pub gaps_unfilled: u64,
    /// Candles refused by the engine's checks (invalid OHLC, off-grid open time).
    pub rejected: u64,
    /// Frames for another symbol or interval, or malformed.
    pub foreign: u64,
}

pub struct Pipeline {
    symbol: String,
    interval: String,
    engine: LiveEngine,
    pub stats: PipeStats,
    /// Last traded price seen in any kline frame (display only; the context uses closed
    /// candles).
    pub last_price: f64,
    /// Exchange event time of the latest kline frame.
    pub last_event_time: i64,
    pub mark: Option<MarkMsg>,
}

impl Pipeline {
    pub fn new(symbol: &str, interval: &str, tf_ms: i64) -> Pipeline {
        Pipeline {
            symbol: symbol.to_uppercase(),
            interval: interval.to_string(),
            engine: LiveEngine::new(tf_ms),
            stats: PipeStats::default(),
            last_price: f64::NAN,
            last_event_time: 0,
            mark: None,
        }
    }

    pub fn engine(&self) -> &LiveEngine {
        &self.engine
    }

    pub fn snapshot(&self) -> Option<&ContextSnapshot> {
        self.engine.snapshot()
    }

    /// Apply already-closed candles (warm-up, REST backfill), oldest first.
    pub fn push_closed(&mut self, candles: &[Candle]) -> Vec<ContextSnapshot> {
        let mut out = Vec::new();
        for c in candles {
            self.offer(*c, &mut |_, _| Vec::new(), &mut out);
        }
        out
    }

    /// Handle one websocket text frame. `backfill(from, to)` must return the closed candles with
    /// `from <= open_time < to` (it is called only when a closed candle arrives after a gap).
    /// Returns the snapshots of the candles applied by this frame, oldest first.
    pub fn on_text(
        &mut self,
        txt: &str,
        backfill: &mut dyn FnMut(i64, i64) -> Vec<Candle>,
    ) -> Vec<ContextSnapshot> {
        self.stats.frames += 1;
        let mut out = Vec::new();
        match parse_ws(txt) {
            Some(WsMsg::Kline {
                symbol,
                interval,
                k,
            }) if symbol == self.symbol && interval == self.interval => {
                self.last_price = k.candle.close;
                self.last_event_time = k.event_time;
                if k.closed {
                    self.offer(k.candle, backfill, &mut out);
                } else {
                    self.stats.partial += 1;
                }
            }
            Some(WsMsg::Mark { symbol, m }) if symbol == self.symbol => self.mark = Some(m),
            Some(WsMsg::Other) => {}
            _ => self.stats.foreign += 1,
        }
        out
    }

    fn offer(
        &mut self,
        c: Candle,
        backfill: &mut dyn FnMut(i64, i64) -> Vec<Candle>,
        out: &mut Vec<ContextSnapshot>,
    ) {
        let tf = self.engine.tf_ms();
        if let Some(last) = self.engine.last_open() {
            if c.open_time <= last {
                self.stats.duplicates += 1;
                return;
            }
            let expected = last + tf;
            if c.open_time > expected {
                let mut fill = backfill(expected, c.open_time);
                fill.sort_by_key(|x| x.open_time);
                for f in fill {
                    let newest = self.engine.last_open().unwrap_or(i64::MIN);
                    if f.open_time > newest && f.open_time < c.open_time {
                        self.apply(&f, out);
                        self.stats.backfilled += 1;
                    }
                }
                if self
                    .engine
                    .last_open()
                    .is_some_and(|l| l + tf < c.open_time)
                {
                    self.stats.gaps_unfilled += 1;
                }
            }
        }
        self.apply(&c, out);
    }

    fn apply(&mut self, c: &Candle, out: &mut Vec<ContextSnapshot>) {
        match self.engine.push_closed(c) {
            Ok(s) => {
                self.stats.applied += 1;
                out.push(s.snapshot);
            }
            Err(_) => self.stats.rejected += 1,
        }
    }
}
