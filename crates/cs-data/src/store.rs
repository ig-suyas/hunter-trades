//! Compact columnar on-disk format ("CSC1").
//!
//! Layout (little endian):
//! ```text
//! "CSC1" | version u16 = 1 | ncols u16 | nrows u64 | meta_len u32 | meta (UTF-8, key=value lines)
//! per column: name_len u8 | name | kind u8 (0 = i64 raw, 1 = f64, 2 = i64 delta-encoded)
//! zlib stream (Adler-32 checked): for each column, nrows * 8 bytes, byte-plane shuffled
//! ```
//! Delta encoding turns regular timestamps into constants and the byte shuffle groups the
//! slowly-changing high bytes of prices together, so plain zlib compresses well. Reading a file
//! loads it fully into memory (one month of 1m candles is ~3.5 MB unpacked).

use std::fs;
use std::io::{Read, Write};
use std::path::Path;

use cs_core::Candle;
use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;

use crate::{Error, Result};

const MAGIC: &[u8; 4] = b"CSC1";
const VERSION: u16 = 1;

#[derive(Debug, Clone, PartialEq)]
pub enum Column {
    I64(Vec<i64>),
    F64(Vec<f64>),
}

impl Column {
    pub fn len(&self) -> usize {
        match self {
            Column::I64(v) => v.len(),
            Column::F64(v) => v.len(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// A named set of equal-length columns plus free-form metadata.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Table {
    pub meta: Vec<(String, String)>,
    pub cols: Vec<(String, Column)>,
}

impl Table {
    pub fn nrows(&self) -> usize {
        self.cols.first().map_or(0, |(_, c)| c.len())
    }

    pub fn meta(&self, key: &str) -> Option<&str> {
        self.meta
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }

    pub fn i64(&self, name: &str) -> Result<&[i64]> {
        match self.cols.iter().find(|(n, _)| n == name) {
            Some((_, Column::I64(v))) => Ok(v),
            _ => Err(Error::Format(format!("missing i64 column {name}"))),
        }
    }

    pub fn f64(&self, name: &str) -> Result<&[f64]> {
        match self.cols.iter().find(|(n, _)| n == name) {
            Some((_, Column::F64(v))) => Ok(v),
            _ => Err(Error::Format(format!("missing f64 column {name}"))),
        }
    }
}

fn shuffle(words: &[[u8; 8]], out: &mut Vec<u8>) {
    for plane in 0..8 {
        out.extend(words.iter().map(|w| w[plane]));
    }
}

fn unshuffle(bytes: &[u8], n: usize) -> Vec<[u8; 8]> {
    let mut words = vec![[0u8; 8]; n];
    for plane in 0..8 {
        for (i, w) in words.iter_mut().enumerate() {
            w[plane] = bytes[plane * n + i];
        }
    }
    words
}

/// Serialise a table to bytes.
pub fn encode(t: &Table) -> Result<Vec<u8>> {
    let n = t.nrows();
    if t.cols.iter().any(|(_, c)| c.len() != n) {
        return Err(Error::Format("columns differ in length".into()));
    }
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    out.extend_from_slice(&VERSION.to_le_bytes());
    let ncols =
        u16::try_from(t.cols.len()).map_err(|_| Error::Format("too many columns".into()))?;
    out.extend_from_slice(&ncols.to_le_bytes());
    out.extend_from_slice(&(n as u64).to_le_bytes());
    let meta = t.meta.iter().fold(String::new(), |mut acc, (k, v)| {
        acc.push_str(k);
        acc.push('=');
        acc.push_str(v);
        acc.push('\n');
        acc
    });
    let meta_len = u32::try_from(meta.len()).map_err(|_| Error::Format("meta too long".into()))?;
    out.extend_from_slice(&meta_len.to_le_bytes());
    out.extend_from_slice(meta.as_bytes());
    for (name, col) in &t.cols {
        let len = u8::try_from(name.len()).map_err(|_| Error::Format("name too long".into()))?;
        out.push(len);
        out.extend_from_slice(name.as_bytes());
        out.push(match col {
            Column::I64(_) => 2,
            Column::F64(_) => 1,
        });
    }
    let mut body = Vec::with_capacity(n * 8 * t.cols.len());
    for (_, col) in &t.cols {
        let words: Vec<[u8; 8]> = match col {
            Column::I64(v) => {
                let mut prev = 0i64;
                v.iter()
                    .map(|&x| {
                        let d = x.wrapping_sub(prev);
                        prev = x;
                        d.to_le_bytes()
                    })
                    .collect()
            }
            Column::F64(v) => v.iter().map(|x| x.to_le_bytes()).collect(),
        };
        shuffle(&words, &mut body);
    }
    let mut enc = ZlibEncoder::new(out, Compression::new(6));
    enc.write_all(&body)?;
    Ok(enc.finish()?)
}

fn take<'a>(b: &mut &'a [u8], n: usize) -> Result<&'a [u8]> {
    if b.len() < n {
        return Err(Error::Format("truncated CSC1 header".into()));
    }
    let (h, rest) = b.split_at(n);
    *b = rest;
    Ok(h)
}

fn arr<const N: usize>(b: &mut &[u8]) -> Result<[u8; N]> {
    let s = take(b, N)?;
    let mut a = [0u8; N];
    a.copy_from_slice(s);
    Ok(a)
}

/// Parse bytes produced by [`encode`].
pub fn decode(mut b: &[u8]) -> Result<Table> {
    if take(&mut b, 4)? != MAGIC {
        return Err(Error::Format("not a CSC1 file".into()));
    }
    let version = u16::from_le_bytes(arr(&mut b)?);
    if version != VERSION {
        return Err(Error::Format(format!("unsupported CSC1 version {version}")));
    }
    let ncols = usize::from(u16::from_le_bytes(arr(&mut b)?));
    let n = usize::try_from(u64::from_le_bytes(arr(&mut b)?))
        .map_err(|_| Error::Format("row count".into()))?;
    let meta_len = u32::from_le_bytes(arr(&mut b)?) as usize;
    let meta_s = std::str::from_utf8(take(&mut b, meta_len)?)
        .map_err(|_| Error::Format("meta not UTF-8".into()))?;
    let meta = meta_s
        .lines()
        .filter_map(|l| l.split_once('='))
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let mut heads = Vec::with_capacity(ncols);
    for _ in 0..ncols {
        let len = usize::from(take(&mut b, 1)?[0]);
        let name = std::str::from_utf8(take(&mut b, len)?)
            .map_err(|_| Error::Format("column name".into()))?
            .to_string();
        let kind = take(&mut b, 1)?[0];
        heads.push((name, kind));
    }
    let mut body = Vec::with_capacity(n * 8 * ncols);
    ZlibDecoder::new(b).read_to_end(&mut body)?;
    if body.len() != n * 8 * ncols {
        return Err(Error::Format(format!(
            "CSC1 body has {} bytes, expected {}",
            body.len(),
            n * 8 * ncols
        )));
    }
    let mut cols = Vec::with_capacity(ncols);
    for (i, (name, kind)) in heads.into_iter().enumerate() {
        let words = unshuffle(&body[i * n * 8..(i + 1) * n * 8], n);
        let col = match kind {
            0 => Column::I64(words.into_iter().map(i64::from_le_bytes).collect()),
            1 => Column::F64(words.into_iter().map(f64::from_le_bytes).collect()),
            2 => {
                let mut acc = 0i64;
                Column::I64(
                    words
                        .into_iter()
                        .map(|w| {
                            acc = acc.wrapping_add(i64::from_le_bytes(w));
                            acc
                        })
                        .collect(),
                )
            }
            k => return Err(Error::Format(format!("unknown column kind {k}"))),
        };
        cols.push((name, col));
    }
    Ok(Table { meta, cols })
}

/// Write atomically (temp file + rename) so an interrupted run never leaves a half file.
pub fn write_file(path: &Path, t: &Table) -> Result<()> {
    let bytes = encode(t)?;
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir)?;
    }
    let tmp = path.with_extension("csc.tmp");
    fs::write(&tmp, bytes)?;
    fs::rename(&tmp, path)?;
    Ok(())
}

pub fn read_file(path: &Path) -> Result<Table> {
    let bytes = fs::read(path)?;
    decode(&bytes).map_err(|e| Error::Format(format!("{}: {e}", path.display())))
}

/// Kline column names, in Binance CSV order (`close_time` kept to check timestamp units).
pub const KLINE_COLS: [&str; 11] = [
    "open_time",
    "open",
    "high",
    "low",
    "close",
    "volume",
    "close_time",
    "quote_volume",
    "trades",
    "taker_buy_base",
    "taker_buy_quote",
];

/// Raw kline rows exactly as parsed from the source (timestamps in the source's unit).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct RawKlines {
    pub open_time: Vec<i64>,
    pub open: Vec<f64>,
    pub high: Vec<f64>,
    pub low: Vec<f64>,
    pub close: Vec<f64>,
    pub volume: Vec<f64>,
    pub close_time: Vec<i64>,
    pub quote_volume: Vec<f64>,
    pub trades: Vec<i64>,
    pub taker_buy_base: Vec<f64>,
    pub taker_buy_quote: Vec<f64>,
}

impl RawKlines {
    pub fn len(&self) -> usize {
        self.open_time.len()
    }

    pub fn is_empty(&self) -> bool {
        self.open_time.is_empty()
    }

    pub fn into_table(self, meta: Vec<(String, String)>) -> Table {
        let c = |n: &str| n.to_string();
        Table {
            meta,
            cols: vec![
                (c("open_time"), Column::I64(self.open_time)),
                (c("open"), Column::F64(self.open)),
                (c("high"), Column::F64(self.high)),
                (c("low"), Column::F64(self.low)),
                (c("close"), Column::F64(self.close)),
                (c("volume"), Column::F64(self.volume)),
                (c("close_time"), Column::I64(self.close_time)),
                (c("quote_volume"), Column::F64(self.quote_volume)),
                (c("trades"), Column::I64(self.trades)),
                (c("taker_buy_base"), Column::F64(self.taker_buy_base)),
                (c("taker_buy_quote"), Column::F64(self.taker_buy_quote)),
            ],
        }
    }

    pub fn from_table(t: &Table) -> Result<RawKlines> {
        Ok(RawKlines {
            open_time: t.i64("open_time")?.to_vec(),
            open: t.f64("open")?.to_vec(),
            high: t.f64("high")?.to_vec(),
            low: t.f64("low")?.to_vec(),
            close: t.f64("close")?.to_vec(),
            volume: t.f64("volume")?.to_vec(),
            close_time: t.i64("close_time")?.to_vec(),
            quote_volume: t.f64("quote_volume")?.to_vec(),
            trades: t.i64("trades")?.to_vec(),
            taker_buy_base: t.f64("taker_buy_base")?.to_vec(),
            taker_buy_quote: t.f64("taker_buy_quote")?.to_vec(),
        })
    }

    pub fn append(&mut self, o: &RawKlines) {
        self.open_time.extend_from_slice(&o.open_time);
        self.open.extend_from_slice(&o.open);
        self.high.extend_from_slice(&o.high);
        self.low.extend_from_slice(&o.low);
        self.close.extend_from_slice(&o.close);
        self.volume.extend_from_slice(&o.volume);
        self.close_time.extend_from_slice(&o.close_time);
        self.quote_volume.extend_from_slice(&o.quote_volume);
        self.trades.extend_from_slice(&o.trades);
        self.taker_buy_base.extend_from_slice(&o.taker_buy_base);
        self.taker_buy_quote.extend_from_slice(&o.taker_buy_quote);
    }

    pub fn row(&self, i: usize) -> Candle {
        Candle {
            open_time: self.open_time[i],
            open: self.open[i],
            high: self.high[i],
            low: self.low[i],
            close: self.close[i],
            volume: self.volume[i],
            quote_volume: self.quote_volume[i],
            trades: self.trades[i],
            taker_buy_base: self.taker_buy_base[i],
            taker_buy_quote: self.taker_buy_quote[i],
        }
    }
}

/// Candles (already clean, ms timestamps) to a table, used for resampled outputs.
pub fn candles_to_table(c: &[Candle], minutes: &[i64], meta: Vec<(String, String)>) -> Table {
    let i = |f: fn(&Candle) -> i64| Column::I64(c.iter().map(f).collect());
    let f = |f: fn(&Candle) -> f64| Column::F64(c.iter().map(f).collect());
    let s = |n: &str| n.to_string();
    Table {
        meta,
        cols: vec![
            (s("open_time"), i(|x| x.open_time)),
            (s("open"), f(|x| x.open)),
            (s("high"), f(|x| x.high)),
            (s("low"), f(|x| x.low)),
            (s("close"), f(|x| x.close)),
            (s("volume"), f(|x| x.volume)),
            (s("quote_volume"), f(|x| x.quote_volume)),
            (s("trades"), i(|x| x.trades)),
            (s("taker_buy_base"), f(|x| x.taker_buy_base)),
            (s("taker_buy_quote"), f(|x| x.taker_buy_quote)),
            (s("minutes"), Column::I64(minutes.to_vec())),
        ],
    }
}

/// Read a resampled-candle table back.
pub fn table_to_candles(t: &Table) -> Result<(Vec<Candle>, Vec<i64>)> {
    let ot = t.i64("open_time")?;
    let (o, h, l, c) = (
        t.f64("open")?,
        t.f64("high")?,
        t.f64("low")?,
        t.f64("close")?,
    );
    let (v, q, n) = (t.f64("volume")?, t.f64("quote_volume")?, t.i64("trades")?);
    let (tb, tq) = (t.f64("taker_buy_base")?, t.f64("taker_buy_quote")?);
    let candles = (0..ot.len())
        .map(|i| Candle {
            open_time: ot[i],
            open: o[i],
            high: h[i],
            low: l[i],
            close: c[i],
            volume: v[i],
            quote_volume: q[i],
            trades: n[i],
            taker_buy_base: tb[i],
            taker_buy_quote: tq[i],
        })
        .collect();
    Ok((candles, t.i64("minutes")?.to_vec()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_table() {
        let t = Table {
            meta: vec![("source".into(), "test".into())],
            cols: vec![
                (
                    "ts".into(),
                    Column::I64(vec![1_730_419_200_000, 1_730_419_260_000, -5]),
                ),
                ("px".into(), Column::F64(vec![70_000.5, f64::NAN, -0.0])),
            ],
        };
        let b = encode(&t).unwrap();
        let back = decode(&b).unwrap();
        assert_eq!(back.meta("source"), Some("test"));
        assert_eq!(back.i64("ts").unwrap(), t.i64("ts").unwrap());
        let px = back.f64("px").unwrap();
        assert_eq!(px[0].to_bits(), 70_000.5f64.to_bits());
        assert!(px[1].is_nan());
        assert!(decode(&b[..b.len() - 3]).is_err());
    }

    #[test]
    fn empty_table() {
        let t = Table {
            meta: vec![],
            cols: vec![("a".into(), Column::I64(vec![]))],
        };
        assert_eq!(decode(&encode(&t).unwrap()).unwrap().nrows(), 0);
    }
}
