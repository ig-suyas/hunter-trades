//! Turning OCR text (and window titles) into a ticker, a time frame and a price. Tolerant of the
//! usual OCR slips on chart fonts (O/0, l/1, S/5 inside numbers, "·" read as "." or "•").

use cs_core::Timeframe;

/// A time frame as the platform writes it, and the candle length it maps to (if any).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TfRead {
    /// The platform's label as written (after OCR fixes), e.g. "15", "1h", "D", "1D", "1W".
    pub raw: String,
    /// `None` for intervals the workspace has no type for (3m, 30m, 2h, 1W, 1M, 10S ...).
    /// A daily chart ("D" or "1D") maps to `D1`, which has no live feed: it is shown as an
    /// unsupported time frame, never treated as an intraday one.
    pub tf: Option<Timeframe>,
}

impl TfRead {
    /// Label for the panel: plain minute numbers get "m" ("15" -> "15m", "60" -> "1h"); any
    /// other label is shown exactly as the platform writes it ("D", "1D", "1W").
    pub fn label(&self) -> String {
        if self.raw.chars().all(|c| c.is_ascii_digit()) {
            return self
                .tf
                .map_or_else(|| format!("{}m", self.raw), |t| t.label().to_string());
        }
        self.raw.clone()
    }
}

/// Interval values `TradingView` writes as plain minutes.
const MINUTE_CODES: [u32; 17] = [
    1, 2, 3, 5, 10, 15, 30, 45, 60, 90, 120, 180, 240, 360, 480, 720, 1440,
];

/// `TradingView` interval codes: minutes as plain numbers ("15", "60", "240"), or a count and
/// a unit: S seconds, m minutes, h hours, D days ("D" = "1D"), W weeks, M months.
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
        // Only interval values the platform offers, so a stray number in a description
        // ("S&P 500") is not taken for a time frame.
        if !MINUTE_CODES.contains(&m) {
            return None;
        }
        return Some(TfRead {
            raw: t.clone(),
            tf: minutes(m),
        });
    }
    let (num, unit) = t.split_at(t.len() - 1);
    // A bare unit is only written for days, weeks and months ("D", "W", "M"); a bare "S" or
    // "m" is a letter of a word ("U.S.").
    if num.is_empty() && !matches!(unit, "D" | "W" | "M") {
        return None;
    }
    let n: u32 = if num.is_empty() { 1 } else { num.parse().ok()? };
    if n == 0 || n > 720 {
        return None;
    }
    let tf = match unit {
        "m" => minutes(n),
        "h" | "H" => minutes(n * 60),
        "D" | "d" => (n == 1).then_some(Timeframe::D1),
        "W" | "w" | "M" | "S" => None,
        _ => return None,
    };
    Some(TfRead { raw: t, tf })
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
    exchange_upper(tok).or_else(|| exchange_name(tok))
}

/// An exchange or source in capitals ("CRYPTO", "BINANCE", OCR's "CRYPT0").
fn exchange_upper(tok: &str) -> Option<String> {
    let t = tok.trim_matches(|c: char| !c.is_alphanumeric());
    if t.len() < 3 || t.len() > 16 {
        return None;
    }
    let fixed: String = t.chars().map(|c| if c == '0' { 'O' } else { c }).collect();
    fixed
        .chars()
        .all(|c| c.is_ascii_uppercase())
        .then_some(fixed)
}

/// Newer legends write the exchange as a name ("Bitstamp", "Binance").
fn exchange_name(tok: &str) -> Option<String> {
    let t = tok.trim_matches(|c: char| !c.is_alphanumeric());
    if t.len() > 16 {
        return None;
    }
    let mut cs = t.chars();
    let capitalised = t.len() >= 4
        && cs.next().is_some_and(|c| c.is_ascii_uppercase())
        && cs.all(|c| c.is_ascii_lowercase());
    capitalised.then(|| t.to_uppercase())
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
    // First an exchange in capitals, then a capitalised name; and the interval as read before
    // one with OCR's digit look-alikes fixed ("IS" -> "15").
    let passes: [(ExchangeFn, bool); 4] = [
        (exchange_upper, false),
        (exchange_name, false),
        (exchange_upper, true),
        (exchange_name, true),
    ];
    for (exch, fix) in passes {
        if let Some(l) = legend_pass(&toks, exch, fix) {
            return Some(l);
        }
    }
    None
}

type ExchangeFn = fn(&str) -> Option<String>;

/// The interval with OCR's digit look-alikes fixed; needs two or three characters, so a lone
/// letter ("S" of "U.S.") never turns into a number.
fn fixed_tf(t: &str) -> Option<TfRead> {
    let core = t.trim_matches(|c: char| !c.is_alphanumeric());
    (2..=3)
        .contains(&core.chars().count())
        .then(|| parse_tf(&fix_digits(core)))
        .flatten()
}

fn legend_pass(toks: &[String], exch: ExchangeFn, fix: bool) -> Option<Legend> {
    for i in 1..toks.len().saturating_sub(1) {
        let tf = if fix {
            fixed_tf(&toks[i])
        } else {
            parse_tf(&toks[i])
        };
        let Some(tf) = tf else {
            continue;
        };
        let Some(exchange) = exch(&toks[i + 1]) else {
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

/// A change such as "+1.69%", "−0.12%" (minus sign), "-0,5%" or "1.7%".
fn is_change(tok: &str) -> bool {
    let Some(num) = tok.strip_suffix('%') else {
        return false;
    };
    let num = num.trim_start_matches(['+', '-', '\u{2212}']);
    !num.is_empty()
        && num.chars().any(|c| c.is_ascii_digit())
        && num
            .chars()
            .all(|c| c.is_ascii_digit() || c == '.' || c == ',')
}

/// Parses the window title of a chart tab against the config's title patterns. A pattern is a
/// list of space-separated parts matched against the start of the title, part by part:
/// `{ticker}` (upper-case symbol), `{price}`, `{arrow}` (▲ or ▼), `{change}` ("+1.69%"), or
/// literal text. Browsers append " - Google Chrome", " — Mozilla Firefox" and the like after
/// the matched part; that tail is ignored.
pub fn parse_title(title: &str, patterns: &[String]) -> Option<TitleRead> {
    let toks: Vec<&str> = title.split_whitespace().collect();
    'pattern: for p in patterns {
        let parts: Vec<&str> = p.split_whitespace().collect();
        if parts.is_empty() || parts.len() > toks.len() {
            continue;
        }
        let (mut ticker, mut price) = (None, None);
        for (part, tok) in parts.iter().zip(&toks) {
            let ok = match *part {
                "{ticker}" => {
                    ticker = normalise_ticker(tok)
                        .filter(|_| !tok.chars().any(|c| c.is_ascii_lowercase()));
                    ticker.is_some()
                }
                "{price}" => {
                    price = parse_price(tok);
                    price.is_some()
                }
                "{arrow}" => matches!(*tok, "\u{25B2}" | "\u{25BC}"),
                "{change}" => is_change(tok),
                lit => lit == *tok,
            };
            if !ok {
                continue 'pattern;
            }
        }
        if let Some(ticker) = ticker {
            return Some(TitleRead { ticker, price });
        }
    }
    None
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
        assert_eq!(parse_tf("3").unwrap().label(), "3m");
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
        let pats = crate::VisionConfig::default_config()
            .unwrap()
            .platforms
            .into_iter()
            .flat_map(|p| p.title_patterns)
            .collect::<Vec<_>>();
        // Real Chrome title seen on this machine (2026-11-27), then real-style Edge, Firefox
        // and desktop-app titles: falling price, minus sign, comma decimals, layout name.
        for (title, ticker, price) in [
            (
                "BTCUSD 61,515 \u{25B2} +1.69% - Google Chrome",
                "BTCUSD",
                Some(61_515.0),
            ),
            (
                "BTCUSD 61,521 \u{25B2} +1.7% - Google Chrome",
                "BTCUSD",
                Some(61_521.0),
            ),
            (
                "BTCUSD 60,626.47 \u{25B2} +0.41% Unnamed - Google Chrome",
                "BTCUSD",
                Some(60_626.47),
            ),
            (
                "ETHUSDT.P 1,704.15 \u{25BC} \u{2212}0.12% Unnamed and 2 more pages - Personal - Microsoft\u{200B} Edge",
                "ETHUSDT.P",
                Some(1_704.15),
            ),
            (
                "BTCUSDT 61,480.2 \u{25BC} -0.35% \u{2014} Mozilla Firefox",
                "BTCUSDT",
                Some(61_480.2),
            ),
            (
                "XAUUSD 3,566.250 \u{25BC} \u{2212}0.65% My layout",
                "XAUUSD",
                Some(3_566.25),
            ),
            (
                "BTCUSD 61,515 +1,69% - Google Chrome",
                "BTCUSD",
                Some(61_515.0),
            ),
        ] {
            let t = parse_title(title, &pats).unwrap_or_else(|| panic!("{title}"));
            assert_eq!(t.ticker, ticker, "{title}");
            assert_eq!(t.price.is_some(), price.is_some(), "{title}");
            if let Some(p) = price {
                assert!(close(t.price, p), "{title}: {:?}", t.price);
            }
        }
        for title in [
            "TradingView \u{2014} Track All Markets - Google Chrome",
            "Inbox (3) - Mail - Google Chrome",
            "60,728.6 BTCUSDT +1.4% | Binance Futures - Google Chrome",
            "NEWS 2024 - Google Chrome",
            "BTCUSD price today - Google Chrome",
            "btcusd 61,515 \u{25B2} +1.69% - Google Chrome",
        ] {
            assert!(parse_title(title, &pats).is_none(), "{title}");
        }
    }

    #[test]
    fn daily_and_other_labels_are_kept_as_written() {
        for (tok, label, tf) in [
            ("D", "D", Some(Timeframe::D1)),
            ("1D", "1D", Some(Timeframe::D1)),
            ("W", "W", None),
            ("1W", "1W", None),
            ("1M", "1M", None),
            ("3", "3m", None),
            ("15", "15m", Some(Timeframe::M15)),
            ("60", "1h", Some(Timeframe::H1)),
            ("4h", "4h", Some(Timeframe::H4)),
            ("10S", "10S", None),
        ] {
            let r = parse_tf(tok).unwrap_or_else(|| panic!("{tok}"));
            assert_eq!((r.label().as_str(), r.tf), (label, tf), "{tok}");
        }
        // Letters of words are not intervals; nor are numbers the platform does not offer.
        for tok in ["S", "m", "500", "7"] {
            assert!(parse_tf(tok).is_none(), "{tok}");
        }
        let l = parse_legend("Bitcoin / U.S. Dollar \u{00B7} 1D \u{00B7} Bitstamp").unwrap();
        assert_eq!((l.tf.raw.as_str(), l.exchange.as_str()), ("1D", "BITSTAMP"));
        let l = parse_legend("Bitcoin / U.S. Dollar \u{00B7} D \u{00B7} BITSTAMP").unwrap();
        assert_eq!(l.tf.label(), "D");
        assert_eq!(l.description, "bitcoinusdollar");
        assert!(parse_legend("S&P 500 Index \u{00B7} Cboe").is_none());
        // Seen on the window-sized trading1 frames (230 s and 10 s): "15" read as "IS", and
        // the legend split into lines with a lone "S." before "Dollar".
        let l = parse_legend("Bitcoin / U.S. Dollar \u{2022} IS CRYPTO").unwrap();
        assert_eq!(l.tf.tf, Some(Timeframe::M15));
        let l = parse_legend("Bitcoin S. Dollar \u{2022} 15 CRYPTO").unwrap();
        assert_eq!((l.tf.raw.as_str(), l.exchange.as_str()), ("15", "CRYPTO"));
    }
}
