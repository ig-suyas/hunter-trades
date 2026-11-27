//! Every word the panel shows comes from here, so the wording can be checked in one place.
//!
//! Rules (the GATE found 0 of 59 setups with an edge): describe the market, never suggest an
//! action. No "long", "short", "buy", "sell", "signal" (except "not a signal"), "probability",
//! "win", "score", "edge", "confidence" or certainty. Arrows give the direction of a move that
//! already happened; they carry text as well, so nothing relies on colour.

use cs_core::time;
use cs_live::snapshot::{HtfView, Marker, ZoneKind, ZoneView};
use cs_live::status::FeedStatus;

/// The footer, always visible (wording given by the user).
pub const FOOTER: &str = "No setup passed testing (0 of 59). Context only, not a signal. \
Random entries lost about 0.4 to 0.55 R per trade after costs.";

pub const APP_TITLE: &str = "Candlescope context";
pub const MUTE: &str = "MUTE";
pub const UNMUTE: &str = "UNMUTE";
pub const MORE: &str = "\u{25BE} More";
pub const LESS: &str = "\u{25B4} Less";
pub const DRAG_HINT: &str = "Drag here to move";
pub const RESIZE_HINT: &str = "Drag to resize";

pub const H_TREND: &str = "Trend";
pub const H_VOL: &str = "Volatility and session";
pub const H_ZONES: &str = "Nearest zones";
pub const H_EVENTS: &str = "Latest structure events";
pub const H_PULLBACK: &str = "Pullback";
pub const H_RISK: &str = "Risk calculator";
pub const H_SETTINGS: &str = "Settings";

pub const RISK_ACCOUNT: &str = "Account size (USDT)";
pub const RISK_PCT: &str = "Risk per trade (%)";
pub const RISK_ENTRY: &str = "Your entry price";
pub const RISK_STOP: &str = "Your stop price";
pub const RISK_FUNDING: &str = "Funding settlements held";
pub const RISK_NEED: &str = "Type your entry and stop prices to size a position.";
pub const RISK_BAD: &str = "Enter positive numbers; entry and stop must differ.";
pub const RISK_NOTE: &str = "Costs use the Phase 3 model: taker fee 5 bps per fill, half of a 1 bp spread and \
slippage of 1 bp + 0.02 ATR per market fill, funding at the current rate. It only calculates; it never places orders.";

/// Under the pullback line: the flag is a description, not an advantage.
pub const PULLBACK_NOTE: &str = "\"Healthy\" = smaller and slower than the move before it. None of the 60 healthy-pullback variants passed testing.";

pub const SET_OPACITY: &str = "Background opacity";
pub const SET_CLICK: &str = "Click-through (mouse passes to the chart)";
pub const SET_HOTKEYS: &str =
    "Hotkeys: Ctrl+Alt+H show/hide, Ctrl+Alt+M mute, Ctrl+Alt+T click-through";
pub const CLICK_ON: &str = "Click-through is ON: press Ctrl+Alt+T to use the panel again.";

pub const MUTED_BODY: &str = "Muted. Panel content is hidden; the feed keeps running.";
pub const MUTED_HOW: &str = "Press UNMUTE or Ctrl+Alt+M to show it again.";
pub const MUTED_SHORT: &str = "Muted: content hidden, feed still running. UNMUTE or Ctrl+Alt+M.";
pub const CLICK_ON_SHORT: &str = "Click-through ON: Ctrl+Alt+T to turn off.";

/// Feed status badge: a glyph and words (never colour alone).
pub fn status_badge(s: FeedStatus) -> String {
    let g = match s {
        FeedStatus::Live => "\u{25CF}",         // ●
        FeedStatus::Stale => "\u{25B2}",        // ▲
        FeedStatus::Disconnected => "\u{2715}", // ✕
        FeedStatus::Starting => "\u{25CB}",     // ○
    };
    format!("{g} {}", s.label())
}

pub fn no_data(note: &str, warm: usize) -> String {
    if note.is_empty() {
        format!("NO DATA yet ({warm} candles loaded).")
    } else {
        format!("NO DATA yet: {note}.")
    }
}

pub fn unsupported(sym: &str) -> String {
    format!(
        "UNSUPPORTED SYMBOL: {sym}. Supported: {} (Binance USDS-M perpetuals) on 5m, 15m and 1h. Pick one above.",
        cs_live::SUPPORTED_SYMBOLS.join(", ")
    )
}

/// One line for compact mode.
pub fn no_data_short(note: &str) -> String {
    let n = note.split(" (").next().unwrap_or(note);
    if n.is_empty() {
        "NO DATA yet: connecting.".to_string()
    } else {
        format!("NO DATA yet: {n}.")
    }
}

pub fn stale_short(age_s: Option<u64>, asof: &str) -> String {
    match age_s {
        Some(a) => format!("Silent {a} s; context as of {asof} UTC"),
        None => format!("Context as of {asof} UTC"),
    }
}

pub fn disconnected_short(asof: &str) -> String {
    format!("Reconnecting; context as of {asof} UTC")
}

pub fn stale_banner(age_s: Option<u64>, asof: &str) -> String {
    match age_s {
        Some(a) => format!(
            "FEED STALE: no data for {a} s. Context below is from the candle closed {asof} and may be out of date."
        ),
        None => format!(
            "FEED STALE. Context below is from the candle closed {asof} and may be out of date."
        ),
    }
}

pub fn disconnected_banner(note: &str, asof: &str) -> String {
    let n = if note.is_empty() {
        "reconnecting"
    } else {
        note
    };
    format!("DISCONNECTED ({n}). Context below is from the candle closed {asof}.")
}

pub fn drift_warning(offset_ms: i64) -> String {
    let dir = if offset_ms > 0 { "behind" } else { "ahead of" };
    format!(
        "\u{26A0} Clock check: this computer is {:.1} s {dir} the exchange. Candle times use exchange time.",
        f64::from(u32::try_from(offset_ms.unsigned_abs()).unwrap_or(u32::MAX)) / 1000.0
    )
}

/// `HH:MM:SS`.
pub fn hms(t: i64) -> String {
    let (_, _, _, h, m, s) = time::split(t);
    format!("{h:02}:{m:02}:{s:02}")
}

/// `MM-DD HH:MM` UTC.
pub fn md_hm(t: i64) -> String {
    let (_, mo, d, h, m, _) = time::split(t);
    format!("{mo:02}-{d:02} {h:02}:{m:02}")
}

pub fn clock(now: i64, local_offset_s: i64) -> String {
    format!(
        "{} UTC \u{00B7} {} local",
        hms(now),
        hms(now + local_offset_s * 1000)
    )
}

pub fn as_of(close_at: i64) -> String {
    format!("as of candle closed {} UTC", md_hm(close_at))
}

fn arrow(d: i8) -> &'static str {
    match d {
        1 => "\u{2191}",  // ↑
        -1 => "\u{2193}", // ↓
        _ => "\u{2194}",  // ↔
    }
}

pub fn stack(s: i8) -> &'static str {
    match s {
        1 => "\u{2191} EMA bull stack",
        -1 => "\u{2193} EMA bear stack",
        _ => "\u{2194} EMA stack off",
    }
}

pub fn structure(s: i8) -> &'static str {
    match s {
        1 => "\u{2191} structure up",
        -1 => "\u{2193} structure down",
        _ => "\u{2194} range",
    }
}

/// One higher time frame, e.g. "1h: ↑ EMA bull stack, ↔ range".
pub fn htf(h: &HtfView) -> String {
    if !h.active {
        return format!("{}: n/a on this time frame", h.label);
    }
    if h.candle_open == i64::MIN {
        return format!("{}: no completed candle yet", h.label);
    }
    let st = if h.ready {
        stack(h.stack)
    } else {
        "EMAs warming up"
    };
    format!("{}: {st}, {}", h.label, structure(h.structure))
}

/// Short form for compact mode, e.g. "1h: EMA (up arrow) bull, (up arrow) up".
pub fn htf_compact(h: &HtfView) -> String {
    if !h.active {
        return format!("{}: n/a", h.label);
    }
    if h.candle_open == i64::MIN || !h.ready {
        return format!("{}: warming up", h.label);
    }
    let st = match h.stack {
        1 => "\u{2191} bull",
        -1 => "\u{2193} bear",
        _ => "\u{2194} off",
    };
    let sr = match h.structure {
        1 => "\u{2191} up",
        -1 => "\u{2193} down",
        _ => "\u{2194} range",
    };
    format!("{}: EMA {st}, {sr}", h.label)
}

pub fn regime(r: i8, adx: f64) -> String {
    let a = if adx.is_finite() {
        format!("ADX {adx:.0}")
    } else {
        "ADX n/a".into()
    };
    match r {
        1 => format!("{a}: \u{2191} trending up"),
        -1 => format!("{a}: \u{2193} trending down"),
        _ => format!("{a}: \u{2194} ranging"),
    }
}

fn ordinal(n: u32) -> String {
    let suf = match (n % 10, n % 100) {
        (_, 11..=13) => "th",
        (1, _) => "st",
        (2, _) => "nd",
        (3, _) => "rd",
        _ => "th",
    };
    format!("{n}{suf}")
}

pub fn vol(p: f64) -> String {
    if p.is_finite() {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let n = (p * 100.0).round().clamp(0.0, 100.0) as u32;
        format!(
            "Volatility: {} percentile (ATR vs last 500 candles)",
            ordinal(n)
        )
    } else {
        "Volatility: warming up".into()
    }
}

pub fn vol_short(p: f64) -> String {
    if p.is_finite() {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let n = (p * 100.0).round().clamp(0.0, 100.0) as u32;
        format!("vol {} pct", ordinal(n))
    } else {
        "vol n/a".into()
    }
}

pub fn session(s: u8) -> &'static str {
    match s {
        0 => "Asia session (00-07 UTC)",
        1 => "London session (07-12 UTC)",
        2 => "London/New York overlap (12-16 UTC)",
        3 => "New York session (16-21 UTC)",
        _ => "Late session (21-24 UTC)",
    }
}

pub fn session_short(s: u8) -> &'static str {
    match s {
        0 => "Asia",
        1 => "London",
        2 => "LDN/NY",
        3 => "New York",
        _ => "Late",
    }
}

/// Price with thousands separators and sensible decimals.
pub fn price(p: f64) -> String {
    if !p.is_finite() {
        return "n/a".into();
    }
    let dec = if p >= 1000.0 {
        1
    } else if p >= 10.0 {
        2
    } else {
        4
    };
    let s = format!("{p:.dec$}");
    let (int, frac) = s.split_once('.').unwrap_or((&s, ""));
    let neg = int.starts_with('-');
    let digits: Vec<char> = int.trim_start_matches('-').chars().collect();
    let mut out = String::new();
    for (k, ch) in digits.iter().enumerate() {
        if k > 0 && (digits.len() - k).is_multiple_of(3) {
            out.push(',');
        }
        out.push(*ch);
    }
    let sign = if neg { "-" } else { "" };
    if frac.is_empty() {
        format!("{sign}{out}")
    } else {
        format!("{sign}{out}.{frac}")
    }
}

fn zone_name(k: ZoneKind) -> &'static str {
    match k {
        ZoneKind::Sr => "S/R",
        ZoneKind::Demand => "Demand",
        ZoneKind::Supply => "Supply",
    }
}

/// e.g. "S/R below: 91,200.0-91,450.0, 0.8 ATR ↓ (3 touches)".
pub fn zone(label: &str, z: &ZoneView) -> String {
    let where_ = match z.side {
        1 => format!("{:.1} ATR above \u{2191}", z.dist_atr),
        -1 => format!("{:.1} ATR below \u{2193}", z.dist_atr),
        _ => "price inside".to_string(),
    };
    let count = match z.kind {
        ZoneKind::Sr => format!("{} touches", z.count),
        _ if z.count == 0 => "fresh".to_string(),
        _ => format!("revisited {}x", z.count),
    };
    format!(
        "{label}: {}\u{2013}{}, {where_} ({count})",
        price(z.lo),
        price(z.hi)
    )
}

pub fn zone_none(label: &str) -> String {
    format!("{label}: none tracked")
}

pub fn zone_compact(z: &ZoneView) -> String {
    let d = match z.side {
        1 => format!("{:.1} ATR \u{2191}", z.dist_atr),
        -1 => format!("{:.1} ATR \u{2193}", z.dist_atr),
        _ => "inside".into(),
    };
    format!("{} {d}", zone_name(z.kind))
}

/// Neutral description of a structure event.
pub fn marker_name(m: &Marker) -> String {
    let a = arrow(m.dir);
    match m.kind {
        "sweep_high" => format!(
            "Sweep of a high {a}: traded above {}, closed back below",
            price(m.price)
        ),
        "sweep_low" => format!(
            "Sweep of a low {a}: traded below {}, closed back above",
            price(m.price)
        ),
        "bos_bull" => format!("BOS up {a} through {}", price(m.price)),
        "bos_bear" => format!("BOS down {a} through {}", price(m.price)),
        "choch_bull" => format!("CHoCH up {a} through {}", price(m.price)),
        "choch_bear" => format!("CHoCH down {a} through {}", price(m.price)),
        other => format!("{other} {a} at {}", price(m.price)),
    }
}

pub fn ago(bars: u32) -> String {
    match bars {
        0 => "latest candle".into(),
        1 => "1 candle ago".into(),
        n => format!("{n} candles ago"),
    }
}

pub fn marker(label: &str, m: Option<&Marker>, idx: u32) -> String {
    match m {
        Some(m) => format!(
            "{label}: {} (candle {} UTC, {})",
            marker_name(m),
            md_hm(m.known_at),
            ago(idx.saturating_sub(m.known_idx))
        ),
        None => format!("{label}: none since start"),
    }
}

pub fn pullback(dir: i8, healthy: bool, known_at: i64, bars: u32) -> String {
    let w = if dir > 0 {
        "\u{2191} within an up-move"
    } else {
        "\u{2193} within a down-move"
    };
    let h = if healthy {
        "flagged healthy"
    } else {
        "not flagged healthy"
    };
    format!(
        "Current pullback {w}: {h} (candle {} UTC, {})",
        md_hm(known_at),
        ago(bars)
    )
}

pub fn pullback_none(age: Option<u32>) -> String {
    match age {
        Some(a) => format!(
            "No current pullback (none in the last {} candles; the latest was {}).",
            cs_live::snapshot::PULLBACK_RECENT_BARS,
            ago(a)
        ),
        None => "No pullback detected since start.".into(),
    }
}

pub fn recorder(days: f64, hours: u32, last_age_s: Option<i64>) -> String {
    let w = match last_age_s {
        Some(a) if a < 180 => "recorder running".to_string(),
        Some(a) => format!("recorder silent for {} min", a / 60),
        None => "recorder status unknown".to_string(),
    };
    format!("Heatmap data recorded: {days:.1} days ({hours} h of order book and liquidations), {w}")
}

pub fn recorder_short(days: f64, last_age_s: Option<i64>) -> String {
    let w = match last_age_s {
        Some(a) if a < 180 => "running",
        Some(_) => "SILENT",
        None => "unknown",
    };
    format!("Recorder: {days:.1} d of heatmap data, {w}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use cs_live::snapshot::{HtfView, Marker, ZoneKind, ZoneView};

    /// Words that must never appear in the panel (case-insensitive), except in the exact
    /// phrase "not a signal".
    const FORBIDDEN: [&str; 21] = [
        "long",
        "short",
        "buy",
        "sell",
        "signal",
        "probab",
        "win",
        "score",
        "edge",
        "confiden",
        "certain",
        "guarantee",
        "target",
        "take profit",
        "opportunit",
        "recommend",
        "should",
        "bullish",
        "bearish",
        "odds",
        "chance",
    ];

    /// A banned word as the start of any word (so "wins", "longs", "signals" are caught but
    /// "window" is not), or a banned phrase anywhere.
    fn bad(s: &str) -> Option<&'static str> {
        let l = s.to_lowercase().replace("not a signal", "");
        let words: Vec<&str> = l
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !matches!(*w, "window" | "windows" | "windir"))
            .collect();
        FORBIDDEN.iter().copied().find(|f| {
            if f.contains(' ') {
                l.contains(f)
            } else {
                words.iter().any(|w| w.starts_with(f))
            }
        })
    }

    #[test]
    fn checker_catches_variants() {
        for s in [
            "2 wins",
            "Long setup",
            "SELL now",
            "signals",
            "high probability",
            "take profit at",
        ] {
            assert!(bad(s).is_some(), "{s}");
        }
        for s in [
            "window size",
            "Context only, not a signal.",
            "position size",
            "stop-out",
        ] {
            assert!(bad(s).is_none(), "{s}");
        }
    }

    fn all_generated() -> Vec<String> {
        let mut v: Vec<String> = Vec::new();
        for s in [
            FeedStatus::Live,
            FeedStatus::Stale,
            FeedStatus::Disconnected,
            FeedStatus::Starting,
        ] {
            v.push(status_badge(s));
        }
        for st in [-1i8, 0, 1] {
            for sr in [-1i8, 0, 1] {
                for ready in [false, true] {
                    for active in [false, true] {
                        let h = HtfView {
                            label: "4h",
                            active,
                            ready,
                            stack: st,
                            structure: sr,
                            candle_open: 0,
                        };
                        v.push(htf(&h));
                        v.push(htf_compact(&h));
                    }
                }
            }
            v.push(regime(st, 27.0));
            v.push(pullback(st, true, 0, 3));
            v.push(pullback(st, false, 0, 3));
        }
        for k in [
            "sweep_high",
            "sweep_low",
            "bos_bull",
            "bos_bear",
            "choch_bull",
            "choch_bear",
        ] {
            let m = Marker {
                kind: k,
                dir: 1,
                price: 91_000.0,
                known_at: 0,
                known_idx: 3,
            };
            v.push(marker("BOS", Some(&m), 5));
        }
        for kind in [ZoneKind::Sr, ZoneKind::Demand, ZoneKind::Supply] {
            for side in [-1i8, 0, 1] {
                let z = ZoneView {
                    kind,
                    lo: 1.0,
                    hi: 2.0,
                    side,
                    dist_atr: 0.5,
                    count: 2,
                };
                v.push(zone("Nearest", &z));
                v.push(zone_compact(&z));
            }
        }
        for s in 0..5 {
            v.push(session(s).into());
            v.push(session_short(s).into());
        }
        v.extend([
            vol(0.63),
            vol_short(0.63),
            no_data("", 0),
            no_data_short("warming up (x)"),
            stale_short(Some(4), "x"),
            disconnected_short("x"),
            MUTED_SHORT.into(),
            CLICK_ON_SHORT.into(),
            unsupported("NVDA"),
            stale_banner(Some(40), "x"),
            disconnected_banner("", "x"),
            drift_warning(1500),
            pullback_none(Some(40)),
            recorder(0.5, 12, Some(30)),
            recorder_short(0.5, Some(30)),
            FOOTER.into(),
            PULLBACK_NOTE.into(),
        ]);
        v
    }

    #[test]
    fn generated_copy_has_no_signal_words() {
        for s in all_generated() {
            assert!(bad(&s).is_none(), "forbidden word {:?} in {s:?}", bad(&s));
        }
    }

    /// Every string literal in the UI sources (outside test modules) passes the same check.
    #[test]
    fn source_literals_have_no_signal_words() {
        let files = [
            ("copy.rs", include_str!("copy.rs")),
            ("view.rs", include_str!("view.rs")),
            ("app.rs", include_str!("app.rs")),
            ("risk.rs", include_str!("risk.rs")),
            ("main.rs", include_str!("main.rs")),
            ("demo.rs", include_str!("demo.rs")),
        ];
        for (name, src) in files {
            let code = src.split("#[cfg(test)]").next().unwrap_or(src);
            // Comments are not shown in the panel.
            let body: String = code
                .lines()
                .filter(|l| !l.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join(
                    "
",
                );
            let mut lits = Vec::new();
            let mut cur = String::new();
            let mut inside = false;
            let mut esc = false;
            for ch in body.chars() {
                if inside {
                    if esc {
                        esc = false;
                    } else if ch == '\\' {
                        esc = true;
                    } else if ch == '"' {
                        inside = false;
                        lits.push(std::mem::take(&mut cur));
                        continue;
                    }
                    cur.push(ch);
                } else if ch == '"' {
                    inside = true;
                }
            }
            for l in lits {
                // Exchange stream names and detector kinds are not shown as such.
                if l.contains('@')
                    || l.starts_with("bos_")
                    || l.starts_with("choch_")
                    || l.starts_with("sweep_")
                {
                    continue;
                }
                assert!(
                    bad(&l).is_none(),
                    "{name}: forbidden word {:?} in literal {l:?}",
                    bad(&l)
                );
            }
        }
    }

    #[test]
    fn prices() {
        assert_eq!(price(91_234.56), "91,234.6");
        assert_eq!(price(3_012.0), "3,012.0");
        assert_eq!(price(123.456), "123.46");
        assert_eq!(price(1_234_567.0), "1,234,567.0");
        assert_eq!(ordinal(11), "11th");
        assert_eq!(ordinal(63), "63rd");
        assert_eq!(ordinal(21), "21st");
    }
}
