//! The only place in this crate that writes files:
//! - the parsed-text log (one line of text per change: symbol, time frame, price; never pixels);
//! - region crops, **only** when the debug flag `CANDLESCOPE_VISION_DEBUG=1` is set (off by
//!   default; crops of the read regions only, never a full frame).
//!
//! A test scans the crate's sources and fails if any other module writes files or opens a
//! network connection.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::sync::atomic::{AtomicU64, Ordering};

use crate::frame::Frame;

/// Environment variable that turns on writing region crops for debugging.
pub const DEBUG_ENV: &str = "CANDLESCOPE_VISION_DEBUG";

static DEBUG_DIR: OnceLock<Option<PathBuf>> = OnceLock::new();
static IMAGES_WRITTEN: AtomicU64 = AtomicU64::new(0);

/// The debug folder, if the flag is set (read once per process).
pub fn debug_dir() -> Option<&'static Path> {
    DEBUG_DIR
        .get_or_init(|| {
            (std::env::var(DEBUG_ENV).as_deref() == Ok("1")).then(|| {
                std::env::var_os("CANDLESCOPE_VISION_DEBUG_DIR")
                    .map_or_else(|| PathBuf::from("data/vision-debug"), PathBuf::from)
            })
        })
        .as_deref()
}

/// Image files written by this process (stays 0 unless the debug flag is set).
pub fn images_written() -> u64 {
    IMAGES_WRITTEN.load(Ordering::Relaxed)
}

/// Writes a region crop as PPM, only with the debug flag.
pub fn debug_crop(key: &str, img: &Frame) {
    let Some(dir) = debug_dir() else {
        return;
    };
    if std::fs::create_dir_all(dir).is_err() {
        return;
    }
    let ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |d| d.as_millis());
    if std::fs::write(dir.join(format!("{ms}-{key}.ppm")), img.to_ppm()).is_ok() {
        IMAGES_WRITTEN.fetch_add(1, Ordering::Relaxed);
    }
}

/// Appends one line of parsed text to the log file (best effort).
pub fn log_text(path: &Path, line: &str) {
    if let Some(d) = path.parent() {
        let _ = std::fs::create_dir_all(d);
    }
    if let Ok(mut f) = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
    {
        let _ = writeln!(f, "{line}");
    }
}

#[cfg(test)]
mod tests {
    /// Source files of this crate, outside their test modules.
    fn sources() -> Vec<(&'static str, &'static str)> {
        vec![
            ("lib.rs", include_str!("lib.rs")),
            ("config.rs", include_str!("config.rs")),
            ("frame.rs", include_str!("frame.rs")),
            ("handshake.rs", include_str!("handshake.rs")),
            ("ocr.rs", include_str!("ocr.rs")),
            ("parse.rs", include_str!("parse.rs")),
            ("read.rs", include_str!("read.rs")),
            ("source.rs", include_str!("source.rs")),
            ("video.rs", include_str!("video.rs")),
            ("watch.rs", include_str!("watch.rs")),
        ]
    }

    #[test]
    fn no_other_module_writes_files_or_uses_the_network() {
        let banned = [
            "fs::write",
            "File::create",
            "OpenOptions",
            "create_dir",
            "TcpStream",
            "UdpSocket",
            "TcpListener",
            "http://",
            "https://",
            "ureq",
            "tungstenite",
            "reqwest",
        ];
        for (name, src) in sources() {
            let code = src.split("#[cfg(test)]").next().unwrap_or(src);
            for b in banned {
                assert!(!code.contains(b), "{name} contains {b:?}");
            }
        }
    }

    #[test]
    fn manifest_has_no_network_crates() {
        let m = include_str!("../Cargo.toml");
        for b in [
            "ureq",
            "tungstenite",
            "reqwest",
            "hyper",
            "native-tls",
            "cs-live",
        ] {
            assert!(!m.contains(b), "Cargo.toml depends on {b}");
        }
    }
}
