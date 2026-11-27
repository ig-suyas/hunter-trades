//! Binance Vision archive (`data.binance.vision`): URLs and SHA-256 checksum checks.

use std::fs;
use std::path::{Path, PathBuf};

use sha2::{Digest, Sha256};

use crate::{Error, Market, Result};

pub const BASE: &str = "https://data.binance.vision/data";

/// Archive datasets used by the project.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Dataset {
    Klines1m(Market),
    Funding,
    Metrics,
    BookDepth,
}

impl Dataset {
    pub fn name(self) -> &'static str {
        match self {
            Dataset::Klines1m(Market::Spot) => "spot klines 1m",
            Dataset::Klines1m(Market::Um) => "um klines 1m",
            Dataset::Funding => "um fundingRate",
            Dataset::Metrics => "um metrics (5m OI)",
            Dataset::BookDepth => "um bookDepth",
        }
    }
}

/// Monthly or daily period of one archive file: "YYYY-MM" or "YYYY-MM-DD".
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum Period {
    Month(i64, i64),
    Day(i64, i64, i64),
}

impl Period {
    pub fn label(&self) -> String {
        match self {
            Period::Month(y, m) => format!("{y:04}-{m:02}"),
            Period::Day(y, m, d) => format!("{y:04}-{m:02}-{d:02}"),
        }
    }

    /// [start, end) in UTC ms.
    pub fn range(&self) -> (i64, i64) {
        use cs_core::time::{month_start, next_month, ymd_hms_to_ms};
        match *self {
            Period::Month(y, m) => {
                let (ny, nm) = next_month(y, m);
                (month_start(y, m), month_start(ny, nm))
            }
            Period::Day(y, m, d) => {
                let s = ymd_hms_to_ms(y, m, d, 0, 0, 0);
                (s, s + 86_400_000)
            }
        }
    }

    fn freq(&self) -> &'static str {
        match self {
            Period::Month(..) => "monthly",
            Period::Day(..) => "daily",
        }
    }
}

/// Path under `data/` of the archive, e.g. `futures/um/monthly/klines/BTCUSDT/1m/BTCUSDT-1m-2024-11.zip`.
pub fn rel_path(ds: Dataset, sym: &str, p: &Period) -> String {
    let (f, l) = (p.freq(), p.label());
    match ds {
        Dataset::Klines1m(Market::Spot) => format!("spot/{f}/klines/{sym}/1m/{sym}-1m-{l}.zip"),
        Dataset::Klines1m(Market::Um) => {
            format!("futures/um/{f}/klines/{sym}/1m/{sym}-1m-{l}.zip")
        }
        Dataset::Funding => format!("futures/um/{f}/fundingRate/{sym}/{sym}-fundingRate-{l}.zip"),
        Dataset::Metrics => format!("futures/um/{f}/metrics/{sym}/{sym}-metrics-{l}.zip"),
        Dataset::BookDepth => format!("futures/um/{f}/bookDepth/{sym}/{sym}-bookDepth-{l}.zip"),
    }
}

pub fn url(rel: &str) -> String {
    format!("{BASE}/{rel}")
}

pub fn raw_path(raw_root: &Path, rel: &str) -> PathBuf {
    raw_root.join("binance-vision").join(rel)
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let digest = Sha256::digest(bytes);
    let mut s = String::with_capacity(64);
    for b in &digest {
        s.push(char::from(HEX[usize::from(b >> 4)]));
        s.push(char::from(HEX[usize::from(b & 15)]));
    }
    s
}

/// Parse a `.CHECKSUM` file: `<64 hex>  <file name>`.
pub fn parse_checksum(text: &str) -> Result<String> {
    let h = text
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    if h.len() == 64 && h.bytes().all(|b| b.is_ascii_hexdigit()) {
        Ok(h)
    } else {
        Err(Error::Checksum(format!(
            "bad CHECKSUM file content {text:?}"
        )))
    }
}

/// Verify the zip at `zip` against `<zip>.CHECKSUM`. Returns the hex digest.
pub fn verify(zip: &Path) -> Result<String> {
    let ck = zip.with_extension("zip.CHECKSUM");
    let expected = parse_checksum(&fs::read_to_string(&ck)?)?;
    let got = sha256_hex(&fs::read(zip)?);
    if got == expected {
        Ok(got)
    } else {
        Err(Error::Checksum(format!(
            "{}: sha256 {got} != expected {expected}",
            zip.display()
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paths() {
        let p = Period::Month(2024, 11);
        assert_eq!(
            rel_path(Dataset::Klines1m(Market::Um), "BTCUSDT", &p),
            "futures/um/monthly/klines/BTCUSDT/1m/BTCUSDT-1m-2024-11.zip"
        );
        assert_eq!(
            rel_path(Dataset::Metrics, "ETHUSDT", &Period::Day(2025, 1, 2)),
            "futures/um/daily/metrics/ETHUSDT/ETHUSDT-metrics-2025-01-02.zip"
        );
        assert_eq!(p.range().1 - p.range().0, 30 * 86_400_000);
    }

    #[test]
    fn checksum_parse_and_hash() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        let line = "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad  x.zip\n";
        assert!(parse_checksum(line).is_ok());
        assert!(parse_checksum("<html>").is_err());
    }
}
