//! Stage 3 (`--final`): open the sealed holdout once, for the finalists that passed validation.
//!
//! Order: refuse if `reports/holdout.lock` exists; refuse (without creating the lock) if no
//! finalist passed validation; create the lock (exclusive); only then load the holdout data.

use std::collections::BTreeSet;
use std::fmt::Write as _;
use std::io::Write as _;
use std::time::Instant;

use cs_backtest::cost::CostModel;
use cs_core::Timeframe;
use cs_core::time::{fmt_minute, fmt_second, now_ms};
use serde_json::{Value, json};

use super::space;
use super::validate::{frozen_stats, matched_random_events};
use super::{
    Access, DATA_END, DISC_START, Desc, LEDGER_HEADER, Opts, SYMBOLS, VAL_END, build_market,
    candles_sha256, check_seal, file_sha256, ledger_line, load_symbol, lock_path,
    random_control_rule, random_events, read_json, rebuild_ledger, run_pair, run_pair_with,
    seal_path, stats, stats_json, trial_events, trial_rule, write_file, write_json,
};

pub fn run(o: &Opts, out: &dyn Fn(&str)) -> Result<(), String> {
    let t_all = Instant::now();
    let seal = check_seal(o, out)?;
    let lock = lock_path(&o.reports);
    if lock.exists() {
        return Err(format!(
            "{} exists: the holdout has already been opened once. Refusing a second run (only the user may delete the lock).",
            lock.display()
        ));
    }
    let vpath = o.research_dir().join("validation.json");
    let val = read_json(&vpath)?;
    let trials = space::enumerate();
    let passed: Vec<u32> = val["passed"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_u64)
        .map(|x| x as u32)
        .collect();
    if passed.is_empty() {
        return Err(
            "no finalist passed validation: the holdout stays sealed (protocol section 2.4). No lock was created and no holdout data was loaded."
                .into(),
        );
    }
    // ---- the lock, before any holdout data is read
    let fz_sha = file_sha256(&o.research_dir().join("finalists.json"))?;
    let val_sha = file_sha256(&vpath)?;
    let seal_sha = file_sha256(&seal_path(&o.reports))?;
    let list: Vec<String> = passed
        .iter()
        .map(|&id| trials[id as usize].label())
        .collect();
    let body = serde_json::to_string_pretty(&json!({
        "opened_utc": fmt_second(now_ms()),
        "seal_sha256": seal_sha,
        "finalists_sha256": fz_sha,
        "validation_sha256": val_sha,
        "frozen_list_sha256": super::sha256_hex(list.join("\n").as_bytes()),
        "evaluated": list,
        "note": "The holdout was opened once by `candlescope research --final`. Delete this file only if you decide to allow another run (that run would no longer be a clean holdout test).",
    }))
    .map_err(|e| e.to_string())?;
    let mut fh = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock)
        .map_err(|e| format!("cannot create {}: {e}", lock.display()))?;
    fh.write_all(body.as_bytes()).map_err(|e| e.to_string())?;
    drop(fh);
    out(&format!(
        "created {}; opening the holdout {} .. {} for {} finalist(s)",
        lock.display(),
        fmt_minute(VAL_END),
        fmt_minute(DATA_END),
        passed.len()
    ));

    // ---- load everything and check the holdout bytes against the seal
    let btc = load_symbol(&o.data, "BTCUSDT", i64::MAX, VAL_END, Access::Final)?;
    let eth = load_symbol(&o.data, "ETHUSDT", i64::MAX, VAL_END, Access::Final)?;
    let mut warnings = Vec::new();
    for (sym, d) in [(SYMBOLS[0], &btc), (SYMBOLS[1], &eth)] {
        let h1: Vec<_> =
            d.m1.iter()
                .copied()
                .filter(|c| c.open_time >= VAL_END)
                .collect();
        if seal["holdout_data_sha256"][sym]["1m"]["sha256"].as_str()
            != Some(candles_sha256(&h1).as_str())
        {
            warnings.push(format!("{sym} 1m holdout candles differ from the seal"));
        }
        for (tf, b) in &d.bars {
            let h: Vec<_> = b
                .iter()
                .copied()
                .filter(|c| c.open_time >= VAL_END)
                .collect();
            if seal["holdout_data_sha256"][sym][tf.label()]["sha256"].as_str()
                != Some(candles_sha256(&h).as_str())
            {
                warnings.push(format!(
                    "{sym} {} holdout candles differ from the seal",
                    tf.label()
                ));
            }
        }
    }
    for w in &warnings {
        out(&format!("WARNING: {w}"));
    }

    let maker = CostModel {
        taker_fee_bps: 2.0,
        ..CostModel::BINANCE_UM
    };
    let mut ledger = String::from(LEDGER_HEADER);
    let mut results = Vec::new();
    let mut baselines = Vec::new();
    let tfs: BTreeSet<Timeframe> = passed.iter().map(|&id| trials[id as usize].tf).collect();
    for tf in tfs {
        let (bm, _) = build_market(btc.bars.get(&tf).ok_or("bars")?, tf)?;
        let (em, _) = build_market(eth.bars.get(&tf).ok_or("bars")?, tf)?;
        let brand = random_events(&bm)?;
        let erand = random_events(&em)?;
        for (sym, m, rand, d) in [
            ("BTCUSDT", &bm, &brand, &btc),
            ("ETHUSDT", &em, &erand, &eth),
        ] {
            let mut rm = m.clone();
            rm.events.clone_from(rand);
            let (r1, r2) = run_pair(&rm, d, &random_control_rule())?;
            let s = stats(&r1, &r2, VAL_END, DATA_END);
            ledger.push_str(&ledger_line(
                "baseline",
                &Desc::other("random", tf, "random-1r", "control"),
                sym,
                "holdout",
                &s,
                f64::NAN,
                None,
                None,
            ));
            baselines.push(json!({"symbol": sym, "tf": tf.label(), "period": "holdout", "stats": stats_json(&s)}));
        }
        let mut mk = bm.clone();
        let mut mke = em.clone();
        for &id in passed.iter().filter(|&&id| trials[id as usize].tf == tf) {
            let t = &trials[id as usize];
            let rule = trial_rule(t);
            mk.events = trial_events(&bm, t);
            let (r1, r2) = run_pair(&mk, &btc, &rule)?;
            let sh = stats(&r1, &r2, VAL_END, DATA_END);
            let xh = super::window_r(&r1, VAL_END, DATA_END);
            let ci80 = super::boot(&xh, 0.8);
            write_file(
                &o.research_dir()
                    .join("equity")
                    .join(format!("holdout-{id}.csv")),
                &holdout_equity(&r1, &r2),
            )?;
            mke.events = trial_events(&em, t);
            let (e1, e2) = run_pair(&mke, &eth, &rule)?;
            let eh = stats(&e1, &e2, VAL_END, DATA_END);
            mk.events = matched_random_events(&bm, &brand, t);
            let (q1, q2) = run_pair(&mk, &btc, &rule)?;
            let rh = stats(&q1, &q2, VAL_END, DATA_END);
            let s7 = sh.n > 0 && sh.exp2 > 0.0;
            let mut maker_v = Value::Null;
            if s7 {
                mk.events = trial_events(&bm, t);
                let (m1, m2) = run_pair_with(&mk, &btc, &rule, maker, maker.with_mult(2.0))?;
                maker_v = json!({
                    "holdout": stats_json(&stats(&m1, &m2, VAL_END, DATA_END)),
                    "validation": stats_json(&stats(&m1, &m2, super::DISC_END, VAL_END)),
                    "months_1_24": stats_json(&stats(&m1, &m2, DISC_START, DATA_END)),
                });
            }
            for (stage, sym, s) in [
                ("holdout", "BTCUSDT", &sh),
                ("holdout_eth", "ETHUSDT", &eh),
                ("holdout_matched_random", "BTCUSDT", &rh),
            ] {
                ledger.push_str(&ledger_line(
                    stage,
                    &Desc::of(t),
                    sym,
                    "holdout",
                    s,
                    f64::NAN,
                    None,
                    None,
                ));
            }
            out(&format!(
                "  {}: holdout {} trades, net {:.4} R at 1x, {:.4} R at 2x -> S7 {}",
                t.label(),
                sh.n,
                sh.exp1,
                sh.exp2,
                if s7 { "PASS" } else { "FAIL" }
            ));
            results.push(json!({
                "trial_id": id,
                "label": t.label(),
                "validation_entry_found": frozen_stats(&val, id).is_some(),
                "holdout": stats_json(&sh),
                "holdout_ci80": [super::jn(ci80.0), super::jn(ci80.1)],
                "eth_holdout": stats_json(&eh),
                "matched_random_holdout": stats_json(&rh),
                "S7_holdout_net_2x_pos": s7,
                "survives": s7,
                "maker_fee_sensitivity": maker_v,
            }));
        }
    }
    write_file(&o.research_dir().join("ledger-holdout.csv"), &ledger)?;
    write_json(
        &o.research_dir().join("holdout.json"),
        &json!({
            "opened_utc": fmt_second(now_ms()),
            "results": results,
            "baselines": baselines,
            "warnings": warnings,
            "seconds": t_all.elapsed().as_secs_f64(),
        }),
    )?;
    let rows = rebuild_ledger(o)?;
    out(&format!("holdout done; ledger {rows} rows"));
    Ok(())
}

fn holdout_equity(r1: &cs_backtest::sim::Run, r2: &cs_backtest::sim::Run) -> String {
    let mut s = String::from("exit_ms,exit_utc,decision_utc,period,net_r_1x,net_r_2x\n");
    for i in super::window_order(&r1.trades, VAL_END, DATA_END) {
        let t = &r1.trades[i];
        let _ = writeln!(
            s,
            "{},{},{},holdout,{:.6},{:.6}",
            t.exit_time,
            fmt_minute(t.exit_time),
            fmt_minute(t.decision_at),
            t.net_r,
            r2.trades[i].net_r
        );
    }
    s
}
