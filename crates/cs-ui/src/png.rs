//! Minimal PNG writer (8-bit RGB, no filtering) on `flate2`, so screenshots need no image crate.

use std::io::Write;
use std::path::Path;

use flate2::Crc;
use flate2::write::ZlibEncoder;

fn chunk(out: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) {
    let len = u32::try_from(data.len()).unwrap_or(u32::MAX);
    out.extend_from_slice(&len.to_be_bytes());
    let mut crc = Crc::new();
    crc.update(&kind);
    crc.update(data);
    out.extend_from_slice(&kind);
    out.extend_from_slice(data);
    out.extend_from_slice(&crc.sum().to_be_bytes());
}

/// Encode `rgb` (`w * h * 3` bytes, rows top to bottom).
pub fn encode_rgb(w: u32, h: u32, rgb: &[u8]) -> std::io::Result<Vec<u8>> {
    let stride = w as usize * 3;
    if rgb.len() != stride * h as usize {
        return Err(std::io::Error::other("pixel buffer size mismatch"));
    }
    let mut z = ZlibEncoder::new(Vec::new(), flate2::Compression::default());
    for row in rgb.chunks(stride) {
        z.write_all(&[0])?;
        z.write_all(row)?;
    }
    let idat = z.finish()?;
    let mut out = vec![0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A];
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&w.to_be_bytes());
    ihdr.extend_from_slice(&h.to_be_bytes());
    ihdr.extend_from_slice(&[8, 2, 0, 0, 0]);
    chunk(&mut out, *b"IHDR", &ihdr);
    chunk(&mut out, *b"IDAT", &idat);
    chunk(&mut out, *b"IEND", &[]);
    Ok(out)
}

pub fn write_rgb(path: &Path, w: u32, h: u32, rgb: &[u8]) -> std::io::Result<()> {
    if let Some(d) = path.parent() {
        std::fs::create_dir_all(d)?;
    }
    std::fs::write(path, encode_rgb(w, h, rgb)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn png_structure() {
        let px = vec![10u8; 2 * 3 * 3];
        let b = encode_rgb(2, 3, &px).unwrap_or_default();
        assert_eq!(&b[..8], &[0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A]);
        assert_eq!(&b[12..16], b"IHDR");
        assert_eq!(&b[b.len() - 8..b.len() - 4], b"IEND");
        assert!(encode_rgb(2, 2, &px).is_err());
    }
}
