//! Colours and type sizes. Dark panel; every text colour keeps WCAG AA contrast (4.5:1) even at
//! the lowest background opacity over a white chart (the worst backdrop), and AAA (7:1) on the
//! opaque panel. Up and down use the same neutral colour: direction is told by arrows and words,
//! and red/green would read like trade cues.

use eframe::egui::Color32;

/// Opaque panel background.
pub const BG: [u8; 3] = [18, 22, 27];
pub const BG_RAISED: [u8; 3] = [30, 36, 44];
pub const TEXT: [u8; 3] = [238, 241, 244];
pub const TEXT_DIM: [u8; 3] = [200, 207, 214];
pub const LIVE: [u8; 3] = [143, 227, 176];
pub const WARN: [u8; 3] = [255, 210, 122];
pub const ALERT: [u8; 3] = [255, 179, 171];
pub const BORDER: [u8; 3] = [96, 108, 122];

/// Lowest background opacity the settings allow.
pub const MIN_OPACITY: f32 = 0.80;
pub const DEFAULT_OPACITY: f32 = 0.94;

/// Minimum font size anywhere (points).
pub const FONT_MIN: f32 = 14.0;
pub const FONT_BODY: f32 = 15.0;
pub const FONT_HEAD: f32 = 16.0;
pub const FONT_BIG: f32 = 18.0;

pub fn c(rgb: [u8; 3]) -> Color32 {
    Color32::from_rgb(rgb[0], rgb[1], rgb[2])
}

/// Background with the chosen opacity.
pub fn bg(opacity: f32) -> Color32 {
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let a = (opacity.clamp(MIN_OPACITY, 1.0) * 255.0).round() as u8;
    Color32::from_rgba_unmultiplied(BG[0], BG[1], BG[2], a)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lin(v: u8) -> f64 {
        let s = f64::from(v) / 255.0;
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    }

    /// WCAG relative luminance.
    pub fn luminance(rgb: [f64; 3]) -> f64 {
        #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
        let l = |x: f64| lin(x.round().clamp(0.0, 255.0) as u8);
        0.2126 * l(rgb[0]) + 0.7152 * l(rgb[1]) + 0.0722 * l(rgb[2])
    }

    pub fn contrast(a: [f64; 3], b: [f64; 3]) -> f64 {
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    fn f(rgb: [u8; 3]) -> [f64; 3] {
        rgb.map(f64::from)
    }

    /// The panel background at opacity `a` over a backdrop.
    pub fn over(a: f64, backdrop: [f64; 3]) -> [f64; 3] {
        let b = f(BG);
        [0, 1, 2].map(|i| a * b[i] + (1.0 - a) * backdrop[i])
    }

    #[test]
    fn text_contrast_holds_at_every_opacity_and_backdrop() {
        let texts = [TEXT, TEXT_DIM, LIVE, WARN, ALERT];
        for t in texts {
            let opaque = contrast(f(t), f(BG));
            assert!(opaque >= 7.0, "{t:?} on opaque panel: {opaque:.2}");
            for backdrop in [
                [255.0, 255.0, 255.0],
                [128.0, 128.0, 128.0],
                [0.0, 0.0, 0.0],
            ] {
                let r = contrast(f(t), over(f64::from(MIN_OPACITY), backdrop));
                assert!(r >= 4.5, "{t:?} at min opacity over {backdrop:?}: {r:.2}");
            }
        }
        // Text on the raised button background too.
        assert!(contrast(f(TEXT), f(BG_RAISED)) >= 7.0);
    }
}
