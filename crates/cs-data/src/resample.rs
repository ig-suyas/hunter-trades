//! Resample clean 1m candles to higher time frames.
//!
//! * Buckets are UTC-aligned multiples of the time-frame length since the Unix epoch
//!   (4h = 00/04/08/12/16/20 UTC, 1d = 00:00 UTC).
//! * Label = bucket open time.
//! * open = first minute's open, close = last minute's close, high/low = max/min,
//!   volume, quote volume, trades and taker volumes summed (trades = -1 if any minute lacks it;
//!   taker volumes are NaN if any minute lacks them).
//! * No partial candles: under [`Policy::Strict`] (the default) a bucket is emitted only if all
//!   its minutes are present. Every candle carries its minute count so callers can see this.
//!   [`Policy::AllowGaps`] also emits buckets with internal gaps, but never a bucket that is
//!   still open at the end of the data or that starts before the data does.
//!
//! The input must be sorted by open time with no duplicates (see `clean`).

use cs_core::{Candle, Timeframe};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Policy {
    Strict,
    AllowGaps,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ResampleStats {
    pub emitted: u64,
    /// Buckets with some but not all minutes that were dropped (`Strict`) or emitted (`AllowGaps`).
    pub incomplete: u64,
    /// Incomplete buckets at the start or end of the data (always dropped).
    pub edge_dropped: u64,
}

struct Acc {
    c: Candle,
    minutes: i64,
}

impl Acc {
    fn new(bucket: i64, m: &Candle) -> Acc {
        Acc {
            c: Candle {
                open_time: bucket,
                ..*m
            },
            minutes: 1,
        }
    }

    fn push(&mut self, m: &Candle) {
        let c = &mut self.c;
        c.high = c.high.max(m.high);
        c.low = c.low.min(m.low);
        c.close = m.close;
        c.volume += m.volume;
        c.quote_volume += m.quote_volume;
        c.trades = if c.trades < 0 || m.trades < 0 {
            -1
        } else {
            c.trades + m.trades
        };
        c.taker_buy_base += m.taker_buy_base;
        c.taker_buy_quote += m.taker_buy_quote;
        self.minutes += 1;
    }
}

/// Returns (candles, minute counts, stats).
pub fn resample(
    m1: &[Candle],
    tf: Timeframe,
    policy: Policy,
) -> (Vec<Candle>, Vec<i64>, ResampleStats) {
    let mut out = Vec::new();
    let mut mins = Vec::new();
    let mut st = ResampleStats::default();
    let Some(first) = m1.first() else {
        return (out, mins, st);
    };
    let Some(last) = m1.last() else {
        return (out, mins, st);
    };
    let data_start = first.open_time;
    let data_end = last.open_time + cs_core::MINUTE_MS; // exclusive
    let expected = tf.minutes();
    let mut flush = |acc: Acc, out: &mut Vec<Candle>, mins: &mut Vec<i64>| {
        let b = acc.c.open_time;
        if acc.minutes == expected {
            st.emitted += 1;
            out.push(acc.c);
            mins.push(acc.minutes);
        } else if b < data_start || b + tf.ms() > data_end {
            st.edge_dropped += 1;
        } else {
            st.incomplete += 1;
            if policy == Policy::AllowGaps {
                st.emitted += 1;
                out.push(acc.c);
                mins.push(acc.minutes);
            }
        }
    };
    let mut acc: Option<Acc> = None;
    for m in m1 {
        let b = tf.bucket(m.open_time);
        match acc.as_mut() {
            Some(a) if a.c.open_time == b => a.push(m),
            _ => {
                if let Some(a) = acc.take() {
                    flush(a, &mut out, &mut mins);
                }
                acc = Some(Acc::new(b, m));
            }
        }
    }
    if let Some(a) = acc {
        flush(a, &mut out, &mut mins);
    }
    (out, mins, st)
}

#[cfg(test)]
mod tests {
    use super::*;
    use cs_core::price_eq;
    use cs_core::time::ymd_hms_to_ms;

    fn m(t: i64, o: f64, h: f64, l: f64, c: f64, v: f64) -> Candle {
        Candle {
            open_time: t,
            open: o,
            high: h,
            low: l,
            close: c,
            volume: v,
            quote_volume: v * c,
            trades: 2,
            taker_buy_base: v / 2.0,
            taker_buy_quote: v * c / 2.0,
        }
    }

    #[allow(clippy::cast_possible_wrap, clippy::cast_precision_loss)]
    /// 10 hand-built minutes from 00:00 UTC: two full 5m buckets.
    fn ten() -> Vec<Candle> {
        let b = ymd_hms_to_ms(2024, 11, 1, 0, 0, 0);
        let px = [
            (100.0, 101.0, 99.0, 100.5),
            (100.5, 102.0, 100.0, 101.5),
            (101.5, 101.8, 98.0, 99.0),
            (99.0, 99.5, 98.5, 99.2),
            (99.2, 100.0, 99.1, 99.9),
            (99.9, 103.0, 99.9, 102.0),
            (102.0, 102.5, 101.0, 101.2),
            (101.2, 101.3, 97.0, 97.5),
            (97.5, 98.0, 97.1, 97.9),
            (97.9, 98.4, 97.8, 98.3),
        ];
        px.iter()
            .enumerate()
            .map(|(i, &(o, h, l, c))| m(b + i as i64 * 60_000, o, h, l, c, 1.0 + i as f64))
            .collect()
    }

    #[test]
    fn five_minute_ohlcv() {
        let (c, mins, st) = resample(&ten(), Timeframe::M5, Policy::Strict);
        assert_eq!(c.len(), 2);
        assert_eq!(mins, vec![5, 5]);
        assert_eq!(
            st,
            ResampleStats {
                emitted: 2,
                incomplete: 0,
                edge_dropped: 0
            }
        );
        let a = c[0];
        assert_eq!(a.open_time, ymd_hms_to_ms(2024, 11, 1, 0, 0, 0));
        assert!(price_eq(a.open, 100.0));
        assert!(price_eq(a.high, 102.0));
        assert!(price_eq(a.low, 98.0));
        assert!(price_eq(a.close, 99.9));
        assert!(price_eq(a.volume, 1.0 + 2.0 + 3.0 + 4.0 + 5.0));
        assert_eq!(a.trades, 10);
        let b = c[1];
        assert_eq!(b.open_time, ymd_hms_to_ms(2024, 11, 1, 0, 5, 0));
        assert!(price_eq(b.open, 99.9));
        assert!(price_eq(b.high, 103.0));
        assert!(price_eq(b.low, 97.0));
        assert!(price_eq(b.close, 98.3));
        assert!(price_eq(b.volume, 6.0 + 7.0 + 8.0 + 9.0 + 10.0));
        assert!(price_eq(b.taker_buy_base, b.volume / 2.0));
    }

    #[test]
    fn no_partial_candles_at_edges() {
        // Drop the first minute and the last two: both 5m buckets are now partial edges.
        let v = ten();
        let (c, _, st) = resample(&v[1..8], Timeframe::M5, Policy::AllowGaps);
        assert_eq!(c.len(), 0);
        assert_eq!(st.edge_dropped, 2);
        // 15m: one bucket that is still open at the end of the data.
        let (c, _, st) = resample(&v, Timeframe::M15, Policy::Strict);
        assert_eq!(c.len(), 0);
        assert_eq!(st.edge_dropped, 1);
    }

    #[test]
    fn internal_gap_policy() {
        let mut v = ten();
        v.extend(ten().into_iter().map(|mut c| {
            c.open_time += 10 * 60_000;
            c
        }));
        v.remove(7); // 00:07 missing -> bucket 00:05 incomplete, inside the data
        let (c, mins, st) = resample(&v, Timeframe::M5, Policy::Strict);
        assert_eq!((c.len(), st.incomplete), (3, 1));
        assert!(mins.iter().all(|&m| m == 5));
        let (c, mins, _) = resample(&v, Timeframe::M5, Policy::AllowGaps);
        assert_eq!(c.len(), 4);
        assert_eq!(mins[1], 4);
    }

    #[test]
    fn utc_alignment_4h_and_1d() {
        // Start at 02:00 UTC: first 4h bucket (00:00) is a partial edge, 04:00 is the first full.
        let start = ymd_hms_to_ms(2024, 11, 1, 2, 0, 0);
        let v: Vec<Candle> = (0..(2 * 1440))
            .map(|i| m(start + i * 60_000, 1.0, 2.0, 0.5, 1.5, 1.0))
            .collect();
        let (c4, ..) = resample(&v, Timeframe::H4, Policy::Strict);
        assert_eq!(c4[0].open_time, ymd_hms_to_ms(2024, 11, 1, 4, 0, 0));
        assert!(c4.iter().all(|c| c.open_time % Timeframe::H4.ms() == 0));
        let (d, mins, st) = resample(&v, Timeframe::D1, Policy::Strict);
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].open_time, ymd_hms_to_ms(2024, 11, 2, 0, 0, 0));
        assert_eq!(mins[0], 1440);
        assert_eq!(st.edge_dropped, 2);
        assert!(price_eq(d[0].volume, 1440.0));
    }
}
