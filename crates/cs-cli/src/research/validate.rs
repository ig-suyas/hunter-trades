//! Stage 2: frozen finalists (and exploratory rules) on VALIDATION data; ETH check (months
//! 1-18); rolling walk-forward; random baselines and matched random controls; regime, session and
//! hour breakdowns; the frozen win-rate >= 80 % variants on validation (information only).

use std::collections::{BTreeSet, HashMap};
use std::fmt::Write as _;
use std::time::Instant;

use cs_backtest::market::Market;
use cs_backtest::sim::Run;
use cs_core::time::{fmt_minute, fmt_second, now_ms};
use cs_core::{Event, Timeframe};
use cs_stats::walkforward::rolling;
use serde_json::{Value, json};

use super::space::{self, Key, TFS, Trial};
use super::{
    Access, DISC_END, DISC_START, Desc, LEDGER_HEADER, MIN_TRADES_P, Opts, SymData, VAL_END, boot,
    breakdown, breakdown_json, build_market, check_seal, conf_count, file_sha256, hour_key, jn,
    ledger_line, load_symbol, month_index, month_label, random_control_rule, random_events,
    read_json, rebuild_ledger, regime_key, run_pair, session_key, stats, stats_json,
    top3_hour_share, trial_events, trial_rule, vol_key, window_r, write_file, write_json,
};

/// Months 1-18: 2024-11 .. 2026-05 (the last one partial).
pub const N_MONTHS: usize = 19;
/// First month index of VALIDATION (2025-11).
pub const VAL_MONTH: usize = 12;

pub struct Frozen {
    pub role: &'static str,
    pub trial: Trial,
}

/// Read the frozen lists and check that each label still matches the enumeration.
pub fn read_frozen(o: &Opts, key: &str, role: &'static str) -> Result<Vec<Frozen>, String> {
    let v = read_json(&o.research_dir().join("finalists.json"))?;
    let trials = space::enumerate();
    let mut out = Vec::new();
    for x in v[key].as_array().into_iter().flatten() {
        let id = x["trial_id"]
            .as_u64()
            .ok_or("finalists.json: bad trial_id")? as usize;
        let t = *trials.get(id).ok_or("finalists.json: unknown trial id")?;
        if x["label"].as_str() != Some(t.label().as_str()) {
            return Err(format!(
                "finalists.json: trial {id} label {:?} does not match the enumeration ({})",
                x["label"],
                t.label()
            ));
        }
        out.push(Frozen { role, trial: t });
    }
    Ok(out)
}

/// Random events for a trial's matched control (confluence-filtered like the trial).
pub fn matched_random_events(full: &Market, rand: &[Event], t: &Trial) -> Vec<Event> {
    if t.conf_min == 0 {
        return rand.to_vec();
    }
    rand.iter()
        .filter(|e| conf_count(full, t.tf, e) >= t.conf_min)
        .copied()
        .collect()
}

/// Walk-forward over months 1-18 for a trial and its plateau neighbourhood.
pub fn walk_forward(
    t: &Trial,
    trials: &[Trial],
    by_key: &HashMap<Key, usize>,
    mk: &mut Market,
    full: &Market,
    d: &SymData,
) -> Result<(Value, String), String> {
    let mut variants = vec![t.id as usize];
    for k in t.neighbour_keys() {
        if let Some(&j) = by_key.get(&k) {
            variants.push(j);
        }
    }
    // per variant, per month: (net1, net2) of trades by decision month
    let mut per: Vec<Vec<Vec<(f64, f64)>>> = Vec::new();
    let mut ledger = String::new();
    for &j in &variants {
        let v = &trials[j];
        mk.events = trial_events(full, v);
        let (r1, r2) = run_pair(mk, d, &trial_rule(v))?;
        let mut months = vec![Vec::new(); N_MONTHS];
        for (a, b) in r1.trades.iter().zip(&r2.trades) {
            if a.decision_at >= DISC_START && a.decision_at < VAL_END {
                let mi = month_index(a.decision_at);
                if mi < N_MONTHS {
                    months[mi].push((a.net_r, b.net_r));
                }
            }
        }
        per.push(months);
        ledger.push_str(&ledger_line(
            "wfo_variant",
            &Desc::of(v),
            &d.symbol,
            "months_1_18",
            &stats(&r1, &r2, DISC_START, VAL_END),
            f64::NAN,
            None,
            None,
        ));
    }
    let mean = |xs: &[f64]| {
        if xs.is_empty() {
            f64::NAN
        } else {
            xs.iter().sum::<f64>() / xs.len() as f64
        }
    };
    let mut steps = Vec::new();
    let (mut oos1, mut oos2) = (Vec::new(), Vec::new());
    let (mut d1, mut v1) = (Vec::new(), Vec::new());
    let (mut pos, mut with_trades) = (0, 0);
    for w in rolling(N_MONTHS, 3, 1, 1) {
        let mut best: Option<(usize, f64, usize)> = None;
        for (vi, months) in per.iter().enumerate() {
            let tr: Vec<f64> = w
                .train
                .clone()
                .flat_map(|m| months[m].iter().map(|x| x.0))
                .collect();
            if tr.len() < MIN_TRADES_P {
                continue;
            }
            let e = mean(&tr);
            if best.is_none_or(|b| e > b.1) {
                best = Some((vi, e, tr.len()));
            }
        }
        let tm = w.test.start;
        let mut step = json!({"test_month": month_label(tm), "chosen": Value::Null});
        if let Some((vi, e, n)) = best {
            let test = &per[vi][tm];
            let x1: Vec<f64> = test.iter().map(|x| x.0).collect();
            let x2: Vec<f64> = test.iter().map(|x| x.1).collect();
            if !x1.is_empty() {
                with_trades += 1;
                if mean(&x1) > 0.0 {
                    pos += 1;
                }
            }
            oos1.extend(&x1);
            oos2.extend(&x2);
            if tm < VAL_MONTH {
                d1.extend(&x1);
            } else {
                v1.extend(&x1);
            }
            step = json!({
                "test_month": month_label(tm),
                "chosen": trials[variants[vi]].rule.name(),
                "chosen_label": trials[variants[vi]].label(),
                "train_trades": n,
                "train_exp_1x": jn(e),
                "test_trades": x1.len(),
                "test_exp_1x": jn(mean(&x1)),
                "test_exp_2x": jn(mean(&x2)),
            });
        }
        steps.push(step);
    }
    let frozen_monthly: Vec<Value> = (0..N_MONTHS)
        .map(|m| {
            let x: Vec<f64> = per[0][m].iter().map(|x| x.0).collect();
            json!({"month": month_label(m), "trades": x.len(), "exp_net_1x": jn(mean(&x))})
        })
        .collect();
    let v = json!({
        "variants": variants.iter().map(|&j| trials[j].label()).collect::<Vec<_>>(),
        "oos_trades": oos1.len(),
        "oos_exp_1x": jn(mean(&oos1)),
        "oos_exp_2x": jn(mean(&oos2)),
        "oos_sum_1x": jn(oos1.iter().sum::<f64>()),
        "oos_discovery_months_trades": d1.len(),
        "oos_discovery_months_exp_1x": jn(mean(&d1)),
        "oos_validation_months_trades": v1.len(),
        "oos_validation_months_exp_1x": jn(mean(&v1)),
        "test_months_with_trades": with_trades,
        "test_months_positive": pos,
        "steps": steps,
        "frozen_monthly": frozen_monthly,
    });
    Ok((v, ledger))
}

fn equity_csv(r1: &Run, r2: &Run, a: i64, b: i64) -> String {
    let mut s = String::from("exit_ms,exit_utc,decision_utc,period,net_r_1x,net_r_2x\n");
    for i in super::window_order(&r1.trades, a, b) {
        let t = &r1.trades[i];
        let period = if t.decision_at < DISC_END {
            "discovery"
        } else if t.decision_at < VAL_END {
            "validation"
        } else {
            "holdout"
        };
        let _ = writeln!(
            s,
            "{},{},{},{period},{:.6},{:.6}",
            t.exit_time,
            fmt_minute(t.exit_time),
            fmt_minute(t.decision_at),
            t.net_r,
            r2.trades[i].net_r
        );
    }
    s
}

fn regimes(m: &Market, r1: &Run, a: i64, b: i64) -> Value {
    let hours = breakdown(m, r1, a, b, hour_key);
    json!({
        "regime": breakdown_json(&breakdown(m, r1, a, b, regime_key)),
        "volatility": breakdown_json(&breakdown(m, r1, a, b, vol_key)),
        "session": breakdown_json(&breakdown(m, r1, a, b, session_key)),
        "hour": breakdown_json(&hours),
        "top3_hour_share": jn(top3_hour_share(&hours)),
    })
}

pub fn run(o: &Opts, out: &dyn Fn(&str)) -> Result<(), String> {
    let t_all = Instant::now();
    check_seal(o, out)?;
    let fz_path = o.research_dir().join("finalists.json");
    let fz_sha = file_sha256(&fz_path)?;
    let mut entries = read_frozen(o, "finalists", "finalist")?;
    entries.extend(read_frozen(o, "exploratory", "exploratory")?);
    let wr80 = read_frozen(o, "wr80", "wr80")?;
    let trials = space::enumerate();
    let by_key: HashMap<Key, usize> = trials.iter().map(|t| (t.key(), t.id as usize)).collect();
    out(&format!(
        "validation: {} finalists + exploratory, {} win-rate >= 80 % variants; BTCUSDT {} .. {} (frozen list sha256 {})",
        entries.len(),
        wr80.len(),
        fmt_minute(DISC_END),
        fmt_minute(VAL_END),
        &fz_sha[..16]
    ));
    let btc = load_symbol(&o.data, "BTCUSDT", VAL_END, VAL_END, Access::Sealed)?;
    let eth = load_symbol(&o.data, "ETHUSDT", VAL_END, VAL_END, Access::Sealed)?;
    let mut ledger = String::from(LEDGER_HEADER);
    let mut out_entries = Vec::new();
    let mut out_wr80 = Vec::new();
    let mut conf_buckets = Vec::new();
    let mut baselines = Vec::new();
    let mut passed = Vec::new();
    let eq_dir = o.research_dir().join("equity");
    let tfs: BTreeSet<Timeframe> = TFS.into_iter().collect();
    for tf in tfs {
        let t0 = Instant::now();
        let (bm, _) = build_market(btc.bars.get(&tf).ok_or("bars")?, tf)?;
        let (em, _) = build_market(eth.bars.get(&tf).ok_or("bars")?, tf)?;
        let brand = random_events(&bm)?;
        let erand = random_events(&em)?;
        // Random-entry controls (Phase 3 rule) per symbol and period.
        for (sym, m, rand, d) in [
            ("BTCUSDT", &bm, &brand, &btc),
            ("ETHUSDT", &em, &erand, &eth),
        ] {
            let mut rm = m.clone();
            rm.events.clone_from(rand);
            let (r1, r2) = run_pair(&rm, d, &random_control_rule())?;
            for (period, a, b) in [
                ("discovery", DISC_START, DISC_END),
                ("validation", DISC_END, VAL_END),
            ] {
                let s = stats(&r1, &r2, a, b);
                ledger.push_str(&ledger_line(
                    "baseline",
                    &Desc::other("random", tf, "random-1r", "control"),
                    sym,
                    period,
                    &s,
                    f64::NAN,
                    None,
                    None,
                ));
                baselines.push(json!({"symbol": sym, "tf": tf.label(), "period": period, "stats": stats_json(&s)}));
            }
        }
        let mut mk = bm.clone();
        let mut mke = em.clone();
        for e in entries.iter().filter(|e| e.trial.tf == tf) {
            let t = &e.trial;
            let rule = trial_rule(t);
            mk.events = trial_events(&bm, t);
            let (r1, r2) = run_pair(&mk, &btc, &rule)?;
            let sd = stats(&r1, &r2, DISC_START, DISC_END);
            let sv = stats(&r1, &r2, DISC_END, VAL_END);
            let s18 = stats(&r1, &r2, DISC_START, VAL_END);
            let xv = window_r(&r1, DISC_END, VAL_END);
            let ci80 = boot(&xv, 0.8);
            let ci95 = boot(&xv, 0.95);
            let reg_v = regimes(&mk, &r1, DISC_END, VAL_END);
            let reg_d = regimes(&mk, &r1, DISC_START, DISC_END);
            write_file(
                &eq_dir.join(format!("{}-{}.csv", e.role, t.id)),
                &equity_csv(&r1, &r2, DISC_START, VAL_END),
            )?;
            for (period, s) in [("validation", &sv), ("discovery_rerun", &sd)] {
                ledger.push_str(&ledger_line(
                    "validation",
                    &Desc::of(t),
                    "BTCUSDT",
                    period,
                    s,
                    f64::NAN,
                    None,
                    None,
                ));
            }
            // Matched random control.
            mk.events = matched_random_events(&bm, &brand, t);
            let (q1, q2) = run_pair(&mk, &btc, &rule)?;
            let rd = stats(&q1, &q2, DISC_START, DISC_END);
            let rv = stats(&q1, &q2, DISC_END, VAL_END);
            ledger.push_str(&ledger_line(
                "matched_random",
                &Desc::of(t),
                "BTCUSDT",
                "validation",
                &rv,
                f64::NAN,
                None,
                None,
            ));
            // ETH, months 1-18.
            mke.events = trial_events(&em, t);
            let (e1, e2) = run_pair(&mke, &eth, &rule)?;
            let ed = stats(&e1, &e2, DISC_START, DISC_END);
            let ev = stats(&e1, &e2, DISC_END, VAL_END);
            let e18 = stats(&e1, &e2, DISC_START, VAL_END);
            let eci = boot(&window_r(&e1, DISC_START, VAL_END), 0.8);
            ledger.push_str(&ledger_line(
                "eth_check",
                &Desc::of(t),
                "ETHUSDT",
                "months_1_18",
                &e18,
                f64::NAN,
                None,
                None,
            ));
            // Walk-forward.
            let (wfo, wl) = walk_forward(t, &trials, &by_key, &mut mk, &bm, &btc)?;
            ledger.push_str(&wl);
            let s1s2 = e.role == "finalist";
            let s3 = sv.n >= 100;
            let s4 = sv.n > 0 && sv.exp2 > 0.0;
            let s5 = ci80.0 > 0.0;
            let eth_status = if e18.n < 2 || !eci.1.is_finite() {
                "untestable"
            } else if eci.0 > 0.0 {
                "confirmed"
            } else if eci.1 >= 0.0 {
                "not_contradicted"
            } else {
                "contradicted"
            };
            let s6 = eth_status == "confirmed" || eth_status == "not_contradicted";
            let ok = s1s2 && s3 && s4 && s5 && s6;
            if ok {
                passed.push(t.id);
            }
            out(&format!(
                "  {} {}: validation {} trades, net {:.4} R (2x {:.4}), 80% CI {:.4} .. {:.4}; ETH {} ({} trades, {:.4} R); WFO OOS {} trades {:.4} R -> {}",
                e.role,
                t.label(),
                sv.n,
                sv.exp1,
                sv.exp2,
                ci80.0,
                ci80.1,
                eth_status,
                e18.n,
                e18.exp1,
                wfo["oos_trades"],
                wfo["oos_exp_1x"].as_f64().unwrap_or(f64::NAN),
                if ok { "PASSES validation" } else { "fails" }
            ));
            out_entries.push(json!({
                "role": e.role,
                "trial": super::discovery::trial_json(t, &sd, f64::NAN, s1s2, s1s2),
                "discovery_rerun": stats_json(&sd),
                "validation": stats_json(&sv),
                "months_1_18": stats_json(&s18),
                "validation_ci80": [jn(ci80.0), jn(ci80.1)],
                "validation_ci95": [jn(ci95.0), jn(ci95.1)],
                "matched_random": {"discovery": stats_json(&rd), "validation": stats_json(&rv)},
                "validation_minus_matched_random_1x": jn(sv.exp1 - rv.exp1),
                "eth": {"discovery": stats_json(&ed), "validation": stats_json(&ev), "months_1_18": stats_json(&e18), "months_1_18_ci80": [jn(eci.0), jn(eci.1)], "status": eth_status},
                "wfo": wfo,
                "regimes": {"validation": reg_v, "discovery": reg_d},
                "criteria": {"S1_fdr": s1s2, "S2_plateau": s1s2, "S3_val_trades_ge_100": s3, "S4_val_net_2x_pos": s4, "S5_val_ci80_lo_pos": s5, "S6_eth_not_contradicted": s6},
                "passed_validation": ok,
            }));
        }
        // Win-rate >= 80 % variants on validation (information only).
        let w_tf: Vec<&Frozen> = wr80.iter().filter(|w| w.trial.tf == tf).collect();
        let got = super::par_map(
            &w_tf,
            o.threads,
            || {
                let mut m = bm.clone();
                m.events.clear();
                m
            },
            |m, w| {
                m.events = trial_events(&bm, &w.trial);
                let (r1, r2) = run_pair(m, &btc, &trial_rule(&w.trial))?;
                Ok((
                    w.trial,
                    stats(&r1, &r2, DISC_START, DISC_END),
                    stats(&r1, &r2, DISC_END, VAL_END),
                ))
            },
        )?;
        for (t, sd, sv) in got {
            ledger.push_str(&ledger_line(
                "wr80_validation",
                &Desc::of(&t),
                "BTCUSDT",
                "validation",
                &sv,
                f64::NAN,
                None,
                None,
            ));
            out_wr80.push(json!({"trial_id": t.id, "label": t.label(), "discovery_rerun": stats_json(&sd), "validation": stats_json(&sv)}));
        }
        // Descriptive confluence buckets (protocol section 9; not trials): the unfiltered b025-t2
        // trades of each confluence setup, split by the exact confluence count of their signal.
        for name in space::CONF_SETUPS {
            let Some(si) = space::SETUPS.iter().position(|s| s.name == name) else {
                continue;
            };
            for dir in space::Dir::ALL {
                let probe = Trial {
                    id: 0,
                    family: space::Family::Main,
                    setup: si,
                    dir,
                    tf,
                    rule: space::RuleSpec {
                        buffer: 0.25,
                        exit: space::Exit::R(2.0),
                    },
                    filter: 0,
                    conf_min: 0,
                };
                let Some(&id) = by_key.get(&probe.key()) else {
                    continue;
                };
                let t = trials[id];
                mk.events = trial_events(&bm, &t);
                let (r1, _) = run_pair(&mk, &btc, &trial_rule(&t))?;
                for (period, a, b) in [
                    ("discovery", DISC_START, DISC_END),
                    ("validation", DISC_END, VAL_END),
                ] {
                    let mut g = [(0usize, 0.0f64, 0usize); 5];
                    for i in super::window_order(&r1.trades, a, b) {
                        let tr = &r1.trades[i];
                        let e = Event::new(tr.det, tr.kind, tr.dir, tr.signal_idx);
                        let c = usize::from(conf_count(&bm, tf, &e)).min(4);
                        g[c].0 += 1;
                        g[c].1 += tr.net_r;
                        g[c].2 += usize::from(tr.net_r > 0.0);
                    }
                    conf_buckets.push(json!({
                        "setup": name, "direction": dir.label(), "tf": tf.label(), "period": period,
                        "buckets": g.iter().enumerate().map(|(k, (n, s, w))| json!({
                            "count": k, "trades": n,
                            "exp_net_1x": jn(if *n > 0 { s / *n as f64 } else { f64::NAN }),
                            "win_rate": jn(if *n > 0 { *w as f64 / *n as f64 } else { f64::NAN }),
                        })).collect::<Vec<_>>(),
                    }));
                }
            }
        }
        out(&format!(
            "  {}: done in {:.1} s",
            tf.label(),
            t0.elapsed().as_secs_f64()
        ));
    }
    let ledger_p = o.research_dir().join("ledger-validation.csv");
    write_file(&ledger_p, &ledger)?;
    let v = json!({
        "created_utc": fmt_second(now_ms()),
        "finalists_sha256": fz_sha,
        "entries": out_entries,
        "passed": passed,
        "baselines": baselines,
        "wr80": out_wr80,
        "confluence_buckets": conf_buckets,
        "seconds": t_all.elapsed().as_secs_f64(),
    });
    write_json(&o.research_dir().join("validation.json"), &v)?;
    let rows = rebuild_ledger(o)?;
    out(&format!(
        "validation done in {:.1} s: {} of {} finalists pass S1-S6 (holdout candidates); ledger {} rows",
        t_all.elapsed().as_secs_f64(),
        passed.len(),
        entries.iter().filter(|e| e.role == "finalist").count(),
        rows
    ));
    Ok(())
}

/// Validation stats of a frozen entry, for the holdout stage.
pub fn frozen_stats(v: &Value, id: u32) -> Option<&Value> {
    v["entries"]
        .as_array()?
        .iter()
        .find(|e| e["trial"]["trial_id"].as_u64() == Some(u64::from(id)))
}
