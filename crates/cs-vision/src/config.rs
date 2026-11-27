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
    /// Chart-tab title patterns ([`crate::parse::parse_title`]), e.g.
    /// `"{ticker} {price} {arrow} {change}"` for "BTCUSD 61,515 ▲ +1.69% - Google Chrome".
    pub title_patterns: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Regions {
    pub symbol: RectF,
    /// The toolbar's interval button ("5m", "D").
    pub interval: RectF,
    pub legend: RectF,
    pub page: RectF,
    pub price_axis: RectF,
}

impl Regions {
    pub fn all(&self) -> [(&'static str, RectF); 5] {
        [
            ("symbol", self.symbol),
            ("interval", self.interval),
            ("legend", self.legend),
            ("page", self.page),
            ("price_axis", self.price_axis),
        ]
    }
}

/// One window layout: where the regions are, and what must never be read.
#[derive(Debug, Clone, PartialEq)]
pub struct Layout {
    pub name: String,
    pub regions: Regions,
    pub exclude: Vec<(String, RectF)>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VisionConfig {
    pub fps: f64,
    pub title_poll_ms: u64,
    pub ocr_scale: u32,
    pub price_tolerance_pct: f64,
    pub platforms: Vec<Platform>,
    /// Layouts in order of preference; the reader uses the one that finds the chart (and keeps
    /// it while it does).
    pub layouts: Vec<Layout>,
    /// Words of the chart toolbar (letters only, lower case) that mark a chart page.
    pub page_words: Vec<String>,
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
        let mut layouts = Vec::new();
        for l in v
            .get("layouts")
            .and_then(Value::as_array)
            .ok_or("config: missing layouts")?
        {
            let name = l
                .get("name")
                .and_then(Value::as_str)
                .unwrap_or("unnamed layout")
                .to_string();
            let r = l
                .get("regions")
                .ok_or_else(|| format!("config: layout {name:?} has no regions"))?;
            let reg = |k: &str| {
                rect(
                    r.get(k)
                        .ok_or_else(|| format!("config: layout {name:?} misses region {k}"))?,
                    k,
                )
            };
            let regions = Regions {
                symbol: reg("symbol")?,
                interval: reg("interval")?,
                legend: reg("legend")?,
                page: reg("page")?,
                price_axis: reg("price_axis")?,
            };
            let mut exclude = Vec::new();
            for e in l
                .get("exclude")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
            {
                let en = e
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("unnamed")
                    .to_string();
                exclude.push((en.clone(), rect(e, &en)?));
            }
            layouts.push(Layout {
                name,
                regions,
                exclude,
            });
        }
        if layouts.is_empty() {
            return Err("config: no layout".into());
        }
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
                title_patterns: strings(p.get("title_patterns")),
            });
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
            layouts,
            page_words: strings(v.get("page_words"))
                .iter()
                .map(|w| crate::parse::letters(w))
                .filter(|w| !w.is_empty())
                .collect(),
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
        for l in &self.layouts {
            for (name, r) in l.regions.all() {
                if !r.valid() {
                    return Err(format!(
                        "config: region {name} of layout {:?} is outside the window",
                        l.name
                    ));
                }
                for (en, e) in &l.exclude {
                    if r.intersects(e) {
                        return Err(format!(
                            "config: region {name} of layout {:?} overlaps the excluded area \"{en}\"",
                            l.name
                        ));
                    }
                }
            }
            for (en, e) in &l.exclude {
                if !e.valid() {
                    return Err(format!(
                        "config: excluded area \"{en}\" is outside the window"
                    ));
                }
            }
            // Read regions cover only a small part of the window.
            let area: f32 = l.regions.all().iter().map(|(_, r)| r.w * r.h).sum();
            if area > 0.15 {
                return Err(format!(
                    "config: the read regions of layout {:?} cover {:.0} % of the window (limit 15 %)",
                    l.name,
                    area * 100.0
                ));
            }
        }
        for p in &self.platforms {
            for t in &p.title_patterns {
                let parts: Vec<&str> = t.split_whitespace().collect();
                if !parts.contains(&"{ticker}") || parts.len() < 3 {
                    return Err(format!(
                        "config: title pattern {t:?} of {} needs {{ticker}} and at least two more parts",
                        p.name
                    ));
                }
                if let Some(bad) = parts.iter().find(|x| {
                    x.starts_with('{')
                        && !["{ticker}", "{price}", "{arrow}", "{change}"].contains(x)
                }) {
                    return Err(format!("config: unknown part {bad} in title pattern {t:?}"));
                }
            }
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
        assert_eq!(c.layouts.len(), 2);
        for l in &c.layouts {
            assert!(l.exclude.len() >= 4, "{}", l.name);
        }
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
        let mut c = VisionConfig::default_config().unwrap();
        c.layouts[0].regions.price_axis.h = 0.80;
        let e = c.validate().unwrap_err();
        assert!(e.contains("bottom trading panel"), "{e}");
    }

    #[test]
    fn whole_window_region_is_refused() {
        let mut c = VisionConfig::default_config().unwrap();
        c.layouts[0].regions.page.h = 0.70;
        assert!(c.validate().unwrap_err().contains("15 %"));
    }

    #[test]
    fn title_patterns_and_page_words_come_from_the_file() {
        let c = VisionConfig::default_config().unwrap();
        let tv = &c.platforms[0];
        assert_eq!(tv.name, "TradingView");
        assert!(
            tv.title_patterns
                .iter()
                .any(|p| p == "{ticker} {price} {arrow} {change}")
        );
        for app in ["chrome", "msedge", "firefox", "tradingview"] {
            assert!(tv.apps.iter().any(|a| a == app), "{app}");
        }
        assert!(c.page_words.iter().any(|w| w == "indicators"));
        let mut bad = c.clone();
        bad.platforms[0].title_patterns = vec!["{price} {change} x".into()];
        assert!(bad.validate().is_err(), "a pattern without a ticker");
        bad.platforms[0].title_patterns = vec!["{ticker} {prize} x".into()];
        assert!(bad.validate().is_err(), "an unknown part");
    }
}
