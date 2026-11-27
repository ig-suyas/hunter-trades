//! The watcher thread. Every `title_poll_ms` it asks which window is in front (title and program
//! only, no pixels). Only when that window is a configured platform does it capture, at most
//! `fps` times per second, plus once at once when the ticker in the window title changes (so a
//! symbol switch is noticed within one poll + one read, without capturing faster). While screen
//! reading is off the thread blocks on its command channel and uses no CPU.

use std::path::PathBuf;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crate::config::VisionConfig;
use crate::ocr::Engines;
use crate::parse;
use crate::privacy;
use crate::read::{Reader, Reading};
use crate::source::{FrameSource, WindowInfo, match_platform};
use crate::stable::Stable;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WatchStatus {
    /// Screen reading switched off by the user.
    Off,
    /// Neither Windows OCR nor Tesseract is available.
    NoEngine,
    /// No platform window in front; nothing is captured.
    Waiting,
    /// A platform window is in front and being read.
    Reading,
}

/// Shared with the overlay.
// Independent on/off facts the panel shows; an enum would not make them clearer.
#[allow(clippy::struct_excessive_bools)]
#[derive(Debug, Clone)]
pub struct WatchState {
    /// Screen reading switched on: the one source of truth for the panel's toggle and text,
    /// set at once by [`Watcher::set_enabled`].
    pub enabled: bool,
    pub status: WatchStatus,
    /// Symbol and time frame of `reading` confirmed by two consecutive complete reads
    /// ([`crate::stable`]); the overlay switches the feed only then.
    pub confirmed: bool,
    /// The platform window left the front less than 3 s ago: status and reading are kept,
    /// nothing is captured.
    pub in_grace: bool,
    /// The last read as it came (before the stability rule), for diagnostics.
    pub raw: Option<Reading>,
    /// Debug flag only: the latest failures with the window that was in front (in memory,
    /// newest last, at most 12; never logged).
    pub failures: Vec<String>,
    /// Program name of the window in front (no title), and the title only when that window is
    /// the platform window (a chart tab title such as "BTCUSD 61,701 ▲ +2%").
    pub front_app: Option<String>,
    pub platform_title: Option<String>,
    /// Platform of the window in front ("`TradingView`").
    pub platform: Option<String>,
    pub reading: Option<Reading>,
    /// When `reading` was produced, and a counter that changes with every new reading.
    pub reading_at: Option<Instant>,
    pub seq: u64,
    /// Ticker from the window title (`TradingView` chart tabs carry it).
    pub title_ticker: Option<String>,
    pub engine: Option<&'static str>,
    pub frames: u64,
    pub ocr_calls: u64,
    pub polls: u64,
    pub note: String,
    /// The platform window was found below this program's own overlay (the overlay had the
    /// focus).
    pub behind_overlay: bool,
    /// Program name and title of the window in front, only with the debug flag
    /// (`CANDLESCOPE_VISION_DEBUG=1`); kept in memory for the panel, never logged.
    pub front_debug: Option<String>,
}

impl Default for WatchState {
    fn default() -> Self {
        WatchState {
            enabled: false,
            status: WatchStatus::Off,
            confirmed: false,
            in_grace: false,
            raw: None,
            failures: Vec::new(),
            front_app: None,
            platform_title: None,
            platform: None,
            reading: None,
            reading_at: None,
            seq: 0,
            title_ticker: None,
            engine: None,
            frames: 0,
            ocr_calls: 0,
            polls: 0,
            note: String::new(),
            behind_overlay: false,
            front_debug: None,
        }
    }
}

enum Cmd {
    Enable(bool),
    Stop,
}

pub struct Watcher {
    state: Arc<Mutex<WatchState>>,
    tx: Sender<Cmd>,
    join: Option<JoinHandle<()>>,
}

fn lock(s: &Mutex<WatchState>) -> std::sync::MutexGuard<'_, WatchState> {
    s.lock().unwrap_or_else(PoisonError::into_inner)
}

impl Watcher {
    /// Starts the thread. `make_source` runs on the thread (window handles are not `Send`).
    /// `log` receives one line of parsed text whenever page, symbol or time frame changes.
    ///
    /// # Panics
    /// Only if the OS refuses to create a thread.
    pub fn spawn<F>(
        cfg: VisionConfig,
        make_source: F,
        log: Option<PathBuf>,
        enabled: bool,
    ) -> Watcher
    where
        F: FnOnce() -> Box<dyn FrameSource> + Send + 'static,
    {
        let state = Arc::new(Mutex::new(WatchState {
            enabled,
            ..WatchState::default()
        }));
        let (tx, rx) = channel();
        let st = Arc::clone(&state);
        #[allow(clippy::expect_used)]
        let join = std::thread::Builder::new()
            .name("cs-vision".into())
            .spawn(move || run(&cfg, make_source(), &rx, &st, log.as_deref(), enabled))
            .expect("spawn the screen-reading thread");
        Watcher {
            state,
            tx,
            join: Some(join),
        }
    }

    pub fn set_enabled(&self, on: bool) {
        {
            // The shared state changes at once, so the panel's toggle and its text can never
            // disagree while the thread catches up.
            let mut s = lock(&self.state);
            s.enabled = on;
            if !on {
                s.status = WatchStatus::Off;
                s.reading = None;
                s.raw = None;
                s.confirmed = false;
            }
        }
        let _ = self.tx.send(Cmd::Enable(on));
    }

    pub fn state(&self) -> WatchState {
        lock(&self.state).clone()
    }
}

impl Drop for Watcher {
    fn drop(&mut self) {
        let _ = self.tx.send(Cmd::Stop);
        if let Some(j) = self.join.take() {
            let _ = j.join();
        }
    }
}

/// The parts of a reading whose change is worth a log line.
fn log_key(r: &Reading) -> String {
    format!("{:?} {:?} {:?}", r.page, r.symbol, r.tf)
}

/// Debug flag only: remembers a failure and the window that was in front (in memory).
fn note_failure(state: &Mutex<WatchState>, what: &str, front: Option<&WindowInfo>) {
    if !privacy::debug_on() {
        return;
    }
    let who = front.map_or_else(
        || "no window".to_string(),
        |w| format!("{} | {}", w.app, w.title),
    );
    let mut s = lock(state);
    let t = cs_core::time::fmt_minute(now_ms());
    s.failures.push(format!("{t} {what}; in front: {who}"));
    if s.failures.len() > 12 {
        s.failures.remove(0);
    }
}

// The watcher loop reads best as one function: poll, decide, capture, read, publish.
#[allow(clippy::too_many_lines)]
fn run(
    cfg: &VisionConfig,
    mut src: Box<dyn FrameSource>,
    rx: &Receiver<Cmd>,
    state: &Mutex<WatchState>,
    log: Option<&std::path::Path>,
    mut enabled: bool,
) {
    let poll = Duration::from_millis(cfg.title_poll_ms);
    let interval = cfg.frame_interval();
    let mut reader: Option<Reader> = None;
    let mut last_capture: Option<Instant> = None;
    let mut last_win: Option<(u32, u32, u32)> = None;
    let mut last_ticker: Option<String> = None;
    let mut last_key = String::new();
    let mut stable = Stable::default();
    // Stale-frame guard: Chrome stops repainting a window that another window fully covers.
    // If the title changed (TradingView puts the price there) but the pixels did not, the
    // frame is an old picture and is not read.
    let mut last_pixels: Option<(u64, String)> = None;
    loop {
        let cmd = if enabled {
            match rx.recv_timeout(poll) {
                Ok(c) => Some(c),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => return,
            }
        } else {
            {
                let mut s = lock(state);
                s.status = WatchStatus::Off;
                s.platform = None;
            }
            // Off: block until told otherwise (no polling, no CPU).
            match rx.recv() {
                Ok(c) => Some(c),
                Err(_) => return,
            }
        };
        match cmd {
            Some(Cmd::Stop) => return,
            Some(Cmd::Enable(on)) => {
                enabled = on;
                last_capture = None;
                last_win = None;
                stable.reset();
                if !on {
                    let mut s = lock(state);
                    s.reading = None;
                    s.reading_at = None;
                    s.seq += 1;
                }
                if !on {
                    continue;
                }
            }
            None => {}
        }
        let rd = reader.get_or_insert_with(|| Reader::new(cfg.clone(), Engines::detect()));
        if !rd.engines.available() {
            let mut s = lock(state);
            s.status = WatchStatus::NoEngine;
            s.note = rd.engines.last_error.clone().unwrap_or_default();
            continue;
        }
        lock(state).polls += 1;
        let front = src.foreground();
        {
            let mut s = lock(state);
            s.front_app = front.as_ref().map(|w| w.app.clone());
            s.platform_title = None;
        }
        if privacy::debug_on() {
            lock(state).front_debug = Some(front.as_ref().map_or_else(
                || "no window in front".to_string(),
                |w| {
                    format!(
                        "{}{} | {}",
                        w.app,
                        if w.behind_overlay {
                            " (below the overlay)"
                        } else {
                            ""
                        },
                        w.title
                    )
                },
            ));
        }
        let Some((info, platform, patterns)) = front.as_ref().and_then(|w| {
            match_platform(cfg, w).map(|p| (w, p.name.clone(), p.title_patterns.clone()))
        }) else {
            let now = Instant::now();
            if stable.in_grace(now) {
                // A popup, a recording toolbar or a click elsewhere for a moment: keep the
                // status and the last reading; nothing is captured.
                if !lock(state).in_grace {
                    note_failure(state, "platform window left the front", front.as_ref());
                }
                lock(state).in_grace = true;
                continue;
            }
            let mut s = lock(state);
            s.status = WatchStatus::Waiting;
            s.platform = None;
            s.behind_overlay = false;
            s.in_grace = false;
            s.confirmed = false;
            continue;
        };
        src.prefer(Some(info.id));
        let title = parse::parse_title(&info.title, &patterns);
        let ticker = title.as_ref().map(|t| t.ticker.clone());
        let key = (info.id, info.w, info.h);
        let due = last_capture.is_none_or(|t| t.elapsed() + poll / 4 >= interval)
            || ticker != last_ticker
            || Some(key) != last_win;
        {
            let mut s = lock(state);
            s.status = WatchStatus::Reading;
            s.in_grace = false;
            s.platform = Some(platform);
            s.platform_title = Some(info.title.clone());
            s.title_ticker.clone_from(&ticker);
            s.behind_overlay = info.behind_overlay;
        }
        if !due {
            continue;
        }
        if Some(key) != last_win {
            rd.reset();
        }
        last_ticker = ticker;
        last_win = Some(key);
        last_capture = Some(Instant::now());
        let Some(mut frame) = src.capture(info) else {
            lock(state).note = "capture failed".into();
            continue;
        };
        let h = frame.hash();
        if let Some((ph, pt)) = &last_pixels
            && *ph == h
            && *pt != info.title
        {
            frame.wipe();
            drop(frame);
            note_failure(state, "window not repainting (covered?)", front.as_ref());
            lock(state).note =
                "TradingView window not repainting (covered by another window)".into();
            continue;
        }
        last_pixels = Some((h, info.title.clone()));
        let reading = rd.read(&frame, title.as_ref());
        // The full frame is wiped and dropped right after the regions were read.
        frame.wipe();
        drop(frame);
        let k = log_key(&reading);
        if k != last_key {
            if let Some(p) = log {
                privacy::log_text(
                    p,
                    &format!(
                        "{} {}",
                        cs_core::time::fmt_minute(now_ms()),
                        reading.summary()
                    ),
                );
            }
            last_key = k;
        }
        if reading.page == crate::read::PageKind::Chart {
            if reading.symbol == crate::read::SymbolRead::Unknown {
                note_failure(state, "symbol not read", front.as_ref());
            }
            if reading.tf.is_none() {
                let why = reading
                    .tf_note
                    .clone()
                    .unwrap_or_else(|| "time frame not read".into());
                note_failure(state, &why, front.as_ref());
            }
        }
        let shown = stable.on_read(&reading, Instant::now());
        let mut s = lock(state);
        s.frames += 1;
        s.ocr_calls += u64::from(reading.ocr_calls);
        s.engine = reading.engine.map(crate::ocr::EngineKind::label);
        s.confirmed = stable.confirmed();
        s.raw = Some(reading);
        s.reading = Some(shown);
        s.reading_at = Some(Instant::now());
        s.seq += 1;
        s.note.clear();
    }
}

fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}
