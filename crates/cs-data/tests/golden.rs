//! Golden test: tiny hand-built kline archive -> checksum -> unzip -> parse -> clean ->
//! validate -> resample. The output must match `fixtures/golden-pipeline.txt` exactly.
//! Regenerate (after checking the diff by hand) with `UPDATE_GOLDEN=1 cargo test -p cs-data`.

#![allow(clippy::unwrap_used, clippy::expect_used)] // test-only helper
//!
//! The fixture (90 minutes from 2024-11-01 00:00 UTC) contains on purpose: a CSV header, an
//! exact duplicate (00:30), a swapped pair (00:40/00:41), a 3-minute gap (00:50-00:52), a
//! zero-volume candle (01:00), a 2% wick (01:10) and one row with microsecond timestamps (01:20).

use std::fmt::Write as _;
use std::path::Path;

use cs_core::Timeframe;
use cs_core::time::fmt_minute;
use cs_data::resample::{Policy, resample};
use cs_data::{clean, parse, validate, vision, zipread};

fn pipeline() -> String {
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let zip = dir.join("BTCUSDT-1m-2024-11-01.zip");
    let sha = vision::verify(&zip).expect("checksum must match");
    let (name, bytes) = zipread::first_entry(&std::fs::read(&zip).unwrap()).unwrap();
    let (raw, notes) = parse::klines_csv(std::str::from_utf8(&bytes).unwrap()).unwrap();
    let (ck, rep) = clean::klines_from_table(&raw.into_table(vec![])).unwrap();
    let q = validate::kline_quality(&ck.candles);

    let mut s = String::new();
    writeln!(s, "entry {name} sha256 {sha}").unwrap();
    writeln!(
        s,
        "header_lines {} raw_rows {}",
        notes.header_lines, rep.rows_in
    )
    .unwrap();
    writeln!(
        s,
        "repairs R1 {} R2 {} R3 {} R4 {} R5 {} rows_out {}",
        rep.r1_unit_us_to_ms,
        rep.r2_misaligned_dropped,
        rep.r3_out_of_order,
        rep.r4_exact_dupes,
        rep.r5_conflicting_dupes,
        rep.rows_out
    )
    .unwrap();
    writeln!(s, "close_time_mismatch {}", ck.close_time_mismatch).unwrap();
    writeln!(
        s,
        "quality rows {} expected {} gaps {} missing {} zero_volume {} zero_trades {} ohlc_bad {} outlier_wicks {} extreme_wicks {}",
        q.rows,
        q.expected,
        q.gaps,
        q.missing_minutes,
        q.zero_volume,
        q.zero_trades,
        q.ohlc_bad,
        q.outlier_wicks,
        q.extreme_wicks
    )
    .unwrap();
    for (t, n) in &q.largest_gaps {
        writeln!(s, "gap {} {n}", fmt_minute(*t)).unwrap();
    }
    for (t, n) in &q.zero_volume_runs {
        writeln!(s, "zero-volume run {} {n}", fmt_minute(*t)).unwrap();
    }
    for (t, f) in &q.top_wicks {
        writeln!(s, "wick {} {:.4}", fmt_minute(*t), f).unwrap();
    }
    for tf in [Timeframe::M5, Timeframe::M15, Timeframe::H1] {
        for policy in [Policy::Strict, Policy::AllowGaps] {
            let (c, mins, st) = resample(&ck.candles, tf, policy);
            writeln!(
                s,
                "{} {policy:?}: emitted {} incomplete {} edge {}",
                tf.label(),
                st.emitted,
                st.incomplete,
                st.edge_dropped
            )
            .unwrap();
            if policy == Policy::Strict || tf == Timeframe::M5 {
                for (k, m) in c.iter().zip(&mins) {
                    writeln!(
                        s,
                        "  {} o {:.2} h {:.2} l {:.2} c {:.2} v {:.3} n {} tb {:.4} min {m}",
                        fmt_minute(k.open_time),
                        k.open,
                        k.high,
                        k.low,
                        k.close,
                        k.volume,
                        k.trades,
                        k.taker_buy_base
                    )
                    .unwrap();
                }
            }
        }
    }
    s
}

#[test]
fn golden_pipeline() {
    let got = pipeline();
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/golden-pipeline.txt");
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, &got).unwrap();
    }
    let want =
        std::fs::read_to_string(&path).expect("golden file missing: run with UPDATE_GOLDEN=1");
    assert_eq!(got.replace("\r\n", "\n"), want.replace("\r\n", "\n"));
}
