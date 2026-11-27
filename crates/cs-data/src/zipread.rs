//! Minimal reader for the single-entry zip archives Binance Vision publishes.
//!
//! Avoids the `zip` crate: the archives hold one deflated (or stored) CSV, and the SHA-256 of the
//! whole archive is verified before this runs. The entry is located through the central
//! directory (so sizes are correct even when the local header uses a data descriptor) and the
//! CRC-32 of the inflated data is checked.

use std::io::Read;

use flate2::read::DeflateDecoder;

use crate::{Error, Result};

fn u16_at(b: &[u8], i: usize) -> Result<u16> {
    b.get(i..i + 2)
        .map(|s| u16::from_le_bytes([s[0], s[1]]))
        .ok_or_else(|| Error::Format("zip truncated".into()))
}

fn u32_at(b: &[u8], i: usize) -> Result<u32> {
    b.get(i..i + 4)
        .map(|s| u32::from_le_bytes([s[0], s[1], s[2], s[3]]))
        .ok_or_else(|| Error::Format("zip truncated".into()))
}

fn crc32(data: &[u8]) -> u32 {
    let mut table = [0u32; 256];
    for (i, slot) in table.iter_mut().enumerate() {
        let mut c = u32::try_from(i).unwrap_or(0);
        for _ in 0..8 {
            c = if c & 1 == 1 {
                0xEDB8_8320 ^ (c >> 1)
            } else {
                c >> 1
            };
        }
        *slot = c;
    }
    let mut crc = 0xFFFF_FFFFu32;
    for &b in data {
        crc = table[((crc ^ u32::from(b)) & 0xFF) as usize] ^ (crc >> 8);
    }
    crc ^ 0xFFFF_FFFF
}

/// Return (entry name, inflated bytes) of the first entry.
pub fn first_entry(zip: &[u8]) -> Result<(String, Vec<u8>)> {
    // End of central directory: signature 0x06054b50 within the last 64 KiB + 22 bytes.
    let min = zip.len().saturating_sub(65_557);
    let eocd = (min..zip.len().saturating_sub(21))
        .rev()
        .find(|&i| zip[i..i + 4] == [0x50, 0x4b, 0x05, 0x06])
        .ok_or_else(|| Error::Format("zip: no end of central directory".into()))?;
    let entries = u16_at(zip, eocd + 10)?;
    if entries == 0 {
        return Err(Error::Format("zip: empty archive".into()));
    }
    let cd = u32_at(zip, eocd + 16)? as usize;
    if u32_at(zip, cd)? != 0x0201_4b50 {
        return Err(Error::Format("zip: bad central directory".into()));
    }
    let method = u16_at(zip, cd + 10)?;
    let crc = u32_at(zip, cd + 16)?;
    let csize = u32_at(zip, cd + 20)? as usize;
    let usize_ = u32_at(zip, cd + 24)? as usize;
    let name_len = u16_at(zip, cd + 28)? as usize;
    let lho = u32_at(zip, cd + 42)? as usize;
    if csize == 0xFFFF_FFFF || usize_ == 0xFFFF_FFFF || lho == 0xFFFF_FFFF {
        return Err(Error::Format("zip: zip64 not supported".into()));
    }
    let name = String::from_utf8_lossy(
        zip.get(cd + 46..cd + 46 + name_len)
            .ok_or_else(|| Error::Format("zip truncated".into()))?,
    )
    .into_owned();
    if u32_at(zip, lho)? != 0x0403_4b50 {
        return Err(Error::Format("zip: bad local header".into()));
    }
    let data_start = lho + 30 + u16_at(zip, lho + 26)? as usize + u16_at(zip, lho + 28)? as usize;
    let comp = zip
        .get(data_start..data_start + csize)
        .ok_or_else(|| Error::Format("zip: data truncated".into()))?;
    let out = match method {
        0 => comp.to_vec(),
        8 => {
            let mut v = Vec::with_capacity(usize_);
            DeflateDecoder::new(comp).read_to_end(&mut v)?;
            v
        }
        m => {
            return Err(Error::Format(format!(
                "zip: compression method {m} not supported"
            )));
        }
    };
    if out.len() != usize_ {
        return Err(Error::Format(format!(
            "zip: size {} != {}",
            out.len(),
            usize_
        )));
    }
    if crc32(&out) != crc {
        return Err(Error::Checksum(format!("zip CRC-32 mismatch in {name}")));
    }
    Ok((name, out))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc_known_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
    }

    #[test]
    fn reads_fixture() {
        let zip = include_bytes!("../tests/fixtures/BTCUSDT-1m-2024-11-01.zip");
        let (name, data) = first_entry(zip).unwrap();
        assert_eq!(name, "BTCUSDT-1m-2024-11-01.csv");
        assert!(data.starts_with(b"open_time,"));
    }
}
