//! Drawing the panel. Pure presentation: reads the feed state, returns the user's actions.
//! Every piece of text comes from [`crate::copy`].

use cs_core::Timeframe;
use cs_live::feed::Shared;
use cs_live::snapshot::ContextSnapshot;
use cs_live::status::FeedStatus;
use eframe::egui::{
    self, Align, Button, Color32, ComboBox, CornerRadius, Frame, Layout, Margin, RichText, Sense,
    Stroke, TextEdit, Ui, vec2,
};

use crate::copy;
use crate::palette::{self as pal, c};
use crate::recorder::RecorderInfo;
use crate::risk;

/// What the panel shows this frame.
pub struct View<'a> {
    pub symbol: &'a str,
    /// `None` when the symbol is unsupported (no feed runs).
    pub status: Option<FeedStatus>,
    pub shared: &'a Shared,
    pub sample: Option<&'a str>,
    pub now_ms: i64,
    pub local_offset_s: i64,
    pub recorder: RecorderInfo,
    pub compact: bool,
    pub muted: bool,
    pub click_through: bool,
    pub hotkeys_ok: bool,
    /// Screen reading: available (watcher running), on, manual override, one status line.
    pub screen_avail: bool,
    pub screen_on: bool,
    pub manual: bool,
    pub screen_line: String,
    /// The screen shows a symbol or time frame without a feed (replaces the context).
    pub screen_block: Option<String>,
    /// Price handshake failed (replaces the context): long and compact wording.
    pub mismatch: Option<(String, String)>,
    /// Footer text for the symbol and time frame shown.
    pub footer: String,
    /// Program and title of the window in front: only with `CANDLESCOPE_VISION_DEBUG=1`, only
    /// in the expanded panel, held in memory and never logged.
    pub debug_front: Option<String>,
    /// Debug flag only: the latest read failures with the window in front (newest last).
    pub debug_failures: Vec<String>,
}

/// Inputs the user edits (kept by the app between frames).
pub struct Controls {
    pub symbol: String,
    pub tf: Timeframe,
    pub opacity: f32,
    pub account: String,
    pub risk_pct: String,
    pub entry: String,
    pub stop: String,
    pub funding_n: String,
    /// Scroll the expanded panel to the risk calculator (set for screenshots with prices).
    pub focus_risk: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Action {
    ToggleMute,
    ToggleCompact,
    SetFeed(String, Timeframe),
    SetClickThrough(bool),
    StartDrag,
    BeginResize,
    ToggleScreen,
    FollowScreen,
}

/// Heights measured while drawing (for the layout check).
#[derive(Debug, Clone, Copy, Default)]
pub struct Layout_ {
    pub header: f32,
    pub footer: f32,
    pub body_needed: f32,
    pub body_avail: f32,
}

fn text(s: impl Into<String>) -> RichText {
    RichText::new(s).color(c(pal::TEXT))
}

fn dim(s: impl Into<String>) -> RichText {
    RichText::new(s).color(c(pal::TEXT_DIM))
}

fn head(ui: &mut Ui, s: &str) {
    ui.add_space(6.0);
    ui.label(
        RichText::new(s)
            .strong()
            .size(pal::FONT_HEAD)
            .color(c(pal::TEXT)),
    );
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

pub fn header(ui: &mut Ui, v: &View, ctl: &mut Controls, out: &mut Vec<Action>) {
    // The whole header strip is a drag handle (widgets on top keep their clicks).
    let strip = ui.max_rect();
    let drag = ui.interact(strip, ui.make_persistent_id("drag"), Sense::drag());
    if drag.drag_started() {
        out.push(Action::StartDrag);
    }
    ui.horizontal(|ui| {
        ui.label(text("\u{2261}").size(pal::FONT_BIG))
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
            .width(if v.compact { 104.0 } else { 120.0 })
            .selected_text(text(ctl.symbol.clone()).strong())
            .show_ui(ui, |ui| {
                for s in &syms {
                    ui.selectable_value(&mut ctl.symbol, s.clone(), s.as_str());
                }
            });
        ComboBox::from_id_salt("tf")
            .width(58.0)
            .selected_text(text(ctl.tf.label()).strong())
            .show_ui(ui, |ui| {
                for tf in cs_live::SUPPORTED_TFS {
                    ui.selectable_value(&mut ctl.tf, tf, tf.label());
                }
            });
        if (ctl.symbol.clone(), ctl.tf) != before {
            out.push(Action::SetFeed(ctl.symbol.clone(), ctl.tf));
        }
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let label = if v.muted { copy::UNMUTE } else { copy::MUTE };
            let b = Button::new(
                RichText::new(label)
                    .strong()
                    .size(pal::FONT_BIG)
                    .color(c(pal::TEXT)),
            )
            .fill(c(pal::BG_RAISED))
            .stroke(Stroke::new(2.0, c(pal::TEXT_DIM)))
            .corner_radius(CornerRadius::same(4))
            .min_size(vec2(84.0, 28.0));
            if ui.add(b).on_hover_text("Ctrl+Alt+M").clicked() {
                out.push(Action::ToggleMute);
            }
        });
    });
    ui.horizontal(|ui| {
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
        ui.label(dim(copy::clock(v.now_ms, v.local_offset_s)));
        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            let (glyph, tip) = if v.compact {
                ("\u{25BE}", copy::MORE)
            } else {
                ("\u{25B4}", copy::LESS)
            };
            let b = Button::new(text(glyph).size(pal::FONT_BIG)).min_size(vec2(30.0, 22.0));
            if ui.add(b).on_hover_text(tip).clicked() {
                out.push(Action::ToggleCompact);
            }
        });
    });
    ui.horizontal(|ui| {
        let (label, col) = if v.screen_on {
            (copy::SCREEN_ON, c(pal::LIVE))
        } else {
            (copy::SCREEN_OFF, c(pal::TEXT_DIM))
        };
        let b = Button::new(RichText::new(label).strong().color(col))
            .fill(c(pal::BG_RAISED))
            .stroke(Stroke::new(1.0, col));
        let r = ui
            .add_enabled(v.screen_avail, b)
            .on_hover_text(copy::SCREEN_TIP);
        if r.clicked() {
            out.push(Action::ToggleScreen);
        }
        if v.screen_on && v.manual && ui.button(text(copy::FOLLOW_SCREEN)).clicked() {
            out.push(Action::FollowScreen);
        }
        if !v.compact || !v.manual {
            ui.add(egui::Label::new(dim(v.screen_line.clone())).truncate());
        }
    });
    if let Some(s) = v.sample {
        ui.label(RichText::new(s).strong().color(c(pal::WARN)));
    }
}

pub fn footer(ui: &mut Ui, v: &View, out: &mut Vec<Action>) {
    if let Some(d) = v.shared.drift
        && d.warn()
        && v.status.is_some()
    {
        wrap(
            ui,
            RichText::new(copy::drift_warning(d.offset_ms)).color(c(pal::WARN)),
        );
    }
    wrap(ui, text(v.footer.clone()).size(pal::FONT_MIN));
    if !v.compact {
        wrap(ui, dim(copy::HELP_KEYS).size(pal::FONT_MIN));
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
    }
    let r = v.recorder;
    let line = if v.compact {
        copy::recorder_short(r.days, r.last_age_s)
    } else {
        copy::recorder(r.days, r.hours, r.last_age_s)
    };
    ui.horizontal(|ui| {
        wrap(ui, dim(line).size(pal::FONT_MIN));
        ui.with_layout(Layout::right_to_left(Align::Max), |ui| {
            let g = ui.add(egui::Label::new(dim("\u{25E2}")).sense(Sense::drag()));
            let g = g.on_hover_text(copy::RESIZE_HINT);
            if g.drag_started() {
                out.push(Action::BeginResize);
            }
        });
    });
}

/// Body. Returns the height the content needed.
pub fn body(ui: &mut Ui, v: &View, ctl: &mut Controls, out: &mut Vec<Action>) -> f32 {
    let top = ui.cursor().top();
    let warn = |t: String| RichText::new(t).strong().color(c(pal::WARN));
    if v.click_through {
        let t = if v.compact {
            copy::CLICK_ON_SHORT
        } else {
            copy::CLICK_ON
        };
        wrap(ui, warn(t.to_string()));
    }
    if v.muted {
        if v.compact {
            wrap(ui, text(copy::MUTED_SHORT).strong());
        } else {
            wrap(ui, text(copy::MUTED_BODY).strong().size(pal::FONT_HEAD));
            if let Some(s) = v.status {
                wrap(ui, dim(format!("Feed: {}", s.label())));
            }
            wrap(ui, dim(copy::MUTED_HOW));
        }
        return ui.cursor().top() - top;
    }
    if let Some(b) = &v.screen_block {
        wrap(ui, warn(b.clone()));
        return ui.cursor().top() - top;
    }
    if let Some((long, short)) = &v.mismatch {
        let t = if v.compact { short } else { long };
        wrap(ui, RichText::new(t.clone()).strong().color(c(pal::ALERT)));
        return ui.cursor().top() - top;
    }
    let Some(status) = v.status else {
        wrap(ui, warn(copy::unsupported(v.symbol)));
        return ui.cursor().top() - top;
    };
    let sh = v.shared;
    let Some(s) = sh.snapshot.as_ref() else {
        if v.compact {
            wrap(ui, warn(copy::no_data_short(&sh.note)));
        } else {
            wrap(ui, warn(copy::no_data(&sh.note, sh.warmup_candles)));
            wrap(
                ui,
                dim("The panel shows context once the warm-up candles have loaded."),
            );
        }
        return ui.cursor().top() - top;
    };
    let asof = copy::md_hm(s.close_at);
    let age = sh.last_msg.map(|t| t.elapsed().as_secs());
    let banner = match (status, v.compact) {
        (FeedStatus::Stale, false) => Some(warn(copy::stale_banner(age, &asof))),
        (FeedStatus::Stale, true) => Some(warn(copy::stale_short(age, &asof))),
        (FeedStatus::Disconnected, false) => Some(
            RichText::new(copy::disconnected_banner(&sh.note, &asof))
                .strong()
                .color(c(pal::ALERT)),
        ),
        (FeedStatus::Disconnected, true) => Some(
            RichText::new(copy::disconnected_short(&asof))
                .strong()
                .color(c(pal::ALERT)),
        ),
        _ => None,
    };
    let has_banner = banner.is_some();
    if let Some(b) = banner {
        wrap(ui, b);
    }
    if v.compact {
        // With a banner (or the click-through notice) the zone line gives way, so the panel
        // keeps its size.
        compact(ui, s, !has_banner && !v.click_through);
    } else {
        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {
                expanded(ui, v, s, ctl, out);
            });
    }
    ui.cursor().top() - top
}

fn compact(ui: &mut Ui, s: &ContextSnapshot, zones: bool) {
    wrap(
        ui,
        text(format!(
            "{} \u{00B7} {}",
            copy::htf_compact(&s.htf[0]),
            copy::htf_compact(&s.htf[1])
        )),
    );
    wrap(
        ui,
        text(format!(
            "{} \u{00B7} {} \u{00B7} {}",
            copy::regime(s.regime, s.adx),
            copy::vol_short(s.vol_pct),
            copy::session_short(s.session)
        )),
    );
    if !zones {
        return;
    }
    let mut z: Vec<String> = Vec::new();
    for x in [s.sr_inside, s.sr_below, s.sr_above].into_iter().flatten() {
        z.push(copy::zone_compact(&x));
    }
    let zl = if z.is_empty() {
        "No S/R zone tracked".to_string()
    } else {
        format!("Nearest {}", z.join(" \u{00B7} "))
    };
    wrap(ui, text(zl));
}

fn expanded(ui: &mut Ui, v: &View, s: &ContextSnapshot, ctl: &mut Controls, out: &mut Vec<Action>) {
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

    if ctl.focus_risk {
        ui.scroll_to_cursor(Some(Align::TOP));
    }
    head(ui, copy::H_RISK);
    risk_panel(ui, v, s, ctl);

    head(ui, copy::H_SETTINGS);
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
    if !v.hotkeys_ok {
        wrap(
            ui,
            RichText::new("Some hotkeys could not be registered (another program uses them).")
                .color(c(pal::WARN)),
        );
    }
}

fn field(ui: &mut Ui, label: &str, val: &mut String) {
    ui.label(text(label));
    ui.add(
        TextEdit::singleline(val)
            .desired_width(110.0)
            .font(egui::TextStyle::Body),
    );
    ui.end_row();
}

fn risk_panel(ui: &mut Ui, v: &View, s: &ContextSnapshot, ctl: &mut Controls) {
    egui::Grid::new("risk")
        .num_columns(2)
        .spacing(vec2(10.0, 6.0))
        .show(ui, |ui| {
            field(ui, copy::RISK_ACCOUNT, &mut ctl.account);
            field(ui, copy::RISK_PCT, &mut ctl.risk_pct);
            field(ui, copy::RISK_ENTRY, &mut ctl.entry);
            field(ui, copy::RISK_STOP, &mut ctl.stop);
            field(ui, copy::RISK_FUNDING, &mut ctl.funding_n);
        });
    let base = v.symbol.trim_end_matches("USDT");
    if ctl.entry.trim().is_empty() || ctl.stop.trim().is_empty() {
        wrap(ui, dim(copy::RISK_NEED));
    } else {
        let rate = v.shared.mark.map_or(f64::NAN, |m| m.funding_rate);
        let inp = risk::RiskIn {
            account: risk::parse(&ctl.account).unwrap_or(f64::NAN),
            risk_pct: risk::parse(&ctl.risk_pct).unwrap_or(f64::NAN),
            entry: risk::parse(&ctl.entry).unwrap_or(f64::NAN),
            stop: risk::parse(&ctl.stop).unwrap_or(f64::NAN),
            atr: s.atr,
            funding_rate: rate,
            funding_n: risk::parse(&ctl.funding_n).unwrap_or(0.0),
            cost: cs_backtest::cost::CostModel::BINANCE_UM,
        };
        match risk::calc(&inp) {
            None => wrap(ui, RichText::new(copy::RISK_BAD).color(c(pal::WARN))),
            Some(r) => {
                let side = if r.side > 0 {
                    "Stop below entry: this loses if price falls to the stop."
                } else {
                    "Stop above entry: this loses if price rises to the stop."
                };
                wrap(ui, dim(side));
                wrap(
                    ui,
                    text(format!(
                        "Position size: {:.4} {base} (notional {} USDT, {:.1}x)",
                        r.qty,
                        copy::price(r.notional),
                        r.leverage
                    ))
                    .strong(),
                );
                if r.capped {
                    wrap(
                        ui,
                        RichText::new(format!(
                            "Capped at {:.0}x leverage: the size is smaller, so the stop risks {:.2} % instead of {} %.",
                            risk::MAX_LEVERAGE,
                            r.risk_frac * 100.0,
                            ctl.risk_pct.trim()
                        ))
                        .color(c(pal::WARN)),
                    );
                }
                let d = r.dist;
                wrap(
                    ui,
                    text(format!(
                        "Round-trip cost at this stop: {:.2} R ({:.2} USDT): fees {:.2} R, spread {:.2} R, slippage {:.2} R, funding {:.2} R",
                        r.cost_r,
                        r.cost_cash,
                        r.fee / d,
                        r.spread / d,
                        r.slip / d,
                        r.funding / d
                    ))
                    .strong(),
                );
                wrap(
                    ui,
                    dim(format!(
                        "A stop-out would lose about {:.2} R ({:.2} % of the account) after these costs.",
                        1.0 + r.cost_r,
                        (1.0 + r.cost_r) * r.risk_frac * 100.0
                    )),
                );
                if r.warn {
                    Frame::new()
                        .fill(c(pal::BG_RAISED))
                        .stroke(Stroke::new(2.0, c(pal::WARN)))
                        .inner_margin(Margin::same(6))
                        .show(ui, |ui| {
                            wrap(
                                ui,
                                RichText::new(format!(
                                    "\u{26A0} WARNING: cost is {:.2} R, above {:.1} R. The stop is tight compared with fees, spread and slippage.",
                                    r.cost_r,
                                    risk::WARN_COST_R
                                ))
                                .strong()
                                .color(c(pal::WARN)),
                            );
                        });
                }
                if r.funding_credit {
                    wrap(
                        ui,
                        dim(
                            "Funding at the current rate would be received; it is not counted as a saving.",
                        ),
                    );
                }
                let fr = if rate.is_finite() {
                    format!(
                        "Funding rate now: {:.4} % per settlement (8 h).",
                        rate * 100.0
                    )
                } else {
                    "Funding rate not received yet: funding counted as 0.".to_string()
                };
                wrap(ui, dim(fr));
            }
        }
    }
    wrap(ui, dim(copy::RISK_NOTE).size(pal::FONT_MIN));
}
