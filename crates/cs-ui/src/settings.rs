//! The overlay's small local settings file, `data/overlay-settings.json` (git-ignored, never
//! sent anywhere): window position and size, opacity, compact/expanded, account size, risk %,
//! the last symbol and time frame, and whether screen reading is on. Read at start, written
//! when something changed (at most every 2 s). A missing or broken file means defaults.

use std::path::{Path, PathBuf};

use cs_core::Timeframe;
use serde_json::{Value, json};

use crate::palette as pal;

#[derive(Debug, Clone, PartialEq)]
pub struct Settings {
    /// Outer window position (screen points); `None` lets Windows place it.
    pub pos: Option<[f32; 2]>,
    /// Inner window size (points); `None` = the default for the mode.
    pub size: Option<[f32; 2]>,
    pub opacity: f32,
    pub compact: bool,
    pub account: String,
    pub risk_pct: String,
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
            compact: true,
            account: "10,000".into(),
            risk_pct: "1".into(),
            symbol: "BTCUSDT".into(),
            tf: Timeframe::M15,
            screen_reading: true,
        }
    }
}

pub fn path(root: &Path) -> PathBuf {
    root.join("data").join("overlay-settings.json")
}

#[allow(clippy::cast_possible_truncation)]
fn pair(v: &Value, lo: f32, hi: f32) -> Option<[f32; 2]> {
    let a = v.as_array()?;
    let x = a.first()?.as_f64()? as f32;
    let y = a.get(1)?.as_f64()? as f32;
    (x.is_finite() && y.is_finite() && (lo..=hi).contains(&x) && (lo..=hi).contains(&y))
        .then_some([x, y])
}

/// Short text field: kept only if it is a plain number the calculator can read.
fn number_text(v: Option<&Value>, default: &str) -> String {
    v.and_then(Value::as_str)
        .filter(|s| s.len() <= 20 && crate::risk::parse(s).is_some_and(|x| x > 0.0))
        .map_or_else(|| default.to_string(), str::to_string)
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
        Settings {
            pos: v.get("pos").and_then(|p| pair(p, -20_000.0, 20_000.0)),
            size: v.get("size").and_then(|p| pair(p, 200.0, 4_000.0)),
            opacity,
            compact: v
                .get("compact")
                .and_then(Value::as_bool)
                .unwrap_or(d.compact),
            account: number_text(v.get("account"), &d.account),
            risk_pct: number_text(v.get("risk_pct"), &d.risk_pct),
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
            "compact": self.compact,
            "account": self.account,
            "risk_pct": self.risk_pct,
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
            && self.compact == o.compact
            && self.account == o.account
            && self.risk_pct == o.risk_pct
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
            size: Some([440.0, 720.0]),
            opacity: 0.87,
            compact: false,
            account: "25,000".into(),
            risk_pct: "0.5".into(),
            symbol: "ETHUSDT".into(),
            tf: Timeframe::H1,
            screen_reading: false,
        }
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
                "pos": ["a", 1], "account": "lots", "risk_pct": "2", "compact": false}"#,
        );
        assert!((s.opacity - pal::MIN_OPACITY).abs() < 1e-6, "clamped");
        assert_eq!(s.symbol, "BTCUSDT", "unsupported symbol not restored");
        assert_eq!(s.tf, Timeframe::M15, "unsupported time frame not restored");
        assert_eq!((s.size, s.pos), (None, None));
        assert_eq!(s.account, "10,000");
        assert_eq!(s.risk_pct, "2");
        assert!(!s.compact);
    }

    #[test]
    fn save_and_load_through_a_file() {
        let dir = std::env::temp_dir().join(format!("cs-ui-settings-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(Settings::load(&dir), Settings::default());
        let s = sample();
        s.save(&dir).unwrap();
        assert_eq!(Settings::load(&dir), s);
        let text = std::fs::read_to_string(path(&dir)).unwrap();
        // Only these keys are stored (no prices, no account data beyond the two numbers).
        let v: Value = serde_json::from_str(&text).unwrap();
        let mut keys: Vec<&str> = v.as_object().unwrap().keys().map(String::as_str).collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "about",
                "account",
                "compact",
                "opacity",
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
