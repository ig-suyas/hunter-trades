//! Recorder status for the footer, read from `data/live` (the Phase 1 recorder's output): how
//! many hours of order-book (heatmap) data exist and whether the recorder is still writing.

use std::collections::BTreeSet;
use std::path::Path;

#[derive(Debug, Clone, Copy, Default)]
pub struct RecorderInfo {
    /// Distinct UTC hours with an order-book (`depth20_um`) file. An hour counts if any of it
    /// was recorded, so this slightly overstates coverage across sleeps and restarts.
    pub hours: u32,
    pub days: f64,
    /// Seconds since the recorder's last message (from `status.json`), if known.
    pub last_age_s: Option<i64>,
}

/// Scan `<data>/live`.
pub fn scan(data: &Path, now_ms: i64) -> RecorderInfo {
    let live = data.join("live");
    let mut hours = BTreeSet::new();
    if let Ok(days) = std::fs::read_dir(live.join("depth20_um")) {
        for d in days.flatten() {
            if let Ok(files) = std::fs::read_dir(d.path()) {
                for f in files.flatten() {
                    let n = f.file_name().to_string_lossy().into_owned();
                    // depth20_um-2026-11-27T03-030000.jsonl(.gz) -> "2026-11-27T03"
                    if let Some(rest) = n.strip_prefix("depth20_um-")
                        && let Some(h) = rest.get(..13)
                    {
                        hours.insert(h.to_string());
                    }
                }
            }
        }
    }
    let last_age_s = std::fs::read_to_string(live.join("status.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| {
            v["connections"].as_object().and_then(|c| {
                c.values()
                    .filter_map(|x| x["last_msg_utc"].as_str())
                    .filter_map(cs_core::time::parse_datetime)
                    .max()
            })
        })
        .map(|t| (now_ms - t) / 1000);
    let h = u32::try_from(hours.len()).unwrap_or(u32::MAX);
    RecorderInfo {
        hours: h,
        days: f64::from(h) / 24.0,
        last_age_s,
    }
}
