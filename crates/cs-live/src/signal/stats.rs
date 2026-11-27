//! Past results of the frozen rules, read from `reports/rule-stats.json`.
//!
//! That file is written by `candlescope rule-stats` from the DISCOVERY and VALIDATION periods
//! only (2024-11-01 .. 2026-05-27, loaded with the research's sealed access, so the holdout is
//! never read). These are in-sample, historical numbers; they are never a forecast.

use std::path::{Path, PathBuf};

use serde_json::Value;

/// Below this many trades a number is flagged "low evidence".
pub const LOW_EVIDENCE_N: u64 = 100;

/// Results of one rule on one symbol over one period.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PeriodStats {
    pub n: u64,
    pub hits: u64,
    pub hit_rate: f64,
    /// 80 % Beta-Binomial credible interval of the hit rate (uniform prior).
    pub hit_ci80: (f64, f64),
    /// Mean net R per trade after costs at 1x and 2x, with 80 % block-bootstrap intervals.
    pub exp_1x: f64,
    pub exp_1x_ci80: (f64, f64),
    pub exp_2x: f64,
    pub exp_2x_ci80: (f64, f64),
    /// Mean cost per trade in R at 1x (gross minus net).
    pub cost_1x: f64,
    /// Median and 75th percentile time in trade, minutes.
    pub hold_median_min: f64,
    pub hold_p75_min: f64,
}

impl PeriodStats {
    pub fn low_evidence(&self) -> bool {
        self.n < LOW_EVIDENCE_N
    }
}

/// One rule on one symbol: months 1-18 (discovery + validation together) and each period.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuleStats {
    pub all: PeriodStats,
    pub discovery: PeriodStats,
    pub validation: PeriodStats,
}

/// Everything in the file.
#[derive(Debug, Clone, Default)]
pub struct StatsBook {
    /// (rule order, symbol) -> stats.
    pub rows: Vec<(u8, String, RuleStats)>,
    pub created_utc: String,
    pub periods: String,
}

pub fn path(root: &Path) -> PathBuf {
    root.join("reports").join("rule-stats.json")
}

fn f(v: &Value) -> f64 {
    v.as_f64().unwrap_or(f64::NAN)
}

fn pair(v: &Value) -> (f64, f64) {
    (f(&v[0]), f(&v[1]))
}

fn period(v: &Value) -> Option<PeriodStats> {
    Some(PeriodStats {
        n: v["n"].as_u64()?,
        hits: v["hits"].as_u64()?,
        hit_rate: f(&v["hit_rate"]),
        hit_ci80: pair(&v["hit_ci80"]),
        exp_1x: f(&v["exp_1x"]),
        exp_1x_ci80: pair(&v["exp_1x_ci80"]),
        exp_2x: f(&v["exp_2x"]),
        exp_2x_ci80: pair(&v["exp_2x_ci80"]),
        cost_1x: f(&v["cost_1x"]),
        hold_median_min: f(&v["hold_median_min"]),
        hold_p75_min: f(&v["hold_p75_min"]),
    })
}

impl StatsBook {
    /// Parse the file's text.
    ///
    /// # Errors
    /// Not JSON, or a rule entry without the expected fields.
    pub fn parse(s: &str) -> Result<StatsBook, String> {
        let v: Value = serde_json::from_str(s.trim_start_matches('\u{feff}'))
            .map_err(|e| format!("rule-stats.json: {e}"))?;
        let mut rows = Vec::new();
        for r in v["rules"].as_array().into_iter().flatten() {
            let order = r["order"]
                .as_u64()
                .and_then(|x| u8::try_from(x).ok())
                .ok_or("rule-stats.json: rule without order")?;
            for (sym, x) in r["symbols"].as_object().into_iter().flatten() {
                let st = RuleStats {
                    all: period(&x["months_1_18"]).ok_or("rule-stats.json: months_1_18")?,
                    discovery: period(&x["discovery"]).ok_or("rule-stats.json: discovery")?,
                    validation: period(&x["validation"]).ok_or("rule-stats.json: validation")?,
                };
                rows.push((order, sym.clone(), st));
            }
        }
        Ok(StatsBook {
            rows,
            created_utc: v["created_utc"].as_str().unwrap_or_default().to_string(),
            periods: v["periods_text"].as_str().unwrap_or_default().to_string(),
        })
    }

    /// Load from the project root.
    ///
    /// # Errors
    /// Missing or unreadable file.
    pub fn load(root: &Path) -> Result<StatsBook, String> {
        let p = path(root);
        let s = std::fs::read_to_string(&p).map_err(|e| format!("{}: {e}", p.display()))?;
        StatsBook::parse(&s)
    }

    pub fn get(&self, order: u8, symbol: &str) -> Option<&RuleStats> {
        self.rows
            .iter()
            .find(|(o, s, _)| *o == order && s == symbol)
            .map(|(_, _, r)| r)
    }
}
