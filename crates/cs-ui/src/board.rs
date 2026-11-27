//! What the main view shows, decided once per alert when it arrives, and logged.
//!
//! * Alerts of one candle form a group. Both directions in a group: WAIT "rules disagree" (until
//!   the entry candle ends). One direction: the first alert in the fixed order of the
//!   pre-registered list is the headline (never ordered by past results), the others are
//!   counted as "+N more".
//! * The headline is shown only if no other shown alert is still running (one at a time) and
//!   the daily limits allow it (alerts per day, losses in a row, daily loss in R; the paper
//!   account is the journal's shown alerts resolved by the backtest rules).
//! * Late alerts (decided after the entry candle closed) and alerts without past results are
//!   never shown.
//! * The decision is made once and logged with the reason; mute, stale feed, source mismatch
//!   and the screen's symbol or time frame only hide the panel while they last.

use std::collections::HashSet;

use cs_live::feed::Shared;
use cs_live::journal::{self, Journal, Limits, LogRow, RowCtx, decision};
use cs_live::signal::stats::StatsBook;
use cs_live::signal::{Signal, TradeState};

/// What the panel knows when alerts arrive.
pub struct Arrival<'a> {
    pub now: i64,
    pub stats: Option<&'a StatsBook>,
    pub limits: Limits,
    pub account: f64,
    pub risk_pct: f64,
    /// What hides the panel right now ("muted", "mismatch", ...; empty = nothing).
    pub hidden: String,
}

#[derive(Debug, Default)]
pub struct Board {
    done: HashSet<String>,
    headline: Option<String>,
    /// Account size and risk % when the headline arrived (its size is frozen).
    sizing: (f64, f64),
    more: usize,
    disagree_until: Option<i64>,
}

impl Board {
    /// Forget the display state (feed switched). Logged decisions stay in the journal.
    pub fn reset(&mut self) {
        self.headline = None;
        self.more = 0;
        self.disagree_until = None;
    }

    /// Decide and log the alerts that arrived since the last call. Returns the new log rows.
    pub fn update(&mut self, sh: &Shared, a: &Arrival<'_>, j: &mut Journal) -> Vec<LogRow> {
        let fresh: Vec<&Signal> = sh
            .signals
            .iter()
            .filter(|s| !self.done.contains(&s.id))
            .collect();
        if fresh.is_empty() {
            return Vec::new();
        }
        let mut groups: Vec<(i64, Vec<&Signal>)> = Vec::new();
        for s in fresh {
            match groups.iter_mut().find(|(k, _)| *k == s.known_at) {
                Some((_, g)) => g.push(s),
                None => groups.push((s.known_at, vec![s])),
            }
        }
        groups.sort_by_key(|(k, _)| *k);
        let mut all_rows = Vec::new();
        for (_, mut g) in groups {
            g.sort_by_key(|s| s.rule);
            let dirs: HashSet<i8> = g.iter().map(|s| s.dir).collect();
            let mut decided: Vec<(&Signal, &'static str)> = Vec::new();
            let running = self.headline_running(sh);
            if g.iter().all(|s| !s.fresh) {
                decided.extend(g.iter().map(|s| (*s, decision::LATE)));
            } else if a.stats.is_none() {
                decided.extend(g.iter().map(|s| (*s, decision::NO_STATS)));
            } else if dirs.len() > 1 {
                let s0 = g[0];
                self.disagree_until = Some(s0.entry_time + s0.tf.ms());
                decided.extend(g.iter().map(|s| (*s, decision::DISAGREE)));
            } else {
                let first = if running {
                    decision::ONE_AT_A_TIME
                } else {
                    journal::hold(&a.limits, &journal::today(&j.rows, a.now)).decision()
                };
                if first == decision::SHOWN {
                    self.headline = Some(g[0].id.clone());
                    self.sizing = (a.account, a.risk_pct);
                    self.more = g.len() - 1;
                    self.disagree_until = None;
                }
                decided.push((g[0], first));
                decided.extend(g[1..].iter().map(|s| (*s, decision::ALSO)));
            }
            let mut rows = Vec::new();
            for (s, d) in decided {
                self.done.insert(s.id.clone());
                let hist = a
                    .stats
                    .and_then(|b| b.get(s.rule, &s.symbol))
                    .map(|r| r.validation);
                rows.push(LogRow::from_signal(
                    s,
                    &RowCtx {
                        logged_at: a.now,
                        decision: d.to_string(),
                        hidden: a.hidden.clone(),
                        hist_hit_rate: hist.map_or(f64::NAN, |h| h.hit_rate),
                        hist_n: hist.map_or(0, |h| h.n),
                        hist_exp_1x: hist.map_or(f64::NAN, |h| h.exp_1x),
                        account: a.account,
                        risk_pct: a.risk_pct,
                    },
                ));
            }
            if let Err(e) = j.append(rows.clone()) {
                eprintln!("journal not written: {e}");
            }
            all_rows.extend(rows);
        }
        if j.persist
            && let Err(e) = journal::sync_fill_sheet(j)
        {
            eprintln!("fill sheet not written: {e}");
        }
        all_rows
    }

    fn headline_running(&self, sh: &Shared) -> bool {
        self.headline().is_some_and(|id| {
            sh.signals
                .iter()
                .position(|s| s.id == id)
                .and_then(|i| sh.signal_states.get(i))
                .is_some_and(|st| *st == TradeState::Running)
        })
    }

    fn headline(&self) -> Option<&str> {
        self.headline.as_deref()
    }

    /// Account size and risk % when the shown alert arrived.
    pub fn sizing(&self) -> (f64, f64) {
        self.sizing
    }

    /// The shown alert while its trade is running, with the "+N more" count.
    pub fn active<'s>(&self, sh: &'s Shared) -> Option<(&'s Signal, usize)> {
        let id = self.headline()?;
        let i = sh.signals.iter().position(|s| s.id == id)?;
        (sh.signal_states.get(i) == Some(&TradeState::Running)).then(|| (&sh.signals[i], self.more))
    }

    /// Up and down alerts on the same candle, while its entry candle is open.
    pub fn disagree(&self, now: i64) -> bool {
        self.disagree_until.is_some_and(|t| now < t)
    }
}

/// An alert for tests: entry 90,000, 900 from the stop; a 2 R target or a trailing exit.
#[cfg(test)]
pub fn tests_alert(rule: u8, dir: i8, symbol: &str, exit: cs_live::signal::rules::Exit) -> Signal {
    use cs_live::signal::rules::Exit;
    let known_at = 1_772_553_600_000;
    let sd = f64::from(dir);
    let trail = matches!(exit, Exit::Trail);
    Signal {
        id: format!("{symbol}-1h-{known_at}-r{rule:02}"),
        symbol: symbol.into(),
        tf: cs_core::Timeframe::H1,
        rule,
        dir,
        known_idx: 10,
        known_at,
        decision_at: known_at + 3_600_000,
        entry_time: known_at + 3_600_000,
        entry: 90_000.0,
        stop: 90_000.0 - sd * 900.0,
        target: if trail {
            f64::NAN
        } else {
            90_000.0 + sd * 1_800.0
        },
        risk: 900.0,
        trail_from: if trail {
            90_000.0 + sd * 900.0
        } else {
            f64::NAN
        },
        time_exit: trail.then_some(known_at + 97 * 3_600_000),
        atr: 450.0,
        det: "fib",
        kind: "fib_zone_touch",
        fresh: true,
        decided_at: known_at + 3_601_000,
        window_start: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cs_core::Timeframe;

    pub fn alert(rule: u8, dir: i8, known_at: i64, fresh: bool) -> Signal {
        Signal {
            id: format!("BTCUSDT-1h-{known_at}-r{rule:02}"),
            symbol: "BTCUSDT".into(),
            tf: Timeframe::H1,
            rule,
            dir,
            known_idx: 10,
            known_at,
            decision_at: known_at + 3_600_000,
            entry_time: known_at + 3_600_000,
            entry: 90_000.0,
            stop: if dir > 0 { 89_100.0 } else { 90_900.0 },
            target: f64::NAN,
            risk: 900.0,
            trail_from: 90_000.0 + f64::from(dir) * 900.0,
            time_exit: Some(known_at + 97 * 3_600_000),
            atr: 450.0,
            det: "fib",
            kind: "fib_zone_touch",
            fresh,
            decided_at: known_at + 3_601_000,
            window_start: 0,
        }
    }

    fn book() -> StatsBook {
        let text = std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../reports/rule-stats.json"
        ))
        .unwrap();
        StatsBook::parse(&text).unwrap()
    }

    fn arrival(b: &StatsBook, now: i64) -> Arrival<'_> {
        Arrival {
            now,
            stats: Some(b),
            limits: Limits::default(),
            account: 10_000.0,
            risk_pct: 1.0,
            hidden: String::new(),
        }
    }

    fn shared(v: Vec<Signal>, st: Vec<TradeState>) -> Shared {
        Shared {
            signals: v,
            signal_states: st,
            has_rules: true,
            ..Shared::default()
        }
    }

    #[test]
    fn first_in_list_order_plus_more_and_disagree() {
        let b = book();
        let t0 = 1_772_553_600_000;
        let mut j = Journal::in_memory();
        let mut bd = Board::default();
        // Rules 10, 2 and 5 up on one candle: rule 2 is first in the list, "+2 more".
        let sh = shared(
            vec![
                alert(10, 1, t0, true),
                alert(5, 1, t0, true),
                alert(2, 1, t0, true),
            ],
            vec![TradeState::Running; 3],
        );
        let rows = bd.update(&sh, &arrival(&b, t0 + 3_601_000), &mut j);
        assert_eq!(rows.len(), 3);
        let (s, more) = bd.active(&sh).unwrap();
        assert_eq!((s.rule, more), (2, 2));
        // The size is frozen at the account size and risk % of the moment it arrived.
        assert_eq!(bd.sizing(), (10_000.0, 1.0));
        let mut later = arrival(&b, t0 + 3_700_000);
        later.account = 50_000.0;
        bd.update(&sh, &later, &mut j);
        assert_eq!(bd.sizing(), (10_000.0, 1.0));
        assert_eq!(rows.iter().filter(|r| r.decision == "shown").count(), 1);
        // A later candle with up and down: rules disagree, nothing new shown, logged as such.
        let t1 = t0 + 3_600_000;
        let mut v = sh.signals.clone();
        v.extend([alert(1, 1, t1, true), alert(6, -1, t1, true)]);
        let sh2 = shared(v, vec![TradeState::Running; 5]);
        let rows = bd.update(&sh2, &arrival(&b, t1 + 3_601_000), &mut j);
        assert!(rows.iter().all(|r| r.decision == "disagree"));
        assert!(bd.disagree(t1 + 3_700_000));
        assert!(!bd.disagree(t1 + 2 * 3_600_000));
        // The first headline keeps running and stays shown (never replaced).
        assert_eq!(bd.active(&sh2).unwrap().0.rule, 2);
    }

    #[test]
    fn one_at_a_time_late_and_limits() {
        let b = book();
        let t0 = 1_772_553_600_000;
        let mut j = Journal::in_memory();
        let mut bd = Board::default();
        let sh = shared(vec![alert(1, 1, t0, true)], vec![TradeState::Running]);
        bd.update(&sh, &arrival(&b, t0 + 3_601_000), &mut j);
        // While it runs, a new alert is not shown.
        let t1 = t0 + 3_600_000;
        let sh = shared(
            vec![
                alert(1, 1, t0, true),
                alert(2, -1, t1, true),
                alert(4, 1, t1 + 3_600_000, false),
            ],
            vec![TradeState::Running; 3],
        );
        let rows = bd.update(&sh, &arrival(&b, t1 + 3_601_000), &mut j);
        assert_eq!(rows[0].decision, "one_at_a_time");
        assert_eq!(rows[1].decision, "late");
        // Once ended, the panel clears.
        let sh = shared(
            sh.signals.clone(),
            vec![
                TradeState::Ended {
                    at: t1,
                    reason: cs_backtest::sim::ExitReason::Stop,
                };
                3
            ],
        );
        assert!(bd.active(&sh).is_none());
        // Three alerts already shown today: the fourth is held by the daily limit.
        let mut j = Journal::in_memory();
        let day = t0 - t0 % 86_400_000;
        let mut bd = Board::default();
        let mut v = Vec::new();
        for (i, h) in [1i64, 5, 9, 13].iter().enumerate() {
            v.push(alert(1, 1, day + h * 3_600_000, true));
            // Every earlier alert has ended; the new one runs.
            let st: Vec<TradeState> = (0..v.len())
                .map(|k| {
                    if k + 1 == v.len() {
                        TradeState::Running
                    } else {
                        TradeState::Ended {
                            at: day + h * 3_600_000,
                            reason: cs_backtest::sim::ExitReason::Trail,
                        }
                    }
                })
                .collect();
            let sh = shared(v.clone(), st);
            let rows = bd.update(&sh, &arrival(&b, day + h * 3_600_000 + 3_601_000), &mut j);
            let want = if i < 3 { "shown" } else { "limit_day" };
            assert_eq!(rows[0].decision, want, "alert {i}");
        }
    }
}
