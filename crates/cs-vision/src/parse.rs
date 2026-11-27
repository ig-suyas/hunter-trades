//! Turning OCR text (and window titles) into a ticker, a time frame and a price. Tolerant of the
//! usual OCR slips on chart fonts (O/0, l/1, S/5 inside numbers, "·" read as "." or "•").

use cs_core::Timeframe;

/// A time frame as the platform writes it, and the candle length it maps to (if any).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TfRead {
    /// Normalised platform label, e.g. "15", "1h", "1D".
    pub raw: String,
    /// `None` for intervals the workspace has no type for (3m, 30m, 2h, 1W, ...).
    pub tf: Option<Timeframe>,
}

impl TfRead {
    /// Human label: "15m", "1h", or the raw label.
    pub fn label(&self) -> String {
        self.tf
            .map_or_else(|| self.raw.clone(), |t| t.label().to_string())
    }
}

/// `TradingView` interval codes: minutes as plain numbers, then h/D/W/M (M = month).
pub fn parse_tf(tok: &str) -> Option<TfRead> {
    let t = tok.trim().trim_matches(|c: char| !c.is_alphanumeric());
    if t.is_empty() || t.len() > 4 {
        return None;
    }
    // OCR reads "1h" as "lh" or "Ih".
    let t = match t {
        "lh" | "Ih" | "lH" | "IH" => "1h".to_string(),
        "lD" | "ID" => "1D".to_string(),
        "lW" | "IW" => "1W".to_string(),
        _ => t.to_string(),
    };
    let minutes = |m: u32| -> Option<Timeframe> {
        match m {
            1 => Some(Timeframe::M1),
            5 => Some(Timeframe::M5),
            15 => Some(Timeframe::M15),
            60 => Some(Timeframe::H1),
            240 => Some(Timeframe::H4),
            1440 => Some(Timeframe::D1),
            _ => None,
        }
    };
    if let Ok(m) = t.parse::<u32>() {
        if m == 0 || m > 1440 {
            return None;
        }
        return Some(TfRead {
            raw: t.clone(),
            tf: minutes(m),
        });
    }
    let (num, unit) = t.split_at(t.len() - 1);
    let n: u32 = if num.is_empty() { 1 } else { num.parse().ok()? };
    if n == 0 {
        return None;
    }
    let tf = match unit {
        "m" => minutes(n),
        "h" | "H" => minutes(n * 60),
        "D" | "d" => (n == 1).then_some(Timeframe::D1),
        "W" | "w" | "M" => None,
        _ => return None,
    };
    let unit = match unit {
        "H" => "h",
        "d" => "D",
        "w" => "W",
        u => u,
    };
    Some(TfRead {
        raw: format!("{n}{unit}"),
        tf,
    })
}

/// Digit fixes inside a token that is mostly digits.
fn fix_digits(tok: &str) -> String {
    tok.chars()
        .map(|c| match c {
            'O' | 'o' | 'Q' | 'D' => '0',
            'l' | 'I' | 'i' | '|' | '!' => '1',
            'S' | 's' => '5',
            'B' => '8',
            'Z' | 'z' => '2',
            'g' => '9',
            c => c,
        })
        .collect()
}

/// A price such as "60,626.47", "60 626.47", "1,704.15" or "0.08123"; `None` for times
/// ("12:34"), percentages and anything that does not look like a price.
pub fn parse_price(tok: &str) -> Option<f64> {
    parse_price_dec(tok).map(|(v, _)| v)
}

/// [`parse_price`] plus the number of decimals as written ("60,617.0" -> 1), so two reads of a
/// label agree only when they read the same digits.
pub fn parse_price_dec(tok: &str) -> Option<(f64, usize)> {
    let t = tok
        .trim()
        .trim_matches(|c: char| matches!(c, '\'' | '"' | '`' | '_' | '•'));
    // A number cut short ("60,617." before garbage) is not a price.
    if t.ends_with(['.', ',', '-']) {
        return None;
    }
    let t = t.trim_start_matches(['.', ',', '-']);
    if t.is_empty() || t.contains(':') || t.contains('%') {
        return None;
    }
    let digits = t.chars().filter(char::is_ascii_digit).count();
    if digits < 2 || digits * 2 < t.chars().filter(|c| !matches!(c, ',' | '.' | ' ')).count() {
        return None;
    }
    let mut f = fix_digits(t);
    // "60,656-51": a decimal point read as a dash (only between digits, before the last 1-4).
    if let Some(i) = f.rfind('-')
        && !f.contains('.')
        && i > 0
        && (1..=4).contains(&(f.len() - i - 1))
        && f[i + 1..].chars().all(|c| c.is_ascii_digit())
    {
        f.replace_range(i..=i, ".");
    }
    if !f
        .chars()
        .all(|c| c.is_ascii_digit() || c == ',' || c == '.')
    {
        return None;
    }
    // Thousands separators: groups of exactly 3 digits after each comma.
    let (int, frac) = match f.rsplit_once('.') {
        Some((a, b)) => (a, Some(b)),
        None => (f.as_str(), None),
    };
    if int.contains('.') || int.is_empty() {
        return None;
    }
    let groups: Vec<&str> = int.split(',').collect();
    if groups.len() > 1
        && (groups[0].is_empty() || groups[0].len() > 3 || groups[1..].iter().any(|g| g.len() != 3))
    {
        return None;
    }
    let mut s: String = groups.concat();
    let dec = frac.map_or(0, str::len);
    if let Some(fr) = frac {
        if fr.is_empty() || fr.contains(',') {
            return None;
        }
        s.push('.');
        s.push_str(fr);
    }
    let v: f64 = s.parse().ok()?;
    (v.is_finite() && v > 0.0).then_some((v, dec))
}

/// The first price in a block of OCR lines (top to bottom), joining a number that OCR split at
/// a thousands comma ("61, 110.73").
pub fn first_price(lines: &[String]) -> Option<f64> {
    first_price_dec(lines).map(|(v, _)| v)
}

/// [`first_price`] with the decimals as written.
pub fn first_price_dec(lines: &[String]) -> Option<(f64, usize)> {
    for l in lines {
        let joined = l.replace(", ", ",").replace(" ,", ",");
        for tok in joined.split_whitespace() {
            if let Some(p) = parse_price_dec(tok) {
                return Some(p);
            }
        }
    }
    None
}

/// A ticker as shown on the platform: "BTCUSD", "BINANCE:BTCUSDT.P" -> "BTCUSDT.P".
pub fn normalise_ticker(s: &str) -> Option<String> {
    let t = s.trim().trim_matches(|c: char| !c.is_alphanumeric());
    let t = t.rsplit(':').next().unwrap_or(t);
    // OCR sometimes reads a gap between letters as a dash ("BTC-USD").
    let up = t.to_uppercase().replace('-', "");
    let ok = (3..=20).contains(&up.len())
        && up
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || matches!(c, '.' | '!' | '_'))
        && up.chars().filter(char::is_ascii_uppercase).count() >= 3;
    ok.then_some(up)
}

/// The ticker in the symbol region: the first word that looks like a ticker.
pub fn ticker_from_lines(lines: &[String]) -> Option<String> {
    lines
        .iter()
        .flat_map(|l| l.split_whitespace())
        // Real tickers are upper case on screen; this skips stray lower-case words.
        .filter(|w| {
            let letters: Vec<char> = w.chars().filter(char::is_ascii_alphabetic).collect();
            !letters.is_empty() && letters.iter().all(char::is_ascii_uppercase)
        })
        .find_map(normalise_ticker)
}

/// Parsed chart legend, e.g. "Bitcoin / U.S. Dollar · 15 · CRYPTO".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Legend {
    /// Letters of the description, lower case ("bitcoinusdollar").
    pub description: String,
    pub tf: TfRead,
    /// Exchange or source as written ("CRYPTO", "BINANCE").
    pub exchange: String,
}

/// Letters only, lower case.
pub fn letters(s: &str) -> String {
    s.chars()
        .filter(char::is_ascii_alphabetic)
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn exchange_like(tok: &str) -> Option<String> {
    let t = tok.trim_matches(|c: char| !c.is_alphanumeric());
    if t.len() < 3 || t.len() > 16 {
        return None;
    }
    // "CRYPT0" -> "CRYPTO".
    let fixed: String = t.chars().map(|c| if c == '0' { 'O' } else { c }).collect();
    fixed
        .chars()
        .all(|c| c.is_ascii_uppercase())
        .then_some(fixed)
}

/// Finds "<description> <sep> <interval> <sep> <EXCHANGE>" in the legend text.
pub fn parse_legend(text: &str) -> Option<Legend> {
    // Separators: the middle dot and its OCR look-alikes become spaces; a dot or comma glued to
    // a token ("Dollar." / ".15." / "15.CRYPTO") is split off below.
    let spaced: String = text
        .chars()
        .map(|c| match c {
            '\u{00B7}' | '\u{2022}' | '\u{2219}' | '\u{22C5}' | '\u{30FB}' | '*' => ' ',
            c => c,
        })
        .collect();
    let mut toks: Vec<String> = Vec::new();
    for w in spaced.split_whitespace() {
        // Split "Dollar.15.CRYPTO" style runs at dots that sit between a letter or digit run
        // and the next run, but keep "U.S." intact by splitting only around digit runs and
        // all-caps runs of 3+ letters.
        let mut cur = String::new();
        let parts: Vec<&str> = w.split('.').collect();
        for (i, p) in parts.iter().enumerate() {
            let standalone = parse_tf(p).is_some() || exchange_like(p).is_some();
            if standalone && !p.is_empty() {
                if !cur.is_empty() {
                    toks.push(std::mem::take(&mut cur));
                }
                toks.push((*p).to_string());
            } else {
                cur.push_str(p);
                if i + 1 < parts.len() {
                    cur.push('.');
                }
            }
        }
        if !cur.is_empty() && cur != "." {
            toks.push(cur);
        }
    }
    for i in 1..toks.len().saturating_sub(1) {
        let Some(tf) = parse_tf(&toks[i]) else {
            continue;
        };
        let Some(exchange) = exchange_like(&toks[i + 1]) else {
            continue;
        };
        // The description: the tokens before, minus leading icon noise ("O", "0", "C)").
        let desc: Vec<&str> = toks[..i]
            .iter()
            .map(String::as_str)
            .skip_while(|t| letters(t).len() <= 1 && !t.contains('/'))
            .collect();
        let description = letters(&desc.join(" "));
        if description.len() < 3 {
            continue;
        }
        return Some(Legend {
            description,
            tf,
            exchange,
        });
    }
    None
}

/// `TradingView`'s chart tab title: "BTCUSD 60,626.47 ▲ +0.41% Unnamed".
#[derive(Debug, Clone, PartialEq)]
pub struct TitleRead {
    pub ticker: String,
    pub price: Option<f64>,
}

/// Parses the window title of a chart tab (browsers append " - Google Chrome" etc.).
pub fn parse_title(title: &str) -> Option<TitleRead> {
    let mut it = title.split_whitespace();
    let first = it.next()?;
    let ticker = normalise_ticker(first)?;
    if first.chars().any(|c| c.is_ascii_lowercase()) {
        return None;
    }
    let price = it.next().and_then(parse_price);
    // The chart title always has a price and a change arrow or percentage after the ticker.
    let rest: String = title.chars().skip(first.len()).collect();
    let shaped = rest.contains('\u{25B2}')
        || rest.contains('\u{25BC}')
        || rest.contains('%')
        || rest.contains('\u{2212}');
    (price.is_some() && shaped).then_some(TitleRead { ticker, price })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(a: Option<f64>, b: f64) -> bool {
        a.is_some_and(|x| (x - b).abs() < 1e-9)
    }

    #[test]
    fn time_frames() {
        assert_eq!(parse_tf("15").unwrap().tf, Some(Timeframe::M15));
        assert_eq!(parse_tf("5").unwrap().tf, Some(Timeframe::M5));
        assert_eq!(parse_tf("60").unwrap().tf, Some(Timeframe::H1));
        assert_eq!(parse_tf("1h").unwrap().tf, Some(Timeframe::H1));
        assert_eq!(parse_tf("lh").unwrap().tf, Some(Timeframe::H1));
        assert_eq!(parse_tf("240").unwrap().tf, Some(Timeframe::H4));
        assert_eq!(parse_tf("1D").unwrap().tf, Some(Timeframe::D1));
        let m = parse_tf("1M").unwrap();
        assert_eq!(m.tf, None, "1M is one month, not one minute");
        assert_eq!(parse_tf("3").unwrap().tf, None);
        assert_eq!(parse_tf("3").unwrap().label(), "3");
        assert!(parse_tf("CRYPTO").is_none());
        assert!(parse_tf("0").is_none());
    }

    #[test]
    fn prices() {
        assert!(close(parse_price("60,626.47"), 60_626.47));
        assert!(close(parse_price("61,000.0'"), 61_000.0));
        assert!(close(parse_price("1,704.15"), 1_704.15));
        assert!(close(parse_price("60,6S2.94"), 60_652.94));
        assert!(close(parse_price("0.08123"), 0.081_23));
        assert!(close(parse_price("6O,626.47"), 60_626.47));
        assert!(close(parse_price("60,656-51"), 60_656.51));
        assert!(parse_price("12:34").is_none());
        assert!(parse_price("+0.41%").is_none());
        assert!(parse_price("60,62.47").is_none(), "bad grouping");
        assert!(parse_price("CRYPTO").is_none());
        assert!(close(first_price(&["61, 110.73".into()]), 61_110.73));
        assert!(close(
            first_price(&["x".into(), "60,659.52".into(), "08:21".into()]),
            60_659.52
        ));
    }

    #[test]
    fn tickers() {
        assert_eq!(normalise_ticker("BINANCE:BTCUSDT.P").unwrap(), "BTCUSDT.P");
        assert_eq!(ticker_from_lines(&["BTCUSD".into()]).unwrap(), "BTCUSD");
        assert_eq!(ticker_from_lines(&["BTC-USD".into()]).unwrap(), "BTCUSD");
        assert_eq!(
            ticker_from_lines(&["@ 1m".into(), "ETHUSDT".into()]).unwrap(),
            "ETHUSDT"
        );
        assert!(ticker_from_lines(&["Indicators".into()]).is_none());
    }

    #[test]
    fn legends_as_ocr_reads_them() {
        for (s, tf) in [
            ("0 Bitcoin / U.S. Dollar. 15.CRYPT0", "15"),
            ("C) Bitcoin / U.S. Dollar. 15. CRYPTO", "15"),
            ("O Bitcoin / U.S. Dollar \u{00B7} 15 \u{00B7} CRYPTO", "15"),
            (
                "O Bitcoin / U.S. Dollar. 15. CRYPTO open z Indicativedata",
                "15",
            ),
            ("Bitcoin / TetherUS \u{2022} 1h \u{2022} BINANCE", "1h"),
            ("Bitcoin / TetherUS \u{00B7} 60 \u{00B7} BINANCE", "60"),
        ] {
            let l = parse_legend(s).unwrap_or_else(|| panic!("{s}"));
            assert_eq!(l.tf.raw, tf, "{s}");
            assert!(l.description.starts_with("bitcoin"), "{s}: {l:?}");
        }
        let l = parse_legend("0 Bitcoin / U.S. Dollar. 15.CRYPT0").unwrap();
        assert_eq!(l.description, "bitcoinusdollar");
        assert_eq!(l.exchange, "CRYPTO");
        let e = parse_legend("Ethereum / TetherUS \u{00B7} 5 \u{00B7} BINANCE").unwrap();
        assert_eq!(
            (e.description.as_str(), e.tf.tf),
            ("ethereumtetherus", Some(Timeframe::M5))
        );
        assert!(parse_legend("Bitcoin / U.S.").is_none());
        assert!(parse_legend("Indicators Alert Replay").is_none());
    }

    #[test]
    fn titles() {
        let t = parse_title("BTCUSD 60,626.47 \u{25B2} +0.41% Unnamed - Google Chrome").unwrap();
        assert_eq!(t.ticker, "BTCUSD");
        assert!(close(t.price, 60_626.47));
        let t = parse_title("ETHUSDT.P 1,704.15 \u{25BC} \u{2212}0.12% Unnamed").unwrap();
        assert_eq!(t.ticker, "ETHUSDT.P");
        assert!(parse_title("TradingView Black Friday Sale - Google Chrome").is_none());
        assert!(parse_title("Inbox (3) - Mail").is_none());
    }
}
