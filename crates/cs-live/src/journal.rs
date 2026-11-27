//! The alert journal in `data/signals/` (git-ignored, local only):
//!
//! * `signals.csv`: every alert the signal engine made while the overlay ran (shown or not, with
//!   the reason), the rule's past results at that moment, the size for the user's settings, and
//!   the outcome once resolved.
//! * Outcomes are resolved with the **backtest's own** `Market::prepare` + `simulate` on the
//!   same candles the live engine used (from the logged window start), with 1-minute candles and
//!   funding when available: exactly the backtest's trade rules.
//! * `fills.csv`: one row per shown alert, pre-filled by the tool; the user types only the four
//!   numbers of the real demo fill (entry, exit, size, fees). Import turns complete rows into
//!   `demo-trades.csv`, which `candlescope journal-report` evaluates.
//! * Paper account, daily limits and the calibration table are computed from `signals.csv`.

// Counts shown as rates are far below 2^52.
#![allow(clippy::cast_precision_loss, clippy::too_many_lines)]

use std::collections::BTreeMap;
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use cs_backtest::cost::{CostModel, Funding};
use cs_backtest::market::{DetectorSource, Market};
use cs_backtest::sim::{ExitReason, Inputs, Sizing, simulate};
use cs_core::time::{day_start, fmt_minute};
use cs_core::{Candle, Timeframe, price_eq};
use cs_stats::bootstrap::block_mean_ci;

use crate::signal::rules::{self, HitKind};
use crate::signal::sizing::MAX_LEVERAGE;

pub const DAY_MS: i64 = 86_400_000;

pub fn dir(root: &Path) -> PathBuf {
    root.join("data").join("signals")
}

// ------------------------------------------------------------------------------------ log rows

/// Why an alert was or was not shown when it arrived.
pub mod decision {
    /// The headline alert, shown.
    pub const SHOWN: &str = "shown";
    /// Same direction as the headline on the same candle, later in the list ("+N more").
    pub const ALSO: &str = "also";
    /// Up and down alerts on the same candle: WAIT "rules disagree".
    pub const DISAGREE: &str = "disagree";
    pub const LIMIT_DAY: &str = "limit_day";
    pub const LIMIT_LOSSES: &str = "limit_losses";
    pub const LIMIT_LOSS_R: &str = "limit_loss_r";
    /// Another shown alert was still running.
    pub const ONE_AT_A_TIME: &str = "one_at_a_time";
    /// Decided only after the entry candle had closed (feed gap or start in mid-candle).
    pub const LATE: &str = "late";
    /// The past-results file was missing.
    pub const NO_STATS: &str = "no_stats";
}

/// One alert in `signals.csv`.
#[derive(Debug, Clone, PartialEq)]
pub struct LogRow {
    pub id: String,
    pub logged_at: i64,
    pub symbol: String,
    pub tf: Timeframe,
    pub rule: u8,
    pub label: String,
    pub dir: i8,
    pub det: String,
    pub kind: String,
    pub known_at: i64,
    pub entry_time: i64,
    pub entry: f64,
    pub stop: f64,
    /// `NaN`: the rule has no target.
    pub target: f64,
    pub trail_from: f64,
    pub time_exit: Option<i64>,
    pub window_start: i64,
    /// "fresh" or "late".
    pub status: String,
    pub decision: String,
    /// What hid the panel when the alert arrived ("muted", "mismatch", ...; empty = nothing).
    pub hidden: String,
    pub hit_kind: String,
    pub hist_hit_rate: f64,
    pub hist_n: u64,
    pub hist_exp_1x: f64,
    pub account: f64,
    pub risk_pct: f64,
    pub size: f64,
    pub outcome: Option<Outcome>,
    pub note: String,
}

/// Resolved outcome of an alert's trade (the backtest's own simulation).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Outcome {
    pub exit_time: i64,
    pub reason: &'static str,
    pub exit_price: f64,
    pub gross_r: f64,
    pub net_r_1x: f64,
    pub net_r_2x: f64,
    pub hit: bool,
}

pub const LOG_HEADER: &str = "id,logged_utc,logged_ms,symbol,tf,rule,label,dir,det,kind,signal_candle_utc,known_at_ms,entry_time_ms,entry,stop,target,trail_from,time_exit_ms,window_start_ms,status,decision,hidden,hit_kind,hist_hit_rate,hist_n,hist_exp_1x,account,risk_pct,size,resolved,exit_time_ms,exit_utc,exit_reason,exit_price,gross_r,net_r_1x,net_r_2x,hit,note";

fn num(x: f64) -> String {
    if x.is_finite() {
        format!("{x}")
    } else {
        String::new()
    }
}

fn clean(s: &str) -> String {
    s.replace([',', '\n', '\r'], ";")
}

fn reason_label(r: &str) -> &'static str {
    ExitReason::ALL
        .iter()
        .find(|x| x.label() == r)
        .map_or("", |x| x.label())
}

/// What the panel knew when an alert arrived.
#[derive(Debug, Clone, Default)]
pub struct RowCtx {
    pub logged_at: i64,
    pub decision: String,
    pub hidden: String,
    pub hist_hit_rate: f64,
    pub hist_n: u64,
    pub hist_exp_1x: f64,
    pub account: f64,
    pub risk_pct: f64,
}

impl LogRow {
    /// The log row of an alert.
    pub fn from_signal(s: &crate::signal::Signal, c: &RowCtx) -> LogRow {
        let def = rules::by_order(s.rule);
        let size = crate::signal::sizing::size(c.account, c.risk_pct, s.entry, s.stop)
            .map_or(f64::NAN, |z| z.qty);
        LogRow {
            id: s.id.clone(),
            logged_at: c.logged_at,
            symbol: s.symbol.clone(),
            tf: s.tf,
            rule: s.rule,
            label: def.map_or_else(String::new, |d| d.label.to_string()),
            dir: s.dir,
            det: s.det.to_string(),
            kind: s.kind.to_string(),
            known_at: s.known_at,
            entry_time: s.entry_time,
            entry: s.entry,
            stop: s.stop,
            target: s.target,
            trail_from: s.trail_from,
            time_exit: s.time_exit,
            window_start: s.window_start,
            status: if s.fresh { "fresh" } else { "late" }.to_string(),
            decision: c.decision.clone(),
            hidden: c.hidden.clone(),
            hit_kind: def.map_or("", |d| d.hit_kind().key()).to_string(),
            hist_hit_rate: c.hist_hit_rate,
            hist_n: c.hist_n,
            hist_exp_1x: c.hist_exp_1x,
            account: c.account,
            risk_pct: c.risk_pct,
            size,
            outcome: None,
            note: String::new(),
        }
    }

    pub fn to_csv(&self) -> String {
        let o = self.outcome;
        [
            clean(&self.id),
            fmt_minute(self.logged_at),
            self.logged_at.to_string(),
            clean(&self.symbol),
            self.tf.label().to_string(),
            self.rule.to_string(),
            clean(&self.label),
            self.dir.to_string(),
            clean(&self.det),
            clean(&self.kind),
            fmt_minute(self.known_at),
            self.known_at.to_string(),
            self.entry_time.to_string(),
            num(self.entry),
            num(self.stop),
            num(self.target),
            num(self.trail_from),
            self.time_exit.map_or_else(String::new, |t| t.to_string()),
            self.window_start.to_string(),
            clean(&self.status),
            clean(&self.decision),
            clean(&self.hidden),
            clean(&self.hit_kind),
            num(self.hist_hit_rate),
            self.hist_n.to_string(),
            num(self.hist_exp_1x),
            num(self.account),
            num(self.risk_pct),
            num(self.size),
            if o.is_some() { "1" } else { "0" }.to_string(),
            o.map_or_else(String::new, |o| o.exit_time.to_string()),
            o.map_or_else(String::new, |o| fmt_minute(o.exit_time)),
            o.map_or_else(String::new, |o| o.reason.to_string()),
            o.map_or_else(String::new, |o| num(o.exit_price)),
            o.map_or_else(String::new, |o| num(o.gross_r)),
            o.map_or_else(String::new, |o| num(o.net_r_1x)),
            o.map_or_else(String::new, |o| num(o.net_r_2x)),
            o.map_or_else(String::new, |o| if o.hit { "1" } else { "0" }.to_string()),
            clean(&self.note),
        ]
        .join(",")
    }
}

/// Columns of a CSV header line, by name.
fn columns(header: &str) -> BTreeMap<String, usize> {
    header
        .trim_start_matches('\u{feff}')
        .split(',')
        .enumerate()
        .map(|(i, c)| (c.trim().to_string(), i))
        .collect()
}

struct Rec<'a> {
    cols: &'a BTreeMap<String, usize>,
    v: Vec<&'a str>,
}

impl Rec<'_> {
    fn s(&self, c: &str) -> &str {
        self.cols
            .get(c)
            .and_then(|&i| self.v.get(i))
            .map_or("", |x| x.trim())
    }
    fn f(&self, c: &str) -> f64 {
        let t = self.s(c).replace(['_', ' '], "");
        if t.is_empty() {
            f64::NAN
        } else {
            t.parse().unwrap_or(f64::NAN)
        }
    }
    fn i(&self, c: &str) -> Option<i64> {
        self.s(c).parse().ok()
    }
}

fn parse_row(cols: &BTreeMap<String, usize>, line: &str) -> Option<LogRow> {
    let r = Rec {
        cols,
        v: line.split(',').collect(),
    };
    let outcome = (r.s("resolved") == "1").then(|| Outcome {
        exit_time: r.i("exit_time_ms").unwrap_or(0),
        reason: reason_label(r.s("exit_reason")),
        exit_price: r.f("exit_price"),
        gross_r: r.f("gross_r"),
        net_r_1x: r.f("net_r_1x"),
        net_r_2x: r.f("net_r_2x"),
        hit: r.s("hit") == "1",
    });
    Some(LogRow {
        id: r.s("id").to_string(),
        logged_at: r.i("logged_ms")?,
        symbol: r.s("symbol").to_string(),
        tf: Timeframe::parse(r.s("tf"))?,
        rule: r.s("rule").parse().ok()?,
        label: r.s("label").to_string(),
        dir: r.s("dir").parse().ok()?,
        det: r.s("det").to_string(),
        kind: r.s("kind").to_string(),
        known_at: r.i("known_at_ms")?,
        entry_time: r.i("entry_time_ms")?,
        entry: r.f("entry"),
        stop: r.f("stop"),
        target: r.f("target"),
        trail_from: r.f("trail_from"),
        time_exit: r.i("time_exit_ms"),
        window_start: r.i("window_start_ms")?,
        status: r.s("status").to_string(),
        decision: r.s("decision").to_string(),
        hidden: r.s("hidden").to_string(),
        hit_kind: r.s("hit_kind").to_string(),
        hist_hit_rate: r.f("hist_hit_rate"),
        hist_n: r.s("hist_n").parse().unwrap_or(0),
        hist_exp_1x: r.f("hist_exp_1x"),
        account: r.f("account"),
        risk_pct: r.f("risk_pct"),
        size: r.f("size"),
        outcome,
        note: r.s("note").to_string(),
    })
}

/// `signals.csv` in memory. The overlay is the only writer.
#[derive(Debug, Clone, Default)]
pub struct Journal {
    pub dir: PathBuf,
    pub rows: Vec<LogRow>,
    /// Lines that could not be read (kept out of every count).
    pub bad_lines: usize,
    /// Write changes to disk (false for demo and screenshot runs: memory only).
    pub persist: bool,
}

impl Journal {
    /// An empty journal that never touches the disk (demo and screenshot runs).
    pub fn in_memory() -> Journal {
        Journal::default()
    }

    pub fn log_path(&self) -> PathBuf {
        self.dir.join("signals.csv")
    }

    /// Reads `signals.csv` in `dir` (missing file: empty journal).
    pub fn load(dir: &Path) -> Journal {
        let mut j = Journal {
            dir: dir.to_path_buf(),
            persist: true,
            ..Journal::default()
        };
        let Ok(text) = std::fs::read_to_string(j.log_path()) else {
            return j;
        };
        let mut lines = text.lines();
        let Some(h) = lines.next() else {
            return j;
        };
        let cols = columns(h);
        for l in lines.filter(|l| !l.trim().is_empty()) {
            match parse_row(&cols, l) {
                Some(r) => j.rows.push(r),
                None => j.bad_lines += 1,
            }
        }
        j
    }

    /// Writes the whole file (through a temporary file).
    ///
    /// # Errors
    /// The folder cannot be created or written.
    pub fn save(&self) -> std::io::Result<()> {
        if !self.persist {
            return Ok(());
        }
        std::fs::create_dir_all(&self.dir)?;
        let mut s = String::from(LOG_HEADER);
        s.push('\n');
        for r in &self.rows {
            s.push_str(&r.to_csv());
            s.push('\n');
        }
        let p = self.log_path();
        let tmp = p.with_extension("csv.tmp");
        std::fs::write(&tmp, s)?;
        std::fs::rename(tmp, p)
    }

    pub fn contains(&self, id: &str) -> bool {
        self.rows.iter().any(|r| r.id == id)
    }

    /// Adds rows (ids already present are skipped) and saves.
    ///
    /// # Errors
    /// The file cannot be written.
    pub fn append(&mut self, rows: Vec<LogRow>) -> std::io::Result<usize> {
        let mut n = 0;
        for r in rows {
            if !self.contains(&r.id) {
                self.rows.push(r);
                n += 1;
            }
        }
        if n > 0 {
            self.save()?;
        }
        Ok(n)
    }

    /// Sets outcomes by id and saves.
    ///
    /// # Errors
    /// The file cannot be written.
    pub fn set_outcomes(
        &mut self,
        out: &[(String, Result<Outcome, String>)],
    ) -> std::io::Result<()> {
        let mut changed = false;
        for (id, o) in out {
            if let Some(r) = self.rows.iter_mut().find(|r| &r.id == id) {
                match o {
                    Ok(o) => {
                        r.outcome = Some(*o);
                        r.note.clear();
                    }
                    Err(e) => r.note = clean(e),
                }
                changed = true;
            }
        }
        if changed {
            self.save()?;
        }
        Ok(())
    }

    /// Rows waiting for an outcome.
    pub fn pending(&self) -> Vec<&LogRow> {
        self.rows.iter().filter(|r| r.outcome.is_none()).collect()
    }
}

// ------------------------------------------------------------------------------------ resolver

/// Resolve one alert on `bars` (closed candles of its symbol and time frame from its
/// `window_start`), with optional 1-minute candles and funding: rebuild the market with
/// `Market::prepare`, find the alert's event, simulate it with the rule (1x and 2x costs) and
/// check that entry, stop and target equal the logged ones. `Ok(None)` while the trade is still
/// open at the end of the candles.
///
/// # Errors
/// The candles do not start at the logged window, the signal candle or event is missing, or the
/// rebuilt trade differs from the logged alert.
pub fn resolve(
    bars: &[Candle],
    m1: &[Candle],
    funding: &[Funding],
    row: &LogRow,
) -> Result<Option<Outcome>, String> {
    let def = rules::by_order(row.rule).ok_or("unknown rule")?;
    if bars.first().map(|c| c.open_time) != Some(row.window_start) {
        return Err("candles do not start at the logged window".into());
    }
    let mut m = Market::prepare(bars.to_vec(), row.tf.ms(), &DetectorSource::default_set())
        .map_err(|e| e.to_string())?;
    let k = m
        .bars
        .iter()
        .position(|c| c.open_time == row.known_at)
        .ok_or("signal candle not in the candles")?;
    let cands: Vec<_> = m
        .events
        .iter()
        .filter(|e| e.known_idx as usize == k && def.matches(e))
        .map(|e| def.trade_event(e))
        .filter(|e| e.det == row.det && e.kind == row.kind && e.dir == row.dir)
        .collect();
    if cands.is_empty() {
        return Err("event not found in the rebuilt candles".into());
    }
    let rule = def.trade_rule();
    let same = |a: f64, b: f64| (a.is_nan() && b.is_nan()) || price_eq(a, b);
    for e in cands {
        m.events = vec![e];
        let run = |m: &Market, c: CostModel| {
            simulate(
                &Inputs {
                    market: m,
                    m1,
                    funding,
                    costs: c,
                    sizing: Sizing::default(),
                },
                &rule,
            )
            .map_err(|e| e.to_string())
        };
        let r1 = run(&m, CostModel::BINANCE_UM)?;
        let Some(t) = r1.trades.first() else {
            continue;
        };
        if !(same(t.entry_raw, row.entry) && same(t.stop0, row.stop) && same(t.target, row.target))
        {
            continue;
        }
        if t.reason == ExitReason::EndOfData {
            return Ok(None);
        }
        let r2 = run(&m, CostModel::BINANCE_UM.with_mult(2.0))?;
        let net2 = r2.trades.first().map_or(f64::NAN, |x| x.net_r);
        let hit = match def.hit_kind() {
            HitKind::Target => t.reason == ExitReason::Target,
            HitKind::InProfit => t.net_r > 0.0,
        };
        return Ok(Some(Outcome {
            exit_time: t.exit_time,
            reason: t.reason.label(),
            exit_price: t.exit_raw,
            gross_r: t.gross_r,
            net_r_1x: t.net_r,
            net_r_2x: net2,
            hit,
        }));
    }
    Err("the rebuilt trade differs from the logged alert".into())
}

/// Funding settlements from the public REST endpoint (`/fapi/v1/fundingRate`).
///
/// # Errors
/// Network, exchange or parse failure, as text.
pub fn fetch_funding(
    client: &mut cs_data::http::Client,
    rest: &str,
    symbol: &str,
    from: i64,
    to: i64,
) -> Result<Vec<Funding>, String> {
    let mut out = Vec::new();
    let mut start = from;
    loop {
        let url = format!(
            "{rest}/fapi/v1/fundingRate?symbol={symbol}&startTime={start}&endTime={to}&limit=1000"
        );
        let v = client.get_json(&url).map_err(|e| e.to_string())?;
        let Some(a) = v.as_array() else {
            return Err(format!("exchange error: {v}"));
        };
        let n = a.len();
        let mut last = start;
        for x in a {
            let t = x["fundingTime"].as_i64().unwrap_or(0);
            let r = x["fundingRate"]
                .as_str()
                .and_then(|s| s.parse().ok())
                .unwrap_or(f64::NAN);
            last = last.max(t);
            if t >= from && t < to {
                out.push(Funding { time: t, rate: r });
            }
        }
        if n < 1000 || last <= start {
            break;
        }
        start = last + 1;
    }
    out.sort_by_key(|f| f.time);
    out.dedup_by_key(|f| f.time);
    Ok(out)
}

/// Resolve every pending alert over the public REST endpoints (klines of the alert's time frame
/// from its window start, 1-minute klines from its entry, funding). Returns (id, outcome or
/// problem) for the alerts whose trade ended; still-open trades are left out.
pub fn resolve_pending_rest(rows: &[LogRow], now: i64) -> Vec<(String, Result<Outcome, String>)> {
    let mut client = cs_data::http::Client::new(std::time::Duration::from_millis(250));
    client.max_retries = 1;
    let mut out = Vec::new();
    let mut groups: BTreeMap<(String, Timeframe, i64), Vec<&LogRow>> = BTreeMap::new();
    for r in rows.iter().filter(|r| r.outcome.is_none()) {
        // Nothing can have happened before the entry candle closed.
        if now < r.entry_time + r.tf.ms() {
            continue;
        }
        groups
            .entry((r.symbol.clone(), r.tf, r.window_start))
            .or_default()
            .push(r);
    }
    for ((sym, tf, start), rs) in groups {
        let cfg = crate::feed::FeedConfig::new(&sym, tf);
        let bars = match crate::feed::fetch_range(&mut client, &cfg, start, now, now) {
            Ok(b) => b,
            Err(e) => {
                for r in rs {
                    out.push((r.id.clone(), Err(format!("klines: {e}"))));
                }
                continue;
            }
        };
        let first_entry = rs.iter().map(|r| r.entry_time).min().unwrap_or(now);
        let m1cfg = crate::feed::FeedConfig::new(&sym, Timeframe::M1);
        let m1 = crate::feed::fetch_range(&mut client, &m1cfg, first_entry, now, now)
            .unwrap_or_default();
        let funding =
            fetch_funding(&mut client, &cfg.rest_base, &sym, first_entry, now).unwrap_or_default();
        for r in rs {
            match resolve(&bars, &m1, &funding, r) {
                Ok(Some(o)) => out.push((r.id.clone(), Ok(o))),
                Ok(None) => {}
                Err(e) => out.push((r.id.clone(), Err(e))),
            }
        }
    }
    out
}

// ------------------------------------------------------------------------- limits and account

/// Daily limits (editable in the panel's details).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Limits {
    /// Alerts shown per UTC day.
    pub max_per_day: u32,
    /// Pause after this many losing alerts in a row (resolved today).
    pub pause_after_losses: u32,
    /// Pause once today's resolved alerts lost this many R in total.
    pub daily_loss_r: f64,
}

impl Default for Limits {
    fn default() -> Self {
        Limits {
            max_per_day: 3,
            pause_after_losses: 3,
            daily_loss_r: 3.0,
        }
    }
}

/// Today's (UTC) figures of the shown alerts.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Today {
    pub shown: u32,
    pub resolved: u32,
    pub losses_in_row: u32,
    pub net_r: f64,
}

pub fn today(rows: &[LogRow], now: i64) -> Today {
    let d0 = day_start(now);
    let d1 = d0 + DAY_MS;
    let shown: Vec<&LogRow> = rows
        .iter()
        .filter(|r| r.decision == decision::SHOWN)
        .collect();
    let mut closed: Vec<(i64, f64)> = shown
        .iter()
        .filter_map(|r| r.outcome)
        .filter(|o| o.exit_time >= d0 && o.exit_time < d1)
        .map(|o| (o.exit_time, o.net_r_1x))
        .collect();
    closed.sort_by_key(|x| x.0);
    let losses_in_row = closed.iter().rev().take_while(|x| x.1 < 0.0).count();
    Today {
        shown: u32::try_from(
            shown
                .iter()
                .filter(|r| r.logged_at >= d0 && r.logged_at < d1)
                .count(),
        )
        .unwrap_or(u32::MAX),
        resolved: u32::try_from(closed.len()).unwrap_or(u32::MAX),
        losses_in_row: u32::try_from(losses_in_row).unwrap_or(u32::MAX),
        net_r: closed.iter().map(|x| x.1).sum(),
    }
}

/// What the limits say right now.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Hold {
    Open,
    /// The day's maximum of shown alerts is reached.
    Day(u32),
    /// Paused after this many losses in a row today.
    Losses(u32),
    /// Paused: today's resolved alerts lost this many R.
    LossR(f64),
}

impl Hold {
    pub fn decision(self) -> &'static str {
        match self {
            Hold::Open => decision::SHOWN,
            Hold::Day(_) => decision::LIMIT_DAY,
            Hold::Losses(_) => decision::LIMIT_LOSSES,
            Hold::LossR(_) => decision::LIMIT_LOSS_R,
        }
    }
}

pub fn hold(l: &Limits, t: &Today) -> Hold {
    if l.pause_after_losses > 0 && t.losses_in_row >= l.pause_after_losses {
        return Hold::Losses(t.losses_in_row);
    }
    if l.daily_loss_r > 0.0 && t.net_r <= -l.daily_loss_r {
        return Hold::LossR(-t.net_r);
    }
    if t.shown >= l.max_per_day {
        return Hold::Day(t.shown);
    }
    Hold::Open
}

/// The paper account: every shown alert, sized at its logged risk %, resolved by the backtest
/// rules, compounded in exit order from `start` (account size) since `since`.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Paper {
    pub start: f64,
    pub balance: f64,
    pub closed: u32,
    pub open: u32,
    pub net_r: f64,
}

pub fn paper(rows: &[LogRow], start: f64, since: i64) -> Paper {
    let mut closed: Vec<&LogRow> = rows
        .iter()
        .filter(|r| r.decision == decision::SHOWN && r.logged_at >= since)
        .collect();
    let open = closed.iter().filter(|r| r.outcome.is_none()).count();
    closed.retain(|r| r.outcome.is_some());
    closed.sort_by_key(|r| r.outcome.map_or(0, |o| o.exit_time));
    let mut bal = start;
    let mut net = 0.0;
    for r in &closed {
        let o = r.outcome.map_or(0.0, |o| o.net_r_1x);
        let dist = (r.entry - r.stop).abs();
        let rf = (r.risk_pct / 100.0).min(MAX_LEVERAGE * dist / r.entry);
        if rf.is_finite() && o.is_finite() {
            bal += bal * rf * o;
            net += o;
        }
    }
    Paper {
        start,
        balance: bal,
        closed: u32::try_from(closed.len()).unwrap_or(u32::MAX),
        open: u32::try_from(open).unwrap_or(u32::MAX),
        net_r: net,
    }
}

/// Calibration needs at least this many resolved fresh alerts.
pub const CALIBRATION_MIN: usize = 30;

/// One calibration row: a rule (or "all"), resolved alerts, the past hit rate shown at the time
/// (mean) and the rate that happened.
#[derive(Debug, Clone, PartialEq)]
pub struct CalRow {
    pub rule: Option<u8>,
    pub n: usize,
    pub expected: f64,
    pub actual: f64,
}

/// Calibration table over the resolved fresh alerts; empty below [`CALIBRATION_MIN`].
pub fn calibration(rows: &[LogRow]) -> (usize, Vec<CalRow>) {
    let res: Vec<&LogRow> = rows
        .iter()
        .filter(|r| r.status == "fresh" && r.outcome.is_some())
        .collect();
    if res.len() < CALIBRATION_MIN {
        return (res.len(), Vec::new());
    }
    let row = |rule: Option<u8>, xs: &[&LogRow]| {
        let n = xs.len();
        CalRow {
            rule,
            n,
            expected: xs.iter().map(|r| r.hist_hit_rate).sum::<f64>() / n as f64,
            actual: xs
                .iter()
                .filter(|r| r.outcome.is_some_and(|o| o.hit))
                .count() as f64
                / n as f64,
        }
    };
    let mut out = Vec::new();
    for def in &rules::RULES {
        let xs: Vec<&LogRow> = res
            .iter()
            .copied()
            .filter(|r| r.rule == def.order)
            .collect();
        if !xs.is_empty() {
            out.push(row(Some(def.order), &xs));
        }
    }
    out.push(row(None, &res));
    (res.len(), out)
}

// ------------------------------------------------------------------------------- demo fills

pub const FILLS_HEADER: &str = "alert_id,logged_utc,symbol,tf,rule,side,planned_entry,planned_stop,planned_target,entry,exit,size,fees";
pub const TRADES_HEADER: &str = "alert_id,logged_utc,symbol,tf,rule,side,planned_entry,planned_stop,planned_target,entry,exit,size,fees,net_usdt,risk_usdt,net_r,hit,hit_kind,hist_hit_rate,hist_exp_1x";

pub fn fills_path(d: &Path) -> PathBuf {
    d.join("fills.csv")
}

pub fn trades_path(d: &Path) -> PathBuf {
    d.join("demo-trades.csv")
}

fn side_word(d: i8) -> &'static str {
    if d > 0 { "buy" } else { "sell" }
}

/// Adds a pre-filled row to `fills.csv` for every shown alert that has none yet (the four fill
/// columns stay empty for the user). Returns the number added.
///
/// # Errors
/// The file cannot be read or written.
pub fn sync_fill_sheet(j: &Journal) -> std::io::Result<usize> {
    let p = fills_path(&j.dir);
    let text = std::fs::read_to_string(&p).unwrap_or_default();
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    if lines.is_empty() {
        lines.push(FILLS_HEADER.to_string());
    }
    let cols = columns(&lines[0]);
    let id_col = cols.get("alert_id").copied().unwrap_or(0);
    let have: Vec<String> = lines
        .iter()
        .skip(1)
        .filter_map(|l| l.split(',').nth(id_col).map(|s| s.trim().to_string()))
        .collect();
    let mut n = 0;
    for r in j.rows.iter().filter(|r| r.decision == decision::SHOWN) {
        if have.contains(&r.id) {
            continue;
        }
        lines.push(format!(
            "{},{},{},{},{},{},{},{},{},,,,",
            r.id,
            fmt_minute(r.logged_at),
            r.symbol,
            r.tf.label(),
            r.rule,
            side_word(r.dir),
            num(r.entry),
            num(r.stop),
            num(r.target)
        ));
        n += 1;
    }
    if n > 0 {
        std::fs::create_dir_all(&j.dir)?;
        let mut s = lines.join("\n");
        s.push('\n');
        std::fs::write(&p, s)?;
    }
    Ok(n)
}

/// One imported demo trade.
#[derive(Debug, Clone, PartialEq)]
pub struct DemoTrade {
    pub alert_id: String,
    pub logged_at: i64,
    pub rule: u8,
    pub dir: i8,
    pub net_usdt: f64,
    pub risk_usdt: f64,
    pub net_r: f64,
    pub hit: bool,
    pub hist_hit_rate: f64,
    pub hist_exp_1x: f64,
    pub line: String,
}

/// Result of an import.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Import {
    pub trades: Vec<DemoTrade>,
    /// Rows still waiting for their four numbers.
    pub empty: usize,
    /// Rows that could not be used, with the reason.
    pub problems: Vec<String>,
}

/// Reads `fills.csv`, keeps the rows whose four fill numbers are filled in and belong to a
/// logged alert, and computes each trade's result in R: net = side x (exit - entry) x size -
/// fees; 1 R = |planned entry - planned stop| x size (the alert's own risk, so a worse fill
/// shows as a worse result).
pub fn import_fills(j: &Journal, text: &str) -> Import {
    let mut imp = Import::default();
    let mut lines = text.lines();
    let Some(h) = lines.next() else {
        return imp;
    };
    let cols = columns(h);
    for (i, l) in lines.enumerate() {
        if l.trim().is_empty() {
            continue;
        }
        let r = Rec {
            cols: &cols,
            v: l.split(',').collect(),
        };
        let line_no = i + 2;
        let id = r.s("alert_id");
        let vals = [r.f("entry"), r.f("exit"), r.f("size"), r.f("fees")];
        let filled = vals.iter().filter(|x| x.is_finite()).count();
        if filled == 0 {
            imp.empty += 1;
            continue;
        }
        if filled < 4 {
            imp.problems
                .push(format!("row {line_no}: fill in all four numbers or none"));
            continue;
        }
        let [entry, exit, size, fees] = vals;
        if !(entry > 0.0 && exit > 0.0 && size > 0.0 && fees >= 0.0) {
            imp.problems.push(format!(
                "row {line_no}: prices and size must be positive, fees zero or more"
            ));
            continue;
        }
        if imp.trades.iter().any(|t| t.alert_id == id) {
            imp.problems
                .push(format!("row {line_no}: this alert already has a fill"));
            continue;
        }
        let Some(a) = j.rows.iter().find(|x| x.id == id) else {
            imp.problems
                .push(format!("row {line_no}: no logged alert with this id"));
            continue;
        };
        let risk_unit = (a.entry - a.stop).abs();
        let sd = f64::from(a.dir);
        let net = sd * (exit - entry) * size - fees;
        let risk = risk_unit * size;
        let net_r = net / risk;
        let hit = match a.hit_kind.as_str() {
            "target" => a.target.is_finite() && sd * (exit - a.target) >= -1e-9 * a.target,
            _ => net > 0.0,
        };
        imp.trades.push(DemoTrade {
            alert_id: a.id.clone(),
            logged_at: a.logged_at,
            rule: a.rule,
            dir: a.dir,
            net_usdt: net,
            risk_usdt: risk,
            net_r,
            hit,
            hist_hit_rate: a.hist_hit_rate,
            hist_exp_1x: a.hist_exp_1x,
            line: format!(
                "{},{},{},{},{},{},{},{},{},{entry},{exit},{size},{fees},{},{},{},{},{},{},{}",
                a.id,
                fmt_minute(a.logged_at),
                a.symbol,
                a.tf.label(),
                a.rule,
                side_word(a.dir),
                num(a.entry),
                num(a.stop),
                num(a.target),
                num(net),
                num(risk),
                num(net_r),
                u8::from(hit),
                a.hit_kind,
                num(a.hist_hit_rate),
                num(a.hist_exp_1x)
            ),
        });
    }
    imp.trades.sort_by_key(|t| t.logged_at);
    imp
}

/// Writes `demo-trades.csv` from an import (all complete rows, replacing the file).
///
/// # Errors
/// The file cannot be written.
pub fn write_trades(d: &Path, imp: &Import) -> std::io::Result<()> {
    std::fs::create_dir_all(d)?;
    let mut s = String::from(TRADES_HEADER);
    s.push('\n');
    for t in &imp.trades {
        s.push_str(&t.line);
        s.push('\n');
    }
    std::fs::write(trades_path(d), s)
}

/// Reads `demo-trades.csv` back.
pub fn read_trades(d: &Path) -> Vec<DemoTrade> {
    let Ok(text) = std::fs::read_to_string(trades_path(d)) else {
        return Vec::new();
    };
    let mut lines = text.lines();
    let Some(h) = lines.next() else {
        return Vec::new();
    };
    let cols = columns(h);
    let mut out = Vec::new();
    for l in lines.filter(|l| !l.trim().is_empty()) {
        let r = Rec {
            cols: &cols,
            v: l.split(',').collect(),
        };
        let Some(rule) = r.s("rule").parse().ok() else {
            continue;
        };
        out.push(DemoTrade {
            alert_id: r.s("alert_id").to_string(),
            logged_at: cs_core::time::parse_datetime(r.s("logged_utc")).unwrap_or(0),
            rule,
            dir: if r.s("side") == "sell" { -1 } else { 1 },
            net_usdt: r.f("net_usdt"),
            risk_usdt: r.f("risk_usdt"),
            net_r: r.f("net_r"),
            hit: r.s("hit") == "1",
            hist_hit_rate: r.f("hist_hit_rate"),
            hist_exp_1x: r.f("hist_exp_1x"),
            line: l.to_string(),
        });
    }
    out
}

// ------------------------------------------------------------------------------------ report

/// Trades needed before the report gives a verdict, and the checkpoint before that.
pub const VERDICT_N: usize = 100;
pub const CHECKPOINT_N: usize = 50;

/// Summary of a series of results in R (in time order).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Summary {
    pub n: usize,
    pub net_r: f64,
    pub win_rate: f64,
    pub expectancy: f64,
    pub ci80: (f64, f64),
    pub max_dd: f64,
}

pub fn summarize(xs: &[f64]) -> Summary {
    let n = xs.len();
    if n == 0 {
        return Summary {
            n,
            net_r: 0.0,
            win_rate: f64::NAN,
            expectancy: f64::NAN,
            ci80: (f64::NAN, f64::NAN),
            max_dd: 0.0,
        };
    }
    let net: f64 = xs.iter().sum();
    let (mut cum, mut peak, mut dd) = (0.0f64, 0.0f64, 0.0f64);
    for x in xs {
        cum += x;
        peak = peak.max(cum);
        dd = dd.max(peak - cum);
    }
    let ci = block_mean_ci(xs, 0, 2000, 0.8, 1).map_or((f64::NAN, f64::NAN), |c| (c.lo, c.hi));
    Summary {
        n,
        net_r: net,
        win_rate: xs.iter().filter(|x| **x > 0.0).count() as f64 / n as f64,
        expectancy: net / n as f64,
        ci80: ci,
        max_dd: dd,
    }
}

/// The verdict rule: PASS only with at least 100 trades, positive net R and an 80 % interval
/// whose lower bound is above zero.
pub fn verdict(s: &Summary) -> String {
    if s.n < CHECKPOINT_N {
        format!("NOT ENOUGH TRADES ({} of {VERDICT_N}): not a verdict.", s.n)
    } else if s.n < VERDICT_N {
        format!("checkpoint, not a verdict ({} of {VERDICT_N} trades).", s.n)
    } else if s.net_r > 0.0 && s.ci80.0 > 0.0 {
        "PASS: at least 100 trades, net R positive and the 80 % interval's lower bound above zero."
            .to_string()
    } else {
        let mut why = Vec::new();
        if s.net_r <= 0.0 {
            why.push("net R is not positive");
        }
        if s.ci80.0.is_nan() || s.ci80.0 <= 0.0 {
            why.push("the 80 % interval's lower bound is not above zero");
        }
        format!("FAIL: {}.", why.join(" and "))
    }
}

fn pct(x: f64) -> String {
    if x.is_finite() {
        format!("{:.1} %", x * 100.0)
    } else {
        "-".into()
    }
}

fn r2(x: f64) -> String {
    if x.is_finite() {
        format!("{x:+.3}")
    } else {
        "-".into()
    }
}

/// The text `candlescope journal-report` prints for the demo trades in `d` (plus the tool's own
/// resolved alerts, for information).
pub fn report(d: &Path) -> String {
    let trades = read_trades(d);
    let j = Journal::load(d);
    let mut s = String::new();
    let _ = writeln!(
        s,
        "Candlescope journal report ({}). The rules shown failed testing (0 of 59 setups survived); these are your own demo results.",
        d.display()
    );
    let _ = writeln!(s);
    let _ = writeln!(
        s,
        "DEMO TRADES (imported fills, {}):",
        trades_path(d).display()
    );
    let xs: Vec<f64> = trades.iter().map(|t| t.net_r).collect();
    let sm = summarize(&xs);
    let _ = writeln!(s, "  count                 {}", sm.n);
    let _ = writeln!(s, "  net R after costs     {}", r2(sm.net_r));
    let _ = writeln!(s, "  win rate (net > 0)    {}", pct(sm.win_rate));
    let _ = writeln!(
        s,
        "  expectancy            {} R per trade, 80 % block-bootstrap interval {} .. {}",
        r2(sm.expectancy),
        r2(sm.ci80.0),
        r2(sm.ci80.1)
    );
    let _ = writeln!(s, "  max drawdown          {:.3} R", sm.max_dd);
    let _ = writeln!(s, "  predicted (past results) vs actual, per rule:");
    let _ = writeln!(
        s,
        "    rule                                            n   hit rate past / actual   exp R past / actual"
    );
    for def in &rules::RULES {
        let ts: Vec<&DemoTrade> = trades.iter().filter(|t| t.rule == def.order).collect();
        if ts.is_empty() {
            continue;
        }
        let n = ts.len() as f64;
        let _ = writeln!(
            s,
            "    {:2} {:44} {:3}   {:>8} / {:<8}        {} / {}",
            def.order,
            def.label,
            ts.len(),
            pct(ts.iter().map(|t| t.hist_hit_rate).sum::<f64>() / n),
            pct(ts.iter().filter(|t| t.hit).count() as f64 / n),
            r2(ts.iter().map(|t| t.hist_exp_1x).sum::<f64>() / n),
            r2(ts.iter().map(|t| t.net_r).sum::<f64>() / n)
        );
    }
    let _ = writeln!(s, "  VERDICT: {}", verdict(&sm));
    let _ = writeln!(s);
    let res: Vec<&LogRow> = j
        .rows
        .iter()
        .filter(|r| r.decision == decision::SHOWN && r.outcome.is_some())
        .collect();
    let mut ord = res.clone();
    ord.sort_by_key(|r| r.outcome.map_or(0, |o| o.exit_time));
    let ps: Vec<f64> = ord
        .iter()
        .map(|r| r.outcome.map_or(f64::NAN, |o| o.net_r_1x))
        .collect();
    let p = summarize(&ps);
    let _ = writeln!(
        s,
        "PAPER (the tool's own shown alerts, resolved by the backtest rules; information only): {} alerts logged, {} shown and resolved, net {} R, expectancy {} R (80 % interval {} .. {}).",
        j.rows.len(),
        p.n,
        r2(p.net_r),
        r2(p.expectancy),
        r2(p.ci80.0),
        r2(p.ci80.1)
    );
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    pub fn row(id: &str, logged: i64, dir: i8, net: Option<f64>) -> LogRow {
        LogRow {
            id: id.into(),
            logged_at: logged,
            symbol: "BTCUSDT".into(),
            tf: Timeframe::H1,
            rule: 1,
            label: rules::RULES[0].label.into(),
            dir,
            det: "fib".into(),
            kind: "fib_zone_touch".into(),
            known_at: logged - 3_600_000,
            entry_time: logged,
            entry: 90_000.0,
            stop: 89_100.0,
            target: f64::NAN,
            trail_from: 90_900.0,
            time_exit: Some(logged + 96 * 3_600_000),
            window_start: 0,
            status: "fresh".into(),
            decision: decision::SHOWN.into(),
            hidden: String::new(),
            hit_kind: "in_profit".into(),
            hist_hit_rate: 0.4,
            hist_n: 233,
            hist_exp_1x: -0.05,
            account: 10_000.0,
            risk_pct: 1.0,
            size: 0.111_111,
            outcome: net.map(|x| Outcome {
                exit_time: logged + 7_200_000,
                reason: "stop",
                exit_price: 89_100.0,
                gross_r: x + 0.1,
                net_r_1x: x,
                net_r_2x: x - 0.1,
                hit: x > 0.0,
            }),
            note: String::new(),
        }
    }

    fn tmp(tag: &str) -> PathBuf {
        let d = std::env::temp_dir().join(format!("cs-journal-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&d);
        d
    }

    #[test]
    fn log_round_trip() {
        let d = tmp("rt");
        let mut j = Journal::load(&d);
        assert_eq!(j.rows.len(), 0);
        let a = row("a", 1_700_000_000_000, 1, Some(-1.07));
        let b = row("b", 1_700_000_360_000, -1, None);
        assert_eq!(j.append(vec![a.clone(), b.clone(), a.clone()]).unwrap(), 2);
        let back = Journal::load(&d);
        assert_eq!(back.bad_lines, 0);
        assert_eq!(back.rows.len(), 2);
        assert_eq!(back.rows[0].to_csv(), a.to_csv());
        assert_eq!(back.rows[1].to_csv(), b.to_csv());
        assert!(back.rows[0].target.is_nan());
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn limits_and_paper() {
        let day = 1_700_006_400_000 - 1_700_006_400_000 % DAY_MS;
        let h = 3_600_000;
        let rows = vec![
            row("1", day + h, 1, Some(-1.1)),
            row("2", day + 4 * h, 1, Some(-1.2)),
            row("3", day + 8 * h, -1, Some(-0.9)),
        ];
        let t = today(&rows, day + 12 * h);
        assert_eq!(t.shown, 3);
        assert_eq!(t.losses_in_row, 3);
        assert!((t.net_r + 3.2).abs() < 1e-12);
        let l = Limits::default();
        assert_eq!(hold(&l, &t), Hold::Losses(3));
        let l2 = Limits {
            pause_after_losses: 5,
            ..l
        };
        assert!(matches!(hold(&l2, &t), Hold::LossR(x) if (x - 3.2).abs() < 1e-12));
        let l3 = Limits {
            pause_after_losses: 5,
            daily_loss_r: 10.0,
            max_per_day: 3,
        };
        assert_eq!(hold(&l3, &t), Hold::Day(3));
        // Next day: all clear.
        assert_eq!(hold(&l, &today(&rows, day + DAY_MS + h)), Hold::Open);
        // Paper: 1 % risk, compounded: 10,000 x (1 - 0.011) x (1 - 0.012) x (1 - 0.009).
        let p = paper(&rows, 10_000.0, 0);
        let want = 10_000.0 * (1.0 - 0.011) * (1.0 - 0.012) * (1.0 - 0.009);
        assert!((p.balance - want).abs() < 1e-6, "{} vs {want}", p.balance);
        assert_eq!(p.closed, 3);
        // A win breaks the streak.
        let mut r2 = rows.clone();
        r2.push(row("4", day + 9 * h, 1, Some(2.0)));
        assert_eq!(today(&r2, day + 12 * h).losses_in_row, 0);
    }

    #[test]
    fn calibration_needs_thirty() {
        let mut rows: Vec<LogRow> = (0..29)
            .map(|i| {
                row(
                    &i.to_string(),
                    1_000_000 + i,
                    1,
                    Some(if i % 2 == 0 { 1.0 } else { -1.0 }),
                )
            })
            .collect();
        assert_eq!(calibration(&rows).1.len(), 0);
        rows.push(row("x", 2_000_000, 1, Some(1.0)));
        let (n, t) = calibration(&rows);
        assert_eq!(n, 30);
        let all = t.last().unwrap();
        assert_eq!(all.n, 30);
        assert!((all.expected - 0.4).abs() < 1e-12);
        assert!((all.actual - 16.0 / 30.0).abs() < 1e-12);
    }

    #[test]
    fn fills_import_and_verdicts() {
        let d = tmp("fills");
        let mut j = Journal::load(&d);
        j.append(vec![
            row("a1", 1_700_000_000_000, 1, None),
            row("a2", 1_700_003_600_000, -1, None),
        ])
        .unwrap();
        assert_eq!(sync_fill_sheet(&j).unwrap(), 2);
        assert_eq!(sync_fill_sheet(&j).unwrap(), 0, "never duplicated");
        let sheet = std::fs::read_to_string(fills_path(&d)).unwrap();
        assert!(sheet.starts_with(FILLS_HEADER));
        // The user fills in the four numbers of a1 only (a buy: 90,010 -> 90,910, 0.1 BTC, 9 USDT).
        let filled = sheet.replace(
            "a1,2023-11-14 22:13,BTCUSDT,1h,1,buy,90000,89100,,,,,",
            "a1,2023-11-14 22:13,BTCUSDT,1h,1,buy,90000,89100,,90010,90910,0.1,9",
        );
        assert_ne!(filled, sheet, "{sheet}");
        let imp = import_fills(&j, &filled);
        assert_eq!(imp.trades.len(), 1);
        assert_eq!(imp.empty, 1);
        assert_eq!(imp.problems.len(), 0, "{:?}", imp.problems);
        // net = 900 x 0.1 - 9 = 81 USDT; 1 R = 900 x 0.1 = 90 -> 0.9 R.
        assert!((imp.trades[0].net_r - 0.9).abs() < 1e-12);
        write_trades(&d, &imp).unwrap();
        assert_eq!(read_trades(&d).len(), 1);
        // The same alert twice is refused (never counted twice).
        let twice = format!(
            "{filled}a1,2023-11-14 22:13,BTCUSDT,1h,1,buy,90000,89100,,90010,90910,0.1,9\n"
        );
        let imp2 = import_fills(&j, &twice);
        assert_eq!(imp2.trades.len(), 1);
        assert_eq!(imp2.problems, ["row 4: this alert already has a fill"]);
        // Partly filled rows are refused with a reason.
        let bad = filled.replace(",90010,90910,0.1,9", ",90010,,0.1,9");
        assert_eq!(import_fills(&j, &bad).problems.len(), 1);
        let rep = report(&d);
        assert!(rep.contains("NOT ENOUGH TRADES (1 of 100)"), "{rep}");
        let _ = std::fs::remove_dir_all(&d);
    }

    #[test]
    fn verdict_rule() {
        let s = |n: usize, mean: f64| {
            let xs: Vec<f64> = (0..n)
                .map(|i| mean + if i % 2 == 0 { 0.5 } else { -0.5 })
                .collect();
            summarize(&xs)
        };
        assert!(verdict(&s(49, 0.5)).starts_with("NOT ENOUGH"));
        assert!(verdict(&s(50, 0.5)).starts_with("checkpoint, not a verdict"));
        assert!(verdict(&s(99, 0.5)).starts_with("checkpoint, not a verdict"));
        assert!(verdict(&s(100, 0.3)).starts_with("PASS"));
        assert!(verdict(&s(120, -0.1)).starts_with("FAIL"));
        // Positive mean but an interval reaching below zero: FAIL.
        let noisy: Vec<f64> = (0..100)
            .map(|i| if i % 2 == 0 { 3.0 } else { -2.9 })
            .collect();
        let sm = summarize(&noisy);
        assert!(sm.net_r > 0.0 && sm.ci80.0 <= 0.0, "{sm:?}");
        assert!(verdict(&sm).starts_with("FAIL"));
        assert!((s(10, 0.0).max_dd - 0.5).abs() < 1e-12);
    }
}
