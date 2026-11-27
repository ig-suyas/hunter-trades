//! Drawing the panel. The main view is built from a [`MainModel`] (a pure function of the
//! state, unit-tested to hold only the listed items); Details is one collapsible section below
//! it. Every word comes from [`crate::copy`] or, for the signal panel only,
//! [`crate::signal_copy`].

use cs_core::Timeframe;
use cs_live::feed::Shared;
use cs_live::journal::{self, Hold, Journal, Limits};
use cs_live::signal::rules::{self, RuleDef};
use cs_live::signal::stats::{RuleStats, StatsBook};
use cs_live::signal::{Signal, sizing};
use cs_live::snapshot::ContextSnapshot;
use cs_live::status::FeedStatus;
use eframe::egui::{
    self, Align, Button, Color32, ComboBox, CornerRadius, Layout, RichText, Sense, Stroke,
    TextEdit, Ui, vec2,
};

use crate::copy;
use crate::palette::{self as pal, c};
use crate::recorder::RecorderInfo;
use crate::signal_copy::{self, Row};

/// Everything the main view may contain (window chrome, the drag strip and the resize grip,
/// aside). A test checks every state against this list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Item {
    /// Symbol and time-frame dropdowns, the screen-reading line: "the symbol / time frame /
    /// screen-reading line".
    SymbolPicker,
    TfPicker,
    ScreenLine,
    FeedDot,
    Mute,
    /// WAIT, BUY or SELL, with a short reason or "+N more".
    State,
    Entry,
    Target,
    Stop,
    Hold,
    Size,
    /// "Past results of this rule: ... No edge shown in testing."
    PastLine,
    /// The collapsed Details section's toggle.
    DetailsToggle,
    /// Demo and screenshot runs only: "REPLAY ...".
    ReplayLabel,
}

/// The main view's content.
#[derive(Debug, Clone, PartialEq)]
pub struct MainModel {
    /// "WAIT", "↑ BUY" or "↓ SELL".
    pub state: String,
    /// +1 BUY, -1 SELL, 0 WAIT.
    pub dir: i8,
    /// WAIT's short reason, or "+N more" for an alert.
    pub sub: Option<String>,
    pub rows: Vec<(Row, String, String)>,
    pub past: Option<String>,
    /// Changes whenever the hero changes (for the reveal).
    pub key: String,
}

impl MainModel {
    fn wait(reason: impl Into<String>) -> MainModel {
        let r = reason.into();
        MainModel {
            state: copy::WAIT.into(),
            dir: 0,
            key: format!("wait:{r}"),
            sub: Some(r),
            rows: Vec::new(),
            past: None,
        }
    }

    /// The items this model puts on the main view.
    pub fn items(&self, replay: bool) -> Vec<Item> {
        let mut v = vec![
            Item::SymbolPicker,
            Item::TfPicker,
            Item::FeedDot,
            Item::Mute,
            Item::ScreenLine,
            Item::State,
        ];
        for (r, _, _) in &self.rows {
            v.push(match r {
                Row::Entry => Item::Entry,
                Row::Target => Item::Target,
                Row::Stop => Item::Stop,
                Row::Hold => Item::Hold,
                Row::Size => Item::Size,
            });
        }
        if self.past.is_some() {
            v.push(Item::PastLine);
        }
        v.push(Item::DetailsToggle);
        if replay {
            v.push(Item::ReplayLabel);
        }
        v
    }
}

/// What decides the main view.
pub struct MainInput<'a> {
    pub muted: bool,
    pub supported: bool,
    pub symbol: &'a str,
    pub screen_block: Option<&'a str>,
    pub mismatch: bool,
    /// Frozen rules tested on this time frame.
    pub n_rules: usize,
    pub status: Option<FeedStatus>,
    pub has_snapshot: bool,
    pub stats: Option<&'a StatsBook>,
    /// The shown alert while it runs, with "+N more".
    pub active: Option<(&'a Signal, usize)>,
    pub disagree: bool,
    pub hold: Hold,
    /// Account size and risk % when the shown alert arrived (its size never changes).
    pub account: f64,
    pub risk_pct: f64,
    pub offset_s: i64,
}

/// The main view for a state. Order: anything that hides the panel first, then the alert, then
/// the quiet states.
pub fn main_model(i: &MainInput<'_>) -> MainModel {
    if i.muted {
        return MainModel::wait(copy::WAIT_MUTED);
    }
    if !i.supported {
        return MainModel::wait(copy::wait_unsupported(i.symbol));
    }
    if let Some(b) = i.screen_block {
        return MainModel::wait(b);
    }
    if i.mismatch {
        return MainModel::wait(copy::WAIT_MISMATCH);
    }
    if i.n_rules == 0 {
        return MainModel::wait(copy::WAIT_NO_RULE);
    }
    match i.status {
        Some(FeedStatus::Stale) => return MainModel::wait(copy::WAIT_STALE),
        Some(FeedStatus::Disconnected) => return MainModel::wait(copy::WAIT_DISCONNECTED),
        Some(FeedStatus::Starting) | None => return MainModel::wait(copy::WAIT_LOADING),
        Some(FeedStatus::Live) => {}
    }
    if !i.has_snapshot {
        return MainModel::wait(copy::WAIT_LOADING);
    }
    let Some(book) = i.stats else {
        return MainModel::wait(copy::WAIT_NO_STATS);
    };
    if let Some((s, more)) = i.active
        && let Some(def) = rules::by_order(s.rule)
        && let Some(st) = book.get(s.rule, &s.symbol)
    {
        let p = signal_copy::panel(s, def, st, more, i.account, i.risk_pct, i.offset_s);
        return MainModel {
            state: p.state,
            dir: s.dir,
            sub: p.more,
            rows: p.rows,
            past: Some(p.past),
            key: s.id.clone(),
        };
    }
    if i.disagree {
        return MainModel::wait(copy::WAIT_DISAGREE);
    }
    if i.hold != Hold::Open {
        return MainModel::wait(copy::wait_hold(i.hold));
    }
    MainModel::wait(copy::wait_idle(i.n_rules))
}

/// What the panel shows this frame.
pub struct View<'a> {
    pub symbol: &'a str,
    pub tf: Timeframe,
    /// `None` when the symbol is unsupported (no feed runs).
    pub status: Option<FeedStatus>,
    pub shared: &'a Shared,
    pub sample: Option<&'a str>,
    pub now_ms: i64,
    pub local_offset_s: i64,
    pub recorder: RecorderInfo,
    pub details: bool,
    pub muted: bool,
    pub click_through: bool,
    pub hotkeys_ok: bool,
    pub screen_avail: bool,
    pub screen_on: bool,
    pub manual: bool,
    pub screen_line: String,
    /// Price check text for Details, while the panel is hidden by it.
    pub mismatch: Option<String>,
    pub debug_front: Option<String>,
    pub debug_failures: Vec<String>,
    pub main: MainModel,
    /// 0..1: the hero fades in when it changes.
    pub reveal: f32,
    pub stats: Option<&'a StatsBook>,
    pub active: Option<(&'a Signal, usize)>,
    pub journal: &'a Journal,
    pub paper_since: i64,
    pub hold: Hold,
    pub import_msg: &'a [String],
}

/// Inputs the user edits (kept by the app between frames).
pub struct Controls {
    pub symbol: String,
    pub tf: Timeframe,
    pub opacity: f32,
    pub account: String,
    pub risk_pct: String,
    pub limits: Limits,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    ToggleMute,
    ToggleDetails,
    SetFeed(String, Timeframe),
    SetClickThrough(bool),
    StartDrag,
    BeginResize,
    ToggleScreen,
    FollowScreen,
    ResetPaper,
    ImportFills,
}

/// Heights measured while drawing (for the layout check).
#[derive(Debug, Clone, Copy, Default)]
pub struct Layout_ {
    pub header: f32,
    pub main_needed: f32,
    pub window: f32,
}

fn text(s: impl Into<String>) -> RichText {
    RichText::new(s).color(c(pal::TEXT))
}

fn dim(s: impl Into<String>) -> RichText {
    RichText::new(s).color(c(pal::TEXT_DIM))
}

fn head(ui: &mut Ui, s: &str) {
    ui.add_space(8.0);
    ui.label(
        RichText::new(s)
            .strong()
            .size(pal::FONT_HEAD)
            .color(c(pal::TEXT)),
    );
    ui.add_space(2.0);
}

fn status_color(s: FeedStatus) -> Color32 {
    match s {
        FeedStatus::Live => c(pal::LIVE),
        FeedStatus::Stale | FeedStatus::Starting => c(pal::WARN),
        FeedStatus::Disconnected => c(pal::ALERT),
    }
}

fn wrap(ui: &mut Ui, t: RichText) {
    ui.add(egui::Label::new(t).wrap());
}

/// Header: navigation dropdowns, feed dot, MUTE; the screen-reading line; the replay label.
pub fn header(ui: &mut Ui, v: &View, ctl: &mut Controls, out: &mut Vec<Action>) {
    // The whole header strip is a drag handle (widgets on top keep their clicks).
    let strip = ui.max_rect();
    let drag = ui.interact(strip, ui.make_persistent_id("drag"), Sense::drag());
    if drag.drag_started() {
        out.push(Action::StartDrag);
    }
    ui.horizontal(|ui| {
        ui.label(dim("\u{2261}").size(pal::FONT_BIG))
            .on_hover_text(copy::DRAG_HINT);
        let before = (ctl.symbol.clone(), ctl.tf);
        let mut syms: Vec<String> = cs_live::SUPPORTED_SYMBOLS
            .iter()
            .map(|s| (*s).to_string())
            .collect();
        if !syms.contains(&ctl.symbol) {
            syms.push(ctl.symbol.clone());
        }
        ComboBox::from_id_salt("symbol")
            .width(98.0)
            .selected_text(text(ctl.symbol.clone()).strong())
            .show_ui(ui, |ui| {
                for s in &syms {
                    ui.selectable_value(&mut ctl.symbol, s.clone(), s.as_str());
                }
            })
            .response
            .on_hover_text(copy::SYMBOL_TIP);
        ComboBox::from_id_salt("tf")
            .width(52.0)
            .selected_text(text(ctl.tf.label()).strong())
            .show_ui(ui, |ui| {
                for tf in cs_live::SUPPORTED_TFS {
                    ui.selectable_value(&mut ctl.tf, tf, tf.label());
                }
            })
            .response
            .on_hover_text(copy::TF_TIP);
        if (ctl.symbol.clone(), ctl.tf) != before {
            out.push(Action::SetFeed(ctl.symbol.clone(), ctl.tf));
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let label = if v.muted { copy::UNMUTE } else { copy::MUTE };
            let b = Button::new(
                RichText::new(label)
                    .strong()
                    .size(pal::FONT_BODY)
                    .color(c(pal::TEXT)),
            )
            .fill(c(pal::BG_RAISED))
            .stroke(Stroke::new(1.5, c(pal::TEXT_DIM)))
            .corner_radius(CornerRadius::same(4))
            .min_size(vec2(76.0, 26.0));
            if ui.add(b).on_hover_text(copy::MUTE_TIP).clicked() {
                out.push(Action::ToggleMute);
            }
            match v.status {
                Some(s) => {
                    ui.label(
                        RichText::new(copy::status_badge(s))
                            .strong()
                            .color(status_color(s)),
                    );
                }
                None => {
                    ui.label(
                        RichText::new("\u{2715} UNSUPPORTED")
                            .strong()
                            .color(c(pal::WARN)),
                    );
                }
            }
        });
    });
    ui.horizontal(|ui| {
        let (label, col) = if v.screen_on {
            (copy::SCREEN_ON, c(pal::LIVE))
        } else {
            (copy::SCREEN_OFF, c(pal::TEXT_DIM))
        };
        let b = Button::new(RichText::new(label).color(col))
            .fill(c(pal::BG_RAISED))
            .stroke(Stroke::new(1.0, col));
        if ui
            .add_enabled(v.screen_avail, b)
            .on_hover_text(copy::SCREEN_TIP)
            .clicked()
        {
            out.push(Action::ToggleScreen);
        }
        if v.screen_on && v.manual && ui.button(text(copy::FOLLOW_SCREEN)).clicked() {
            out.push(Action::FollowScreen);
        }
        ui.add(egui::Label::new(dim(v.screen_line.clone())).truncate());
    });
    if let Some(s) = v.sample {
        ui.label(RichText::new(s).strong().color(c(pal::WARN)));
    }
}

/// The main view: the hero state, the alert's rows and its past-results line, the Details
/// toggle; then Details when open. Returns the height the main view needed.
pub fn body(ui: &mut Ui, v: &View, ctl: &mut Controls, out: &mut Vec<Action>) -> f32 {
    let top = ui.cursor().top();
    let m = &v.main;
    let alpha = v.reveal.clamp(0.0, 1.0);
    let fade = |col: Color32| col.gamma_multiply(0.25 + 0.75 * alpha);
    ui.add_space(2.0);
    ui.horizontal(|ui| {
        let col = if m.dir == 0 {
            c(pal::TEXT_DIM)
        } else {
            c(pal::TEXT)
        };
        ui.label(
            RichText::new(&m.state)
                .strong()
                .size(pal::FONT_HERO)
                .color(fade(col)),
        );
        if m.dir != 0
            && let Some(more) = &m.sub
        {
            ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
                ui.label(dim(more.clone()).size(pal::FONT_BODY));
            });
        }
    });
    if m.dir == 0
        && let Some(r) = &m.sub
    {
        wrap(ui, RichText::new(r.clone()).color(fade(c(pal::WARN))));
    }
    if !m.rows.is_empty() {
        ui.add_space(4.0);
        egui::Grid::new("alert")
            .num_columns(2)
            .spacing(vec2(12.0, 3.0))
            .show(ui, |ui| {
                for (r, label, value) in &m.rows {
                    ui.label(dim(label.clone()).size(pal::FONT_BODY));
                    let strong = matches!(r, Row::Entry | Row::Target | Row::Stop);
                    let t = text(value.clone()).size(pal::FONT_BODY);
                    ui.add(egui::Label::new(if strong { t.strong() } else { t }).wrap());
                    ui.end_row();
                }
            });
    }
    if let Some(p) = &m.past {
        ui.add_space(4.0);
        wrap(ui, dim(p.clone()).size(pal::FONT_MIN));
    }
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        // The arrow turns when open and shifts a little on hover.
        let id = ui.make_persistent_id("details_hover");
        let hovered = ui.ctx().data(|d| d.get_temp::<bool>(id)).unwrap_or(false);
        let arrow = if v.details { "\u{25BE}" } else { "\u{25B8}" };
        let gap = if hovered { "\u{2002}" } else { " " };
        let b = Button::new(text(format!("{arrow}{gap}{}", copy::DETAILS)))
            .fill(c(pal::BG_RAISED))
            .stroke(Stroke::new(1.0, c(pal::BORDER)));
        let r = ui.add(b).on_hover_text(copy::DETAILS_TIP);
        ui.ctx().data_mut(|d| d.insert_temp(id, r.hovered()));
        if r.clicked() {
            out.push(Action::ToggleDetails);
        }
        ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
            let g = ui.add(egui::Label::new(dim("\u{25E2}")).sense(Sense::drag()));
            let g = g.on_hover_text(copy::RESIZE_HINT);
            if g.drag_started() {
                out.push(Action::BeginResize);
            }
        });
    });
    let needed = ui.cursor().top() - top;
    if v.details {
        ui.separator();
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| details(ui, v, ctl, out));
    }
    needed
}

fn context(ui: &mut Ui, s: &ContextSnapshot) {
    wrap(ui, dim(copy::as_of(s.close_at)));
    head(ui, copy::H_TREND);
    for h in &s.htf {
        wrap(ui, text(copy::htf(h)));
    }
    wrap(ui, text(copy::regime(s.regime, s.adx)));
    head(ui, copy::H_VOL);
    wrap(ui, text(copy::vol(s.vol_pct)));
    wrap(ui, text(copy::session(s.session)));
    head(ui, copy::H_ZONES);
    if let Some(z) = s.sr_inside {
        wrap(ui, text(copy::zone("S/R (price inside)", &z)));
    }
    let zl = |o: Option<cs_live::snapshot::ZoneView>, l: &str| match o {
        Some(z) => copy::zone(l, &z),
        None => copy::zone_none(l),
    };
    wrap(ui, text(zl(s.sr_above, "S/R above")));
    wrap(ui, text(zl(s.sr_below, "S/R below")));
    wrap(ui, text(zl(s.supply, "Supply")));
    wrap(ui, text(zl(s.demand, "Demand")));
    head(ui, copy::H_EVENTS);
    wrap(ui, text(copy::marker("Sweep", s.sweep.as_ref(), s.idx)));
    wrap(ui, text(copy::marker("BOS", s.bos.as_ref(), s.idx)));
    wrap(ui, text(copy::marker("CHoCH", s.choch.as_ref(), s.idx)));
    head(ui, copy::H_PULLBACK);
    match s.current_pullback() {
        Some(p) => wrap(
            ui,
            text(copy::pullback(
                p.dir,
                p.healthy,
                p.known_at,
                s.idx - p.known_idx,
            )),
        ),
        None => wrap(ui, text(copy::pullback_none(s.pullback_age()))),
    }
    wrap(ui, dim(copy::PULLBACK_NOTE).size(pal::FONT_MIN));
}

fn rule_numbers(ui: &mut Ui, def: &RuleDef, st: &RuleStats, s: &Signal) {
    wrap(ui, text(copy::rule_title(def)).strong());
    wrap(ui, dim(def.describe()));
    for (n, p) in [
        ("Months 1-18", &st.all),
        ("Discovery (picked here)", &st.discovery),
        ("Validation (after picking)", &st.validation),
    ] {
        wrap(ui, text(copy::period_line(n, def, p)));
    }
    let c1 = sizing::cost_r(
        cs_backtest::cost::CostModel::BINANCE_UM,
        s.dir,
        s.entry,
        s.stop,
        s.atr,
    );
    let c2 = sizing::cost_r(
        cs_backtest::cost::CostModel::BINANCE_UM.with_mult(2.0),
        s.dir,
        s.entry,
        s.stop,
        s.atr,
    );
    wrap(ui, text(copy::cost_line(c1, c2)));
    wrap(ui, dim(copy::PAST_NOTE).size(pal::FONT_MIN));
}

fn field(ui: &mut Ui, label: &str, val: &mut String, max: f64) {
    ui.label(text(label));
    let ok = crate::settings::parse_number(val).is_some_and(|x| x > 0.0 && x <= max);
    let r = ui.add(
        TextEdit::singleline(val)
            .desired_width(96.0)
            .font(egui::TextStyle::Body),
    );
    if !ok {
        r.on_hover_text(copy::SET_BAD_NUMBER);
    }
    ui.end_row();
}

fn details(ui: &mut Ui, v: &View, ctl: &mut Controls, out: &mut Vec<Action>) {
    // The alert's rule, or every rule of this time frame.
    if let Some((s, _)) = v.active
        && let Some(def) = rules::by_order(s.rule)
        && let Some(st) = v.stats.and_then(|b| b.get(s.rule, &s.symbol))
    {
        head(ui, copy::H_RULE);
        rule_numbers(ui, def, st, s);
    } else {
        head(ui, copy::H_RULES);
        let rs = rules::rules_for(v.tf);
        if rs.is_empty() {
            wrap(ui, text(copy::WAIT_NO_RULE));
        }
        for def in rs {
            let st = v.stats.and_then(|b| b.get(def.order, v.symbol));
            wrap(ui, text(copy::rule_row(def, st)));
        }
    }

    head(ui, copy::H_PAPER);
    let account = crate::settings::parse_number(&ctl.account).unwrap_or(f64::NAN);
    let paper = journal::paper(&v.journal.rows, account, v.paper_since);
    wrap(ui, text(copy::paper_line(&paper)));
    let today = journal::today(&v.journal.rows, v.now_ms);
    wrap(ui, text(copy::today_line(&today, &ctl.limits)));
    if v.hold != Hold::Open {
        wrap(
            ui,
            RichText::new(copy::wait_hold(v.hold)).color(c(pal::WARN)),
        );
    }
    egui::Grid::new("settings")
        .num_columns(2)
        .spacing(vec2(10.0, 5.0))
        .show(ui, |ui| {
            field(ui, copy::SET_ACCOUNT, &mut ctl.account, 1e12);
            field(ui, copy::SET_RISK, &mut ctl.risk_pct, 100.0);
            ui.label(text(copy::SET_MAX_DAY));
            ui.add(egui::DragValue::new(&mut ctl.limits.max_per_day).range(1..=50));
            ui.end_row();
            ui.label(text(copy::SET_PAUSE_LOSSES));
            ui.add(egui::DragValue::new(&mut ctl.limits.pause_after_losses).range(1..=50));
            ui.end_row();
            ui.label(text(copy::SET_DAILY_LOSS));
            ui.add(
                egui::DragValue::new(&mut ctl.limits.daily_loss_r)
                    .range(0.5..=100.0)
                    .speed(0.1)
                    .max_decimals(1),
            );
            ui.end_row();
        });
    if ui.button(text(copy::SET_RESET_PAPER)).clicked() {
        out.push(Action::ResetPaper);
    }
    wrap(ui, dim(copy::PAPER_NOTE).size(pal::FONT_MIN));

    head(ui, copy::H_CAL);
    let (n, rows) = journal::calibration(&v.journal.rows);
    for l in copy::calibration_lines(n, &rows) {
        wrap(ui, text(l));
    }

    head(ui, copy::H_FILLS);
    wrap(ui, dim(copy::FILLS_HOW).size(pal::FONT_MIN));
    if ui.button(text(copy::FILLS_IMPORT)).clicked() {
        out.push(Action::ImportFills);
    }
    for l in v.import_msg {
        wrap(ui, text(l.clone()));
    }

    if let Some(s) = v.shared.snapshot.as_ref() {
        context(ui, s);
    } else {
        head(ui, copy::H_TREND);
        wrap(ui, dim(copy::WAIT_LOADING));
    }

    head(ui, copy::H_WINDOW);
    wrap(ui, dim(copy::clock(v.now_ms, v.local_offset_s)));
    wrap(ui, dim(copy::times_note(v.local_offset_s)));
    if let Some(s) = v.status {
        wrap(
            ui,
            dim(copy::feed_line(
                s,
                v.shared.reconnects,
                v.shared.warmup_candles,
            )),
        );
    }
    if let Some(d) = v.shared.drift
        && d.warn()
    {
        wrap(
            ui,
            RichText::new(copy::drift_warning(d.offset_ms)).color(c(pal::WARN)),
        );
    }
    if let Some(m) = &v.mismatch {
        wrap(ui, RichText::new(m.clone()).color(c(pal::ALERT)));
    }
    ui.horizontal(|ui| {
        ui.label(text(copy::SET_OPACITY));
        ui.add(
            egui::Slider::new(&mut ctl.opacity, pal::MIN_OPACITY..=1.0)
                .custom_formatter(|x, _| format!("{:.0} %", x * 100.0)),
        );
    });
    let mut ct = v.click_through;
    if ui.checkbox(&mut ct, text(copy::SET_CLICK)).changed() {
        out.push(Action::SetClickThrough(ct));
    }
    if v.click_through {
        wrap(ui, RichText::new(copy::CLICK_ON).color(c(pal::WARN)));
    }
    wrap(ui, dim(copy::HELP_KEYS).size(pal::FONT_MIN));
    if !v.hotkeys_ok {
        wrap(ui, RichText::new(copy::HOTKEYS_TAKEN).color(c(pal::WARN)));
    }
    let r = v.recorder;
    wrap(
        ui,
        dim(copy::recorder(r.days, r.hours, r.last_age_s)).size(pal::FONT_MIN),
    );
    if let Some(d) = &v.debug_front {
        wrap(
            ui,
            RichText::new(copy::front_debug(d))
                .size(pal::FONT_MIN)
                .color(c(pal::WARN)),
        );
        for f in &v.debug_failures {
            wrap(ui, dim(format!("Debug \u{00B7} {f}")).size(pal::FONT_MIN));
        }
    }
    head(ui, copy::H_GATE);
    wrap(ui, dim(copy::gate_note(v.symbol, v.tf)).size(pal::FONT_MIN));
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn book() -> StatsBook {
        StatsBook::parse(include_str!("../../../reports/rule-stats.json")).unwrap()
    }

    fn input<'a>(b: &'a StatsBook, active: Option<(&'a Signal, usize)>) -> MainInput<'a> {
        MainInput {
            muted: false,
            supported: true,
            symbol: "BTCUSDT",
            screen_block: None,
            mismatch: false,
            n_rules: 8,
            status: Some(FeedStatus::Live),
            has_snapshot: true,
            stats: Some(b),
            active,
            disagree: false,
            hold: Hold::Open,
            account: 10_000.0,
            risk_pct: 1.0,
            offset_s: 19_800,
        }
    }

    /// Everything the main view may hold, as the user listed it.
    fn allowed() -> BTreeSet<Item> {
        [
            Item::SymbolPicker,
            Item::TfPicker,
            Item::ScreenLine,
            Item::FeedDot,
            Item::Mute,
            Item::State,
            Item::Entry,
            Item::Target,
            Item::Stop,
            Item::Hold,
            Item::Size,
            Item::PastLine,
            Item::DetailsToggle,
            Item::ReplayLabel,
        ]
        .into_iter()
        .collect()
    }

    /// Every state: the main view holds only the listed items; WAIT holds no alert rows and
    /// no past-results line; BUY and SELL hold exactly Entry, Target, Stop, Hold, Size and the
    /// past-results line, in that order.
    #[test]
    fn main_view_contains_only_the_listed_items() {
        let b = book();
        let up = crate::board::tests_alert(1, 1, "BTCUSDT", rules::Exit::Trail);
        let down = crate::board::tests_alert(4, -1, "BTCUSDT", rules::Exit::R(1.0));
        let mut models = Vec::new();
        let base = input(&b, None);
        models.push(main_model(&base));
        models.push(main_model(&MainInput {
            muted: true,
            active: Some((&up, 0)),
            ..input(&b, None)
        }));
        models.push(main_model(&MainInput {
            supported: false,
            ..input(&b, None)
        }));
        models.push(main_model(&MainInput {
            screen_block: Some("Unsupported time frame: D (one day); 5m, 15m and 1h only"),
            ..input(&b, None)
        }));
        models.push(main_model(&MainInput {
            mismatch: true,
            active: Some((&up, 0)),
            ..input(&b, None)
        }));
        models.push(main_model(&MainInput {
            n_rules: 0,
            ..input(&b, None)
        }));
        for st in [
            FeedStatus::Stale,
            FeedStatus::Disconnected,
            FeedStatus::Starting,
        ] {
            models.push(main_model(&MainInput {
                status: Some(st),
                active: Some((&up, 0)),
                ..input(&b, None)
            }));
        }
        models.push(main_model(&MainInput {
            stats: None,
            ..input(&b, None)
        }));
        models.push(main_model(&MainInput {
            disagree: true,
            ..input(&b, None)
        }));
        models.push(main_model(&MainInput {
            hold: Hold::Losses(3),
            ..input(&b, None)
        }));
        let buy = main_model(&input(&b, Some((&up, 2))));
        let sell = main_model(&input(&b, Some((&down, 0))));
        let allowed = allowed();
        for m in &models {
            assert_eq!(m.state, "WAIT", "{m:?}");
            assert!(m.sub.is_some(), "WAIT always has a short reason: {m:?}");
            let items = m.items(false);
            assert!(items.iter().all(|i| allowed.contains(i)));
            for i in [
                Item::Entry,
                Item::Target,
                Item::Stop,
                Item::Hold,
                Item::Size,
                Item::PastLine,
            ] {
                assert!(!items.contains(&i), "{i:?} in WAIT: {m:?}");
            }
        }
        for (m, word) in [(&buy, "BUY"), (&sell, "SELL")] {
            assert!(
                m.state.contains(word)
                    && (m.state.contains('\u{2191}') || m.state.contains('\u{2193}'))
            );
            let items = m.items(true);
            assert!(items.iter().all(|i| allowed.contains(i)));
            assert_eq!(
                items,
                [
                    Item::SymbolPicker,
                    Item::TfPicker,
                    Item::FeedDot,
                    Item::Mute,
                    Item::ScreenLine,
                    Item::State,
                    Item::Entry,
                    Item::Target,
                    Item::Stop,
                    Item::Hold,
                    Item::Size,
                    Item::PastLine,
                    Item::DetailsToggle,
                    Item::ReplayLabel
                ]
            );
            assert!(
                m.past
                    .as_ref()
                    .unwrap()
                    .ends_with("No edge shown in testing.")
            );
        }
        assert_eq!(buy.sub.as_deref(), Some("+2 more"));
        // Muting, a stale feed and a price mismatch hide an active alert.
        assert_eq!(models[1].sub.as_deref(), Some(copy::WAIT_MUTED));
        assert_eq!(models[4].sub.as_deref(), Some(copy::WAIT_MISMATCH));
        assert_eq!(
            models[5].sub.as_deref(),
            Some("no tested rule for this time frame")
        );
        assert_eq!(models[10].sub.as_deref(), Some("rules disagree"));
        assert_eq!(
            models[11].sub.as_deref(),
            Some("paused: 3 losses in a row today")
        );
    }

    /// The main line flags "low evidence" below 100 trades (with the real file every rule has
    /// at least 110 in months 1-18, so this uses its validation numbers).
    #[test]
    fn low_evidence_on_the_main_line() {
        let b = book();
        let mut low = b.clone();
        for r in &mut low.rows {
            r.2.all = r.2.validation;
        }
        let up = crate::board::tests_alert(1, 1, "BTCUSDT", rules::Exit::Trail);
        let m = main_model(&input(&low, Some((&up, 0))));
        let p = m.past.unwrap();
        assert!(p.contains("(n=83, low evidence)"), "{p}");
        let m = main_model(&input(&b, Some((&up, 0))));
        assert!(!m.past.unwrap().contains("low evidence"));
    }

    /// Size on the main view = the hand calculation (10,000 USDT, 1 %, entry 90,000, stop 89,100
    /// -> 100 USDT / 900 = 0.1111 BTC, 10,000 USDT notional).
    #[test]
    fn size_row_matches_hand_calculation() {
        let b = book();
        let up = crate::board::tests_alert(1, 1, "BTCUSDT", rules::Exit::Trail);
        let m = main_model(&input(&b, Some((&up, 0))));
        let size = m.rows.iter().find(|r| r.0 == Row::Size).unwrap();
        assert_eq!(size.2, "0.1111 BTC (10,000.0 USDT)");
        let m = main_model(&MainInput {
            account: 25_000.0,
            risk_pct: 0.5,
            ..input(&b, Some((&up, 0)))
        });
        let size = m.rows.iter().find(|r| r.0 == Row::Size).unwrap();
        // 125 USDT / 900 = 0.138889 BTC.
        assert_eq!(size.2, "0.1389 BTC (12,500.0 USDT)");
    }
}
