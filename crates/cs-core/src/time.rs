//! UTC calendar helpers on `i64` milliseconds (no time-zone database; everything is UTC).

const DAY_MS: i64 = 86_400_000;

/// Days since 1970-01-01 for a proleptic Gregorian date (H. Hinnant's algorithm).
pub const fn days_from_civil(y: i64, m: i64, d: i64) -> i64 {
    let y = if m <= 2 { y - 1 } else { y };
    let era = y.div_euclid(400);
    let yoe = y - era * 400;
    let mp = (m + 9) % 12;
    let doy = (153 * mp + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    era * 146_097 + doe - 719_468
}

/// (year, month, day) for days since 1970-01-01.
pub const fn civil_from_days(z: i64) -> (i64, i64, i64) {
    let z = z + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    (if m <= 2 { y + 1 } else { y }, m, d)
}

pub const fn ymd_hms_to_ms(y: i64, mo: i64, d: i64, h: i64, mi: i64, s: i64) -> i64 {
    days_from_civil(y, mo, d) * DAY_MS + ((h * 60 + mi) * 60 + s) * 1000
}

/// Start of the UTC day containing `t`.
pub const fn day_start(t: i64) -> i64 {
    t.div_euclid(DAY_MS) * DAY_MS
}

/// (year, month, day, hour, minute, second) of `t`.
pub const fn split(t: i64) -> (i64, i64, i64, i64, i64, i64) {
    let days = t.div_euclid(DAY_MS);
    let rem = t.rem_euclid(DAY_MS) / 1000;
    let (y, m, d) = civil_from_days(days);
    (y, m, d, rem / 3600, (rem / 60) % 60, rem % 60)
}

/// `YYYY-MM-DD HH:MM` (UTC).
pub fn fmt_minute(t: i64) -> String {
    let (y, m, d, h, mi, _) = split(t);
    format!("{y:04}-{m:02}-{d:02} {h:02}:{mi:02}")
}

/// `YYYY-MM-DD HH:MM:SS` (UTC).
pub fn fmt_second(t: i64) -> String {
    let (y, m, d, h, mi, s) = split(t);
    format!("{y:04}-{m:02}-{d:02} {h:02}:{mi:02}:{s:02}")
}

/// `YYYY-MM-DD` (UTC).
pub fn fmt_date(t: i64) -> String {
    let (y, m, d, ..) = split(t);
    format!("{y:04}-{m:02}-{d:02}")
}

/// Parse `YYYY-MM-DD` as UTC midnight.
pub fn parse_date(s: &str) -> Option<i64> {
    let mut it = s.trim().split('-');
    let y: i64 = it.next()?.parse().ok()?;
    let m: i64 = it.next()?.parse().ok()?;
    let d: i64 = it.next()?.parse().ok()?;
    if it.next().is_some() || !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return None;
    }
    Some(ymd_hms_to_ms(y, m, d, 0, 0, 0))
}

/// Parse `YYYY-MM-DD HH:MM:SS` (treated as UTC).
pub fn parse_datetime(s: &str) -> Option<i64> {
    let s = s.trim();
    let (date, time) = s.split_once([' ', 'T'])?;
    let base = parse_date(date)?;
    let mut it = time.split(':');
    let h: i64 = it.next()?.parse().ok()?;
    let mi: i64 = it.next()?.parse().ok()?;
    let sec: i64 = it.next().unwrap_or("0").parse().ok()?;
    if h > 23 || mi > 59 || sec > 60 {
        return None;
    }
    Some(base + ((h * 60 + mi) * 60 + sec) * 1000)
}

/// Start of the month (UTC) for (year, month).
pub const fn month_start(y: i64, m: i64) -> i64 {
    ymd_hms_to_ms(y, m, 1, 0, 0, 0)
}

/// Next (year, month).
pub const fn next_month(y: i64, m: i64) -> (i64, i64) {
    if m == 12 { (y + 1, 1) } else { (y, m + 1) }
}

/// Current wall-clock time in UTC ms.
pub fn now_ms() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| i64::try_from(d.as_millis()).unwrap_or(i64::MAX))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip() {
        for days in [-1000, 0, 1, 19_000, 20_418, 40_000] {
            let (y, m, d) = civil_from_days(days);
            assert_eq!(days_from_civil(y, m, d), days);
        }
        // Binance BTCUSDT 1m first candle of 2024-11 = 1730419200000
        assert_eq!(ymd_hms_to_ms(2024, 11, 1, 0, 0, 0), 1_730_419_200_000);
        assert_eq!(fmt_second(1_730_419_200_000), "2024-11-01 00:00:00");
        assert_eq!(
            parse_datetime("2024-11-01 00:05:00"),
            Some(1_730_419_500_000)
        );
        assert_eq!(parse_date("2024-02-30x"), None);
        assert_eq!(next_month(2024, 12), (2025, 1));
    }
}
