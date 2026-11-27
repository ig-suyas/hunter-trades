//! Phase 3 self-checks (all must pass):
//! 1. hand-calculated scenario with known P&L;
//! 2. random-entry baseline: expectancy = minus costs within noise;
//! 3. lookahead canary: a strategy that reads the next candle's close is caught;
//! 4. cost monotonicity: higher costs never raise expectancy;
//! 5. determinism: identical hashes on repeated runs;
//!
//! Plus: no entry before the close of the `known_at` candle, and filters never use data after
//! `known_at`.
//!
//! Run `cargo test -p cs-backtest --test selfcheck -- --nocapture` to see the numbers.
// Test code: panics are the failure mode, sizes are tiny, printing is the report.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stdout,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::needless_range_loop,
    clippy::too_many_lines
)]

mod common;

use common::*;
use cs_backtest::cost::{CostModel, Funding};
use cs_backtest::market::{
    DetectorSource, Market, RandomSource, SignalSource, audit_cuts, audit_prefix,
};
use cs_backtest::metrics::{BootCfg, summarize};
use cs_backtest::output::{data_fingerprint, market_fingerprint, summary_json, trades_csv};
use cs_backtest::rule::{
    Align, DirSel, Entry, Filter, LimitAt, Overlap, Select, StopRule, Target, TradeRule, Trail,
};
use cs_backtest::sim::{ExitReason, simulate};
use cs_backtest::testkit::{T0, aggregate, funding_8h, random_walk_1m, regime_walk_1m};
use cs_backtest::{Error, backtest};
use cs_core::{Candle, Event};
use cs_patterns::Detector;
use cs_patterns::pullback::{Pullback, PullbackParams};
use cs_patterns::structure::{BreakParams, StructureBreak};

// ---------------------------------------------------------------- 1. hand-calculated scenario

#[test]
fn check1_hand_calculated_scenario() {
    // ATR(14) stays exactly 2.0: every candle has true range 2.
    let mut spec = flat(21, 100.0); // 0..=20
    spec.push((100.0, 101.5, 99.5, 101.0)); // 21 05:15 entry A at 100
    spec.push((101.0, 102.5, 100.5, 102.0)); // 22
    spec.push((102.0, 103.5, 101.5, 103.0)); // 23
    spec.push((103.0, 104.5, 102.5, 104.0)); // 24 06:00 high 104.5 > target 104
    spec.extend(flat(5, 104.0)); // 25..=29 (signal B at 29)
    spec.extend(flat(2, 104.0)); // 30 07:30 entry B at 104, 31
    spec.push((104.5, 106.0, 104.0, 105.5)); // 32 08:00 high touches stop 106
    spec.extend(vec![(105.5, 106.5, 104.5, 105.5); 3]); // 33..=35
    let bs = bars(&spec);
    // A: long, structural low 99 -> stop 99 - 0.5 * 2 = 98, risk 2, target 2R = 104.
    // B: short, structural high 105 -> stop 106, risk 2, target 100.
    let events = vec![ev(&bs, 20, 1, 99.0, 100.0), ev(&bs, 29, -1, 104.0, 105.0)];
    let m = market(bs, events);
    for k in 13..m.bars.len() {
        assert!(
            close(m.features[k].atr, 2.0),
            "ATR at {k} = {}",
            m.features[k].atr
        );
    }
    let h = 3_600_000;
    let funding = vec![
        Funding {
            time: T0,
            rate: 0.0005,
        }, // before A: not charged
        Funding {
            time: T0 + 8 * h,
            rate: 0.0001,
        }, // B is short across it: receives
        Funding {
            time: T0 + 16 * h,
            rate: 0.0005,
        }, // after the data
    ];
    let costs = CostModel {
        taker_fee_bps: 5.0,
        spread_bps: 2.0,
        slip_bps: 1.0,
        slip_atr: 0.1,
        mult: 1.0,
        funding: true,
    };
    let inp = inputs(&m, &[], &funding, costs);
    let run = simulate(&inp, &rule(0.5, Target::R(2.0))).unwrap();
    assert_eq!(run.trades.len(), 2);
    let (a, b) = (run.trades[0], run.trades[1]);

    // Trade A (by hand):
    // entry buy at 100: half spread 0.01, slippage 0.01 + 0.1 * 2 = 0.21 -> fill 100.22,
    //   fee 100.22 * 0.0005 = 0.05011
    // exit sell at the target 104: half spread 0.0104, slippage 0.0104 + 0.2 = 0.2104
    //   -> fill 103.7792, fee 0.0518896
    // gross 2 R; spread (0.01 + 0.0104) / 2 = 0.0102 R; slippage (0.21 + 0.2104) / 2 = 0.2102 R;
    // fee (0.05011 + 0.0518896) / 2 = 0.0509998 R; net 2 - 0.2713998 = 1.7286002 R
    // (a first hand calculation said 1.7285002; the engine disagreed and re-adding showed the
    // hand sum was wrong, so this check does catch arithmetic, in both directions)
    assert_eq!(a.entry_time, T0 + 5 * h + 15 * MIN);
    assert_eq!(a.exit_time, T0 + 6 * h);
    assert_eq!(a.reason, ExitReason::Target);
    assert!(close(a.stop0, 98.0) && close(a.target, 104.0) && close(a.risk, 2.0));
    assert!(close(a.entry_fill, 100.22));
    assert!(close(a.exit_fill, 103.7792));
    assert!(close(a.gross_r, 2.0));
    assert!(close(a.spread_r, 0.0102));
    assert!(close(a.slip_r, 0.2102));
    assert!(close(a.fee_r, 0.050_999_8));
    assert!(close(a.funding_r, 0.0) && a.funding_events == 0);
    assert!(close(a.net_r, 1.728_600_2), "{}", a.net_r);

    // Trade B (by hand):
    // entry sell at 104: half spread 0.0104, slippage 0.2104 -> fill 103.7792, fee 0.0518896
    // exit buy at the stop 106: half spread 0.0106, slippage 0.2106 -> fill 106.2212,
    //   fee 0.0531106
    // funding at 08:00 (entry 07:30 < 08:00 <= exit 08:00): price = open of the 08:00 candle
    //   104.5, short receives 104.5 * 0.0001 = 0.01045 -> -0.005225 R
    // gross -1 R; spread 0.0105 R; slippage 0.2105 R; fee 0.0525001 R;
    // net -1 - 0.0105 - 0.2105 - 0.0525001 + 0.005225 = -1.2682751 R
    assert_eq!(b.entry_time, T0 + 7 * h + 30 * MIN);
    assert_eq!(b.exit_time, T0 + 8 * h);
    assert_eq!(b.reason, ExitReason::Stop);
    assert!(close(b.entry_fill, 103.7792));
    assert!(close(b.exit_fill, 106.2212));
    assert!(close(b.gross_r, -1.0));
    assert!(close(b.spread_r, 0.0105));
    assert!(close(b.slip_r, 0.2105));
    assert!(close(b.fee_r, 0.052_500_1));
    assert_eq!(b.funding_events, 1);
    assert!(close(b.funding_r, -0.005_225));
    assert!(close(b.net_r, -1.268_275_1), "{}", b.net_r);

    // Account: 1 % risk of 10,000 -> A earns 172.86002; B risks 1 % of 10,172.86002 and loses
    // 10,172.86002 * 0.012682751 = 129.0198506; final 10,043.8401694
    assert!(close(a.pnl, 172.860_02));
    assert!(close(b.equity_before, 10_172.860_02));
    assert!((b.pnl + 129.019_850_6).abs() < 1e-6, "{}", b.pnl);

    let s = summarize(&run, TF, BootCfg::default());
    assert_eq!(
        (s.n, s.wins, s.losses, s.longest_losing_streak),
        (2, 1, 1, 1)
    );
    assert!(close(s.win_rate, 0.5));
    assert!(close(s.expectancy_r, 0.230_162_55));
    assert!(close(s.profit_factor, 1.728_600_2 / 1.268_275_1));
    assert!(close(s.max_dd_r, 1.268_275_1));
    assert!(
        (s.total_return - 0.004_384_016_94).abs() < 1e-9,
        "{}",
        s.total_return
    );
    println!(
        "check 1: A net {:.7} R (hand 1.7286002), B net {:.7} R (hand -1.2682751), \
         expectancy {:.8} R, total return {:.6} %",
        a.net_r,
        b.net_r,
        s.expectancy_r,
        100.0 * s.total_return
    );
}

// ---------------------------------------------------------------- 2. random-entry baseline

#[test]
fn check2_random_entry_baseline() {
    // One year of a driftless 1m random walk (a martingale), 15m setup candles, random entries
    // with a symmetric bracket: stop 1 ATR, target 1 R. Without information the gross
    // expectancy is 0, so the net expectancy must be minus the costs within noise.
    let m1s = random_walk_1m(15 * 35_040, 30_000.0, 0.0006, 11);
    let bs = aggregate(&m1s, TF);
    let src = RandomSource { seed: 5, prob: 0.5 };
    let m = Market::prepare(bs, TF, &src).unwrap();
    audit_prefix(&m, &src, &audit_cuts(m.bars.len(), 3, 1)).unwrap();
    let funding = funding_8h(m.bars[0].open_time, m.close_at(m.bars.len() - 1), 0.0001);
    let costs = CostModel::BINANCE_UM;
    let rule = TradeRule {
        name: "random".into(),
        select: Select::det("random"),
        entry: Entry::NextOpen,
        stop: StopRule::Atr { mult: 1.0 },
        target: Target::R(1.0),
        time_stop_bars: None,
        trail: None,
        filters: vec![],
        overlap: Overlap::OnePerSetup,
        min_stop_atr: 0.0,
    };
    let inp = inputs(&m, &m1s, &funding, costs);
    let run = simulate(&inp, &rule).unwrap();
    let s = summarize(&run, TF, BootCfg::default());
    let n = s.n as f64;
    let sd_g = {
        let mu = s.gross_expectancy_r;
        (run.trades
            .iter()
            .map(|t| (t.gross_r - mu).powi(2))
            .sum::<f64>()
            / (n - 1.0))
            .sqrt()
    };
    let se = sd_g / n.sqrt();
    // Independent cost estimate from the cost parameters and raw prices only:
    // per leg (fee + half spread + fixed slippage) of the price, plus slip_atr x ATR.
    let per_leg = (costs.taker_fee_bps + 0.5 * costs.spread_bps + costs.slip_bps) / 1e4;
    let predicted: f64 = run
        .trades
        .iter()
        .map(|t| {
            let atr = m.features[t.signal_idx as usize].atr;
            (per_leg * (t.entry_raw + t.exit_raw) + 2.0 * costs.slip_atr * atr) / t.risk
        })
        .sum::<f64>()
        / n;
    let txn: f64 = run
        .trades
        .iter()
        .map(|t| t.fee_r + t.spread_r + t.slip_r)
        .sum::<f64>()
        / n;
    println!(
        "check 2: {} trades, expectancy {:.4} R, gross {:.4} R (SE {:.4}), mean cost {:.4} R \
         (fee+spread+slip {:.4} R, funding {:.5} R), cost predicted from parameters {:.4} R, \
         95% CI [{:.4}, {:.4}]",
        s.n,
        s.expectancy_r,
        s.gross_expectancy_r,
        se,
        s.mean_cost_r,
        txn,
        s.mean_funding_r,
        predicted,
        s.ci.unwrap().lo,
        s.ci.unwrap().hi
    );
    assert!(s.n > 5000, "{} trades", s.n);
    assert!(
        s.gross_expectancy_r.abs() < 4.0 * se,
        "gross {} vs SE {se}",
        s.gross_expectancy_r
    );
    assert!(close(s.expectancy_r, s.gross_expectancy_r - s.mean_cost_r));
    assert!(
        (txn - predicted).abs() < 0.03 * predicted,
        "{txn} vs {predicted}"
    );
    assert!(
        (s.expectancy_r + s.mean_cost_r).abs() < 4.0 * se,
        "expectancy {} is not minus the costs {} within noise",
        s.expectancy_r,
        s.mean_cost_r
    );
    let ci = s.ci.unwrap();
    assert!(ci.lo < -s.mean_cost_r && -s.mean_cost_r < ci.hi, "{ci:?}");
}

// ---------------------------------------------------------------- 3. lookahead canary

/// Deliberately cheating: decides at candle k from the close of candle k + 1.
struct Cheat;

impl SignalSource for Cheat {
    fn name(&self) -> String {
        "cheat(next close)".into()
    }

    fn events(&self, bars: &[Candle], _tf: i64) -> Vec<Event> {
        let mut v = Vec::new();
        for k in 0..bars.len().saturating_sub(1) {
            let dir = if bars[k + 1].close > bars[k].close {
                1
            } else {
                -1
            };
            let mut e = Event::new("cheat", "peek", dir, k as u32).with_price(bars[k].close);
            e.known_at = bars[k].open_time;
            e.formed_at = e.known_at;
            v.push(e);
        }
        v
    }
}

fn cheat_rule() -> TradeRule {
    TradeRule {
        name: "cheat".into(),
        select: Select::det("cheat"),
        entry: Entry::NextOpen,
        stop: StopRule::Atr { mult: 3.0 },
        target: Target::None,
        time_stop_bars: Some(1),
        trail: None,
        filters: vec![],
        overlap: Overlap::OnePerSetup,
        min_stop_atr: 0.0,
    }
}

#[test]
fn check3_lookahead_canary() {
    let m1s = random_walk_1m(15 * 6000, 30_000.0, 0.0006, 21);
    let bs = aggregate(&m1s, TF);
    let m = Market::prepare(bs, TF, &Cheat).unwrap();
    // Guard 1: the engine's prefix audit refuses the source.
    let cuts = audit_cuts(m.bars.len(), 4, 9);
    let err = audit_prefix(&m, &Cheat, &cuts).unwrap_err();
    assert!(matches!(err, Error::Lookahead(_)), "{err}");
    println!("check 3: audit -> {err}");
    // Guard 2: forced through anyway, the result is flagged as impossible.
    for costs in [CostModel::ZERO, CostModel::BINANCE_UM] {
        let inp = inputs(&m, &m1s, &[], costs);
        let run = simulate(&inp, &cheat_rule()).unwrap();
        let s = summarize(&run, TF, BootCfg::default());
        println!(
            "check 3: cheat at cost x{} ({} bps fee): {} trades, win rate {:.1} %, expectancy \
             {:.3} R, t-stat {:.1}, flags {:?}",
            costs.mult,
            costs.taker_fee_bps,
            s.n,
            100.0 * s.win_rate,
            s.expectancy_r,
            s.t_stat,
            s.flags
        );
        assert!(
            s.flags.iter().any(|f| f.starts_with("SUSPECT_LOOKAHEAD")),
            "{:?}",
            s.flags
        );
        if costs.taker_fee_bps == 0.0 {
            assert!(s.win_rate > 0.95);
        }
    }
    // Controls: honest sources pass the same audit and are not flagged.
    let src = RandomSource { seed: 3, prob: 0.3 };
    let mr = Market::prepare(m.bars.clone(), TF, &src).unwrap();
    audit_prefix(&mr, &src, &cuts).unwrap();
    let mut r = cheat_rule();
    r.select = Select::det("random");
    let s = summarize(
        &simulate(&inputs(&mr, &m1s, &[], CostModel::ZERO), &r).unwrap(),
        TF,
        BootCfg::default(),
    );
    assert!(
        !s.flags.iter().any(|f| f.starts_with("SUSPECT")),
        "{:?}",
        s.flags
    );
    let det = DetectorSource::default_set();
    let md = Market::prepare(m.bars[..3000].to_vec(), TF, &det).unwrap();
    audit_prefix(&md, &det, &audit_cuts(3000, 3, 2)).unwrap();
}

// ---------------------------------------------------------------- 4. cost monotonicity

fn detector_market(n_bars: usize, seed: u64) -> (Vec<Candle>, Market) {
    let m1s = regime_walk_1m(15 * n_bars, 30_000.0, 0.0007, 0.000_04, seed);
    let bs = aggregate(&m1s, TF);
    let m = Market::prepare(bs, TF, &DetectorSource::default_set()).unwrap();
    (m1s, m)
}

fn rule_set() -> Vec<TradeRule> {
    let mut v = Vec::new();
    for det in [
        "fakeout",
        "pullback",
        "sweep",
        "cdl_bullish_engulfing",
        "bos",
    ] {
        v.push(TradeRule::basic(
            &format!("{det}-2r"),
            Select::det(det),
            Target::R(2.0),
        ));
        v.push(TradeRule::basic(
            &format!("{det}-lvl"),
            Select::det(det),
            Target::NextLevel { min_r: 1.0 },
        ));
        let mut l = TradeRule::basic(&format!("{det}-lim"), Select::det(det), Target::R(1.5));
        l.entry = Entry::Limit {
            at: LimitAt::ZoneNear,
            timeout_bars: 10,
        };
        v.push(l);
        let mut t = TradeRule::basic(&format!("{det}-trail"), Select::det(det), Target::R(3.0));
        t.trail = Some(Trail {
            activate_r: 1.0,
            atr_mult: 2.0,
        });
        t.time_stop_bars = Some(48);
        t.overlap = Overlap::Unlimited;
        v.push(t);
    }
    v
}

#[test]
fn check4_cost_monotonicity() {
    let (m1s, m) = detector_market(8000, 4);
    let funding = funding_8h(m.bars[0].open_time, m.close_at(m.bars.len() - 1), 0.0001);
    let mults = [0.0, 0.5, 1.0, 1.5, 2.0, 4.0];
    let mut total = 0;
    for r in rule_set() {
        let runs: Vec<_> = mults
            .iter()
            .map(|&x| {
                simulate(
                    &inputs(&m, &m1s, &funding, CostModel::BINANCE_UM.with_mult(x)),
                    &r,
                )
                .unwrap()
            })
            .collect();
        let exp: Vec<f64> = runs
            .iter()
            .map(|x| summarize(x, TF, BootCfg::default()).expectancy_r)
            .collect();
        total += runs[0].trades.len();
        for w in runs.windows(2) {
            assert_eq!(
                w[0].trades.len(),
                w[1].trades.len(),
                "{}: trade set changed",
                r.name
            );
            for (a, b) in w[0].trades.iter().zip(&w[1].trades) {
                assert_eq!((a.entry_time, a.exit_time), (b.entry_time, b.exit_time));
                assert_eq!(a.exit_raw.to_bits(), b.exit_raw.to_bits());
                assert!(
                    b.net_r <= a.net_r + 1e-12,
                    "{}: {} -> {}",
                    r.name,
                    a.net_r,
                    b.net_r
                );
            }
        }
        for w in exp.windows(2) {
            if w[0].is_finite() {
                assert!(w[1] <= w[0] + 1e-12, "{}: expectancy {:?}", r.name, exp);
            }
        }
        if r.name == "fakeout-2r" {
            println!(
                "check 4: {} expectancy at cost x{mults:?}: {exp:.4?}",
                r.name
            );
        }
    }
    // Each component on its own.
    let r = &rule_set()[0];
    let base = CostModel::BINANCE_UM;
    let bumps = [
        CostModel {
            taker_fee_bps: 10.0,
            ..base
        },
        CostModel {
            spread_bps: 5.0,
            ..base
        },
        CostModel {
            slip_bps: 4.0,
            ..base
        },
        CostModel {
            slip_atr: 0.2,
            ..base
        },
    ];
    let e0 = summarize(
        &simulate(&inputs(&m, &m1s, &funding, base), r).unwrap(),
        TF,
        BootCfg::default(),
    )
    .expectancy_r;
    for c in bumps {
        let e = summarize(
            &simulate(&inputs(&m, &m1s, &funding, c), r).unwrap(),
            TF,
            BootCfg::default(),
        )
        .expectancy_r;
        assert!(e < e0, "{c:?}: {e} >= {e0}");
    }
    println!(
        "check 4: {} rules x {} cost levels, {total} trades per level, monotone",
        rule_set().len(),
        mults.len()
    );
    assert!(total > 500, "only {total} trades");
}

// ---------------------------------------------------------------- 5. determinism

fn full_pipeline(seed: u64) -> (Vec<String>, String, String) {
    let (m1s, m) = detector_market(5000, 7);
    let src = DetectorSource::default_set();
    audit_prefix(&m, &src, &audit_cuts(m.bars.len(), 2, seed)).unwrap();
    let funding = funding_8h(m.bars[0].open_time, m.close_at(m.bars.len() - 1), 0.0001);
    let mfp = market_fingerprint(&m);
    let dfp = data_fingerprint(&m1s, &funding);
    let boot = BootCfg {
        seed,
        ..BootCfg::default()
    };
    let mut hashes = Vec::new();
    let mut csv = String::new();
    let mut json = String::new();
    for r in rule_set() {
        let o = backtest(
            &inputs(&m, &m1s, &funding, CostModel::BINANCE_UM),
            &r,
            boot,
            &mfp,
            &dfp,
        )
        .unwrap();
        hashes.push(format!("{} {}", o.inputs_hash, o.results_hash));
        csv.push_str(&trades_csv(&o.run.trades));
        json.push_str(
            &summary_json(&o.run, &o.summary, boot, &o.inputs_hash, &o.results_hash).to_string(),
        );
    }
    (hashes, csv, json)
}

#[test]
fn check5_determinism() {
    let (h1, c1, j1) = full_pipeline(1);
    let (h2, c2, j2) = full_pipeline(1);
    assert_eq!(h1, h2);
    assert_eq!(c1, c2);
    assert_eq!(j1, j2);
    // The hash is sensitive: another bootstrap seed changes the CI and so the results hash.
    let (h3, ..) = full_pipeline(2);
    assert_ne!(h1, h3);
    println!(
        "check 5: {} rule runs, identical hashes twice; first {}",
        h1.len(),
        h1[0]
    );
}

// ---------------------------------------------------------------- known_at entry timing

#[test]
fn no_entry_before_close_of_known_candle_property() {
    let (m1s, m) = detector_market(6000, 13);
    let mut rules = rule_set();
    let mut any = TradeRule::basic(
        "any",
        Select {
            det: None,
            kinds: vec![],
            dir: DirSel::Both,
        },
        Target::R(1.0),
    );
    any.overlap = Overlap::Unlimited;
    rules.push(any);
    let mut n = 0;
    for r in &rules {
        let run = simulate(&inputs(&m, &m1s, &[], CostModel::BINANCE_UM), r).unwrap();
        for t in &run.trades {
            let k = t.signal_idx as usize;
            let close_k = m.bars[k].open_time + TF;
            assert_eq!(t.known_at, m.bars[k].open_time);
            assert_eq!(t.decision_at, close_k);
            assert!(
                t.entry_time >= close_k,
                "{}: entry {} before close {close_k}",
                r.name,
                t.entry_time
            );
            assert!(t.entry_idx as usize > k);
            assert!(t.exit_time >= t.entry_time);
            if r.entry == Entry::NextOpen {
                assert_eq!(t.entry_time, close_k);
                assert_eq!(t.entry_raw.to_bits(), m.bars[k + 1].open.to_bits());
            }
            n += 1;
        }
    }
    println!("known_at: {n} trades checked, none entered before the close of its known_at candle");
    assert!(n > 1000);
}

#[test]
fn no_entry_on_the_known_candle_even_when_it_would_win() {
    // Candle 20 is the signal candle: it dips to 95 and spikes to 110, so entering on it (at its
    // open or on its dip) would hit any target. From candle 21 on, price falls to the stop.
    let mut spec = flat(20, 100.0);
    spec.push((100.0, 110.0, 95.0, 100.0)); // 20: known candle
    spec.push((100.0, 100.5, 99.5, 99.8)); // 21
    spec.push((99.8, 99.9, 98.5, 98.6)); // 22: trades through 99 (limit fill)
    spec.push((98.6, 98.7, 90.0, 90.5)); // 23: stop
    spec.extend(flat(3, 90.0));
    let bs = bars(&spec);
    let e = ev(&bs, 20, 1, 97.0, 99.0);
    let m = market(bs.clone(), vec![e]);
    let ms = minutes(&bs, &[]);
    // Next open: enters at the open of 21, never at 20.
    let run = simulate(
        &inputs(&m, &ms, &[], CostModel::ZERO),
        &rule(0.5, Target::R(1.0)),
    )
    .unwrap();
    let t = run.trades[0];
    assert_eq!(t.entry_idx, 21);
    assert_eq!(t.entry_time, bs[21].open_time);
    assert!(close(t.entry_raw, 100.0));
    assert_eq!(t.reason, ExitReason::Stop);
    assert!(t.net_r < 0.0);
    // Limit at the zone top 99: candle 20 trades through 99, but the order exists only from 21.
    let mut r = rule(0.5, Target::R(1.0));
    r.entry = Entry::Limit {
        at: LimitAt::ZoneNear,
        timeout_bars: 5,
    };
    let run = simulate(&inputs(&m, &ms, &[], CostModel::ZERO), &r).unwrap();
    let t = run.trades[0];
    assert_eq!(t.entry_idx, 22, "{t:?}");
    assert!(t.entry_time >= bs[21].open_time && close(t.entry_raw, 99.0));
    assert_eq!(t.reason, ExitReason::Stop);
    // A signal on the last candle cannot be traded.
    let n = bs.len();
    let m2 = market(bs.clone(), vec![ev(&bs, n - 1, 1, 89.0, 90.0)]);
    let run = simulate(
        &inputs(&m2, &ms, &[], CostModel::ZERO),
        &rule(0.5, Target::R(1.0)),
    )
    .unwrap();
    assert!(run.trades.is_empty() && run.skips.no_next_candle == 1);
}

#[test]
fn inconsistent_known_at_is_refused() {
    let bs = bars(&flat(30, 100.0));
    let mut e = ev(&bs, 20, 1, 99.0, 100.0);
    e.known_at = bs[21].open_time; // claims to be known one candle later than its index says
    let err =
        Market::prepare(bs.clone(), TF, &cs_backtest::market::FixedSource(vec![e])).unwrap_err();
    assert!(matches!(err, Error::Lookahead(_)), "{err}");
    let mut e = ev(&bs, 20, 1, 99.0, 100.0);
    e.formed_idx = 21; // formed after it is known
    let err = Market::prepare(bs, TF, &cs_backtest::market::FixedSource(vec![e])).unwrap_err();
    assert!(matches!(err, Error::Lookahead(_)), "{err}");
}

// ---------------------------------------------------------------- filters use only known data

fn all_filters() -> Vec<Filter> {
    let mut v = Vec::new();
    for slot in 0..2 {
        for align in [Align::With, Align::Against, Align::Neutral] {
            v.push(Filter::HtfStack { slot, align });
            v.push(Filter::HtfStructure { slot, align });
        }
    }
    for align in [Align::With, Align::Against, Align::Neutral] {
        v.push(Filter::Regime(align));
    }
    v.push(Filter::Healthy(true));
    v.push(Filter::Healthy(false));
    v.push(Filter::VolPct { min: 0.2, max: 0.8 });
    v.push(Filter::Sessions(0b0_0110));
    v.push(Filter::Hours(0x00_ff00));
    v
}

fn pullback_and_bos(_base: i64) -> Vec<Box<dyn Detector>> {
    vec![
        Box::new(Pullback::new(PullbackParams::default())),
        Box::new(StructureBreak::new(BreakParams::default())),
    ]
}

#[test]
fn filters_never_use_data_after_known_at() {
    let m1s = regime_walk_1m(15 * 16_000, 30_000.0, 0.0007, 0.000_04, 17);
    let bs = aggregate(&m1s, TF);
    let src = DetectorSource {
        make: pullback_and_bos,
    };
    let full = Market::prepare(bs.clone(), TF, &src).unwrap();
    let filters = all_filters();
    // HTF state at a candle comes from HTF candles completed by that candle's close.
    for (k, f) in full.features.iter().enumerate() {
        for h in &f.htf {
            assert!(
                h.n == 0 || h.known_at <= full.bars[k].open_time,
                "candle {k}"
            );
        }
    }
    let healthy = full
        .events
        .iter()
        .filter(|e| e.det == "pullback" && e.tags & 16 != 0)
        .count();
    let passes = |m: &Market, e: &Event| -> Vec<bool> {
        let f = &m.features[e.known_idx as usize];
        filters.iter().map(|x| x.passes(e, f)).collect()
    };
    let mut compared = 0;
    let mut pass_counts = vec![0usize; filters.len()];
    for cut in [4000, 9000, 13_000, 15_500] {
        // (a) data truncated right after the cut
        let trunc = Market::prepare(bs[..cut].to_vec(), TF, &src).unwrap();
        // (b) a completely different future after the cut: a crash with 3x volatility
        let mut alt = bs[..cut].to_vec();
        let tail = random_walk_1m(15 * 600, bs[cut - 1].close, 0.002, 99);
        let mut tail = aggregate(&tail, TF);
        for (q, c) in tail.iter_mut().enumerate() {
            let shift = bs[cut - 1].open_time + TF * (q as i64 + 1);
            c.open_time = shift;
            let f = 1.0 - 0.0005 * q as f64;
            c.open *= f;
            c.high *= f;
            c.low *= f;
            c.close *= f;
        }
        alt.extend(tail);
        let altm = Market::prepare(alt, TF, &src).unwrap();
        for e in full.events.iter().filter(|e| (e.known_idx as usize) < cut) {
            let want = passes(&full, e);
            let et = trunc
                .events
                .iter()
                .find(|x| x.same(e))
                .expect("event missing after truncation");
            let ea = altm
                .events
                .iter()
                .find(|x| x.same(e))
                .expect("event missing with another future");
            assert_eq!(
                want,
                passes(&trunc, et),
                "cut {cut}, event at {}",
                e.known_idx
            );
            assert_eq!(
                want,
                passes(&altm, ea),
                "cut {cut}, event at {}",
                e.known_idx
            );
            for (c, p) in pass_counts.iter_mut().zip(&want) {
                *c += usize::from(*p);
            }
            compared += 1;
        }
    }
    println!(
        "filters: {compared} event x 4-cut comparisons, {healthy} healthy pullbacks, passes per filter {pass_counts:?}"
    );
    assert!(compared > 500 && healthy > 0);
    // Every filter is exercised both ways.
    for (f, c) in filters.iter().zip(&pass_counts) {
        assert!(*c > 0 && *c < compared, "{f:?} passed {c} of {compared}");
    }
}
