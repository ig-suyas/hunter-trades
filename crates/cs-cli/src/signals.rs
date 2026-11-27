//! `candlescope rule-stats`: past results of the 10 frozen exploratory rules for the signal
//! view, from the DISCOVERY and VALIDATION periods only.
//!
//! Data are loaded with the research's sealed access up to the seal boundary (2026-05-27), so
//! the holdout is never read. Markets, events and trades come from the research's own functions
//! (`build_market` with the prefix audit, `trial_events`, `trial_rule`, `run_pair`), so the
//! numbers are the ones the GATE report's validation stage computed; the command refuses to write
//! anything if its validation numbers differ from `reports/research/validation.json`.
//!
//! Output: `reports/rule-stats.json` (read by the overlay and the journal) and
//! `reports/rule-stats.md` (the same numbers as tables).

// Counts shown as rates are far below 2^52.
#![allow(clippy::cast_precision_loss, clippy::too_many_lines)]

use std::collections::BTreeMap;
use std::fmt::Write as _;

use cs_backtest::sim::{ExitReason, Run};
use cs_core::Timeframe;
use cs_core::time::{fmt_minute, fmt_second, now_ms};
use cs_live::signal::rules::{HitKind, RULES, RuleDef};
use cs_stats::beta::credible_interval;
use serde_json::{Value, json};

use crate::research::space::{self, SETUPS};
use crate::research::{
    Access, DISC_END, DISC_START, Opts, SYMBOLS, VAL_END, boot, build_market, check_seal, jn,
    load_symbol, read_json, run_pair, stats, trial_events, trial_rule, window_order, write_file,
    write_json,
};

/// Numbers of one rule, one symbol, one period.
pub struct Period {
    pub n: usize,
    pub hits: usize,
    pub hit_rate: f64,
    pub hit_ci80: (f64, f64),
    pub exp1: f64,
    pub exp1_ci: (f64, f64),
    pub exp2: f64,
    pub exp2_ci: (f64, f64),
    pub cost1: f64,
    pub hold_med: f64,
    pub hold_p75: f64,
}

fn quantile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return f64::NAN;
    }
    #[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]
    let i = ((sorted.len() - 1) as f64 * q).round() as usize;
    sorted[i.min(sorted.len() - 1)]
}

pub fn period(r1: &Run, r2: &Run, a: i64, b: i64, kind: HitKind) -> Period {
    let o = window_order(&r1.trades, a, b);
    let n = o.len();
    let hits = o
        .iter()
        .filter(|&&i| match kind {
            HitKind::Target => r1.trades[i].reason == ExitReason::Target,
            HitKind::InProfit => r1.trades[i].net_r > 0.0,
        })
        .count();
    let x1: Vec<f64> = o.iter().map(|&i| r1.trades[i].net_r).collect();
    let x2: Vec<f64> = o.iter().map(|&i| r2.trades[i].net_r).collect();
    let mean = |x: &[f64]| {
        if x.is_empty() {
            f64::NAN
        } else {
            x.iter().sum::<f64>() / x.len() as f64
        }
    };
    let gross = mean(&o.iter().map(|&i| r1.trades[i].gross_r).collect::<Vec<_>>());
    let mut holds: Vec<f64> = o
        .iter()
        .map(|&i| (r1.trades[i].exit_time - r1.trades[i].entry_time) as f64 / 60_000.0)
        .collect();
    holds.sort_by(f64::total_cmp);
    Period {
        n,
        hits,
        hit_rate: if n == 0 {
            f64::NAN
        } else {
            hits as f64 / n as f64
        },
        hit_ci80: credible_interval(hits as u64, n as u64, 0.8).unwrap_or((f64::NAN, f64::NAN)),
        exp1: mean(&x1),
        exp1_ci: boot(&x1, 0.8),
        exp2: mean(&x2),
        exp2_ci: boot(&x2, 0.8),
        cost1: gross - mean(&x1),
        hold_med: quantile(&holds, 0.5),
        hold_p75: quantile(&holds, 0.75),
    }
}

fn period_json(p: &Period) -> Value {
    json!({
        "n": p.n,
        "hits": p.hits,
        "hit_rate": jn(p.hit_rate),
        "hit_ci80": [jn(p.hit_ci80.0), jn(p.hit_ci80.1)],
        "exp_1x": jn(p.exp1),
        "exp_1x_ci80": [jn(p.exp1_ci.0), jn(p.exp1_ci.1)],
        "exp_2x": jn(p.exp2),
        "exp_2x_ci80": [jn(p.exp2_ci.0), jn(p.exp2_ci.1)],
        "cost_1x": jn(p.cost1),
        "hold_median_min": jn(p.hold_med),
        "hold_p75_min": jn(p.hold_p75),
    })
}

/// The research trial of a frozen rule, checked field by field against the signal view's copy.
fn trial_of(def: &RuleDef, trials: &[space::Trial]) -> Result<space::Trial, String> {
    let t = *trials
        .get(def.trial_id as usize)
        .ok_or_else(|| format!("trial {} not in the enumeration", def.trial_id))?;
    let st = &SETUPS[t.setup];
    let ok = t.label() == def.label
        && trial_rule(&t) == def.trade_rule()
        && st.name == def.setup
        && st.det == def.det
        && st.kinds == def.kinds
        && (t.dir == space::Dir::Reversed) == def.reversed
        && t.tf == def.tf
        && usize::from(def.filter_id) == t.filter
        && t.conf_min == 0;
    if ok {
        Ok(t)
    } else {
        Err(format!(
            "frozen rule {} differs from research trial {} ({})",
            def.label,
            def.trial_id,
            t.label()
        ))
    }
}

fn f3(x: f64) -> String {
    if x.is_finite() {
        format!("{x:+.3}")
    } else {
        "-".into()
    }
}

fn p1(x: f64) -> String {
    if x.is_finite() {
        format!("{:.1} %", x * 100.0)
    } else {
        "-".into()
    }
}

pub fn run(o: &Opts, out: &dyn Fn(&str)) -> Result<(), String> {
    check_seal(o, out)?;
    let trials = space::enumerate();
    let frozen: Vec<(&RuleDef, space::Trial)> = RULES
        .iter()
        .map(|d| trial_of(d, &trials).map(|t| (d, t)))
        .collect::<Result<_, _>>()?;
    let val = read_json(&o.research_dir().join("validation.json"))?;
    out(&format!(
        "rule-stats: 10 frozen rules, BTCUSDT and ETHUSDT, {} .. {} (discovery + validation); the holdout from {} is not loaded",
        fmt_minute(DISC_START),
        fmt_minute(VAL_END),
        fmt_minute(VAL_END)
    ));
    let mut per: BTreeMap<(u8, &str), (Period, Period, Period)> = BTreeMap::new();
    for sym in SYMBOLS {
        // Sealed access: refuses anything past the boundary before reading a file.
        let d = load_symbol(&o.data, sym, VAL_END, VAL_END, Access::Sealed)?;
        for tf in [Timeframe::M15, Timeframe::H1] {
            let (m, _) = build_market(d.bars.get(&tf).ok_or("bars")?, tf)?;
            let mut mk = m.clone();
            for (def, t) in frozen.iter().filter(|(d, _)| d.tf == tf) {
                mk.events = trial_events(&m, t);
                // The signal view selects and flips events itself: same events.
                let own: Vec<_> = m
                    .events
                    .iter()
                    .filter(|e| def.matches(e))
                    .map(|e| def.trade_event(e))
                    .collect();
                if own.len() != mk.events.len()
                    || !own.iter().zip(&mk.events).all(|(a, b)| a.same(b))
                {
                    return Err(format!(
                        "{}: event selection differs from the research",
                        def.label
                    ));
                }
                let rule = def.trade_rule();
                let (r1, r2) = run_pair(&mk, &d, &rule)?;
                // Cross-check with the validation stage's own numbers.
                let entry = val["entries"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .find(|e| e["trial"]["trial_id"].as_u64() == Some(u64::from(def.trial_id)))
                    .ok_or_else(|| format!("{} not in validation.json", def.label))?;
                let checks: Vec<(&str, i64, i64, &Value)> = if sym == "BTCUSDT" {
                    vec![
                        ("validation", DISC_END, VAL_END, &entry["validation"]),
                        ("discovery", DISC_START, DISC_END, &entry["discovery_rerun"]),
                        ("months 1-18", DISC_START, VAL_END, &entry["months_1_18"]),
                    ]
                } else {
                    vec![(
                        "ETH months 1-18",
                        DISC_START,
                        VAL_END,
                        &entry["eth"]["months_1_18"],
                    )]
                };
                for (what, a, b, want) in checks {
                    let s = stats(&r1, &r2, a, b);
                    let same_n = want["trades"].as_u64() == Some(s.n as u64);
                    let same_e = want["exp_net_1x"]
                        .as_f64()
                        .is_some_and(|x| (x - s.exp1).abs() < 1e-9);
                    if !(same_n && same_e) {
                        return Err(format!(
                            "{} {sym} {what}: {} trades / {:.6} R here, validation.json says {} / {}",
                            def.label, s.n, s.exp1, want["trades"], want["exp_net_1x"]
                        ));
                    }
                }
                let k = def.hit_kind();
                per.insert(
                    (def.order, sym),
                    (
                        period(&r1, &r2, DISC_START, VAL_END, k),
                        period(&r1, &r2, DISC_START, DISC_END, k),
                        period(&r1, &r2, DISC_END, VAL_END, k),
                    ),
                );
            }
        }
        out(&format!("  {sym}: done (numbers equal validation.json)"));
    }
    let mut rules_json = Vec::new();
    let mut md = String::new();
    let _ = writeln!(
        md,
        "# Signal view: past results of the 10 frozen rules\n\nGenerated by `candlescope rule-stats` on {} UTC. Periods: discovery {} .. {}, validation {} .. {} (months 1-18). The sealed holdout (from {}) was not loaded. Historical numbers; the rules **all failed the GATE**; nothing here is a forecast.\n\n**The overlay's main line shows the validation row** (out of sample: after the rules were picked). The months 1-18 and discovery rows are **in-sample**: the rules were picked on BTC discovery data, so those rows flatter them. On ETH the rules were never picked, but the same period split is kept.\n\nHit rate: share of trades that exited at the target (rules with a target) or ended above 0 R after costs (trailing rules, no target). Intervals: 80 % Beta-Binomial credible interval (uniform prior) for the hit rate; 80 % circular block bootstrap (block ceil(n^(1/3)), 2000 resamples, seed 1) for expected R. Costs: Phase 3 model at 1x and 2x.\n",
        fmt_second(now_ms()),
        fmt_minute(DISC_START),
        fmt_minute(DISC_END),
        fmt_minute(DISC_END),
        fmt_minute(VAL_END),
        fmt_minute(VAL_END)
    );
    for (def, t) in &frozen {
        let mut syms = serde_json::Map::new();
        for sym in SYMBOLS {
            let Some((all, dsc, vld)) = per.get(&(def.order, sym)) else {
                continue;
            };
            syms.insert(
                sym.to_string(),
                json!({
                    "months_1_18": period_json(all),
                    "discovery": period_json(dsc),
                    "validation": period_json(vld),
                }),
            );
        }
        rules_json.push(json!({
            "order": def.order,
            "trial_id": def.trial_id,
            "label": def.label,
            "tf": def.tf.label(),
            "hit_kind": def.hit_kind().key(),
            "parameters": def.describe(),
            "research_label": t.label(),
            "symbols": Value::Object(syms),
        }));
        let _ = writeln!(
            md,
            "\n## {}. {} ({})\n\n{}\n\n| symbol | period | trades | hit rate (80 % interval) | expected R 1x (80 %) | expected R 2x (80 %) | cost R 1x | median hold |\n|---|---|---|---|---|---|---|---|",
            def.order,
            def.label,
            def.hit_kind().key(),
            def.describe()
        );
        for sym in SYMBOLS {
            let Some((all, dsc, vld)) = per.get(&(def.order, sym)) else {
                continue;
            };
            for (name, p) in [
                ("**validation** (out of sample; the main line)", vld),
                ("months 1-18 (in-sample)", all),
                ("discovery (in-sample: picked here)", dsc),
            ] {
                let _ = writeln!(
                    md,
                    "| {sym} | {name} | {}{} | {} ({} .. {}) | {} ({} .. {}) | {} ({} .. {}) | {:.3} | {:.0} min |",
                    p.n,
                    if p.n < 100 { " (low evidence)" } else { "" },
                    p1(p.hit_rate),
                    p1(p.hit_ci80.0),
                    p1(p.hit_ci80.1),
                    f3(p.exp1),
                    f3(p.exp1_ci.0),
                    f3(p.exp1_ci.1),
                    f3(p.exp2),
                    f3(p.exp2_ci.0),
                    f3(p.exp2_ci.1),
                    p.cost1,
                    p.hold_med
                );
            }
        }
    }
    let v = json!({
        "about": "Past results of the 10 frozen exploratory rules (all failed the GATE). Discovery and validation periods only; the sealed holdout was not loaded. In-sample, historical; never a forecast.",
        "created_utc": fmt_second(now_ms()),
        "periods_text": format!("{} .. {} UTC (discovery + validation)", fmt_minute(DISC_START), fmt_minute(VAL_END)),
        "periods": {
            "discovery": [fmt_minute(DISC_START), fmt_minute(DISC_END)],
            "validation": [fmt_minute(DISC_END), fmt_minute(VAL_END)],
        },
        "holdout": "not loaded (research sealed access, data end = seal boundary)",
        "hit_rate_definition": "target: exit at the target; in_profit: net R after costs > 0 (trailing rules have no target)",
        "intervals": "hit rate: 80 % Beta-Binomial credible interval, uniform prior; expected R: 80 % circular block bootstrap, 2000 resamples, seed 1",
        "costs": "Phase 3 cost model (taker 5 bps per fill, spread 1 bp, slippage 1 bp + 0.02 ATR per market fill, historical funding); 2x doubles fee, spread and slippage",
        "rules": rules_json,
    });
    write_json(&o.reports.join("rule-stats.json"), &v)?;
    write_file(&o.reports.join("rule-stats.md"), &md)?;
    out(&format!(
        "wrote {} and {}",
        o.reports.join("rule-stats.json").display(),
        o.reports.join("rule-stats.md").display()
    ));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The signal view's 10 rules are the research's trials, field by field: same setup,
    /// detector, kinds, direction, time frame, filter and the identical engine rule.
    #[test]
    fn frozen_rules_equal_research_trials() {
        let trials = space::enumerate();
        for def in &RULES {
            let t = trial_of(def, &trials).unwrap();
            assert_eq!(t.label(), def.label);
            assert_eq!(t.family, space::Family::Main);
        }
        // A changed parameter is caught.
        let mut bad = RULES[2];
        bad.buffer_atr = 0.25;
        assert!(trial_of(&bad, &trials).is_err());
        let mut bad = RULES[3];
        bad.reversed = false;
        assert!(trial_of(&bad, &trials).is_err());
    }
}
