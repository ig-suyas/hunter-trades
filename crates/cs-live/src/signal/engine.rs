//! The signal engine: the 10 frozen rules run through the **backtest's own code**
//! ([`cs_backtest::sim::simulate`] over a [`cs_backtest::market::Market`]) on the live candles.
//!
//! * Candles, context features and detector events come from the live engine
//!   ([`crate::engine::LiveEngine`], which the parity tests prove bit-identical to
//!   `Market::prepare`); support/resistance levels for the next-level exit come from
//!   `Market::prepare` itself.
//! * Closed candles only. A decision about candle `k` (the `known_at` candle) is made once, when
//!   the open price of candle `k + 1` is known: from the first in-progress websocket frame of
//!   `k + 1` (its open never changes), or from the closed candle `k + 1` when no in-progress
//!   frame came (then the alert is marked late and is not shown). The backtest enters at that
//!   same open, so the decision uses exactly what the backtest uses: candles `<= k` and the open
//!   of `k + 1`.
//! * To get the backtest's decision without editing the engine, `simulate` runs on the market
//!   plus one provisional candle `k + 1` that holds only the real open (high = low = close =
//!   open). Entry, stop, target and the one-trade-at-a-time check at `k` depend only on candles
//!   `<= k` and that open, so the provisional candle cannot change them; nothing after its open is
//!   read, and the provisional candle is removed again.
//! * Alerts are append-only: once made, an alert is never changed or removed. Only its status
//!   (still running / ended at a time) is updated as later candles close.

use cs_backtest::cost::CostModel;
use cs_backtest::market::{FixedSource, Levels, Market, NL};
use cs_backtest::sim::{ExitReason, Inputs, Run, Sizing, Trade, simulate};
use cs_core::{Candle, Event, Timeframe};
use cs_patterns::context::Features;

use super::rules::{self, RuleDef};

/// A decided alert. Never changed after it is made.
#[derive(Debug, Clone, PartialEq)]
pub struct Signal {
    /// Unique id: symbol, time frame, signal candle and rule.
    pub id: String,
    pub symbol: String,
    pub tf: Timeframe,
    /// Rule list position (1-10).
    pub rule: u8,
    /// +1 up (buy), -1 down (sell).
    pub dir: i8,
    /// Index and open time of the signal (`known_at`) candle in the engine's stream.
    pub known_idx: u32,
    pub known_at: i64,
    /// Close of the signal candle = open of the entry candle (the earliest possible entry).
    pub decision_at: i64,
    pub entry_time: i64,
    /// Entry at the open of the next candle, initial stop, target (`NaN`: the rule has none).
    pub entry: f64,
    pub stop: f64,
    pub target: f64,
    /// Price per unit between entry and stop (1 R before costs).
    pub risk: f64,
    /// Trailing exit: price at which the trail starts (`NaN` without a trailing exit).
    pub trail_from: f64,
    /// Latest exit time (time stop), if the rule has one.
    pub time_exit: Option<i64>,
    /// ATR(14) of the signal candle (cost estimate).
    pub atr: f64,
    pub det: &'static str,
    pub kind: &'static str,
    /// Decided while the entry candle was still open (true) or only after it had closed.
    pub fresh: bool,
    /// Exchange time when the alert was decided.
    pub decided_at: i64,
    /// Open time of the first candle of the engine's stream (the resolver rebuilds the same
    /// market from it).
    pub window_start: i64,
}

impl Signal {
    pub fn rule_def(&self) -> Option<&'static RuleDef> {
        rules::by_order(self.rule)
    }
}

/// What the latest run says about an alert's virtual trade.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TradeState {
    Running,
    Ended { at: i64, reason: ExitReason },
}

/// How long after the entry candle's open an alert still counts as fresh.
pub const FRESH_MS: i64 = 60_000;

pub struct SignalEngine {
    symbol: String,
    tf: Timeframe,
    rules: Vec<&'static RuleDef>,
    market: Market,
    /// Per rule: the setup's events as traded (flipped for reversed rules), in `known_idx` order.
    rule_events: Vec<Vec<Event>>,
    /// Number of candles whose levels are final (computed by `Market::prepare`).
    levels_done: usize,
    /// Candle waiting for the next open.
    pending: Option<usize>,
    live: bool,
    signals: Vec<Signal>,
    states: Vec<TradeState>,
    errors: u64,
    pub last_error: Option<String>,
}

fn nan_levels() -> Levels {
    Levels {
        above: [f64::NAN; NL],
        below: [f64::NAN; NL],
    }
}

impl SignalEngine {
    /// Engine for one symbol and time frame. `None` when no frozen rule covers the time frame.
    pub fn new(symbol: &str, tf: Timeframe) -> Option<SignalEngine> {
        let rules = rules::rules_for(tf);
        if rules.is_empty() {
            return None;
        }
        let n = rules.len();
        Some(SignalEngine {
            symbol: symbol.to_uppercase(),
            tf,
            rules,
            market: Market {
                tf_ms: tf.ms(),
                bars: Vec::new(),
                features: Vec::new(),
                levels: Vec::new(),
                events: Vec::new(),
                source: "live".into(),
            },
            rule_events: vec![Vec::new(); n],
            levels_done: 0,
            pending: None,
            live: false,
            signals: Vec::new(),
            states: Vec::new(),
            errors: 0,
            last_error: None,
        })
    }

    pub fn tf(&self) -> Timeframe {
        self.tf
    }

    pub fn rules(&self) -> &[&'static RuleDef] {
        &self.rules
    }

    /// Every alert made so far (append-only), oldest first.
    pub fn signals(&self) -> &[Signal] {
        &self.signals
    }

    /// Trade state of each alert (same order as [`SignalEngine::signals`]).
    pub fn states(&self) -> &[TradeState] {
        &self.states
    }

    pub fn errors(&self) -> u64 {
        self.errors
    }

    /// Candles in the engine's stream.
    pub fn len(&self) -> usize {
        self.market.bars.len()
    }

    pub fn is_empty(&self) -> bool {
        self.market.bars.is_empty()
    }

    /// The candles, features and per-rule events as held (parity tests compare them with
    /// `Market::prepare`).
    pub fn market(&self) -> &Market {
        &self.market
    }

    pub fn rule_events(&self) -> &[Vec<Event>] {
        &self.rule_events
    }

    /// From now on decisions are made (before: warm-up history, never decided or shown).
    pub fn set_live(&mut self) {
        self.live = true;
    }

    pub fn is_live(&self) -> bool {
        self.live
    }

    /// The last closed candle still waits for the next open (its decision is not made yet).
    pub fn waiting(&self) -> bool {
        self.pending.is_some()
    }

    /// A closed candle the live engine accepted, with its features and the events that became
    /// known at its close. If the previous candle still waits for its decision (no in-progress
    /// frame came), it is decided first with this candle's open and marked late.
    pub fn on_closed(&mut self, c: &Candle, f: Features, events: &[Event]) -> Vec<Signal> {
        let mut out = Vec::new();
        if let Some(k) = self.pending.take()
            && self.live
            && c.open_time == self.market.bars[k].open_time + self.tf.ms()
        {
            out = self.decide(k, c.open, false, c.open_time + self.tf.ms());
        }
        self.market.bars.push(*c);
        self.market.features.push(f);
        self.market.levels.push(nan_levels());
        for e in events {
            for (r, def) in self.rules.iter().enumerate() {
                if def.matches(e) {
                    self.rule_events[r].push(def.trade_event(e));
                }
            }
        }
        self.pending = Some(self.market.bars.len() - 1);
        out
    }

    /// The open price of the candle after the last closed one became known (first in-progress
    /// frame). `event_time` is the exchange time of that frame.
    pub fn on_next_open(&mut self, open_time: i64, open: f64, event_time: i64) -> Vec<Signal> {
        let Some(k) = self.pending else {
            return Vec::new();
        };
        if !self.live
            || open_time != self.market.bars[k].open_time + self.tf.ms()
            || !(open.is_finite() && open > 0.0)
        {
            return Vec::new();
        }
        self.pending = None;
        let fresh = event_time - open_time <= FRESH_MS;
        self.decide(k, open, fresh, event_time)
    }

    /// Levels of candles `..=k` from `Market::prepare` (the backtest's own level tracker).
    fn ensure_levels(&mut self, k: usize) -> Result<(), String> {
        if self.levels_done > k {
            return Ok(());
        }
        let m = Market::prepare(
            self.market.bars[..=k].to_vec(),
            self.tf.ms(),
            &FixedSource(Vec::new()),
        )
        .map_err(|e| e.to_string())?;
        for (i, l) in m.levels.into_iter().enumerate() {
            self.market.levels[i] = l;
        }
        self.levels_done = k + 1;
        Ok(())
    }

    /// `simulate` reads a candle's levels only for an event of a next-level rule that passes the
    /// rule's filter. True when such an event at or before `k` has no final levels yet.
    fn needs_levels(&self, r: usize, k: usize) -> bool {
        if !matches!(self.rules[r].exit, rules::Exit::Level) {
            return false;
        }
        let rule = self.rules[r].trade_rule();
        self.rule_events[r]
            .iter()
            .rev()
            .take_while(|e| e.known_idx as usize >= self.levels_done)
            .any(|e| {
                let i = e.known_idx as usize;
                i <= k && rule.filters_pass(e, &self.market.features[i])
            })
    }

    fn run_rule(&mut self, r: usize) -> Result<Run, String> {
        let rule = self.rules[r].trade_rule();
        self.market.events = std::mem::take(&mut self.rule_events[r]);
        let inp = Inputs {
            market: &self.market,
            m1: &[],
            funding: &[],
            costs: CostModel::BINANCE_UM,
            sizing: Sizing::default(),
        };
        let res = simulate(&inp, &rule).map_err(|e| format!("{}: {e}", rule.name));
        self.rule_events[r] = std::mem::take(&mut self.market.events);
        res
    }

    /// The alert of the backtest's trade `t` (decided at candle `t.signal_idx`).
    fn alert(&self, def: &RuleDef, t: &Trade, fresh: bool, now: i64) -> Signal {
        let tf = self.tf.ms();
        let trail_from = match def.exit {
            rules::Exit::Trail => t.entry_raw + f64::from(t.dir) * rules::TRAIL_ACTIVATE_R * t.risk,
            _ => f64::NAN,
        };
        Signal {
            id: format!(
                "{}-{}-{}-r{:02}",
                self.symbol,
                self.tf.label(),
                t.known_at,
                def.order
            ),
            symbol: self.symbol.clone(),
            tf: self.tf,
            rule: def.order,
            dir: t.dir,
            known_idx: t.signal_idx,
            known_at: t.known_at,
            decision_at: t.decision_at,
            entry_time: t.entry_time,
            entry: t.entry_raw,
            stop: t.stop0,
            target: t.target,
            risk: t.risk,
            trail_from,
            time_exit: def
                .time_stop_bars()
                .map(|b| t.entry_time + i64::from(b) * tf),
            atr: self.market.features[t.signal_idx as usize].atr,
            det: t.det,
            kind: t.kind,
            fresh,
            decided_at: now,
            window_start: self.market.bars[0].open_time,
        }
    }

    /// Decide candle `k` with the next candle's open `next_open`.
    fn decide(&mut self, k: usize, next_open: f64, fresh: bool, now: i64) -> Vec<Signal> {
        let tf = self.tf.ms();
        let known_at = self.market.bars[k].open_time;
        // Rules with an event at k, and rules whose earlier alerts may still be running.
        let mut todo = Vec::new();
        for r in 0..self.rules.len() {
            let fires = self.rule_events[r]
                .last()
                .is_some_and(|e| e.known_idx as usize == k);
            let order = self.rules[r].order;
            let running = self
                .signals
                .iter()
                .zip(&self.states)
                .any(|(s, st)| s.rule == order && *st == TradeState::Running);
            if fires || running {
                todo.push(r);
            }
        }
        if todo.is_empty() {
            return Vec::new();
        }
        if todo.iter().any(|&r| self.needs_levels(r, k))
            && let Err(e) = self.ensure_levels(k)
        {
            self.errors += 1;
            self.last_error = Some(e);
            return Vec::new();
        }
        // Provisional candle k + 1: the real open only.
        let mut prov = self.market.bars[k];
        prov.open_time = known_at + tf;
        prov.open = next_open;
        prov.high = next_open;
        prov.low = next_open;
        prov.close = next_open;
        prov.volume = 0.0;
        let mut pf = self.market.features[k];
        pf.idx += 1;
        pf.open_time = prov.open_time;
        pf.atr = f64::NAN;
        self.market.bars.push(prov);
        self.market.features.push(pf);
        self.market.levels.push(nan_levels());
        let mut out = Vec::new();
        for r in todo {
            let run = match self.run_rule(r) {
                Ok(run) => run,
                Err(e) => {
                    self.errors += 1;
                    self.last_error = Some(e);
                    continue;
                }
            };
            let def = self.rules[r];
            // Status of earlier alerts of this rule.
            for (s, st) in self.signals.iter().zip(self.states.iter_mut()) {
                if s.rule != def.order || *st != TradeState::Running {
                    continue;
                }
                if let Some(t) = run.trades.iter().find(|t| t.signal_idx == s.known_idx)
                    && t.reason != ExitReason::EndOfData
                {
                    *st = TradeState::Ended {
                        at: t.exit_time,
                        reason: t.reason,
                    };
                }
            }
            if let Some(t) = run.trades.iter().find(|t| t.signal_idx as usize == k) {
                out.push(self.alert(def, t, fresh, now));
            }
        }
        self.market.bars.pop();
        self.market.features.pop();
        self.market.levels.pop();
        // Fixed list order inside one candle, whatever order the rules ran in.
        out.sort_by_key(|s| s.rule);
        for s in &out {
            self.signals.push(s.clone());
            self.states.push(TradeState::Running);
        }
        out
    }
}
