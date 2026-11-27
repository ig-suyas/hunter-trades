//! Parity: one historical week replayed through the live path gives exactly the batch output.
//!
//! * Live path: kline websocket frames (several in-progress frames per candle, then the closed
//!   frame), reconnect replays of closed frames, frames for other symbols and intervals, mark
//!   price frames, and a disconnect that loses three candles (recovered by the backfill), all
//!   through [`Pipeline::on_text`], after the same REST-style warm-up the transport does.
//! * Batch: the engine over the same candles in one go, and the backtest's own
//!   `Market::prepare` (context features and detector events), compared bit for bit.
//!
//! Data: BTCUSDT USDS-M 15m from the local store, **cut at the holdout seal before anything is
//! computed** (the sealed period is never used). Skipped when the store is absent.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stdout,
    clippy::too_many_lines
)]

use std::path::PathBuf;

use cs_backtest::market::{DetectorSource, Market};
use cs_core::{Candle, Timeframe, time};
use cs_data::store;
use cs_live::engine::LiveEngine;
use cs_live::feed::default_warmup;
use cs_live::kline::kline_frame;
use cs_live::pipeline::Pipeline;
use cs_live::snapshot::ContextSnapshot;
use sha2::{Digest, Sha256};

const TF: i64 = 900_000;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

/// Pre-boundary BTCUSDT 15m candles, or `None` if the store or seal is missing.
fn load_sealed() -> Option<Vec<Candle>> {
    let seal = std::fs::read_to_string(root().join("reports/holdout-seal.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&seal).ok()?;
    let boundary = v["boundary_ms"].as_i64()?;
    let p = root().join("data/store/resampled/um/BTCUSDT/15m.csc");
    let t = store::read_file(&p).ok()?;
    let (c, _) = store::table_to_candles(&t).ok()?;
    Some(
        c.into_iter()
            .filter(|c| c.open_time + TF <= boundary)
            .collect(),
    )
}

fn partial(c: &Candle, step: u32) -> Candle {
    // In-progress views of the candle: open only, then half way, then nearly done.
    let mut p = *c;
    match step {
        0 => {
            p.high = c.open;
            p.low = c.open;
            p.close = c.open;
            p.volume = 0.0;
        }
        1 => {
            p.close = f64::midpoint(c.open, c.close);
            p.high = p.close.max(c.open);
            p.low = p.close.min(c.open);
            p.volume = c.volume * 0.5;
        }
        _ => p.volume = c.volume * 0.9,
    }
    p
}

#[test]
fn live_path_matches_batch_for_one_week() {
    let Some(all) = load_sealed() else {
        println!("parity: SKIPPED (no local store or seal)");
        return;
    };
    let week_start = time::ymd_hms_to_ms(2026, 3, 2, 0, 0, 0);
    let week_end = time::ymd_hms_to_ms(2026, 3, 9, 0, 0, 0);
    let s = all.iter().position(|c| c.open_time >= week_start).unwrap();
    let e = all.iter().position(|c| c.open_time >= week_end).unwrap();
    let warm_n = default_warmup(Timeframe::M15);
    let input: Vec<Candle> = all[s - warm_n..e].to_vec();
    let week = &all[s..e];
    assert_eq!(week.len(), 672, "one week of 15m candles");

    // ---- live path ----------------------------------------------------------------------
    let mut pipe = Pipeline::new("BTCUSDT", "15m", TF);
    let mut live: Vec<ContextSnapshot> = pipe.push_closed(&all[s - warm_n..s]);
    let mut backfill_calls = 0;
    let lost = 200..203; // a disconnect loses every frame of these three candles
    let mut frames = 0usize;
    for (k, c) in week.iter().enumerate() {
        if lost.contains(&k) {
            continue;
        }
        let mut send = |txt: String, pipe: &mut Pipeline, live: &mut Vec<ContextSnapshot>| {
            frames += 1;
            let mut bf = |from: i64, to: i64| {
                backfill_calls += 1;
                week.iter()
                    .filter(|x| x.open_time >= from && x.open_time < to)
                    .copied()
                    .collect::<Vec<_>>()
            };
            live.extend(pipe.on_text(&txt, &mut bf));
        };
        for step in 0..3 {
            let t = c.open_time + i64::from(step + 1) * 200_000;
            send(
                kline_frame("BTCUSDT", "15m", TF, &partial(c, step), false, t),
                &mut pipe,
                &mut live,
            );
        }
        // Noise that must be ignored: another symbol, another interval, a mark price frame.
        send(
            kline_frame("ETHUSDT", "15m", TF, c, true, c.open_time + TF),
            &mut pipe,
            &mut live,
        );
        send(
            kline_frame("BTCUSDT", "1m", 60_000, c, true, c.open_time + TF),
            &mut pipe,
            &mut live,
        );
        send(
            format!(
                r#"{{"stream":"btcusdt@markPrice@1s","data":{{"e":"markPriceUpdate","E":{},"s":"BTCUSDT","p":"{}","r":"0.0001","T":0}}}}"#,
                c.open_time + 1000,
                c.close
            ),
            &mut pipe,
            &mut live,
        );
        send(
            kline_frame("BTCUSDT", "15m", TF, c, true, c.open_time + TF),
            &mut pipe,
            &mut live,
        );
        if k % 50 == 49 {
            // Reconnect: the exchange replays the last closed frames.
            for old in &week[k - 1..=k] {
                send(
                    kline_frame("BTCUSDT", "15m", TF, old, true, old.open_time + TF),
                    &mut pipe,
                    &mut live,
                );
            }
        }
    }

    // ---- batch ----------------------------------------------------------------------------
    let mut eng = LiveEngine::new(TF);
    let mut batch = Vec::new();
    let mut feats = Vec::new();
    let mut events = Vec::new();
    for c in &input {
        let st = eng.push_closed(c).unwrap();
        feats.push(st.features);
        events.extend_from_slice(st.events);
        batch.push(st.snapshot);
    }
    let market = Market::prepare(input.clone(), TF, &DetectorSource::default_set()).unwrap();

    // Live == batch, snapshot by snapshot.
    assert_eq!(live.len(), batch.len(), "every candle applied exactly once");
    let mut mism = 0;
    for (a, b) in live.iter().zip(&batch) {
        if a.exact() != b.exact() {
            mism += 1;
        }
    }
    assert_eq!(mism, 0, "snapshot mismatches");
    // The engine's features and events == the backtest's batch preparation, bit for bit.
    assert_eq!(feats.len(), market.features.len());
    assert!(
        feats.iter().zip(&market.features).all(|(a, b)| a.same(b)),
        "features differ from Market::prepare"
    );
    assert_eq!(
        events.len(),
        market.events.len(),
        "event count differs from Market::prepare"
    );
    assert!(
        events.iter().zip(&market.events).all(|(a, b)| a.same(b)),
        "events differ from Market::prepare"
    );

    let st = pipe.stats;
    assert_eq!(st.backfilled, 3);
    assert_eq!(st.gaps_unfilled, 0);
    assert_eq!(st.duplicates, 2 * 13);
    assert_eq!(st.rejected, 0);
    assert_eq!(st.partial, 3 * 669);
    assert_eq!(backfill_calls, 1);

    let mut h = Sha256::new();
    for x in &live[warm_n..] {
        h.update(x.exact().as_bytes());
    }
    let hash = h.finalize().iter().fold(String::new(), |mut acc, b| {
        use std::fmt::Write as _;
        let _ = write!(acc, "{b:02x}");
        acc
    });
    let week_events = events.iter().filter(|e| e.known_at >= week_start).count();

    // Information only: how much does the shorter live warm-up change the week's context
    // compared with a warm-up over all earlier (pre-seal) history?
    let full = cs_live::engine::batch_snapshots(&all[..e], TF);
    let full_week = &full[full.len() - 672..];
    let cat = |x: &ContextSnapshot| {
        format!(
            "{:?}{:?}{}{}{}{}",
            x.htf.map(|h| (h.stack, h.structure, h.ready)),
            x.regime,
            x.sweep.map_or(0, |m| m.known_at),
            x.bos.map_or(0, |m| m.known_at),
            x.choch.map_or(0, |m| m.known_at),
            x.pullback.map_or(0, |m| m.known_at)
        )
    };
    let same_cat = full_week
        .iter()
        .zip(&live[warm_n..])
        .filter(|(a, b)| cat(a) == cat(b))
        .count();
    println!(
        "parity: PASS, week 2026-03-02..03-09 BTCUSDT 15m, {} warm-up + 672 candles, {frames} frames \
         ({} in-progress, {} replays dropped, {} backfilled after a lost-frames gap), 0 of {} snapshots differ, \
         features and {} events ({week_events} in the week) bit-identical to Market::prepare, week snapshot sha256 {hash}; \
         trend/regime/marker fields equal to a full-history warm-up on {same_cat} of 672 candles",
        warm_n,
        st.partial,
        st.duplicates,
        st.backfilled,
        live.len(),
        events.len()
    );
}
