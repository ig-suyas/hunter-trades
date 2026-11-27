//! Per-candle evaluation latency of the live engine: all 13 detector groups, the context
//! features and the snapshot, for one closed candle, after a full warm-up.
//!
//! `cargo bench -p cs-live --bench per_candle` prints criterion's estimate and then p50 / p99 /
//! max over 20,000 individually timed candles. Network round trip is measured separately
//! (`cargo run -p cs-live --release --example net_rtt`).

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stdout,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]

use std::hint::black_box;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use criterion::{Criterion, criterion_group};
use cs_core::Candle;
use cs_live::engine::LiveEngine;

const TF: i64 = 900_000;

/// Pre-seal BTCUSDT 15m candles from the local store, or a synthetic series.
fn data() -> (Vec<Candle>, &'static str) {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let boundary = std::fs::read_to_string(root.join("reports/holdout-seal.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v["boundary_ms"].as_i64());
    if let (Some(b), Ok(t)) = (
        boundary,
        cs_data::store::read_file(&root.join("data/store/resampled/um/BTCUSDT/15m.csc")),
    ) && let Ok((c, _)) = cs_data::store::table_to_candles(&t)
    {
        let c: Vec<Candle> = c.into_iter().filter(|c| c.open_time + TF <= b).collect();
        return (c, "BTCUSDT 15m (store, before the seal)");
    }
    (cs_patterns::testutil::synthetic(40_000, 3), "synthetic")
}

/// Endless candle source: cycles the data with the time shifted forward each lap.
struct Cycle {
    c: Vec<Candle>,
    k: usize,
    lap: i64,
}

impl Cycle {
    fn next(&mut self) -> Candle {
        if self.k == self.c.len() {
            self.k = 0;
            self.lap += 1;
        }
        let mut x = self.c[self.k];
        x.open_time += self.lap * (self.c.len() as i64 + 1) * TF;
        self.k += 1;
        x
    }
}

fn warmed() -> (LiveEngine, Cycle) {
    let (c, _) = data();
    let mut e = LiveEngine::new(TF);
    let warm = 4000.min(c.len() / 2);
    for x in &c[..warm] {
        let _ = e.push_closed(x);
    }
    (e, Cycle { k: warm, c, lap: 0 })
}

fn bench(cr: &mut Criterion) {
    let (mut e, mut src) = warmed();
    cr.bench_function("per_candle_eval", |b| {
        b.iter_custom(|iters| {
            let mut t = Duration::ZERO;
            for _ in 0..iters {
                let c = src.next();
                let s = Instant::now();
                let r = e.push_closed(black_box(&c)).map(|s| s.snapshot.close);
                t += s.elapsed();
                black_box(r.ok());
            }
            t
        });
    });
}

criterion_group!(benches, bench);

fn main() {
    benches();
    Criterion::default().configure_from_args().final_summary();
    // Percentiles over individually timed candles.
    let (_, what) = data();
    let (mut e, mut src) = warmed();
    let n = 20_000;
    let mut t: Vec<f64> = Vec::with_capacity(n);
    for _ in 0..n {
        let c = src.next();
        let s = Instant::now();
        let r = e.push_closed(black_box(&c)).map(|s| s.snapshot.close);
        t.push(s.elapsed().as_secs_f64() * 1e6);
        black_box(r.ok());
    }
    t.sort_by(f64::total_cmp);
    let q = |p: f64| t[((n as f64 - 1.0) * p).round() as usize];
    println!(
        "per-candle evaluation, {what}, {n} candles after a 4,000-candle warm-up: p50 {:.1} us, p99 {:.1} us, max {:.1} us, mean {:.1} us",
        q(0.50),
        q(0.99),
        t[n - 1],
        t.iter().sum::<f64>() / n as f64
    );
}
