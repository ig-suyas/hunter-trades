//! cs-patterns: rule-based pattern detectors as streaming state machines.
//!
//! * Feed one closed candle at a time ([`Detector::update`]). Work per candle is constant or
//!   amortised constant (bounded by fixed windows and fixed zone/pool capacities), and no
//!   detector allocates after construction: rings and bounded lists are sized up front and the
//!   caller owns (and reuses) the output buffer.
//! * No lookahead: an event is emitted only at the candle whose close reveals it
//!   (`known_idx` = that candle), carries `formed_idx` (where the pattern completes) and is never
//!   revised later. There is no end-of-data flush. `tests/prefix.rs` checks this for every
//!   detector ("prefix invariance").
//! * Every detector has a `*Params` struct with documented defaults and sensible ranges.
//!
//! Definitions follow `notes/knowledge.md` (IDs S*, T*, K* there).

// Index/price conversions between u32 bar indices, usize window sizes and f64 are bounded by the
// window and history sizes used here (far below 2^32 bars / 2^52), so these casts are exact.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::too_many_lines
)]

pub mod breakout;
pub mod candles;
pub mod context;
pub mod fib;
pub mod htf;
pub mod pullback;
pub mod structure;
pub mod sweep;
pub mod swing;
pub mod testutil;
pub mod trend;
pub mod util;
pub mod zones;

use cs_core::{Candle, Event};

use crate::context::{Context, ContextParams, Features};
use crate::util::Ring;

/// A streaming detector.
pub trait Detector {
    /// Detector names this instance can emit (used for reporting and zero-count checks).
    fn names(&self) -> &'static [&'static str];

    /// Feed candle `i` (0-based index in the stream, strictly increasing by one) after it closed.
    /// Every pushed event has `known_idx == i`.
    fn update(&mut self, i: u32, c: &Candle, out: &mut Vec<Event>);
}

/// One entry of the default detector set.
pub struct Entry {
    /// Short label of the detector group (for tests and reports).
    pub group: &'static str,
    /// Build the detector for a series whose candles are `base_ms` long (only the
    /// higher-time-frame group uses it).
    pub make: fn(base_ms: i64) -> Box<dyn Detector>,
}

/// The full tier-1 detector set with default parameters.
pub fn registry() -> Vec<Entry> {
    fn b<D: Detector + 'static>(d: D) -> Box<dyn Detector> {
        Box::new(d)
    }
    vec![
        Entry {
            group: "swing",
            make: |_| b(swing::Swings::new(swing::SwingParams::default())),
        },
        Entry {
            group: "structure",
            make: |_| {
                b(structure::Structure::new(
                    structure::StructureParams::default(),
                ))
            },
        },
        Entry {
            group: "bos_choch",
            make: |_| {
                b(structure::StructureBreak::new(
                    structure::BreakParams::default(),
                ))
            },
        },
        Entry {
            group: "sr_zones",
            make: |_| b(zones::SrZones::new(zones::SrParams::default())),
        },
        Entry {
            group: "sd_zones",
            make: |_| b(zones::SdZones::new(zones::SdParams::default())),
        },
        Entry {
            group: "ema",
            make: |_| b(trend::EmaState::new(trend::EmaParams::default())),
        },
        Entry {
            group: "trendline",
            make: |_| b(trend::Trendline::new(trend::TrendlineParams::default())),
        },
        Entry {
            group: "fib",
            make: |_| b(fib::Fib::new(fib::FibParams::default())),
        },
        Entry {
            group: "pullback",
            make: |_| b(pullback::Pullback::new(pullback::PullbackParams::default())),
        },
        Entry {
            group: "breakouts",
            make: |_| b(breakout::Breakouts::new(breakout::BreakoutParams::default())),
        },
        Entry {
            group: "sweeps",
            make: |_| b(sweep::Sweeps::new(sweep::SweepParams::default())),
        },
        Entry {
            group: "htf_trend",
            make: |base_ms| b(htf::HtfSet::new(htf::HtfSetParams::for_base(base_ms))),
        },
        Entry {
            group: "candles",
            make: |_| b(candles::Candles::new(candles::CandleParams::default())),
        },
    ]
}

/// Default detector set for candles `base_ms` long, ready to run.
pub fn default_set(base_ms: i64) -> Vec<Box<dyn Detector>> {
    registry().into_iter().map(|e| (e.make)(base_ms)).collect()
}

/// How far back `formed_idx` may lie for the runner to fill `formed_at`.
pub const TIME_RING: usize = 4096;

/// Runs a set of detectors (and optionally the context features) over a candle stream and
/// fills `formed_at` / `known_at` from the candles' open times.
pub struct Runner {
    dets: Vec<Box<dyn Detector>>,
    ctx: Option<Context>,
    times: Ring<i64>,
    n: u32,
}

impl Runner {
    pub fn new(dets: Vec<Box<dyn Detector>>, context: Option<ContextParams>) -> Runner {
        Runner {
            dets,
            ctx: context.map(Context::new),
            times: Ring::new(TIME_RING),
            n: 0,
        }
    }

    /// All detector names of this runner, in order.
    pub fn names(&self) -> Vec<&'static str> {
        self.dets
            .iter()
            .flat_map(|d| d.names().iter().copied())
            .collect()
    }

    /// Feed the next closed candle; events are appended to `out` (not cleared).
    pub fn step(&mut self, c: &Candle, out: &mut Vec<Event>) -> Option<Features> {
        let i = self.n;
        self.n += 1;
        self.times.push(c.open_time);
        let start = out.len();
        for d in &mut self.dets {
            d.update(i, c, out);
        }
        for e in &mut out[start..] {
            e.known_at = c.open_time;
            let back = (i - e.formed_idx.min(i)) as usize;
            e.formed_at = self.times.back(back).unwrap_or(i64::MIN);
        }
        self.ctx.as_mut().map(|x| x.update(i, c))
    }
}

/// Run detectors over a whole slice (batch helper used by tests and the CLI).
pub fn run_all(dets: Vec<Box<dyn Detector>>, candles: &[Candle]) -> Vec<Event> {
    let mut r = Runner::new(dets, None);
    let mut out = Vec::with_capacity(candles.len() / 2 + 16);
    for c in candles {
        r.step(c, &mut out);
    }
    out
}
