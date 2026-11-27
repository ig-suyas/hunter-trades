//! `candlescope-overlay`: the always-on-top context panel.
//!
//! ```text
//! candlescope-overlay [--symbol BTCUSDT] [--tf 15m] [--root <project folder>] [--expanded]
//!                     [--muted] [--click-through]
//!                     [--demo live|stale|disconnected|nodata|unsupported|muted|replay]
//!                     [--entry <price> --stop <price>] [--screenshot <png>] [--layout-check]
//!                     [--exit-after <seconds>] [--size <W>x<H>]
//! ```
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
mod view;

use std::path::PathBuf;
use std::time::Duration;

use cs_core::Timeframe;
use eframe::egui;

fn parse() -> Result<app::Opts, String> {
    let mut o = app::Opts {
        symbol: "BTCUSDT".into(),
        tf: Timeframe::M15,
        root: std::env::current_dir().map_err(|e| e.to_string())?,
        demo: None,
        expanded: false,
        muted: false,
        click_through: false,
        screenshot: None,
        entry: String::new(),
        stop: String::new(),
        layout_check: false,
        exit_after: None,
        size: None,
    };
    let mut it = std::env::args().skip(1);
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
                if v == "unsupported" && cs_live::is_supported(&o.symbol) {
                    o.symbol = "NVDAUSDT".into();
                }
                o.demo = Some((st, v));
            }
            "--expanded" => o.expanded = true,
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
        .with_mouse_passthrough(opts.click_through);
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
