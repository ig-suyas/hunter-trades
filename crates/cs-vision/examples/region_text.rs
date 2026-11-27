//! Development aid: prints the OCR text of each configured region of a video frame (text only).
#![allow(clippy::print_stdout)]
use cs_vision::ocr::Engines;
fn main() -> Result<(), String> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let cfg = cs_vision::VisionConfig::default_config()?;
    let mut e = Engines::detect();
    let k: u32 = a.get(2).map_or(Ok(2), |s| s.parse()).map_err(|_| "scale")?;
    let mut f =
        cs_vision::video::frame_at(std::path::Path::new(&a[0]), a[1].parse().map_err(|_| "t")?)?;
    if let Ok(c) = std::env::var("CS_CROP") {
        let v: Vec<u32> = c.split(',').filter_map(|x| x.parse().ok()).collect();
        f = f.crop_px(v[0], v[1], v[2], v[3]);
    }
    for (name, r) in cfg.regions.all() {
        let c = f.crop(&r);
        let l = e.read(&c.binarize(200).upscale(k))?;
        println!(
            "{name} ({} x {}): {:?}",
            c.w,
            c.h,
            l.iter().map(|x| x.text.clone()).collect::<Vec<_>>()
        );
    }
    Ok(())
}
