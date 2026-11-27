//! Sample states for screenshots and checks, all from **real history**: stored BTCUSDT candles
//! **before the holdout seal** (the sealed period is cut off before anything runs) replayed as
//! websocket frames through the live path, the same board and the same journal the live panel
//! uses. The replay starts at a fixed moment chosen before looking at any outcome
//! (2026-03-02 00:00 UTC, the parity-test week) and stops at the first moment the wanted state
//! occurs. Labelled "REPLAY" in the panel; nothing is invented.

use std::path::Path;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use cs_backtest::cost::Funding;
use cs_core::{Candle, Timeframe, time};
use cs_live::feed::{Shared, default_warmup, publish_signals};
use cs_live::journal::{self, Journal, Limits};
use cs_live::kline::{MarkMsg, kline_frame};
use cs_live::pipeline::Pipeline;
use cs_live::signal::stats::StatsBook;
use cs_live::status::{Drift, FeedStatus};

use crate::board::{Arrival, Board};

/// Which state to show.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DemoState {
    /// Replay: no alert running.
    Wait,
    /// Replay: the first shown alert of each direction.
    Up,
    Down,
    /// Replay: the first candle where rules fired both ways.
    Disagree,
    /// 5m: no frozen rule was tested there.
    NoRule,
    /// Replay: the first shown alert whose rule has fewer than 100 validation trades.
    LowEvidence,
    /// Replay with outcomes resolved by the backtest rules until a daily limit holds.
    Guardrail,
    /// Feed states (context sample at 2026-03-05 14:00 UTC, 15m).
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
            "wait" => DemoState::Wait,
            "up" | "muted" | "details" => DemoState::Up,
            "down" => DemoState::Down,
            "disagree" => DemoState::Disagree,
            "no-rule" => DemoState::NoRule,
            "low-evidence" => DemoState::LowEvidence,
            "guardrail" => DemoState::Guardrail,
            "live" | "unsupported" | "mismatch" => DemoState::Live,
            "stale" => DemoState::Stale,
            "disconnected" => DemoState::Disconnected,
            "nodata" => DemoState::NoData,
            "replay" => DemoState::Replay,
            _ => return None,
        })
    }
}

/// What a demo hands to the app.
pub struct DemoOut {
    pub shared: Shared,
    pub label: String,
    /// The replay moment (the panel's clock), or `None` for the wall clock.
    pub now: Option<i64>,
    pub board: Board,
    pub journal: Journal,
    /// Replaces the past-results book (low-evidence demo only).
    pub stats: Option<StatsBook>,
    pub symbol: String,
    pub tf: Timeframe,
}

fn boundary(root: &Path) -> Option<i64> {
    std::fs::read_to_string(root.join("reports").join("holdout-seal.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v["boundary_ms"].as_i64())
}

/// Pre-seal candles from the store (empty if the store or seal is missing).
pub fn sealed_candles(root: &Path, sym: &str, tf: Timeframe) -> Vec<Candle> {
    let Some(b) = boundary(root) else {
        return Vec::new();
    };
    let p = root.join(format!("data/store/resampled/um/{sym}/{}.csc", tf.label()));
    cs_data::store::read_file(&p)
        .ok()
        .and_then(|t| cs_data::store::table_to_candles(&t).ok())
        .map(|(c, _)| {
            c.into_iter()
                .filter(|c| c.open_time + tf.ms() <= b)
                .collect()
        })
        .unwrap_or_default()
}

/// Pre-seal 1-minute candles and funding (cut before use), for resolving outcomes.
fn sealed_m1_funding(root: &Path, sym: &str) -> (Vec<Candle>, Vec<Funding>) {
    let Some(b) = boundary(root) else {
        return (Vec::new(), Vec::new());
    };
    let dd = cs_data::DataDir::new(root.join("data"));
    let m1: Vec<Candle> = cs_data::clean::klines(&dd, cs_data::Market::Um, sym)
        .map(|k| k.candles)
        .unwrap_or_default()
        .into_iter()
        .filter(|c| c.open_time < b)
        .collect();
    let mut funding = Vec::new();
    if let Ok((ft, _, _)) = cs_data::clean::series(&dd.funding(sym), None)
        && ft.nrows() > 0
        && let (Ok(ts), Ok(rs)) = (ft.i64("time"), ft.f64("rate"))
    {
        funding = ts
            .iter()
            .zip(rs)
            .filter(|(t, _)| **t < b)
            .map(|(&time, &rate)| Funding { time, rate })
            .collect();
    }
    (m1, funding)
}

/// Replay start, fixed in advance: the parity-test week.
pub const REPLAY_START: i64 = time::ymd_hms_to_ms(2026, 3, 2, 0, 0, 0);

fn base_shared() -> Shared {
    Shared {
        ever_connected: true,
        connected: true,
        warmup_done: true,
        drift: Some(Drift {
            offset_ms: 0,
            rtt_ms: 0,
            at_ms: 0,
        }),
        last_msg: Some(Instant::now()),
        ..Shared::default()
    }
}

fn shared_of(p: &Pipeline, warm: usize) -> Shared {
    let mut sh = base_shared();
    sh.snapshot = p.snapshot().copied();
    sh.stats = p.stats;
    sh.warmup_candles = warm;
    sh.last_price = p.last_price;
    sh.mark = Some(MarkMsg {
        event_time: 0,
        mark: p.last_price,
        funding_rate: 0.0001,
        next_funding: 0,
    });
    publish_signals(&mut sh, p);
    sh
}

fn label(sym: &str, tf: Timeframe, t: i64, extra: &str) -> String {
    format!(
        "REPLAY {sym} {} \u{00B7} {} UTC{extra}",
        tf.label(),
        time::fmt_minute(t)
    )
}

/// Replay BTCUSDT 1h from [`REPLAY_START`] until `stop` says so (checked right after each
/// candle's first in-progress frame, the moment alerts are decided).
fn replay_until(
    root: &Path,
    book: Option<&StatsBook>,
    resolve: bool,
    stop: &dyn Fn(&Board, &Shared, &Journal, i64) -> bool,
) -> Option<(Shared, Board, Journal, i64)> {
    let tf = Timeframe::H1;
    let tfm = tf.ms();
    let all = sealed_candles(root, "BTCUSDT", tf);
    let s = all.iter().position(|c| c.open_time >= REPLAY_START)?;
    let warm = default_warmup(tf);
    let start = s.checked_sub(warm)?;
    let mut p = Pipeline::new("BTCUSDT", tf.label(), tfm);
    p.push_closed(&all[start..s]);
    let (m1, funding) = if resolve {
        sealed_m1_funding(root, "BTCUSDT")
    } else {
        (Vec::new(), Vec::new())
    };
    let mut board = Board::default();
    let mut j = Journal::in_memory();
    let limits = Limits::default();
    for k in s..all.len() {
        let c = all[k];
        let mut o = c;
        o.high = c.open;
        o.low = c.open;
        o.close = c.open;
        let now = c.open_time + 1_000;
        p.on_text(
            &kline_frame("BTCUSDT", tf.label(), tfm, &o, false, now),
            &mut |_, _| Vec::new(),
        );
        let sh = shared_of(&p, warm);
        board.update(
            &sh,
            &Arrival {
                now,
                stats: book,
                limits,
                account: 10_000.0,
                risk_pct: 1.0,
                hidden: String::new(),
            },
            &mut j,
        );
        if stop(&board, &sh, &j, now) {
            return Some((sh, board, j, now));
        }
        p.on_text(
            &kline_frame("BTCUSDT", tf.label(), tfm, &c, true, c.open_time + tfm),
            &mut |_, _| Vec::new(),
        );
        if resolve {
            // Outcomes of shown alerts, by the backtest rules on the candles closed so far.
            let pending: Vec<_> = j
                .pending()
                .into_iter()
                .filter(|r| r.decision == journal::decision::SHOWN)
                .cloned()
                .collect();
            let mut out = Vec::new();
            for r in pending {
                let Some(w) = all.iter().position(|x| x.open_time == r.window_start) else {
                    continue;
                };
                if let Ok(Some(o)) = journal::resolve(&all[w..=k], &m1, &funding, &r) {
                    out.push((r.id.clone(), Ok(o)));
                }
            }
            let _ = j.set_outcomes(&out);
        }
    }
    None
}

/// Builds a demo state.
pub fn build(root: &Path, st: DemoState, book: Option<&StatsBook>) -> DemoOut {
    let empty = |label: String| DemoOut {
        shared: base_shared(),
        label,
        now: None,
        board: Board::default(),
        journal: Journal::in_memory(),
        stats: None,
        symbol: "BTCUSDT".into(),
        tf: Timeframe::H1,
    };
    let replayed =
        |r: Option<(Shared, Board, Journal, i64)>, extra: &str, stats: Option<StatsBook>| match r {
            Some((sh, board, journal, now)) => DemoOut {
                shared: sh,
                label: label("BTCUSDT", Timeframe::H1, now - 1_000, extra),
                now: Some(now),
                board,
                journal,
                stats,
                symbol: "BTCUSDT".into(),
                tf: Timeframe::H1,
            },
            None => empty("REPLAY: state not found in the stored history".into()),
        };
    match st {
        DemoState::Wait => {
            let tf = Timeframe::H1;
            let all = sealed_candles(root, "BTCUSDT", tf);
            let Some(s) = all.iter().position(|c| c.open_time >= REPLAY_START) else {
                return empty("REPLAY: no local store".into());
            };
            let warm = default_warmup(tf);
            let mut p = Pipeline::new("BTCUSDT", tf.label(), tf.ms());
            p.push_closed(&all[s.saturating_sub(warm)..s]);
            DemoOut {
                shared: shared_of(&p, warm),
                label: label("BTCUSDT", tf, REPLAY_START, ""),
                now: Some(REPLAY_START),
                ..empty(String::new())
            }
        }
        DemoState::Up | DemoState::Down => {
            let want: i8 = if st == DemoState::Up { 1 } else { -1 };
            let r = replay_until(root, book, false, &|b, sh, _, _| {
                b.active(sh).is_some_and(|(s, _)| s.dir == want)
            });
            replayed(r, "", None)
        }
        DemoState::Disagree => {
            let r = replay_until(root, book, false, &|b, sh, _, now| {
                b.disagree(now) && b.active(sh).is_none()
            });
            replayed(r, "", None)
        }
        DemoState::LowEvidence => {
            // The first shown alert whose rule has fewer than 100 validation trades.
            let r = replay_until(root, book, false, &|b, sh, _, _| {
                b.active(sh).is_some_and(|(s, _)| {
                    book.and_then(|k| k.get(s.rule, &s.symbol))
                        .is_some_and(|st| st.validation.low_evidence())
                })
            });
            replayed(r, "", None)
        }
        DemoState::Guardrail => {
            let limits = Limits::default();
            let r = replay_until(root, book, true, &|b, sh, j, now| {
                b.active(sh).is_none()
                    && journal::hold(&limits, &journal::today(&j.rows, now)) != journal::Hold::Open
            });
            replayed(r, "", None)
        }
        DemoState::NoRule => {
            let tf = Timeframe::M5;
            let all = sealed_candles(root, "BTCUSDT", tf);
            let Some(s) = all.iter().position(|c| c.open_time >= REPLAY_START) else {
                return empty("REPLAY: no local store".into());
            };
            let warm = default_warmup(tf);
            let mut p = Pipeline::new("BTCUSDT", tf.label(), tf.ms());
            p.push_closed(&all[s.saturating_sub(warm)..s]);
            DemoOut {
                shared: shared_of(&p, warm),
                label: label("BTCUSDT", tf, REPLAY_START, ""),
                now: Some(REPLAY_START),
                tf,
                ..empty(String::new())
            }
        }
        DemoState::Live
        | DemoState::Stale
        | DemoState::Disconnected
        | DemoState::NoData
        | DemoState::Replay => feed_state(root, st),
    }
}

/// Sample moment of the feed states: the candle closing at this time.
pub fn sample_time() -> i64 {
    time::ymd_hms_to_ms(2026, 3, 5, 14, 0, 0)
}

const TF15: i64 = 900_000;

/// The feed-state demos (15m context sample, feed status set by hand).
fn feed_state(root: &Path, st: DemoState) -> DemoOut {
    let tf = Timeframe::M15;
    let all = sealed_candles(root, "BTCUSDT", tf);
    let mut sh = base_shared();
    let label = if all.is_empty() {
        "SAMPLE DATA (no local store)".to_string()
    } else {
        let end = all
            .iter()
            .position(|c| c.open_time + TF15 > sample_time())
            .unwrap_or(all.len());
        let start = end.saturating_sub(default_warmup(tf));
        let mut p = Pipeline::new("BTCUSDT", "15m", TF15);
        p.push_closed(&all[start..end]);
        sh = shared_of(&p, end - start);
        sh.last_price = all[end - 1].close;
        format!(
            "SAMPLE DATA, {} UTC",
            time::fmt_minute(all[end - 1].open_time + TF15)
        )
    };
    match st {
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
        }
        _ => {}
    }
    DemoOut {
        shared: sh,
        label,
        now: None,
        board: Board::default(),
        journal: Journal::in_memory(),
        stats: None,
        symbol: "BTCUSDT".into(),
        tf,
    }
}

/// Starts the CPU-check replay: one historical 15m candle per second through the live path.
pub fn start_replay(root: &Path, shared: &Arc<Mutex<Shared>>) {
    let all = sealed_candles(root, "BTCUSDT", Timeframe::M15);
    if all.is_empty() {
        return;
    }
    let s2 = Arc::clone(shared);
    let end = all
        .iter()
        .position(|c| c.open_time + TF15 > sample_time())
        .unwrap_or(all.len());
    thread::spawn(move || replay(&all, end, &s2));
}

/// Push one historical candle per second through the live path (frames, then the closed one).
fn replay(all: &[Candle], from: usize, shared: &Mutex<Shared>) {
    let mut p = Pipeline::new("BTCUSDT", "15m", TF15);
    p.push_closed(&all[from.saturating_sub(4000)..from]);
    for c in &all[from..] {
        for (k, closed) in [(1, false), (2, false), (3, true)] {
            let f = kline_frame("BTCUSDT", "15m", TF15, c, closed, c.open_time + k * 300_000);
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
        publish_signals(&mut s, &p);
    }
}

/// Forced status for a demo state.
pub fn status_of(st: DemoState) -> FeedStatus {
    match st {
        DemoState::Stale => FeedStatus::Stale,
        DemoState::Disconnected => FeedStatus::Disconnected,
        DemoState::NoData => FeedStatus::Starting,
        _ => FeedStatus::Live,
    }
}
