//! The 10 pre-registered **exploratory** rules of the Phase 4 GATE report, frozen exactly as
//! `reports/research/finalists.json` lists them (same order, same parameters). They failed the
//! GATE (0 of 59 setups survived); they are used here only because the user asked for a view
//! that shows them next to their past results. Nothing is tuned, added or removed.
//!
//! Each rule is a setup (detector and event kinds), a direction (native or reversed), a time
//! frame, a stop buffer, an exit and a context filter, built into the backtest's own
//! [`TradeRule`] exactly as `cs-cli`'s `research::space::RuleSpec::trade_rule` builds it. A test
//! in `cs-cli` checks every field against the research enumeration by trial id.

use cs_backtest::rule::{
    Align, DirSel, Entry, Filter, Overlap, Select, StopRule, Target, TradeRule, Trail,
};
use cs_core::{Event, Timeframe};

/// Exit of a rule (`reports/search-space.md`, trade rules).
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Exit {
    /// Fixed target of this many R.
    R(f64),
    /// Nearest S/R or supply/demand zone edge at least 1 R away (skip if none).
    Level,
    /// No target: trailing stop 2 ATR behind the best close once 1 R in profit, time stop 96
    /// candles.
    Trail,
}

/// How a "hit" is counted for the hit rate shown next to a rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HitKind {
    /// The rule has a price target: a hit is an exit at the target.
    Target,
    /// The rule has no target (trailing exit): a hit is a trade that ends above 0 R after costs
    /// (the protocol's win definition).
    InProfit,
}

impl HitKind {
    pub const fn key(self) -> &'static str {
        match self {
            HitKind::Target => "target",
            HitKind::InProfit => "in_profit",
        }
    }
}

/// Trailing exit parameters of the search space.
pub const TRAIL_ACTIVATE_R: f64 = 1.0;
pub const TRAIL_ATR: f64 = 2.0;
pub const TRAIL_TIME_STOP: u32 = 96;

/// One frozen rule.
#[derive(Debug, Clone, Copy)]
pub struct RuleDef {
    /// Position in the pre-registered list (1-10). Several rules firing together are shown in
    /// this order, never ordered by past results.
    pub order: u8,
    /// Trial id in `reports/trial-ledger.csv` and `finalists.json`.
    pub trial_id: u32,
    /// Label exactly as in `finalists.json`.
    pub label: &'static str,
    pub setup: &'static str,
    pub det: &'static str,
    /// Event kinds; empty = every kind of the detector.
    pub kinds: &'static [&'static str],
    /// Direction "reversed": every event's direction is flipped before simulation.
    pub reversed: bool,
    pub tf: Timeframe,
    /// Stop buffer beyond the structure, in ATR.
    pub buffer_atr: f64,
    pub exit: Exit,
    /// Context filter id of the search space (F00 none, F07 ADX trend with the trade, F08 ADX
    /// range, F09 session 07-21 UTC).
    pub filter_id: u8,
}

const EMA_CROSS: &[&str] = &[
    "cross_up_fast_mid",
    "cross_down_fast_mid",
    "cross_up_mid_slow",
    "cross_down_mid_slow",
];

/// The frozen list, in the order of `finalists.json` (`exploratory`).
pub const RULES: [RuleDef; 10] = [
    RuleDef {
        order: 1,
        trial_id: 28172,
        label: "fib_touch native 1h b025-trail F09",
        setup: "fib_touch",
        det: "fib",
        kinds: &["fib_zone_touch"],
        reversed: false,
        tf: Timeframe::H1,
        buffer_atr: 0.25,
        exit: Exit::Trail,
        filter_id: 9,
    },
    RuleDef {
        order: 2,
        trial_id: 28490,
        label: "fib_leg native 1h b025-trail F09",
        setup: "fib_leg",
        det: "fib",
        kinds: &["fib_leg_up", "fib_leg_down"],
        reversed: false,
        tf: Timeframe::H1,
        buffer_atr: 0.25,
        exit: Exit::Trail,
        filter_id: 9,
    },
    RuleDef {
        order: 3,
        trial_id: 15499,
        label: "ema_cross native 15m b100-t3 F07",
        setup: "ema_cross",
        det: "ema",
        kinds: EMA_CROSS,
        reversed: false,
        tf: Timeframe::M15,
        buffer_atr: 1.0,
        exit: Exit::R(3.0),
        filter_id: 7,
    },
    RuleDef {
        order: 4,
        trial_id: 28743,
        label: "pullback reversed 1h b100-t1 F08",
        setup: "pullback",
        det: "pullback",
        kinds: &["pullback_long", "pullback_short"],
        reversed: true,
        tf: Timeframe::H1,
        buffer_atr: 1.0,
        exit: Exit::R(1.0),
        filter_id: 8,
    },
    RuleDef {
        order: 5,
        trial_id: 31648,
        label: "cdl_three_outside_up native 1h b025-trail F00",
        setup: "cdl_three_outside_up",
        det: "cdl_three_outside_up",
        kinds: &[],
        reversed: false,
        tf: Timeframe::H1,
        buffer_atr: 0.25,
        exit: Exit::Trail,
        filter_id: 0,
    },
    RuleDef {
        order: 6,
        trial_id: 28127,
        label: "tl_formed reversed 1h b100-trail F07",
        setup: "tl_formed",
        det: "trendline",
        kinds: &["tl_up_formed", "tl_down_formed"],
        reversed: true,
        tf: Timeframe::H1,
        buffer_atr: 1.0,
        exit: Exit::Trail,
        filter_id: 7,
    },
    RuleDef {
        order: 7,
        trial_id: 16605,
        label: "fib_ext native 15m b100-trail F07",
        setup: "fib_ext",
        det: "fib",
        kinds: &["fib_ext_1272", "fib_ext_1618"],
        reversed: false,
        tf: Timeframe::M15,
        buffer_atr: 1.0,
        exit: Exit::Trail,
        filter_id: 7,
    },
    RuleDef {
        order: 8,
        trial_id: 25684,
        label: "swing native 1h b100-lvl F07",
        setup: "swing",
        det: "swing",
        kinds: &["swing_high", "swing_low"],
        reversed: false,
        tf: Timeframe::H1,
        buffer_atr: 1.0,
        exit: Exit::Level,
        filter_id: 7,
    },
    RuleDef {
        order: 9,
        trial_id: 33442,
        label: "cdl_tweezer_top reversed 1h b025-trail F00",
        setup: "cdl_tweezer_top",
        det: "cdl_tweezer_top",
        kinds: &[],
        reversed: true,
        tf: Timeframe::H1,
        buffer_atr: 0.25,
        exit: Exit::Trail,
        filter_id: 0,
    },
    RuleDef {
        order: 10,
        trial_id: 26398,
        label: "choch reversed 1h b100-trail F09",
        setup: "choch",
        det: "choch",
        kinds: &["choch_bull", "choch_bear"],
        reversed: true,
        tf: Timeframe::H1,
        buffer_atr: 1.0,
        exit: Exit::Trail,
        filter_id: 9,
    },
];

/// The rules tested on a time frame, in list order (none on 5m).
pub fn rules_for(tf: Timeframe) -> Vec<&'static RuleDef> {
    RULES.iter().filter(|r| r.tf == tf).collect()
}

/// Whether any frozen rule was tested on this time frame.
pub fn has_rules(tf: Timeframe) -> bool {
    RULES.iter().any(|r| r.tf == tf)
}

/// Rule by list position (1-10).
pub fn by_order(order: u8) -> Option<&'static RuleDef> {
    RULES.iter().find(|r| r.order == order)
}

impl RuleDef {
    /// Directional event of this rule's setup (before any flip).
    pub fn matches(&self, e: &Event) -> bool {
        e.dir != 0 && e.det == self.det && (self.kinds.is_empty() || self.kinds.contains(&e.kind))
    }

    /// The event as traded: direction flipped for a reversed rule.
    pub fn trade_event(&self, e: &Event) -> Event {
        let mut x = *e;
        if self.reversed {
            x.dir = -x.dir;
        }
        x
    }

    /// Engine filters of the rule's filter id (the research's `filters_of`).
    pub fn filters(&self) -> Vec<Filter> {
        match self.filter_id {
            7 => vec![Filter::Regime(Align::With)],
            8 => vec![Filter::Regime(Align::Neutral)],
            9 => vec![Filter::Sessions(0b01110)],
            _ => Vec::new(),
        }
    }

    pub fn time_stop_bars(&self) -> Option<u32> {
        match self.exit {
            Exit::Trail => Some(TRAIL_TIME_STOP),
            _ => None,
        }
    }

    pub fn hit_kind(&self) -> HitKind {
        match self.exit {
            Exit::Trail => HitKind::InProfit,
            _ => HitKind::Target,
        }
    }

    /// The backtest rule, built as the research builds it (events are pre-selected per setup,
    /// so the selection takes every directional event).
    pub fn trade_rule(&self) -> TradeRule {
        let (target, trail, time_stop) = match self.exit {
            Exit::R(x) => (Target::R(x), None, None),
            Exit::Level => (Target::NextLevel { min_r: 1.0 }, None, None),
            Exit::Trail => (
                Target::None,
                Some(Trail {
                    activate_r: TRAIL_ACTIVATE_R,
                    atr_mult: TRAIL_ATR,
                }),
                Some(TRAIL_TIME_STOP),
            ),
        };
        TradeRule {
            name: self.label.to_string(),
            select: Select {
                det: None,
                kinds: Vec::new(),
                dir: DirSel::Both,
            },
            entry: Entry::NextOpen,
            stop: StopRule::Structural {
                buffer_atr: self.buffer_atr,
            },
            target,
            time_stop_bars: time_stop,
            trail,
            filters: self.filters(),
            overlap: Overlap::OnePerSetup,
            min_stop_atr: 0.2,
        }
    }

    /// Plain description of the frozen parameters (Markdown and the panel's details).
    pub fn describe(&self) -> String {
        let dir = if self.reversed {
            "direction reversed"
        } else {
            "detector's direction"
        };
        let exit = match self.exit {
            Exit::R(x) => format!("fixed {x} R exit"),
            Exit::Level => {
                "exit at the nearest S/R or supply/demand zone boundary at least 1 R away"
                    .to_string()
            }
            Exit::Trail => format!(
                "no fixed exit price; trails {TRAIL_ATR} ATR behind the best close from +{TRAIL_ACTIVATE_R} R, \
                 time limit {TRAIL_TIME_STOP} candles"
            ),
        };
        let filter = match self.filter_id {
            7 => "ADX regime trending with the trade",
            8 => "ADX regime range",
            9 => "only 07-21 UTC",
            _ => "no filter",
        };
        format!(
            "{} {}, {dir}, buffer {} ATR beyond the structure, {exit}, {filter}",
            self.setup,
            self.tf.label(),
            self.buffer_atr
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_is_complete_and_ordered() {
        assert_eq!(RULES.len(), 10);
        for (i, r) in RULES.iter().enumerate() {
            assert_eq!(usize::from(r.order), i + 1);
            assert!(r.label.contains(r.tf.label()));
            assert!(r.label.starts_with(r.setup));
        }
        assert!(rules_for(Timeframe::M5).is_empty());
        assert!(!has_rules(Timeframe::M5));
        let m15: Vec<u8> = rules_for(Timeframe::M15).iter().map(|r| r.order).collect();
        assert_eq!(m15, [3, 7]);
        let h1: Vec<u8> = rules_for(Timeframe::H1).iter().map(|r| r.order).collect();
        assert_eq!(h1, [1, 2, 4, 5, 6, 8, 9, 10]);
    }

    /// The labels and order equal `reports/research/finalists.json` (the frozen list).
    #[test]
    fn matches_the_frozen_file() {
        let v: serde_json::Value =
            serde_json::from_str(include_str!("../../../../reports/research/finalists.json"))
                .unwrap();
        let ex = v["exploratory"].as_array().unwrap();
        assert_eq!(ex.len(), 10);
        for (r, x) in RULES.iter().zip(ex) {
            assert_eq!(x["label"].as_str(), Some(r.label));
            assert_eq!(x["trial_id"].as_u64(), Some(u64::from(r.trial_id)));
        }
        assert_eq!(v["finalists"].as_array().unwrap().len(), 0);
    }
}
