//! cs-backtest: event-driven simulator that turns detector events into trades, with costs,
//! pessimistic fills and metrics.
//!
//! Pipeline: [`market::Market::prepare`] (candles + context features + levels + signal events)
//! -> [`market::audit_prefix`] (lookahead guard: everything known at candle `k` must be the same
//! when the data end at `k`) -> [`sim::simulate`] for each [`rule::TradeRule`] ->
//! [`metrics::summarize`] -> [`output`] (trades CSV, summary JSON, hashes). [`backtest`] runs the
//! last three steps.
//!
//! Advisory only: nothing in this workspace places orders.

// Candle indices are u32 by the event contract (series far below 2^32 candles); counts shown as
// f64 rates are far below 2^52.
#![allow(
    clippy::cast_precision_loss,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::cast_sign_loss,
    clippy::missing_errors_doc,
    clippy::too_many_lines
)]

pub mod cost;
pub mod market;
pub mod metrics;
pub mod output;
pub mod rule;
pub mod sim;
pub mod testkit;

use std::fmt;

pub use cost::{CostModel, Funding};
pub use market::{Market, SignalSource};
pub use metrics::{BootCfg, Summary};
pub use rule::TradeRule;
pub use sim::{Inputs, Run, Sizing, Trade};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// A decision would use data from after its decision time (or an entry before it).
    Lookahead(String),
    /// Malformed inputs.
    Input(String),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::Lookahead(s) => write!(f, "lookahead: {s}"),
            Error::Input(s) => write!(f, "input: {s}"),
        }
    }
}

impl std::error::Error for Error {}

/// One finished rule run: trades, summary and hashes.
#[derive(Debug, Clone)]
pub struct Outcome {
    pub run: Run,
    pub summary: Summary,
    pub inputs_hash: String,
    pub results_hash: String,
}

/// Simulate, summarise and hash one rule. `market_fp` / `data_fp` come from
/// [`output::market_fingerprint`] / [`output::data_fingerprint`] (computed once per data set).
pub fn backtest(
    inp: &Inputs<'_>,
    rule: &TradeRule,
    boot: BootCfg,
    market_fp: &str,
    data_fp: &str,
) -> Result<Outcome, Error> {
    let run = sim::simulate(inp, rule)?;
    let summary = metrics::summarize(&run, inp.market.tf_ms, boot);
    let results_hash = output::results_hash(&run, &summary);
    let inputs_hash = output::inputs_hash(market_fp, data_fp, &run, boot);
    Ok(Outcome {
        run,
        summary,
        inputs_hash,
        results_hash,
    })
}
