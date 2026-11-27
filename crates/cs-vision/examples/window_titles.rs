//! Development aid: lists the windows `xcap` sees (program name, focus flag and title; no
//! pixels). Titles of non-browser windows, and browser titles that do not look like a chart tab,
//! are redacted so nothing personal is printed.
#![allow(clippy::print_stdout)]

fn chart_like(t: &str) -> bool {
    let l = t.to_lowercase();
    l.contains("tradingview")
        || t.split_whitespace().next().is_some_and(|w| {
            w.len() >= 3
                && w.chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || ".!:_-".contains(c))
        })
}

fn main() {
    let cfg = cs_vision::VisionConfig::default_config().ok();
    let own = std::process::id();
    let Ok(all) = xcap::Window::all() else {
        println!("window list unavailable");
        return;
    };
    for w in all {
        let app = w.app_name().unwrap_or_default();
        let title = w.title().unwrap_or_default();
        let focused = w.is_focused().unwrap_or(false);
        let min = w.is_minimized().unwrap_or(false);
        let browser = [
            "chrome",
            "edge",
            "firefox",
            "brave",
            "opera",
            "vivaldi",
            "tradingview",
        ]
        .iter()
        .any(|b| app.to_lowercase().contains(b));
        let shown = if browser && chart_like(&title) {
            format!("{title:?}")
        } else {
            // Shape only: lower-case letters masked as "a", the rest kept.
            let shape: String = title
                .chars()
                .map(|c| if c.is_lowercase() { 'a' } else { c })
                .collect();
            if browser {
                format!("[shape {shape:?}]")
            } else {
                format!("[redacted, {} chars]", title.chars().count())
            }
        };
        let info = cs_vision::source::WindowInfo {
            id: w.id().unwrap_or(0),
            title: title.clone(),
            app: app.clone(),
            w: w.width().unwrap_or(0),
            h: w.height().unwrap_or(0),
            behind_overlay: false,
        };
        let m = cfg
            .as_ref()
            .and_then(|c| cs_vision::source::match_platform(c, &info))
            .map_or("no".to_string(), |p| p.name.clone());
        let pid = w.pid().unwrap_or(0);
        println!(
            "{}{}{} match={m} app={app:?} size={}x{} title={shown}",
            if focused { "FOCUSED " } else { "" },
            if min { "MINIMISED " } else { "" },
            if pid == own { "OWN " } else { "" },
            w.width().unwrap_or(0),
            w.height().unwrap_or(0),
        );
    }
}
