//! The pre-registered Phase 4 search space (`reports/search-space.md`): setups, directions, time
//! frames, trade rules, filters and families, the deterministic trial enumeration, and the
//! parameter-plateau neighbours (`reports/protocol.md` section 6).

use cs_backtest::rule::{
    Align, Entry, Filter, Overlap, Select, StopRule, Target, TradeRule, Trail,
};
use cs_core::{Event, Timeframe};

/// A setup: a detector, or a group of its event kinds.
#[derive(Debug, Clone, Copy)]
pub struct Setup {
    pub name: &'static str,
    pub det: &'static str,
    /// Event kinds; empty = every kind of the detector.
    pub kinds: &'static [&'static str],
    /// Only defined below the 1h time frame (the 1h state on a 1h series does not exist).
    pub sub_1h_only: bool,
}

impl Setup {
    /// Directional event of this setup.
    pub fn matches(&self, e: &Event) -> bool {
        e.dir != 0 && e.det == self.det && (self.kinds.is_empty() || self.kinds.contains(&e.kind))
    }
}

const fn s(name: &'static str, det: &'static str, kinds: &'static [&'static str]) -> Setup {
    Setup {
        name,
        det,
        kinds,
        sub_1h_only: false,
    }
}

const fn s1(name: &'static str, det: &'static str, kinds: &'static [&'static str]) -> Setup {
    Setup {
        name,
        det,
        kinds,
        sub_1h_only: true,
    }
}

const fn c(det: &'static str) -> Setup {
    s(det, det, &[])
}

/// The 59 setups, in the order of `search-space.md`.
pub const SETUPS: [Setup; 59] = [
    s("swing", "swing", &["swing_high", "swing_low"]),
    s("structure_label", "structure", &["hh", "hl", "lh", "ll"]),
    s("structure_trend", "structure", &["trend_up", "trend_down"]),
    s("bos", "bos", &["bos_bull", "bos_bear"]),
    s("choch", "choch", &["choch_bull", "choch_bear"]),
    s("sr_break", "sr_zone", &["zone_break_up", "zone_break_down"]),
    s("sr_formed", "sr_zone", &["zone_formed"]),
    s(
        "role_reversal",
        "role_reversal",
        &["flip_support", "flip_resistance"],
    ),
    s("sd_test", "sd_zone", &["demand_test", "supply_test"]),
    s("sd_formed", "sd_zone", &["demand_formed", "supply_formed"]),
    s("sd_broken", "sd_zone", &["demand_broken", "supply_broken"]),
    s("ema_stack", "ema", &["bull_stack", "bear_stack"]),
    s(
        "ema_cross",
        "ema",
        &[
            "cross_up_fast_mid",
            "cross_down_fast_mid",
            "cross_up_mid_slow",
            "cross_down_mid_slow",
        ],
    ),
    s("tl_touch", "trendline", &["tl_up_touch", "tl_down_touch"]),
    s("tl_break", "trendline", &["tl_up_break", "tl_down_break"]),
    s(
        "tl_formed",
        "trendline",
        &["tl_up_formed", "tl_down_formed"],
    ),
    s("fib_touch", "fib", &["fib_zone_touch"]),
    s("fib_ext", "fib", &["fib_ext_1272", "fib_ext_1618"]),
    s("fib_leg", "fib", &["fib_leg_up", "fib_leg_down"]),
    s("pullback", "pullback", &["pullback_long", "pullback_short"]),
    s("breakout", "breakout", &["breakout_up", "breakout_down"]),
    s("retest", "retest", &["retest_long", "retest_short"]),
    s(
        "fakeout",
        "fakeout",
        &["fakeout_down_failed", "fakeout_up_failed"],
    ),
    s("eq_levels", "eq_levels", &["eq_highs", "eq_lows"]),
    s("sweep", "sweep", &["sweep_high", "sweep_low"]),
    s1("h1_stack", "htf_trend", &["h1_bull_stack", "h1_bear_stack"]),
    s1("h1_trend", "htf_trend", &["h1_trend_up", "h1_trend_down"]),
    s("h4_stack", "htf_trend", &["h4_bull_stack", "h4_bear_stack"]),
    s("h4_trend", "htf_trend", &["h4_trend_up", "h4_trend_down"]),
    c("cdl_hammer"),
    c("cdl_inverted_hammer"),
    c("cdl_bullish_engulfing"),
    c("cdl_piercing_line"),
    c("cdl_morning_star"),
    c("cdl_morning_doji_star"),
    c("cdl_three_white_soldiers"),
    c("cdl_tweezer_bottom"),
    c("cdl_bullish_harami"),
    c("cdl_bullish_harami_cross"),
    c("cdl_three_inside_up"),
    c("cdl_three_outside_up"),
    c("cdl_rising_three_methods"),
    c("cdl_bullish_marubozu"),
    c("cdl_dragonfly_doji"),
    c("cdl_hanging_man"),
    c("cdl_shooting_star"),
    c("cdl_bearish_engulfing"),
    c("cdl_dark_cloud_cover"),
    c("cdl_evening_star"),
    c("cdl_evening_doji_star"),
    c("cdl_three_black_crows"),
    c("cdl_tweezer_top"),
    c("cdl_bearish_harami"),
    c("cdl_bearish_harami_cross"),
    c("cdl_three_inside_down"),
    c("cdl_three_outside_down"),
    c("cdl_falling_three_methods"),
    c("cdl_bearish_marubozu"),
    c("cdl_gravestone_doji"),
];

/// Setups of the confluence family.
pub const CONF_SETUPS: [&str; 8] = [
    "sweep",
    "fakeout",
    "retest",
    "role_reversal",
    "pullback",
    "sd_test",
    "fib_touch",
    "choch",
];

pub const TFS: [Timeframe; 3] = [Timeframe::M5, Timeframe::M15, Timeframe::H1];

pub fn tf_index(tf: Timeframe) -> usize {
    TFS.iter().position(|t| *t == tf).unwrap_or(0)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Dir {
    Native,
    Reversed,
}

impl Dir {
    pub const ALL: [Dir; 2] = [Dir::Native, Dir::Reversed];

    pub const fn label(self) -> &'static str {
        match self {
            Dir::Native => "native",
            Dir::Reversed => "reversed",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Exit {
    R(f64),
    Lvl,
    Trail,
}

/// Stop buffer and exit.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RuleSpec {
    pub buffer: f64,
    pub exit: Exit,
}

pub const BUFFERS: [f64; 2] = [0.25, 1.0];
/// Target ladder L (protocol section 6).
pub const LADDER: [f64; 7] = [0.25, 0.5, 0.75, 1.0, 1.5, 2.0, 3.0];
/// Fixed targets of the main grid.
pub const MAIN_TARGETS: [f64; 3] = [1.0, 2.0, 3.0];
/// Extra targets of the frontier family.
pub const FRONTIER_TARGETS: [f64; 4] = [0.25, 0.5, 0.75, 1.5];

#[allow(clippy::cast_possible_truncation)]
fn centi(x: f64) -> i64 {
    (x * 100.0).round() as i64
}

impl RuleSpec {
    /// The 10 main-grid rules.
    pub fn main() -> Vec<RuleSpec> {
        let mut v = Vec::new();
        for b in BUFFERS {
            for t in MAIN_TARGETS {
                v.push(RuleSpec {
                    buffer: b,
                    exit: Exit::R(t),
                });
            }
            v.push(RuleSpec {
                buffer: b,
                exit: Exit::Lvl,
            });
            v.push(RuleSpec {
                buffer: b,
                exit: Exit::Trail,
            });
        }
        v
    }

    pub fn name(&self) -> String {
        let b = format!("b{:03}", centi(self.buffer));
        match self.exit {
            Exit::R(x) => format!("{b}-t{x}"),
            Exit::Lvl => format!("{b}-lvl"),
            Exit::Trail => format!("{b}-trail"),
        }
    }

    #[cfg(test)]
    pub fn parse(s: &str) -> Option<RuleSpec> {
        let (b, e) = s.split_once('-')?;
        let buffer = b.strip_prefix('b')?.parse::<f64>().ok()? / 100.0;
        let exit = match e {
            "lvl" => Exit::Lvl,
            "trail" => Exit::Trail,
            x => Exit::R(x.strip_prefix('t')?.parse().ok()?),
        };
        Some(RuleSpec { buffer, exit })
    }

    /// Integer key (buffer, exit kind, target) for lookups.
    pub fn key(&self) -> (i64, u8, i64) {
        match self.exit {
            Exit::R(x) => (centi(self.buffer), 0, centi(x)),
            Exit::Lvl => (centi(self.buffer), 1, 0),
            Exit::Trail => (centi(self.buffer), 2, 0),
        }
    }

    /// The engine rule. Events are pre-selected per setup, so the selection takes every
    /// directional event.
    pub fn trade_rule(&self, name: &str, filters: Vec<Filter>) -> TradeRule {
        let (target, trail, time_stop) = match self.exit {
            Exit::R(x) => (Target::R(x), None, None),
            Exit::Lvl => (Target::NextLevel { min_r: 1.0 }, None, None),
            Exit::Trail => (
                Target::None,
                Some(Trail {
                    activate_r: 1.0,
                    atr_mult: 2.0,
                }),
                Some(96),
            ),
        };
        TradeRule {
            name: name.to_string(),
            select: Select {
                det: None,
                kinds: Vec::new(),
                dir: cs_backtest::rule::DirSel::Both,
            },
            entry: Entry::NextOpen,
            stop: StopRule::Structural {
                buffer_atr: self.buffer,
            },
            target,
            time_stop_bars: time_stop,
            trail,
            filters,
            overlap: Overlap::OnePerSetup,
            min_stop_atr: 0.2,
        }
    }
}

pub const FILTER_IDS: [&str; 11] = [
    "F00", "F01", "F02", "F03", "F04", "F05", "F06", "F07", "F08", "F09", "F10",
];

pub const FILTER_LABELS: [&str; 11] = [
    "none",
    "1h EMA stack with",
    "4h EMA stack with",
    "1h structure with",
    "4h structure with",
    "1h+4h EMA stack with",
    "4h EMA stack against",
    "ADX trend with",
    "ADX range",
    "session 07-21 UTC",
    "healthy pullback",
];

/// Engine filters of a filter id.
pub fn filters_of(id: usize) -> Vec<Filter> {
    let st = |slot, align| Filter::HtfStack { slot, align };
    match id {
        1 => vec![st(0, Align::With)],
        2 => vec![st(1, Align::With)],
        3 => vec![Filter::HtfStructure {
            slot: 0,
            align: Align::With,
        }],
        4 => vec![Filter::HtfStructure {
            slot: 1,
            align: Align::With,
        }],
        5 => vec![st(0, Align::With), st(1, Align::With)],
        6 => vec![st(1, Align::Against)],
        7 => vec![Filter::Regime(Align::With)],
        8 => vec![Filter::Regime(Align::Neutral)],
        9 => vec![Filter::Sessions(0b01110)],
        10 => vec![Filter::Healthy(true)],
        _ => Vec::new(),
    }
}

/// Filters F01, F03 and F05 need the 1h state, which a 1h series does not have.
pub fn filter_on_tf(id: usize, tf: Timeframe) -> bool {
    !(tf == Timeframe::H1 && matches!(id, 1 | 3 | 5))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Family {
    Main,
    Healthy,
    Frontier,
    Confluence,
}

impl Family {
    pub const fn label(self) -> &'static str {
        match self {
            Family::Main => "main",
            Family::Healthy => "healthy",
            Family::Frontier => "frontier",
            Family::Confluence => "confluence",
        }
    }
}

/// One pre-registered trial.
#[derive(Debug, Clone, Copy)]
pub struct Trial {
    pub id: u32,
    pub family: Family,
    pub setup: usize,
    pub dir: Dir,
    pub tf: Timeframe,
    pub rule: RuleSpec,
    pub filter: usize,
    /// Minimum confluence count (confluence family), else 0.
    pub conf_min: u8,
}

/// Lookup key of a trial: (setup, direction, time frame, filter, confluence, rule key).
pub type Key = (usize, Dir, usize, usize, u8, (i64, u8, i64));

impl Trial {
    pub fn key(&self) -> Key {
        (
            self.setup,
            self.dir,
            tf_index(self.tf),
            self.filter,
            self.conf_min,
            self.rule.key(),
        )
    }

    pub fn label(&self) -> String {
        let mut s = format!(
            "{} {} {} {} {}",
            SETUPS[self.setup].name,
            self.dir.label(),
            self.tf.label(),
            self.rule.name(),
            FILTER_IDS[self.filter]
        );
        if self.conf_min > 0 {
            s.push_str(" conf>=");
            s.push_str(&self.conf_min.to_string());
        }
        s
    }

    /// Plateau neighbours (protocol section 6), as keys; neighbours outside the space are
    /// dropped by the caller (they are simply not found).
    pub fn neighbour_keys(&self) -> Vec<Key> {
        let mut v = Vec::new();
        let base = |rule: RuleSpec, conf: u8| -> Key {
            (
                self.setup,
                self.dir,
                tf_index(self.tf),
                self.filter,
                conf,
                rule.key(),
            )
        };
        if self.family == Family::Confluence {
            for k in [self.conf_min.wrapping_sub(1), self.conf_min + 1] {
                if (1..=3).contains(&k) {
                    v.push(base(self.rule, k));
                }
            }
            return v;
        }
        for b in BUFFERS {
            if (b - self.rule.buffer).abs() > 1e-9 {
                v.push(base(
                    RuleSpec {
                        buffer: b,
                        exit: self.rule.exit,
                    },
                    0,
                ));
            }
        }
        match self.rule.exit {
            Exit::R(x) => {
                let ladder: &[f64] = if self.filter == 0 {
                    &LADDER
                } else {
                    &MAIN_TARGETS
                };
                if let Some(i) = ladder.iter().position(|y| (y - x).abs() < 1e-9) {
                    for j in [i.wrapping_sub(1), i + 1] {
                        if let Some(&t) = ladder.get(j) {
                            v.push(base(
                                RuleSpec {
                                    buffer: self.rule.buffer,
                                    exit: Exit::R(t),
                                },
                                0,
                            ));
                        }
                    }
                }
            }
            Exit::Lvl | Exit::Trail => v.push(base(
                RuleSpec {
                    buffer: self.rule.buffer,
                    exit: Exit::R(2.0),
                },
                0,
            )),
        }
        v
    }
}

/// Every pre-registered trial in a fixed order (time frame, setup, direction, family, rule,
/// filter, threshold); ids are positions in this order.
pub fn enumerate() -> Vec<Trial> {
    let mut v = Vec::new();
    let push = |v: &mut Vec<Trial>, mut t: Trial| {
        t.id = u32::try_from(v.len()).unwrap_or(u32::MAX);
        v.push(t);
    };
    for tf in TFS {
        for (si, st) in SETUPS.iter().enumerate() {
            if st.sub_1h_only && tf == Timeframe::H1 {
                continue;
            }
            for dir in Dir::ALL {
                let t0 = Trial {
                    id: 0,
                    family: Family::Main,
                    setup: si,
                    dir,
                    tf,
                    rule: RuleSpec {
                        buffer: 0.25,
                        exit: Exit::R(1.0),
                    },
                    filter: 0,
                    conf_min: 0,
                };
                for rule in RuleSpec::main() {
                    for f in 0..10 {
                        if filter_on_tf(f, tf) {
                            push(
                                &mut v,
                                Trial {
                                    rule,
                                    filter: f,
                                    ..t0
                                },
                            );
                        }
                    }
                }
                if st.name == "pullback" {
                    for rule in RuleSpec::main() {
                        push(
                            &mut v,
                            Trial {
                                family: Family::Healthy,
                                rule,
                                filter: 10,
                                ..t0
                            },
                        );
                    }
                }
                for b in BUFFERS {
                    for t in FRONTIER_TARGETS {
                        push(
                            &mut v,
                            Trial {
                                family: Family::Frontier,
                                rule: RuleSpec {
                                    buffer: b,
                                    exit: Exit::R(t),
                                },
                                ..t0
                            },
                        );
                    }
                }
                if CONF_SETUPS.contains(&st.name) {
                    for k in 1..=3 {
                        push(
                            &mut v,
                            Trial {
                                family: Family::Confluence,
                                rule: RuleSpec {
                                    buffer: 0.25,
                                    exit: Exit::R(2.0),
                                },
                                conf_min: k,
                                ..t0
                            },
                        );
                    }
                }
            }
        }
    }
    v
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn counts_match_search_space_md() {
        let t = enumerate();
        assert_eq!(t.len(), 34_584);
        let n = |f: Family| t.iter().filter(|x| x.family == f).count();
        assert_eq!(n(Family::Main), 31_580);
        assert_eq!(n(Family::Healthy), 60);
        assert_eq!(n(Family::Frontier), 2_800);
        assert_eq!(n(Family::Confluence), 144);
        let keys: HashSet<Key> = t.iter().map(Trial::key).collect();
        assert_eq!(keys.len(), t.len(), "trial keys must be unique");
        for (i, x) in t.iter().enumerate() {
            assert_eq!(x.id as usize, i);
        }
        let per_tf = |tf| {
            t.iter()
                .filter(|x| x.family == Family::Main && x.tf == tf)
                .count()
        };
        assert_eq!(per_tf(Timeframe::M5), 11_800);
        assert_eq!(per_tf(Timeframe::M15), 11_800);
        assert_eq!(per_tf(Timeframe::H1), 7_980);
    }

    #[test]
    fn rule_names_round_trip() {
        for r in RuleSpec::main() {
            assert_eq!(RuleSpec::parse(&r.name()), Some(r));
        }
        let f = RuleSpec {
            buffer: 1.0,
            exit: Exit::R(0.25),
        };
        assert_eq!(f.name(), "b100-t0.25");
        assert_eq!(RuleSpec::parse("b100-t0.25"), Some(f));
        assert_eq!(
            RuleSpec::parse("b025-t1.5").map(|r| r.exit),
            Some(Exit::R(1.5))
        );
    }

    #[test]
    fn neighbours_exist_in_space() {
        let t = enumerate();
        let keys: HashSet<Key> = t.iter().map(Trial::key).collect();
        let find = |label: &str| t.iter().find(|x| x.label() == label).copied();
        // Unfiltered 1 R: other buffer, 0.75 R and 1.5 R (frontier family).
        let a = find("sweep native 15m b025-t1 F00").unwrap();
        let nb = a.neighbour_keys();
        assert_eq!(nb.len(), 3);
        assert!(nb.iter().all(|k| keys.contains(k)));
        // Filtered 1 R: other buffer and 2 R only.
        let b = find("sweep native 15m b025-t1 F02").unwrap();
        let nb = b.neighbour_keys();
        assert_eq!(nb.len(), 2);
        assert!(nb.iter().all(|k| keys.contains(k)));
        // Trail: other buffer and 2 R.
        let c = find("bos reversed 1h b100-trail F07").unwrap();
        assert!(c.neighbour_keys().iter().all(|k| keys.contains(k)));
        // Confluence >= 2: thresholds 1 and 3.
        let d = find("sweep native 5m b025-t2 F00 conf>=2").unwrap();
        let nb = d.neighbour_keys();
        assert_eq!(nb.len(), 2);
        assert!(nb.iter().all(|k| keys.contains(k)));
        assert_eq!(
            find("sweep native 5m b025-t2 F00 conf>=3")
                .unwrap()
                .neighbour_keys()
                .len(),
            1
        );
        // Every neighbour of every trial is either in the space or dropped by the caller; at
        // least one neighbour exists for every trial.
        for x in &t {
            assert!(
                x.neighbour_keys().iter().any(|k| keys.contains(k)),
                "{}",
                x.label()
            );
        }
    }
}
