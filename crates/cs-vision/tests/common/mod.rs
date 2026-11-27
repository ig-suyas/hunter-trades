//! Fixture frames: extracted with ffmpeg from the input videos straight into memory (the videos
//! are git-ignored inputs in the project folder; the frames are never written to disk).

#![allow(dead_code)]

use std::path::PathBuf;
use std::sync::Arc;

use cs_vision::Frame;
use cs_vision::source::WindowInfo;

pub fn video(name: &str) -> Option<PathBuf> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(name);
    if p.exists() {
        Some(p)
    } else {
        eprintln!("SKIP: {name} not in the project folder (fixture video missing)");
        None
    }
}

/// trading1.mp4 is a full-screen recording (1920 x 1080): the maximised browser window is the
/// top 1920 x 1020 (the same size as the user's real window); the bottom 60 rows are the
/// Windows taskbar, which a window capture never contains, so they are dropped.
pub fn trading1(t_ms: u64) -> Option<Frame> {
    let p = video("trading1.mp4")?;
    let f = cs_vision::video::frame_at(&p, t_ms).expect("ffmpeg frame");
    Some(f.crop_px(0, 0, 1920, 1020))
}

/// trading.mp4 is a screen share inside a video call: the shared browser window (1920 x 1020
/// on the sharer's screen, scaled by 0.887) sits at (140, 61), 1703 x 905 px. Only this
/// rectangle is kept; the call window around it is dropped at once.
pub fn trading_xau(t_ms: u64) -> Option<Frame> {
    let p = video("trading.mp4")?;
    let f = cs_vision::video::frame_at(&p, t_ms).expect("ffmpeg frame");
    Some(f.crop_px(140, 61, 1703, 905))
}

pub fn window(id: u32, title: &str, f: &Frame) -> WindowInfo {
    WindowInfo {
        id,
        title: title.into(),
        app: "Google Chrome".into(),
        w: f.w,
        h: f.h,
        behind_overlay: false,
    }
}

pub fn arc(f: Frame) -> Arc<Frame> {
    Arc::new(f)
}

/// Binance USDS-M BTCUSDT 1m closes, 2026-11-26 20:00..20:05 UTC (public klines endpoint).
/// trading1.mp4 starts at 20:00:10 UTC (`TradingView`'s clock in the first frame).
pub const BTCUSDT_1M_CLOSE: [f64; 6] = [60_659.2, 60_709.3, 60_711.6, 60_694.2, 60_696.0, 60_684.1];
/// ETHUSDT closes for the same minutes (for the mismatch case).
pub const ETHUSDT_1M_CLOSE: [f64; 6] = [1_702.54, 1_704.0, 1_704.15, 1_703.28, 1_703.58, 1_703.23];

/// Feed price for a trading1 frame: the close of the minute the frame falls in.
pub fn btc_feed_at(t_ms: u64) -> f64 {
    let s = 10 + t_ms / 1000;
    BTCUSDT_1M_CLOSE[usize::try_from(s / 60).unwrap_or(0).min(5)]
}
