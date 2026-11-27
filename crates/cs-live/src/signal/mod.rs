//! Signal view (user's request after the GATE): the 10 pre-registered exploratory rules, which
//! all **failed** testing, run live through the backtest's own code, with their past results
//! next to every alert. It never places orders and holds no keys.
//!
//! * [`rules`]: the frozen list and its trade templates.
//! * [`engine`]: [`engine::SignalEngine`], closed candles in, append-only alerts out.
//! * [`stats`]: the rules' past results from `reports/rule-stats.json` (discovery and
//!   validation periods only).
//! * [`sizing`]: position size for the user's account and risk %.

pub mod engine;
pub mod rules;
pub mod sizing;
pub mod stats;

pub use engine::{Signal, SignalEngine, TradeState};
pub use rules::{RULES, RuleDef};
