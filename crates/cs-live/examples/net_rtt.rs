//! Network round trip to the exchange (public `GET /fapi/v1/time`, 20 requests) and the local
//! clock offset, reported separately from the per-candle evaluation benchmark.

#![allow(
    clippy::print_stdout,
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]

use std::time::Duration;

use cs_data::http::Client;
use cs_live::feed::measure_drift;

fn main() {
    let mut c = Client::new(Duration::from_millis(300));
    // The first request also pays DNS and the TLS handshake; it is kept and shows as the max.
    let mut rtt = Vec::new();
    let mut off = Vec::new();
    for _ in 0..20 {
        // Wait out the client's minimum request gap so it is not timed as network time.
        std::thread::sleep(Duration::from_millis(600));
        match measure_drift(&mut c, "https://fapi.binance.com") {
            Ok(d) => {
                rtt.push(d.rtt_ms);
                off.push(d.offset_ms);
            }
            Err(e) => println!("request failed: {e}"),
        }
    }
    if rtt.is_empty() {
        println!("no successful request");
        return;
    }
    rtt.sort_unstable();
    off.sort_unstable();
    let q = |v: &[i64], p: f64| v[((v.len() as f64 - 1.0) * p).round() as usize];
    println!(
        "network round trip (GET /fapi/v1/time, {} ok of 20, first one includes the TLS handshake): p50 {} ms, p99 {} ms, min {} ms, max {} ms; local clock offset (exchange - local) median {} ms",
        rtt.len(),
        q(&rtt, 0.5),
        q(&rtt, 0.99),
        rtt[0],
        rtt[rtt.len() - 1],
        q(&off, 0.5)
    );
}
