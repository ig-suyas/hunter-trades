//! CPU benchmark of the watcher (measure the process from outside, e.g. PowerShell
//! `(Get-Process watch_bench).TotalProcessorTime`).
//!
//! `watch_bench scripted <secs>`: a `TradingView` window in front for the whole run, its price
//!   label changing every 500 ms (two trading1.mp4 frames), read at 2 fps with real OCR.
//! `watch_bench poll <secs>`: the real window list (xcap), with capture disabled: measures the
//!   cost of checking which window is in front. No pixels are captured in this mode.
#![allow(clippy::print_stdout, clippy::unwrap_used, clippy::expect_used)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use cs_vision::source::{FrameSource, ScriptedSource, WindowInfo, XcapSource};
use cs_vision::{Frame, VisionConfig, Watcher};

/// The real window list, but `capture` always refuses.
struct PollOnly(XcapSource);
impl FrameSource for PollOnly {
    fn foreground(&mut self) -> Option<WindowInfo> {
        self.0.foreground()
    }
    fn capture(&mut self, _w: &WindowInfo) -> Option<Frame> {
        None
    }
}

fn main() {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let mode = a.first().map_or("scripted", String::as_str).to_string();
    let secs: u64 = a.get(1).and_then(|s| s.parse().ok()).unwrap_or(60);
    let cfg = VisionConfig::default_config().unwrap();
    let w = if mode == "poll" {
        Watcher::spawn(
            cfg,
            || Box::new(PollOnly(XcapSource::new())) as Box<dyn FrameSource>,
            None,
            true,
        )
    } else {
        let v = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../trading1.mp4");
        let f1 = Arc::new(cs_vision::video::frame_at(&v, 5_000).unwrap());
        let f2 = Arc::new(cs_vision::video::frame_at(&v, 10_000).unwrap());
        let title = "BTCUSD 60,626.47 \u{25B2} +0.41% Unnamed - Google Chrome";
        let win = WindowInfo {
            id: 7,
            title: title.into(),
            app: "Google Chrome".into(),
            w: 1920,
            h: 1080,
        };
        let steps: Vec<_> = (0..secs * 2)
            .map(|k| {
                let f = if k % 2 == 0 { &f1 } else { &f2 };
                (Duration::from_millis(500 * k), win.clone(), Arc::clone(f))
            })
            .collect();
        Watcher::spawn(
            cfg,
            move || {
                Box::new(ScriptedSource::new(steps, Duration::from_millis(40)))
                    as Box<dyn FrameSource>
            },
            None,
            true,
        )
    };
    let t0 = Instant::now();
    std::thread::sleep(Duration::from_secs(secs));
    let s = w.state();
    println!(
        "mode {mode}: {:.1} s, status {:?}, polls {}, frames read {}, OCR calls {}, last {}",
        t0.elapsed().as_secs_f64(),
        s.status,
        s.polls,
        s.frames,
        s.ocr_calls,
        s.reading.map_or_else(|| "none".into(), |r| r.summary())
    );
}
