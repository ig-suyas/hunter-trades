//! Frames from a local video through an ffmpeg pipe, straight into memory (test fixtures and the
//! development examples; the overlay never uses this). Nothing is written to disk.

use std::path::Path;
use std::process::{Command, Stdio};

use crate::frame::Frame;

/// Width and height of the first video stream (ffprobe).
///
/// # Errors
/// ffprobe is missing or the file has no video stream.
pub fn size(path: &Path) -> Result<(u32, u32), String> {
    let out = Command::new("ffprobe")
        .args([
            "-v",
            "error",
            "-select_streams",
            "v:0",
            "-show_entries",
            "stream=width,height",
            "-of",
            "csv=p=0:s=x",
        ])
        .arg(path)
        .stderr(Stdio::null())
        .output()
        .map_err(|e| format!("ffprobe: {e}"))?;
    let s = String::from_utf8_lossy(&out.stdout);
    let (w, h) = s
        .trim()
        .split_once('x')
        .ok_or_else(|| format!("ffprobe: unexpected output {s:?}"))?;
    Ok((
        w.trim().parse().map_err(|_| "ffprobe: bad width")?,
        h.trim().parse().map_err(|_| "ffprobe: bad height")?,
    ))
}

/// The frame at `t_ms` milliseconds, as BGRA in memory.
///
/// # Errors
/// ffmpeg is missing or returned the wrong number of bytes.
pub fn frame_at(path: &Path, t_ms: u64) -> Result<Frame, String> {
    let (w, h) = size(path)?;
    let ts = format!("{}.{:03}", t_ms / 1000, t_ms % 1000);
    let out = Command::new("ffmpeg")
        .args(["-v", "error", "-ss", &ts, "-i"])
        .arg(path)
        .args(["-frames:v", "1", "-f", "rawvideo", "-pix_fmt", "bgra", "-"])
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .map_err(|e| format!("ffmpeg: {e}"))?;
    Frame::from_bgra(w, h, out.stdout)
        .ok_or_else(|| format!("ffmpeg: no {w} x {h} frame at {ts} s"))
}
