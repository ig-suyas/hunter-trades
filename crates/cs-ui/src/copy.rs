//! Every word the panel shows outside the signal panel comes from here, so the wording can be
//! checked in one place. The signal panel's own words (BUY, SELL, entry, target, stop) live in
//! [`crate::signal_copy`] and nowhere else.
//!
//! Rules (the GATE found 0 of 59 setups with an edge): outside the signal panel, describe, never
//! suggest an action. No "long", "short", "buy", "sell", "signal" (except "not a signal"),
//! "probability", "win", "score", "edge", "confidence", "target", "stop", "entry" or certainty.
//! Arrows give the direction of a move that already happened; they carry text as well, so
//! nothing relies on colour.

use cs_core::{Timeframe, time};
use cs_live::journal::{CalRow, Hold, Import, Limits, Paper, Today};
use cs_live::signal::rules::{Exit, RuleDef};
use cs_live::signal::stats::{PeriodStats, RuleStats};
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

/// The GATE in one paragraph (Details), with the random-trade loss for the symbol and time
/// frame on screen.
pub fn gate_note(symbol: &str, tf: Timeframe) -> String {
    let head = "No setup passed testing (0 of 59). The rules here are the 10 pre-registered exploratory ones, all failed.";
    match RANDOM_ENTRY_R
        .iter()
        .find(|(s, t, _, _)| *s == symbol && *t == tf)
    {
        Some((s, t, a, b)) => {
            let (lo, hi) = (a.abs().min(b.abs()), a.abs().max(b.abs()));
            format!(
                "{head} Random trades on {} {} lost {lo:.2} to {hi:.2} R per trade after costs.",
                s.trim_end_matches("USDT"),
                t.label()
            )
        }
        None => format!(
            "{head} Random trades lost 0.12 to 1.10 R per trade after costs (BTC and ETH, 5m to 1h)."
        ),
    }
}

// ------------------------------------------------------------------------- WAIT and its reasons

pub const WAIT: &str = "WAIT";
pub const WAIT_MUTED: &str = "muted \u{00B7} UNMUTE or Ctrl+Alt+M to show";
pub const WAIT_NO_RULE: &str = "no tested rule for this time frame";
pub const WAIT_DISAGREE: &str = "rules disagree";
pub const WAIT_MISMATCH: &str = "screen and feed prices differ";
pub const WAIT_LOADING: &str = "loading history";
pub const WAIT_STALE: &str = "feed stale";
pub const WAIT_DISCONNECTED: &str = "feed disconnected";
pub const WAIT_NO_STATS: &str = "past results file missing (run candlescope rule-stats)";

pub fn wait_unsupported(sym: &str) -> String {
    format!("unsupported symbol: {sym}")
}

/// The quiet WAIT: how many frozen rules run on this time frame (all failed testing).
pub fn wait_idle(n_rules: usize) -> String {
    if n_rules == 1 {
        "1 rule watching \u{00B7} it failed testing".into()
    } else {
        format!("{n_rules} rules watching \u{00B7} all failed testing")
    }
}

/// WAIT reason of a daily limit.
pub fn wait_hold(h: Hold) -> String {
    match h {
        Hold::Open => String::new(),
        Hold::Day(n) => format!("daily limit: {n} alerts today"),
        Hold::Losses(n) => format!("paused: {n} losses in a row today"),
        Hold::LossR(r) => format!("paused: daily loss {r:.1} R reached"),
    }
}

/// Screen reading toggle and states.
pub const SCREEN_ON: &str = "\u{25C9} Screen";
pub const SCREEN_OFF: &str = "\u{25CB} Screen";
pub const SCREEN_TIP: &str = "Screen reading: symbol, time frame and last price from the TradingView window in front \
(small regions, in memory, nothing saved or sent). Ctrl+Alt+S";
pub const FOLLOW_SCREEN: &str = "Follow screen";

/// One line under the header about what the screen shows.
pub fn screen_line(state: &cs_vision::WatchState, manual: bool) -> String {
    use cs_vision::read::{PageKind, SymbolRead};
    use cs_vision::watch::WatchStatus;
    // One source of truth: the same `enabled` flag drives the toggle.
    if !state.enabled {
        return "Screen reading is off.".into();
    }
    if manual {
        return "Manual pick: the screen is ignored.".into();
    }
    match state.status {
        WatchStatus::Off => "Starting screen reading\u{2026}".into(),
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
                let held = if r.held.is_empty() {
                    String::new()
                } else {
                    " (last good read held)".to_string()
                };
                format!("Screen: {sym} {tf} \u{00B7} {px}{below}{held}")
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
        "Unsupported on screen: {ticker} (no feed; {} only)",
        cs_live::SUPPORTED_SYMBOLS.join(", ")
    )
}

/// `tf` is the platform's label as written ("D", "1D", "3", "4h").
pub fn screen_unsupported_tf(tf: &str) -> String {
    format!(
        "Unsupported time frame: {tf} ({}); 5m, 15m and 1h only",
        interval_words(tf)
    )
}

/// Debug line (only with `CANDLESCOPE_VISION_DEBUG=1`): the window in front, as text.
pub fn front_debug(s: &str) -> String {
    format!("Debug \u{00B7} window in front: {s}")
}

/// Details: the price check that hid the panel.
pub fn mismatch(screen: f64, feed: f64, symbol: &str, diff_pct: f64) -> String {
    format!(
        "Price check: the screen shows {}, the {symbol} feed {} ({diff_pct:+.2} %).",
        price(screen),
        price(feed)
    )
}

/// Hotkey help (Details).
pub const HELP_KEYS: &str = "Keys: Ctrl+Alt+M mute \u{00B7} Ctrl+Alt+T click-through \u{00B7} Ctrl+Alt+H hide \u{00B7} Ctrl+Alt+S screen reading";

pub const APP_TITLE: &str = "Candlescope";
pub const MUTE: &str = "MUTE";
pub const UNMUTE: &str = "UNMUTE";
pub const MUTE_TIP: &str =
    "Hides the panel's content; the feed and the log keep running. Ctrl+Alt+M";
pub const DETAILS: &str = "Details";
pub const DETAILS_TIP: &str = "Market context, the rule's full numbers, paper account, limits, calibration, demo fills, settings";
pub const DRAG_HINT: &str = "Drag here to move";
pub const RESIZE_HINT: &str = "Drag to resize";
pub const SYMBOL_TIP: &str = "Symbol (follows the screen while screen reading is on)";
pub const TF_TIP: &str = "Time frame: rules were tested on 15m and 1h; 5m has context only";

pub const H_RULE: &str = "This rule's past results";
pub const H_RULES: &str = "Rules on this time frame";
pub const H_PAPER: &str = "Paper account and daily limits";
pub const H_CAL: &str = "Calibration (your logged alerts)";
pub const H_FILLS: &str = "Demo fills";
pub const H_TREND: &str = "Trend";
pub const H_VOL: &str = "Volatility and session";
pub const H_ZONES: &str = "Nearest zones";
pub const H_EVENTS: &str = "Latest structure events";
pub const H_PULLBACK: &str = "Pullback";
pub const H_WINDOW: &str = "Window and keys";
pub const H_GATE: &str = "About these numbers";

pub const SET_ACCOUNT: &str = "Account size (USDT)";
pub const SET_RISK: &str = "Risk per trade (%)";
pub const SET_MAX_DAY: &str = "Alerts shown per day (max)";
pub const SET_PAUSE_LOSSES: &str = "Pause after losses in a row";
pub const SET_DAILY_LOSS: &str = "Pause after a daily loss of (R)";
pub const SET_RESET_PAPER: &str = "Reset paper account";
pub const SET_OPACITY: &str = "Background opacity";
pub const SET_CLICK: &str = "Click-through (mouse passes to the chart)";
pub const SET_BAD_NUMBER: &str = "Enter a positive number (risk % at most 100).";
pub const FILLS_HOW: &str = "One row per shown alert is added to data/signals/fills.csv. Fill in its four empty columns from your demo account (fill prices, size, fees), then press Import, or drop the file on this panel.";
pub const FILLS_IMPORT: &str = "Import demo fills";
pub const PAPER_NOTE: &str = "Paper account: every shown alert at your risk %, results from the backtest rules (1-minute candles and funding from the exchange). Limits reset at 00:00 UTC.";
pub const PAST_NOTE: &str = "The main line shows the validation period: out of sample, after the rules were picked. Months 1-18 and discovery are in-sample: the rules were picked on the discovery year, so those numbers flatter them. Historical numbers only; the sealed holdout was never used.";

/// Under the pullback line: the flag is a description, not an advantage.
pub const PULLBACK_NOTE: &str = "\"Healthy\" = smaller and slower than the move before it. None of the 60 healthy-pullback variants passed testing.";
pub const CLICK_ON: &str = "Click-through is ON: press Ctrl+Alt+T to use the panel again.";
pub const HOTKEYS_TAKEN: &str = "Some hotkeys could not be registered (another program uses them).";

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

pub fn session(s: u8) -> &'static str {
    match s {
        0 => "Asia session (00-07 UTC)",
        1 => "London session (07-12 UTC)",
        2 => "London/New York overlap (12-16 UTC)",
        3 => "New York session (16-21 UTC)",
        _ => "Late session (21-24 UTC)",
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

// ------------------------------------------------------------------------------- Details text

/// "95 min", "10 h 30 min", "2 days 4 h" (rounded to 5 minutes).
pub fn duration(min: f64) -> String {
    if !min.is_finite() || min < 0.0 {
        return "n/a".into();
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let m = ((min / 5.0).round() * 5.0) as u64;
    if m < 120 {
        return format!("{m} min");
    }
    let (h, mm) = (m / 60, m % 60);
    if h >= 48 {
        let (d, hh) = (h / 24, h % 24);
        return format!("{d} days {hh} h");
    }
    if mm == 0 {
        format!("{h} h")
    } else {
        format!("{h} h {mm} min")
    }
}

fn pct(x: f64) -> String {
    if x.is_finite() {
        // A no-break space keeps the number and its % sign on one line.
        format!("{:.1}\u{00A0}%", x * 100.0)
    } else {
        "n/a".into()
    }
}

fn rr(x: f64) -> String {
    if x.is_finite() {
        format!("{x:+.2}")
    } else {
        "n/a".into()
    }
}

/// How a rule's hit rate is counted, in words.
pub fn rate_word(def: &RuleDef) -> &'static str {
    match def.exit {
        Exit::Trail => "ended above 0 R",
        _ => "hit rate",
    }
}

/// One period of a rule's past results (Details).
pub fn period_line(name: &str, def: &RuleDef, p: &PeriodStats) -> String {
    let low = if p.low_evidence() {
        " \u{00B7} low evidence"
    } else {
        ""
    };
    format!(
        "{name}: n = {}{low}; {} {} (80 % interval {} \u{2013} {}); expected R after costs {} at 1x (80 % {} \u{2026} {}), {} at 2x (80 % {} \u{2026} {}); median time in trade {}",
        p.n,
        rate_word(def),
        pct(p.hit_rate),
        pct(p.hit_ci80.0),
        pct(p.hit_ci80.1),
        rr(p.exp_1x),
        rr(p.exp_1x_ci80.0),
        rr(p.exp_1x_ci80.1),
        rr(p.exp_2x),
        rr(p.exp_2x_ci80.0),
        rr(p.exp_2x_ci80.1),
        duration(p.hold_median_min)
    )
}

/// The rule's identity and frozen parameters (Details).
pub fn rule_title(def: &RuleDef) -> String {
    format!(
        "Rule {} of 10: {} (trial {})",
        def.order, def.label, def.trial_id
    )
}

/// Round-trip cost at the alert's distance (Details).
pub fn cost_line(c1: f64, c2: f64) -> String {
    if c1.is_finite() {
        format!(
            "Round-trip cost at this distance: {c1:.2} R at 1x, {c2:.2} R at 2x costs (fees, spread, slippage; funding not included)."
        )
    } else {
        "Round-trip cost at this distance: n/a.".into()
    }
}

/// One line per rule of the time frame (Details, when no alert is shown): validation numbers.
pub fn rule_row(def: &RuleDef, st: Option<&RuleStats>) -> String {
    match st {
        Some(s) => format!(
            "{} \u{00B7} {} \u{00B7} validation: {} {:.0} % (n={}{}), {} R",
            def.order,
            def.label,
            rate_word(def),
            s.validation.hit_rate * 100.0,
            s.validation.n,
            if s.validation.low_evidence() {
                ", low evidence"
            } else {
                ""
            },
            rr(s.validation.exp_1x)
        ),
        None => format!(
            "{} \u{00B7} {} \u{00B7} no past results",
            def.order, def.label
        ),
    }
}

pub fn paper_line(p: &Paper) -> String {
    format!(
        "Paper account: {} \u{2192} {} USDT ({} closed, {} open, {} R).",
        price(p.start),
        price(p.balance),
        p.closed,
        p.open,
        rr(p.net_r)
    )
}

pub fn today_line(t: &Today, l: &Limits) -> String {
    format!(
        "Today (UTC): {} of {} alerts shown, {} closed, {} R, {} losses in a row.",
        t.shown,
        l.max_per_day,
        t.resolved,
        rr(t.net_r),
        t.losses_in_row
    )
}

/// Calibration: rows once there are enough, else how many are still needed.
pub fn calibration_lines(n: usize, rows: &[CalRow]) -> Vec<String> {
    if rows.is_empty() {
        return vec![format!(
            "Shown once {} of your alerts are resolved ({n} so far).",
            cs_live::journal::CALIBRATION_MIN
        )];
    }
    rows.iter()
        .map(|r| {
            let who = r
                .rule
                .map_or_else(|| "All rules".to_string(), |o| format!("Rule {o}"));
            format!(
                "{who}: {} resolved, past rate {} vs your {}",
                r.n,
                pct(r.expected),
                pct(r.actual)
            )
        })
        .collect()
}

pub fn import_lines(i: &Import) -> Vec<String> {
    let mut v = vec![format!(
        "Imported {} demo trades into data/signals/demo-trades.csv; {} rows still empty.",
        i.trades.len(),
        i.empty
    )];
    v.extend(i.problems.iter().take(4).cloned());
    v
}

pub fn times_note(offset_s: i64) -> String {
    let sign = if offset_s < 0 { '-' } else { '+' };
    let a = offset_s.unsigned_abs();
    format!(
        "Panel times are local (UTC{sign}{:02}:{:02}).",
        a / 3600,
        (a % 3600) / 60
    )
}

pub fn feed_line(s: FeedStatus, reconnects: u64, warm: usize) -> String {
    format!(
        "Feed: {} \u{00B7} {warm} candles loaded \u{00B7} {reconnects} reconnects",
        s.label()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::signal_copy;
    use cs_live::signal::rules::RULES;
    use cs_live::signal::stats::StatsBook;
    use cs_live::snapshot::{HtfView, Marker, ZoneKind, ZoneView};

    /// Words that must never appear in the panel (case-insensitive, as word starts), except in
    /// the exact phrase "not a signal", and except the signal panel's own words below.
    const FORBIDDEN: [&str; 24] = [
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
        "stop",
        "entry",
        "entri",
    ];

    /// Allowed **only** inside the signal panel ([`crate::signal_copy`]), next to its
    /// "No edge shown in testing." line.
    const PANEL_ONLY: [&str; 5] = ["buy", "sell", "target", "stop", "entry"];

    /// The first forbidden word in `s`. `panel` = text of the signal panel: its own words and the
    /// exact phrase "No edge shown in testing." are allowed there, nothing else.
    pub fn bad(s: &str, panel: bool) -> Option<&'static str> {
        // The log folder's name (the user's choice) is a path, not a word on its own.
        let mut l = s
            .to_lowercase()
            .replace("not a signal", "")
            .replace("data/signals/", "data/");
        if panel {
            l = l.replace(&signal_copy::NO_EDGE.to_lowercase(), "");
        }
        let words: Vec<&str> = l
            .split(|c: char| !c.is_alphanumeric())
            .filter(|w| !matches!(*w, "window" | "windows" | "windir"))
            .collect();
        FORBIDDEN
            .iter()
            .copied()
            .filter(|f| !(panel && PANEL_ONLY.contains(f)))
            .find(|f| {
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
            "Stop at 60,000",
            "entries",
            "targets",
            "an edge",
            "BUY",
            "Entry",
        ] {
            assert!(bad(s, false).is_some(), "{s}");
        }
        for s in [
            "window size",
            "Context only, not a signal.",
            "position size",
            "Hold about 3 h",
            "see data/signals/fills.csv",
        ] {
            assert!(bad(s, false).is_none(), "{s}");
        }
        assert!(
            bad("signals/fills.csv", false).is_some(),
            "only the exact folder path"
        );
        // Inside the signal panel: its own words pass, everything else is still caught.
        for s in [
            "\u{2191} BUY",
            "\u{2193} SELL",
            "Entry",
            "Target",
            "Stop",
            "No edge shown in testing.",
        ] {
            assert!(bad(s, true).is_none(), "{s}");
        }
        for s in [
            "an edge",
            "edge",
            "strong signal",
            "win rate",
            "long",
            "you should",
            "high chance",
        ] {
            assert!(bad(s, true).is_some(), "{s}");
        }
    }

    fn book() -> StatsBook {
        StatsBook::parse(include_str!("../../../reports/rule-stats.json")).unwrap()
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
            v.push(feed_line(s, 2, 1500));
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
            }
        }
        for s in 0..5 {
            v.push(session(s).into());
        }
        // WAIT and every reason it can carry.
        for h in [Hold::Open, Hold::Day(3), Hold::Losses(3), Hold::LossR(3.24)] {
            v.push(wait_hold(h));
        }
        v.extend([
            WAIT.into(),
            WAIT_MUTED.into(),
            WAIT_NO_RULE.into(),
            WAIT_DISAGREE.into(),
            WAIT_MISMATCH.into(),
            WAIT_LOADING.into(),
            WAIT_STALE.into(),
            WAIT_DISCONNECTED.into(),
            WAIT_NO_STATS.into(),
            wait_unsupported("NVDAUSDT"),
            wait_idle(1),
            wait_idle(8),
            vol(0.63),
            drift_warning(1500),
            pullback_none(Some(40)),
            recorder(0.5, 12, Some(30)),
            PULLBACK_NOTE.into(),
            SCREEN_ON.into(),
            SCREEN_OFF.into(),
            SCREEN_TIP.into(),
            FOLLOW_SCREEN.into(),
            screen_unsupported("XAUUSD"),
            screen_unsupported_tf("4h"),
            mismatch(60_626.47, 1_704.15, "ETHUSDT", 3_457.5),
            HELP_KEYS.into(),
            APP_TITLE.into(),
            MUTE.into(),
            UNMUTE.into(),
            MUTE_TIP.into(),
            DETAILS.into(),
            DETAILS_TIP.into(),
            SYMBOL_TIP.into(),
            TF_TIP.into(),
            FILLS_HOW.into(),
            FILLS_IMPORT.into(),
            PAPER_NOTE.into(),
            PAST_NOTE.into(),
            CLICK_ON.into(),
            HOTKEYS_TAKEN.into(),
            SET_ACCOUNT.into(),
            SET_RISK.into(),
            SET_MAX_DAY.into(),
            SET_PAUSE_LOSSES.into(),
            SET_DAILY_LOSS.into(),
            SET_RESET_PAPER.into(),
            SET_OPACITY.into(),
            SET_CLICK.into(),
            SET_BAD_NUMBER.into(),
            cost_line(0.24, 0.48),
            cost_line(f64::NAN, f64::NAN),
            times_note(19_800),
            times_note(-3_600),
        ]);
        for h in [
            H_RULE, H_RULES, H_PAPER, H_CAL, H_FILLS, H_TREND, H_VOL, H_ZONES, H_EVENTS,
            H_PULLBACK, H_WINDOW, H_GATE,
        ] {
            v.push(h.into());
        }
        for s in ["BTCUSDT", "ETHUSDT", "XAUUSD"] {
            for tf in Timeframe::ALL {
                v.push(gate_note(s, tf));
            }
        }
        // The rules' full numbers in Details, from the real file.
        let b = book();
        for def in &RULES {
            v.push(def.describe());
            v.push(rule_title(def));
            v.push(rule_row(def, None));
            for sym in ["BTCUSDT", "ETHUSDT"] {
                let st = b.get(def.order, sym).unwrap();
                v.push(rule_row(def, Some(st)));
                for (n, p) in [
                    ("months 1-18", &st.all),
                    ("discovery", &st.discovery),
                    ("validation", &st.validation),
                ] {
                    v.push(period_line(n, def, p));
                }
            }
        }
        // Paper account, limits, calibration and the import messages (incl. its problems).
        let p = Paper {
            start: 10_000.0,
            balance: 9_870.5,
            closed: 3,
            open: 1,
            net_r: -1.2,
        };
        v.push(paper_line(&p));
        v.push(today_line(
            &Today {
                shown: 2,
                resolved: 1,
                losses_in_row: 1,
                net_r: -1.1,
            },
            &Limits::default(),
        ));
        v.extend(calibration_lines(12, &[]));
        v.extend(calibration_lines(
            31,
            &[
                CalRow {
                    rule: Some(1),
                    n: 31,
                    expected: 0.37,
                    actual: 0.3,
                },
                CalRow {
                    rule: None,
                    n: 31,
                    expected: 0.37,
                    actual: 0.3,
                },
            ],
        ));
        let j = cs_live::journal::Journal::in_memory();
        let imp = cs_live::journal::import_fills(
            &j,
            &format!(
                "{}\nx,1,2,3,4,5,6,7,8,1,,1,1\ny,1,2,3,4,5,6,7,8,1,2,1,1\nz,1,2,3,4,5,6,7,8,-1,2,1,1\n",
                cs_live::journal::FILLS_HEADER
            ),
        );
        assert_eq!(imp.problems.len(), 3);
        v.extend(import_lines(&imp));
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
                        enabled: true,
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
                            tf_from: "toolbar",
                            tf_note: None,
                            held: Vec::new(),
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
            enabled: true,
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
                tf_from: "toolbar",
                tf_note: None,
                held: Vec::new(),
            }),
            ..WatchState::default()
        };
        let waiting = WatchState {
            enabled: true,
            status: WatchStatus::Waiting,
            ..WatchState::default()
        };
        assert_eq!(screen_line(&waiting, false), "TradingView not in front.");
        let off = WatchState {
            enabled: false,
            status: WatchStatus::Reading,
            ..WatchState::default()
        };
        assert_eq!(screen_line(&off, false), "Screen reading is off.");
        assert_eq!(
            screen_line(&state(btc.clone(), Some("D")), false),
            "BTCUSD: unsupported time frame: D"
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

    /// The GATE note in Details quotes the random-trade loss of the symbol and time frame on
    /// screen, with the numbers of gate-report.md Appendix C.
    #[test]
    fn gate_note_follows_symbol_and_time_frame() {
        assert!(gate_note("BTCUSDT", Timeframe::M5).contains("BTC 5m lost 1.01 to 1.10 R"));
        assert!(gate_note("BTCUSDT", Timeframe::M15).contains("BTC 15m lost 0.53 to 0.57 R"));
        assert!(gate_note("BTCUSDT", Timeframe::H1).contains("BTC 1h lost 0.23 to 0.33 R"));
        assert!(gate_note("ETHUSDT", Timeframe::M5).contains("ETH 5m lost 0.59 to 0.74 R"));
        assert!(gate_note("ETHUSDT", Timeframe::M15).contains("ETH 15m lost 0.34 to 0.39 R"));
        assert!(gate_note("ETHUSDT", Timeframe::H1).contains("ETH 1h lost 0.12 to 0.25 R"));
        assert!(gate_note("XAUUSD", Timeframe::H4).contains("0.12 to 1.10 R"));
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

    /// Everything outside the signal panel (WAIT reasons, Details, errors, other states): none
    /// of the forbidden words, BUY, SELL, target, stop and entry included.
    #[test]
    fn generated_copy_outside_the_panel_has_no_signal_words() {
        let all = all_generated();
        assert!(all.len() > 300);
        for s in all {
            assert!(
                bad(&s, false).is_none(),
                "forbidden word {:?} in {s:?}",
                bad(&s, false)
            );
        }
    }

    /// The signal panel: BUY, SELL, target, stop and entry are allowed there, nothing else
    /// forbidden, and every panel carries the past-results line ending in "No edge shown in
    /// testing." right under them.
    #[test]
    fn panel_words_only_next_to_the_no_edge_line() {
        let b = book();
        let mut n = 0;
        for def in &RULES {
            for sym in ["BTCUSDT", "ETHUSDT"] {
                let st = b.get(def.order, sym).unwrap();
                for dir in [1i8, -1] {
                    for (account, risk) in [(10_000.0, 1.0), (10_000.0, 100.0), (f64::NAN, 1.0)] {
                        let s = crate::board::tests_alert(def.order, dir, sym, def.exit);
                        let p = signal_copy::panel(&s, def, st, 2, account, risk, 19_800);
                        let mut texts = vec![p.state.clone(), p.past.clone()];
                        texts.extend(p.more.clone());
                        for (_, l, v) in &p.rows {
                            texts.push(l.clone());
                            texts.push(v.clone());
                        }
                        for t in &texts {
                            assert!(
                                bad(t, true).is_none(),
                                "{:?} in panel text {t:?}",
                                bad(t, true)
                            );
                        }
                        assert!(p.past.ends_with(signal_copy::NO_EDGE), "{}", p.past);
                        assert!(p.past.starts_with("Past results of this rule: "));
                        assert!(
                            texts.iter().any(|t| bad(t, false).is_some()),
                            "panel words present"
                        );
                        n += 1;
                    }
                }
            }
        }
        assert_eq!(n, 10 * 2 * 2 * 3);
    }

    /// Every string literal of the UI sources (outside test modules): the signal panel's module
    /// may use its own words; every other file none of the forbidden words.
    #[test]
    fn source_literals_have_no_signal_words_outside_the_panel() {
        let files = [
            ("copy.rs", include_str!("copy.rs"), false),
            ("view.rs", include_str!("view.rs"), false),
            ("app.rs", include_str!("app.rs"), false),
            ("main.rs", include_str!("main.rs"), false),
            ("demo.rs", include_str!("demo.rs"), false),
            ("screen.rs", include_str!("screen.rs"), false),
            ("settings.rs", include_str!("settings.rs"), false),
            ("board.rs", include_str!("board.rs"), false),
            ("recorder.rs", include_str!("recorder.rs"), false),
            ("palette.rs", include_str!("palette.rs"), false),
            ("signal_copy.rs", include_str!("signal_copy.rs"), true),
        ];
        for (name, src, panel) in files {
            let code = src.split("#[cfg(test)]").next().unwrap_or(src);
            // Comments are not shown in the panel.
            let body: String = code
                .lines()
                .filter(|l| !l.trim_start().starts_with("//"))
                .collect::<Vec<_>>()
                .join("\n");
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
                    bad(&l, panel).is_none(),
                    "{name}: forbidden word {:?} in literal {l:?}",
                    bad(&l, panel)
                );
            }
        }
    }

    /// The risk calculator and its three price fields are gone from every UI source.
    #[test]
    fn risk_calculator_and_price_fields_are_gone() {
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        assert!(!dir.join("risk.rs").exists(), "risk.rs still exists");
        let mut n = 0;
        for e in std::fs::read_dir(&dir)
            .unwrap()
            .chain(std::fs::read_dir(dir.join("bin")).unwrap())
        {
            let p = e.unwrap().path();
            if p.extension().and_then(|x| x.to_str()) != Some("rs") {
                continue;
            }
            let src = std::fs::read_to_string(&p).unwrap();
            let code = src.split("#[cfg(test)]").next().unwrap_or(&src);
            for gone in [
                "Your entry price",
                "Your stop price",
                "Funding settlements",
                "Risk calculator",
                "risk_panel",
                "RiskIn",
                "funding_n",
                "focus_risk",
                "\"--entry\"",
                "\"--stop\"",
                "mod risk",
                "pub entry: String",
                "pub stop: String",
            ] {
                assert!(!code.contains(gone), "{}: {gone}", p.display());
            }
            n += 1;
        }
        assert!(n >= 10);
    }

    #[test]
    fn prices_and_durations() {
        assert_eq!(price(91_234.56), "91,234.6");
        assert_eq!(price(3_012.0), "3,012.0");
        assert_eq!(price(123.456), "123.46");
        assert_eq!(price(1_234_567.0), "1,234,567.0");
        assert_eq!(ordinal(11), "11th");
        assert_eq!(ordinal(63), "63rd");
        assert_eq!(ordinal(21), "21st");
        assert_eq!(duration(95.0), "95 min");
        assert_eq!(duration(631.0), "10 h 30 min");
        assert_eq!(duration(3000.0), "2 days 2 h");
    }
}
