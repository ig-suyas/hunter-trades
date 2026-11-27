//! cs-data: downloaders, validators, storage, resampling and the live recorder.
//!
//! Advisory only: this crate talks to public market-data endpoints only. It has no order
//! endpoints, no signed requests and no key fields, and it uploads nothing.
//!
//! Every fallible function returns [`Error`]: I/O, HTTP, checksum, format or parse failures.

#![allow(clippy::missing_errors_doc)]

pub mod bybit;
pub mod clean;
pub mod fetch;
pub mod http;
pub mod parse;
pub mod record;
pub mod resample;
pub mod store;
pub mod validate;
pub mod vision;
pub mod zipread;

use std::fmt;
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub enum Error {
    Io(std::io::Error),
    Http(String),
    /// HTTP status that is not retried (404, 403, 451 ...).
    Status(u16, String),
    Checksum(String),
    Format(String),
    Parse(String),
    Other(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Io(e) => write!(f, "io: {e}"),
            Error::Http(e) => write!(f, "http: {e}"),
            Error::Status(s, u) => write!(f, "http status {s} for {u}"),
            Error::Checksum(e) => write!(f, "checksum: {e}"),
            Error::Format(e) => write!(f, "format: {e}"),
            Error::Parse(e) => write!(f, "parse: {e}"),
            Error::Other(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for Error {}

impl From<std::io::Error> for Error {
    fn from(e: std::io::Error) -> Self {
        Error::Io(e)
    }
}

pub type Result<T> = std::result::Result<T, Error>;

/// Markets fetched from Binance.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub enum Market {
    Spot,
    /// USDS-M perpetual futures.
    Um,
}

impl Market {
    pub const ALL: [Market; 2] = [Market::Spot, Market::Um];

    pub const fn dir(self) -> &'static str {
        match self {
            Market::Spot => "spot",
            Market::Um => "um",
        }
    }

    pub fn parse(s: &str) -> Option<Market> {
        match s {
            "spot" => Some(Market::Spot),
            "um" | "futures" | "perp" => Some(Market::Um),
            _ => None,
        }
    }
}

/// Paths under the data directory.
#[derive(Debug, Clone)]
pub struct DataDir {
    pub root: PathBuf,
}

impl DataDir {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        DataDir { root: root.into() }
    }

    pub fn raw(&self) -> PathBuf {
        self.root.join("raw")
    }

    pub fn store(&self) -> PathBuf {
        self.root.join("store")
    }

    /// `store/klines/<market>/<SYMBOL>/1m`
    pub fn klines_1m(&self, m: Market, sym: &str) -> PathBuf {
        self.store()
            .join("klines")
            .join(m.dir())
            .join(sym)
            .join("1m")
    }

    /// `store/resampled/<market>/<SYMBOL>/<tf>.csc`
    pub fn resampled(&self, m: Market, sym: &str, tf: &str) -> PathBuf {
        self.store()
            .join("resampled")
            .join(m.dir())
            .join(sym)
            .join(format!("{tf}.csc"))
    }

    pub fn funding(&self, sym: &str) -> PathBuf {
        self.store().join("funding").join("um").join(sym)
    }

    pub fn metrics(&self, sym: &str) -> PathBuf {
        self.store().join("metrics").join("um").join(sym)
    }

    pub fn book_depth(&self, sym: &str) -> PathBuf {
        self.store().join("bookdepth").join("um").join(sym)
    }

    pub fn live(&self) -> PathBuf {
        self.root.join("live")
    }
}

/// All `*.csc` files in a directory, sorted by file name.
pub fn list_csc(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut v = Vec::new();
    if !dir.exists() {
        return Ok(v);
    }
    for e in std::fs::read_dir(dir)? {
        let p = e?.path();
        if p.extension().is_some_and(|x| x == "csc") {
            v.push(p);
        }
    }
    v.sort();
    Ok(v)
}

/// Small logger used by long commands: writes to stderr (never to the network).
pub fn log(msg: &str) {
    eprintln!(
        "[{}] {msg}",
        cs_core::time::fmt_second(cs_core::time::now_ms())
    );
}
