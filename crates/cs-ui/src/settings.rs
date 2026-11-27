//! The overlay's small local settings file, `data/overlay-settings.json` (git-ignored, never
//! sent anywhere): window position and size, opacity, whether Details is open, the two sizing
//! inputs (account size, risk % per trade), the daily limits, the paper account's start time,
//! the last symbol and time frame, and whether screen reading is on. Read at start, written when
//! something changed (at most every 2 s). A missing or broken file means defaults, field by
//! field.

use std::path::{Path, PathBuf};

use cs_core::Timeframe;
use cs_live::journal::Limits;
use serde_json::{Value, json};

use crate::palette as pal;

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    /// Outer window position (screen points); `None` lets Windows place it.
    pub pos: Option<[f32; 2]>,
    /// Inner window size with Details closed (points); `None` = the default.
    pub size: Option<[f32; 2]>,
    pub opacity: f32,
    /// The Details section is open (closed by default).
    pub details_open: bool,
    /// Account size in USDT and risk % per trade (the only two inputs), as typed.
    pub account: String,
    pub risk_pct: String,
    pub limits: Limits,
    /// The paper account counts shown alerts from this time on (UTC ms; 0 = all).
    pub paper_since: i64,
    pub symbol: String,
    pub tf: Timeframe,
    pub screen_reading: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Settings {
            pos: None,
            size: None,
            opacity: pal::DEFAULT_OPACITY,
            details_open: false,
            account: "10,000".into(),
            risk_pct: "1".into(),
            limits: Limits::default(),
            paper_since: 0,
            symbol: "BTCUSDT".into(),
            tf: Timeframe::M15,
            screen_reading: true,
        }
    }
}

pub fn path(root: &Path) -> PathBuf {
    root.join("data").join("overlay-settings.json")
}

/// A number as typed ("10,000", "0.5"): commas, spaces and underscores ignored.
pub fn parse_number(s: &str) -> Option<f64> {
    let t: String = s
        .chars()
        .filter(|c| !matches!(c, ',' | ' ' | '_'))
        .collect();
    t.parse::<f64>().ok().filter(|x| x.is_finite())
}

#[allow(clippy::cast_possible_truncation)]
fn pair(v: &Value, lo: f32, hi: f32) -> Option<[f32; 2]> {
    let a = v.as_array()?;
    let x = a.first()?.as_f64()? as f32;
    let y = a.get(1)?.as_f64()? as f32;
    (x.is_finite() && y.is_finite() && (lo..=hi).contains(&x) && (lo..=hi).contains(&y))
        .then_some([x, y])
}

/// Short text field: kept only if it is a positive number within `max`.
fn number_text(v: Option<&Value>, default: &str, max: f64) -> String {
    v.and_then(Value::as_str)
        .filter(|s| s.len() <= 20 && parse_number(s).is_some_and(|x| x > 0.0 && x <= max))
        .map_or_else(|| default.to_string(), str::to_string)
}

fn count(v: Option<&Value>, default: u32, max: u64) -> u32 {
    v.and_then(Value::as_u64)
        .filter(|x| (1..=max).contains(x))
        .and_then(|x| u32::try_from(x).ok())
        .unwrap_or(default)
}

impl Settings {
    /// Parses the file's text; unknown or invalid fields fall back to defaults one by one.
    pub fn from_json(s: &str) -> Settings {
        let d = Settings::default();
        // Notepad and PowerShell 5 write a byte-order mark; serde_json does not accept one.
        let s = s.trim_start_matches('\u{feff}');
        let Ok(v) = serde_json::from_str::<Value>(s) else {
            return d;
        };
        #[allow(clippy::cast_possible_truncation)]
        let opacity = v
            .get("opacity")
            .and_then(Value::as_f64)
            .map_or(d.opacity, |x| (x as f32).clamp(pal::MIN_OPACITY, 1.0));
        let symbol = v
            .get("symbol")
            .and_then(Value::as_str)
            .map(str::to_uppercase)
            .filter(|s| cs_live::is_supported(s))
            .unwrap_or(d.symbol);
        let tf = v
            .get("tf")
            .and_then(Value::as_str)
            .and_then(Timeframe::parse)
            .filter(|t| cs_live::SUPPORTED_TFS.contains(t))
            .unwrap_or(d.tf);
        let lim = v.get("limits").unwrap_or(&Value::Null);
        let limits = Limits {
            max_per_day: count(lim.get("max_per_day"), d.limits.max_per_day, 50),
            pause_after_losses: count(
                lim.get("pause_after_losses"),
                d.limits.pause_after_losses,
                50,
            ),
            daily_loss_r: lim
                .get("daily_loss_r")
                .and_then(Value::as_f64)
                .filter(|x| *x > 0.0 && *x <= 100.0)
                .unwrap_or(d.limits.daily_loss_r),
        };
        Settings {
            pos: v.get("pos").and_then(|p| pair(p, -20_000.0, 20_000.0)),
            size: v.get("size").and_then(|p| pair(p, 200.0, 4_000.0)),
            opacity,
            details_open: v
                .get("details_open")
                .and_then(Value::as_bool)
                .unwrap_or(d.details_open),
            account: number_text(v.get("account"), &d.account, 1e12),
            risk_pct: number_text(v.get("risk_pct"), &d.risk_pct, 100.0),
            limits,
            paper_since: v
                .get("paper_since_ms")
                .and_then(Value::as_i64)
                .filter(|x| *x >= 0)
                .unwrap_or(d.paper_since),
            symbol,
            tf,
            screen_reading: v
                .get("screen_reading")
                .and_then(Value::as_bool)
                .unwrap_or(d.screen_reading),
        }
    }

    pub fn to_json(&self) -> String {
        let v = json!({
            "about": "Candlescope overlay settings (local only). Delete this file to reset.",
            "pos": self.pos,
            "size": self.size,
            "opacity": (f64::from(self.opacity) * 100.0).round() / 100.0,
            "details_open": self.details_open,
            "account": self.account,
            "risk_pct": self.risk_pct,
            "limits": {
                "max_per_day": self.limits.max_per_day,
                "pause_after_losses": self.limits.pause_after_losses,
                "daily_loss_r": self.limits.daily_loss_r,
            },
            "paper_since_ms": self.paper_since,
            "symbol": self.symbol,
            "tf": self.tf.label(),
            "screen_reading": self.screen_reading,
        });
        serde_json::to_string_pretty(&v).unwrap_or_default()
    }

    /// Reads `data/overlay-settings.json`, or defaults.
    pub fn load(root: &Path) -> Settings {
        std::fs::read_to_string(path(root))
            .map_or_else(|_| Settings::default(), |s| Settings::from_json(&s))
    }

    /// Writes the file (through a temporary file, so a crash never leaves half a file).
    ///
    /// # Errors
    /// The folder cannot be created or written.
    pub fn save(&self, root: &Path) -> std::io::Result<()> {
        let p = path(root);
        if let Some(d) = p.parent() {
            std::fs::create_dir_all(d)?;
        }
        let tmp = p.with_extension("json.tmp");
        std::fs::write(&tmp, self.to_json())?;
        std::fs::rename(tmp, p)
    }

    /// Same content for saving purposes (positions rounded to whole points).
    // Positions are bounded to +-20,000 points by the parser, so the cast cannot overflow.
    #[allow(clippy::cast_possible_truncation)]
    pub fn same(&self, o: &Settings) -> bool {
        let r = |a: Option<[f32; 2]>| a.map(|[x, y]| (x.round() as i64, y.round() as i64));
        r(self.pos) == r(o.pos)
            && r(self.size) == r(o.size)
            && (self.opacity - o.opacity).abs() < 0.005
            && self.details_open == o.details_open
            && self.account == o.account
            && self.risk_pct == o.risk_pct
            && self.limits.max_per_day == o.limits.max_per_day
            && self.limits.pause_after_losses == o.limits.pause_after_losses
            && (self.limits.daily_loss_r - o.limits.daily_loss_r).abs() < 1e-9
            && self.paper_since == o.paper_since
            && self.symbol == o.symbol
            && self.tf == o.tf
            && self.screen_reading == o.screen_reading
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Settings {
        Settings {
            pos: Some([1510.0, 96.0]),
            size: Some([360.0, 340.0]),
            opacity: 0.87,
            details_open: true,
            account: "25,000".into(),
            risk_pct: "0.5".into(),
            limits: Limits {
                max_per_day: 2,
                pause_after_losses: 4,
                daily_loss_r: 2.5,
            },
            paper_since: 1_772_409_600_000,
            symbol: "ETHUSDT".into(),
            tf: Timeframe::H1,
            screen_reading: false,
        }
    }

    #[test]
    fn defaults_are_the_requested_ones() {
        let d = Settings::default();
        assert!(!d.details_open, "Details closed by default");
        assert_eq!(parse_number(&d.account), Some(10_000.0), "10,000 USDT");
        assert_eq!(parse_number(&d.risk_pct), Some(1.0), "1 %");
    }

    #[test]
    fn round_trip() {
        let s = sample();
        let back = Settings::from_json(&s.to_json());
        assert!(back.same(&s), "{back:?}");
        assert_eq!(back, s);
        // A file saved by Notepad starts with a byte-order mark.
        assert_eq!(Settings::from_json(&format!("\u{feff}{}", s.to_json())), s);
    }

    #[test]
    fn missing_or_broken_file_gives_defaults() {
        assert_eq!(Settings::from_json(""), Settings::default());
        assert_eq!(Settings::from_json("{not json"), Settings::default());
        assert_eq!(Settings::from_json("{}"), Settings::default());
    }

    #[test]
    fn invalid_fields_fall_back_one_by_one() {
        let s = Settings::from_json(
            r#"{"opacity": 0.1, "symbol": "XAUUSD", "tf": "4h", "size": [5, 5],
                "pos": ["a", 1], "account": "lots", "risk_pct": "200", "details_open": true,
                "limits": {"max_per_day": 0, "pause_after_losses": 2, "daily_loss_r": -1}}"#,
        );
        assert!((s.opacity - pal::MIN_OPACITY).abs() < 1e-6, "clamped");
        assert_eq!(s.symbol, "BTCUSDT", "unsupported symbol not restored");
        assert_eq!(s.tf, Timeframe::M15, "unsupported time frame not restored");
        assert_eq!((s.size, s.pos), (None, None));
        assert_eq!(s.account, "10,000");
        assert_eq!(s.risk_pct, "1", "risk above 100 % refused");
        assert!(s.details_open);
        assert_eq!(s.limits.max_per_day, 3);
        assert_eq!(s.limits.pause_after_losses, 2);
        assert!((s.limits.daily_loss_r - 3.0).abs() < 1e-12);
    }

    /// The settings persist: saved, loaded back identical, and only these keys are stored (no
    /// prices, no account data beyond the two sizing numbers).
    #[test]
    fn settings_persist_through_the_file() {
        let dir = std::env::temp_dir().join(format!("cs-ui-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(Settings::load(&dir), Settings::default());
        let s = sample();
        s.save(&dir).unwrap();
        assert_eq!(Settings::load(&dir), s);
        let text = std::fs::read_to_string(path(&dir)).unwrap();
        let v: Value = serde_json::from_str(&text).unwrap();
        let mut keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "about",
                "account",
                "details_open",
                "limits",
                "opacity",
                "paper_since_ms",
                "pos",
                "risk_pct",
                "screen_reading",
                "size",
                "symbol",
                "tf"
            ]
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
