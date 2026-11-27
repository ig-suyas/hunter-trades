//! Every word the panel shows comes from here, so the wording can be checked in one place.
//!
//! Rules (the GATE found 0 of 59 setups with an edge): describe the market, never suggest an
//! action. No "long", "short", "buy", "sell", "signal" (except "not a signal"), "probability",
//! "win", "score", "edge", "confidence" or certainty. Arrows give the direction of a move that
//! already happened; they carry text as well, so nothing relies on colour.

use cs_core::{Timeframe, time};
use cs_live::snapshot::{HtfView, Marker, ZoneKind, ZoneView};
use cs_live::status::FeedStatus;

/// Net result of random entries (stop 1 ATR, target 1 R, Binance USDS-M costs) per symbol and
/// time frame, R per trade, from `reports/gate-report.md` Appendix C (random-entry control):
/// (symbol, time frame, discovery period, validation period).
pub const RANDOM_ENTRY_R: [(&str, Timeframe, f64, f64); 6] = [
    ("BTCUSDT", Timeframe::M5, -1.1004, -1.0099),
    ("BTCUSDT", Timeframe::M15, -0.5668, -0.5326),
    ("BTCUSDT", Timeframe::H1, -0.2314, -0.3296),
    ("ETHUSDT", Timeframe::M5, -0.5882, -0.7437),
    ("ETHUSDT", Timeframe::M15, -0.3358, -0.3919),
    ("ETHUSDT", Timeframe::H1, -0.1167, -0.2453),
];

/// The footer, always visible (wording given by the user), with the random-entry loss for the
/// symbol and time frame on screen.
pub fn footer(symbol: &str, tf: Timeframe) -> String {
    let head = "No setup passed testing (0 of 59). Context only, not a signal.";
    match RANDOM_ENTRY_R
        .iter()
        .find(|(s, t, _, _)| *s == symbol && *t == tf)
    {
        Some((s, t, a, b)) => {
            let (lo, hi) = (a.abs().min(b.abs()), a.abs().max(b.abs()));
            format!(
                "{head} Random entries on {} {} lost {lo:.2} to {hi:.2} R per trade after costs.",
                s.trim_end_matches("USDT"),
                t.label()
            )
        }
        None => format!(
            "{head} Random entries lost 0.12 to 1.10 R per trade after costs (BTC and ETH, 5m to 1h)."
        ),
    }
}

/// Screen reading toggle and states.
pub const SCREEN_ON: &str = "\u{25C9} Screen reading ON";
pub const SCREEN_OFF: &str = "\u{25CB} Screen reading off";
pub const SCREEN_TIP: &str = "Reads symbol, time frame and last price from the TradingView window in front \
(small regions, in memory, nothing saved or sent). Ctrl+Alt+S";
pub const FOLLOW_SCREEN: &str = "Follow screen";

/// One line under the header about what the screen shows.
pub fn screen_line(state: &cs_vision::WatchState, manual: bool) -> String {
    use cs_vision::read::{PageKind, SymbolRead};
    use cs_vision::watch::WatchStatus;
    if manual {
        return "Manual pick: the screen is ignored.".into();
    }
    match state.status {
        WatchStatus::Off => "Screen reading is off.".into(),
        WatchStatus::NoEngine => "Screen reading unavailable: no OCR engine.".into(),
        WatchStatus::Waiting => "TradingView not in front.".into(),
        WatchStatus::Reading => match &state.reading {
            None => "Reading the TradingView window\u{2026}".into(),
            Some(r) if r.page != PageKind::Chart => "TradingView: no chart on this page.".into(),
            Some(r) => {
                // The failure states first, each in its own words.
                let mut problems: Vec<String> = Vec::new();
                let sym = match &r.symbol {
                    SymbolRead::Unknown => {
                        problems.push("symbol not read".into());
                        None
                    }
                    SymbolRead::Supported { ticker, .. } | SymbolRead::Unsupported { ticker } => {
                        Some(ticker.clone())
                    }
                };
                match &r.tf {
                    None => problems.push("time frame not read".into()),
                    Some(t) if !t.tf.is_some_and(|x| cs_live::SUPPORTED_TFS.contains(&x)) => {
                        problems.push(format!("unsupported time frame: {}", t.raw));
                    }
                    Some(_) => {}
                }
                if !problems.is_empty() {
                    let who = sym.map_or_else(String::new, |s| format!("{s}: "));
                    return format!("{who}{}", problems.join(" \u{00B7} "));
                }
                let tf =
                    r.tf.as_ref()
                        .map_or_else(String::new, cs_vision::parse::TfRead::label);
                let px = r
                    .price
                    .map_or_else(|| "price not read".into(), |(p, _)| price(p));
                let sym = sym.unwrap_or_default();
                let below = if state.behind_overlay {
                    " (window below the panel)"
                } else {
                    ""
                };
                format!("Screen: {sym} {tf} \u{00B7} {px}{below}")
            }
        },
    }
}

/// Plain words for an interval the panel has no feed for ("D" -> "one day").
pub fn interval_words(raw: &str) -> String {
    let (num, unit) = raw.split_at(raw.len().saturating_sub(1));
    let n = if num.is_empty() { "1" } else { num };
    let one = n == "1";
    let unit = match unit {
        "D" => {
            if one {
                "day"
            } else {
                "days"
            }
        }
        "W" => {
            if one {
                "week"
            } else {
                "weeks"
            }
        }
        "M" => {
            if one {
                "month"
            } else {
                "months"
            }
        }
        "S" => {
            if one {
                "second"
            } else {
                "seconds"
            }
        }
        "h" => {
            if one {
                "hour"
            } else {
                "hours"
            }
        }
        _ if raw.chars().all(|c| c.is_ascii_digit()) => {
            return format!("{raw}-minute candles");
        }
        _ => return raw.to_string(),
    };
    if one {
        format!("one {unit}")
    } else {
        format!("{n} {unit}")
    }
}

pub fn screen_unsupported(ticker: &str) -> String {
    format!(
        "UNSUPPORTED on screen: {ticker}. No feed for it, so no context. Supported: {} on 5m, 15m and 1h.",
        cs_live::SUPPORTED_SYMBOLS.join(", ")
    )
}

/// `tf` is the platform's label as written ("D", "1D", "3", "4h").
pub fn screen_unsupported_tf(ticker: &str, tf: &str) -> String {
    format!(
        "Unsupported time frame: {tf} ({}) on {ticker}. Context exists for 5m, 15m and 1h only, so nothing is shown for this chart.",
        interval_words(tf)
    )
}

/// Debug line (only with `CANDLESCOPE_VISION_DEBUG=1`): the window in front, as text.
pub fn front_debug(s: &str) -> String {
    format!("Debug \u{00B7} window in front: {s}")
}

pub fn mismatch(screen: f64, feed: f64, symbol: &str, diff_pct: f64) -> String {
    format!(
        "SOURCE MISMATCH: the screen shows {}, the {symbol} feed {} ({diff_pct:+.2} %). Context hidden until they agree.",
        price(screen),
        price(feed)
    )
}

pub fn mismatch_short(diff_pct: f64) -> String {
    format!("SOURCE MISMATCH ({diff_pct:+.2} %): screen and feed prices differ. Context hidden.")
}

/// Hotkey help, shown in the expanded panel.
pub const HELP_KEYS: &str = "Keys: Ctrl+Alt+M mute \u{00B7} Ctrl+Alt+T click-through \u{00B7} Ctrl+Alt+H hide \u{00B7} Ctrl+Alt+S screen reading";

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
            footer("BTCUSDT", Timeframe::M15),
            footer("NVDAUSDT", Timeframe::M5),
            PULLBACK_NOTE.into(),
            SCREEN_ON.into(),
            SCREEN_OFF.into(),
            SCREEN_TIP.into(),
            FOLLOW_SCREEN.into(),
            screen_unsupported("XAUUSD"),
            screen_unsupported_tf("BTCUSD", "4h"),
            mismatch(60_626.47, 1_704.15, "ETHUSDT", 3_457.5),
            mismatch_short(-1.2),
            HELP_KEYS.into(),
        ]);
        for s in ["BTCUSDT", "ETHUSDT", "XAUUSD"] {
            for tf in Timeframe::ALL {
                v.push(footer(s, tf));
            }
        }
        v.extend(screen_lines());
        v
    }

    fn screen_lines() -> Vec<String> {
        use cs_vision::read::{PageKind, PriceFrom, Reading, SymbolRead};
        use cs_vision::watch::{WatchState, WatchStatus};
        let mut out = Vec::new();
        for st in [
            WatchStatus::Off,
            WatchStatus::NoEngine,
            WatchStatus::Waiting,
            WatchStatus::Reading,
        ] {
            for page in [PageKind::Chart, PageKind::Other] {
                for sym in [
                    SymbolRead::Unknown,
                    SymbolRead::Unsupported {
                        ticker: "XAUUSD".into(),
                    },
                    SymbolRead::Supported {
                        ticker: "BTCUSD".into(),
                        exchange: "BTCUSDT".into(),
                    },
                ] {
                    let s = WatchState {
                        status: st,
                        reading: Some(Reading {
                            page,
                            symbol: sym,
                            tf: cs_vision::parse::parse_tf("15"),
                            price: Some((60_626.47, PriceFrom::AxisLabel)),
                            source: None,
                            symbol_from: "symbol region",
                            engine: None,
                            ocr_calls: 1,
                            read_ms: 1.0,
                        }),
                        ..WatchState::default()
                    };
                    out.push(screen_line(&s, false));
                    out.push(screen_line(&s, true));
                }
            }
        }
        out
    }

    /// One message per failure state, in the user's words.
    #[test]
    fn screen_messages_per_state() {
        use cs_vision::parse::parse_tf;
        use cs_vision::read::{PageKind, PriceFrom, Reading, SymbolRead};
        use cs_vision::watch::{WatchState, WatchStatus};
        let btc = SymbolRead::Supported {
            ticker: "BTCUSD".into(),
            exchange: "BTCUSDT".into(),
        };
        let state = |sym: SymbolRead, tf: Option<&str>| WatchState {
            status: WatchStatus::Reading,
            reading: Some(Reading {
                page: PageKind::Chart,
                symbol: sym,
                tf: tf.and_then(parse_tf),
                price: Some((61_515.0, PriceFrom::AxisLabel)),
                source: None,
                symbol_from: "symbol region",
                engine: None,
                ocr_calls: 1,
                read_ms: 1.0,
            }),
            ..WatchState::default()
        };
        let waiting = WatchState {
            status: WatchStatus::Waiting,
            ..WatchState::default()
        };
        assert_eq!(screen_line(&waiting, false), "TradingView not in front.");
        assert_eq!(
            screen_line(&state(SymbolRead::Unknown, Some("15")), false),
            "symbol not read"
        );
        assert_eq!(
            screen_line(&state(btc.clone(), None), false),
            "BTCUSD: time frame not read"
        );
        assert_eq!(
            screen_line(&state(btc.clone(), Some("D")), false),
            "BTCUSD: unsupported time frame: D"
        );
        assert_eq!(
            screen_line(&state(btc.clone(), Some("1D")), false),
            "BTCUSD: unsupported time frame: 1D"
        );
        assert_eq!(
            screen_line(&state(SymbolRead::Unknown, None), false),
            "symbol not read \u{00B7} time frame not read"
        );
        assert_eq!(
            screen_line(&state(btc, Some("15")), false),
            "Screen: BTCUSD 15m \u{00B7} 61,515.0"
        );
        assert!(front_debug("Google Chrome | BTCUSD").starts_with("Debug"));
    }

    /// The footer quotes the random-entry loss of the symbol and time frame on screen, with
    /// the numbers of gate-report.md Appendix C.
    #[test]
    fn footer_follows_symbol_and_time_frame() {
        assert!(
            footer("BTCUSDT", Timeframe::M5).contains("BTC 5m lost 1.01 to 1.10 R"),
            "{}",
            footer("BTCUSDT", Timeframe::M5)
        );
        assert!(footer("BTCUSDT", Timeframe::M15).contains("BTC 15m lost 0.53 to 0.57 R"));
        assert!(footer("BTCUSDT", Timeframe::H1).contains("BTC 1h lost 0.23 to 0.33 R"));
        assert!(footer("ETHUSDT", Timeframe::M5).contains("ETH 5m lost 0.59 to 0.74 R"));
        assert!(footer("ETHUSDT", Timeframe::M15).contains("ETH 15m lost 0.34 to 0.39 R"));
        assert!(footer("ETHUSDT", Timeframe::H1).contains("ETH 1h lost 0.12 to 0.25 R"));
        let other = footer("XAUUSD", Timeframe::H4);
        assert!(other.contains("0.12 to 1.10 R"), "{other}");
        for s in ["BTCUSDT", "ETHUSDT", "XAUUSD"] {
            for tf in Timeframe::ALL {
                let f = footer(s, tf);
                assert!(
                    f.starts_with("No setup passed testing (0 of 59). Context only, not a signal.")
                );
            }
        }
        // Every row of the table is in the report it cites.
        let report = include_str!("../../../reports/gate-report.md");
        for (s, tf, a, b) in RANDOM_ENTRY_R {
            for (period, v) in [("discovery", a), ("validation", b)] {
                let row = format!("| {s} | {} | {period} |", tf.label());
                let line = report
                    .lines()
                    .find(|l| l.starts_with(&row))
                    .unwrap_or_else(|| panic!("{row} not in gate-report.md"));
                assert!(line.contains(&format!("| {v:.4} |")), "{line} vs {v}");
            }
        }
    }

    #[test]
    fn help_line_names_every_hotkey() {
        for k in [
            "Ctrl+Alt+M mute",
            "Ctrl+Alt+T click-through",
            "Ctrl+Alt+H hide",
            "Ctrl+Alt+S",
        ] {
            assert!(HELP_KEYS.contains(k), "{k}");
        }
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
            ("screen.rs", include_str!("screen.rs")),
            ("settings.rs", include_str!("settings.rs")),
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
