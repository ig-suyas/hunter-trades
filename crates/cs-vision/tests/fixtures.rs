//! Real frames from trading1.mp4 (`TradingView`, BTCUSD 15m, light theme, 1920 x 1080): page type,
//! symbol, time frame and last price per frame, the price handshake against the Binance feed,
//! and unsupported symbols. `cargo test -p cs-vision --test fixtures -- --nocapture` prints the
//! table.

#![allow(clippy::print_stdout, clippy::unwrap_used, clippy::expect_used)]

mod common;

use cs_core::Timeframe;
use cs_vision::handshake::{Handshake, check_price};
use cs_vision::ocr::{EngineKind, Engines};
use cs_vision::read::{PageKind, PriceFrom, Reader, SymbolRead};
use cs_vision::{VisionConfig, privacy};

/// (time in the video ms, expected last-price label or None where no reading is expected).
/// The expected prices were read by Windows OCR and confirmed by Tesseract on 16 frames; the
/// 6 frames without a price have the label hidden or unreadable (the reader then gives none,
/// never a guess).
const CHART: [(u64, Option<f64>); 24] = [
    (0, Some(60_627.45)),
    (10_000, Some(60_623.13)),
    (20_000, None),
    (30_000, Some(60_619.31)),
    (40_000, None),
    (50_000, Some(60_606.04)),
    (60_000, Some(60_607.34)),
    (70_000, Some(60_629.63)),
    (80_000, Some(60_640.00)),
    (110_000, Some(60_653.66)),
    (120_000, Some(60_653.90)),
    (130_000, None),
    (140_000, Some(60_655.78)),
    (150_000, None),
    (160_000, Some(60_657.43)),
    (170_000, None),
    (180_000, Some(60_657.03)),
    (190_000, Some(60_644.95)),
    (200_000, Some(60_632.14)),
    (210_000, Some(60_632.78)),
    (220_000, Some(60_639.52)),
    (230_000, None),
    (240_000, Some(60_639.57)),
    (250_000, Some(60_636.40)),
];

/// `TradingView`'s pricing page (90-100 s) and the page at the end of the video.
const OTHER: [u64; 3] = [90_000, 100_000, 260_000];

#[test]
fn trading1_frames() {
    if common::video("trading1.mp4").is_none() {
        return;
    }
    let cfg = VisionConfig::default_config().unwrap();
    let tol = cfg.price_tolerance_pct;
    let mut r = Reader::new(cfg, Engines::detect());
    assert!(r.engines.primary.is_some(), "Windows OCR must be available");
    let (mut prices, mut slow) = (0, 0.0f64);
    println!("t(s)   page   symbol             tf   price      feed      diff %  handshake  ms");
    for (t, want) in CHART {
        let mut f = common::trading1(t).unwrap();
        r.reset();
        let x = r.read(&f, None);
        f.wipe();
        assert_eq!(x.engine, Some(EngineKind::Windows));
        assert_eq!(x.page, PageKind::Chart, "t={t}");
        assert_eq!(
            x.symbol,
            SymbolRead::Supported {
                ticker: "BTCUSD".into(),
                exchange: "BTCUSDT".into()
            },
            "t={t}"
        );
        let tf = x.tf.as_ref().expect("time frame");
        assert_eq!(
            (tf.raw.as_str(), tf.tf),
            ("15", Some(Timeframe::M15)),
            "t={t}"
        );
        assert_eq!(x.source.as_deref(), Some("CRYPTO"));
        let got = x.price.map(|(p, from)| {
            assert_eq!(from, PriceFrom::AxisLabel);
            p
        });
        match (want, got) {
            (Some(w), Some(g)) => assert!((w - g).abs() < 0.005, "t={t}: read {g}, expected {w}"),
            (None, None) => {}
            _ => panic!("t={t}: price {got:?}, expected {want:?}"),
        }
        let feed = common::btc_feed_at(t);
        let h = check_price(got, feed, tol);
        if got.is_some() {
            prices += 1;
            // TradingView's BTCUSD index and Binance's BTCUSDT perpetual agree within 0.3 %.
            assert!(matches!(h, Handshake::Match { .. }), "t={t}: {h:?}");
        } else {
            assert_eq!(h, Handshake::NoScreenPrice);
        }
        slow = slow.max(x.read_ms);
        let diff = match h {
            Handshake::Match { diff_pct } | Handshake::Mismatch { diff_pct, .. } => {
                format!("{diff_pct:+.3}")
            }
            _ => "-".into(),
        };
        println!(
            "{:>5}  chart  BTCUSD -> BTCUSDT  {:<4} {:<10} {feed:<9} {diff:<7} {:<10} {:.0}",
            t / 1000,
            tf.label(),
            got.map_or("none".into(), |p| format!("{p:.2}")),
            match h {
                Handshake::Match { .. } => "match",
                Handshake::Mismatch { .. } => "MISMATCH",
                _ => "no price",
            },
            x.read_ms
        );
    }
    for t in OTHER {
        let mut f = common::trading1(t).unwrap();
        r.reset();
        let x = r.read(&f, None);
        f.wipe();
        assert_eq!(x.page, PageKind::Other, "t={t}");
        assert_eq!(x.symbol, SymbolRead::Unknown);
        assert!(x.tf.is_none() && x.price.is_none());
        println!(
            "{:>5}  other  (no chart: nothing read beyond page and legend)",
            t / 1000
        );
    }
    assert_eq!(prices, 18);
    println!(
        "slowest cold read {slow:.0} ms; image files written: {}",
        privacy::images_written()
    );
    assert_eq!(
        privacy::images_written(),
        0,
        "no frame or crop may be saved"
    );
}

#[test]
fn handshake_flags_wrong_feed() {
    if common::video("trading1.mp4").is_none() {
        return;
    }
    let cfg = VisionConfig::default_config().unwrap();
    let tol = cfg.price_tolerance_pct;
    let mut r = Reader::new(cfg, Engines::detect());
    let f = common::trading1(5_000).unwrap();
    let screen = r.read(&f, None).price.map(|(p, _)| p);
    assert!(screen.is_some());
    // Right feed: match. ETH feed (wrong symbol): mismatch. BTC feed 0.5 % away (stale or a
    // different market): mismatch.
    assert!(matches!(
        check_price(screen, common::BTCUSDT_1M_CLOSE[0], tol),
        Handshake::Match { .. }
    ));
    for feed in [
        common::ETHUSDT_1M_CLOSE[0],
        common::BTCUSDT_1M_CLOSE[0] * 1.005,
        common::BTCUSDT_1M_CLOSE[0] * 0.995,
    ] {
        let h = check_price(screen, feed, tol);
        assert!(h.is_mismatch(), "feed {feed}: {h:?}");
    }
}

#[test]
fn unsupported_symbols() {
    // A real BTCUSD frame with a symbol table that lacks BTCUSD: shown as unsupported.
    if let Some(f) = common::trading1(5_000) {
        let mut cfg = VisionConfig::default_config().unwrap();
        cfg.symbols.remove("BTCUSD");
        let mut r = Reader::new(cfg, Engines::detect());
        let x = r.read(&f, None);
        assert_eq!(
            x.symbol,
            SymbolRead::Unsupported {
                ticker: "BTCUSD".into()
            }
        );
        assert!(x.symbol.exchange().is_none());
    }
    // Real XAUUSD 4h (OANDA) frames from the screen share in trading.mp4.
    if common::video("trading.mp4").is_some() {
        let mut r = Reader::new(VisionConfig::default_config().unwrap(), Engines::detect());
        for t in [60_000, 240_000] {
            let mut f = common::trading_xau(t).unwrap();
            let x = r.read(&f, None);
            f.wipe();
            println!("trading.mp4 t={}s: {}", t / 1000, x.summary());
            assert_eq!(
                x.symbol,
                SymbolRead::Unsupported {
                    ticker: "XAUUSD".into()
                },
                "t={t}"
            );
            assert_eq!(x.tf.as_ref().and_then(|t| t.tf), Some(Timeframe::H4));
        }
    }
}
