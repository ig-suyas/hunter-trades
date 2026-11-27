//! Signal view: the live path against the backtest, on one historical week before the holdout.
//!
//! * Parity: one week (BTCUSDT 2026-03-02 .. 03-09, pre-seal) replayed as websocket frames
//!   through the live path gives exactly the backtest's trades of the 10 frozen rules: same rule,
//!   candle, direction, entry time, entry, stop and target, bit for bit. Twice: with the live
//!   warm-up (the backtest over the same candles) and with a warm-up over all earlier pre-seal
//!   history (the research's own validation market).
//! * `known_at`: every alert is rebuilt from the candles up to its `known_at` candle plus the next
//!   open only (with a wild in-progress frame), and is identical.
//! * No repainting: after every frame the alert list is checked to be an append-only extension
//!   of the list before, field by field, and an ended trade never changes again.
//! * Outcomes: the journal's resolver (the backtest's `simulate` on a rebuilt market) gives the
//!   backtest's own exit and net R for every alert, with 1-minute candles and funding.
//!
//! Data: the local store **cut at the holdout seal before anything is computed** (the sealed
//! period is never used). Skipped when the store is absent.

#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stdout,
    clippy::too_many_lines,
    clippy::cast_possible_truncation,
    clippy::float_cmp
)]

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::OnceLock;

use cs_backtest::cost::{CostModel, Funding};
use cs_backtest::market::{DetectorSource, Market};
use cs_backtest::sim::{ExitReason, Inputs, Sizing, Trade, simulate};
use cs_core::{Candle, Timeframe, time};
use cs_data::{DataDir, Market as DataMarket, clean, store};
use cs_live::feed::default_warmup;
use cs_live::journal::{self, LogRow, RowCtx};
use cs_live::kline::kline_frame;
use cs_live::pipeline::Pipeline;
use cs_live::signal::rules::rules_for;
use cs_live::signal::{Signal, TradeState};

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
}

fn boundary() -> Option<i64> {
    let seal = std::fs::read_to_string(root().join("reports/holdout-seal.json")).ok()?;
    let v: serde_json::Value = serde_json::from_str(&seal).ok()?;
    v["boundary_ms"].as_i64()
}

/// Pre-boundary candles of one time frame.
fn load_bars(sym: &str, tf: Timeframe) -> Option<Vec<Candle>> {
    let b = boundary()?;
    let p = root().join(format!("data/store/resampled/um/{sym}/{}.csc", tf.label()));
    let t = store::read_file(&p).ok()?;
    let (c, _) = store::table_to_candles(&t).ok()?;
    Some(
        c.into_iter()
            .filter(|c| c.open_time + tf.ms() <= b)
            .collect(),
    )
}

/// Pre-boundary 1-minute candles and funding (cut before use).
fn load_m1_funding(sym: &str) -> (Vec<Candle>, Vec<Funding>) {
    let b = boundary().unwrap();
    let dd = DataDir::new(root().join("data"));
    let m1: Vec<Candle> = clean::klines(&dd, DataMarket::Um, sym)
        .map(|k| k.candles)
        .unwrap_or_default()
        .into_iter()
        .filter(|c| c.open_time < b)
        .collect();
    let mut funding = Vec::new();
    if let Ok((ft, _, _)) = clean::series(&dd.funding(sym), None)
        && ft.nrows() > 0
    {
        let ts = ft.i64("time").unwrap();
        let rs = ft.f64("rate").unwrap();
        funding = ts
            .iter()
            .zip(rs)
            .filter(|(t, _)| **t < b)
            .map(|(&time, &rate)| Funding { time, rate })
            .collect();
    }
    (m1, funding)
}

const WEEK_START: i64 = time::ymd_hms_to_ms(2026, 3, 2, 0, 0, 0);
const WEEK_END: i64 = time::ymd_hms_to_ms(2026, 3, 9, 0, 0, 0);

fn partial(c: &Candle, step: u32) -> Candle {
    let mut p = *c;
    if step == 0 {
        // First frame of a candle: the open only.
        p.high = c.open;
        p.low = c.open;
        p.close = c.open;
        p.volume = 0.0;
    } else {
        p.close = f64::midpoint(c.open, c.close);
        p.high = p.close.max(c.open);
        p.low = p.close.min(c.open);
        p.volume = c.volume * 0.5;
    }
    p
}

/// Same alert: every field equal, floats bit for bit (`NaN` = no target / no trailing exit).
fn same_alert(a: &Signal, b: &Signal) -> bool {
    let bits =
        |s: &Signal| [s.entry, s.stop, s.target, s.risk, s.trail_from, s.atr].map(f64::to_bits);
    bits(a) == bits(b) && format!("{a:?}") == format!("{b:?}")
}

struct Replay {
    alerts: Vec<Signal>,
    frames: usize,
    prefix_checks: usize,
    state_checks: usize,
}

/// Warm-up `warm` (REST style), then every candle of `live` as websocket frames: the open-only
/// frame one second after the open, a mid frame, noise (another symbol, another interval, a mark
/// price), the closed frame; reconnect replays every 50 candles; the candles in `lose` lost
/// (recovered by the backfill when the next closed frame arrives). After every frame the alert
/// list must extend the previous one without changing anything.
fn replay(sym: &str, tf: Timeframe, warm: &[Candle], live: &[Candle], lose: &[usize]) -> Replay {
    let tfm = tf.ms();
    let iv = tf.label();
    let mut pipe = Pipeline::new(sym, iv, tfm);
    pipe.push_closed(warm);
    let mut prev: Vec<Signal> = Vec::new();
    let mut prev_states: Vec<TradeState> = Vec::new();
    let mut r = Replay {
        alerts: Vec::new(),
        frames: 0,
        prefix_checks: 0,
        state_checks: 0,
    };
    for (k, c) in live.iter().enumerate() {
        if lose.contains(&k) {
            continue;
        }
        let mut frames = vec![
            kline_frame(sym, iv, tfm, &partial(c, 0), false, c.open_time + 1_000),
            kline_frame(sym, iv, tfm, &partial(c, 1), false, c.open_time + tfm / 2),
            kline_frame("ETHUSDT", iv, tfm, c, true, c.open_time + tfm),
            kline_frame(sym, "1m", 60_000, c, true, c.open_time + tfm),
            format!(
                r#"{{"stream":"x@markPrice@1s","data":{{"e":"markPriceUpdate","E":{},"s":"{sym}","p":"{}","r":"0.0001","T":0}}}}"#,
                c.open_time + 2_000,
                c.close
            ),
            kline_frame(sym, iv, tfm, c, true, c.open_time + tfm),
        ];
        if k % 50 == 49 {
            for old in &live[k - 1..=k] {
                frames.push(kline_frame(sym, iv, tfm, old, true, old.open_time + tfm));
            }
        }
        for f in frames {
            r.frames += 1;
            let mut bf = |from: i64, to: i64| {
                live.iter()
                    .filter(|x| x.open_time >= from && x.open_time < to)
                    .copied()
                    .collect::<Vec<_>>()
            };
            pipe.on_text(&f, &mut bf);
            let se = pipe.signal_engine().unwrap();
            let now = se.signals();
            assert!(now.len() >= prev.len(), "an alert was removed");
            for (a, b) in prev.iter().zip(now) {
                assert!(same_alert(a, b), "alert changed: {a:?} -> {b:?}");
                r.prefix_checks += 1;
            }
            for (a, b) in prev_states.iter().zip(se.states()) {
                if let TradeState::Ended { .. } = a {
                    assert_eq!(a, b, "an ended trade changed");
                    r.state_checks += 1;
                }
            }
            prev = now.to_vec();
            prev_states = se.states().to_vec();
        }
    }
    r.alerts = prev;
    if let Some(e) = &pipe.signal_engine().unwrap().last_error {
        println!("signal engine error: {e}");
    }
    r
}

/// The backtest's trades of the frozen rules on `bars` (the research's way: `Market::prepare`,
/// setup events flipped for reversed rules, `simulate`).
fn backtest_trades(
    tf: Timeframe,
    bars: &[Candle],
    m1: &[Candle],
    funding: &[Funding],
) -> (Market, Vec<(u8, Trade)>) {
    let m = Market::prepare(bars.to_vec(), tf.ms(), &DetectorSource::default_set()).unwrap();
    let mut mk = m.clone();
    let mut out = Vec::new();
    for def in rules_for(tf) {
        mk.events = m
            .events
            .iter()
            .filter(|e| def.matches(e))
            .map(|e| def.trade_event(e))
            .collect();
        let run = simulate(
            &Inputs {
                market: &mk,
                m1,
                funding,
                costs: CostModel::BINANCE_UM,
                sizing: Sizing::default(),
            },
            &def.trade_rule(),
        )
        .unwrap();
        out.extend(run.trades.into_iter().map(|t| (def.order, t)));
    }
    (m, out)
}

fn key(rule: u8, known_at: i64) -> (i64, u8) {
    (known_at, rule)
}

/// A backtest trade the live path cannot know at its decision time: the previous trade of the
/// same rule ended *inside* the next candle (by a touch, not at its open), and the engine stamps
/// such an exit at the candle's open (without 1-minute data) or at the open of the 1-minute candle
/// (with them), which equals the decision time when it happens in the first minute. Its
/// one-trade-at-a-time check (`busy > decision`) then lets the new trade through, using price
/// action after the decision. The live engine only knows the open at that moment, so the
/// previous trade is still running and the new one is skipped.
fn needs_later_data(m: &Market, trades: &[(u8, Trade)], rule: u8, t: &Trade) -> bool {
    let prev = trades
        .iter()
        .filter(|(r, x)| *r == rule && x.signal_idx < t.signal_idx)
        .map(|(_, x)| x)
        .next_back();
    let Some(p) = prev else {
        return false;
    };
    if p.exit_time != t.decision_at {
        return false;
    }
    let open = m.bars[t.signal_idx as usize + 1].open;
    let sd = f64::from(p.dir);
    let at_open = match p.reason {
        ExitReason::Time | ExitReason::EndOfData => true,
        // A gapped stop fills at the open itself.
        ExitReason::Stop | ExitReason::Trail => p.exit_raw.to_bits() == open.to_bits(),
        // A gapped target fills at the target with the open beyond it.
        ExitReason::Target => sd * (open - p.target) > 0.0,
    };
    !at_open
}

/// Compare the alerts of the week with the backtest trades of the week: every alert is a backtest
/// trade, bit for bit; every backtest trade without an alert is one that needed later data
/// ([`needs_later_data`]). Returns (alerts, late alerts, backtest trades needing later data).
fn compare(
    alerts: &[Signal],
    m: &Market,
    trades: &[(u8, Trade)],
    what: &str,
) -> (usize, usize, usize) {
    let want: BTreeMap<(i64, u8), &Trade> = trades
        .iter()
        .filter(|(_, t)| t.known_at >= WEEK_START && t.known_at < WEEK_END)
        .map(|(r, t)| (key(*r, t.known_at), t))
        .collect();
    let got: BTreeMap<(i64, u8), &Signal> = alerts
        .iter()
        .filter(|s| s.known_at >= WEEK_START && s.known_at < WEEK_END)
        .map(|s| (key(s.rule, s.known_at), s))
        .collect();
    let extra: Vec<_> = got.keys().filter(|k| !want.contains_key(k)).collect();
    assert!(
        extra.is_empty(),
        "{what}: alerts that are not backtest trades: {extra:?}"
    );
    let mut later = 0;
    for (k, t) in &want {
        if !got.contains_key(k) {
            assert!(
                needs_later_data(m, trades, k.1, t),
                "{what}: backtest trade {k:?} has no alert and did not need later data"
            );
            later += 1;
        }
    }
    for (k, s) in &got {
        let t = want[k];
        assert_eq!(s.dir, t.dir, "{what} {k:?}");
        assert_eq!(s.entry_time, t.entry_time, "{what} {k:?}");
        assert_eq!(s.decision_at, t.decision_at, "{what} {k:?}");
        assert_eq!(
            s.entry.to_bits(),
            t.entry_raw.to_bits(),
            "{what} {k:?} entry"
        );
        assert_eq!(s.stop.to_bits(), t.stop0.to_bits(), "{what} {k:?} stop");
        assert_eq!(
            s.target.to_bits(),
            t.target.to_bits(),
            "{what} {k:?} target"
        );
        assert_eq!(s.risk.to_bits(), t.risk.to_bits(), "{what} {k:?} risk");
    }
    (got.len(), got.values().filter(|s| !s.fresh).count(), later)
}

struct Case {
    tf: Timeframe,
    all: Vec<Candle>,
    s: usize,
    e: usize,
    warm_n: usize,
    lose: Vec<usize>,
    live: Replay,
}

/// The exact-window replay of each time frame (shared by the tests below).
fn cases() -> Option<&'static Vec<Case>> {
    static C: OnceLock<Option<Vec<Case>>> = OnceLock::new();
    C.get_or_init(|| {
        let mut v = Vec::new();
        for tf in [Timeframe::M15, Timeframe::H1] {
            let all = load_bars("BTCUSDT", tf)?;
            let s = all.iter().position(|c| c.open_time >= WEEK_START)?;
            let e = all.iter().position(|c| c.open_time >= WEEK_END)?;
            let warm_n = default_warmup(tf);
            // A disconnect loses three candles in the middle of the week.
            let lose = vec![(e - s) / 3, (e - s) / 3 + 1, (e - s) / 3 + 2];
            // Week candles plus the first candle after it (its open decides the last one).
            let live = replay("BTCUSDT", tf, &all[s - warm_n..s], &all[s..=e], &lose);
            v.push(Case {
                tf,
                all,
                s,
                e,
                warm_n,
                lose,
                live,
            });
        }
        Some(v)
    })
    .as_ref()
}

#[test]
fn live_alerts_equal_backtest_trades_for_one_week() {
    let Some(cases) = cases() else {
        println!("signal parity: SKIPPED (no local store or seal)");
        return;
    };
    let (m1, funding) = load_m1_funding("BTCUSDT");
    for c in cases {
        let tf = c.tf;
        let alerts = &c.live.alerts;
        // 1) The backtest over the same candles (no 1-minute data, like the live engine).
        let bars = &c.all[c.s - c.warm_n..=c.e];
        let (m, trades) = backtest_trades(tf, bars, &[], &[]);
        let (n, late, later) = compare(alerts, &m, &trades, "same window");
        assert!(n > 0, "{}: no alert in the week", tf.label());
        // 2) The research's own validation market: all pre-seal history (from 2024-11-01) as
        //    warm-up, with 1-minute candles and funding, exactly as `research --stage validate`.
        let full = replay("BTCUSDT", tf, &c.all[..c.s], &c.all[c.s..=c.e], &c.lose);
        let (rm, rtrades) = backtest_trades(tf, &c.all, &m1, &funding);
        let (rn, rlate, rlater) = compare(&full.alerts, &rm, &rtrades, "research market");
        // How often the research's own trades (months 1-18, these rules) needed later data.
        let all_later = rtrades
            .iter()
            .filter(|(r, t)| needs_later_data(&rm, &rtrades, *r, t))
            .count();
        let per_rule: BTreeMap<u8, usize> = alerts
            .iter()
            .filter(|s| s.known_at >= WEEK_START && s.known_at < WEEK_END)
            .fold(BTreeMap::new(), |mut m, s| {
                *m.entry(s.rule).or_insert(0) += 1;
                m
            });
        println!(
            "signal parity {} BTCUSDT week 2026-03-02..03-09, {} frames (in-progress, closed, noise, replays, a 3-candle gap): \
             (1) same candles, {} warm-up: {n} alerts = {n} backtest trades bit for bit ({late} late), per rule {per_rule:?}; \
             {later} backtest trade(s) without an alert, each one needing data after its decision; \
             (2) research market ({} pre-seal warm-up candles, 1m + funding): {rn} alerts = {rn} research trades bit for bit ({rlate} late), \
             {rlater} research trade(s) in the week needed later data; in all months 1-18 of these rules: {all_later} of {} research trades",
            tf.label(),
            c.live.frames,
            c.warm_n,
            c.s,
            rtrades.len()
        );
    }
}

#[test]
fn alerts_use_nothing_after_known_at() {
    let Some(cases) = cases() else {
        return;
    };
    let mut checked = 0;
    for c in cases {
        let tf = c.tf;
        let tfm = tf.ms();
        let start = c.s - c.warm_n;
        for a in &c.live.alerts {
            assert_eq!(a.entry_time, a.known_at + tfm, "entry at the next open");
            assert_eq!(a.decision_at, a.known_at + tfm);
            assert!(a.decided_at >= a.known_at + tfm, "decided before the close");
            let k = c
                .all
                .iter()
                .position(|x| x.open_time == a.known_at)
                .unwrap();
            // Candles up to the known_at candle only, then one in-progress frame of the next
            // candle whose high, low and close are wild: only its open may matter.
            let mut p = Pipeline::new("BTCUSDT", tf.label(), tfm);
            p.push_closed(&c.all[start..=k]);
            let next = c.all[k + 1];
            let mut wild = next;
            wild.high = next.open * 1.5;
            wild.low = next.open * 0.5;
            wild.close = next.open * 1.4;
            let f = kline_frame(
                "BTCUSDT",
                tf.label(),
                tfm,
                &wild,
                false,
                next.open_time + 1_000,
            );
            p.on_text(&f, &mut |_, _| Vec::new());
            let got = p.signal_engine().unwrap().signals();
            let b = got
                .iter()
                .find(|x| x.rule == a.rule && x.known_at == a.known_at)
                .unwrap_or_else(|| panic!("alert {} not rebuilt from data up to known_at", a.id));
            assert_eq!(
                (
                    b.dir,
                    b.entry_time,
                    b.entry.to_bits(),
                    b.stop.to_bits(),
                    b.target.to_bits()
                ),
                (
                    a.dir,
                    a.entry_time,
                    a.entry.to_bits(),
                    a.stop.to_bits(),
                    a.target.to_bits()
                ),
                "{}",
                a.id
            );
            checked += 1;
        }
    }
    println!(
        "known_at: {checked} alerts rebuilt from candles <= known_at + the next open: identical"
    );
    assert!(checked > 0);
}

#[test]
fn alerts_are_never_repainted() {
    let Some(cases) = cases() else {
        return;
    };
    for c in cases {
        assert!(c.live.prefix_checks > 0);
        println!(
            "no repaint {}: {} frames, alert list checked after every frame ({} alert comparisons, {} ended-trade checks): append-only, nothing changed or removed",
            c.tf.label(),
            c.live.frames,
            c.live.prefix_checks,
            c.live.state_checks
        );
    }
}

#[test]
fn outcomes_resolve_like_the_backtest() {
    let Some(cases) = cases() else {
        return;
    };
    let (m1, funding) = load_m1_funding("BTCUSDT");
    for c in cases {
        let tf = c.tf;
        let start = c.s - c.warm_n;
        // Enough candles after the week for every trade to end (time stop 96 candles).
        let end = (c.e + 200).min(c.all.len());
        let bars = &c.all[start..end];
        let (_, trades) = backtest_trades(tf, bars, &m1, &funding);
        let mut n = 0;
        for a in &c.live.alerts {
            let row = LogRow::from_signal(
                a,
                &RowCtx {
                    logged_at: a.decided_at,
                    decision: journal::decision::SHOWN.into(),
                    account: 10_000.0,
                    risk_pct: 1.0,
                    ..RowCtx::default()
                },
            );
            let o = journal::resolve(bars, &m1, &funding, &row)
                .unwrap()
                .expect("trade ended");
            let (_, t) = trades
                .iter()
                .find(|(r, t)| *r == a.rule && t.known_at == a.known_at)
                .unwrap();
            assert_eq!(o.exit_time, t.exit_time, "{}", a.id);
            assert_eq!(o.reason, t.reason.label(), "{}", a.id);
            assert_eq!(o.net_r_1x.to_bits(), t.net_r.to_bits(), "{}", a.id);
            n += 1;
        }
        println!(
            "outcomes {}: {n} alerts resolved by the journal = the backtest's exit time, reason and net R (with 1m candles and funding)",
            tf.label()
        );
        assert!(n > 0);
    }
}
