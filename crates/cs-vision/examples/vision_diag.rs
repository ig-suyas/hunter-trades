//! DIAGNOSTIC ONLY (never used by the overlay): finds the TradingView window by its title, even
//! when it is not in front, captures it into memory and prints text about it: window and capture
//! size, per-region pixel statistics and the OCR text of each region with each preprocessing.
//! Nothing is saved unless `--crops <dir>` is given (small region crops as PPM, for debugging).
//!
//! `vision_diag regions [--crops DIR]` | `vision_diag dump` (OCR lines with boxes, top-left
//! part of the window only) | `vision_diag read N` (N reads through the real reader)
// A development tool: a panic on a missing window or capture is the right behaviour here.
#![allow(
    clippy::print_stdout,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::cast_lossless,
    clippy::expect_used,
    clippy::too_many_lines,
    clippy::doc_markdown,
    clippy::chunks_exact_to_as_chunks
)]

use std::time::Instant;

use cs_vision::ocr::Engines;
use cs_vision::read::Reader;
use cs_vision::source::{WindowInfo, match_platform};
use cs_vision::{Frame, RectF, VisionConfig};

fn find(cfg: &VisionConfig) -> Option<(xcap::Window, WindowInfo)> {
    for w in xcap::Window::all().ok()? {
        if w.is_minimized().unwrap_or(true) {
            continue;
        }
        let info = WindowInfo {
            id: w.id().ok()?,
            title: w.title().unwrap_or_default(),
            app: w.app_name().unwrap_or_default(),
            w: w.width().unwrap_or(0),
            h: w.height().unwrap_or(0),
            behind_overlay: false,
        };
        if match_platform(cfg, &info).is_some() {
            return Some((w, info));
        }
    }
    None
}

fn capture(w: &xcap::Window) -> Option<Frame> {
    let img = w.capture_image().ok()?;
    let (iw, ih) = (img.width(), img.height());
    Frame::from_rgba(iw, ih, img.into_raw())
}

fn stats(f: &Frame) -> String {
    let n = (f.w * f.h) as f64;
    let (mut black, mut white, mut sum) = (0u64, 0u64, 0u64);
    for p in f.bgra.chunks_exact(4) {
        let l = (u32::from(p[0]) * 29 + u32::from(p[1]) * 150 + u32::from(p[2]) * 77) >> 8;
        sum += u64::from(l);
        if l < 12 {
            black += 1;
        }
        if l > 245 {
            white += 1;
        }
    }
    format!(
        "{}x{} mean {:.0}, black {:.0} %, white {:.0} %",
        f.w,
        f.h,
        sum as f64 / n,
        black as f64 / n * 100.0,
        white as f64 / n * 100.0
    )
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mode = args.first().map_or("regions", String::as_str);
    let cfg = VisionConfig::default_config().expect("config");
    let Some((win, info)) = find(&cfg) else {
        println!("no TradingView window found");
        return;
    };
    println!(
        "window: app {:?}, title {:?}, size {}x{}, focused {}",
        info.app,
        info.title,
        info.w,
        info.h,
        win.is_focused().unwrap_or(false)
    );
    let mut eng = Engines::detect();
    match mode {
        "dump" => {
            let t0 = Instant::now();
            let mut f = capture(&win).expect("capture");
            println!("capture {} in {:?}", stats(&f), t0.elapsed());
            // Top-left part only (toolbar and legend; not the account corner or panels).
            let top = f.crop(&RectF::new(0.0, 0.0, 0.75, 0.25));
            for (k, lines) in [
                (1u32, eng.read(&top.upscale(1))),
                (2, eng.read(&top.binarize(50).upscale(2))),
            ] {
                println!("-- OCR x{k}");
                for l in lines.unwrap_or_default() {
                    let x0 = l.words.iter().map(|w| w.x).fold(f32::INFINITY, f32::min) / k as f32;
                    let y0 = l.words.iter().map(|w| w.y).fold(f32::INFINITY, f32::min) / k as f32;
                    let x1 = l.words.iter().map(|w| w.x + w.w).fold(0.0, f32::max) / k as f32;
                    let y1 = l.words.iter().map(|w| w.y + w.h).fold(0.0, f32::max) / k as f32;
                    println!("{x0:6.0} {y0:5.0} {x1:6.0} {y1:5.0}  {}", l.text);
                }
            }
            f.wipe();
        }
        "axis" => {
            // Where is the price axis? Price-like OCR tokens with their x position, and columns
            // that hold a red/green label band (text and numbers only).
            let mut f = capture(&win).expect("capture");
            let band = f.crop(&RectF::new(0.45, 0.12, 0.55, 0.75));
            let x0 = (0.45 * f.w as f32) as u32;
            let y0 = (0.12 * f.h as f32) as u32;
            for l in eng.read(&band.binarize(50).upscale(2)).unwrap_or_default() {
                if cs_vision::parse::first_price(std::slice::from_ref(&l.text)).is_some() {
                    let x = l.words.iter().map(|w| w.x).fold(f32::INFINITY, f32::min) / 2.0;
                    let y = l.words.iter().map(|w| w.y).fold(f32::INFINITY, f32::min) / 2.0;
                    println!(
                        "price-like at x {:.0} y {:.0}: {}",
                        x + x0 as f32,
                        y + y0 as f32,
                        l.text
                    );
                }
            }
            let mut runs: Vec<(u32, u32, u32)> = Vec::new();
            for x in (f.w / 2)..f.w {
                let mut best = 0u32;
                let mut cur = 0u32;
                let mut at = 0u32;
                for y in (f.h / 8)..(f.h * 7 / 8) {
                    let (b, g, r) = f.px(x, y);
                    let (b, g, r) = (i32::from(b), i32::from(g), i32::from(r));
                    let red = r > 170 && g < 110 && b < 130 && r - g > 90;
                    let green = g > 110 && r < 100 && g - r > 60 && b > 80;
                    if red || green {
                        cur += 1;
                        if cur > best {
                            best = cur;
                            at = y;
                        }
                    } else {
                        cur = 0;
                    }
                }
                if best >= 12 {
                    runs.push((x, best, at));
                }
            }
            let mut i = 0;
            while i < runs.len() {
                let s0 = runs[i].0;
                let mut j = i;
                while j + 1 < runs.len() && runs[j + 1].0 == runs[j].0 + 1 {
                    j += 1;
                }
                println!(
                    "coloured band columns x {}..{} (tallest {} rows, ending y {})",
                    s0,
                    runs[j].0,
                    runs[i..=j].iter().map(|r| r.1).max().unwrap_or(0),
                    runs[i].2
                );
                i = j + 1;
            }
            f.wipe();
        }
        "overlay" => {
            // Does a capture of the chart window contain the overlay's pixels? The overlay's
            // panel is dark (about rgb 18,22,27); the chart here is light.
            let ov = xcap::Window::all().ok().into_iter().flatten().find(|w| {
                w.app_name()
                    .unwrap_or_default()
                    .contains("candlescope-overlay")
            });
            let Some(ov) = ov else {
                println!("overlay not running");
                return;
            };
            let (ox, oy, ow, oh) = (
                ov.x().unwrap_or(0),
                ov.y().unwrap_or(0),
                ov.width().unwrap_or(0),
                ov.height().unwrap_or(0),
            );
            let (wx, wy) = (win.x().unwrap_or(0), win.y().unwrap_or(0));
            let mut f = capture(&win).expect("capture");
            // The capture starts at the visible part of the window (no invisible border).
            let bx = (info.w.saturating_sub(f.w)) / 2;
            let dark = |x0: i32, y0: i32| {
                let (mut n, mut d) = (0u64, 0u64);
                for y in y0.max(0)..(y0 + oh as i32).min(f.h as i32) {
                    for x in x0.max(0)..(x0 + ow as i32).min(f.w as i32) {
                        let (b, g, r) = f.px(x as u32, y as u32);
                        let l = (u32::from(b) * 29 + u32::from(g) * 150 + u32::from(r) * 77) >> 8;
                        n += 1;
                        if l < 45 {
                            d += 1;
                        }
                    }
                }
                if n == 0 {
                    f64::NAN
                } else {
                    d as f64 / n as f64 * 100.0
                }
            };
            let ix = ox - wx - bx as i32;
            let iy = oy - wy - bx as i32;
            println!(
                "overlay at screen ({ox},{oy}) {ow}x{oh}, chart window at ({wx},{wy}); in the capture: dark pixels under the overlay {:.1} %, same-size area at the chart's left {:.1} %",
                dark(ix, iy),
                dark(60, iy)
            );
            f.wipe();
        }
        "read" => {
            let n: usize = args.get(1).and_then(|s| s.parse().ok()).unwrap_or(1);
            let mut r = Reader::new(cfg.clone(), eng);
            let pats = cfg.platforms[0].title_patterns.clone();
            let fresh = args.iter().any(|a| a == "--fresh");
            for i in 0..n {
                let title = win.title().unwrap_or_default();
                let t = cs_vision::parse::parse_title(&title, &pats);
                let mut f = capture(&win).expect("capture");
                if fresh {
                    // Cold read: no cached region results, every region is read again.
                    r.reset();
                }
                let x = r.read(&f, t.as_ref());
                f.wipe();
                let ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map_or(0, |d| d.as_millis());
                println!(
                    "{i:3} ts {ms} layout {} focused {} | {}",
                    r.layout,
                    win.is_focused().unwrap_or(false),
                    x.summary()
                );
                std::thread::sleep(std::time::Duration::from_millis(700));
            }
        }
        _ => {
            let crops = args
                .iter()
                .position(|a| a == "--crops")
                .and_then(|i| args.get(i + 1))
                .map(std::path::PathBuf::from);
            let t0 = Instant::now();
            let mut f = capture(&win).expect("capture");
            println!("capture {} in {:?}", stats(&f), t0.elapsed());
            for (name, r) in cfg.layouts[0].regions.all() {
                let c = f.crop(&r);
                let (x, y, w, h) = r.to_px(f.w, f.h);
                println!("== {name}: px ({x},{y}) {w}x{h}: {}", stats(&c));
                for (label, img) in [
                    ("bin200 x2", c.binarize(200).upscale(2)),
                    ("bin50 x2", c.binarize(50).upscale(2)),
                    ("gray x2", c.upscale(2)),
                    ("gray x3", c.upscale(3)),
                ] {
                    let t: Vec<String> = eng
                        .read(&img)
                        .unwrap_or_default()
                        .into_iter()
                        .map(|l| l.text)
                        .collect();
                    println!("   {label}: {t:?}");
                }
                if let Some(d) = &crops {
                    let _ = std::fs::create_dir_all(d);
                    let _ = std::fs::write(d.join(format!("{name}.ppm")), c.to_ppm());
                }
            }
            if let Some(b) =
                cs_vision::read::find_price_label(&f.crop(&cfg.layouts[0].regions.price_axis))
            {
                println!("price label rows in axis region: {b:?}");
            } else {
                println!("price label: no red/green band found");
            }
            f.wipe();
        }
    }
}
