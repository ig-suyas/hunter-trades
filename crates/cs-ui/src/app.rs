//! The overlay window: borderless, always on top, draggable and resizable, adjustable
//! background opacity, click-through, global hotkeys and the MUTE kill switch. It only
//! displays and logs: there is no order code anywhere in the workspace and no keys.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cs_core::Timeframe;
use cs_live::feed::{FeedConfig, FeedHandle, Shared, spawn};
use cs_live::journal::{self, Journal, Outcome};
use cs_live::signal::rules;
use cs_live::signal::stats::StatsBook;
use cs_live::status::FeedStatus;
use eframe::egui::{
    self, FontData, FontDefinitions, FontFamily, FontId, TextStyle, Vec2, ViewportCommand, vec2,
};
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

use cs_vision::handshake::{Handshake, MismatchGate, check_price};
use cs_vision::read::PageKind;
use cs_vision::source::{FrameSource, XcapSource};
use cs_vision::{VisionConfig, WatchState, Watcher};

use crate::board::{Arrival, Board};
use crate::copy;
use crate::demo::{self, DemoState};
use crate::palette as pal;
use crate::recorder::{self, RecorderInfo};
use crate::screen;
use crate::settings::{Settings, parse_number};
use crate::view::{self, Action, Controls, MainInput, View};

/// Main view (Details closed) and Details open.
pub const MAIN: Vec2 = Vec2::new(390.0, 410.0);
pub const DETAILS: Vec2 = Vec2::new(460.0, 820.0);

/// Command line options.
#[derive(Debug, Clone)]
pub struct Opts {
    pub symbol: String,
    pub tf: Timeframe,
    /// Project root (holds `data/` and `reports/`).
    pub root: PathBuf,
    pub demo: Option<(DemoState, String)>,
    pub details: bool,
    pub muted: bool,
    pub click_through: bool,
    pub screenshot: Option<PathBuf>,
    pub layout_check: bool,
    pub exit_after: Option<Duration>,
    /// Window size override (screenshots).
    pub size: Option<Vec2>,
    /// Window position restored from the settings file.
    pub pos: Option<[f32; 2]>,
    pub settings: Settings,
    /// Screen reading on at start.
    pub screen: bool,
    /// Read and write `data/overlay-settings.json` and `data/signals/` (off for demos,
    /// screenshots, checks).
    pub use_settings: bool,
    /// Demo of the SOURCE MISMATCH state.
    pub demo_mismatch: bool,
}

/// Outcomes from the background resolver: (alert id, outcome or problem).
type Resolved = Vec<(String, Result<Outcome, String>)>;

enum Source {
    Live(FeedHandle),
    Demo {
        shared: Arc<Mutex<Shared>>,
        state: DemoState,
        label: String,
        /// The replay moment (static demos); `None` = wall clock.
        now: Option<i64>,
    },
    Unsupported,
}

struct Keys {
    _mgr: GlobalHotKeyManager,
    rx: Receiver<GlobalHotKeyEvent>,
    hide: u32,
    mute: u32,
    click: u32,
    screen: u32,
    /// Every hotkey registered (false: another program owns at least one).
    all_ok: bool,
}

pub struct OverlayApp {
    opts: Opts,
    source: Source,
    ctl: Controls,
    details: bool,
    muted: bool,
    click_through: bool,
    visible: bool,
    keys: Option<Keys>,
    recorder: RecorderInfo,
    recorder_at: Option<Instant>,
    local_offset_s: i64,
    frames: u64,
    started: Instant,
    shot_sent: bool,
    shot_done: Arc<AtomicBool>,
    layout: view::Layout_,
    layout_printed: bool,
    exit_logged: bool,
    /// Screen reading (Phase 7): the watcher thread, or why there is none.
    vision: Option<Watcher>,
    vision_note: Option<String>,
    screen_on: bool,
    /// The user picked a symbol by hand: the screen is ignored until "Follow screen".
    manual: bool,
    screen_seq: u64,
    screen_state: WatchState,
    screen_block: Option<String>,
    gate: MismatchGate,
    mismatch: Option<(f64, f64, f64)>,
    tolerance_pct: f64,
    saved: Settings,
    saved_at: Instant,
    /// Signal view.
    stats: Option<StatsBook>,
    board: Board,
    journal: Journal,
    paper_since: i64,
    resolver: Option<Receiver<Resolved>>,
    resolve_at: Option<Instant>,
    import_msg: Vec<String>,
    hero_key: String,
    hero_since: Instant,
}

fn load_fonts(ctx: &egui::Context) -> bool {
    let mut defs = FontDefinitions::default();
    let dir = std::env::var_os("WINDIR")
        .map_or_else(|| PathBuf::from("C:\\Windows"), PathBuf::from)
        .join("Fonts");
    let mut ok = false;
    for (name, file) in [("segoe", "segoeui.ttf"), ("segoe_sym", "seguisym.ttf")] {
        if let Ok(b) = std::fs::read(dir.join(file)) {
            defs.font_data
                .insert(name.to_string(), Arc::new(FontData::from_owned(b)));
            ok = true;
        }
    }
    if ok {
        let prop = defs.families.entry(FontFamily::Proportional).or_default();
        let mut front = Vec::new();
        for n in ["segoe", "segoe_sym"] {
            if defs.font_data.contains_key(n) {
                front.push(n.to_string());
            }
        }
        let rest: Vec<String> = std::mem::take(prop);
        prop.extend(front.clone());
        prop.extend(rest);
        if defs.font_data.contains_key("segoe_sym") {
            defs.families
                .entry(FontFamily::Monospace)
                .or_default()
                .push("segoe_sym".into());
        }
    }
    ctx.set_fonts(defs);
    ok
}

fn style(ctx: &egui::Context) {
    ctx.all_styles_mut(|s| {
        s.text_styles = [
            (TextStyle::Small, FontId::proportional(pal::FONT_MIN)),
            (TextStyle::Body, FontId::proportional(pal::FONT_BODY)),
            (TextStyle::Button, FontId::proportional(pal::FONT_BODY)),
            (TextStyle::Heading, FontId::proportional(pal::FONT_HEAD)),
            (TextStyle::Monospace, FontId::monospace(pal::FONT_MIN)),
        ]
        .into();
        s.spacing.item_spacing = vec2(6.0, 3.0);
        s.spacing.button_padding = vec2(7.0, 3.0);
        s.visuals = egui::Visuals::dark();
        s.visuals.override_text_color = Some(pal::c(pal::TEXT));
        s.visuals.panel_fill = egui::Color32::TRANSPARENT;
        s.visuals.window_fill = pal::c(pal::BG_RAISED);
        s.visuals.extreme_bg_color = pal::c(pal::BG_RAISED);
        s.visuals.widgets.inactive.weak_bg_fill = pal::c(pal::BG_RAISED);
        s.visuals.widgets.inactive.bg_stroke = egui::Stroke::new(1.0, pal::c(pal::BORDER));
    });
}

fn local_offset() -> i64 {
    time::UtcOffset::current_local_offset().map_or(0, |o| i64::from(o.whole_seconds()))
}

fn register_keys(ctx: &egui::Context) -> Option<Keys> {
    let mgr = GlobalHotKeyManager::new().ok()?;
    let m = Some(Modifiers::CONTROL | Modifiers::ALT);
    let (h, mu, t, sc) = (
        HotKey::new(m, Code::KeyH),
        HotKey::new(m, Code::KeyM),
        HotKey::new(m, Code::KeyT),
        HotKey::new(m, Code::KeyS),
    );
    // One at a time: a key another program owns must not cost the others.
    let ok: Vec<bool> = [h, mu, t, sc]
        .iter()
        .map(|k| mgr.register(*k).is_ok())
        .collect();
    if !ok.iter().any(|x| *x) {
        return None;
    }
    let (tx, rx) = channel();
    let c2 = ctx.clone();
    // The handler runs on the window thread's message loop, also while the window is hidden;
    // the repaint request makes eframe call `logic`, which handles the key.
    GlobalHotKeyEvent::set_event_handler(Some(move |e: GlobalHotKeyEvent| {
        let _ = tx.send(e);
        c2.request_repaint();
    }));
    Some(Keys {
        _mgr: mgr,
        rx,
        hide: h.id(),
        mute: mu.id(),
        click: t.id(),
        screen: sc.id(),
        all_ok: ok.iter().all(|x| *x),
    })
}

/// Starts the screen watcher (never in demo, screenshot or layout-check runs: those must not
/// capture anything).
fn start_vision(opts: &Opts) -> (Option<Watcher>, Option<String>, f64) {
    let tol = 0.3;
    if opts.demo.is_some() || opts.screenshot.is_some() || opts.layout_check {
        return (None, Some("not used in demo runs".into()), tol);
    }
    match VisionConfig::load(&opts.root) {
        Ok(cfg) => {
            let tol = cfg.price_tolerance_pct;
            let log = opts.root.join("data").join("logs").join("vision.log");
            let w = Watcher::spawn(
                cfg,
                || Box::new(XcapSource::new()) as Box<dyn FrameSource>,
                Some(log),
                opts.screen,
            );
            (Some(w), None, tol)
        }
        Err(e) => (None, Some(e), tol),
    }
}

impl OverlayApp {
    pub fn new(cc: &eframe::CreationContext<'_>, opts: Opts) -> OverlayApp {
        let fonts = load_fonts(&cc.egui_ctx);
        if opts.layout_check {
            eprintln!("fonts: Segoe UI loaded = {fonts}");
        }
        style(&cc.egui_ctx);
        let keys = register_keys(&cc.egui_ctx);
        let (vision, vision_note, tolerance_pct) = start_vision(&opts);
        let screen_on = vision.is_some() && opts.screen;
        let mismatch = opts.demo_mismatch.then_some((
            1_704.15,
            91_000.0,
            (1_704.15 - 91_000.0) / 91_000.0 * 100.0,
        ));
        if opts.click_through {
            cc.egui_ctx
                .send_viewport_cmd(ViewportCommand::MousePassthrough(true));
        }
        let st = &opts.settings;
        let mut stats = StatsBook::load(&opts.root).ok();
        let mut journal = if opts.use_settings {
            Journal::load(&journal::dir(&opts.root))
        } else {
            Journal::in_memory()
        };
        let mut board = Board::default();
        let mut symbol = opts.symbol.clone();
        let mut tf = opts.tf;
        let source = match &opts.demo {
            Some((ds, _)) if cs_live::is_supported(&symbol) => {
                let d = demo::build(&opts.root, *ds, stats.as_ref());
                symbol.clone_from(&d.symbol);
                tf = d.tf;
                if d.stats.is_some() {
                    stats = d.stats;
                }
                journal = d.journal;
                board = d.board;
                let shared = Arc::new(Mutex::new(d.shared));
                if *ds == DemoState::Replay {
                    demo::start_replay(&opts.root, &shared);
                }
                Source::Demo {
                    shared,
                    state: *ds,
                    label: d.label,
                    now: d.now,
                }
            }
            _ => make_source(&symbol, tf),
        };
        OverlayApp {
            ctl: Controls {
                symbol,
                tf,
                opacity: st.opacity.clamp(pal::MIN_OPACITY, 1.0),
                account: st.account.clone(),
                risk_pct: st.risk_pct.clone(),
                limits: st.limits,
            },
            details: opts.details,
            muted: opts.muted,
            click_through: opts.click_through,
            visible: true,
            keys,
            recorder: RecorderInfo::default(),
            recorder_at: None,
            local_offset_s: local_offset(),
            frames: 0,
            started: Instant::now(),
            shot_sent: false,
            shot_done: Arc::new(AtomicBool::new(false)),
            layout: view::Layout_::default(),
            layout_printed: false,
            exit_logged: false,
            vision,
            vision_note,
            screen_on,
            manual: false,
            screen_seq: u64::MAX,
            screen_state: WatchState::default(),
            screen_block: None,
            gate: MismatchGate::new(2),
            mismatch,
            tolerance_pct,
            saved: opts.settings.clone(),
            saved_at: Instant::now(),
            stats,
            board,
            journal,
            paper_since: st.paper_since,
            resolver: None,
            resolve_at: None,
            import_msg: Vec::new(),
            hero_key: String::new(),
            hero_since: Instant::now(),
            source,
            opts,
        }
    }

    fn now(&self) -> i64 {
        match &self.source {
            Source::Demo { now: Some(t), .. } => *t,
            _ => cs_live::now_ms(),
        }
    }

    /// Switches the feed (dropping the old handle stops its thread).
    fn switch_feed(&mut self, sym: &str, tf: Timeframe) {
        self.ctl.symbol = sym.to_string();
        self.ctl.tf = tf;
        self.source = make_source(sym, tf);
        self.board.reset();
        self.gate.reset();
        if !self.opts.demo_mismatch {
            self.mismatch = None;
        }
    }

    /// Follows the screen: a new reading switches the feed, blocks an unsupported symbol or
    /// time frame, or runs the price handshake.
    fn process_screen(&mut self) {
        let Some(w) = &self.vision else {
            return;
        };
        self.screen_state = w.state();
        // The watcher's flag is the one source of truth for on/off.
        self.screen_on = self.screen_state.enabled;
        if !self.screen_on || self.manual {
            self.screen_block = None;
            self.mismatch = None;
            self.gate.reset();
            return;
        }
        if self.screen_state.seq == self.screen_seq {
            return;
        }
        self.screen_seq = self.screen_state.seq;
        let Some(r) = self.screen_state.reading.clone() else {
            self.screen_block = None;
            return;
        };
        // Switch, block or check prices only on a reading confirmed by two consistent reads.
        if !self.screen_state.confirmed {
            return;
        }
        let d = screen::decide(&r, &self.ctl.symbol, self.ctl.tf);
        self.screen_block = d.block;
        if let Some((sym, tf)) = d.switch {
            self.switch_feed(&sym, tf);
            return;
        }
        if r.page == PageKind::Chart && r.symbol.exchange() == Some(self.ctl.symbol.as_str()) {
            let feed = self.read().0.last_price;
            let h = check_price(r.price.map(|p| p.0), feed, self.tolerance_pct);
            let on = self.gate.update(h);
            if let Handshake::Mismatch {
                screen,
                feed,
                diff_pct,
            } = h
            {
                if on {
                    self.mismatch = Some((screen, feed, diff_pct));
                }
            } else if !on {
                self.mismatch = None;
            }
        }
    }

    fn set_screen(&mut self, on: bool) {
        if let Some(w) = &self.vision {
            self.screen_on = on;
            self.manual = false;
            self.screen_seq = u64::MAX;
            w.set_enabled(on);
        }
    }

    /// Current settings (window geometry from the viewport).
    fn current_settings(&self, ctx: &egui::Context) -> Settings {
        let (pos, size) = ctx.input(|i| {
            let vp = i.viewport();
            (
                vp.outer_rect.map(|r| [r.min.x, r.min.y]),
                vp.inner_rect.map(|r| [r.width(), r.height()]),
            )
        });
        let limits = self.ctl.limits;
        Settings {
            pos: pos.or(self.saved.pos),
            // The main-view size only (Details open uses its own default size).
            size: if self.details {
                self.saved.size
            } else {
                size.or(self.saved.size)
            },
            opacity: self.ctl.opacity,
            details_open: self.details,
            account: if parse_number(&self.ctl.account).is_some_and(|x| x > 0.0) {
                self.ctl.account.clone()
            } else {
                self.saved.account.clone()
            },
            risk_pct: if parse_number(&self.ctl.risk_pct).is_some_and(|x| x > 0.0 && x <= 100.0) {
                self.ctl.risk_pct.clone()
            } else {
                self.saved.risk_pct.clone()
            },
            limits,
            paper_since: self.paper_since,
            symbol: if cs_live::is_supported(&self.ctl.symbol) {
                self.ctl.symbol.clone()
            } else {
                self.saved.symbol.clone()
            },
            tf: self.ctl.tf,
            screen_reading: if self.vision.is_some() {
                self.screen_on
            } else {
                self.opts.screen
            },
        }
    }

    fn save_settings(&mut self, ctx: &egui::Context) {
        if !self.opts.use_settings || self.saved_at.elapsed() < Duration::from_secs(2) {
            return;
        }
        self.saved_at = Instant::now();
        let cur = self.current_settings(ctx);
        if !cur.same(&self.saved) {
            if let Err(e) = cur.save(&self.opts.root) {
                eprintln!("settings not saved: {e}");
            }
            self.saved = cur;
        }
    }

    fn read(&self) -> (Shared, Option<FeedStatus>, Option<String>) {
        match &self.source {
            Source::Live(h) => (h.read(), Some(h.status()), None),
            Source::Demo {
                shared,
                state,
                label,
                ..
            } => {
                let s = shared
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .clone();
                (s, Some(demo::status_of(*state)), Some(label.clone()))
            }
            Source::Unsupported => (Shared::default(), None, None),
        }
    }

    fn import_fills(&mut self, text: Option<String>) {
        let d = journal::dir(&self.opts.root);
        let text = text.or_else(|| std::fs::read_to_string(journal::fills_path(&d)).ok());
        let Some(text) = text else {
            self.import_msg = vec![copy::FILLS_HOW.to_string()];
            return;
        };
        let imp = journal::import_fills(&self.journal, &text);
        if self.journal.persist
            && let Err(e) = journal::write_trades(&d, &imp)
        {
            eprintln!("demo trades not written: {e}");
        }
        self.import_msg = copy::import_lines(&imp);
    }

    fn apply(&mut self, ctx: &egui::Context, a: Action) {
        match a {
            Action::ToggleMute => self.muted = !self.muted,
            Action::ToggleDetails => {
                self.details = !self.details;
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(if self.details {
                    DETAILS
                } else {
                    self.saved.size.map_or(MAIN, |[w, h]| egui::vec2(w, h))
                }));
            }
            Action::SetFeed(sym, tf) => {
                // A pick by hand overrides the screen until "Follow screen".
                if self.screen_on {
                    self.manual = true;
                    self.screen_block = None;
                }
                self.switch_feed(&sym, tf);
            }
            Action::ToggleScreen => self.set_screen(!self.screen_on),
            Action::FollowScreen => {
                self.manual = false;
                self.screen_seq = u64::MAX;
            }
            Action::SetClickThrough(on) => {
                self.click_through = on;
                ctx.send_viewport_cmd(ViewportCommand::MousePassthrough(on));
            }
            Action::StartDrag => ctx.send_viewport_cmd(ViewportCommand::StartDrag),
            Action::BeginResize => ctx.send_viewport_cmd(ViewportCommand::BeginResize(
                egui::viewport::ResizeDirection::SouthEast,
            )),
            Action::ResetPaper => self.paper_since = self.now(),
            Action::ImportFills => self.import_fills(None),
        }
    }

    /// Resolve the journal's pending alerts over the exchange's public endpoints, in the
    /// background (live runs only; at most every 60 s while something is pending).
    fn resolve(&mut self) {
        if let Some(rx) = &self.resolver
            && let Ok(out) = rx.try_recv()
        {
            self.resolver = None;
            if let Err(e) = self.journal.set_outcomes(&out) {
                eprintln!("journal not written: {e}");
            }
        }
        if !self.opts.use_settings
            || self.resolver.is_some()
            || self
                .resolve_at
                .is_some_and(|t| t.elapsed() < Duration::from_secs(60))
        {
            return;
        }
        self.resolve_at = Some(Instant::now());
        let now = cs_live::now_ms();
        let rows: Vec<_> = self
            .journal
            .pending()
            .into_iter()
            .filter(|r| now >= r.entry_time + r.tf.ms())
            .cloned()
            .collect();
        if rows.is_empty() {
            return;
        }
        let (tx, rx) = channel();
        let ok = std::thread::Builder::new()
            .name("cs-ui resolver".into())
            .spawn(move || {
                let _ = tx.send(journal::resolve_pending_rest(&rows, now));
            })
            .is_ok();
        if ok {
            self.resolver = Some(rx);
        }
    }

    fn screenshot(&mut self, ctx: &egui::Context) {
        let Some(path) = self.opts.screenshot.clone() else {
            return;
        };
        if self.shot_done.load(Ordering::Relaxed) {
            ctx.send_viewport_cmd(ViewportCommand::Close);
            return;
        }
        if !self.shot_sent && self.frames >= 6 {
            self.shot_sent = true;
            let done = Arc::clone(&self.shot_done);
            let c2 = ctx.clone();
            ctx.send_viewport_cmd(ViewportCommand::Screenshot(egui::ScreenshotCallback::new(
                move |img| {
                    // Composite the semi-transparent panel over a neutral dark backdrop (a chart).
                    let back = [40u16, 44, 52];
                    let mut rgb = Vec::with_capacity(img.pixels.len() * 3);
                    for p in &img.pixels {
                        let a = u16::from(p.a());
                        for (k, v) in [p.r(), p.g(), p.b()].into_iter().enumerate() {
                            let x = u16::from(v) + back[k] * (255 - a) / 255;
                            rgb.push(u8::try_from(x.min(255)).unwrap_or(255));
                        }
                    }
                    let w = u32::try_from(img.size[0]).unwrap_or(0);
                    let h = u32::try_from(img.size[1]).unwrap_or(0);
                    match crate::png::write_rgb(&path, w, h, &rgb) {
                        Ok(()) => eprintln!("screenshot: {} ({w} x {h} px)", path.display()),
                        Err(e) => eprintln!("screenshot failed: {e}"),
                    }
                    done.store(true, Ordering::Relaxed);
                    c2.request_repaint();
                },
            )));
        }
        ctx.request_repaint();
    }
}

fn make_source(symbol: &str, tf: Timeframe) -> Source {
    if !cs_live::is_supported(symbol) {
        return Source::Unsupported;
    }
    Source::Live(spawn(FeedConfig::new(symbol, tf)))
}

impl eframe::App for OverlayApp {
    fn clear_color(&self, _v: &egui::Visuals) -> [f32; 4] {
        [0.0, 0.0, 0.0, 0.0]
    }

    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let mut acts = Vec::new();
        let mut pressed = false;
        if let Some(k) = &self.keys {
            while let Ok(e) = k.rx.try_recv() {
                if e.state != HotKeyState::Pressed {
                    continue;
                }
                pressed = true;
                if e.id == k.hide {
                    self.visible = !self.visible;
                    ctx.send_viewport_cmd(ViewportCommand::Visible(self.visible));
                } else if e.id == k.mute {
                    acts.push(Action::ToggleMute);
                } else if e.id == k.click {
                    acts.push(Action::SetClickThrough(!self.click_through));
                } else if e.id == k.screen {
                    acts.push(Action::ToggleScreen);
                }
            }
        }
        for a in acts {
            self.apply(ctx, a);
        }
        if pressed && self.opts.layout_check {
            let (sh, st, _) = self.read();
            eprintln!(
                "hotkey: muted {}, click-through {}, visible {}, feed {:?}, frames so far {}, candles applied {}",
                self.muted, self.click_through, self.visible, st, sh.stats.frames, sh.stats.applied
            );
        }
        // A CSV dropped on the panel is imported as demo fills.
        let dropped: Vec<PathBuf> = ctx.input(|i| {
            i.raw
                .dropped_files
                .iter()
                .map(|f| f.path().to_path_buf())
                .collect()
        });
        for p in dropped {
            if p.extension().and_then(|x| x.to_str()) == Some("csv") {
                self.import_fills(std::fs::read_to_string(&p).ok());
            }
        }
        if self
            .recorder_at
            .is_none_or(|t| t.elapsed() > Duration::from_secs(60))
        {
            self.recorder = recorder::scan(&self.opts.root.join("data"), cs_live::now_ms());
            self.recorder_at = Some(Instant::now());
            self.local_offset_s = local_offset();
        }
        if let Some(d) = self.opts.exit_after
            && self.started.elapsed() >= d
        {
            if self.opts.layout_check && !self.exit_logged {
                self.exit_logged = true;
                let (sh, st, _) = self.read();
                eprintln!(
                    "state at exit: status {:?}, warm-up candles {}, snapshot as of {}, last price {}, drift {:?}, reconnects {}, stats {:?}, alerts {}, journal rows {}, note {:?}",
                    st,
                    sh.warmup_candles,
                    sh.snapshot
                        .map_or_else(|| "none".into(), |s| cs_core::time::fmt_minute(s.close_at)),
                    sh.last_price,
                    sh.drift,
                    sh.reconnects,
                    sh.stats,
                    sh.signals.len(),
                    self.journal.rows.len(),
                    sh.note
                );
            }
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        self.process_screen();
        self.resolve();
        self.save_settings(ctx);
        self.screenshot(ctx);
        // Faster repaints while screen reading is on, so a switch shows within ~0.25 s.
        let every = if self.screen_on { 250 } else { 1000 };
        ctx.request_repaint_after(Duration::from_millis(every));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.frames += 1;
        let (shared, status, sample) = self.read();
        let now = self.now();
        let symbol = self.ctl.symbol.clone();
        let account = parse_number(&self.ctl.account).unwrap_or(f64::NAN);
        let risk_pct = parse_number(&self.ctl.risk_pct).unwrap_or(f64::NAN);
        // Alerts that arrived since the last frame: decided once and logged.
        let hidden = if self.muted {
            "muted"
        } else if self.mismatch.is_some() {
            "mismatch"
        } else if self.screen_block.is_some() {
            "screen"
        } else {
            ""
        };
        self.board.update(
            &shared,
            &Arrival {
                now,
                stats: self.stats.as_ref(),
                limits: self.ctl.limits,
                account,
                risk_pct,
                hidden: hidden.to_string(),
            },
            &mut self.journal,
        );
        let hold = journal::hold(&self.ctl.limits, &journal::today(&self.journal.rows, now));
        let active = self.board.active(&shared);
        // Rules tested on this time frame (none on 5m), known before the feed reports in.
        let n_rules = rules::rules_for(self.ctl.tf).len();
        let main = view::main_model(&MainInput {
            muted: self.muted,
            supported: !matches!(self.source, Source::Unsupported),
            symbol: &symbol,
            screen_block: self.screen_block.as_deref(),
            mismatch: self.mismatch.is_some(),
            n_rules,
            status,
            has_snapshot: shared.snapshot.is_some(),
            stats: self.stats.as_ref(),
            active,
            disagree: self.board.disagree(now),
            hold,
            account: self.board.sizing().0,
            risk_pct: self.board.sizing().1,
            offset_s: self.local_offset_s,
        });
        if main.key != self.hero_key {
            self.hero_key.clone_from(&main.key);
            self.hero_since = Instant::now();
        }
        // The hero fades in over 0.35 s (not in screenshot runs).
        let reveal = if self.opts.screenshot.is_some() {
            1.0
        } else {
            (self.hero_since.elapsed().as_secs_f32() / 0.35).min(1.0)
        };
        if reveal < 1.0 {
            ui.ctx().request_repaint();
        }
        let v = View {
            symbol: &symbol,
            tf: self.ctl.tf,
            status,
            shared: &shared,
            sample: sample.as_deref(),
            now_ms: now,
            local_offset_s: self.local_offset_s,
            recorder: self.recorder,
            details: self.details,
            muted: self.muted,
            click_through: self.click_through,
            hotkeys_ok: self.keys.as_ref().is_some_and(|k| k.all_ok),
            screen_avail: self.vision.is_some(),
            screen_on: self.screen_on,
            manual: self.manual,
            screen_line: match (&self.vision, &self.vision_note) {
                (Some(_), _) => copy::screen_line(&self.screen_state, self.manual),
                (None, Some(n)) => format!("Screen reading unavailable: {n}."),
                (None, None) => "Screen reading unavailable.".into(),
            },
            mismatch: self
                .mismatch
                .map(|(s, f, d)| copy::mismatch(s, f, &symbol, d)),
            debug_front: self.screen_state.front_debug.clone(),
            debug_failures: self
                .screen_state
                .failures
                .iter()
                .rev()
                .take(3)
                .rev()
                .cloned()
                .collect(),
            main,
            reveal,
            stats: self.stats.as_ref(),
            active,
            journal: &self.journal,
            paper_since: self.paper_since,
            hold,
            import_msg: &self.import_msg,
        };
        let fill = pal::bg(self.ctl.opacity);
        let frame = |top: bool| {
            egui::Frame::new()
                .fill(fill)
                .inner_margin(egui::Margin::symmetric(10, if top { 7 } else { 6 }))
        };
        let mut acts = Vec::new();
        let h = egui::Panel::top("header")
            .frame(frame(true).stroke(egui::Stroke::NONE))
            .show(ui, |ui| view::header(ui, &v, &mut self.ctl, &mut acts));
        let b = egui::CentralPanel::default()
            .frame(frame(false))
            .show(ui, |ui| view::body(ui, &v, &mut self.ctl, &mut acts));
        let win = ui.ctx().content_rect().size();
        self.layout = view::Layout_ {
            header: h.response.rect.height(),
            main_needed: b.inner,
            window: win.y,
        };
        if self.opts.layout_check && !self.layout_printed && self.frames >= 4 {
            self.layout_printed = true;
            let l = self.layout;
            let glyphs = "\u{25C9}\u{2026}\u{2261}\u{25CF}\u{25B2}\u{2715}\u{25CB}\u{2191}\u{2193}\u{2194}\u{25BE}\u{25B8}\u{25E2}\u{26A0}\u{00B7}\u{2013}\u{2192}";
            let has = ui
                .ctx()
                .fonts_mut(|f| f.has_glyphs(&FontId::proportional(pal::FONT_MIN), glyphs));
            // Header + main view + the central panel's top and bottom margins (6 + 6).
            let used = l.header + l.main_needed + 12.0;
            eprintln!(
                "layout: window {:.0}x{:.0}, state {:?}, items {:?}, header {:.0}, main view needs {:.0}, total {:.0} -> {}; min font {} pt; all glyphs present: {has}",
                win.x,
                win.y,
                v.main.state,
                v.main.items(v.sample.is_some()),
                l.header,
                l.main_needed,
                used,
                if used <= l.window + 0.5 {
                    "fits"
                } else {
                    "OVERFLOW"
                },
                pal::FONT_MIN
            );
        }
        let ctl_done = std::mem::take(&mut acts);
        for a in ctl_done {
            self.apply(ui.ctx(), a);
        }
    }
}
