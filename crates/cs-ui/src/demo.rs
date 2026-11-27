//! Sample data for the screenshots and the resource check: a real context snapshot computed by
//! the live path from stored BTCUSDT 15m candles **before the holdout seal** (the sealed period
//! is cut off before anything runs), with the feed state set by hand. Clearly labelled as
//! sample data in the panel.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use cs_core::{Candle, time};
use cs_live::feed::Shared;
use cs_live::kline::{MarkMsg, kline_frame};
use cs_live::pipeline::Pipeline;
use cs_live::status::{Drift, FeedStatus};

const TF: i64 = 900_000;

/// Which state to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemoState {
    Live,
    Stale,
    Disconnected,
    NoData,
    /// A new historical candle through the live path every second (for the CPU check).
    Replay,
}

impl DemoState {
    pub fn parse(s: &str) -> Option<DemoState> {
        Some(match s {
            "live" | "muted" | "unsupported" | "mismatch" => DemoState::Live,
            "stale" => DemoState::Stale,
            "disconnected" => DemoState::Disconnected,
            "nodata" => DemoState::NoData,
            "replay" => DemoState::Replay,
            _ => return None,
        })
    }
}

/// Pre-seal BTCUSDT 15m candles from the store (empty if the store or seal is missing).
pub fn sealed_candles(root: &Path) -> Vec<Candle> {
    let boundary = std::fs::read_to_string(root.join("reports").join("holdout-seal.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v["boundary_ms"].as_i64());
    let Some(b) = boundary else {
        return Vec::new();
    };
    let p = root.join("data/store/resampled/um/BTCUSDT/15m.csc");
    cs_data::store::read_file(&p)
        .ok()
        .and_then(|t| cs_data::store::table_to_candles(&t).ok())
        .map(|(c, _)| c.into_iter().filter(|c| c.open_time + TF <= b).collect())
        .unwrap_or_default()
}

/// Sample moment: the candle closing at this time (picked before looking at its context).
pub fn sample_time() -> i64 {
    time::ymd_hms_to_ms(2026, 3, 5, 14, 0, 0)
}

/// Shared state for a demo state, plus the label shown in the header.
pub fn shared_for(root: &Path, st: DemoState) -> (Arc<Mutex<Shared>>, String) {
    let all = sealed_candles(root);
    let mut sh = Shared {
        ever_connected: true,
        connected: true,
        warmup_done: true,
        drift: Some(Drift {
            offset_ms: 0,
            rtt_ms: 0,
            at_ms: 0,
        }),
        ..Shared::default()
    };
    let label = if all.is_empty() {
        "SAMPLE DATA (no local store)".to_string()
    } else {
        let end = all
            .iter()
            .position(|c| c.open_time + TF > sample_time())
            .unwrap_or(all.len());
        let start = end.saturating_sub(4000);
        let mut p = Pipeline::new("BTCUSDT", "15m", TF);
        p.push_closed(&all[start..end]);
        sh.snapshot = p.snapshot().copied();
        sh.stats = p.stats;
        sh.warmup_candles = end - start;
        sh.last_price = all[end - 1].close;
        sh.mark = Some(MarkMsg {
            event_time: 0,
            mark: all[end - 1].close,
            funding_rate: 0.0001,
            next_funding: 0,
        });
        format!(
            "SAMPLE DATA, {} UTC",
            time::fmt_minute(all[end - 1].open_time + TF)
        )
    };
    match st {
        DemoState::Live | DemoState::Replay => sh.last_msg = Some(Instant::now()),
        DemoState::Stale => {
            sh.last_msg = Instant::now().checked_sub(Duration::from_secs(41));
        }
        DemoState::Disconnected => {
            sh.connected = false;
            sh.last_msg = Instant::now().checked_sub(Duration::from_secs(9));
            sh.note = "read: connection reset; retry in 4 s".into();
            sh.reconnects = 2;
        }
        DemoState::NoData => {
            sh.snapshot = None;
            sh.connected = false;
            sh.ever_connected = false;
            sh.warmup_done = false;
            sh.warmup_candles = 0;
            sh.note =
                "warming up from REST klines (cannot reach the exchange: connect timed out)".into();
        }
    }
    let shared = Arc::new(Mutex::new(sh));
    if st == DemoState::Replay && !all.is_empty() {
        let s2 = Arc::clone(&shared);
        let end = all
            .iter()
            .position(|c| c.open_time + TF > sample_time())
            .unwrap_or(all.len());
        thread::spawn(move || replay(&all, end, &s2));
    }
    (shared, label)
}

/// Push one historical candle per second through the live path (frames, then the closed one).
fn replay(all: &[Candle], from: usize, shared: &Mutex<Shared>) {
    let mut p = Pipeline::new("BTCUSDT", "15m", TF);
    p.push_closed(&all[from.saturating_sub(4000)..from]);
    for c in &all[from..] {
        for (k, closed) in [(1, false), (2, false), (3, true)] {
            let f = kline_frame("BTCUSDT", "15m", TF, c, closed, c.open_time + k * 300_000);
            p.on_text(&f, &mut |_, _| Vec::new());
            thread::sleep(Duration::from_millis(333));
        }
        let mut s = shared
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        s.snapshot = p.snapshot().copied();
        s.stats = p.stats;
        s.last_price = p.last_price;
        s.last_msg = Some(Instant::now());
    }
}

/// Forced status for a demo state.
pub fn status_of(st: DemoState) -> FeedStatus {
    match st {
        DemoState::Live | DemoState::Replay => FeedStatus::Live,
        DemoState::Stale => FeedStatus::Stale,
        DemoState::Disconnected => FeedStatus::Disconnected,
        DemoState::NoData => FeedStatus::Starting,
    }
}
