//! Offline OCR of small regions. Primary: the Windows OCR engine (`Windows.Media.Ocr`, part of
//! Windows, runs on this machine, no network). Fallback: the Tesseract command-line program,
//! fed through a pipe (the crop is never written to a file).

use std::io::{Read, Write};
use std::process::{Command, Stdio};

use crate::frame::Frame;

/// One recognised word with its box in the image it was read from (pixels).
#[derive(Debug, Clone, PartialEq)]
pub struct Word {
    pub text: String,
    pub x: f32,
    pub y: f32,
    pub w: f32,
    pub h: f32,
}

/// One recognised line.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    pub text: String,
    pub words: Vec<Word>,
}

impl Line {
    /// Left edge and vertical centre of the line's box.
    pub fn x(&self) -> f32 {
        self.words.iter().map(|w| w.x).fold(f32::INFINITY, f32::min)
    }
    pub fn cy(&self) -> f32 {
        let n = self.words.len().max(1);
        #[allow(clippy::cast_precision_loss)]
        let n = n as f32;
        self.words.iter().map(|w| w.y + w.h / 2.0).sum::<f32>() / n
    }
}

/// Which engine read the text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EngineKind {
    Windows,
    Tesseract,
}

impl EngineKind {
    pub fn label(self) -> &'static str {
        match self {
            EngineKind::Windows => "Windows OCR (Windows.Media.Ocr, offline)",
            EngineKind::Tesseract => "Tesseract (local command-line program)",
        }
    }
}

pub trait Ocr {
    fn kind(&self) -> EngineKind;
    /// Reads every line of text in `img`.
    ///
    /// # Errors
    /// The engine failed (it is then up to the caller to try the fallback).
    fn read(&mut self, img: &Frame) -> Result<Vec<Line>, String>;
}

/// The Windows OCR engine, English (en-US) if installed, else the user's profile languages.
pub struct WinOcr {
    engine: windows::Media::Ocr::OcrEngine,
    max_dim: u32,
}

impl WinOcr {
    /// # Errors
    /// No OCR language is installed, or `WinRT` is unavailable.
    pub fn new() -> Result<WinOcr, String> {
        use windows::Globalization::Language;
        use windows::Media::Ocr::OcrEngine;
        let e = |x: windows::core::Error| format!("Windows OCR: {x}");
        let lang = Language::CreateLanguage(&windows::core::HSTRING::from("en-US")).map_err(e)?;
        let engine = if OcrEngine::IsLanguageSupported(&lang).map_err(e)? {
            OcrEngine::TryCreateFromLanguage(&lang).map_err(e)?
        } else {
            OcrEngine::TryCreateFromUserProfileLanguages().map_err(e)?
        };
        let max_dim = OcrEngine::MaxImageDimension().map_err(e)?;
        Ok(WinOcr { engine, max_dim })
    }
}

impl Ocr for WinOcr {
    fn kind(&self) -> EngineKind {
        EngineKind::Windows
    }

    fn read(&mut self, img: &Frame) -> Result<Vec<Line>, String> {
        use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap};
        use windows::Storage::Streams::DataWriter;
        let e = |x: windows::core::Error| format!("Windows OCR: {x}");
        if img.w > self.max_dim || img.h > self.max_dim {
            return Err(format!(
                "Windows OCR: image {} x {} above the engine limit {}",
                img.w, img.h, self.max_dim
            ));
        }
        let wr = DataWriter::new().map_err(e)?;
        wr.WriteBytes(&img.bgra).map_err(e)?;
        let buf = wr.DetachBuffer().map_err(e)?;
        let w = i32::try_from(img.w).map_err(|x| x.to_string())?;
        let h = i32::try_from(img.h).map_err(|x| x.to_string())?;
        let bmp = SoftwareBitmap::CreateCopyWithAlphaFromBuffer(
            &buf,
            BitmapPixelFormat::Bgra8,
            w,
            h,
            BitmapAlphaMode::Ignore,
        )
        .map_err(e)?;
        let res = self
            .engine
            .RecognizeAsync(&bmp)
            .map_err(e)?
            .join()
            .map_err(e)?;
        let mut out = Vec::new();
        for line in res.Lines().map_err(e)? {
            let mut words = Vec::new();
            for wd in line.Words().map_err(e)? {
                let r = wd.BoundingRect().map_err(e)?;
                words.push(Word {
                    text: wd.Text().map_err(e)?.to_string_lossy(),
                    x: r.X,
                    y: r.Y,
                    w: r.Width,
                    h: r.Height,
                });
            }
            out.push(Line {
                text: line.Text().map_err(e)?.to_string_lossy(),
                words,
            });
        }
        Ok(out)
    }
}

/// Tesseract through stdin/stdout (`tesseract stdin stdout --psm 6 tsv`).
pub struct Tesseract {
    exe: String,
}

impl Tesseract {
    /// Finds `tesseract` on PATH or in its default install folder.
    pub fn find() -> Option<Tesseract> {
        let mut cands = vec!["tesseract".to_string()];
        for v in ["ProgramFiles", "ProgramFiles(x86)", "LOCALAPPDATA"] {
            if let Some(p) = std::env::var_os(v) {
                let mut pb = std::path::PathBuf::from(p);
                if v == "LOCALAPPDATA" {
                    pb.push("Programs");
                }
                pb.push("Tesseract-OCR");
                pb.push("tesseract.exe");
                cands.push(pb.to_string_lossy().into_owned());
            }
        }
        cands
            .into_iter()
            .find(|c| {
                Command::new(c)
                    .arg("--version")
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()
                    .is_ok_and(|s| s.success())
            })
            .map(|exe| Tesseract { exe })
    }
}

impl Ocr for Tesseract {
    fn kind(&self) -> EngineKind {
        EngineKind::Tesseract
    }

    fn read(&mut self, img: &Frame) -> Result<Vec<Line>, String> {
        let mut child = Command::new(&self.exe)
            .args(["stdin", "stdout", "--psm", "6", "tsv"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|x| format!("Tesseract: {x}"))?;
        let ppm = img.to_ppm();
        if let Some(mut si) = child.stdin.take() {
            si.write_all(&ppm).map_err(|x| format!("Tesseract: {x}"))?;
        }
        let mut s = String::new();
        if let Some(mut so) = child.stdout.take() {
            so.read_to_string(&mut s)
                .map_err(|x| format!("Tesseract: {x}"))?;
        }
        child.wait().map_err(|x| format!("Tesseract: {x}"))?;
        Ok(parse_tsv(&s))
    }
}

/// Tesseract TSV: level `page_num` `block_num` `par_num` `line_num` `word_num` left top width height conf text
fn parse_tsv(s: &str) -> Vec<Line> {
    let mut out: Vec<((u32, u32, u32), Line)> = Vec::new();
    for row in s.lines().skip(1) {
        let c: Vec<&str> = row.split('\t').collect();
        if c.len() < 12 || c[0] != "5" {
            continue;
        }
        let t = c[11].trim();
        if t.is_empty() {
            continue;
        }
        let n = |i: usize| c[i].parse::<u32>().unwrap_or(0);
        let f = |i: usize| c[i].parse::<f32>().unwrap_or(0.0);
        let key = (n(2), n(3), n(4));
        let w = Word {
            text: t.to_string(),
            x: f(6),
            y: f(7),
            w: f(8),
            h: f(9),
        };
        match out.iter_mut().find(|(k, _)| *k == key) {
            Some((_, l)) => {
                l.text.push(' ');
                l.text.push_str(t);
                l.words.push(w);
            }
            None => out.push((
                key,
                Line {
                    text: t.to_string(),
                    words: vec![w],
                },
            )),
        }
    }
    out.into_iter().map(|(_, l)| l).collect()
}

/// The engine chain: Windows OCR first, Tesseract when Windows OCR cannot start or fails.
pub struct Engines {
    pub primary: Option<WinOcr>,
    pub fallback: Option<Tesseract>,
    /// Engine that produced the latest result.
    pub last: Option<EngineKind>,
    pub last_error: Option<String>,
}

impl Engines {
    pub fn detect() -> Engines {
        let (primary, err) = match WinOcr::new() {
            Ok(w) => (Some(w), None),
            Err(e) => (None, Some(e)),
        };
        let fallback = Tesseract::find();
        Engines {
            primary,
            fallback,
            last: None,
            last_error: err,
        }
    }

    pub fn available(&self) -> bool {
        self.primary.is_some() || self.fallback.is_some()
    }

    /// Reads with the primary engine, falling back to Tesseract on error.
    ///
    /// # Errors
    /// Both engines failed or none is installed.
    pub fn read(&mut self, img: &Frame) -> Result<Vec<Line>, String> {
        if let Some(p) = self.primary.as_mut() {
            match p.read(img) {
                Ok(l) => {
                    self.last = Some(EngineKind::Windows);
                    return Ok(l);
                }
                Err(e) => self.last_error = Some(e),
            }
        }
        if let Some(t) = self.fallback.as_mut() {
            let l = t.read(img)?;
            self.last = Some(EngineKind::Tesseract);
            return Ok(l);
        }
        Err(self
            .last_error
            .clone()
            .unwrap_or_else(|| "no OCR engine available".into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tsv_lines() {
        let s = "level\tpage_num\tblock_num\tpar_num\tline_num\tword_num\tleft\ttop\twidth\theight\tconf\ttext\n\
5\t1\t1\t1\t1\t1\t10\t5\t30\t12\t95\tBitcoin\n\
5\t1\t1\t1\t1\t2\t45\t5\t8\t12\t95\t/\n\
5\t1\t1\t1\t2\t1\t10\t30\t50\t12\t90\t60,626.47\n";
        let l = parse_tsv(s);
        assert_eq!(l.len(), 2);
        assert_eq!(l[0].text, "Bitcoin /");
        assert_eq!(l[1].text, "60,626.47");
        assert!((l[0].x() - 10.0).abs() < 1e-6);
    }
}
