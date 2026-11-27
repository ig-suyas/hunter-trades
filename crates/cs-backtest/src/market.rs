//! The prepared market: candles of the setup time frame plus everything a rule may look at when
//! it decides on a signal (context features, support/resistance levels, the signal events). All
//! of it is a function of candles `<= k` at candle `k`; [`audit_prefix`] checks this by
//! rebuilding the market on truncated candles and comparing (the engine's lookahead guard).

use cs_core::{Candle, Event};
use cs_patterns::context::{Context, ContextParams, Features};
use cs_patterns::zones::{SdParams, SdZones, SrParams, SrZones};
use cs_patterns::{Detector, Runner};
use cs_stats::Rng;

use crate::Error;

/// Source of signal events for a candle series.
///
/// Contract: the events with `known_idx <= k` may depend on `bars[..=k]` only, `known_idx` must
/// be a valid index and `known_at == bars[known_idx].open_time`. [`Market::prepare`] checks the
/// indices and times; [`audit_prefix`] checks the dependence on the future.
pub trait SignalSource {
    fn name(&self) -> String;
    fn events(&self, bars: &[Candle], tf_ms: i64) -> Vec<Event>;
}

/// The Phase 2 detectors (by default the full tier-1 set) as a signal source.
pub struct DetectorSource {
    pub make: fn(i64) -> Vec<Box<dyn Detector>>,
}

impl DetectorSource {
    /// The full default detector set ([`cs_patterns::default_set`]).
    pub fn default_set() -> DetectorSource {
        DetectorSource {
            make: cs_patterns::default_set,
        }
    }
}

impl SignalSource for DetectorSource {
    fn name(&self) -> String {
        "detectors".to_string()
    }

    fn events(&self, bars: &[Candle], tf_ms: i64) -> Vec<Event> {
        let mut r = Runner::new((self.make)(tf_ms), None);
        let mut out = Vec::with_capacity(bars.len() * 2 + 16);
        for c in bars {
            r.step(c, &mut out);
        }
        out
    }
}

/// Random entries (the baseline): at each candle, with probability `prob`, a signal in a random
/// direction. Two random draws per candle whatever happens, so the signals up to `k` depend only
/// on the seed and `k` (prefix invariant). Detector name `random`, kinds `random_long` /
/// `random_short`, `price` = close, no zone (use [`crate::rule::StopRule::Atr`]).
pub struct RandomSource {
    pub seed: u64,
    pub prob: f64,
}

impl SignalSource for RandomSource {
    fn name(&self) -> String {
        format!("random(seed={}, p={})", self.seed, self.prob)
    }

    fn events(&self, bars: &[Candle], _tf_ms: i64) -> Vec<Event> {
        let mut rng = Rng::new(self.seed);
        let mut out = Vec::new();
        for (k, c) in bars.iter().enumerate() {
            let take = rng.f64() < self.prob;
            let long = rng.f64() < 0.5;
            if take {
                let (kind, dir) = if long {
                    ("random_long", 1)
                } else {
                    ("random_short", -1)
                };
                let mut e = Event::new("random", kind, dir, k as u32).with_price(c.close);
                e.known_at = c.open_time;
                e.formed_at = c.open_time;
                out.push(e);
            }
        }
        out
    }
}

/// Hand-made events (tests). Events whose `known_idx` lies beyond the candles are dropped, so a
/// truncated run sees exactly the events known by then.
pub struct FixedSource(pub Vec<Event>);

impl SignalSource for FixedSource {
    fn name(&self) -> String {
        "fixed".to_string()
    }

    fn events(&self, bars: &[Candle], _tf_ms: i64) -> Vec<Event> {
        self.0
            .iter()
            .filter(|e| (e.known_idx as usize) < bars.len())
            .copied()
            .collect()
    }
}

/// Number of support/resistance levels kept per side and candle.
pub const NL: usize = 4;

/// Zone edges known at a candle's close: the nearest `NL` lower edges of zones entirely above
/// the close (ascending) and upper edges of zones entirely below it (descending), `NaN`-padded.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Levels {
    pub above: [f64; NL],
    pub below: [f64; NL],
}

impl Levels {
    fn same(&self, o: &Levels) -> bool {
        let f =
            |a: &[f64; NL], b: &[f64; NL]| a.iter().zip(b).all(|(x, y)| x.to_bits() == y.to_bits());
        f(&self.above, &o.above) && f(&self.below, &o.below)
    }
}

/// Streaming S/R + supply/demand zones (same detectors and defaults as the context features).
struct LevelTracker {
    sr: SrZones,
    sd: SdZones,
    scratch: Vec<Event>,
    above: Vec<f64>,
    below: Vec<f64>,
}

impl LevelTracker {
    fn new() -> LevelTracker {
        LevelTracker {
            sr: SrZones::new(SrParams::default()),
            sd: SdZones::new(SdParams::default()),
            scratch: Vec::with_capacity(32),
            above: Vec::with_capacity(96),
            below: Vec::with_capacity(96),
        }
    }

    fn step(&mut self, i: u32, c: &Candle) -> Levels {
        self.scratch.clear();
        self.sr.step(i, c, &mut self.scratch);
        self.sd.step(i, c, &mut self.scratch);
        self.above.clear();
        self.below.clear();
        let close = c.close;
        let mut add = |lo: f64, hi: f64| {
            if lo > close {
                self.above.push(lo);
            } else if hi < close {
                self.below.push(hi);
            }
        };
        for z in self.sr.zones() {
            add(z.lo, z.hi);
        }
        for z in self.sd.zones() {
            add(z.lo, z.hi);
        }
        self.above.sort_by(f64::total_cmp);
        self.below.sort_by(|a, b| b.total_cmp(a));
        let mut l = Levels {
            above: [f64::NAN; NL],
            below: [f64::NAN; NL],
        };
        for (d, s) in l.above.iter_mut().zip(&self.above) {
            *d = *s;
        }
        for (d, s) in l.below.iter_mut().zip(&self.below) {
            *d = *s;
        }
        l
    }
}

/// Candles of the setup time frame with their decision-time data.
#[derive(Debug, Clone)]
pub struct Market {
    pub tf_ms: i64,
    pub bars: Vec<Candle>,
    /// Context features of each candle (ATR, ADX regime, volatility percentile, session, 1h/4h
    /// state), from candles `<= i`.
    pub features: Vec<Features>,
    pub levels: Vec<Levels>,
    /// Signal events in `known_idx` order.
    pub events: Vec<Event>,
    pub source: String,
}

impl Market {
    /// Validate the candles, run the signal source, the context features and the level tracker.
    pub fn prepare(bars: Vec<Candle>, tf_ms: i64, src: &dyn SignalSource) -> Result<Market, Error> {
        validate_bars(&bars, tf_ms)?;
        let mut ctx = Context::new(ContextParams::for_base(tf_ms));
        let mut lv = LevelTracker::new();
        let mut features = Vec::with_capacity(bars.len());
        let mut levels = Vec::with_capacity(bars.len());
        for (i, c) in bars.iter().enumerate() {
            features.push(ctx.update(i as u32, c));
            levels.push(lv.step(i as u32, c));
        }
        let events = src.events(&bars, tf_ms);
        validate_events(&bars, &events)?;
        Ok(Market {
            tf_ms,
            bars,
            features,
            levels,
            events,
            source: src.name(),
        })
    }

    /// Close time (exclusive end) of candle `i`: the moment its data becomes usable.
    pub fn close_at(&self, i: usize) -> i64 {
        self.bars[i].open_time + self.tf_ms
    }
}

fn validate_bars(bars: &[Candle], tf_ms: i64) -> Result<(), Error> {
    if tf_ms <= 0 {
        return Err(Error::Input(format!("bad candle length {tf_ms} ms")));
    }
    if bars.len() >= u32::MAX as usize {
        return Err(Error::Input("too many candles".into()));
    }
    for (i, c) in bars.iter().enumerate() {
        let ok = [c.open, c.high, c.low, c.close]
            .iter()
            .all(|x| x.is_finite())
            && c.low <= c.open.min(c.close)
            && c.high >= c.open.max(c.close)
            && c.low > 0.0;
        if !ok {
            return Err(Error::Input(format!("candle {i} has invalid OHLC: {c:?}")));
        }
        if i > 0 {
            let d = c.open_time - bars[i - 1].open_time;
            if d <= 0 || d % tf_ms != 0 {
                return Err(Error::Input(format!(
                    "candle {i}: open times must increase in steps of {tf_ms} ms"
                )));
            }
        }
    }
    Ok(())
}

fn validate_events(bars: &[Candle], events: &[Event]) -> Result<(), Error> {
    let mut last = 0u32;
    for e in events {
        let k = e.known_idx as usize;
        if k >= bars.len() {
            return Err(Error::Input(format!(
                "event {}/{} known at index {k} beyond the {} candles",
                e.det,
                e.kind,
                bars.len()
            )));
        }
        if e.known_at != bars[k].open_time {
            return Err(Error::Lookahead(format!(
                "event {}/{}: known_at {} is not the open time {} of its known_idx candle {k}",
                e.det, e.kind, e.known_at, bars[k].open_time
            )));
        }
        if e.formed_idx > e.known_idx {
            return Err(Error::Lookahead(format!(
                "event {}/{} formed at {} after it is known ({k})",
                e.det, e.kind, e.formed_idx
            )));
        }
        if !(-1..=1).contains(&e.dir) {
            return Err(Error::Input(format!(
                "event {}/{}: dir {}",
                e.det, e.kind, e.dir
            )));
        }
        if e.known_idx < last {
            return Err(Error::Input(format!(
                "events not in known order at {}/{} (index {k} after {last})",
                e.det, e.kind
            )));
        }
        last = e.known_idx;
    }
    Ok(())
}

/// Result of a passed [`audit_prefix`].
#[derive(Debug, Clone)]
pub struct Audit {
    /// Candle counts the market was truncated to.
    pub cuts: Vec<usize>,
    /// Events compared (summed over cuts).
    pub events_compared: usize,
}

/// Cut points for the audit: `count` seeded random cuts in the last 90 % of the series plus the
/// last candle (where a peek at the next candle shows up for sure).
pub fn audit_cuts(n: usize, count: usize, seed: u64) -> Vec<usize> {
    if n < 3 || count == 0 {
        return Vec::new();
    }
    let mut rng = Rng::new(seed ^ 0x5eed_a0d1);
    let lo = (n / 10).max(1);
    let mut v: Vec<usize> = (0..count.saturating_sub(1))
        .map(|_| lo + rng.below((n - 1 - lo).max(1) as u64) as usize)
        .collect();
    v.push(n - 1);
    v.sort_unstable();
    v.dedup();
    v
}

/// Lookahead guard: rebuild the market on `bars[..cut]` for each cut and require that features,
/// levels and events up to the cut are identical to the full run. Any difference means some
/// decision-time value at a candle depends on later candles, and the run is refused.
pub fn audit_prefix(full: &Market, src: &dyn SignalSource, cuts: &[usize]) -> Result<Audit, Error> {
    let mut compared = 0;
    for &cut in cuts {
        if cut == 0 || cut >= full.bars.len() {
            continue;
        }
        let m = Market::prepare(full.bars[..cut].to_vec(), full.tf_ms, src)?;
        for i in 0..cut {
            if !full.features[i].same(&m.features[i]) {
                return Err(Error::Lookahead(format!(
                    "context features of candle {i} change when the data ends at candle {cut}"
                )));
            }
            if !full.levels[i].same(&m.levels[i]) {
                return Err(Error::Lookahead(format!(
                    "levels of candle {i} change when the data ends at candle {cut}"
                )));
            }
        }
        let want: Vec<&Event> = full
            .events
            .iter()
            .filter(|e| (e.known_idx as usize) < cut)
            .collect();
        if want.len() != m.events.len() {
            return Err(Error::Lookahead(format!(
                "source {}: {} events known before candle {cut} in the full run, {} when the data \
                 ends there",
                src.name(),
                want.len(),
                m.events.len()
            )));
        }
        for (a, b) in want.iter().zip(&m.events) {
            if !a.same(b) {
                return Err(Error::Lookahead(format!(
                    "source {}: event {}/{} at candle {} differs when the data ends at candle {cut}",
                    src.name(),
                    a.det,
                    a.kind,
                    a.known_idx
                )));
            }
        }
        compared += want.len();
    }
    Ok(Audit {
        cuts: cuts.to_vec(),
        events_compared: compared,
    })
}
