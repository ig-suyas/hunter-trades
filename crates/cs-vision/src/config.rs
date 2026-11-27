//! The region config (`config/screen-regions.json` in the project folder): which windows count
//! as a trading platform, the four regions that are read, the regions that must never be read,
//! the platform-to-exchange symbol table and the price tolerance.

use std::collections::BTreeMap;
use std::path::Path;

use serde_json::Value;

use crate::frame::RectF;

/// The copy that ships with the code (same file as in the project folder).
pub const DEFAULT_JSON: &str = include_str!("../../../config/screen-regions.json");

/// Path of the config inside the project folder.
pub const CONFIG_PATH: &str = "config/screen-regions.json";

#[derive(Debug, Clone, PartialEq)]
pub struct Platform {
    pub name: String,
    /// Lower-case parts of the program name that may host the platform (e.g. "chrome").
    pub apps: Vec<String>,
    /// The window title contains one of these (case-insensitive).
    pub title_contains: Vec<String>,
    /// Also accept `TradingView`'s chart title shape: "TICKER 60,626.47 ▲ +0.41% ...".
    pub title_chart_pattern: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Regions {
    pub symbol: RectF,
    pub legend: RectF,
    pub page: RectF,
    pub price_axis: RectF,
}

impl Regions {
    pub fn all(&self) -> [(&'static str, RectF); 4] {
        [
            ("symbol", self.symbol),
            ("legend", self.legend),
            ("page", self.page),
            ("price_axis", self.price_axis),
        ]
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisionConfig {
    pub fps: f64,
    pub title_poll_ms: u64,
    pub ocr_scale: u32,
    pub price_tolerance_pct: f64,
    pub platforms: Vec<Platform>,
    pub regions: Regions,
    pub exclude: Vec<(String, RectF)>,
    /// Platform ticker (upper case) -> exchange symbol.
    pub symbols: BTreeMap<String, String>,
    /// Normalised legend description (letters only, lower case) -> platform ticker.
    pub descriptions: BTreeMap<String, String>,
}

fn num(v: &Value, k: &str) -> Result<f64, String> {
    v.get(k)
        .and_then(Value::as_f64)
        .ok_or_else(|| format!("missing number {k:?}"))
}

#[allow(clippy::cast_possible_truncation)]
fn rect(v: &Value, what: &str) -> Result<RectF, String> {
    let g = |k| {
        num(v, k)
            .map(|x| x as f32)
            .map_err(|e| format!("{what}: {e}"))
    };
    Ok(RectF::new(g("x")?, g("y")?, g("w")?, g("h")?))
}

fn strings(v: Option<&Value>) -> Vec<String> {
    v.and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn table(v: Option<&Value>) -> BTreeMap<String, String> {
    v.and_then(Value::as_object)
        .map(|o| {
            o.iter()
                .filter_map(|(k, x)| x.as_str().map(|s| (k.clone(), s.to_string())))
                .collect()
        })
        .unwrap_or_default()
}

impl VisionConfig {
    /// Parses and validates a config.
    ///
    /// # Errors
    /// Bad JSON, a missing field, a region outside the window, or a read region that overlaps
    /// an excluded one.
    pub fn from_json(s: &str) -> Result<VisionConfig, String> {
        let v: Value = serde_json::from_str(s).map_err(|e| format!("config: {e}"))?;
        let cap = v.get("capture").ok_or("config: missing capture")?;
        let r = v.get("regions").ok_or("config: missing regions")?;
        let reg = |k: &str| {
            rect(
                r.get(k)
                    .ok_or_else(|| format!("config: missing region {k}"))?,
                k,
            )
        };
        let mut platforms = Vec::new();
        for p in v
            .get("platforms")
            .and_then(Value::as_array)
            .ok_or("config: missing platforms")?
        {
            platforms.push(Platform {
                name: p
                    .get("name")
                    .and_then(Value::as_str)
                    .ok_or("config: platform without name")?
                    .to_string(),
                apps: strings(p.get("apps"))
                    .into_iter()
                    .map(|s| s.to_lowercase())
                    .collect(),
                title_contains: strings(p.get("title_contains")),
                title_chart_pattern: p
                    .get("title_chart_pattern")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            });
        }
        let mut exclude = Vec::new();
        for e in v
            .get("exclude")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            let name = e
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("unnamed")
                .to_string();
            exclude.push((name.clone(), rect(e, &name)?));
        }
        let scale = num(cap, "ocr_scale").unwrap_or(2.0);
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let ocr_scale = scale.clamp(1.0, 4.0) as u32;
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let title_poll_ms = num(cap, "title_poll_ms")?.clamp(50.0, 2000.0) as u64;
        let c = VisionConfig {
            fps: num(cap, "fps")?,
            title_poll_ms,
            ocr_scale,
            price_tolerance_pct: num(&v, "price_tolerance_pct")?,
            platforms,
            regions: Regions {
                symbol: reg("symbol")?,
                legend: reg("legend")?,
                page: reg("page")?,
                price_axis: reg("price_axis")?,
            },
            exclude,
            symbols: table(v.get("symbols"))
                .into_iter()
                .map(|(k, x)| (k.to_uppercase(), x.to_uppercase()))
                .collect(),
            descriptions: table(v.get("descriptions")),
        };
        c.validate()?;
        Ok(c)
    }

    /// The built-in default.
    ///
    /// # Errors
    /// Only if the shipped file is broken (a test guards against that).
    pub fn default_config() -> Result<VisionConfig, String> {
        VisionConfig::from_json(DEFAULT_JSON)
    }

    /// `<root>/config/screen-regions.json` if it exists, else the built-in default.
    ///
    /// # Errors
    /// The file exists but is invalid (the caller shows the message; screen reading stays off).
    pub fn load(root: &Path) -> Result<VisionConfig, String> {
        match std::fs::read_to_string(root.join(CONFIG_PATH)) {
            Ok(s) => VisionConfig::from_json(&s).map_err(|e| format!("{CONFIG_PATH}: {e}")),
            Err(_) => VisionConfig::default_config(),
        }
    }

    /// # Errors
    /// See [`VisionConfig::from_json`].
    pub fn validate(&self) -> Result<(), String> {
        if !(0.2..=4.0).contains(&self.fps) {
            return Err(format!("config: fps {} outside 0.2..4", self.fps));
        }
        if !(0.01..=5.0).contains(&self.price_tolerance_pct) {
            return Err(format!(
                "config: price_tolerance_pct {} outside 0.01..5",
                self.price_tolerance_pct
            ));
        }
        for (name, r) in self.regions.all() {
            if !r.valid() {
                return Err(format!("config: region {name} is outside the window"));
            }
            for (en, e) in &self.exclude {
                if r.intersects(e) {
                    return Err(format!(
                        "config: region {name} overlaps the excluded area \"{en}\""
                    ));
                }
            }
        }
        for (en, e) in &self.exclude {
            if !e.valid() {
                return Err(format!(
                    "config: excluded area \"{en}\" is outside the window"
                ));
            }
        }
        // Read regions cover only a small part of the window.
        let area: f32 = self.regions.all().iter().map(|(_, r)| r.w * r.h).sum();
        if area > 0.15 {
            return Err(format!(
                "config: the read regions cover {:.0} % of the window (limit 15 %)",
                area * 100.0
            ));
        }
        Ok(())
    }

    /// Capture interval.
    pub fn frame_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs_f64(1.0 / self.fps)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shipped_config_is_valid() {
        let c = VisionConfig::default_config().unwrap();
        assert!((c.price_tolerance_pct - 0.3).abs() < 1e-9);
        assert!((1.0..=2.0).contains(&c.fps));
        assert_eq!(c.symbols.get("BTCUSD").map(String::as_str), Some("BTCUSDT"));
        assert!(c.exclude.len() >= 4);
        // Every exchange symbol in the table is one the feed supports.
        for x in c.symbols.values() {
            assert!(["BTCUSDT", "ETHUSDT"].contains(&x.as_str()), "{x}");
        }
        // Every description maps to a ticker of the symbol table.
        for t in c.descriptions.values() {
            assert!(c.symbols.contains_key(t), "{t}");
        }
    }

    #[test]
    fn overlap_with_excluded_area_is_refused() {
        let s = DEFAULT_JSON.replace(
            "\"price_axis\": { \"x\": 0.875, \"y\": 0.140, \"w\": 0.044, \"h\": 0.700 }",
            "\"price_axis\": { \"x\": 0.875, \"y\": 0.140, \"w\": 0.044, \"h\": 0.800 }",
        );
        assert_ne!(s, DEFAULT_JSON);
        let e = VisionConfig::from_json(&s).unwrap_err();
        assert!(e.contains("bottom trading panel"), "{e}");
    }

    #[test]
    fn whole_window_region_is_refused() {
        let s = DEFAULT_JSON.replace(
            "\"page\": { \"x\": 0.380, \"y\": 0.095, \"w\": 0.240, \"h\": 0.030 }",
            "\"page\": { \"x\": 0.380, \"y\": 0.095, \"w\": 0.240, \"h\": 0.700 }",
        );
        assert_ne!(s, DEFAULT_JSON);
        assert!(VisionConfig::from_json(&s).is_err());
    }
}
