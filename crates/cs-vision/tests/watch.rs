//! Symbol-change detection through the real watcher thread and real OCR, with a scripted screen:
//! the window in front switches between a BTCUSD 15m frame (trading1.mp4) and an XAUUSD 4h frame
//! (trading.mp4) every 2 s. The scripted capture sleeps 40 ms to stand in for a real window
//! capture. `-- --nocapture` prints the latencies.

#![allow(clippy::print_stdout, clippy::unwrap_used, clippy::expect_used)]

mod common;

use std::sync::Arc;
use std::time::{Duration, Instant};

use cs_vision::read::SymbolRead;
use cs_vision::source::{FrameSource, ScriptedSource, WindowInfo};
use cs_vision::watch::WatchStatus;
use cs_vision::{Frame, VisionConfig, Watcher};

const BTC_TITLE: &str = "BTCUSD 60,626.47 \u{25B2} +0.41% Unnamed - Google Chrome";
const XAU_TITLE: &str = "XAUUSD 3,566.25 \u{25BC} \u{2212}0.65% Unnamed - Google Chrome";

fn info(title: &str) -> WindowInfo {
    // Same window and size throughout, so only the title or the pixels can trigger a read.
    WindowInfo {
        id: 7,
        title: title.into(),
        app: "Google Chrome".into(),
        w: 1920,
        h: 1080,
    }
}

fn ticker(r: Option<&cs_vision::Reading>) -> Option<String> {
    r.and_then(|r| r.symbol.ticker().map(str::to_string))
}

/// Runs the script and returns the detection latency of each switch.
fn run(btc: &Arc<Frame>, xau: &Arc<Frame>, titles: (&str, &str), switches: u32) -> Vec<f64> {
    let period = Duration::from_millis(2_000);
    let mut steps = Vec::new();
    for k in 0..=switches {
        let (t, f) = if k % 2 == 0 {
            (titles.0, btc)
        } else {
            (titles.1, xau)
        };
        steps.push((period * k, info(t), Arc::clone(f)));
    }
    let start = Instant::now();
    let cfg = VisionConfig::default_config().unwrap();
    let w = Watcher::spawn(
        cfg,
        move || {
            let mut s = ScriptedSource::new(steps, Duration::from_millis(40));
            s.t0 = start;
            Box::new(s) as Box<dyn FrameSource>
        },
        None,
        true,
    );
    let mut lat = Vec::new();
    for k in 1..=switches {
        let want = if k % 2 == 0 { "BTCUSD" } else { "XAUUSD" };
        let at = start + period * k;
        loop {
            let s = w.state();
            if let (Some(t), Some(r_at)) = (ticker(s.reading.as_ref()), s.reading_at)
                && t == want
                && r_at >= at
            {
                lat.push((r_at - at).as_secs_f64() * 1000.0);
                break;
            }
            assert!(
                at.elapsed() < Duration::from_secs(5),
                "switch {k} not noticed"
            );
            std::thread::sleep(Duration::from_millis(2));
        }
    }
    let s = w.state();
    assert_eq!(s.status, WatchStatus::Reading);
    println!(
        "frames read {}, OCR calls {}, polls {} in {:.1} s",
        s.frames,
        s.ocr_calls,
        s.polls,
        start.elapsed().as_secs_f64()
    );
    lat
}

#[test]
fn symbol_change_noticed_quickly() {
    let (Some(b), Some(x)) = (common::trading1(5_000), common::trading_xau(60_000)) else {
        return;
    };
    let (btc, xau) = (Arc::new(b), Arc::new(x));
    // Browser tab: the ticker in the window title changes with the symbol.
    let lat = run(&btc, &xau, (BTC_TITLE, XAU_TITLE), 6);
    let max = lat.iter().copied().fold(0.0, f64::max);
    println!("title changes with the symbol: latencies ms {lat:.0?}, max {max:.0}");
    assert!(max < 500.0, "symbol change took {max:.0} ms");
    // A window whose title does not change (e.g. a desktop app): only the 2 fps capture
    // notices, so the bound is one frame interval plus one read.
    let lat = run(&btc, &xau, ("TradingView", "TradingView"), 4);
    let max = lat.iter().copied().fold(0.0, f64::max);
    println!("title unchanged: latencies ms {lat:.0?}, max {max:.0}");
    assert!(max < 1_000.0, "symbol change took {max:.0} ms");
}

#[test]
fn off_means_no_capture_and_no_reading() {
    let f = Arc::new(Frame::from_bgra(64, 64, vec![255; 64 * 64 * 4]).unwrap());
    let steps = vec![(Duration::ZERO, info(BTC_TITLE), f)];
    let w = Watcher::spawn(
        VisionConfig::default_config().unwrap(),
        move || Box::new(ScriptedSource::new(steps, Duration::ZERO)) as Box<dyn FrameSource>,
        None,
        false,
    );
    std::thread::sleep(Duration::from_millis(400));
    let s = w.state();
    assert_eq!(s.status, WatchStatus::Off);
    assert_eq!(
        (s.frames, s.polls),
        (0, 0),
        "nothing polled or captured while off"
    );
    w.set_enabled(true);
    std::thread::sleep(Duration::from_millis(400));
    assert!(w.state().polls > 0);
    w.set_enabled(false);
    std::thread::sleep(Duration::from_millis(200));
    let s = w.state();
    assert_eq!(s.status, WatchStatus::Off);
    assert!(s.reading.is_none(), "turning it off clears the reading");
}

#[test]
fn other_windows_are_never_captured() {
    let f = Arc::new(Frame::from_bgra(64, 64, vec![255; 64 * 64 * 4]).unwrap());
    let mut mail = info("Inbox - Mail - Google Chrome");
    mail.app = "Google Chrome".into();
    let steps = vec![(Duration::ZERO, mail, f)];
    let w = Watcher::spawn(
        VisionConfig::default_config().unwrap(),
        move || Box::new(ScriptedSource::new(steps, Duration::ZERO)) as Box<dyn FrameSource>,
        None,
        true,
    );
    std::thread::sleep(Duration::from_millis(600));
    let s = w.state();
    assert_eq!(s.status, WatchStatus::Waiting);
    assert!(s.polls >= 2);
    assert_eq!(s.frames, 0, "a non-platform window must not be captured");
    assert!(matches!(
        s.reading.map(|r| r.symbol),
        None | Some(SymbolRead::Unknown)
    ));
}
