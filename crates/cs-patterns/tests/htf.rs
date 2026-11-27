//! Higher-time-frame trend state and the healthy-pullback flag, checked against independent
//! batch / brute-force reference implementations (the streaming prefix-invariance checks for
//! both live in `tests/prefix.rs`).
//!
//! HTF reference: aggregate the base series into complete UTC buckets in one batch pass, run the
//! EMA state and structure over that list, and require that the streaming state at every base
//! candle equals the state after the last HTF candle that had *completed* by that candle's close,
//! with `known_at` = the open time of the base candle that ended the bucket. Data with random
//! missing candles makes some buckets incomplete; those must never be used.

// Test code: panics are the failure mode, sizes are tiny, printing is the report.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stdout,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_precision_loss,
    clippy::cast_sign_loss,
    clippy::similar_names
)]

use std::path::PathBuf;

use cs_core::{Candle, Event};
use cs_patterns::context::ContextParams;
use cs_patterns::htf::{HtfParams, HtfTrend};
use cs_patterns::pullback::{HEALTHY, Pullback, PullbackParams};
use cs_patterns::structure::{Structure, StructureParams};
use cs_patterns::testutil::{STEP_MS, T0, synthetic};
use cs_patterns::trend::{EmaParams, EmaState};
use cs_patterns::util::Rng;
use cs_patterns::{Detector, Runner, run_all};

const H1: i64 = 3_600_000;
const H4: i64 = 4 * H1;

/// Batch aggregation into complete buckets: (HTF candle, open time of the base candle that ended it).
fn batch(base: &[Candle], base_ms: i64, htf_ms: i64) -> Vec<(Candle, i64)> {
    let need = htf_ms / base_ms;
    let mut out = Vec::new();
    let mut k = 0;
    while k < base.len() {
        let b = base[k].open_time.div_euclid(htf_ms) * htf_ms;
        let mut j = k;
        while j < base.len() && base[j].open_time.div_euclid(htf_ms) * htf_ms == b {
            j += 1;
        }
        let grp = &base[k..j];
        let complete =
            grp.len() as i64 == need && grp.last().unwrap().open_time + base_ms == b + htf_ms;
        if complete {
            let mut a = Candle {
                open_time: b,
                ..grp[0]
            };
            for c in &grp[1..] {
                a.high = a.high.max(c.high);
                a.low = a.low.min(c.low);
                a.close = c.close;
                a.volume += c.volume;
                a.quote_volume += c.quote_volume;
                a.taker_buy_base += c.taker_buy_base;
                a.taker_buy_quote += c.taker_buy_quote;
                a.trades = if a.trades < 0 || c.trades < 0 {
                    -1
                } else {
                    a.trades + c.trades
                };
            }
            out.push((a, grp.last().unwrap().open_time));
        }
        k = j;
    }
    out
}

fn with_gaps(mut v: Vec<Candle>, seed: u64, rate: u64) -> Vec<Candle> {
    let mut r = Rng::new(seed);
    v.retain(|_| r.below(rate) != 0);
    v
}

fn retime(mut v: Vec<Candle>, step: i64) -> Vec<Candle> {
    for (i, c) in v.iter_mut().enumerate() {
        c.open_time = T0 + i as i64 * step;
    }
    v
}

/// Returns (base candles checked, HTF candles completed, base candles with EMA-ready state).
fn check_against_batch(base: &[Candle], base_ms: i64, htf_ms: i64) -> (usize, usize, usize) {
    let ref_c = batch(base, base_ms, htf_ms);
    let mut ema = EmaState::new(EmaParams::default());
    let mut st = Structure::new(StructureParams::default());
    let mut sink: Vec<Event> = Vec::new();
    let mut ref_s = Vec::new();
    for (n, (c, _)) in ref_c.iter().enumerate() {
        ema.step(n as u32, c, &mut sink);
        st.step(n as u32, c, &mut sink);
        ref_s.push((ema.snap(), st.trend()));
    }
    let mut t = HtfTrend::new(HtfParams {
        base_ms,
        htf_ms,
        ..HtfParams::default()
    });
    let mut done = 0usize;
    let mut ready = 0usize;
    for (i, c) in base.iter().enumerate() {
        t.step(i as u32, c);
        let s = t.snap();
        // Completed by this candle's close: buckets whose last base candle is at or before it.
        while done < ref_c.len() && ref_c[done].1 <= c.open_time {
            done += 1;
        }
        assert_eq!(s.n as usize, done, "base {i}: completed HTF candle count");
        if done == 0 {
            assert_eq!(s.candle_open, i64::MIN);
            continue;
        }
        let (hc, known) = ref_c[done - 1];
        let (e, tr) = ref_s[done - 1];
        assert_eq!(s.candle_open, hc.open_time, "base {i}: HTF candle used");
        assert_eq!(s.known_at, known, "base {i}: known_at");
        assert_eq!(
            s.known_at + base_ms,
            s.candle_open + htf_ms,
            "known_at ends the bucket"
        );
        assert!(
            s.known_at <= c.open_time,
            "state from the future at base {i}"
        );
        assert_eq!(s.close.to_bits(), hc.close.to_bits());
        assert_eq!(s.structure, tr);
        assert_eq!(s.ema.ready, e.ready);
        assert_eq!(s.ema.stack, e.stack);
        for (x, y) in [
            (s.ema.fast, e.fast),
            (s.ema.mid, e.mid),
            (s.ema.slow, e.slow),
            (s.ema.slope_fast, e.slope_fast),
            (s.ema.slope_mid, e.slope_mid),
            (s.ema.slope_slow, e.slope_slow),
        ] {
            assert_eq!(x.to_bits(), y.to_bits(), "base {i}: EMA value differs");
        }
        if s.ema.ready {
            ready += 1;
        }
    }
    (base.len(), ref_c.len(), ready)
}

#[test]
fn htf_state_matches_batch_reference() {
    for seed in [1u64, 2, 3] {
        let base15 = with_gaps(synthetic(12_000, seed), seed + 100, 400);
        for htf in [H1, H4] {
            let (n, k, ready) = check_against_batch(&base15, STEP_MS, htf);
            let expected_full = 12_000 / (htf / STEP_MS) as usize;
            println!(
                "htf reference 15m->{}h seed {seed}: {n} base, {k} complete HTF candles \
                 (of ~{expected_full}), {ready} base candles with EMA ready: PASS",
                htf / H1
            );
            assert!(k > 0 && k < expected_full, "gaps must drop some buckets");
        }
        let base5 = retime(synthetic(12_000, seed + 7), 300_000);
        let base5 = with_gaps(base5, seed + 200, 500);
        for htf in [H1, H4] {
            let (n, k, _) = check_against_batch(&base5, 300_000, htf);
            println!(
                "htf reference 5m->{}h seed {seed}: {n} base, {k} HTF candles: PASS",
                htf / H1
            );
        }
    }
}

#[test]
fn htf_inactive_when_not_higher() {
    let t = HtfTrend::new(HtfParams {
        base_ms: H1,
        htf_ms: H1,
        ..HtfParams::default()
    });
    assert!(!t.active());
    let t = HtfTrend::new(HtfParams {
        base_ms: 7 * 60_000,
        htf_ms: H1,
        ..HtfParams::default()
    });
    assert!(!t.active(), "1h is not a multiple of 7m");
}

#[test]
fn context_features_carry_htf_state() {
    let base = synthetic(6000, 9);
    let mut r = Runner::new(vec![], Some(ContextParams::for_base(STEP_MS)));
    let mut out = Vec::new();
    let mut seen_ready = [false; 2];
    for c in &base {
        let f = r.step(c, &mut out).unwrap();
        for (slot, s) in f.htf.iter().enumerate() {
            if s.n > 0 {
                assert!(s.known_at <= c.open_time);
                assert_eq!(s.known_at + STEP_MS, s.candle_open + s.htf_ms);
            }
            seen_ready[slot] |= s.ema.ready;
        }
    }
    assert!(
        seen_ready[0] && seen_ready[1],
        "both HTF EMA states become ready"
    );
}

// ---------------------------------------------------------------------------------------------
// Healthy pullback: brute-force reference
// ---------------------------------------------------------------------------------------------

/// Brute-force fractal swing lows/highs (k = 3): (index, price, known index).
fn swings(cs: &[Candle], high: bool) -> Vec<(usize, f64, usize)> {
    let k = 3;
    let mut v = Vec::new();
    for j in k..cs.len().saturating_sub(k) {
        let x = |m: usize| if high { cs[m].high } else { cs[m].low };
        let ok = (j - k..j).all(|m| if high { x(j) > x(m) } else { x(j) < x(m) })
            && (j + 1..=j + k).all(|m| if high { x(j) >= x(m) } else { x(j) <= x(m) });
        if ok {
            v.push((j, x(j), j + k));
        }
    }
    v
}

fn healthy_reference(
    cs: &[Candle],
    sw_lo: &[(usize, f64, usize)],
    sw_hi: &[(usize, f64, usize)],
    e: &Event,
    p: &PullbackParams,
) -> (Option<(usize, usize)>, bool) {
    let i = e.known_idx as usize;
    let up = e.dir > 0;
    // B: extreme of bars i-1 .. i-(swing_bars-1), latest on ties.
    let mut b: Option<usize> = None;
    for k in 1..p.swing_bars.min(i + 1) {
        let j = i - k;
        let better = b.is_none_or(|bb| {
            if up {
                cs[j].high > cs[bb].high
            } else {
                cs[j].low < cs[bb].low
            }
        });
        if better {
            b = Some(j);
        }
    }
    let Some(b) = b else { return (None, false) };
    let mut a: Option<usize> = None;
    for j in (b.saturating_sub(p.impulse_bars)..b).rev() {
        let better = a.is_none_or(|aa| {
            if up {
                cs[j].low < cs[aa].low
            } else {
                cs[j].high > cs[aa].high
            }
        });
        if better {
            a = Some(j);
        }
    }
    let Some(a) = a else { return (None, false) };
    let (pa, pb_top) = if up {
        (cs[a].low, cs[b].high)
    } else {
        (cs[a].high, cs[b].low)
    };
    let imp = (pb_top - pa).abs();
    let seg = &cs[b..=i];
    let ext = if up {
        seg.iter().map(|c| c.low).fold(f64::INFINITY, f64::min)
    } else {
        seg.iter().map(|c| c.high).fold(f64::NEG_INFINITY, f64::max)
    };
    let pb = (pb_top - ext).abs();
    let smaller = imp > 0.0 && pb < p.healthy_max_ratio * imp;
    let slower = i > b && b > a && pb / ((i - b) as f64) < imp / ((b - a) as f64);
    let sw = if up { sw_lo } else { sw_hi };
    let last = sw
        .iter()
        .filter(|s| s.2 <= i && s.0 < b)
        .max_by_key(|s| s.2);
    let holds = last.is_some_and(|s| {
        seg.iter()
            .all(|c| if up { c.close >= s.1 } else { c.close <= s.1 })
    });
    (Some((a, b)), smaller && slower && holds)
}

fn real_data() -> Option<Vec<Candle>> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/store/resampled/um/BTCUSDT/15m.csc");
    let t = cs_data::store::read_file(&p).ok()?;
    cs_data::store::table_to_candles(&t).ok().map(|x| x.0)
}

#[test]
fn healthy_flag_matches_brute_force() {
    let p = PullbackParams::default();
    let mut sets = vec![
        synthetic(8000, 21),
        synthetic(8000, 22),
        synthetic(8000, 23),
    ];
    if let Some(r) = real_data() {
        sets.push(r);
    }
    let (mut total, mut healthy) = (0, 0);
    for cs in &sets {
        let ev = run_all(vec![Box::new(Pullback::new(p)) as Box<dyn Detector>], cs);
        let (sw_lo, sw_hi) = (swings(cs, false), swings(cs, true));
        for e in ev.iter().filter(|e| e.det == "pullback") {
            let (ab, want) = healthy_reference(cs, &sw_lo, &sw_hi, e, &p);
            let got = e.tags & HEALTHY != 0;
            assert_eq!(
                got, want,
                "healthy flag differs at bar {} ({})",
                e.known_idx, e.kind
            );
            if let Some((a, b)) = ab {
                assert_eq!((e.ax as usize, e.bx as usize), (a, b), "impulse anchors");
            }
            total += 1;
            healthy += usize::from(got);
        }
    }
    println!(
        "healthy-pullback reference: {total} pullback events checked, {healthy} healthy: PASS"
    );
    assert!(
        healthy > 0 && healthy < total,
        "the flag must take both values"
    );
}
