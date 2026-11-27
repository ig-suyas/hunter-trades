//! Polite blocking HTTP client for public market-data endpoints.
//!
//! * a minimum gap between requests (politeness delay),
//! * retries with exponential backoff on network errors and 5xx,
//! * honours `Retry-After` on 429 and 418 (Binance's IP ban response) and backs off,
//! * watches Binance's `X-MBX-USED-WEIGHT-1M` and Bybit's `X-Bapi-Limit-Status` headers and
//!   slows down when the budget runs low,
//! * resumable downloads to `<file>.part` using HTTP `Range`.
//!
//! GET only. No keys, no signed requests, nothing uploaded.

use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::Path;
use std::thread::sleep;
use std::time::{Duration, Instant};

use ureq::Agent;
use ureq::tls::{TlsConfig, TlsProvider};

use crate::{Error, Result, log};

/// Generic client name (no personal details).
pub const USER_AGENT: &str = "candlescope-data/0.1";

pub struct Client {
    agent: Agent,
    min_gap: Duration,
    last: Option<Instant>,
    pub max_retries: u32,
    pub requests: u64,
    pub retries: u64,
    pub throttled: u64,
}

pub enum Fetched {
    Body(Vec<u8>),
    NotFound,
}

fn header<'a>(r: &'a ureq::http::Response<ureq::Body>, name: &str) -> Option<&'a str> {
    r.headers().get(name).and_then(|v| v.to_str().ok())
}

impl Client {
    pub fn new(min_gap: Duration) -> Client {
        let agent: Agent = Agent::config_builder()
            .tls_config(
                TlsConfig::builder()
                    .provider(TlsProvider::NativeTls)
                    .build(),
            )
            .http_status_as_error(false)
            .user_agent(USER_AGENT)
            .timeout_connect(Some(Duration::from_secs(20)))
            .timeout_global(Some(Duration::from_secs(600)))
            .build()
            .into();
        Client {
            agent,
            min_gap,
            last: None,
            max_retries: 6,
            requests: 0,
            retries: 0,
            throttled: 0,
        }
    }

    fn pace(&mut self) {
        if let Some(last) = self.last {
            let el = last.elapsed();
            if el < self.min_gap {
                sleep(self.min_gap.saturating_sub(el));
            }
        }
        self.last = Some(Instant::now());
    }

    fn backoff(attempt: u32) -> Duration {
        Duration::from_millis(500u64.saturating_mul(1u64 << attempt.min(7)).min(60_000))
    }

    /// Slow down when an exchange reports that most of the request budget is used.
    fn watch_budget(&mut self, r: &ureq::http::Response<ureq::Body>) {
        if let Some(w) = header(r, "x-mbx-used-weight-1m").and_then(|v| v.parse::<u32>().ok()) {
            // Futures budget is 2400/min, spot 6000/min: stay under ~half of the smaller one.
            if w > 1200 {
                self.throttled += 1;
                log(&format!("binance weight {w}/min: pausing 30 s"));
                sleep(Duration::from_secs(30));
            }
        }
        if let Some(rem) = header(r, "x-bapi-limit-status").and_then(|v| v.parse::<u32>().ok())
            && rem < 5
        {
            self.throttled += 1;
            let wait = header(r, "x-bapi-limit-reset-timestamp")
                .and_then(|v| v.parse::<i64>().ok())
                .map_or(1000, |t| (t - cs_core::time::now_ms()).clamp(100, 10_000));
            sleep(Duration::from_millis(u64::try_from(wait).unwrap_or(1000)));
        }
    }

    /// Run a GET with retry. `range_from` sets a `Range: bytes=N-` header.
    fn get_resp(
        &mut self,
        url: &str,
        range_from: Option<u64>,
    ) -> Result<ureq::http::Response<ureq::Body>> {
        let mut attempt = 0;
        loop {
            self.pace();
            self.requests += 1;
            let mut req = self.agent.get(url);
            if let Some(n) = range_from {
                req = req.header("Range", &format!("bytes={n}-"));
            }
            let res = req.call();
            let retry_wait = match res {
                Ok(r) => {
                    let s = r.status().as_u16();
                    self.watch_budget(&r);
                    match s {
                        200..=299 => return Ok(r),
                        429 | 418 => {
                            self.throttled += 1;
                            let ra = header(&r, "retry-after")
                                .and_then(|v| v.trim().parse::<u64>().ok())
                                .unwrap_or(60);
                            log(&format!("{s} rate limited on {url}: waiting {ra} s"));
                            Some(Duration::from_secs(ra.clamp(1, 600)))
                        }
                        500..=599 => Some(Self::backoff(attempt)),
                        _ => return Err(Error::Status(s, url.to_string())),
                    }
                }
                Err(e) => {
                    if attempt >= self.max_retries {
                        return Err(Error::Http(format!("{url}: {e}")));
                    }
                    log(&format!("network error on {url}: {e}; retrying"));
                    Some(Self::backoff(attempt))
                }
            };
            if attempt >= self.max_retries {
                return Err(Error::Http(format!(
                    "{url}: gave up after {attempt} retries"
                )));
            }
            attempt += 1;
            self.retries += 1;
            if let Some(w) = retry_wait {
                sleep(w);
            }
        }
    }

    /// GET a small body into memory. 404 is returned as `Fetched::NotFound`.
    pub fn get(&mut self, url: &str) -> Result<Fetched> {
        match self.get_resp(url, None) {
            Ok(mut r) => {
                let mut v = Vec::new();
                r.body_mut()
                    .with_config()
                    .limit(256 * 1024 * 1024)
                    .reader()
                    .read_to_end(&mut v)
                    .map_err(|e| Error::Http(format!("{url}: body: {e}")))?;
                Ok(Fetched::Body(v))
            }
            Err(Error::Status(404, _)) => Ok(Fetched::NotFound),
            Err(e) => Err(e),
        }
    }

    /// GET JSON text.
    pub fn get_json(&mut self, url: &str) -> Result<serde_json::Value> {
        match self.get(url)? {
            Fetched::Body(b) => {
                serde_json::from_slice(&b).map_err(|e| Error::Parse(format!("{url}: json: {e}")))
            }
            Fetched::NotFound => Err(Error::Status(404, url.to_string())),
        }
    }

    /// Download `url` to `dest`, resuming a previous `<dest>.part` with `Range` when the server
    /// supports it. Returns `Ok(false)` on 404.
    pub fn download(&mut self, url: &str, dest: &Path) -> Result<bool> {
        if let Some(dir) = dest.parent() {
            fs::create_dir_all(dir)?;
        }
        let part = dest.with_extension(format!(
            "{}.part",
            dest.extension().and_then(|e| e.to_str()).unwrap_or("bin")
        ));
        let have = fs::metadata(&part).map_or(0, |m| m.len());
        let resp = match self.get_resp(url, (have > 0).then_some(have)) {
            Ok(r) => r,
            Err(Error::Status(404, _)) => return Ok(false),
            // 416 = the part file is already complete (or bigger than the file): restart.
            Err(Error::Status(416, _)) => {
                fs::remove_file(&part)?;
                return self.download(url, dest);
            }
            Err(e) => return Err(e),
        };
        let append = resp.status().as_u16() == 206;
        let mut resp = resp;
        let mut f = OpenOptions::new()
            .create(true)
            .write(true)
            .append(append)
            .truncate(!append)
            .open(&part)?;
        let mut reader = resp
            .body_mut()
            .with_config()
            .limit(4 * 1024 * 1024 * 1024)
            .reader();
        let mut buf = vec![0u8; 256 * 1024];
        loop {
            let n = reader
                .read(&mut buf)
                .map_err(|e| Error::Http(format!("{url}: body: {e}")))?;
            if n == 0 {
                break;
            }
            f.write_all(&buf[..n])?;
        }
        f.sync_all()?;
        drop(f);
        fs::rename(&part, dest)?;
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn backoff_grows_and_caps() {
        assert_eq!(Client::backoff(0), Duration::from_millis(500));
        assert_eq!(Client::backoff(1), Duration::from_millis(1000));
        assert_eq!(Client::backoff(20), Duration::from_secs(60));
    }
}
