//! `candlescope-overlay`: the always-on-top context panel.
//!
//! ```text
//! candlescope-overlay [--symbol BTCUSDT] [--tf 15m] [--root <project folder>] [--expanded]
//!                     [--muted] [--click-through]
//!                     [--demo live|stale|disconnected|nodata|unsupported|muted|replay]
//!                     [--entry <price> --stop <price>] [--screenshot <png>] [--layout-check]
//!                     [--exit-after <seconds>] [--size <W>x<H>]
//!                     [--screen on|off] [--no-settings]
//! ```
//!
//! Settings (position, size, opacity, mode, account size, risk %, last symbol and time frame,
//! screen reading on/off) are restored from `data/overlay-settings.json`; arguments win over
//! the file. Demo, screenshot and layout-check runs neither read nor write the file and never
//! capture the screen.
//!
//! Context only: the GATE found no setup that survives costs (0 of 59). It displays; it never
//! places orders and holds no keys. Network use: Binance public market data only.

#![windows_subsystem = "windows"]
// UI state is a handful of independent on/off flags, and egui layout code reads best top to
// bottom in one function per panel.
#![allow(clippy::struct_excessive_bools, clippy::too_many_lines)]

mod app;
mod copy;
mod demo;
mod palette;
mod png;
mod recorder;
mod risk;
mod screen;
mod settings;
mod view;

use std::path::PathBuf;
use std::time::Duration;

use cs_core::Timeframe;
use eframe::egui;

fn parse() -> Result<app::Opts, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut root = std::env::current_dir().map_err(|e| e.to_string())?;
    if let Some(i) = args.iter().position(|a| a == "--root")
        && let Some(r) = args.get(i + 1)
    {
        root = PathBuf::from(r);
    }
    let check_run = args.iter().any(|a| {
        matches!(
            a.as_str(),
            "--demo" | "--screenshot" | "--layout-check" | "--no-settings"
        )
    });
    let st = if check_run {
        settings::Settings::default()
    } else {
        settings::Settings::load(&root)
    };
    let mut o = app::Opts {
        symbol: st.symbol.clone(),
        tf: st.tf,
        root,
        demo: None,
        expanded: !st.compact,
        muted: false,
        click_through: false,
        screenshot: None,
        entry: String::new(),
        stop: String::new(),
        layout_check: false,
        exit_after: None,
        size: st.size.map(|[w, h]| egui::vec2(w, h)),
        pos: st.pos,
        opacity: st.opacity,
        account: st.account.clone(),
        risk_pct: st.risk_pct.clone(),
        screen: st.screen_reading,
        use_settings: !check_run,
        demo_mismatch: false,
    };
    let mut it = args.into_iter();
    while let Some(a) = it.next() {
        let mut val = || it.next().ok_or_else(|| format!("{a} needs a value"));
        match a.as_str() {
            "--symbol" => o.symbol = val()?.to_uppercase(),
            "--tf" => {
                let v = val()?;
                o.tf = Timeframe::parse(&v).ok_or_else(|| format!("unknown time frame {v}"))?;
            }
            "--root" => o.root = PathBuf::from(val()?),
            "--demo" => {
                let v = val()?;
                let st =
                    demo::DemoState::parse(&v).ok_or_else(|| format!("unknown demo state {v}"))?;
                if v == "muted" {
                    o.muted = true;
                }
                if v == "mismatch" {
                    o.demo_mismatch = true;
                }
                if v == "unsupported" && cs_live::is_supported(&o.symbol) {
                    o.symbol = "NVDAUSDT".into();
                }
                o.demo = Some((st, v));
            }
            "--expanded" => {
                o.expanded = true;
                o.size = None;
            }
            "--screen" => {
                o.screen = match val()?.as_str() {
                    "on" => true,
                    "off" => false,
                    v => return Err(format!("--screen takes on or off, not {v}")),
                };
            }
            "--no-settings" => o.use_settings = false,
            "--muted" => o.muted = true,
            "--click-through" => o.click_through = true,
            "--screenshot" => o.screenshot = Some(PathBuf::from(val()?)),
            "--entry" => o.entry = val()?,
            "--stop" => o.stop = val()?,
            "--layout-check" => o.layout_check = true,
            "--exit-after" => {
                let v: u64 = val()?
                    .parse()
                    .map_err(|_| "--exit-after needs seconds".to_string())?;
                o.exit_after = Some(Duration::from_secs(v));
            }
            "--size" => {
                let v = val()?;
                let (w, h) = v.split_once('x').ok_or("--size needs WxH")?;
                let w: f32 = w.parse().map_err(|_| "--size needs WxH")?;
                let h: f32 = h.parse().map_err(|_| "--size needs WxH")?;
                o.size = Some(egui::vec2(w, h));
            }
            other => return Err(format!("unknown argument {other}")),
        }
    }
    if !cs_live::SUPPORTED_TFS.contains(&o.tf) {
        return Err(format!(
            "time frame {} not supported (5m, 15m, 1h)",
            o.tf.label()
        ));
    }
    Ok(o)
}

fn main() -> Result<(), String> {
    let opts = parse()?;
    let size = opts.size.unwrap_or(if opts.expanded {
        app::EXPANDED
    } else {
        app::COMPACT
    });
    let vp = egui::ViewportBuilder::default()
        .with_title(copy::APP_TITLE)
        .with_decorations(false)
        .with_transparent(true)
        .with_always_on_top()
        .with_resizable(true)
        .with_inner_size(size)
        .with_min_inner_size([300.0, 200.0])
        .with_mouse_passthrough(opts.click_through)
        // Start without taking the focus: the chart window stays the window in front, which
        // is what screen reading follows.
        .with_active(false);
    let vp = match opts.pos {
        Some(p) => vp.with_position(p),
        None => vp,
    };
    let native = eframe::NativeOptions {
        viewport: vp,
        renderer: eframe::Renderer::Glow,
        ..Default::default()
    };
    eframe::run_native(
        copy::APP_TITLE,
        native,
        Box::new(move |cc| Ok(Box::new(app::OverlayApp::new(cc, opts)))),
    )
    .map_err(|e| e.to_string())
}
