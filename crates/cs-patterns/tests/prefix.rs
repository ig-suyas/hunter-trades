//! Prefix invariance: for random prefixes of the data, the events produced from the prefix must
//! equal the events from the full data whose `known_idx` lies inside the prefix. A second check
//! replaces everything after the cut with a different future and requires the same events up to
//! the cut. Every detector group in the registry gets its own test, on synthetic regime-switching
//! data (4 seeds) and, when the Phase 1 store is present, on real BTCUSDT USDS-M 15m candles.

// Test code: panics are the failure mode, sizes are tiny, printing is the report.
#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    clippy::print_stdout,
    clippy::cast_possible_truncation,
    clippy::cast_possible_wrap,
    clippy::unreadable_literal
)]

use std::collections::BTreeMap;
use std::path::PathBuf;

use cs_core::{Candle, Event};
use cs_patterns::context::{ContextParams, Features};
use cs_patterns::testutil::{STEP_MS, synthetic};
use cs_patterns::util::Rng;
use cs_patterns::{Detector, Runner, registry};

fn run(make: fn(i64) -> Box<dyn Detector>, data: &[Candle]) -> Vec<Event> {
    let mut r = Runner::new(vec![make(STEP_MS)], None);
    let mut out = Vec::new();
    for (i, c) in data.iter().enumerate() {
        let before = out.len();
        r.step(c, &mut out);
        for e in &out[before..] {
            let i = u32::try_from(i).unwrap();
            assert_eq!(
                e.known_idx, i,
                "{} {} emitted with known_idx != current bar",
                e.det, e.kind
            );
            assert!(
                e.formed_idx <= e.known_idx,
                "{} {} formed after known",
                e.det,
                e.kind
            );
            assert!(
                e.start_idx <= e.known_idx,
                "{} {} starts after known",
                e.det,
                e.kind
            );
            assert_ne!(
                e.formed_at,
                i64::MIN,
                "{} {} formed too far back",
                e.det,
                e.kind
            );
            assert!(e.formed_at <= e.known_at);
        }
    }
    out
}

fn compare(name: &str, cut: usize, got: &[Event], full: &[Event]) {
    let want: Vec<&Event> = full
        .iter()
        .filter(|e| (e.known_idx as usize) < cut)
        .collect();
    assert_eq!(
        got.len(),
        want.len(),
        "{name}: prefix {cut}: {} events from the prefix, {} from the full run",
        got.len(),
        want.len()
    );
    for (k, (a, b)) in got.iter().zip(want).enumerate() {
        assert!(
            a.same(b),
            "{name}: prefix {cut}: event {k} differs:\n{a:?}\n{b:?}"
        );
    }
}

/// A different future: same prefix, then a fresh synthetic path scaled to continue from the
/// last close.
fn alt_future(data: &[Candle], cut: usize, seed: u64) -> Vec<Candle> {
    let mut v = data[..cut].to_vec();
    let rest = data.len() - cut;
    if rest == 0 || cut == 0 {
        return v;
    }
    let alt = synthetic(rest + 1, seed);
    let scale = data[cut - 1].close / alt[0].open;
    let step = if cut >= 2 {
        data[cut - 1].open_time - data[cut - 2].open_time
    } else {
        900_000
    };
    for (k, c) in alt.iter().take(rest).enumerate() {
        let mut c = *c;
        c.open *= scale;
        c.high *= scale;
        c.low *= scale;
        c.close *= scale;
        c.open_time = data[cut - 1].open_time + step * (k as i64 + 1);
        v.push(c);
    }
    v
}

fn real_data() -> Option<Vec<Candle>> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../data/store/resampled/um/BTCUSDT/15m.csc");
    let t = cs_data::store::read_file(&p).ok()?;
    cs_data::store::table_to_candles(&t).ok().map(|x| x.0)
}

fn check_group(group: &str) {
    let entry = registry()
        .into_iter()
        .find(|e| e.group == group)
        .expect("group");
    let names = (entry.make)(STEP_MS).names();
    let mut counts: BTreeMap<&str, [usize; 2]> = names.iter().map(|n| (*n, [0, 0])).collect();
    let mut rng = Rng::new(0xC0FFEE ^ group.len() as u64);
    let mut sets: Vec<(usize, Vec<Candle>, usize)> = (0..4u64)
        .map(|s| (0, synthetic(6000, 1000 + s), 8))
        .collect();
    if let Some(real) = real_data() {
        sets.push((1, real, 6));
    }
    for (kind, data, nprefix) in &sets {
        let full = run(entry.make, data);
        for e in &full {
            if let Some(c) = counts.get_mut(e.det) {
                c[*kind] += 1;
            } else {
                panic!("{group}: emitted undeclared detector name {}", e.det);
            }
        }
        for k in 0..*nprefix {
            let cut = 1 + rng.below(data.len() as u64 - 1) as usize;
            let cut = if k == 0 { data.len() / 3 } else { cut };
            let got = run(entry.make, &data[..cut]);
            compare(group, cut, &got, &full);
            if k < 2 {
                let alt = alt_future(data, cut, 77 + k as u64);
                let alt_ev = run(entry.make, &alt);
                let alt_pre: Vec<Event> = alt_ev
                    .into_iter()
                    .filter(|e| (e.known_idx as usize) < cut)
                    .collect();
                compare(group, cut, &alt_pre, &full);
            }
        }
    }
    for (n, c) in &counts {
        let flag = if c[0] + c[1] == 0 {
            "  VACUOUS (no events)"
        } else {
            ""
        };
        println!(
            "prefix-invariance {group}/{n}: PASS (events synthetic={} real={}){flag}",
            c[0], c[1]
        );
    }
}

macro_rules! prefix_tests {
    ($($f:ident => $g:literal),* $(,)?) => {
        $( #[test] fn $f() { check_group($g); } )*
    };
}

prefix_tests! {
    prefix_swing => "swing",
    prefix_structure => "structure",
    prefix_bos_choch => "bos_choch",
    prefix_sr_zones => "sr_zones",
    prefix_sd_zones => "sd_zones",
    prefix_ema => "ema",
    prefix_trendline => "trendline",
    prefix_fib => "fib",
    prefix_pullback => "pullback",
    prefix_breakouts => "breakouts",
    prefix_sweeps => "sweeps",
    prefix_candles => "candles",
    prefix_htf_trend => "htf_trend",
}

#[test]
fn registry_is_fully_covered() {
    let groups: Vec<&str> = registry().iter().map(|e| e.group).collect();
    assert_eq!(
        groups.len(),
        13,
        "add a prefix test for every new registry group: {groups:?}"
    );
}

fn features(data: &[Candle]) -> Vec<Features> {
    let mut r = Runner::new(vec![], Some(ContextParams::default()));
    let mut out = Vec::new();
    data.iter().filter_map(|c| r.step(c, &mut out)).collect()
}

#[test]
fn prefix_context_features() {
    let mut sets = vec![synthetic(6000, 5), synthetic(6000, 6)];
    if let Some(r) = real_data() {
        sets.push(r);
    }
    let mut rng = Rng::new(9);
    for data in &sets {
        let full = features(data);
        assert_eq!(full.len(), data.len());
        for _ in 0..5 {
            let cut = 1 + rng.below(data.len() as u64 - 1) as usize;
            let pre = features(&data[..cut]);
            assert_eq!(pre.len(), cut);
            for (a, b) in pre.iter().zip(&full) {
                assert!(
                    a.same(b),
                    "context features differ at {} (cut {cut})",
                    a.idx
                );
            }
            let alt = features(&alt_future(data, cut, 3));
            for (a, b) in alt.iter().take(cut).zip(&full) {
                assert!(
                    a.same(b),
                    "context features see the future at {} (cut {cut})",
                    a.idx
                );
            }
        }
    }
    println!("prefix-invariance context: PASS");
}

/// Negative control: a swing detector that cheats by reading the whole data set and reporting a
/// pivot at the pivot bar itself (before its right-hand bars exist). The prefix check must
/// reject it, otherwise the harness proves nothing.
struct Cheat {
    data: std::rc::Rc<Vec<Candle>>,
}

impl Detector for Cheat {
    fn names(&self) -> &'static [&'static str] {
        &["cheat"]
    }

    fn update(&mut self, i: u32, c: &Candle, out: &mut Vec<Event>) {
        let i_us = i as usize;
        let d = &self.data;
        let right = d.get(i_us + 1..i_us + 4);
        let left = d.get(i_us.saturating_sub(3)..i_us);
        if let (Some(r), Some(l)) = (right, left)
            && l.len() == 3
            && r.iter().chain(l).all(|x| x.high < c.high)
        {
            out.push(Event::new("cheat", "peek_high", -1, i).with_price(c.high));
        }
    }
}

#[test]
fn prefix_check_catches_lookahead() {
    let data = synthetic(3000, 42);
    let run_cheat = |d: &[Candle]| {
        let rc = std::rc::Rc::new(d.to_vec());
        let mut r = Runner::new(vec![Box::new(Cheat { data: rc })], None);
        let mut out = Vec::new();
        for c in d {
            r.step(c, &mut out);
        }
        out
    };
    let full = run_cheat(&data);
    assert!(!full.is_empty());
    let mut caught = 0;
    for cut in (100..3000).step_by(37) {
        let got = run_cheat(&data[..cut]);
        let want = full.iter().filter(|e| (e.known_idx as usize) < cut).count();
        if got.len() != want {
            caught += 1;
        }
    }
    assert!(
        caught > 0,
        "the prefix check failed to catch a lookahead detector"
    );
    println!("negative control: lookahead caught at {caught} cuts");
}
