//! The overlay window: borderless, always on top, draggable and resizable, adjustable
//! background opacity, click-through, global hotkeys and the MUTE kill switch. It only
//! displays: there is no order code anywhere in the workspace.

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use cs_core::Timeframe;
use cs_live::feed::{FeedConfig, FeedHandle, Shared, spawn};
use cs_live::status::FeedStatus;
use eframe::egui::{
    self, FontData, FontDefinitions, FontFamily, FontId, TextStyle, Vec2, ViewportCommand, vec2,
};
use global_hotkey::hotkey::{Code, HotKey, Modifiers};
use global_hotkey::{GlobalHotKeyEvent, GlobalHotKeyManager, HotKeyState};

use crate::demo::{self, DemoState};
use crate::palette as pal;
use crate::recorder::{self, RecorderInfo};
use crate::view::{self, Action, Controls, View};

pub const COMPACT: Vec2 = Vec2::new(330.0, 260.0);
pub const EXPANDED: Vec2 = Vec2::new(440.0, 720.0);

/// Command line options.
#[derive(Debug, Clone)]
pub struct Opts {
    pub symbol: String,
    pub tf: Timeframe,
    /// Project root (holds `data/` and `reports/`).
    pub root: PathBuf,
    pub demo: Option<(DemoState, String)>,
    pub expanded: bool,
    pub muted: bool,
    pub click_through: bool,
    pub screenshot: Option<PathBuf>,
    pub entry: String,
    pub stop: String,
    pub layout_check: bool,
    pub exit_after: Option<Duration>,
    /// Window size override (screenshots).
    pub size: Option<Vec2>,
}

enum Source {
    Live(FeedHandle),
    Demo {
        shared: Arc<Mutex<Shared>>,
        state: DemoState,
        label: String,
    },
    Unsupported,
}

struct Keys {
    _mgr: GlobalHotKeyManager,
    rx: Receiver<GlobalHotKeyEvent>,
    hide: u32,
    mute: u32,
    click: u32,
}

pub struct OverlayApp {
    opts: Opts,
    source: Source,
    ctl: Controls,
    compact: bool,
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

fn style(ctx: &egui::Context, compact: bool) {
    let body = if compact {
        pal::FONT_MIN
    } else {
        pal::FONT_BODY
    };
    ctx.all_styles_mut(|s| {
        s.text_styles = [
            (TextStyle::Small, FontId::proportional(pal::FONT_MIN)),
            (TextStyle::Body, FontId::proportional(body)),
            (TextStyle::Button, FontId::proportional(body)),
            (TextStyle::Heading, FontId::proportional(pal::FONT_HEAD)),
            (TextStyle::Monospace, FontId::monospace(pal::FONT_MIN)),
        ]
        .into();
        s.spacing.item_spacing = vec2(6.0, 2.0);
        s.spacing.button_padding = vec2(6.0, 3.0);
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
    let (h, mu, t) = (
        HotKey::new(m, Code::KeyH),
        HotKey::new(m, Code::KeyM),
        HotKey::new(m, Code::KeyT),
    );
    mgr.register_all(&[h, mu, t]).ok()?;
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
    })
}

fn make_source(opts: &Opts, symbol: &str, tf: Timeframe) -> Source {
    if !cs_live::is_supported(symbol) {
        return Source::Unsupported;
    }
    if let Some((st, _)) = &opts.demo {
        let (shared, label) = demo::shared_for(&opts.root, *st);
        return Source::Demo {
            shared,
            state: *st,
            label,
        };
    }
    Source::Live(spawn(FeedConfig::new(symbol, tf)))
}

impl OverlayApp {
    pub fn new(cc: &eframe::CreationContext<'_>, opts: Opts) -> OverlayApp {
        let compact = !opts.expanded;
        let fonts = load_fonts(&cc.egui_ctx);
        if opts.layout_check {
            eprintln!("fonts: Segoe UI loaded = {fonts}");
        }
        style(&cc.egui_ctx, compact);
        let keys = register_keys(&cc.egui_ctx);
        let source = make_source(&opts, &opts.symbol, opts.tf);
        if opts.click_through {
            cc.egui_ctx
                .send_viewport_cmd(ViewportCommand::MousePassthrough(true));
        }
        OverlayApp {
            ctl: Controls {
                symbol: opts.symbol.clone(),
                tf: opts.tf,
                opacity: pal::DEFAULT_OPACITY,
                account: "10,000".into(),
                risk_pct: "1".into(),
                entry: opts.entry.clone(),
                stop: opts.stop.clone(),
                funding_n: "1".into(),
                focus_risk: opts.screenshot.is_some() && !opts.entry.is_empty(),
            },
            compact,
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
            source,
            opts,
        }
    }

    fn read(&self) -> (Shared, Option<FeedStatus>, Option<String>) {
        match &self.source {
            Source::Live(h) => (h.read(), Some(h.status()), None),
            Source::Demo {
                shared,
                state,
                label,
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

    fn apply(&mut self, ctx: &egui::Context, a: Action) {
        match a {
            Action::ToggleMute => self.muted = !self.muted,
            Action::ToggleCompact => {
                self.compact = !self.compact;
                style(ctx, self.compact);
                ctx.send_viewport_cmd(ViewportCommand::InnerSize(if self.compact {
                    COMPACT
                } else {
                    EXPANDED
                }));
            }
            Action::SetFeed(sym, tf) => {
                // Dropping the old handle stops its thread.
                self.source = make_source(&self.opts, &sym, tf);
            }
            Action::SetClickThrough(on) => {
                self.click_through = on;
                ctx.send_viewport_cmd(ViewportCommand::MousePassthrough(on));
            }
            Action::StartDrag => ctx.send_viewport_cmd(ViewportCommand::StartDrag),
            Action::BeginResize => ctx.send_viewport_cmd(ViewportCommand::BeginResize(
                egui::viewport::ResizeDirection::SouthEast,
            )),
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
                    "state at exit: status {:?}, warm-up candles {}, snapshot as of {}, last price {}, funding {:?}, drift {:?}, reconnects {}, stats {:?}, note {:?}",
                    st,
                    sh.warmup_candles,
                    sh.snapshot
                        .map_or_else(|| "none".into(), |s| cs_core::time::fmt_minute(s.close_at)),
                    sh.last_price,
                    sh.mark.map(|m| m.funding_rate),
                    sh.drift,
                    sh.reconnects,
                    sh.stats,
                    sh.note
                );
            }
            ctx.send_viewport_cmd(ViewportCommand::Close);
        }
        self.screenshot(ctx);
        ctx.request_repaint_after(Duration::from_millis(1000));
    }

    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        self.frames += 1;
        let (shared, status, sample) = self.read();
        let symbol = self.ctl.symbol.clone();
        let v = View {
            symbol: &symbol,
            status,
            shared: &shared,
            sample: sample.as_deref(),
            now_ms: cs_live::now_ms(),
            local_offset_s: self.local_offset_s,
            recorder: self.recorder,
            compact: self.compact,
            muted: self.muted,
            click_through: self.click_through,
            hotkeys_ok: self.keys.is_some(),
        };
        let fill = pal::bg(self.ctl.opacity);
        let frame = |top: bool| {
            egui::Frame::new()
                .fill(fill)
                .inner_margin(egui::Margin::symmetric(8, if top { 6 } else { 5 }))
        };
        let mut acts = Vec::new();
        let h = egui::Panel::top("header")
            .frame(frame(true).stroke(egui::Stroke::NONE))
            .show(ui, |ui| view::header(ui, &v, &mut self.ctl, &mut acts));
        let f = egui::Panel::bottom("footer")
            .frame(frame(false))
            .show(ui, |ui| view::footer(ui, &v, &mut acts));
        let b = egui::CentralPanel::default()
            .frame(frame(false))
            .show(ui, |ui| {
                let avail = ui.available_height();
                (view::body(ui, &v, &mut self.ctl, &mut acts), avail)
            });
        self.layout = view::Layout_ {
            header: h.response.rect.height(),
            footer: f.response.rect.height(),
            body_needed: b.inner.0,
            body_avail: b.inner.1,
        };
        if self.opts.layout_check && !self.layout_printed && self.frames >= 4 {
            self.layout_printed = true;
            let l = self.layout;
            let glyphs = "\u{2261}\u{25CF}\u{25B2}\u{2715}\u{25CB}\u{2191}\u{2193}\u{2194}\u{25BE}\u{25B4}\u{25E2}\u{26A0}\u{00B7}\u{2013}";
            let has = ui
                .ctx()
                .fonts_mut(|f| f.has_glyphs(&FontId::proportional(pal::FONT_MIN), glyphs));
            let win = ui.ctx().content_rect().size();
            eprintln!(
                "layout: window {:.0}x{:.0}, header {:.0}, footer {:.0}, body needs {:.0} of {:.0} available -> {}; all glyphs present: {has}",
                win.x,
                win.y,
                l.header,
                l.footer,
                l.body_needed,
                l.body_avail,
                if l.body_needed <= l.body_avail + 0.5 || !self.compact {
                    "fits"
                } else {
                    "OVERFLOW"
                }
            );
        }
        for a in acts {
            self.apply(ui.ctx(), a);
        }
    }
}
