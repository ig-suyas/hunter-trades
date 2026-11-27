//! Trade rules: which events to trade and how (entry, stop, target, exits, filters, overlap).
//!
//! Every decision a rule makes at a signal uses only what is known at the close of the signal's
//! `known_idx` candle: the event itself and the context features of that candle
//! ([`cs_patterns::context::Features`], which come from candles `<= known_idx` only, including the
//! 1h/4h state built from completed higher-time-frame candles). See [`Filter::passes`].

use std::fmt;

use cs_core::Event;
use cs_patterns::context::Features;
use cs_patterns::pullback::HEALTHY;

/// Which events a rule trades.
#[derive(Debug, Clone, PartialEq)]
pub struct Select {
    /// Detector name (`Event::det`); `None` = any detector.
    pub det: Option<String>,
    /// Event kinds (`Event::kind`); empty = any kind.
    pub kinds: Vec<String>,
    pub dir: DirSel,
}

impl Select {
    pub fn det(name: &str) -> Select {
        Select {
            det: Some(name.to_string()),
            kinds: Vec::new(),
            dir: DirSel::Both,
        }
    }

    /// Directional events only (`dir != 0`), matching detector, kind and direction.
    pub fn matches(&self, e: &Event) -> bool {
        e.dir != 0
            && self.det.as_deref().is_none_or(|d| d == e.det)
            && (self.kinds.is_empty() || self.kinds.iter().any(|k| k == e.kind))
            && match self.dir {
                DirSel::Both => true,
                DirSel::Long => e.dir > 0,
                DirSel::Short => e.dir < 0,
            }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DirSel {
    Both,
    Long,
    Short,
}

/// Entry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Entry {
    /// Market order at the open of the candle after the `known_idx` candle (that candle must
    /// start exactly when the `known_idx` candle closes; across a data gap the signal is skipped).
    NextOpen,
    /// Limit order placed when the `known_idx` candle closes, active from the next candle for
    /// `timeout_bars` candles. Long: buy limit at the zone, filled only when price trades
    /// strictly below the limit (at the limit price, never better). If the limit is already
    /// marketable at the first open, it fills at that open as a market order.
    Limit { at: LimitAt, timeout_bars: u32 },
}

/// Where a limit order sits.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LimitAt {
    /// Near edge of the event zone: `hi` for longs, `lo` for shorts.
    ZoneNear,
    /// Middle of the event zone.
    ZoneMid,
    /// The event's key price.
    Price,
}

/// Initial stop.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum StopRule {
    /// Structural invalidation plus a buffer: long stop = `lo - buffer_atr * ATR`, short stop =
    /// `hi + buffer_atr * ATR`, where `lo`/`hi` is the event's zone or pattern extent, or the
    /// `known_idx` candle's low/high when the event has none. ATR = ATR(14) at `known_idx`.
    Structural { buffer_atr: f64 },
    /// `mult * ATR` from the entry price (used by the random baseline).
    Atr { mult: f64 },
}

/// Profit target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Target {
    /// Fixed multiple of the initial risk (entry to stop) from the entry price.
    R(f64),
    /// The nearest support/resistance or supply/demand zone edge beyond the entry, from the zones
    /// known at `known_idx` (near edge: a long targets the zone's lower edge), that is at least
    /// `min_r` R away. No such level: the signal is skipped.
    NextLevel { min_r: f64 },
    /// No target (exit by stop, trailing stop, time stop or end of data).
    None,
}

/// Trailing stop: once the best close since entry is `activate_r` R in profit, the stop trails
/// at best close -/+ `atr_mult` x ATR(14). Updated at each candle close with that candle's data
/// and used from the next candle on; it never loosens.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Trail {
    pub activate_r: f64,
    pub atr_mult: f64,
}

/// What to do with a new signal while an earlier one of the same rule is still active.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Overlap {
    /// At most one open trade or pending order per rule (default).
    OnePerSetup,
    /// At most one per rule and direction.
    OnePerDirection,
    /// Every signal is traded, positions may overlap.
    Unlimited,
}

/// Trend alignment of a context state with the trade direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Align {
    /// State equals the trade direction (+1 for longs, -1 for shorts).
    With,
    /// State is the opposite direction.
    Against,
    /// State is 0 (no trend / no stack).
    Neutral,
}

impl Align {
    fn check(self, state: i8, dir: i8) -> bool {
        match self {
            Align::With => state != 0 && state == dir,
            Align::Against => state != 0 && state == -dir,
            Align::Neutral => state == 0,
        }
    }
}

/// Context filter. Each one reads only the event and the features of the `known_idx` candle.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Filter {
    /// Higher-time-frame EMA 20/50/200 stack (slot 0 = 1h, 1 = 4h by default). Requires the HTF
    /// EMA state to be ready.
    HtfStack { slot: usize, align: Align },
    /// Higher-time-frame swing-structure trend.
    HtfStructure { slot: usize, align: Align },
    /// Healthy-pullback flag (`pullback` events, `tags` bit [`HEALTHY`]): `true` keeps only
    /// healthy pullbacks, `false` only unhealthy ones.
    Healthy(bool),
    /// ADX regime of the setup time frame relative to the trade direction.
    Regime(Align),
    /// Volatility percentile (ATR rank) in `[min, max]`.
    VolPct { min: f64, max: f64 },
    /// UTC sessions as a bit mask over [`cs_patterns::context::SESSIONS`] (bit 0 = asia ...).
    Sessions(u8),
    /// UTC hours of the `known_idx` candle's open as a bit mask (bit h = hour h).
    Hours(u32),
}

impl Filter {
    /// Whether a signal passes. `f` must be the features of the event's `known_idx` candle.
    pub fn passes(&self, e: &Event, f: &Features) -> bool {
        let d = e.dir;
        match *self {
            Filter::HtfStack { slot, align } => f
                .htf
                .get(slot)
                .is_some_and(|h| h.n > 0 && h.ema.ready && align.check(h.ema.stack, d)),
            Filter::HtfStructure { slot, align } => f
                .htf
                .get(slot)
                .is_some_and(|h| h.n > 0 && align.check(h.structure, d)),
            Filter::Healthy(want) => (e.det == "pullback") && ((e.tags & HEALTHY != 0) == want),
            Filter::Regime(align) => align.check(f.regime, d),
            Filter::VolPct { min, max } => {
                f.vol_pct.is_finite() && f.vol_pct >= min && f.vol_pct <= max
            }
            Filter::Sessions(mask) => mask & (1u8 << f.session.min(7)) != 0,
            Filter::Hours(mask) => mask & (1u32 << u32::from(f.hour.min(31))) != 0,
        }
    }
}

/// A complete trade rule.
#[derive(Debug, Clone, PartialEq)]
pub struct TradeRule {
    /// Short unique name (used in file names).
    pub name: String,
    pub select: Select,
    pub entry: Entry,
    pub stop: StopRule,
    pub target: Target,
    /// Exit at the open of the candle `n` candles after the entry candle.
    pub time_stop_bars: Option<u32>,
    pub trail: Option<Trail>,
    pub filters: Vec<Filter>,
    pub overlap: Overlap,
    /// Skip the signal if the initial risk (entry to stop) is below this many ATR (costs would
    /// dominate). Default 0.2.
    pub min_stop_atr: f64,
}

impl TradeRule {
    /// Next-open entry, structural stop with a 0.25 ATR buffer, fixed R target, one trade at a
    /// time, no filters.
    pub fn basic(name: &str, select: Select, target: Target) -> TradeRule {
        TradeRule {
            name: name.to_string(),
            select,
            entry: Entry::NextOpen,
            stop: StopRule::Structural { buffer_atr: 0.25 },
            target,
            time_stop_bars: None,
            trail: None,
            filters: Vec::new(),
            overlap: Overlap::OnePerSetup,
            min_stop_atr: 0.2,
        }
    }

    /// All filters pass.
    pub fn filters_pass(&self, e: &Event, f: &Features) -> bool {
        self.filters.iter().all(|x| x.passes(e, f))
    }
}

impl fmt::Display for TradeRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}
