//! In-memory frames. A frame is never written to disk by this crate (the only exception is the
//! opt-in debug flag in [`crate::privacy`], which writes small region crops, never full frames).

// Pixel geometry converts between pixel counts (u32/usize) and fractions (f32) all the time; the
// values are bounded by the frame size, so the casts cannot overflow.
#![allow(
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss,
    clippy::cast_precision_loss
)]

/// A rectangle as fractions of the window (0..1), so one config fits any window size.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RectF {
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

impl RectF {
    pub const fn new(x: f32, y: f32, w: f32, h: f32) -> RectF {
        RectF { x, y, w, h }
    }

    /// True when both rectangles share some area.
    pub fn intersects(&self, o: &RectF) -> bool {
        self.x < o.x + o.w && o.x < self.x + self.w && self.y < o.y + o.h && o.y < self.y + self.h
    }

    /// Inside the unit square with a positive size.
    pub fn valid(&self) -> bool {
        self.w > 0.0
            && self.h > 0.0
            && self.x >= 0.0
            && self.y >= 0.0
            && self.x + self.w <= 1.0 + 1e-6
            && self.y + self.h <= 1.0 + 1e-6
    }

    /// Pixel rectangle (x, y, w, h) inside a `fw` x `fh` frame; at least 1 x 1.
    pub fn to_px(&self, fw: u32, fh: u32) -> (u32, u32, u32, u32) {
        let x0 = ((self.x * fw as f32).floor() as u32).min(fw.saturating_sub(1));
        let y0 = ((self.y * fh as f32).floor() as u32).min(fh.saturating_sub(1));
        let x1 = (((self.x + self.w) * fw as f32).ceil() as u32).clamp(x0 + 1, fw);
        let y1 = (((self.y + self.h) * fh as f32).ceil() as u32).clamp(y0 + 1, fh);
        (x0, y0, x1 - x0, y1 - y0)
    }
}

/// A BGRA image held in memory.
#[derive(Clone, PartialEq, Eq)]
#[must_use]
pub struct Frame {
    pub w: u32,
    pub h: u32,
    /// `w * h * 4` bytes, B G R A per pixel, rows top to bottom.
    pub bgra: Vec<u8>,
}

impl std::fmt::Debug for Frame {
    // Never dump pixels into logs.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Frame({} x {})", self.w, self.h)
    }
}

impl Frame {
    /// Wraps a BGRA buffer; `None` if the size does not match.
    pub fn from_bgra(w: u32, h: u32, bgra: Vec<u8>) -> Option<Frame> {
        (bgra.len() == w as usize * h as usize * 4 && w > 0 && h > 0).then_some(Frame {
            w,
            h,
            bgra,
        })
    }

    /// Converts an RGBA buffer (as `xcap` returns it) in place.
    pub fn from_rgba(w: u32, h: u32, mut rgba: Vec<u8>) -> Option<Frame> {
        for p in rgba.as_chunks_mut::<4>().0 {
            p.swap(0, 2);
        }
        Frame::from_bgra(w, h, rgba)
    }

    /// (b, g, r) at a pixel.
    #[inline]
    pub fn px(&self, x: u32, y: u32) -> (u8, u8, u8) {
        let i = (y as usize * self.w as usize + x as usize) * 4;
        (self.bgra[i], self.bgra[i + 1], self.bgra[i + 2])
    }

    /// Copy of a pixel rectangle (clamped to the frame).
    pub fn crop_px(&self, x: u32, y: u32, w: u32, h: u32) -> Frame {
        let x = x.min(self.w - 1);
        let y = y.min(self.h - 1);
        let w = w.clamp(1, self.w - x);
        let h = h.clamp(1, self.h - y);
        let mut out = Vec::with_capacity(w as usize * h as usize * 4);
        for row in y..y + h {
            let a = (row as usize * self.w as usize + x as usize) * 4;
            out.extend_from_slice(&self.bgra[a..a + w as usize * 4]);
        }
        Frame { w, h, bgra: out }
    }

    pub fn crop(&self, r: &RectF) -> Frame {
        let (x, y, w, h) = r.to_px(self.w, self.h);
        self.crop_px(x, y, w, h)
    }

    /// Integer upscale (bilinear), which helps OCR read 11-13 px chart text.
    pub fn upscale(&self, k: u32) -> Frame {
        if k <= 1 {
            return self.clone();
        }
        self.resize(self.w * k, self.h * k)
    }

    /// Bilinear resize to `nw` x `nh`.
    pub fn resize(&self, nw: u32, nh: u32) -> Frame {
        let (nw, nh) = (nw.max(1), nh.max(1));
        let mut out = vec![0u8; nw as usize * nh as usize * 4];
        let kx = nw as f32 / self.w as f32;
        let ky = nh as f32 / self.h as f32;
        for y in 0..nh {
            let sy = ((y as f32 + 0.5) / ky - 0.5).max(0.0);
            let y0 = (sy as u32).min(self.h - 1);
            let y1 = (y0 + 1).min(self.h - 1);
            let fy = sy - y0 as f32;
            for x in 0..nw {
                let sx = ((x as f32 + 0.5) / kx - 0.5).max(0.0);
                let x0 = (sx as u32).min(self.w - 1);
                let x1 = (x0 + 1).min(self.w - 1);
                let fx = sx - x0 as f32;
                let o = (y as usize * nw as usize + x as usize) * 4;
                for c in 0..3 {
                    let g = |xx: u32, yy: u32| {
                        f32::from(self.bgra[(yy as usize * self.w as usize + xx as usize) * 4 + c])
                    };
                    let top = g(x0, y0) * (1.0 - fx) + g(x1, y0) * fx;
                    let bot = g(x0, y1) * (1.0 - fx) + g(x1, y1) * fx;
                    out[o + c] = (top * (1.0 - fy) + bot * fy).round().clamp(0.0, 255.0) as u8;
                }
                out[o + 3] = 255;
            }
        }
        Frame {
            w: nw,
            h: nh,
            bgra: out,
        }
    }

    /// Cheap content hash (FNV-1a over every 2nd pixel of every row): decides whether a region
    /// changed since the last OCR, so unchanged regions are not read again.
    pub fn hash(&self) -> u64 {
        let mut h: u64 = 0xcbf2_9ce4_8422_2325;
        for p in self.bgra.as_chunks::<8>().0 {
            for &b in &p[..3] {
                h ^= u64::from(b);
                h = h.wrapping_mul(0x0100_0000_01b3);
            }
        }
        h ^ (u64::from(self.w) << 32 | u64::from(self.h))
    }

    /// Overwrites the pixels (called before a full frame is dropped).
    pub fn wipe(&mut self) {
        self.bgra.fill(0);
    }

    /// Binary PPM (P6), used to hand a crop to Tesseract through a pipe.
    pub fn to_ppm(&self) -> Vec<u8> {
        let mut v = format!("P6\n{} {}\n255\n", self.w, self.h).into_bytes();
        v.reserve(self.w as usize * self.h as usize * 3);
        for p in self.bgra.as_chunks::<4>().0 {
            v.extend_from_slice(&[p[2], p[1], p[0]]);
        }
        v
    }

    /// Dark text on white: luminance (inverted on a dark theme), then a two-tone split at 55 %
    /// of the way from the darkest ink to the background. Chart fonts are thin and the grey
    /// separators in the legend confuse OCR. (Otsu's threshold was tried first and erased thin
    /// strokes.) The ink level is the darkest `1 / ink_div` of the pixels: 200 (0.5 %) suits a
    /// short ticker in a mostly empty region, 50 (2 %) a full line of text.
    pub fn binarize(&self, ink_div: usize) -> Frame {
        let mut lum: Vec<u8> = self
            .bgra
            .as_chunks::<4>()
            .0
            .iter()
            .map(|p| {
                ((u32::from(p[0]) * 29 + u32::from(p[1]) * 150 + u32::from(p[2]) * 77) >> 8) as u8
            })
            .collect();
        let n = lum.len();
        let mean = lum.iter().map(|&v| u64::from(v)).sum::<u64>() as f64 / n as f64;
        if mean < 128.0 {
            for v in &mut lum {
                *v = 255 - *v;
            }
        }
        let mut sorted = lum.clone();
        sorted.sort_unstable();
        let ink = f32::from(sorted[n / ink_div.max(1)]);
        let bg = f32::from(sorted[(n * 9) / 10]);
        let th = if bg - ink < 40.0 {
            // No contrast to speak of: a two-tone split would only amplify noise.
            0
        } else {
            (ink + 0.55 * (bg - ink)).round() as u8
        };
        let mut out = self.clone();
        for (p, &v) in out.bgra.as_chunks_mut::<4>().0.iter_mut().zip(&lum) {
            let o = if v < th { 0 } else { 255 };
            p[0] = o;
            p[1] = o;
            p[2] = o;
            p[3] = 255;
        }
        out
    }

    /// Mean luminance 0..255 (dark or light chart theme).
    pub fn mean_luma(&self) -> f32 {
        let mut s = 0u64;
        for p in self.bgra.as_chunks::<4>().0 {
            s += u64::from(p[0]) * 29 + u64::from(p[1]) * 150 + u64::from(p[2]) * 77;
        }
        (s as f64 / (256.0 * (f64::from(self.w) * f64::from(self.h)))) as f32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn solid(w: u32, h: u32, c: u8) -> Frame {
        Frame::from_bgra(w, h, vec![c; (w * h * 4) as usize]).unwrap()
    }

    #[test]
    fn rect_geometry() {
        let r = RectF::new(0.25, 0.5, 0.5, 0.25);
        assert_eq!(r.to_px(100, 200), (25, 100, 50, 50));
        assert!(r.valid());
        assert!(!RectF::new(0.8, 0.0, 0.3, 0.1).valid());
        assert!(r.intersects(&RectF::new(0.7, 0.7, 0.2, 0.2)));
        assert!(!r.intersects(&RectF::new(0.0, 0.0, 0.25, 0.5)));
    }

    #[test]
    fn crop_upscale_hash() {
        let mut f = solid(10, 10, 7);
        f.bgra[(3 * 10 + 4) * 4] = 200;
        let c = f.crop_px(4, 3, 2, 2);
        assert_eq!((c.w, c.h), (2, 2));
        assert_eq!(c.px(0, 0).0, 200);
        let u = c.upscale(2);
        assert_eq!((u.w, u.h), (4, 4));
        assert_ne!(f.hash(), solid(10, 10, 7).hash());
        let mut g = f.clone();
        g.wipe();
        assert!(g.bgra.iter().all(|&b| b == 0));
        assert!(format!("{f:?}").len() < 20);
        assert_eq!(c.to_ppm().len(), 11 + 12);
    }
}
