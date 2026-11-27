//! The context shown by the overlay after each closed candle. It is built only from the context
//! features, the detector events and the zones of the same Phase 2 code the backtest uses, all
//! known at that candle's close. It carries **no direction call, score or probability**: the
//! GATE found no setup with an edge (0 of 59), so the overlay describes the market and nothing
//! else.

use cs_core::Event;
use cs_patterns::context::{Context, Features};
use cs_patterns::pullback;

/// A pullback older than this many candles is not "current".
pub const PULLBACK_RECENT_BARS: u32 = 12;

/// Higher-time-frame state (from completed higher-time-frame candles only).
#[derive(Debug, Clone, Copy)]
pub struct HtfView {
    /// e.g. "1h", "4h".
    pub label: &'static str,
    /// `false` when the higher time frame is not a larger multiple of the chart's time frame.
    pub active: bool,
    /// The slow EMA has enough candles.
    pub ready: bool,
    /// +1 bull stack, -1 bear stack, 0 neither ("off").
    pub stack: i8,
    /// +1 structure up (HH + HL), -1 down (LH + LL), 0 range.
    pub structure: i8,
    /// Open time of the latest completed higher-time-frame candle (`i64::MIN` if none).
    pub candle_open: i64,
}

/// What kind of zone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ZoneKind {
    /// Support/resistance zone from clustered swing pivots.
    Sr,
    /// Demand zone (base before a strong move up).
    Demand,
    /// Supply zone (base before a strong move down).
    Supply,
}

/// One zone relative to the latest close.
#[derive(Debug, Clone, Copy)]
pub struct ZoneView {
    pub kind: ZoneKind,
    pub lo: f64,
    pub hi: f64,
    /// +1 the zone is above the close, -1 below, 0 the close is inside it.
    pub side: i8,
    /// Distance from the close to the nearest edge, in ATR (0 inside).
    pub dist_atr: f64,
    /// Pivot touches (S/R) or revisits (supply/demand).
    pub count: u16,
}

/// The latest event of one kind (sweep, `BOS`, `CHoCH`).
#[derive(Debug, Clone, Copy)]
pub struct Marker {
    /// Detector kind, e.g. `"bos_bull"`; the UI turns it into neutral words.
    pub kind: &'static str,
    /// +1 upward, -1 downward (direction of the move that happened, not a call).
    pub dir: i8,
    pub price: f64,
    /// Open time of the candle at whose close it became known.
    pub known_at: i64,
    pub known_idx: u32,
}

/// The latest pullback event.
#[derive(Debug, Clone, Copy)]
pub struct PullbackView {
    /// +1 pullback inside an up-move, -1 inside a down-move.
    pub dir: i8,
    /// The Phase 2 "healthy" flag (smaller and slower than the impulse, no close beyond the last
    /// swing). Descriptive only: the flag showed no edge in Phase 4.
    pub healthy: bool,
    pub known_at: i64,
    pub known_idx: u32,
}

/// Context after one closed candle.
#[derive(Debug, Clone, Copy)]
pub struct ContextSnapshot {
    /// Candle index in the engine's stream (warm-up included).
    pub idx: u32,
    pub open_time: i64,
    /// Exclusive close time: the moment this context became known.
    pub close_at: i64,
    pub close: f64,
    pub atr: f64,
    pub adx: f64,
    /// ADX regime: +1 trend up, -1 trend down, 0 range.
    pub regime: i8,
    /// Volatility percentile in [0, 1] (ATR rank over the last 500 candles).
    pub vol_pct: f64,
    pub hour: u8,
    /// Index into [`cs_patterns::context::SESSIONS`].
    pub session: u8,
    pub htf: [HtfView; 2],
    /// Nearest S/R zone entirely below / above the close, and one the close is inside.
    pub sr_below: Option<ZoneView>,
    pub sr_above: Option<ZoneView>,
    pub sr_inside: Option<ZoneView>,
    /// Nearest demand and supply zones (any side).
    pub demand: Option<ZoneView>,
    pub supply: Option<ZoneView>,
    pub sweep: Option<Marker>,
    pub bos: Option<Marker>,
    pub choch: Option<Marker>,
    pub pullback: Option<PullbackView>,
}

impl ContextSnapshot {
    /// Candles since the latest pullback event, if any.
    pub fn pullback_age(&self) -> Option<u32> {
        self.pullback.map(|p| self.idx.saturating_sub(p.known_idx))
    }

    /// The latest pullback if it is recent enough to call "current".
    pub fn current_pullback(&self) -> Option<PullbackView> {
        match self.pullback_age() {
            Some(a) if a <= PULLBACK_RECENT_BARS => self.pullback,
            _ => None,
        }
    }

    /// Exact text form (floats in shortest round-trip form, so equal text means equal bits up to
    /// NaN payloads). Used by the parity test.
    pub fn exact(&self) -> String {
        format!("{self:?}")
    }
}

/// Keeps the latest markers across candles (append-only: a later event replaces the shown one,
/// nothing is ever removed or rewritten for a past candle).
#[derive(Debug, Clone, Default)]
pub struct Markers {
    pub sweep: Option<Marker>,
    pub bos: Option<Marker>,
    pub choch: Option<Marker>,
    pub pullback: Option<PullbackView>,
}

impl Markers {
    /// Take in the events of one candle (in emission order).
    pub fn absorb(&mut self, events: &[Event]) {
        for e in events {
            let m = Marker {
                kind: e.kind,
                dir: e.dir,
                price: e.price,
                known_at: e.known_at,
                known_idx: e.known_idx,
            };
            match e.det {
                "sweep" => self.sweep = Some(m),
                "bos" => self.bos = Some(m),
                "choch" => self.choch = Some(m),
                "pullback" => {
                    self.pullback = Some(PullbackView {
                        dir: e.dir,
                        healthy: e.tags & pullback::HEALTHY != 0,
                        known_at: e.known_at,
                        known_idx: e.known_idx,
                    });
                }
                _ => {}
            }
        }
    }
}

fn htf_label(ms: i64) -> &'static str {
    match ms {
        3_600_000 => "1h",
        14_400_000 => "4h",
        86_400_000 => "1d",
        _ => "htf",
    }
}

fn nearer(best: Option<ZoneView>, z: ZoneView) -> ZoneView {
    match best {
        Some(b) if b.dist_atr <= z.dist_atr => b,
        _ => z,
    }
}

/// Build the snapshot of candle `f.idx` from its features, the context's zones and the markers.
pub fn build(
    f: &Features,
    close: f64,
    tf_ms: i64,
    base_ms: i64,
    ctx: &Context,
    mk: &Markers,
) -> ContextSnapshot {
    let atr = f.atr;
    let dist = |d: f64| if atr > 0.0 { d / atr } else { f64::NAN };
    let place = |kind: ZoneKind, lo: f64, hi: f64, count: u16| {
        let (side, d) = if lo > close {
            (1, lo - close)
        } else if hi < close {
            (-1, close - hi)
        } else {
            (0, 0.0)
        };
        ZoneView {
            kind,
            lo,
            hi,
            side,
            dist_atr: dist(d),
            count,
        }
    };
    let (mut below, mut above, mut inside) = (None, None, None);
    for z in ctx.sr_zones() {
        let v = place(ZoneKind::Sr, z.lo, z.hi, z.touches);
        match v.side {
            -1 => below = Some(nearer(below, v)),
            1 => above = Some(nearer(above, v)),
            _ => inside = Some(nearer(inside, v)),
        }
    }
    let (mut demand, mut supply) = (None, None);
    for z in ctx.sd_zones() {
        if z.dir > 0 {
            demand = Some(nearer(demand, place(ZoneKind::Demand, z.lo, z.hi, z.tests)));
        } else {
            supply = Some(nearer(supply, place(ZoneKind::Supply, z.lo, z.hi, z.tests)));
        }
    }
    let htf = f.htf.map(|h| HtfView {
        label: htf_label(h.htf_ms),
        active: h.htf_ms > base_ms && h.htf_ms % base_ms == 0,
        ready: h.ema.ready,
        stack: h.ema.stack,
        structure: h.structure,
        candle_open: h.candle_open,
    });
    ContextSnapshot {
        idx: f.idx,
        open_time: f.open_time,
        close_at: f.open_time + tf_ms,
        close,
        atr,
        adx: f.adx,
        regime: f.regime,
        vol_pct: f.vol_pct,
        hour: f.hour,
        session: f.session,
        htf,
        sr_below: below,
        sr_above: above,
        sr_inside: inside,
        demand,
        supply,
        sweep: mk.sweep,
        bos: mk.bos,
        choch: mk.choch,
        pullback: mk.pullback,
    }
}
