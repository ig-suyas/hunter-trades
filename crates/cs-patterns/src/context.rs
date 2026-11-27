//! Per-candle context features: ATR(14), trend regime, volatility percentile, UTC hour and
//! session, distance to the nearest zone in ATR, volume z-score. Streaming, no lookahead: the
//! features of bar `i` use bars `<= i` only.

use cs_core::{Candle, Event};

use crate::htf::{HtfSet, HtfSetParams, HtfSnap};
use crate::util::{Adx, Atr, Ring, RollStats};
use crate::zones::{SdParams, SdZone, SdZones, SrParams, SrZone, SrZones};

/// Context parameters.
#[derive(Debug, Clone, Copy)]
pub struct ContextParams {
    /// ADX length and thresholds: trend when ADX >= `adx_trend`, range when ADX < `adx_range`,
    /// in between keep the previous regime (hysteresis). Defaults 14 / 25 / 20.
    pub adx_len: usize,
    pub adx_trend: f64,
    pub adx_range: f64,
    /// Volatility percentile window (ATR rank among the last N bars). Default 500 (~5 days on
    /// 15m); sensible 100..=2000.
    pub vol_window: usize,
    /// Volume z-score window (previous N bars). Default 20; sensible 10..=100.
    pub volz_window: usize,
    pub sr: SrParams,
    pub sd: SdParams,
    /// Higher-time-frame trend state (default 1h and 4h on a 15m base; set `htf.base_ms` to the
    /// series' candle length, e.g. with [`ContextParams::for_base`]).
    pub htf: HtfSetParams,
}

impl ContextParams {
    /// Defaults for a series of candles `base_ms` long.
    pub fn for_base(base_ms: i64) -> ContextParams {
        ContextParams {
            htf: HtfSetParams::for_base(base_ms),
            ..ContextParams::default()
        }
    }
}

impl Default for ContextParams {
    fn default() -> Self {
        ContextParams {
            adx_len: 14,
            adx_trend: 25.0,
            adx_range: 20.0,
            vol_window: 500,
            volz_window: 20,
            sr: SrParams::default(),
            sd: SdParams::default(),
            htf: HtfSetParams::default(),
        }
    }
}

/// UTC sessions (no daylight-saving shift): 0 Asia 00-07, 1 London 07-12, 2 London/New York
/// overlap 12-16, 3 New York 16-21, 4 late 21-24.
pub const SESSIONS: [&str; 5] = ["asia", "london", "overlap", "newyork", "late"];

pub fn session_of_hour(h: u8) -> u8 {
    match h {
        0..=6 => 0,
        7..=11 => 1,
        12..=15 => 2,
        16..=20 => 3,
        _ => 4,
    }
}

/// Features of one candle.
#[derive(Debug, Clone, Copy)]
pub struct Features {
    pub idx: u32,
    pub open_time: i64,
    pub atr: f64,
    pub adx: f64,
    /// +1 up-trend, -1 down-trend, 0 range.
    pub regime: i8,
    /// Share of the last `vol_window` ATR values below the current one, in [0, 1].
    pub vol_pct: f64,
    pub hour: u8,
    pub session: u8,
    /// Distance from the close to the nearest S/R or supply/demand zone in ATR (0 inside a zone,
    /// `NaN` if there is no zone).
    pub zone_dist_atr: f64,
    /// (volume - mean) / std of the previous `volz_window` volumes.
    pub vol_z: f64,
    /// Higher-time-frame state (slot 0 = 1h, slot 1 = 4h by default), from completed HTF
    /// candles only; see [`HtfSnap::known_at`].
    pub htf: [HtfSnap; 2],
}

impl Features {
    pub fn same(&self, o: &Features) -> bool {
        let f = |a: f64, b: f64| a.to_bits() == b.to_bits();
        self.idx == o.idx
            && self.open_time == o.open_time
            && f(self.atr, o.atr)
            && f(self.adx, o.adx)
            && self.regime == o.regime
            && f(self.vol_pct, o.vol_pct)
            && self.hour == o.hour
            && self.session == o.session
            && f(self.zone_dist_atr, o.zone_dist_atr)
            && f(self.vol_z, o.vol_z)
            && self.htf[0].same(&o.htf[0])
            && self.htf[1].same(&o.htf[1])
    }
}

/// Context feature tracker.
#[derive(Debug, Clone)]
pub struct Context {
    p: ContextParams,
    atr: Atr,
    adx: Adx,
    regime: i8,
    atr_hist: Ring<f64>,
    /// The same window kept sorted (allocated once; insert / remove shift at most `vol_window`
    /// floats), so the percentile is a binary search instead of a scan.
    atr_sorted: Vec<f64>,
    vol: RollStats,
    sr: SrZones,
    sd: SdZones,
    scratch: Vec<Event>,
    htf: HtfSet,
}

impl Context {
    pub fn new(p: ContextParams) -> Context {
        Context {
            p,
            atr: Atr::new(14),
            adx: Adx::new(p.adx_len),
            regime: 0,
            atr_hist: Ring::new(p.vol_window.max(1)),
            atr_sorted: Vec::with_capacity(p.vol_window.max(1)),
            vol: RollStats::new(p.volz_window.max(2)),
            sr: SrZones::new(p.sr),
            sd: SdZones::new(p.sd),
            scratch: Vec::with_capacity(32),
            htf: HtfSet::new(p.htf),
        }
    }

    /// Formed S/R zones as of the latest update (read only; the live context panel shows them).
    pub fn sr_zones(&self) -> impl Iterator<Item = &SrZone> {
        self.sr.zones()
    }

    /// Supply/demand zones as of the latest update (read only).
    pub fn sd_zones(&self) -> &[SdZone] {
        self.sd.zones()
    }

    pub fn update(&mut self, i: u32, c: &Candle) -> Features {
        for t in &mut self.htf.trends {
            t.step(i, c);
        }
        self.atr.update(c);
        let atr = self.atr.get();
        self.adx.update(c);
        let (adx, pdi, mdi) = self.adx.get();
        if !adx.is_nan() {
            if adx >= self.p.adx_trend {
                self.regime = if pdi >= mdi { 1 } else { -1 };
            } else if adx < self.p.adx_range {
                self.regime = 0;
            }
        }
        let vol_pct = if atr.is_nan() || self.atr_sorted.is_empty() {
            f64::NAN
        } else {
            let below = self.atr_sorted.partition_point(|x| *x < atr);
            below as f64 / self.atr_sorted.len() as f64
        };
        if !atr.is_nan() {
            if self.atr_hist.is_full()
                && let Some(old) = self.atr_hist.get(0)
            {
                let k = self
                    .atr_sorted
                    .partition_point(|x| x.total_cmp(&old).is_lt());
                if k < self.atr_sorted.len() {
                    self.atr_sorted.remove(k);
                }
            }
            self.atr_hist.push(atr);
            let k = self
                .atr_sorted
                .partition_point(|x| x.total_cmp(&atr).is_lt());
            self.atr_sorted.insert(k, atr);
        }
        let (m, s) = (self.vol.mean(), self.vol.std());
        let vol_z = if s > 0.0 {
            (c.volume - m) / s
        } else {
            f64::NAN
        };
        self.vol.push(c.volume);
        self.scratch.clear();
        self.sr.step(i, c, &mut self.scratch);
        self.sd.step(i, c, &mut self.scratch);
        let mut best = f64::INFINITY;
        let mut dist = |lo: f64, hi: f64| {
            let d = if c.close < lo {
                lo - c.close
            } else if c.close > hi {
                c.close - hi
            } else {
                0.0
            };
            best = best.min(d);
        };
        for z in self.sr.zones() {
            dist(z.lo, z.hi);
        }
        for z in self.sd.zones() {
            dist(z.lo, z.hi);
        }
        let zone_dist_atr = if best.is_finite() && atr > 0.0 {
            best / atr
        } else {
            f64::NAN
        };
        let hour = u8::try_from(c.open_time.rem_euclid(86_400_000) / 3_600_000).unwrap_or(0);
        Features {
            idx: i,
            open_time: c.open_time,
            atr,
            adx,
            regime: self.regime,
            vol_pct,
            hour,
            session: session_of_hour(hour),
            zone_dist_atr,
            vol_z,
            htf: self.htf.snaps(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::synthetic;

    #[test]
    fn vol_percentile_matches_brute_force() {
        let data = synthetic(3000, 11);
        let p = ContextParams {
            vol_window: 100,
            ..ContextParams::default()
        };
        let mut ctx = Context::new(p);
        let mut atr = Atr::new(14);
        let mut hist: Vec<f64> = Vec::new();
        for (i, c) in data.iter().enumerate() {
            let f = ctx.update(i as u32, c);
            atr.update(c);
            let a = atr.get();
            if a.is_nan() {
                assert!(f.vol_pct.is_nan());
                continue;
            }
            if !hist.is_empty() {
                let below = hist.iter().filter(|x| **x < a).count();
                let want = below as f64 / hist.len() as f64;
                assert!(
                    (f.vol_pct - want).abs() < 1e-12,
                    "bar {i}: {} vs {want}",
                    f.vol_pct
                );
            }
            hist.push(a);
            if hist.len() > 100 {
                hist.remove(0);
            }
            assert_eq!(f.session, session_of_hour(f.hour));
        }
    }
}
