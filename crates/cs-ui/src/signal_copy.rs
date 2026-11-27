//! The words of the signal panel, and only those. This is the **one** place in the UI where
//! BUY, SELL, target, stop and entry may appear, and every panel built here carries the small
//! line ending in "No edge shown in testing." (a test enforces both: these words nowhere else,
//! and never without that line).
//!
//! The rules shown all failed the GATE (0 of 59 setups survived). The past-results line is
//! historical and in-sample (discovery and validation periods); it is never a forecast.

use cs_core::time;
use cs_live::signal::Signal;
use cs_live::signal::rules::{Exit, RuleDef};
use cs_live::signal::sizing::{self, Size};
use cs_live::signal::stats::RuleStats;

use crate::copy::{duration, price};

/// The closing words of the past-results line (the only place "edge" may appear).
pub const NO_EDGE: &str = "No edge shown in testing.";

/// One row of the panel: what it is, its label and its value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Row {
    Entry,
    Target,
    Stop,
    Hold,
    Size,
}

/// Everything the signal panel shows for an active alert.
#[derive(Debug, Clone, PartialEq)]
pub struct Panel {
    /// "↑ BUY" or "↓ SELL" (arrow and word, never colour alone).
    pub state: String,
    /// "+2 more" when more rules fired the same way on the same candle.
    pub more: Option<String>,
    pub rows: Vec<(Row, String, String)>,
    /// "Past results of this rule: ... No edge shown in testing."
    pub past: String,
}

pub fn state(dir: i8) -> &'static str {
    if dir > 0 {
        "\u{2191} BUY"
    } else {
        "\u{2193} SELL"
    }
}

const WEEKDAYS: [&str; 7] = ["Thu", "Fri", "Sat", "Sun", "Mon", "Tue", "Wed"];

/// Local wall time `HH:MM`, with the weekday when it is not on the same local day as `base`
/// (the alert's entry time, so the text of an alert never changes while it is shown).
pub fn local_time(t: i64, base: i64, offset_s: i64) -> String {
    let lt = t + offset_s * 1000;
    let ln = base + offset_s * 1000;
    let (_, _, _, h, m, _) = time::split(lt);
    let day = lt.div_euclid(86_400_000);
    if day == ln.div_euclid(86_400_000) {
        format!("{h:02}:{m:02}")
    } else {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let wd = WEEKDAYS[day.rem_euclid(7) as usize];
        format!("{wd} {h:02}:{m:02}")
    }
}

/// The past-results line: months 1-18 of this rule on this symbol (historical, in-sample).
pub fn past_line(def: &RuleDef, st: &RuleStats) -> String {
    let a = &st.all;
    let rate = match def.exit {
        Exit::Trail => "ended above 0 R",
        _ => "hit rate",
    };
    let low = if a.low_evidence() {
        ", low evidence"
    } else {
        ""
    };
    let mut line = format!(
        "Past results of this rule: {rate} {:.0}% (n={}{low}), expected R after costs {:+.2}. ",
        a.hit_rate * 100.0,
        a.n,
        a.exp_1x
    );
    line.push_str(NO_EDGE);
    line
}

/// Size row text.
pub fn size_text(s: Option<Size>, base: &str) -> String {
    match s {
        None => "set account size and risk % in Details".into(),
        Some(z) if z.capped => format!(
            "{:.4} {base} ({} USDT, 10x cap: risks {:.2} %)",
            z.qty,
            price(z.notional),
            z.risk_frac * 100.0
        ),
        Some(z) => format!("{:.4} {base} ({} USDT)", z.qty, price(z.notional)),
    }
}

/// The whole panel for an alert. `account` and `risk_pct` are the values when the alert
/// arrived (frozen, so the shown size never changes).
pub fn panel(
    s: &Signal,
    def: &RuleDef,
    st: &RuleStats,
    more: usize,
    account: f64,
    risk_pct: f64,
    offset_s: i64,
) -> Panel {
    let base = s.symbol.trim_end_matches("USDT");
    let target = if s.target.is_finite() {
        price(s.target)
    } else if s.trail_from.is_finite() {
        format!("none \u{00B7} trailing from {} (+1 R)", price(s.trail_from))
    } else {
        "none".into()
    };
    let hold = match s.time_exit {
        Some(t) => format!(
            "about {}, exit by {} at the latest",
            duration(st.all.hold_median_min),
            local_time(t, s.entry_time, offset_s)
        ),
        None => format!(
            "about {}; this rule has no time exit",
            duration(st.all.hold_median_min)
        ),
    };
    Panel {
        state: state(s.dir).to_string(),
        more: (more > 0).then(|| format!("+{more} more")),
        rows: vec![
            (
                Row::Entry,
                "Entry".into(),
                format!(
                    "{} at {}",
                    price(s.entry),
                    local_time(s.entry_time, s.entry_time, offset_s)
                ),
            ),
            (Row::Target, "Target".into(), target),
            (Row::Stop, "Stop".into(), price(s.stop)),
            (Row::Hold, "Hold".into(), hold),
            (
                Row::Size,
                "Size".into(),
                size_text(sizing::size(account, risk_pct, s.entry, s.stop), base),
            ),
        ],
        past: past_line(def, st),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn local_times() {
        // 2026-03-02 05:00 UTC is a Monday; 10:30 local at +5:30.
        let t = time::ymd_hms_to_ms(2026, 3, 2, 5, 0, 0);
        assert_eq!(local_time(t, t, 19_800), "10:30");
        assert_eq!(local_time(t + 4 * 86_400_000, t, 19_800), "Fri 10:30");
    }
}
