//! One frame -> what the platform shows: page type, ticker, time frame, last price. Reads only
//! the configured regions; a region whose pixels did not change since the last frame is not
//! read again.

use std::collections::HashMap;
use std::time::Instant;

use crate::config::VisionConfig;
use crate::frame::{Frame, RectF};
use crate::ocr::{EngineKind, Engines, Line};
use crate::parse::{self, Legend, TfRead, TitleRead};
use crate::privacy;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageKind {
    /// A chart (legend or chart toolbar found).
    Chart,
    /// Something else on the platform (pricing page, settings, loading).
    Other,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SymbolRead {
    /// Nothing readable.
    Unknown,
    /// A ticker the table maps to an exchange feed.
    Supported { ticker: String, exchange: String },
    /// A ticker with no feed (shown as "unsupported", no context).
    Unsupported { ticker: String },
}

impl SymbolRead {
    pub fn exchange(&self) -> Option<&str> {
        match self {
            SymbolRead::Supported { exchange, .. } => Some(exchange),
            _ => None,
        }
    }
    pub fn ticker(&self) -> Option<&str> {
        match self {
            SymbolRead::Supported { ticker, .. } | SymbolRead::Unsupported { ticker } => {
                Some(ticker)
            }
            SymbolRead::Unknown => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PriceFrom {
    /// The coloured last-price label on the price axis.
    AxisLabel,
    /// The window title (`TradingView` puts the last price there).
    Title,
}

/// What one frame showed.
#[derive(Debug, Clone, PartialEq)]
pub struct Reading {
    pub page: PageKind,
    pub symbol: SymbolRead,
    pub tf: Option<TfRead>,
    pub price: Option<(f64, PriceFrom)>,
    /// Legend exchange/source ("CRYPTO", "BINANCE").
    pub source: Option<String>,
    /// Where the ticker came from: "symbol region", "window title" or "legend".
    pub symbol_from: &'static str,
    pub engine: Option<EngineKind>,
    /// OCR calls made for this frame (unchanged regions are skipped).
    pub ocr_calls: u32,
    /// Time spent reading this frame (crop + OCR + parse), milliseconds.
    pub read_ms: f64,
}

impl Reading {
    /// One line of parsed text (what the log keeps; never pixels).
    pub fn summary(&self) -> String {
        let sym = match &self.symbol {
            SymbolRead::Unknown => "symbol ?".to_string(),
            SymbolRead::Supported { ticker, exchange } => format!("{ticker} -> {exchange}"),
            SymbolRead::Unsupported { ticker } => format!("{ticker} (unsupported)"),
        };
        let tf = self
            .tf
            .as_ref()
            .map_or_else(|| "tf ?".into(), TfRead::label);
        let px = self.price.map_or_else(
            || "price ?".into(),
            |(p, f)| {
                format!(
                    "price {p} ({})",
                    if f == PriceFrom::AxisLabel {
                        "axis"
                    } else {
                        "title"
                    }
                )
            },
        );
        format!(
            "page {:?}, {sym} [{}], {tf}, {px}, {} OCR calls, {:.0} ms",
            self.page, self.symbol_from, self.ocr_calls, self.read_ms
        )
    }
}

/// Reads frames with a cache of region results.
pub struct Reader {
    pub cfg: VisionConfig,
    pub engines: Engines,
    cache: HashMap<&'static str, (u64, Vec<String>)>,
    /// Text read from the last price label and its rows in the axis region (diagnostics; text
    /// only).
    pub last_label: Option<((u32, u32), Vec<String>)>,
}

/// Sorts OCR lines into reading order (rows, then left to right) and returns their text.
fn ordered(mut lines: Vec<Line>) -> Vec<String> {
    lines.sort_by(|a, b| {
        let (ya, yb) = (a.cy(), b.cy());
        if (ya - yb).abs() > 6.0 {
            ya.total_cmp(&yb)
        } else {
            a.x().total_cmp(&b.x())
        }
    });
    lines.into_iter().map(|l| l.text).collect()
}

/// Makes white text on a coloured label dark on light: 255 - min(r, g, b).
fn label_to_ink(f: &Frame) -> Frame {
    let mut out = f.clone();
    for p in out.bgra.as_chunks_mut::<4>().0 {
        let v = 255 - p[0].min(p[1]).min(p[2]);
        p[0] = v;
        p[1] = v;
        p[2] = v;
    }
    out
}

/// A label background pixel: `TradingView`'s red (#F23645-like) or green (#089981 / #26A69A).
fn label_colour(b: u8, g: u8, r: u8) -> bool {
    let (b, g, r) = (i32::from(b), i32::from(g), i32::from(r));
    let red = r > 170 && g < 110 && b < 130 && r - g > 90;
    let green = g > 110 && r < 100 && g - r > 60 && b > 80 && g >= b - 10;
    red || green
}

/// Pixel rows (inside `axis`) of the tallest red/green label band: the last-price label.
pub fn find_price_label(axis: &Frame) -> Option<(u32, u32)> {
    let mut rows = Vec::with_capacity(axis.h as usize);
    for y in 0..axis.h {
        let n = (0..axis.w)
            .filter(|&x| {
                let (b, g, r) = axis.px(x, y);
                label_colour(b, g, r)
            })
            .count();
        rows.push(n * 10 >= axis.w as usize * 3);
    }
    let mut best: Option<(u32, u32)> = None;
    let mut y = 0u32;
    while y < axis.h {
        if !rows[y as usize] {
            y += 1;
            continue;
        }
        let start = y;
        let mut end = y;
        let mut gap = 0;
        while y < axis.h && gap <= 3 {
            if rows[y as usize] {
                end = y;
                gap = 0;
            } else {
                gap += 1;
            }
            y += 1;
        }
        let h = end - start + 1;
        if h >= 8 && best.is_none_or(|(s, e)| h > e - s + 1) {
            best = Some((start, end));
        }
    }
    best
}

impl Reader {
    pub fn new(cfg: VisionConfig, engines: Engines) -> Reader {
        Reader {
            cfg,
            engines,
            cache: HashMap::new(),
            last_label: None,
        }
    }

    /// Forgets cached region results (after the window or the config changed).
    pub fn reset(&mut self) {
        self.cache.clear();
    }

    /// OCR of one prepared crop, cached by the hash of the source pixels.
    fn ocr(
        &mut self,
        key: &'static str,
        src: &Frame,
        prepare: impl Fn(&Frame) -> Frame,
        calls: &mut u32,
    ) -> Vec<String> {
        let h = src.hash();
        if let Some((ch, lines)) = self.cache.get(key)
            && *ch == h
        {
            return lines.clone();
        }
        let img = prepare(src);
        privacy::debug_crop(key, &img);
        *calls += 1;
        let lines = self.engines.read(&img).map(ordered).unwrap_or_default();
        self.cache.insert(key, (h, lines.clone()));
        lines
    }

    fn region(
        &mut self,
        key: &'static str,
        frame: &Frame,
        r: RectF,
        calls: &mut u32,
    ) -> Vec<String> {
        let crop = frame.crop(&r);
        let k = self.cfg.ocr_scale;
        self.ocr(key, &crop, |c| c.binarize(200).upscale(k), calls)
    }

    /// One try of a retry ladder: two-tone with an ink level (`Some(div)`) or plain, at a scale.
    fn try_read(
        &mut self,
        key: &'static str,
        crop: &Frame,
        bin: Option<usize>,
        scale: u32,
        calls: &mut u32,
    ) -> Vec<String> {
        self.ocr(
            key,
            crop,
            |c| match bin {
                Some(d) => c.binarize(d).upscale(scale),
                None => c.upscale(scale),
            },
            calls,
        )
    }

    /// The ticker in the symbol region. A ticker in the symbol table is taken from one read;
    /// any other needs two reads that agree (so "UUUSD" from a bad read of XAUUSD is not shown).
    fn read_ticker(&mut self, frame: &Frame, calls: &mut u32) -> Option<String> {
        let crop = frame.crop(&self.cfg.regions.symbol);
        let k = self.cfg.ocr_scale;
        let ladder: [(&'static str, Option<usize>, u32); 4] = [
            ("symbol", Some(200), k),
            ("symbol_b50", Some(50), k),
            ("symbol_x3", Some(200), k + 1),
            ("symbol_g3", None, k + 1),
        ];
        let mut seen: Vec<String> = Vec::new();
        for (key, bin, scale) in ladder {
            let lines = self.try_read(key, &crop, bin, scale, calls);
            if let Some(t) = parse::ticker_from_lines(&lines) {
                if self.cfg.symbols.contains_key(&t) || seen.contains(&t) {
                    return Some(t);
                }
                seen.push(t);
            }
        }
        None
    }

    /// Reads one frame. `title` is the parsed window title, if it had the chart shape.
    // One pass over the regions, top to bottom, reads best as one function.
    #[allow(clippy::too_many_lines)]
    pub fn read(&mut self, frame: &Frame, title: Option<&TitleRead>) -> Reading {
        let t0 = Instant::now();
        let mut calls = 0u32;
        self.last_label = None;
        let regions = self.cfg.regions.clone();
        let page_lines = self.region("page", frame, regions.page, &mut calls);
        let toolbar = page_lines
            .iter()
            .map(|l| parse::letters(l))
            .collect::<String>();
        let toolbar_chart = self
            .cfg
            .page_words
            .iter()
            .any(|w| toolbar.contains(w.as_str()));
        // OCR drops the short interval between the legend's dots now and then; on a chart page
        // the legend is retried with other preprocessing until it parses (each try is cached
        // until the legend's pixels change).
        let legend_crop = frame.crop(&regions.legend);
        let k = self.cfg.ocr_scale;
        let mut legend: Option<Legend> = None;
        let tries: [(&'static str, Option<usize>, u32); 5] = [
            ("legend", Some(50), k),
            ("legend_b200", Some(200), k),
            ("legend_b50x3", Some(50), k + 1),
            ("legend_g2", None, k),
            ("legend_g3", None, k + 1),
        ];
        for (i, (key, bin, scale)) in tries.into_iter().enumerate() {
            if i > 0 && !toolbar_chart && title.is_none() {
                break;
            }
            let lines = self.try_read(key, &legend_crop, bin, scale, &mut calls);
            legend = parse::parse_legend(&lines.join(" "));
            if legend.is_some() {
                break;
            }
        }
        // A window title in the chart-tab shape (ticker, price, change) also says "chart".
        let chart = legend.is_some() || toolbar_chart || title.is_some();
        if !chart {
            return Reading {
                page: PageKind::Other,
                symbol: SymbolRead::Unknown,
                tf: None,
                price: None,
                source: None,
                symbol_from: "none",
                engine: self.engines.last,
                ocr_calls: calls,
                read_ms: t0.elapsed().as_secs_f64() * 1000.0,
            };
        }
        let (ticker, from) = if let Some(t) = self.read_ticker(frame, &mut calls) {
            (Some(t), "symbol region")
        } else if let Some(t) = title {
            (Some(t.ticker.clone()), "window title")
        } else if let Some(t) = legend
            .as_ref()
            .and_then(|l| self.cfg.descriptions.get(&l.description))
        {
            (Some(t.clone()), "legend")
        } else {
            (None, "none")
        };
        let symbol = match ticker {
            None => SymbolRead::Unknown,
            Some(t) => match self.cfg.symbols.get(&t) {
                Some(x) => SymbolRead::Supported {
                    ticker: t,
                    exchange: x.clone(),
                },
                None => SymbolRead::Unsupported { ticker: t },
            },
        };
        // Last-price label: located by its colour inside the axis region, then only that band
        // is read.
        let axis = frame.crop(&regions.price_axis);
        let mut price = None;
        if let Some((y0, y1)) = find_price_label(&axis) {
            let band = axis.crop_px(0, y0.saturating_sub(1), axis.w, y1 - y0 + 3);
            // OCR of small digits slips now and then (a 6 read as 0), and a wrong price would
            // raise a false SOURCE MISMATCH. So the label is read at up to four scales and a
            // value counts only when two reads agree.
            let mut seen: Vec<(f64, usize)> = Vec::new();
            let mut all_lines = Vec::new();
            for (key, k) in [
                ("price_label_x3", 3u32),
                ("price_label_x4", 4),
                ("price_label_x5", 5),
                ("price_label_x2", 2),
            ] {
                let lines = self.ocr(key, &band, |c| label_to_ink(c).upscale(k), &mut calls);
                let p = parse::first_price_dec(&lines);
                all_lines.extend(lines);
                if let Some(p) = p {
                    if seen.iter().any(|s| (s.0 - p.0).abs() < 1e-9 && s.1 == p.1) {
                        price = Some((p.0, PriceFrom::AxisLabel));
                        break;
                    }
                    seen.push(p);
                }
            }
            self.last_label = Some(((y0, y1), all_lines));
        }
        if price.is_none() {
            price = title.and_then(|t| t.price).map(|p| (p, PriceFrom::Title));
        }
        Reading {
            page: PageKind::Chart,
            symbol,
            tf: legend.as_ref().map(|l| l.tf.clone()),
            price,
            source: legend.map(|l| l.exchange),
            symbol_from: from,
            engine: self.engines.last,
            ocr_calls: calls,
            read_ms: t0.elapsed().as_secs_f64() * 1000.0,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_band_found_by_colour() {
        // 40 x 100 axis, white, with a red band at rows 30..=53 and a grey one at 70..=80.
        let mut bgra = vec![255u8; 40 * 100 * 4];
        for y in 30..54 {
            for x in 0..40 {
                let i = (y * 40 + x) * 4;
                bgra[i..i + 3].copy_from_slice(&[0x45, 0x36, 0xF2]);
            }
        }
        for y in 70..81 {
            for x in 0..40 {
                let i = (y * 40 + x) * 4;
                bgra[i..i + 3].copy_from_slice(&[0x22, 0x17, 0x13]);
            }
        }
        let f = Frame::from_bgra(40, 100, bgra).unwrap();
        assert_eq!(find_price_label(&f), Some((30, 53)));
        let ink = label_to_ink(&f.crop_px(0, 30, 40, 24));
        assert!(ink.px(0, 0).0 > 150, "label background becomes light");
    }
}
