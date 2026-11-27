//! Development aid: reads video frames with the region config and prints the parsed text only.
#![allow(clippy::print_stdout)]
use cs_vision::ocr::Engines;
use cs_vision::read::Reader;
fn main() -> Result<(), String> {
    let mut a = std::env::args().skip(1);
    let path = a.next().ok_or("usage: read_video <video> <t_ms>...")?;
    let cfg = cs_vision::VisionConfig::default_config()?;
    let mut eng = Engines::detect();
    if std::env::var("CS_ENGINE").as_deref() == Ok("tesseract") {
        eng.primary = None;
    }
    let mut r = Reader::new(cfg, eng);
    for t in a {
        let t: u64 = t.parse().map_err(|_| "t_ms")?;
        let mut f = cs_vision::video::frame_at(std::path::Path::new(&path), t)?;
        if let Ok(c) = std::env::var("CS_CROP") {
            let v: Vec<u32> = c.split(',').filter_map(|x| x.parse().ok()).collect();
            f = f.crop_px(v[0], v[1], v[2], v[3]);
            if v.len() == 6 {
                f = f.resize(v[4], v[5]);
            }
        }
        r.reset();
        let x = r.read(&f, None);
        f.wipe();
        println!("{t:>7} ms: {}  label {:?}", x.summary(), r.last_label);
    }
    Ok(())
}
