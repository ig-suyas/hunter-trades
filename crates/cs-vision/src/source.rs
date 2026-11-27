//! Where frames come from. The real source captures **only the window in front, and only when
//! its program and title match a configured platform**; every other window is never captured.
//!
//! "In front" is the focused window, except when the focus is on this program's own window
//! (the overlay takes the focus when it starts and whenever it is clicked): then it is the first
//! normal window below the overlay in z-order, i.e. the window the user was looking at. That
//! window too is captured only if it is a platform window.

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
    /// The window sits directly below this program's own (focused) overlay.
    pub behind_overlay: bool,
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
            || parse::parse_title(&w.title, &p.title_patterns).is_some();
        app_ok && title_ok
    })
}

/// What [`pick_front`] needs to know about one top-level window (z-order: first = top).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Candidate {
    pub pid: u32,
    pub focused: bool,
    pub minimized: bool,
    pub w: u32,
    pub h: u32,
}

/// Index of the window "in front", and whether it was found below this program's own window.
/// The focused window counts, unless it belongs to `own_pid`: then the first window below it in
/// z-order that is not ours, not minimised and at least 300 x 200 px (so the taskbar and tool
/// strips do not count).
pub fn pick_front(ws: &[Candidate], own_pid: u32) -> Option<(usize, bool)> {
    let f = ws.iter().position(|c| c.focused)?;
    if ws[f].pid != own_pid {
        return (!ws[f].minimized).then_some((f, false));
    }
    ws.iter()
        .enumerate()
        .skip(f + 1)
        .find(|(_, c)| c.pid != own_pid && !c.minimized && c.w >= 300 && c.h >= 200)
        .map(|(i, _)| (i, true))
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
        let own = std::process::id();
        let cands: Vec<Candidate> = all
            .iter()
            .map(|w| Candidate {
                pid: w.pid().unwrap_or(0),
                focused: w.is_focused().unwrap_or(false),
                minimized: w.is_minimized().unwrap_or(true),
                w: w.width().unwrap_or(0),
                h: w.height().unwrap_or(0),
            })
            .collect();
        let (i, behind_overlay) = pick_front(&cands, own)?;
        let w = all.into_iter().nth(i)?;
        let pid = cands[i].pid;
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
            behind_overlay,
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
            h: 1020,
            behind_overlay: false,
        }
    }

    #[test]
    fn only_platform_windows_match() {
        let c = VisionConfig::default_config().unwrap();
        let yes = [
            // Seen on this machine: xcap's program name and the title of the Chrome chart tab
            // (BITSTAMP:BTCUSD, daily).
            win(
                "BTCUSD 61,515 \u{25B2} +1.69% - Google Chrome",
                "Google Chrome",
            ),
            win(
                "BTCUSD 61,521 \u{25B2} +1.7% - Google Chrome",
                "Google Chrome",
            ),
            win(
                "BTCUSD 60,626.47 \u{25B2} +0.41% Unnamed - Google Chrome",
                "Google Chrome",
            ),
            win(
                "ETHUSDT.P 1,704.15 \u{25BC} \u{2212}0.12% Unnamed and 2 more pages - Personal - Microsoft\u{200B} Edge",
                "Microsoft Edge",
            ),
            win(
                "BTCUSDT 61,480.2 \u{25BC} -0.35% \u{2014} Mozilla Firefox",
                "Firefox",
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
            win("BTCUSD 61,515 \u{25B2} +1.69% - Google Chrome", "Claude"),
            win("candlescope-handoff.md - Visual Studio Code", "Code"),
            win("Candlescope context", "candlescope-overlay"),
            win("Binance Futures", "Google Chrome"),
            win(
                "60,728.6 BTCUSDT +1.4% | Binance Futures - Google Chrome",
                "Google Chrome",
            ),
            win("NEWS 2024 - Google Chrome", "Google Chrome"),
        ];
        for w in &no {
            assert!(match_platform(&c, w).is_none(), "{w:?}");
        }
    }

    fn cand(pid: u32, focused: bool, minimized: bool, w: u32, h: u32) -> Candidate {
        Candidate {
            pid,
            focused,
            minimized,
            w,
            h,
        }
    }

    /// The case seen on this machine: the overlay (pid 7) has the focus right after it starts;
    /// the taskbar is above it, the Claude app and the Chrome chart below it.
    #[test]
    fn the_overlay_in_front_is_looked_through() {
        let ws = [
            cand(1, false, false, 1920, 60),   // taskbar (topmost)
            cand(7, true, false, 550, 925),    // overlay, focused
            cand(2, false, false, 1920, 1020), // window right below the overlay
            cand(3, false, false, 1920, 1020), // Chrome further down
        ];
        assert_eq!(pick_front(&ws, 7), Some((2, true)));
        // The window right below is minimised: the next one counts.
        let ws2 = [ws[0], ws[1], cand(2, false, true, 1920, 1020), ws[3]];
        assert_eq!(pick_front(&ws2, 7), Some((3, true)));
        // Another program has the focus: that window, as before.
        let ws3 = [
            ws[0],
            cand(7, false, false, 550, 925),
            cand(3, true, false, 1920, 1020),
        ];
        assert_eq!(pick_front(&ws3, 7), Some((2, false)));
        // Only small strips below the overlay: nothing.
        let ws4 = [
            cand(7, true, false, 550, 925),
            cand(1, false, false, 1920, 60),
        ];
        assert_eq!(pick_front(&ws4, 7), None);
        // A minimised focused window: nothing.
        assert_eq!(pick_front(&[cand(3, true, true, 1920, 1020)], 7), None);
        assert_eq!(pick_front(&[], 7), None);
    }
}
