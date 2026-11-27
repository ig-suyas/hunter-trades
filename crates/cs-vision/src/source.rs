//! Where frames come from. The real source captures **only the foreground window, and only when
//! its title and program match a configured platform**; every other window is never captured.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use crate::config::{Platform, VisionConfig};
use crate::frame::Frame;
use crate::parse;

/// The foreground window (title and program name only; no pixels).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WindowInfo {
    pub id: u32,
    pub title: String,
    /// Program name or description ("Google Chrome", "msedge").
    pub app: String,
    pub w: u32,
    pub h: u32,
}

pub trait FrameSource {
    /// The window in front, or `None`.
    fn foreground(&mut self) -> Option<WindowInfo>;
    /// Captures that window (the caller has already checked it is a platform window).
    fn capture(&mut self, w: &WindowInfo) -> Option<Frame>;
}

/// The platform a window belongs to, if any.
pub fn match_platform<'a>(cfg: &'a VisionConfig, w: &WindowInfo) -> Option<&'a Platform> {
    let app = w.app.to_lowercase();
    let title = w.title.to_lowercase();
    cfg.platforms.iter().find(|p| {
        let app_ok = p.apps.iter().any(|a| app.contains(a.as_str()));
        let title_ok = p
            .title_contains
            .iter()
            .any(|t| title.contains(&t.to_lowercase()))
            || (p.title_chart_pattern && parse::parse_title(&w.title).is_some());
        app_ok && title_ok
    })
}

/// Real windows through `xcap` (`EnumWindows` / `GetForegroundWindow` / `PrintWindow` inside the
/// crate). Not `Send`: create it on the thread that uses it.
#[derive(Default)]
pub struct XcapSource {
    front: Option<xcap::Window>,
    /// Program name per process id (reading it opens the executable's version block).
    apps: HashMap<u32, String>,
}

impl XcapSource {
    pub fn new() -> XcapSource {
        XcapSource::default()
    }
}

impl FrameSource for XcapSource {
    fn foreground(&mut self) -> Option<WindowInfo> {
        self.front = None;
        let all = xcap::Window::all().ok()?;
        let w = all.into_iter().find(|w| w.is_focused().unwrap_or(false))?;
        if w.is_minimized().unwrap_or(true) {
            return None;
        }
        let pid = w.pid().ok()?;
        let app = if let Some(a) = self.apps.get(&pid) {
            a.clone()
        } else {
            let a = w.app_name().unwrap_or_default();
            if self.apps.len() > 256 {
                self.apps.clear();
            }
            self.apps.insert(pid, a.clone());
            a
        };
        let info = WindowInfo {
            id: w.id().ok()?,
            title: w.title().unwrap_or_default(),
            app,
            w: w.width().ok()?,
            h: w.height().ok()?,
        };
        self.front = Some(w);
        Some(info)
    }

    fn capture(&mut self, info: &WindowInfo) -> Option<Frame> {
        let w = self.front.as_ref()?;
        if w.id().ok()? != info.id {
            return None;
        }
        let img = w.capture_image().ok()?;
        let (iw, ih) = (img.width(), img.height());
        Frame::from_rgba(iw, ih, img.into_raw())
    }
}

/// A scripted screen for tests and benchmarks: from each start time on, the given window and
/// frame are "in front". `capture_delay` stands in for the cost of a real capture.
pub struct ScriptedSource {
    pub steps: Vec<(Duration, WindowInfo, Arc<Frame>)>,
    pub t0: Instant,
    pub capture_delay: Duration,
    pub captures: u64,
}

impl ScriptedSource {
    pub fn new(steps: Vec<(Duration, WindowInfo, Arc<Frame>)>, capture_delay: Duration) -> Self {
        ScriptedSource {
            steps,
            t0: Instant::now(),
            capture_delay,
            captures: 0,
        }
    }

    fn current(&self) -> Option<&(Duration, WindowInfo, Arc<Frame>)> {
        let e = self.t0.elapsed();
        self.steps.iter().rev().find(|(s, _, _)| *s <= e)
    }
}

impl FrameSource for ScriptedSource {
    fn foreground(&mut self) -> Option<WindowInfo> {
        self.current().map(|(_, w, _)| w.clone())
    }

    fn capture(&mut self, w: &WindowInfo) -> Option<Frame> {
        std::thread::sleep(self.capture_delay);
        self.captures += 1;
        let (_, cw, f) = self.current()?;
        // A capture returns a fresh buffer, like the real one.
        (cw.id == w.id).then(|| (**f).clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn win(title: &str, app: &str) -> WindowInfo {
        WindowInfo {
            id: 1,
            title: title.into(),
            app: app.into(),
            w: 1920,
            h: 1080,
        }
    }

    #[test]
    fn only_platform_windows_match() {
        let c = VisionConfig::default_config().unwrap();
        let yes = [
            win(
                "BTCUSD 60,626.47 \u{25B2} +0.41% Unnamed - Google Chrome",
                "Google Chrome",
            ),
            win("TradingView \u{2014} Track All Markets", "Microsoft Edge"),
            win(
                "ETHUSDT.P 1,704.15 \u{25BC} \u{2212}0.12% Unnamed",
                "TradingView",
            ),
        ];
        for w in &yes {
            assert!(match_platform(&c, w).is_some(), "{w:?}");
        }
        let no = [
            win("Inbox - Mail - Google Chrome", "Google Chrome"),
            win("BTCUSD 60,626.47 \u{25B2} +0.41% Unnamed", "Notepad"),
            win("candlescope-handoff.md - Visual Studio Code", "Code"),
            win("Candlescope context", "candlescope-overlay"),
            win("Binance Futures", "Google Chrome"),
        ];
        for w in &no {
            assert!(match_platform(&c, w).is_none(), "{w:?}");
        }
    }
}
