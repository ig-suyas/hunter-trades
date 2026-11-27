//! Development aid: prints pixel colours (text) along a column of a video frame.
#![allow(clippy::print_stdout)]
fn main() -> Result<(), String> {
    let a: Vec<String> = std::env::args().skip(1).collect();
    let f =
        cs_vision::video::frame_at(std::path::Path::new(&a[0]), a[1].parse().map_err(|_| "t")?)?;
    let x: u32 = a[2].parse().map_err(|_| "x")?;
    let (y0, y1): (u32, u32) = (
        a[3].parse().map_err(|_| "y0")?,
        a[4].parse().map_err(|_| "y1")?,
    );
    for y in y0..y1 {
        let row: Vec<String> = (x..x + 80)
            .step_by(8)
            .map(|xx| {
                let (b, g, r) = f.px(xx, y);
                format!("{r:02x}{g:02x}{b:02x}")
            })
            .collect();
        println!("{y:4} {}", row.join(" "));
    }
    Ok(())
}
