//! `vision-watch`: runs the overlay's screen reading (same watcher, same config, same stability
//! rule) without the panel and prints one line per second: the window in front, what was read
//! (symbol, time frame, price) and where each came from, and the Binance USDS-M last price of
//! the matching symbol with the price check.
//!
//! ```text
//! vision-watch [--seconds 60] [--root <project folder>] [--diag-by-title]
//! ```
//!
//! `--diag-by-title` is for diagnosis only: it reads the `TradingView` window found by its title
//! even when another window has the focus. Normal runs (and the overlay) read only the window
//! in front. Nothing is saved; the only network use is Binance's public ticker price.

use std::path::PathBuf;
use std::time::{Duration, Instant};

use cs_vision::handshake::{Handshake, check_price};
use cs_vision::read::{PageKind, PriceFrom, SymbolRead};
use cs_vision::source::{DiagTitleSource, FrameSource, XcapSource};
use cs_vision::watch::WatchStatus;
use cs_vision::{VisionConfig, Watcher};

struct Args {
    seconds: u64,
    root: PathBuf,
    by_title: bool,
}

fn args() -> Result<Args, String> {
    let mut a = Args {
        seconds: 60,
        root: std::env::current_dir().map_err(|e| e.to_string())?,
        by_title: false,
    };
    let mut it = std::env::args().skip(1);
    while let Some(x) = it.next() {
        match x.as_str() {
            "--seconds" => {
                a.seconds = it
                    .next()
                    .and_then(|v| v.parse().ok())
                    .ok_or("--seconds needs a number")?;
            }
            "--root" => a.root = PathBuf::from(it.next().ok_or("--root needs a folder")?),
            "--diag-by-title" => a.by_title = true,
            other => return Err(format!("unknown argument {other}")),
        }
    }
    Ok(a)
}

/// Binance USDS-M last price (public endpoint, no key).
fn binance_price(client: &mut cs_data::http::Client, symbol: &str) -> Option<f64> {
    let url = format!("https://fapi.binance.com/fapi/v1/ticker/price?symbol={symbol}");
    let v = client.get_json(&url).ok()?;
    v.get("price")?.as_str()?.parse().ok()
}

fn hms() -> String {
    let t = cs_core::time::now_ms();
    let (_, _, _, h, m, s) = cs_core::time::split(t);
    format!("{h:02}:{m:02}:{s:02}")
}

#[allow(clippy::print_stdout, clippy::too_many_lines)]
fn main() -> Result<(), String> {
    let a = args()?;
    let cfg = VisionConfig::load(&a.root)?;
    let tol = cfg.price_tolerance_pct;
    let by_title = a.by_title;
    let src_cfg = cfg.clone();
    let w = Watcher::spawn(
        cfg,
        move || {
            if by_title {
                Box::new(DiagTitleSource::new(src_cfg)) as Box<dyn FrameSource>
            } else {
                Box::new(XcapSource::new()) as Box<dyn FrameSource>
            }
        },
        None,
        true,
    );
    let mut client = cs_data::http::Client::new(Duration::from_millis(250));
    println!(
        "vision-watch: {} s, {} (UTC times; nothing is saved)",
        a.seconds,
        if by_title {
            "DIAGNOSTIC: TradingView window found by title"
        } else {
            "window in front only, as in the overlay"
        }
    );
    let start = Instant::now();
    let mut n = 0u64;
    let (mut reads, mut full, mut matched) = (0u64, 0u64, 0u64);
    let mut last_frames = 0u64;
    while start.elapsed() < Duration::from_secs(a.seconds) {
        n += 1;
        let next = start + Duration::from_secs(n);
        std::thread::sleep(next.saturating_duration_since(Instant::now()));
        let s = w.state();
        let front = match (&s.front_app, &s.platform_title) {
            (Some(app), Some(t)) => format!("{app} | {t}"),
            (Some(app), None) => format!("{app} | [title not shown: not a chart tab]"),
            (None, _) => "no window".into(),
        };
        let status = match s.status {
            WatchStatus::Off => "off",
            WatchStatus::NoEngine => "NO OCR ENGINE",
            WatchStatus::Waiting => "TradingView not in front",
            WatchStatus::Reading if s.in_grace => "left the front < 3 s (held)",
            WatchStatus::Reading => "reading",
        };
        let new_read = s.frames > last_frames;
        last_frames = s.frames;
        let Some(raw) = s.raw.as_ref() else {
            println!("{} | front: {front} | {status} | no read yet", hms());
            continue;
        };
        let shown = s.reading.as_ref().unwrap_or(raw);
        let sym = match &shown.symbol {
            SymbolRead::Unknown => "symbol not read".to_string(),
            SymbolRead::Supported { ticker, exchange } => {
                format!("{ticker}->{exchange} ({})", shown.symbol_from)
            }
            SymbolRead::Unsupported { ticker } => {
                format!("{ticker} unsupported ({})", shown.symbol_from)
            }
        };
        let tf = match &shown.tf {
            None => match &raw.tf_note {
                Some(n) => format!("time frame not read ({n})"),
                None => "time frame not read".into(),
            },
            Some(t) => {
                let ok = t.tf.is_some_and(|x| cs_live::SUPPORTED_TFS.contains(&x));
                if ok {
                    format!("{} ({})", t.label(), shown.tf_from)
                } else {
                    format!("unsupported time frame: {} ({})", t.raw, shown.tf_from)
                }
            }
        };
        let px = shown.price.map_or_else(
            || "price not read".to_string(),
            |(p, f)| {
                format!(
                    "{p} ({})",
                    if f == PriceFrom::AxisLabel {
                        "price label"
                    } else {
                        "window title"
                    }
                )
            },
        );
        let feed = shown
            .symbol
            .exchange()
            .and_then(|x| binance_price(&mut client, x).map(|p| (x.to_string(), p)));
        let check = match feed {
            Some((ref x, fp)) => match check_price(shown.price.map(|p| p.0), fp, tol) {
                Handshake::Match { diff_pct } => {
                    format!("Binance {x} {fp} ({diff_pct:+.3} %, match)")
                }
                Handshake::Mismatch { diff_pct, .. } => {
                    format!("Binance {x} {fp} ({diff_pct:+.3} %, SOURCE MISMATCH)")
                }
                _ => format!("Binance {x} {fp}"),
            },
            None => "Binance: n/a".into(),
        };
        let held = if shown.held.is_empty() {
            String::new()
        } else {
            format!(" | held: {}", shown.held.join("+"))
        };
        let confirmed = if s.confirmed {
            "confirmed"
        } else {
            "not yet confirmed"
        };
        println!(
            "{} | front: {front} | {status} | {sym} | {tf} | {px} | {check} | {confirmed}{held}{}",
            hms(),
            if new_read {
                ""
            } else {
                " | (no new read this second)"
            }
        );
        if new_read && raw.page == PageKind::Chart {
            reads += 1;
            let complete =
                raw.symbol.exchange().is_some() && raw.tf.is_some() && raw.price.is_some();
            if complete {
                full += 1;
            }
            if complete
                && matches!(
                    check_price(raw.price.map(|p| p.0), feed.map_or(f64::NAN, |f| f.1), tol),
                    Handshake::Match { .. }
                )
            {
                matched += 1;
            }
        }
    }
    println!(
        "summary: {reads} new reads of a chart; {full} gave symbol, time frame and price; {matched} of those matched Binance within {tol} %"
    );
    Ok(())
}
