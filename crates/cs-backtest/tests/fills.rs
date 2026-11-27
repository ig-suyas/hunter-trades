//! Fill logic: intrabar resolution with 1-minute candles, gaps, limit orders, trailing and time
//! stops, funding boundaries, overlap, leverage cap and cost decomposition.
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
use cs_backtest::market::{DetectorSource, Market};
use cs_backtest::rule::{
    DirSel, Entry, LimitAt, Overlap, Select, StopRule, Target, TradeRule, Trail,
};
use cs_backtest::sim::{ExitReason, Inputs, Sizing, simulate};
use cs_backtest::testkit::{T0, aggregate, funding_8h, m1, regime_walk_1m};
use cs_core::Candle;

/// Flat warm-up 0..=19 (ATR 2), long signal at 19 with structural low 99 -> stop 98, entry at
/// the open of 20 (100), risk 2, then the given candles from 20 on.
fn long_case(after: &[(f64, f64, f64, f64)]) -> Vec<Candle> {
    let mut s = flat(20, 100.0);
    s.extend_from_slice(after);
    s.extend(flat(3, after.last().map_or(100.0, |x| x.3)));
    bars(&s)
}

fn run1(bs: &[Candle], ms: &[Candle], r: &TradeRule) -> cs_backtest::Run {
    let m = market(bs.to_vec(), vec![ev(bs, 19, 1, 99.0, 100.0)]);
    simulate(&inputs(&m, ms, &[], CostModel::ZERO), r).unwrap()
}

#[test]
fn both_levels_in_one_candle_resolved_by_minutes() {
    // Candle 20 touches the stop (98) and the target (102).
    let bs = long_case(&[(100.0, 102.5, 97.5, 100.0)]);
    let r = rule(0.5, Target::R(1.0));
    // 1m path high first -> target
    let run = run1(&bs, &minutes(&bs, &[(20, split_hl(&bs[20]))]), &r);
    let t = run.trades[0];
    assert_eq!(t.reason, ExitReason::Target);
    assert!(t.ambiguous && t.resolved_1m && !t.stop_first);
    assert!(close(t.exit_raw, 102.0) && close(t.gross_r, 1.0));
    assert_eq!(run.fills.ambiguous_resolved_1m, 1);
    // 1m path low first -> stop
    let run = run1(&bs, &minutes(&bs, &[(20, split_lh(&bs[20]))]), &r);
    let t = run.trades[0];
    assert_eq!(t.reason, ExitReason::Stop);
    assert!(close(t.exit_raw, 98.0) && t.resolved_1m && !t.stop_first);
    // both inside one minute -> stop assumed first
    let b = bs[20];
    let mut one = vec![m1(b.open_time, 100.0, 102.5, 97.5, 100.0)];
    for q in 1..15 {
        one.push(m1(b.open_time + q * MIN, 100.0, 100.0, 100.0, 100.0));
    }
    let run = run1(&bs, &minutes(&bs, &[(20, one)]), &r);
    let t = run.trades[0];
    assert_eq!(t.reason, ExitReason::Stop);
    assert!(t.stop_first && t.resolved_1m);
    assert_eq!(run.fills.stop_first_same_minute, 1);
    // no 1-minute data -> stop assumed first
    let run = run1(&bs, &[], &r);
    let t = run.trades[0];
    assert_eq!(t.reason, ExitReason::Stop);
    assert!(t.stop_first && !t.resolved_1m);
    assert_eq!(run.fills.stop_first_no_1m, 1);
    // 1-minute data that do not match the candle (a minute missing) are not used -> stop
    let mut bad = minutes(&bs, &[(20, split_hl(&bs[20]))]);
    bad.retain(|c| c.open_time != bs[20].open_time + 5 * MIN);
    let run = run1(&bs, &bad, &r);
    assert_eq!(run.trades[0].reason, ExitReason::Stop);
    assert!(run.fills.m1_unusable >= 1);
}

#[test]
fn gaps_fill_pessimistically() {
    let r = rule(0.5, Target::R(1.0));
    // Stop gapped: candle 21 opens at 97 below the 98 stop -> exit at 97, not 98.
    let bs = long_case(&[(100.0, 101.0, 99.0, 99.2), (97.0, 97.5, 96.5, 97.0)]);
    let t = run1(&bs, &minutes(&bs, &[]), &r).trades[0];
    assert_eq!(t.reason, ExitReason::Stop);
    assert!(close(t.exit_raw, 97.0) && close(t.gross_r, -1.5));
    // Target gapped: candle 21 opens at 103 above the 102 target -> exit at 102, not 103.
    let bs = long_case(&[(100.0, 101.0, 99.5, 101.0), (103.0, 103.5, 102.5, 103.0)]);
    let t = run1(&bs, &minutes(&bs, &[]), &r).trades[0];
    assert_eq!(t.reason, ExitReason::Target);
    assert!(close(t.exit_raw, 102.0) && close(t.gross_r, 1.0));
    // A touch of the target is not enough: high == target does not fill.
    let bs = long_case(&[(100.0, 102.0, 99.5, 101.0)]);
    let run = run1(&bs, &minutes(&bs, &[]), &r);
    assert_ne!(run.trades[0].reason, ExitReason::Target);
    // A touch of the stop is enough: low == stop fills.
    let bs = long_case(&[(100.0, 100.5, 98.0, 99.0)]);
    let t = run1(&bs, &minutes(&bs, &[]), &r).trades[0];
    assert_eq!(t.reason, ExitReason::Stop);
}

fn limit_rule(timeout: u32) -> TradeRule {
    let mut r = rule(0.5, Target::R(1.0));
    r.entry = Entry::Limit {
        at: LimitAt::ZoneNear,
        timeout_bars: timeout,
    };
    r
}

/// Flat warm-up, long signal at 19 with zone 98.5..99: buy limit 99, stop 98.5 - 1 = 97.5,
/// risk 1.5, target 100.5.
fn limit_market(after: &[(f64, f64, f64, f64)]) -> (Vec<Candle>, Market) {
    let mut s = flat(20, 100.0);
    s.extend_from_slice(after);
    s.extend(flat(3, after.last().map_or(100.0, |x| x.3)));
    let bs = bars(&s);
    let m = market(bs.clone(), vec![ev(&bs, 19, 1, 98.5, 99.0)]);
    (bs, m)
}

#[test]
fn limit_needs_trade_through_and_ignores_target_in_fill_minute() {
    // 20: low exactly 99 -> no fill. 21: trades to 98.9 -> fill at 99 (not 98.9).
    let (bs, m) = limit_market(&[
        (100.0, 100.4, 99.0, 100.0),
        (100.0, 100.8, 98.9, 100.6),
        (100.6, 101.6, 100.4, 101.0),
    ]);
    // The fill minute itself spikes above the target: not counted (order unknown in the minute).
    let b = bs[21];
    let mut ms21 = vec![m1(b.open_time, 100.0, 100.8, 98.9, 99.5)];
    let mut last = 99.5;
    for q in 1..15 {
        let c = if q == 14 {
            100.6
        } else {
            99.5 + 0.05 * q as f64
        };
        ms21.push(m1(b.open_time + q * MIN, last, last.max(c), last.min(c), c));
        last = c;
    }
    let ms = minutes(&bs, &[(21, ms21)]);
    let run = simulate(&inputs(&m, &ms, &[], CostModel::ZERO), &limit_rule(5)).unwrap();
    let t = run.trades[0];
    assert_eq!(t.entry_idx, 21);
    assert_eq!(t.entry_time, b.open_time);
    assert!(close(t.entry_raw, 99.0) && !t.entry_market);
    assert!(close(t.risk, 1.5) && close(t.target, 100.5));
    // 100.6 at the end of candle 21 is above the target, so it exits in minute 14 of candle 21
    assert_eq!(t.reason, ExitReason::Target);
    assert_eq!(t.exit_time, b.open_time + 14 * MIN);
    assert_eq!(run.fills.limit_fills_1m, 1);

    // Without 1-minute data the target is ignored for the whole fill candle: exit in 22.
    let run = simulate(&inputs(&m, &[], &[], CostModel::ZERO), &limit_rule(5)).unwrap();
    let t = run.trades[0];
    assert_eq!(t.entry_idx, 21);
    assert_eq!(t.reason, ExitReason::Target);
    assert_eq!(t.exit_idx, 22);
    assert!(close(t.exit_raw, 100.5));

    // A stop touch in the fill candle (no 1m) counts: filled and stopped in the same candle.
    let (_, m2) = limit_market(&[(100.0, 100.4, 99.2, 100.0), (100.0, 100.8, 97.4, 98.0)]);
    let t = simulate(&inputs(&m2, &[], &[], CostModel::ZERO), &limit_rule(5))
        .unwrap()
        .trades[0];
    assert_eq!(t.reason, ExitReason::Stop);
    assert_eq!(t.entry_idx, t.exit_idx);
}

#[test]
fn limit_expiry_blocks_overlap_and_marketable_limits_fill_at_open() {
    // Never trades through 99 within 2 candles -> expired; a second signal at 20 (decided while
    // the order is pending) is blocked by the one-per-setup policy.
    let mut s = flat(20, 100.0);
    s.extend(vec![(100.0, 101.0, 99.5, 100.0); 6]);
    let bs = bars(&s);
    let events = vec![ev(&bs, 19, 1, 98.5, 99.0), ev(&bs, 20, 1, 98.5, 99.0)];
    let m = market(bs.clone(), events);
    let run = simulate(&inputs(&m, &[], &[], CostModel::ZERO), &limit_rule(2)).unwrap();
    assert!(run.trades.is_empty());
    assert_eq!(run.skips.limit_expired, 1);
    assert_eq!(run.skips.overlap, 1);
    // Zone top 101 above the next open (100): marketable, fills at the open as a market order.
    let m = market(bs.clone(), vec![ev(&bs, 19, 1, 98.0, 101.0)]);
    let run = simulate(&inputs(&m, &[], &[], CostModel::BINANCE_UM), &limit_rule(2)).unwrap();
    let t = run.trades[0];
    assert!(t.entry_market && close(t.entry_raw, 100.0) && t.entry_fill > 100.0);
    assert_eq!(run.fills.marketable_limits, 1);
}

#[test]
fn trailing_stop_applies_from_the_next_candle() {
    // Entry 100, stop 98. Candle 21 closes at 102.5 (+1.25 R): trail activates at its close with
    // stop = 102.5 - 1 x ATR(21). Candle 21's own low (99) is below that level but must not
    // trigger it; candle 22 does.
    let bs = long_case(&[
        (100.0, 101.0, 99.5, 100.5),
        (100.5, 102.6, 99.0, 102.5),
        (102.5, 102.7, 100.0, 100.2),
    ]);
    let mut r = rule(0.5, Target::None);
    r.trail = Some(Trail {
        activate_r: 1.0,
        atr_mult: 1.0,
    });
    let m = market(bs.clone(), vec![ev(&bs, 19, 1, 99.0, 100.0)]);
    let run = simulate(&inputs(&m, &[], &[], CostModel::ZERO), &r).unwrap();
    let t = run.trades[0];
    let trail_level = 102.5 - m.features[21].atr;
    // above candle 21's low (99) and above the entry: it would have triggered inside 21
    assert!(trail_level > 100.0);
    assert_eq!(t.exit_idx, 22, "{t:?}");
    assert_eq!(t.reason, ExitReason::Trail);
    assert!(close(t.exit_raw, trail_level));
}

#[test]
fn time_stop_exits_at_the_open() {
    let bs = long_case(&[(100.0, 101.0, 99.5, 100.5); 5]);
    let mut r = rule(0.5, Target::R(3.0));
    r.time_stop_bars = Some(3);
    let t = run1(&bs, &[], &r).trades[0];
    assert_eq!(t.reason, ExitReason::Time);
    assert_eq!(t.exit_idx, 23);
    assert_eq!(t.exit_time, bs[23].open_time);
    assert!(close(t.exit_raw, bs[23].open));
}

#[test]
fn funding_window_is_entry_exclusive_exit_inclusive() {
    // Entry at the open of 20 (05:00), time-stop exit at the open of 24 (06:00).
    let bs = long_case(&[(100.0, 101.0, 99.5, 100.0); 6]);
    let mut r = rule(0.5, Target::None);
    r.time_stop_bars = Some(4);
    let m = market(bs.clone(), vec![ev(&bs, 19, 1, 99.0, 100.0)]);
    let h = 3_600_000;
    let f = vec![
        Funding {
            time: T0 + 5 * h,
            rate: 0.01,
        }, // at entry: not charged
        Funding {
            time: T0 + 5 * h + 30 * MIN + 3,
            rate: 0.001,
        }, // a few ms past the minute
        Funding {
            time: T0 + 6 * h,
            rate: 0.002,
        }, // at exit: charged
        Funding {
            time: T0 + 6 * h + 15 * MIN,
            rate: 0.01,
        }, // after exit
    ];
    let c = CostModel {
        funding: true,
        ..CostModel::ZERO
    };
    let t = simulate(&inputs(&m, &[], &f, c), &r).unwrap().trades[0];
    assert_eq!(t.funding_events, 2);
    // long pays: price at 05:30 = open of candle 22 (100), at 06:00 = open of candle 24 (100)
    // (100 x 0.001 + 100 x 0.002) / risk 2
    assert!(close(t.funding_r, 0.15));
    // switched off -> nothing
    let t = simulate(&inputs(&m, &[], &f, CostModel::ZERO), &r)
        .unwrap()
        .trades[0];
    assert_eq!(t.funding_events, 0);
}

#[test]
fn overlap_policies() {
    let bs = long_case(&[(100.0, 101.0, 99.5, 100.0); 8]);
    let mut s2 = ev(&bs, 21, -1, 99.0, 101.0);
    s2.kind = "short";
    let events = vec![ev(&bs, 19, 1, 99.0, 100.0), ev(&bs, 21, 1, 99.0, 100.0), s2];
    let m = market(bs.clone(), events);
    let mut r = rule(0.5, Target::R(5.0));
    let n = |r: &TradeRule| simulate(&inputs(&m, &[], &[], CostModel::ZERO), r).unwrap();
    let a = n(&r);
    assert_eq!((a.trades.len(), a.skips.overlap), (1, 2));
    r.overlap = Overlap::OnePerDirection;
    let b = n(&r);
    assert_eq!((b.trades.len(), b.skips.overlap), (2, 1));
    r.overlap = Overlap::Unlimited;
    assert_eq!(n(&r).trades.len(), 3);
    r.select.dir = DirSel::Short;
    assert_eq!(n(&r).trades.len(), 1);
}

#[test]
fn leverage_cap_reduces_the_risk_share() {
    let bs = long_case(&[(100.0, 101.0, 99.5, 100.0); 4]);
    let m = market(bs.clone(), vec![ev(&bs, 19, 1, 99.0, 100.0)]);
    let mut r = rule(0.5, Target::R(1.0));
    r.stop = StopRule::Atr { mult: 0.1 }; // risk 0.2 on a price of 100
    let inp = Inputs {
        sizing: Sizing {
            equity0: 10_000.0,
            risk_frac: 0.01,
            max_leverage: 2.0,
        },
        ..inputs(&m, &[], &[], CostModel::ZERO)
    };
    let t = simulate(&inp, &r).unwrap().trades[0];
    // 1 % risk would need 5x notional; capped at 2x -> 2 x 0.2 / 100 = 0.4 % risk
    assert!(close(t.risk_frac, 0.004), "{}", t.risk_frac);
    assert!(close(t.ret, t.net_r * 0.004));
}

#[test]
fn costs_are_charged_once_per_fill_and_add_up() {
    let m1s = regime_walk_1m(15 * 4000, 30_000.0, 0.0007, 0.000_04, 31);
    let bs = aggregate(&m1s, TF);
    let m = Market::prepare(bs, TF, &DetectorSource::default_set()).unwrap();
    let funding = funding_8h(m.bars[0].open_time, m.close_at(m.bars.len() - 1), 0.0003);
    let c = CostModel::BINANCE_UM;
    let mut r = TradeRule::basic(
        "all",
        Select {
            det: None,
            kinds: vec![],
            dir: DirSel::Both,
        },
        Target::R(1.5),
    );
    r.overlap = Overlap::Unlimited;
    r.time_stop_bars = Some(40);
    let run = simulate(&inputs(&m, &m1s, &funding, c), &r).unwrap();
    assert!(run.trades.len() > 1000);
    for t in &run.trades {
        let d = f64::from(t.dir);
        // fills include exactly one half-spread and one slippage per leg
        let via_fills = d * (t.exit_fill - t.entry_fill) / t.risk;
        assert!((via_fills - (t.gross_r - t.spread_r - t.slip_r)).abs() < 1e-9);
        assert!(close(
            t.spread_r * t.risk,
            0.5 * c.spread_bps / 1e4 * (t.entry_raw + t.exit_raw)
        ));
        assert!(close(
            t.fee_r * t.risk,
            c.taker_fee_bps / 1e4 * (t.entry_fill + t.exit_fill)
        ));
        assert!(t.spread_r >= 0.0 && t.slip_r >= 0.0 && t.fee_r >= 0.0);
        assert!(close(
            t.net_r,
            t.gross_r - t.spread_r - t.slip_r - t.fee_r - t.funding_r
        ));
        assert!(t.funding_events == 0 || t.funding_r.abs() > 0.0);
    }
}
