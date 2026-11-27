//! Development aid: OCR a whole video frame and print the text lines with their boxes (text only;
//! the frame stays in memory).
#![allow(clippy::print_stdout)]
use cs_vision::ocr::Engines;
fn main() -> Result<(), String> {
    let mut a = std::env::args().skip(1);
    let path = a.next().ok_or("usage: ocr_dump <video> <t_ms> [scale]")?;
    let t: u64 = a.next().ok_or("t_ms")?.parse().map_err(|_| "t_ms")?;
    let k: u32 = a.next().map_or(Ok(1), |s| s.parse()).map_err(|_| "scale")?;
    let f = cs_vision::video::frame_at(std::path::Path::new(&path), t)?;
    println!("frame {} x {}", f.w, f.h);
    let mut e = Engines::detect();
    let img = f.upscale(k);
    let t0 = std::time::Instant::now();
    let lines = e.read(&img)?;
    println!(
        "engine {:?}, {} lines in {:?}",
        e.last,
        lines.len(),
        t0.elapsed()
    );
    #[allow(clippy::cast_precision_loss)]
    let kf = k as f32;
    for l in lines {
        let x0 = l.words.iter().map(|w| w.x).fold(f32::INFINITY, f32::min) / kf;
        let y0 = l.words.iter().map(|w| w.y).fold(f32::INFINITY, f32::min) / kf;
        let x1 = l.words.iter().map(|w| w.x + w.w).fold(0.0, f32::max) / kf;
        let y1 = l.words.iter().map(|w| w.y + w.h).fold(0.0, f32::max) / kf;
        println!("{x0:6.0} {y0:5.0} {x1:6.0} {y1:5.0}  {}", l.text);
    }
    Ok(())
}
