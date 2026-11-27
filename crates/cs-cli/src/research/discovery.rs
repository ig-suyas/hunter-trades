//! Stage 1: every pre-registered trial on BTCUSDT DISCOVERY data, FDR control, plateau check,
//! finalists frozen to `reports/research/finalists.json`.

use std::collections::{BTreeMap, HashMap, HashSet};
use std::time::Instant;

use cs_backtest::metrics::{BootCfg, summarize};
use cs_core::time::{fmt_second, now_ms};
use cs_stats::hyp::benjamini_hochberg;
use serde_json::{Value, json};

use super::space::{self, Dir, FILTER_IDS, FILTER_LABELS, Key, SETUPS, TFS, Trial};
use super::{
    Access, DISC_END, DISC_START, Desc, FDR_Q, LEDGER_HEADER, MIN_TRADES_P, Opts, Stats, VAL_END,
    build_market, check_seal, conf_count, file_sha256, jn, ledger_line, load_symbol,
    random_control_rule, random_events, rebuild_ledger, run_pair, setup_events, stats, stats_json,
    trial_rule, write_file, write_json,
};

pub const MAX_FINALISTS: usize = 20;
pub const MAX_PER_COMBO: usize = 3;
pub const MIN_CANDIDATES: usize = 5;
pub const MAX_EXPLORATORY: usize = 10;
pub const EXPLORATORY_MIN_TRADES: usize = 100;

pub fn trial_json(t: &Trial, s: &Stats, q: f64, bh: bool, plateau: bool) -> Value {
    json!({
        "trial_id": t.id,
        "label": t.label(),
        "family": t.family.label(),
        "setup": SETUPS[t.setup].name,
        "direction": t.dir.label(),
        "tf": t.tf.label(),
        "rule": t.rule.name(),
        "filter": FILTER_IDS[t.filter],
        "filter_label": FILTER_LABELS[t.filter],
        "conf_min": t.conf_min,
        "q_value": jn(q),
        "bh_pass": bh,
        "plateau": plateau,
        "discovery": stats_json(s),
    })
}

pub fn run(o: &Opts, out: &dyn Fn(&str)) -> Result<(), String> {
    let t_all = Instant::now();
    let seal = check_seal(o, out)?;
    let trials = space::enumerate();
    out(&format!(
        "discovery: {} pre-registered trials, BTCUSDT {} .. {}",
        trials.len(),
        cs_core::time::fmt_minute(DISC_START),
        cs_core::time::fmt_minute(DISC_END)
    ));
    let btc = load_symbol(&o.data, "BTCUSDT", DISC_END, VAL_END, Access::Sealed)?;
    let mut res: Vec<Option<Stats>> = vec![None; trials.len()];
    let mut baselines = Vec::new();
    let mut audits = Vec::new();
    let mut base_lines = String::new();
    for tf in TFS {
        let t0 = Instant::now();
        let bars = btc.bars.get(&tf).ok_or("missing bars")?;
        let (market, cmp) = build_market(bars, tf)?;
        audits.push(json!({"tf": tf.label(), "candles": market.bars.len(), "events": market.events.len(), "audit_events_compared": cmp}));
        let mut units: BTreeMap<(usize, Dir), Vec<usize>> = BTreeMap::new();
        for t in trials.iter().filter(|t| t.tf == tf) {
            units
                .entry((t.setup, t.dir))
                .or_default()
                .push(t.id as usize);
        }
        let units: Vec<((usize, Dir), Vec<usize>)> = units.into_iter().collect();
        let got = super::par_map(
            &units,
            o.threads,
            || {
                let mut m = market.clone();
                m.events.clear();
                m
            },
            |mk, ((si, dir), ids)| {
                let base = setup_events(&market, &SETUPS[*si], *dir);
                let counts: Vec<u8> = if ids.iter().any(|&i| trials[i].conf_min > 0) {
                    base.iter().map(|e| conf_count(&market, tf, e)).collect()
                } else {
                    Vec::new()
                };
                let mut v = Vec::with_capacity(ids.len());
                for &id in ids {
                    let t = &trials[id];
                    mk.events = if t.conf_min == 0 {
                        base.clone()
                    } else {
                        base.iter()
                            .zip(&counts)
                            .filter(|(_, c)| **c >= t.conf_min)
                            .map(|(e, _)| *e)
                            .collect()
                    };
                    let rule = trial_rule(t);
                    let (r1, r2) = run_pair(mk, &btc, &rule)?;
                    let s = stats(&r1, &r2, DISC_START, DISC_END);
                    if id % 509 == 0 {
                        // Cross-check against the engine's own metrics.
                        let sm = summarize(
                            &r1,
                            tf.ms(),
                            BootCfg {
                                reps: 0,
                                ..BootCfg::default()
                            },
                        );
                        let ok = sm.n == s.n
                            && (s.n == 0
                                || ((sm.expectancy_r - s.exp1).abs() < 1e-9
                                    && (sm.win_rate - s.win_rate).abs() < 1e-12
                                    && (sm.max_dd_r - s.max_dd).abs() < 1e-9
                                    && sm.longest_losing_streak == s.streak));
                        if !ok {
                            return Err(format!(
                                "metrics cross-check failed for {}: engine n {} exp {} vs {} {}",
                                t.label(),
                                sm.n,
                                sm.expectancy_r,
                                s.n,
                                s.exp1
                            ));
                        }
                    }
                    v.push((id, s));
                }
                Ok(v)
            },
        )?;
        for (id, s) in got.into_iter().flatten() {
            res[id] = Some(s);
        }
        // Random-entry control for this time frame (Phase 3 rule).
        let mut rm = market.clone();
        rm.events = random_events(&market)?;
        let (r1, r2) = run_pair(&rm, &btc, &random_control_rule())?;
        let s = stats(&r1, &r2, DISC_START, DISC_END);
        base_lines.push_str(&ledger_line(
            "baseline",
            &Desc::other("random", tf, "random-1r", "control"),
            "BTCUSDT",
            "discovery",
            &s,
            f64::NAN,
            None,
            None,
        ));
        baselines.push(json!({"symbol": "BTCUSDT", "tf": tf.label(), "period": "discovery", "stats": stats_json(&s)}));
        out(&format!(
            "  {}: {} candles, {} events, audit PASS, {} trials in {:.1} s; random control {} trades, net {:.4} R (cost {:.4} R)",
            tf.label(),
            market.bars.len(),
            market.events.len(),
            trials.iter().filter(|t| t.tf == tf).count(),
            t0.elapsed().as_secs_f64(),
            s.n,
            s.exp1,
            s.cost1
        ));
    }
    let res: Vec<Stats> = res
        .into_iter()
        .enumerate()
        .map(|(i, s)| s.ok_or(format!("trial {i} has no result")))
        .collect::<Result<_, _>>()?;

    // ---- FDR over all trials
    let p: Vec<f64> = res.iter().map(|s| s.p).collect();
    let bh = benjamini_hochberg(&p, FDR_Q);

    // ---- plateau
    let by_key: HashMap<Key, usize> = trials.iter().map(|t| (t.key(), t.id as usize)).collect();
    let plateau: Vec<bool> = trials
        .iter()
        .map(|t| {
            let nb: Vec<usize> = t
                .neighbour_keys()
                .iter()
                .filter_map(|k| by_key.get(k).copied())
                .collect();
            !nb.is_empty()
                && nb
                    .iter()
                    .all(|&j| res[j].n >= MIN_TRADES_P && res[j].exp1 > 0.0)
        })
        .collect();

    // ---- candidates and finalists
    // Rank by discovery t, best first; an undefined t (fewer than 2 trades or zero spread)
    // ranks last (`total_cmp` alone would put NaN above +inf).
    let tkey = |i: usize| {
        if res[i].t.is_nan() {
            f64::NEG_INFINITY
        } else {
            res[i].t
        }
    };
    let by_t = |ids: &mut Vec<usize>| {
        ids.sort_by(|&a, &b| tkey(b).total_cmp(&tkey(a)).then(a.cmp(&b)));
    };
    let mut cand: Vec<usize> = (0..trials.len())
        .filter(|&i| bh.reject[i] && plateau[i])
        .collect();
    by_t(&mut cand);
    let combo = |i: usize| (trials[i].setup, trials[i].dir, trials[i].tf);
    let mut per: HashMap<_, usize> = HashMap::new();
    let mut finalists = Vec::new();
    for &i in &cand {
        let c = per.entry(combo(i)).or_insert(0);
        if *c < MAX_PER_COMBO && finalists.len() < MAX_FINALISTS {
            *c += 1;
            finalists.push(i);
        }
    }
    let mut exploratory = Vec::new();
    if cand.len() < MIN_CANDIDATES {
        let fin: HashSet<usize> = finalists.iter().copied().collect();
        let mut pool: Vec<usize> = (0..trials.len())
            .filter(|&i| {
                !fin.contains(&i) && res[i].n >= EXPLORATORY_MIN_TRADES && res[i].exp1 > 0.0
            })
            .collect();
        by_t(&mut pool);
        let mut used: HashSet<_> = finalists.iter().map(|&i| combo(i)).collect();
        for i in pool {
            if exploratory.len() >= MAX_EXPLORATORY {
                break;
            }
            if used.insert(combo(i)) {
                exploratory.push(i);
            }
        }
    }
    let mut wr80: Vec<usize> = (0..trials.len())
        .filter(|&i| res[i].n >= MIN_TRADES_P && res[i].win_rate >= 0.8)
        .collect();
    wr80.sort_by(|&a, &b| res[b].exp1.total_cmp(&res[a].exp1).then(a.cmp(&b)));
    let wr80_small = (0..trials.len())
        .filter(|&i| res[i].n > 0 && res[i].n < MIN_TRADES_P && res[i].win_rate >= 0.8)
        .count();

    // ---- ledger
    let mut body = String::from(LEDGER_HEADER);
    for (i, t) in trials.iter().enumerate() {
        body.push_str(&ledger_line(
            "discovery",
            &Desc::of(t),
            "BTCUSDT",
            "discovery",
            &res[i],
            bh.qvalue[i],
            Some(bh.reject[i]),
            Some(plateau[i]),
        ));
    }
    body.push_str(&base_lines);
    let ledger_p = o.research_dir().join("ledger-discovery.csv");
    write_file(&ledger_p, &body)?;
    let ledger_sha = file_sha256(&ledger_p)?;

    // ---- summaries
    let tj = |i: usize| trial_json(&trials[i], &res[i], bh.qvalue[i], bh.reject[i], plateau[i]);
    let mut top: Vec<usize> = (0..trials.len()).collect();
    by_t(&mut top);
    let n_ge30 = res.iter().filter(|s| s.n >= MIN_TRADES_P).count();
    let fam = |f: space::Family| trials.iter().filter(|t| t.family == f).count();
    let summary = json!({
        "created_utc": fmt_second(now_ms()),
        "seal_created_utc": seal["created_utc"],
        "m": trials.len(),
        "families": {
            "main": fam(space::Family::Main),
            "healthy": fam(space::Family::Healthy),
            "frontier": fam(space::Family::Frontier),
            "confluence": fam(space::Family::Confluence),
        },
        "q": FDR_Q,
        "bh_rejections": bh.k,
        "bh_threshold_p": bh.threshold,
        "min_p": p.iter().copied().fold(1.0f64, f64::min),
        "trials_with_trades": res.iter().filter(|s| s.n > 0).count(),
        "trials_ge30": n_ge30,
        "trials_ge30_net1x_pos": res.iter().filter(|s| s.n >= MIN_TRADES_P && s.exp1 > 0.0).count(),
        "trials_ge30_net2x_pos": res.iter().filter(|s| s.n >= MIN_TRADES_P && s.exp2 > 0.0).count(),
        "trials_ge30_gross_pos": res.iter().filter(|s| s.n >= MIN_TRADES_P && s.gross > 0.0).count(),
        "trials_p_lt_0_05": res.iter().filter(|s| s.p < 0.05).count(),
        "trials_p_lt_0_01": res.iter().filter(|s| s.p < 0.01).count(),
        "plateau_pass": plateau.iter().filter(|x| **x).count(),
        "candidates": cand.len(),
        "candidate_list": cand.iter().take(200).map(|&i| tj(i)).collect::<Vec<_>>(),
        "top_by_t": top.iter().take(50).map(|&i| tj(i)).collect::<Vec<_>>(),
        "finalists": finalists.iter().map(|&i| tj(i)).collect::<Vec<_>>(),
        "exploratory": exploratory.iter().map(|&i| tj(i)).collect::<Vec<_>>(),
        "wr80_count": wr80.len(),
        "wr80_small_sample_count": wr80_small,
        "baselines": baselines,
        "markets": audits,
        "ledger_sha256": ledger_sha,
        "seconds": t_all.elapsed().as_secs_f64(),
    });
    write_json(&o.research_dir().join("discovery.json"), &summary)?;
    let frozen = json!({
        "frozen_utc": fmt_second(now_ms()),
        "discovery_ledger_sha256": ledger_sha,
        "finalists": finalists.iter().map(|&i| json!({"trial_id": i, "label": trials[i].label()})).collect::<Vec<_>>(),
        "exploratory": exploratory.iter().map(|&i| json!({"trial_id": i, "label": trials[i].label()})).collect::<Vec<_>>(),
        "wr80": wr80.iter().map(|&i| json!({"trial_id": i, "label": trials[i].label()})).collect::<Vec<_>>(),
    });
    write_json(&o.research_dir().join("finalists.json"), &frozen)?;
    let rows = rebuild_ledger(o)?;
    out(&format!(
        "discovery done in {:.1} s: m = {}, trials with >= 30 trades {}, BH rejections {} (threshold p {:.3e}), plateau {}, candidates {}, finalists {}, exploratory {}, win rate >= 80 % variants {}; ledger {} rows",
        t_all.elapsed().as_secs_f64(),
        trials.len(),
        n_ge30,
        bh.k,
        bh.threshold,
        plateau.iter().filter(|x| **x).count(),
        cand.len(),
        finalists.len(),
        exploratory.len(),
        wr80.len(),
        rows
    ));
    Ok(())
}

/// Descriptive add-on, written after the results were seen (protocol section 12): target-hit
/// rate and gross win rate of every fixed-target discovery trial next to its net win rate. The
/// same trials on the same data at 1x costs; nothing is selected from it and nothing enters the
/// ledger or the frozen lists.
pub fn winrates(o: &Opts, out: &dyn Fn(&str)) -> Result<(), String> {
    use cs_backtest::sim::{ExitReason, Inputs, Sizing, simulate};
    check_seal(o, out)?;
    let trials = space::enumerate();
    let btc = load_symbol(&o.data, "BTCUSDT", DISC_END, VAL_END, Access::Sealed)?;
    let mut body = String::from(
        "trial_id,label,trades,target_hits,target_hit_rate,gross_win_rate,net_win_rate,exp_net_1x,longest_losing_streak,worst_trade_r\n",
    );
    for tf in TFS {
        let (market, _) = build_market(btc.bars.get(&tf).ok_or("missing bars")?, tf)?;
        let mut units: BTreeMap<(usize, Dir), Vec<usize>> = BTreeMap::new();
        for t in trials
            .iter()
            .filter(|t| t.tf == tf && matches!(t.rule.exit, space::Exit::R(_)))
        {
            units
                .entry((t.setup, t.dir))
                .or_default()
                .push(t.id as usize);
        }
        let units: Vec<((usize, Dir), Vec<usize>)> = units.into_iter().collect();
        let got = super::par_map(
            &units,
            o.threads,
            || {
                let mut m = market.clone();
                m.events.clear();
                m
            },
            |mk, ((si, dir), ids)| {
                let base = setup_events(&market, &SETUPS[*si], *dir);
                let counts: Vec<u8> = if ids.iter().any(|&i| trials[i].conf_min > 0) {
                    base.iter().map(|e| conf_count(&market, tf, e)).collect()
                } else {
                    Vec::new()
                };
                let mut v = Vec::with_capacity(ids.len());
                for &id in ids {
                    let t = &trials[id];
                    mk.events = if t.conf_min == 0 {
                        base.clone()
                    } else {
                        base.iter()
                            .zip(&counts)
                            .filter(|(_, c)| **c >= t.conf_min)
                            .map(|(e, _)| *e)
                            .collect()
                    };
                    let inp = Inputs {
                        market: mk,
                        m1: &btc.m1,
                        funding: &btc.funding,
                        costs: super::costs(1.0),
                        sizing: Sizing::default(),
                    };
                    let r = simulate(&inp, &trial_rule(t)).map_err(|e| e.to_string())?;
                    let s = stats(&r, &r, DISC_START, DISC_END);
                    let n = r.trades.len();
                    let hits = r
                        .trades
                        .iter()
                        .filter(|x| x.reason == ExitReason::Target)
                        .count();
                    let gw = r.trades.iter().filter(|x| x.gross_r > 0.0).count();
                    let rate = |k: usize| {
                        if n == 0 {
                            String::new()
                        } else {
                            format!("{:.4}", k as f64 / n as f64)
                        }
                    };
                    v.push(format!(
                        "{id},{},{n},{hits},{},{},{},{},{},{}\n",
                        t.label(),
                        rate(hits),
                        rate(gw),
                        super::f(s.win_rate, 4),
                        super::f(s.exp1, 4),
                        s.streak,
                        super::f(s.worst, 3)
                    ));
                }
                Ok(v)
            },
        )?;
        for line in got.into_iter().flatten() {
            body.push_str(&line);
        }
    }
    let p = o.research_dir().join("winrates-discovery.csv");
    write_file(&p, &body)?;
    out(&format!("wrote {}", p.display()));
    Ok(())
}
