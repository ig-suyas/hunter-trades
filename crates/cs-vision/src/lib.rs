//! cs-vision: screen awareness (Phase 7). Notices which symbol, time frame and last price a
//! trading platform shows, so the overlay can follow it with the matching exchange feed.
//!
//! It never reads candles from pixels: the exchange feed stays the only data source. It only
//! reads four small regions (symbol, legend with the time frame, page type, last-price label).
//!
//! Privacy: frames are captured only from the foreground window whose title matches a known
//! platform, held in memory, cropped to the configured regions and wiped; nothing is saved or
//! sent anywhere. Logs hold parsed text only (images only with the opt-in debug flag).

pub mod config;
pub mod frame;
pub mod handshake;
pub mod ocr;
pub mod parse;
pub mod privacy;
pub mod read;
pub mod source;
pub mod stable;
pub mod video;
pub mod watch;

pub use config::VisionConfig;
pub use frame::{Frame, RectF};
pub use handshake::{Handshake, check_price};
pub use read::{PageKind, Reading, SymbolRead};
pub use watch::{WatchState, Watcher};
