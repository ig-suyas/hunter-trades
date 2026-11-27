//! What the overlay does with a screen reading: follow the symbol and time frame on screen,
//! show "unsupported" for what has no feed, and hide the context when the screen price and the
//! feed price disagree. Pure logic (no window code), so it is unit-tested.

use cs_core::Timeframe;
use cs_vision::read::{PageKind, Reading, SymbolRead};

use crate::copy;

/// What a reading asks the overlay to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Decision {
    /// Switch the feed to this exchange symbol and time frame.
    pub switch: Option<(String, Timeframe)>,
    /// The screen shows something without a feed: this message replaces the context.
    pub block: Option<String>,
}

/// Decides from one reading, given the feed that runs now.
pub fn decide(r: &Reading, cur_sym: &str, cur_tf: Timeframe) -> Decision {
    let none = Decision {
        switch: None,
        block: None,
    };
    if r.page != PageKind::Chart {
        // Pricing page, settings, loading: keep the last feed.
        return none;
    }
    match &r.symbol {
        SymbolRead::Unknown => none,
        SymbolRead::Unsupported { ticker } => Decision {
            switch: None,
            block: Some(copy::screen_unsupported(ticker)),
        },
        SymbolRead::Supported { exchange, .. } => {
            let tf = match &r.tf {
                // Time frame not read this time: keep the current one.
                None => cur_tf,
                Some(t) => match t.tf.filter(|x| cs_live::SUPPORTED_TFS.contains(x)) {
                    Some(x) => x,
                    None => {
                        return Decision {
                            switch: None,
                            block: Some(copy::screen_unsupported_tf(&t.raw)),
                        };
                    }
                },
            };
            let switch = (exchange != cur_sym || tf != cur_tf).then(|| (exchange.clone(), tf));
            Decision {
                switch,
                block: None,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cs_vision::parse::TfRead;
    use cs_vision::read::PriceFrom;

    fn reading(sym: SymbolRead, tf: Option<(&str, Option<Timeframe>)>) -> Reading {
        Reading {
            page: PageKind::Chart,
            symbol: sym,
            tf: tf.map(|(raw, tf)| TfRead {
                raw: raw.into(),
                tf,
            }),
            price: Some((60_626.47, PriceFrom::AxisLabel)),
            source: Some("CRYPTO".into()),
            symbol_from: "symbol region",
            engine: None,
            ocr_calls: 4,
            read_ms: 70.0,
            tf_from: "toolbar",
            tf_note: None,
            held: Vec::new(),
        }
    }

    fn sup(t: &str, x: &str) -> SymbolRead {
        SymbolRead::Supported {
            ticker: t.into(),
            exchange: x.into(),
        }
    }

    #[test]
    fn follows_symbol_and_time_frame() {
        let r = reading(sup("ETHUSD", "ETHUSDT"), Some(("5", Some(Timeframe::M5))));
        let d = decide(&r, "BTCUSDT", Timeframe::M15);
        assert_eq!(d.switch, Some(("ETHUSDT".into(), Timeframe::M5)));
        assert!(d.block.is_none());
        // Already on it: nothing to do.
        assert_eq!(decide(&r, "ETHUSDT", Timeframe::M5).switch, None);
        // Time frame not read: symbol follows, time frame stays.
        let r = reading(sup("ETHUSD", "ETHUSDT"), None);
        assert_eq!(
            decide(&r, "BTCUSDT", Timeframe::H1).switch,
            Some(("ETHUSDT".into(), Timeframe::H1))
        );
    }

    #[test]
    fn unsupported_symbol_or_time_frame_blocks_context() {
        let r = reading(
            SymbolRead::Unsupported {
                ticker: "XAUUSD".into(),
            },
            Some(("4h", Some(Timeframe::H4))),
        );
        let d = decide(&r, "BTCUSDT", Timeframe::M15);
        assert!(d.switch.is_none());
        assert!(d.block.unwrap().contains("XAUUSD"));
        for (raw, tf, words) in [
            (
                "D",
                Some(Timeframe::D1),
                "Unsupported time frame: D (one day)",
            ),
            (
                "1D",
                Some(Timeframe::D1),
                "Unsupported time frame: 1D (one day)",
            ),
            (
                "4h",
                Some(Timeframe::H4),
                "Unsupported time frame: 4h (4 hours)",
            ),
            (
                "1",
                Some(Timeframe::M1),
                "Unsupported time frame: 1 (1-minute candles)",
            ),
            ("3", None, "Unsupported time frame: 3 (3-minute candles)"),
            ("1W", None, "Unsupported time frame: 1W (one week)"),
        ] {
            let r = reading(sup("BTCUSD", "BTCUSDT"), Some((raw, tf)));
            let d = decide(&r, "BTCUSDT", Timeframe::M15);
            assert!(d.switch.is_none(), "{raw}");
            let b = d.block.unwrap();
            assert!(b.starts_with(words), "{raw}: {b}");
            assert!(b.contains("5m, 15m and 1h only"), "{b}");
        }
    }

    #[test]
    fn the_overlay_never_uses_the_diagnostic_source() {
        for (name, src) in [
            ("app.rs", include_str!("app.rs")),
            ("main.rs", include_str!("main.rs")),
            ("view.rs", include_str!("view.rs")),
        ] {
            assert!(!src.contains("DiagTitleSource"), "{name}");
            assert!(!src.contains("diag-by-title"), "{name}");
        }
        assert!(include_str!("app.rs").contains("XcapSource::new()"));
    }

    #[test]
    fn other_pages_and_unknown_symbols_keep_the_feed() {
        let mut r = reading(sup("ETHUSD", "ETHUSDT"), Some(("5", Some(Timeframe::M5))));
        r.page = PageKind::Other;
        assert_eq!(
            decide(&r, "BTCUSDT", Timeframe::M15),
            Decision {
                switch: None,
                block: None
            }
        );
        let r = reading(SymbolRead::Unknown, Some(("5", Some(Timeframe::M5))));
        assert!(decide(&r, "BTCUSDT", Timeframe::M15).switch.is_none());
    }
}
