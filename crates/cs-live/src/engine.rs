//! The incremental context engine. One code path with the backtest: the detector events come
//! from [`cs_patterns::Runner`] over [`cs_patterns::default_set`] and the features from
//! [`cs_patterns::context::Context`] with [`ContextParams::for_base`], exactly as
//! `cs_backtest::market::Market::prepare` builds them. The engine only adds the candle checks a
//! live feed needs and the [`ContextSnapshot`] for the panel.
//!
//! Only closed candles go in, strictly forward in time. A candle at or before the last accepted
//! one is rejected, so nothing already shown is ever recomputed, repainted or removed.

use cs_core::{Candle, Event};
use cs_patterns::Runner;
use cs_patterns::context::{Context, ContextParams, Features};

use crate::snapshot::{self, ContextSnapshot, Markers};

/// Why a candle was not taken.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Reject {
    /// At or before the last accepted candle (duplicate or replay): ignored, never re-applied.
    NotNewer { open_time: i64, last: i64 },
    /// Open time is not on the time-frame grid after the last candle.
    Misaligned { open_time: i64, last: i64 },
    /// Non-finite or inconsistent OHLC.
    Invalid { open_time: i64 },
}

/// Output of one accepted candle.
pub struct Step<'a> {
    pub features: Features,
    /// Events that became known at this candle's close (in emission order).
    pub events: &'a [Event],
    pub snapshot: ContextSnapshot,
    /// Missing candles skipped before this one (0 normally; the live transport backfills gaps
    /// before pushing, the batch preparation allows them the same way).
    pub gap: i64,
}

pub struct LiveEngine {
    tf_ms: i64,
    runner: Runner,
    ctx: Context,
    buf: Vec<Event>,
    markers: Markers,
    last_open: Option<i64>,
    last: Option<ContextSnapshot>,
    accepted: u64,
    gaps: u64,
}

impl LiveEngine {
    pub fn new(tf_ms: i64) -> LiveEngine {
        LiveEngine {
            tf_ms,
            runner: Runner::new(cs_patterns::default_set(tf_ms), None),
            ctx: Context::new(ContextParams::for_base(tf_ms)),
            buf: Vec::with_capacity(64),
            markers: Markers::default(),
            last_open: None,
            last: None,
            accepted: 0,
            gaps: 0,
        }
    }

    pub fn tf_ms(&self) -> i64 {
        self.tf_ms
    }

    /// Open time of the last accepted candle.
    pub fn last_open(&self) -> Option<i64> {
        self.last_open
    }

    /// Latest snapshot.
    pub fn snapshot(&self) -> Option<&ContextSnapshot> {
        self.last.as_ref()
    }

    pub fn accepted(&self) -> u64 {
        self.accepted
    }

    pub fn gaps(&self) -> u64 {
        self.gaps
    }

    /// Feed one **closed** candle.
    ///
    /// # Errors
    /// [`Reject`] for a candle at or before the last one, off the time-frame grid, or with
    /// invalid prices; the engine state is then unchanged.
    pub fn push_closed(&mut self, c: &Candle) -> Result<Step<'_>, Reject> {
        let ok = [c.open, c.high, c.low, c.close]
            .iter()
            .all(|x| x.is_finite())
            && c.low <= c.open.min(c.close)
            && c.high >= c.open.max(c.close)
            && c.low > 0.0;
        if !ok {
            return Err(Reject::Invalid {
                open_time: c.open_time,
            });
        }
        let mut gap = 0;
        if let Some(last) = self.last_open {
            let d = c.open_time - last;
            if d <= 0 {
                return Err(Reject::NotNewer {
                    open_time: c.open_time,
                    last,
                });
            }
            if d % self.tf_ms != 0 {
                return Err(Reject::Misaligned {
                    open_time: c.open_time,
                    last,
                });
            }
            gap = d / self.tf_ms - 1;
        }
        let i = u32::try_from(self.accepted).map_err(|_| Reject::Invalid {
            open_time: c.open_time,
        })?;
        self.buf.clear();
        self.runner.step(c, &mut self.buf);
        let features = self.ctx.update(i, c);
        self.markers.absorb(&self.buf);
        let snap = snapshot::build(
            &features,
            c.close,
            self.tf_ms,
            self.tf_ms,
            &self.ctx,
            &self.markers,
        );
        self.last = Some(snap);
        self.last_open = Some(c.open_time);
        self.accepted += 1;
        if gap > 0 {
            self.gaps += 1;
        }
        Ok(Step {
            features,
            events: &self.buf,
            snapshot: snap,
            gap,
        })
    }
}

/// Batch reference: run the engine over a whole slice in one go and return every snapshot.
pub fn batch_snapshots(candles: &[Candle], tf_ms: i64) -> Vec<ContextSnapshot> {
    let mut e = LiveEngine::new(tf_ms);
    candles
        .iter()
        .filter_map(|c| e.push_closed(c).ok().map(|s| s.snapshot))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use cs_patterns::testutil::synthetic;

    #[test]
    fn rejects_replays_and_never_rewrites() {
        let data = synthetic(1500, 7);
        let mut e = LiveEngine::new(900_000);
        let mut shown = Vec::new();
        for (k, c) in data.iter().enumerate() {
            let s = e.push_closed(c).map(|s| s.snapshot.exact());
            shown.push(s.unwrap_or_default());
            if k > 10 && k % 97 == 0 {
                // A reconnect replays the last two closed candles: both refused, state unchanged.
                for old in &data[k - 1..=k] {
                    assert!(matches!(e.push_closed(old), Err(Reject::NotNewer { .. })));
                }
                assert_eq!(
                    e.snapshot().map(ContextSnapshot::exact),
                    shown.last().cloned()
                );
            }
        }
        // Prefix invariance of the snapshot: a run cut at k shows exactly what the full run
        // showed at k (nothing shown is revised by later candles).
        for cut in [50usize, 333, 1000, 1499] {
            let b = batch_snapshots(&data[..=cut], 900_000);
            assert_eq!(
                b.last().map(ContextSnapshot::exact).as_ref(),
                shown.get(cut)
            );
        }
        let mut bad = data[1499];
        bad.open_time += 900_000;
        bad.high = bad.low - 1.0;
        assert!(matches!(e.push_closed(&bad), Err(Reject::Invalid { .. })));
        bad = data[1499];
        bad.open_time += 450_000;
        assert!(matches!(
            e.push_closed(&bad),
            Err(Reject::Misaligned { .. })
        ));
    }
}
