//! Higher-time-frame trend state (knowledge T1 / S8 on a higher time frame, T6 condition 1).
//!
//! For a base series (e.g. 5m or 15m), base candles are aggregated into UTC-aligned higher-time-
//! frame candles (default 1h and 4h). **Only completed higher-time-frame candles are used:** an
//! HTF candle completes at the close of the base candle that ends its bucket
//! (`base open + base length == bucket open + HTF length`), and only if every base candle of the
//! bucket is present (strict, like `cs-data` resampling); incomplete buckets are dropped. On each
//! completed HTF candle the EMA 20/50/200 state and slope ([`EmaState`]) and the market-structure
//! trend ([`Structure`], swings of strength 3 on HTF candles) are updated. Between completions the
//! state is held. `known_at` is the open time of the base candle whose close completed the HTF
//! candle, so the state is usable from the next base candle's open.

use cs_core::{Candle, Event};

use crate::Detector;
use crate::structure::{Structure, StructureParams};
use crate::trend::{EmaParams, EmaSnap, EmaState};

const H1: i64 = 3_600_000;
const H4: i64 = 4 * H1;

/// Parameters for one higher time frame.
#[derive(Debug, Clone, Copy)]
pub struct HtfParams {
    /// Base candle length in ms (the series fed in). Default 15m.
    pub base_ms: i64,
    /// Higher time frame in ms; must be a multiple of `base_ms` and larger, otherwise the tracker
    /// stays inactive. Default 1h.
    pub htf_ms: i64,
    /// EMA periods / slope bars on HTF candles. Defaults 20 / 50 / 200 / 5.
    pub ema: EmaParams,
    /// Swing strength on HTF candles. Default k = 3 (sensible 2..=5).
    pub structure: StructureParams,
}

impl Default for HtfParams {
    fn default() -> Self {
        HtfParams {
            base_ms: 15 * 60_000,
            htf_ms: H1,
            ema: EmaParams::default(),
            structure: StructureParams::default(),
        }
    }
}

/// State of one higher time frame after a base candle.
#[derive(Debug, Clone, Copy)]
pub struct HtfSnap {
    pub htf_ms: i64,
    /// Number of completed HTF candles so far (0 = no state yet).
    pub n: u32,
    /// Open time of the latest completed HTF candle (`i64::MIN` if none).
    pub candle_open: i64,
    /// Open time of the base candle whose close completed it (`i64::MIN` if none).
    pub known_at: i64,
    pub close: f64,
    /// EMA state on HTF candles (stack, slopes in HTF-ATR per HTF bar; `ema.ready` false until
    /// the slow EMA has enough HTF candles).
    pub ema: EmaSnap,
    /// HTF market-structure trend: +1 up (HH + HL), -1 down (LH + LL), 0 range.
    pub structure: i8,
}

impl HtfSnap {
    fn empty(htf_ms: i64) -> HtfSnap {
        HtfSnap {
            htf_ms,
            n: 0,
            candle_open: i64::MIN,
            known_at: i64::MIN,
            close: f64::NAN,
            ema: EmaSnap::default(),
            structure: 0,
        }
    }

    /// Bit-exact equality (floats by bits).
    pub fn same(&self, o: &HtfSnap) -> bool {
        let f = |a: f64, b: f64| a.to_bits() == b.to_bits();
        let (a, b) = (&self.ema, &o.ema);
        self.htf_ms == o.htf_ms
            && self.n == o.n
            && self.candle_open == o.candle_open
            && self.known_at == o.known_at
            && f(self.close, o.close)
            && self.structure == o.structure
            && f(a.fast, b.fast)
            && f(a.mid, b.mid)
            && f(a.slow, b.slow)
            && f(a.slope_fast, b.slope_fast)
            && f(a.slope_mid, b.slope_mid)
            && f(a.slope_slow, b.slope_slow)
            && a.stack == b.stack
            && a.ready == b.ready
    }
}

/// One higher-time-frame tracker.
#[derive(Debug, Clone)]
pub struct HtfTrend {
    p: HtfParams,
    active: bool,
    expected: i64,
    bucket: i64,
    acc: Option<Candle>,
    count: i64,
    first_idx: u32,
    ema: EmaState,
    st: Structure,
    scratch: Vec<Event>,
    snap: HtfSnap,
    /// HTF candle completed on the latest update: (candle, first base index of the bucket).
    pub completed: Option<(Candle, u32)>,
}

impl HtfTrend {
    pub fn new(p: HtfParams) -> HtfTrend {
        let active = p.base_ms > 0 && p.htf_ms > p.base_ms && p.htf_ms % p.base_ms == 0;
        HtfTrend {
            p,
            active,
            expected: if active { p.htf_ms / p.base_ms } else { 0 },
            bucket: i64::MIN,
            acc: None,
            count: 0,
            first_idx: 0,
            ema: EmaState::new(p.ema),
            st: Structure::new(p.structure),
            scratch: Vec::with_capacity(16),
            snap: HtfSnap::empty(p.htf_ms),
            completed: None,
        }
    }

    pub fn active(&self) -> bool {
        self.active
    }

    pub fn snap(&self) -> HtfSnap {
        self.snap
    }

    /// Feed base candle `i`. Returns true if an HTF candle completed at this candle's close.
    pub fn step(&mut self, i: u32, c: &Candle) -> bool {
        self.completed = None;
        if !self.active {
            return false;
        }
        let htf = self.p.htf_ms;
        let b = c.open_time.div_euclid(htf) * htf;
        if b == self.bucket {
            if let Some(a) = self.acc.as_mut() {
                a.high = a.high.max(c.high);
                a.low = a.low.min(c.low);
                a.close = c.close;
                a.volume += c.volume;
                a.quote_volume += c.quote_volume;
                a.taker_buy_base += c.taker_buy_base;
                a.taker_buy_quote += c.taker_buy_quote;
                a.trades = if a.trades < 0 || c.trades < 0 {
                    -1
                } else {
                    a.trades + c.trades
                };
            }
            self.count += 1;
        } else {
            self.bucket = b;
            self.acc = Some(Candle { open_time: b, ..*c });
            self.count = 1;
            self.first_idx = i;
        }
        if c.open_time + self.p.base_ms != b + htf || self.count != self.expected {
            return false;
        }
        let Some(h) = self.acc else { return false };
        let n = self.snap.n;
        self.scratch.clear();
        self.ema.step(n, &h, &mut self.scratch);
        self.st.step(n, &h, &mut self.scratch);
        self.snap = HtfSnap {
            htf_ms: htf,
            n: n + 1,
            candle_open: b,
            known_at: c.open_time,
            close: h.close,
            ema: self.ema.snap(),
            structure: self.st.trend(),
        };
        self.completed = Some((h, self.first_idx));
        true
    }
}

/// Parameters for the HTF set (default 1h and 4h on a 15m base).
#[derive(Debug, Clone, Copy)]
pub struct HtfSetParams {
    pub base_ms: i64,
    pub htf_ms: [i64; 2],
    pub ema: EmaParams,
    pub structure: StructureParams,
}

impl Default for HtfSetParams {
    fn default() -> Self {
        HtfSetParams {
            base_ms: 15 * 60_000,
            htf_ms: [H1, H4],
            ema: EmaParams::default(),
            structure: StructureParams::default(),
        }
    }
}

impl HtfSetParams {
    pub fn for_base(base_ms: i64) -> HtfSetParams {
        HtfSetParams {
            base_ms,
            ..HtfSetParams::default()
        }
    }
}

/// Event kinds per tracker slot: [bull stack, bear stack, stack off, trend up, down, range].
const KINDS: [[&str; 6]; 2] = [
    [
        "h1_bull_stack",
        "h1_bear_stack",
        "h1_stack_off",
        "h1_trend_up",
        "h1_trend_down",
        "h1_trend_range",
    ],
    [
        "h4_bull_stack",
        "h4_bear_stack",
        "h4_stack_off",
        "h4_trend_up",
        "h4_trend_down",
        "h4_trend_range",
    ],
];

/// The two higher-time-frame trackers, as a context source and as a detector.
///
/// Detector `htf_trend` emits on the base candle that completes an HTF candle whose state
/// changed: `h1_bull_stack` / `h1_bear_stack` / `h1_stack_off` (EMA stack, once the slow EMA is
/// ready) and `h1_trend_up` / `h1_trend_down` / `h1_trend_range` (structure); `h4_*` likewise.
/// The first slot uses the `h1_` names and the second `h4_` whatever `htf_ms` holds. `start_idx`
/// = first base candle of the HTF candle, `price` = HTF close, `lo..hi` = HTF low..high,
/// `value` = mid-EMA slope (HTF ATR per HTF bar), `tags` = HTF length in minutes.
#[derive(Debug, Clone)]
pub struct HtfSet {
    pub trends: [HtfTrend; 2],
}

impl HtfSet {
    pub fn new(p: HtfSetParams) -> HtfSet {
        let mk = |htf_ms| {
            HtfTrend::new(HtfParams {
                base_ms: p.base_ms,
                htf_ms,
                ema: p.ema,
                structure: p.structure,
            })
        };
        HtfSet {
            trends: [mk(p.htf_ms[0]), mk(p.htf_ms[1])],
        }
    }

    pub fn snaps(&self) -> [HtfSnap; 2] {
        [self.trends[0].snap(), self.trends[1].snap()]
    }
}

impl Detector for HtfSet {
    fn names(&self) -> &'static [&'static str] {
        &["htf_trend"]
    }

    fn update(&mut self, i: u32, c: &Candle, out: &mut Vec<Event>) {
        for (slot, t) in self.trends.iter_mut().enumerate() {
            let before = t.snap();
            if !t.step(i, c) {
                continue;
            }
            let Some((h, first)) = t.completed else {
                continue;
            };
            let s = t.snap();
            let k = KINDS[slot];
            let ev = |kind, dir: i8| {
                Event::new("htf_trend", kind, dir, i)
                    .with_start(first)
                    .with_price(h.close)
                    .with_zone(h.low, h.high)
                    .with_tags(u32::try_from(s.htf_ms / 60_000).unwrap_or(0))
                    .with_value(s.ema.slope_mid)
            };
            if s.ema.ready && (!before.ema.ready || before.ema.stack != s.ema.stack) {
                let (kind, dir) = match s.ema.stack {
                    1 => (k[0], 1),
                    -1 => (k[1], -1),
                    _ => (k[2], 0),
                };
                out.push(ev(kind, dir));
            }
            if s.structure != before.structure {
                let (kind, dir) = match s.structure {
                    1 => (k[3], 1),
                    -1 => (k[4], -1),
                    _ => (k[5], 0),
                };
                out.push(ev(kind, dir));
            }
        }
    }
}
