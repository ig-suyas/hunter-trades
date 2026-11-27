//! Event-driven trade simulation for one rule.
//!
//! Timing (no lookahead): a signal from an event with `known_idx = k` is decided when candle `k`
//! has closed (`decision = open_time(k) + tf`). Everything the decision uses (event, features
//! and levels of candle `k`) is known then. The earliest fill is at the open of candle `k + 1`;
//! the engine refuses (error) to fill on a candle that opens before the decision time.
//!
//! Fills are pessimistic:
//! * stops trigger on a touch; targets need a trade strictly beyond the target price;
//! * a stop that is gapped through fills at the (worse) open; a target that is gapped through
//!   fills at the target price, never at the better open;
//! * if a candle touches both stop and target, its 1-minute candles decide the order; if one
//!   1-minute candle touches both, or no complete 1-minute data exist for the candle, the stop is
//!   assumed to come first;
//! * a limit entry fills at its limit price only when price trades strictly through it; inside
//!   the 1-minute candle of the fill (or the whole candle without 1-minute data) a stop touch
//!   counts, a target touch does not;
//! * every fill pays the taker fee, market fills also half the spread and slippage
//!   ([`CostModel`]).

use cs_core::{Candle, Event, price_eq};

use crate::Error;
use crate::cost::{CostModel, Fill, Funding};
use crate::market::Market;
use crate::rule::{Entry, LimitAt, Overlap, StopRule, Target, TradeRule};

/// One minute in ms.
pub const MINUTE_MS: i64 = 60_000;

/// Position sizing: fixed fractional risk of the equity at entry, capped by leverage.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Sizing {
    pub equity0: f64,
    /// Share of equity risked per trade (entry to stop, before costs). Default 0.01.
    pub risk_frac: f64,
    /// Maximum notional / equity. If the stop is so tight that `risk_frac` would need more, the
    /// position is smaller and the trade risks less than `risk_frac`. Default 10.
    pub max_leverage: f64,
}

impl Default for Sizing {
    fn default() -> Self {
        Sizing {
            equity0: 10_000.0,
            risk_frac: 0.01,
            max_leverage: 10.0,
        }
    }
}

/// Everything a simulation reads.
#[derive(Clone, Copy)]
pub struct Inputs<'a> {
    pub market: &'a Market,
    /// 1-minute candles sorted by open time (may be empty: then no intrabar resolution).
    pub m1: &'a [Candle],
    /// Funding settlements sorted by time (may be empty).
    pub funding: &'a [Funding],
    pub costs: CostModel,
    pub sizing: Sizing,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ExitReason {
    /// Initial stop.
    Stop,
    /// Stop after the trailing stop moved it.
    Trail,
    Target,
    Time,
    EndOfData,
}

impl ExitReason {
    pub const ALL: [ExitReason; 5] = [
        ExitReason::Stop,
        ExitReason::Trail,
        ExitReason::Target,
        ExitReason::Time,
        ExitReason::EndOfData,
    ];

    pub const fn label(self) -> &'static str {
        match self {
            ExitReason::Stop => "stop",
            ExitReason::Trail => "trail",
            ExitReason::Target => "target",
            ExitReason::Time => "time",
            ExitReason::EndOfData => "end_of_data",
        }
    }
}

/// One simulated trade. Prices are per unit of the base asset; `*_r` values are in R (initial
/// risk = `|entry_raw - stop0|` per unit); `net_r = gross_r - spread_r - slip_r - fee_r -
/// funding_r`.
#[derive(Debug, Clone, Copy)]
#[allow(clippy::struct_excessive_bools)]
pub struct Trade {
    pub det: &'static str,
    pub kind: &'static str,
    pub dir: i8,
    /// Candle index of the signal (`known_idx`) and its open time (`known_at`).
    pub signal_idx: u32,
    pub known_at: i64,
    /// Close of the signal candle: the earliest moment the trade may be entered.
    pub decision_at: i64,
    pub entry_idx: u32,
    /// Entry time: candle open, or the 1-minute candle of a limit fill when known.
    pub entry_time: i64,
    pub entry_market: bool,
    pub entry_raw: f64,
    pub entry_fill: f64,
    pub stop0: f64,
    /// `NaN` when the rule has no target.
    pub target: f64,
    pub risk: f64,
    pub exit_idx: u32,
    /// Exit time: candle open, or the 1-minute candle where the exit happened when resolved;
    /// close of the last candle for end-of-data exits.
    pub exit_time: i64,
    pub exit_raw: f64,
    pub exit_fill: f64,
    pub reason: ExitReason,
    pub gross_r: f64,
    pub spread_r: f64,
    pub slip_r: f64,
    pub fee_r: f64,
    pub funding_r: f64,
    pub net_r: f64,
    pub funding_events: u32,
    /// Share of equity actually risked (`risk_frac`, or less under the leverage cap).
    pub risk_frac: f64,
    /// Net result as a share of the equity at entry (`net_r * risk_frac`).
    pub ret: f64,
    pub equity_before: f64,
    /// Net result in account currency.
    pub pnl: f64,
    /// Both stop and target were touched inside the exit candle.
    pub ambiguous: bool,
    /// The exit was located with 1-minute candles.
    pub resolved_1m: bool,
    /// The stop was assumed to come first (same 1-minute candle, or no 1-minute data).
    pub stop_first: bool,
    /// Context of the signal candle, for breakdowns.
    pub regime: i8,
    pub session: u8,
}

/// Signals that did not become trades, by reason.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Skips {
    pub filtered: u32,
    pub overlap: u32,
    pub no_next_candle: u32,
    pub data_gap: u32,
    pub no_atr: u32,
    pub no_zone: u32,
    /// Stop on the wrong side of the entry (including price beyond the stop before entry).
    pub invalid_stop: u32,
    pub tight_stop: u32,
    pub no_level: u32,
    pub limit_expired: u32,
}

impl Skips {
    pub fn fields(&self) -> [(&'static str, u32); 10] {
        [
            ("filtered", self.filtered),
            ("overlap", self.overlap),
            ("no_next_candle", self.no_next_candle),
            ("data_gap", self.data_gap),
            ("no_atr", self.no_atr),
            ("no_zone", self.no_zone),
            ("invalid_stop", self.invalid_stop),
            ("tight_stop", self.tight_stop),
            ("no_level", self.no_level),
            ("limit_expired", self.limit_expired),
        ]
    }
}

/// How exits and fills were resolved.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct FillStats {
    /// Exit candles that touched both stop and target.
    pub ambiguous: u32,
    /// Of those, decided cleanly by 1-minute candles.
    pub ambiguous_resolved_1m: u32,
    /// Stop assumed first because one 1-minute candle touched both.
    pub stop_first_same_minute: u32,
    /// Stop assumed first because there were no usable 1-minute candles.
    pub stop_first_no_1m: u32,
    /// Exits located with 1-minute candles.
    pub exits_1m: u32,
    /// Candles whose 1-minute data were missing or did not match the candle.
    pub m1_unusable: u32,
    pub limit_fills: u32,
    pub limit_fills_1m: u32,
    pub marketable_limits: u32,
}

impl FillStats {
    pub fn fields(&self) -> [(&'static str, u32); 9] {
        [
            ("ambiguous", self.ambiguous),
            ("ambiguous_resolved_1m", self.ambiguous_resolved_1m),
            ("stop_first_same_minute", self.stop_first_same_minute),
            ("stop_first_no_1m", self.stop_first_no_1m),
            ("exits_1m", self.exits_1m),
            ("m1_unusable", self.m1_unusable),
            ("limit_fills", self.limit_fills),
            ("limit_fills_1m", self.limit_fills_1m),
            ("marketable_limits", self.marketable_limits),
        ]
    }
}

/// Result of one rule.
#[derive(Debug, Clone)]
pub struct Run {
    pub rule: TradeRule,
    pub costs: CostModel,
    pub sizing: Sizing,
    pub trades: Vec<Trade>,
    pub skips: Skips,
    pub fills: FillStats,
    /// Events matching the rule's selection.
    pub signals: u32,
    /// First candle open and last candle close of the market.
    pub span: (i64, i64),
}

#[derive(Clone, Copy)]
struct Hit {
    target: bool,
    raw: f64,
    both: bool,
}

/// Scan one candle (any resolution) for a stop or target hit. Gap rule at the open first, then
/// the touches; both touched = stop (pessimistic, flagged with `both`).
fn scan(c: &Candle, dir: i8, stop: f64, target: f64, allow_target: bool) -> Option<Hit> {
    let up = dir > 0;
    let s_touch = |p: f64| if up { p <= stop } else { p >= stop };
    let t_through =
        |p: f64| allow_target && target.is_finite() && if up { p > target } else { p < target };
    if s_touch(c.open) {
        return Some(Hit {
            target: false,
            raw: c.open,
            both: false,
        });
    }
    if t_through(c.open) {
        return Some(Hit {
            target: true,
            raw: target,
            both: false,
        });
    }
    let (adv, fav) = if up { (c.low, c.high) } else { (c.high, c.low) };
    match (s_touch(adv), t_through(fav)) {
        (true, t) => Some(Hit {
            target: false,
            raw: stop,
            both: t,
        }),
        (false, true) => Some(Hit {
            target: true,
            raw: target,
            both: false,
        }),
        (false, false) => None,
    }
}

enum Minutes<'a> {
    Ok(&'a [Candle]),
    /// No 1-minute data at all (or the setup candles are 1-minute candles themselves).
    Absent,
    /// 1-minute data exist but do not cover this candle completely or do not match it.
    Unusable,
}

struct Exit {
    hit: Hit,
    time: i64,
    resolved_1m: bool,
    ambiguous: bool,
}

enum Skip {
    NoNextCandle,
    DataGap,
    NoAtr,
    NoZone,
    InvalidStop,
    TightStop,
    NoLevel,
    /// Limit order expired; busy until the given time.
    LimitExpired(i64),
}

struct Sim<'a> {
    inp: Inputs<'a>,
    m: &'a Market,
    tf: i64,
    fills: FillStats,
}

impl<'a> Sim<'a> {
    fn minutes(&self, j: usize) -> Minutes<'a> {
        let m1: &'a [Candle] = self.inp.m1;
        let m: &'a Market = self.m;
        let need = self.tf / MINUTE_MS;
        if m1.is_empty() || need <= 1 {
            return Minutes::Absent;
        }
        let bar = &m.bars[j];
        let a = m1.partition_point(|x| x.open_time < bar.open_time);
        let b = m1.partition_point(|x| x.open_time < bar.open_time + self.tf);
        let s = &m1[a..b];
        let complete =
            s.len() as i64 == need && s.first().is_some_and(|x| x.open_time == bar.open_time);
        let hi = s.iter().map(|x| x.high).fold(f64::NEG_INFINITY, f64::max);
        let lo = s.iter().map(|x| x.low).fold(f64::INFINITY, f64::min);
        let open = s.first().map_or(f64::NAN, |x| x.open);
        if complete && price_eq(hi, bar.high) && price_eq(lo, bar.low) && price_eq(open, bar.open) {
            Minutes::Ok(s)
        } else {
            Minutes::Unusable
        }
    }

    /// Exit inside candle `j` for a position open since before its open.
    fn exit_in_bar(&mut self, j: usize, dir: i8, stop: f64, target: f64) -> Option<Exit> {
        let bar = self.m.bars[j];
        let bh = scan(&bar, dir, stop, target, true)?;
        if bh.both {
            self.fills.ambiguous += 1;
        }
        match self.minutes(j) {
            Minutes::Ok(ms) => {
                for c in ms {
                    if let Some(h) = scan(c, dir, stop, target, true) {
                        self.fills.exits_1m += 1;
                        if h.both {
                            self.fills.stop_first_same_minute += 1;
                        } else if bh.both {
                            self.fills.ambiguous_resolved_1m += 1;
                        }
                        return Some(Exit {
                            hit: h,
                            time: c.open_time,
                            resolved_1m: true,
                            ambiguous: bh.both,
                        });
                    }
                }
                // Cannot happen with matching data (checked in `minutes`); fall back.
                self.fills.m1_unusable += 1;
            }
            Minutes::Unusable => self.fills.m1_unusable += 1,
            Minutes::Absent => {}
        }
        if bh.both {
            self.fills.stop_first_no_1m += 1;
        }
        Some(Exit {
            hit: bh,
            time: bar.open_time,
            resolved_1m: false,
            ambiguous: bh.both,
        })
    }

    /// Exit inside candle `j` after a limit fill in it. `fill_minute`: index of the fill's
    /// 1-minute candle in `ms`, or `None` without usable 1-minute data.
    fn exit_in_fill_bar(
        &mut self,
        j: usize,
        dir: i8,
        stop: f64,
        target: f64,
        ms: Option<(&[Candle], usize)>,
    ) -> Option<Exit> {
        if let Some((ms, mi)) = ms {
            for (q, c) in ms[mi..].iter().enumerate() {
                // In the fill minute the order of the fill and the high/low is unknown: a stop
                // touch counts, a target touch does not.
                if let Some(h) = scan(c, dir, stop, target, q > 0) {
                    self.fills.exits_1m += 1;
                    if h.both {
                        self.fills.stop_first_same_minute += 1;
                    }
                    return Some(Exit {
                        hit: h,
                        time: c.open_time,
                        resolved_1m: true,
                        ambiguous: h.both,
                    });
                }
            }
            return None;
        }
        let bar = self.m.bars[j];
        scan(&bar, dir, stop, target, false).map(|h| Exit {
            hit: h,
            time: bar.open_time,
            resolved_1m: false,
            ambiguous: false,
        })
    }

    fn atr_before(&self, j: usize) -> f64 {
        j.checked_sub(1)
            .and_then(|i| self.m.features.get(i))
            .map_or(f64::NAN, |f| f.atr)
    }

    /// Price at time `t` for funding: the open of the 1-minute candle starting at `t`, else the
    /// open (or close, after it) of the setup candle containing `t`.
    fn price_at(&self, t: i64) -> f64 {
        let m1 = self.inp.m1;
        let i = m1.partition_point(|c| c.open_time < t);
        if let Some(c) = m1.get(i)
            && c.open_time == t
        {
            return c.open;
        }
        let bars = &self.m.bars;
        let k = bars.partition_point(|b| b.open_time <= t);
        match k.checked_sub(1).and_then(|k| bars.get(k)) {
            Some(b) if t < b.open_time + self.tf => b.open,
            Some(b) => b.close,
            None => bars.first().map_or(f64::NAN, |b| b.open),
        }
    }

    /// Funding paid per unit (positive = cost) for a position open from `t0` (exclusive) to `t1`
    /// (inclusive). Funding times are floored to the minute.
    fn funding(&self, dir: i8, t0: i64, t1: i64) -> (f64, u32) {
        if !self.inp.costs.funding {
            return (0.0, 0);
        }
        let fl = |t: i64| t.div_euclid(MINUTE_MS) * MINUTE_MS;
        let fs = self.inp.funding;
        let a = fs.partition_point(|f| fl(f.time) <= t0);
        let mut sum = 0.0;
        let mut n = 0;
        for f in &fs[a..] {
            let t = fl(f.time);
            if t > t1 {
                break;
            }
            if f.rate.is_finite() {
                sum += f64::from(dir) * self.price_at(t) * f.rate;
                n += 1;
            }
        }
        (sum, n)
    }

    #[allow(clippy::too_many_lines)]
    fn trade(&mut self, rule: &TradeRule, e: &Event) -> Result<Result<Trade, Skip>, Error> {
        let m = self.m;
        let n = m.bars.len();
        let k = e.known_idx as usize;
        let dir = e.dir;
        let sd = f64::from(dir);
        let f = &m.features[k];
        let decision = m.close_at(k);
        let atr = f.atr;
        if !(atr.is_finite() && atr > 0.0) {
            return Ok(Err(Skip::NoAtr));
        }
        let j0 = k + 1;
        if j0 >= n {
            return Ok(Err(Skip::NoNextCandle));
        }
        if m.bars[j0].open_time < decision {
            return Err(Error::Lookahead(format!(
                "entry candle {j0} opens at {} before signal candle {k} closes at {decision}",
                m.bars[j0].open_time
            )));
        }
        if m.bars[j0].open_time != decision {
            return Ok(Err(Skip::DataGap));
        }
        let structural = |buffer: f64| {
            let r = if dir > 0 {
                if e.lo.is_finite() {
                    e.lo
                } else {
                    m.bars[k].low
                }
            } else if e.hi.is_finite() {
                e.hi
            } else {
                m.bars[k].high
            };
            r - sd * buffer * atr
        };
        let costs = self.inp.costs;

        // ---- entry
        let (entry_idx, entry_time, entry_raw, market, fill_ms): (
            usize,
            i64,
            f64,
            bool,
            Option<Option<usize>>,
        );
        match rule.entry {
            Entry::NextOpen => {
                entry_idx = j0;
                entry_time = m.bars[j0].open_time;
                entry_raw = m.bars[j0].open;
                market = true;
                fill_ms = None;
            }
            Entry::Limit { at, timeout_bars } => {
                let l = match at {
                    LimitAt::ZoneNear => {
                        if dir > 0 {
                            e.hi
                        } else {
                            e.lo
                        }
                    }
                    LimitAt::ZoneMid => f64::midpoint(e.lo, e.hi),
                    LimitAt::Price => e.price,
                };
                if !l.is_finite() {
                    return Ok(Err(Skip::NoZone));
                }
                if let StopRule::Structural { buffer_atr } = rule.stop
                    && sd * (l - structural(buffer_atr)) <= 0.0
                {
                    return Ok(Err(Skip::InvalidStop));
                }
                let last = (k + timeout_bars.max(1) as usize).min(n - 1);
                let mut found = None;
                for (j, &b) in m.bars.iter().enumerate().take(last + 1).skip(j0) {
                    if j == j0 && sd * (b.open - l) <= 0.0 {
                        // Marketable when placed: fills at the open as a market order.
                        self.fills.marketable_limits += 1;
                        found = Some((j, b.open_time, b.open, true, None));
                        break;
                    }
                    let through = if dir > 0 { b.low < l } else { b.high > l };
                    if !through {
                        continue;
                    }
                    self.fills.limit_fills += 1;
                    let pos = match self.minutes(j) {
                        Minutes::Ok(ms) => ms
                            .iter()
                            .position(|c| if dir > 0 { c.low < l } else { c.high > l })
                            .map(|q| (q, ms[q].open_time)),
                        Minutes::Unusable => {
                            self.fills.m1_unusable += 1;
                            None
                        }
                        Minutes::Absent => None,
                    };
                    match pos {
                        Some((q, t)) => {
                            self.fills.limit_fills_1m += 1;
                            found = Some((j, t, l, false, Some(Some(q))));
                        }
                        None => found = Some((j, b.open_time, l, false, Some(None))),
                    }
                    break;
                }
                let Some((j, t, p, mk, fm)) = found else {
                    return Ok(Err(Skip::LimitExpired(m.close_at(last))));
                };
                entry_idx = j;
                entry_time = t;
                entry_raw = p;
                market = mk;
                fill_ms = fm;
            }
        }
        if entry_time < decision || entry_idx <= k {
            return Err(Error::Lookahead(format!(
                "entry at {entry_time} (candle {entry_idx}) before signal candle {k} closed at {decision}"
            )));
        }

        // ---- stop and target (from raw prices only, so costs never change the path)
        let stop0 = match rule.stop {
            StopRule::Structural { buffer_atr } => structural(buffer_atr),
            StopRule::Atr { mult } => entry_raw - sd * mult * atr,
        };
        if !(stop0.is_finite() && sd * (entry_raw - stop0) > 0.0) {
            return Ok(Err(Skip::InvalidStop));
        }
        let risk = (entry_raw - stop0).abs();
        if risk < rule.min_stop_atr * atr {
            return Ok(Err(Skip::TightStop));
        }
        let target = match rule.target {
            Target::R(x) => entry_raw + sd * x * risk,
            Target::None => f64::NAN,
            Target::NextLevel { min_r } => {
                let lv = &m.levels[k];
                let need = entry_raw + sd * min_r * risk;
                let pick = if dir > 0 {
                    lv.above
                        .iter()
                        .copied()
                        .find(|x| x.is_finite() && *x >= need)
                } else {
                    lv.below
                        .iter()
                        .copied()
                        .find(|x| x.is_finite() && *x <= need)
                };
                match pick {
                    Some(x) => x,
                    None => return Ok(Err(Skip::NoLevel)),
                }
            }
        };
        let ef: Fill = costs.fill(dir, entry_raw, self.atr_before(entry_idx), market);

        // ---- walk to the exit
        let mut stop = stop0;
        let mut moved = false;
        let mut best = entry_raw;
        let mut trail_on = false;
        let mut exit: Option<(usize, Exit, ExitReason)> = None;
        let first = match fill_ms {
            None => self.exit_in_bar(entry_idx, dir, stop, target),
            Some(fm) => {
                let ms = match (fm, self.minutes(entry_idx)) {
                    (Some(q), Minutes::Ok(ms)) => Some((ms, q)),
                    _ => None,
                };
                self.exit_in_fill_bar(entry_idx, dir, stop, target, ms)
            }
        };
        if let Some(x) = first {
            let r = if x.hit.target {
                ExitReason::Target
            } else {
                ExitReason::Stop
            };
            exit = Some((entry_idx, x, r));
        }
        let mut j = entry_idx;
        while exit.is_none() {
            // close of candle j: trailing stop update, effective from candle j + 1
            if let Some(tr) = rule.trail {
                let c = m.bars[j].close;
                best = if dir > 0 { best.max(c) } else { best.min(c) };
                if !trail_on && sd * (best - entry_raw) >= tr.activate_r * risk {
                    trail_on = true;
                }
                let a = m.features[j].atr;
                if trail_on && a.is_finite() {
                    let cand = best - sd * tr.atr_mult * a;
                    let next = if dir > 0 {
                        stop.max(cand)
                    } else {
                        stop.min(cand)
                    };
                    if next.to_bits() != stop.to_bits() {
                        stop = next;
                        moved = true;
                    }
                }
            }
            j += 1;
            if j >= n {
                break;
            }
            if let Some(t) = rule.time_stop_bars
                && j >= entry_idx + t.max(1) as usize
            {
                let b = m.bars[j];
                exit = Some((
                    j,
                    Exit {
                        hit: Hit {
                            target: false,
                            raw: b.open,
                            both: false,
                        },
                        time: b.open_time,
                        resolved_1m: false,
                        ambiguous: false,
                    },
                    ExitReason::Time,
                ));
                break;
            }
            if let Some(x) = self.exit_in_bar(j, dir, stop, target) {
                let r = if x.hit.target {
                    ExitReason::Target
                } else if moved {
                    ExitReason::Trail
                } else {
                    ExitReason::Stop
                };
                exit = Some((j, x, r));
            }
        }
        let (exit_idx, x, reason) = exit.unwrap_or_else(|| {
            let last = m.bars[n - 1];
            (
                n - 1,
                Exit {
                    hit: Hit {
                        target: false,
                        raw: last.close,
                        both: false,
                    },
                    time: m.close_at(n - 1),
                    resolved_1m: false,
                    ambiguous: false,
                },
                ExitReason::EndOfData,
            )
        });
        let atr_x = if reason == ExitReason::EndOfData {
            m.features[n - 1].atr
        } else {
            self.atr_before(exit_idx)
        };
        let xf = costs.fill(-dir, x.hit.raw, atr_x, true);
        let (fund, fund_n) = self.funding(dir, entry_time, x.time);
        let gross_r = sd * (x.hit.raw - entry_raw) / risk;
        let spread_r = (ef.spread + xf.spread) / risk;
        let slip_r = (ef.slip + xf.slip) / risk;
        let fee_r = (ef.fee + xf.fee) / risk;
        let funding_r = fund / risk;
        let net_r = gross_r - spread_r - slip_r - fee_r - funding_r;
        let sz = self.inp.sizing;
        let risk_frac = sz.risk_frac.min(sz.max_leverage * risk / entry_raw);
        Ok(Ok(Trade {
            det: e.det,
            kind: e.kind,
            dir,
            signal_idx: e.known_idx,
            known_at: e.known_at,
            decision_at: decision,
            entry_idx: entry_idx as u32,
            entry_time,
            entry_market: market,
            entry_raw,
            entry_fill: ef.price,
            stop0,
            target,
            risk,
            exit_idx: exit_idx as u32,
            exit_time: x.time,
            exit_raw: x.hit.raw,
            exit_fill: xf.price,
            reason,
            gross_r,
            spread_r,
            slip_r,
            fee_r,
            funding_r,
            net_r,
            funding_events: fund_n,
            risk_frac,
            ret: net_r * risk_frac,
            equity_before: 0.0,
            pnl: 0.0,
            ambiguous: x.ambiguous,
            resolved_1m: x.resolved_1m,
            stop_first: x.hit.both || (x.ambiguous && !x.resolved_1m),
            regime: f.regime,
            session: f.session,
        }))
    }
}

/// Simulate one rule over the market.
pub fn simulate(inp: &Inputs<'_>, rule: &TradeRule) -> Result<Run, Error> {
    let m = inp.market;
    let n = m.bars.len();
    let mut run = Run {
        rule: rule.clone(),
        costs: inp.costs,
        sizing: inp.sizing,
        trades: Vec::new(),
        skips: Skips::default(),
        fills: FillStats::default(),
        signals: 0,
        span: if n == 0 {
            (0, 0)
        } else {
            (m.bars[0].open_time, m.close_at(n - 1))
        },
    };
    let mut sim = Sim {
        inp: *inp,
        m,
        tf: m.tf_ms,
        fills: FillStats::default(),
    };
    // Busy-until per direction (long, short): exit time of the last trade or expiry of the last
    // limit order. Known at a decision time because only past exits/expiries are compared.
    let mut busy = [i64::MIN; 2];
    for e in m.events.iter().filter(|e| rule.select.matches(e)) {
        run.signals += 1;
        let k = e.known_idx as usize;
        if k >= n || m.bars[k].open_time != e.known_at {
            return Err(Error::Lookahead(format!(
                "event {}/{} known_at does not match candle {k}",
                e.det, e.kind
            )));
        }
        let decision = m.close_at(k);
        let side = usize::from(e.dir < 0);
        let blocked = match rule.overlap {
            Overlap::OnePerSetup => busy[0].max(busy[1]) > decision,
            Overlap::OnePerDirection => busy[side] > decision,
            Overlap::Unlimited => false,
        };
        if blocked {
            run.skips.overlap += 1;
            continue;
        }
        if !rule.filters_pass(e, &m.features[k]) {
            run.skips.filtered += 1;
            continue;
        }
        match sim.trade(rule, e)? {
            Ok(t) => {
                busy[side] = t.exit_time;
                run.trades.push(t);
            }
            Err(s) => {
                let sk = &mut run.skips;
                match s {
                    Skip::NoNextCandle => sk.no_next_candle += 1,
                    Skip::DataGap => sk.data_gap += 1,
                    Skip::NoAtr => sk.no_atr += 1,
                    Skip::NoZone => sk.no_zone += 1,
                    Skip::InvalidStop => sk.invalid_stop += 1,
                    Skip::TightStop => sk.tight_stop += 1,
                    Skip::NoLevel => sk.no_level += 1,
                    Skip::LimitExpired(until) => {
                        sk.limit_expired += 1;
                        busy[side] = until;
                    }
                }
            }
        }
    }
    run.fills = sim.fills;
    apply_equity(&mut run.trades, inp.sizing.equity0);
    Ok(run)
}

/// Compound the account: each trade risks its `risk_frac` of the equity at its entry (after all
/// exits up to then; entries at the same instant as an exit are sized first).
fn apply_equity(trades: &mut [Trade], equity0: f64) {
    let mut ev: Vec<(i64, u8, usize)> = Vec::with_capacity(trades.len() * 2);
    for (i, t) in trades.iter().enumerate() {
        ev.push((t.entry_time, 0, i));
        ev.push((t.exit_time, 1, i));
    }
    ev.sort_unstable();
    let mut eq = equity0;
    for (_, kind, i) in ev {
        let t = &mut trades[i];
        if kind == 0 {
            t.equity_before = eq;
        } else {
            t.pnl = t.ret * t.equity_before;
            eq += t.pnl;
        }
    }
}
