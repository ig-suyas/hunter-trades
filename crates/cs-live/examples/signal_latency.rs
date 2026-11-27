//! Per-candle latency of the signal path, release build: each pre-seal candle as its closed
//! websocket frame plus the next candle's first (open-only) frame through the live pipeline
//! (context engine + the frozen rules' decision through the backtest's `simulate`). Prints p50,
//! p99 and max per time frame. Data: the local store, cut at the holdout seal.
//!
//! `cargo run -p cs-live --release --example signal_latency`

#![allow(
    clippy::unwrap_used,
    clippy::print_stdout,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::path::PathBuf;
use std::time::Instant;

use cs_core::{Candle, Timeframe, time};
use cs_data::store;
use cs_live::feed::default_warmup;
use cs_live::kline::kline_frame;
use cs_live::pipeline::Pipeline;

fn main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let seal = std::fs::read_to_string(root.join("reports/holdout-seal.json")).unwrap();
    let v: serde_json::Value = serde_json::from_str(&seal).unwrap();
    let b = v["boundary_ms"].as_i64().unwrap();
    let start = time::ymd_hms_to_ms(2026, 1, 1, 0, 0, 0);
    for tf in [Timeframe::M15, Timeframe::H1] {
        let t = store::read_file(&root.join(format!(
            "data/store/resampled/um/BTCUSDT/{}.csc",
            tf.label()
        )))
        .unwrap();
        let (all, _) = store::table_to_candles(&t).unwrap();
        let all: Vec<Candle> = all
            .into_iter()
            .filter(|c| c.open_time + tf.ms() <= b)
            .collect();
        let s = all.iter().position(|c| c.open_time >= start).unwrap();
        let warm = default_warmup(tf);
        let mut p = Pipeline::new("BTCUSDT", tf.label(), tf.ms());
        p.push_closed(&all[s - warm..s]);
        let mut us = Vec::new();
        for k in s..all.len() - 1 {
            let c = all[k];
            let n = all[k + 1];
            let mut o = n;
            o.high = n.open;
            o.low = n.open;
            o.close = n.open;
            let closed = kline_frame(
                "BTCUSDT",
                tf.label(),
                tf.ms(),
                &c,
                true,
                c.open_time + tf.ms(),
            );
            let first = kline_frame(
                "BTCUSDT",
                tf.label(),
                tf.ms(),
                &o,
                false,
                n.open_time + 1_000,
            );
            let t0 = Instant::now();
            p.on_text(&closed, &mut |_, _| Vec::new());
            p.on_text(&first, &mut |_, _| Vec::new());
            us.push(t0.elapsed().as_secs_f64() * 1e6);
        }
        us.sort_by(f64::total_cmp);
        let q = |x: f64| us[((us.len() - 1) as f64 * x).round() as usize];
        let alerts = p.signal_engine().map_or(0, |e| e.signals().len());
        println!(
            "{} BTCUSDT 2026-01-01..seal: {} candles, {} alerts; per candle (closed frame + next open, context + {} rules): p50 {:.0} us, p99 {:.0} us, max {:.0} us",
            tf.label(),
            us.len(),
            alerts,
            p.signal_engine().map_or(0, |e| e.rules().len()),
            q(0.5),
            q(0.99),
            us.last().copied().unwrap_or(f64::NAN)
        );
    }
}
