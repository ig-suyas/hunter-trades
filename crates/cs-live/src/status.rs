//! Feed health: live / stale / disconnected, and the local clock against the exchange clock.

use std::time::Duration;

/// Feed state shown in the panel header.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeedStatus {
    /// Warming up or connecting for the first time.
    Starting,
    /// Frames arriving and the latest candle closed on time.
    Live,
    /// Connected, but no frame for longer than the stale limit, or the next closed candle is
    /// overdue.
    Stale,
    /// Not connected (reconnecting with backoff).
    Disconnected,
}

impl FeedStatus {
    pub const fn label(self) -> &'static str {
        match self {
            FeedStatus::Starting => "STARTING",
            FeedStatus::Live => "LIVE",
            FeedStatus::Stale => "FEED STALE",
            FeedStatus::Disconnected => "DISCONNECTED",
        }
    }
}

/// Grace after a candle's close before its final frame counts as overdue.
pub const CLOSE_GRACE_MS: i64 = 90_000;

/// Local clock offset above which the panel warns.
pub const DRIFT_WARN_MS: i64 = 1_000;

/// Classify the feed.
///
/// * `last_msg_age`: time since the last frame of any kind (`None`: none yet).
/// * `next_close_due`: exchange time (ms) by which the next closed candle should have arrived
///   (last applied candle's close + one candle + grace), and `exchange_now` the current exchange
///   time estimate.
pub fn classify(
    connected: bool,
    ever_connected: bool,
    last_msg_age: Option<Duration>,
    stale_after: Duration,
    next_close_due: Option<i64>,
    exchange_now: i64,
) -> FeedStatus {
    if !connected {
        return if ever_connected {
            FeedStatus::Disconnected
        } else {
            FeedStatus::Starting
        };
    }
    let silent = last_msg_age.is_none_or(|a| a > stale_after);
    let overdue = next_close_due.is_some_and(|d| exchange_now > d);
    if silent || overdue {
        FeedStatus::Stale
    } else {
        FeedStatus::Live
    }
}

/// One exchange-time measurement (`GET /fapi/v1/time`).
#[derive(Debug, Clone, Copy)]
pub struct Drift {
    /// Exchange clock minus local clock, in ms (positive: the local clock is behind).
    pub offset_ms: i64,
    /// Round trip of the request, in ms.
    pub rtt_ms: i64,
    /// Local time of the measurement (UTC ms).
    pub at_ms: i64,
}

impl Drift {
    /// NTP-style estimate: the server stamped `server_ms` halfway through the round trip.
    pub fn from_times(sent_ms: i64, server_ms: i64, recv_ms: i64) -> Drift {
        Drift {
            offset_ms: server_ms - i64::midpoint(sent_ms, recv_ms),
            rtt_ms: recv_ms - sent_ms,
            at_ms: recv_ms,
        }
    }

    pub fn warn(&self) -> bool {
        self.offset_ms.abs() > DRIFT_WARN_MS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn states() {
        let s = Duration::from_secs(15);
        let a = Some(Duration::from_secs(2));
        assert_eq!(
            classify(false, false, None, s, None, 0),
            FeedStatus::Starting
        );
        assert_eq!(
            classify(false, true, a, s, None, 0),
            FeedStatus::Disconnected
        );
        assert_eq!(classify(true, true, a, s, Some(100), 50), FeedStatus::Live);
        assert_eq!(
            classify(true, true, Some(Duration::from_secs(16)), s, Some(100), 50),
            FeedStatus::Stale
        );
        // Frames still flowing (mark price) but the closed candle is overdue.
        assert_eq!(
            classify(true, true, a, s, Some(100), 101),
            FeedStatus::Stale
        );
        assert_eq!(classify(true, true, None, s, None, 0), FeedStatus::Stale);
    }

    #[test]
    fn drift() {
        let d = Drift::from_times(1_000, 2_600, 1_200);
        assert_eq!(d.offset_ms, 1_500);
        assert_eq!(d.rtt_ms, 200);
        assert!(d.warn());
        assert!(!Drift::from_times(0, 150, 200).warn());
    }
}
