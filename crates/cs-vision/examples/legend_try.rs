//! Development aid: legend OCR with different preprocessing (text output only).
#![allow(
    clippy::print_stdout,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
use cs_vision::ocr::Engines;
fn main() -> Result<(), String> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let cfg = cs_vision::VisionConfig::default_config()?;
    let mut e = Engines::detect();
    let f =
        cs_vision::video::frame_at(std::path::Path::new(&a[0]), a[1].parse().map_err(|_| "t")?)?;
    let c = f.crop(&cfg.regions.legend);
    for k in [2, 3] {
        let l = e.read(&c.binarize(200).upscale(k))?;
        println!(
            "otsu x{k}: {:?}",
            l.iter().map(|x| x.text.clone()).collect::<Vec<_>>()
        );
        let l = e.read(&c.upscale(k).binarize(200))?;
        println!(
            "up-then-otsu x{k}: {:?}",
            l.iter().map(|x| x.text.clone()).collect::<Vec<_>>()
        );
    }
    for (name, th) in [
        ("gray", 0i32),
        ("bin140", 140),
        ("bin180", 180),
        ("bin210", 210),
    ] {
        let mut g = c.clone();
        for p in g.bgra.as_chunks_mut::<4>().0 {
            let l = (i32::from(p[0]) * 29 + i32::from(p[1]) * 150 + i32::from(p[2]) * 77) / 256;
            let v = if th == 0 {
                l
            } else if l < th {
                0
            } else {
                255
            };
            p[0] = v as u8;
            p[1] = v as u8;
            p[2] = v as u8;
        }
        for k in [2, 3] {
            let l = e.read(&g.upscale(k))?;
            println!(
                "{name} x{k}: {:?}",
                l.iter().map(|x| x.text.clone()).collect::<Vec<_>>()
            );
        }
    }
    Ok(())
}
