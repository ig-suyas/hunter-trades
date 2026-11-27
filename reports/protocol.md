# Phase 4 research protocol (pre-registered)

Written 2026-11-27 04:50 UTC, **before any Phase 4 run and before any tactic outcome was seen.**
The only outcome numbers seen before this file are the Phase 3 random-entry controls (BTCUSDT 15m
net -0.5548 R, ETHUSDT 15m net -0.3555 R; costs 0.55 / 0.35 R per trade at 1-ATR stops).
The search space is in `reports/search-space.md`; both files are hashed into the holdout seal
(`reports/holdout-seal.json`). Any later change to this protocol must be written in section 12
with the reason, and is reported in the gate report.

The aim is to find out whether any tier-1 setup has an edge after costs, not to find something
profitable. "Nothing survived" is an acceptable result.

## 1. Data and periods

Binance USDS-M perpetuals, 1m candles 2024-11-01 00:00 to 2026-11-25 23:59 UTC (755 complete days,
validated in Phase 1), resampled strictly to 5m / 15m / 1h; historical funding; costs from the Phase 3
model (taker 5 bps per fill, spread 1 bp, slippage 1 bp + 0.02 ATR per market fill, funding).

| period | from (UTC, incl.) | to (UTC, excl.) | length | use |
|---|---|---|---|---|
| DISCOVERY | 2024-11-01 00:00 | 2025-11-01 00:00 | 12 months | full search, FDR, plateau |
| VALIDATION | 2025-11-01 00:00 | 2026-05-27 00:00 | 207 days (~6.8 months) | frozen finalists only |
| HOLDOUT | 2026-05-27 00:00 | 2026-11-26 00:00 | 183 days | sealed; opened once with `--final` |

The holdout is the last 183 days of the data (the Phase 3 default). Validation therefore gets the
~0.8 extra month that the 24.8-month history has beyond 24 months.

Primary symbol for the search: **BTCUSDT**. ETHUSDT is used only to check frozen finalists
(months 1-18) and, for finalists that reach the holdout, in the same single holdout run.

**Market building per period.** Each period's run builds the market from all candles *before the end
of that period* (discovery: < 2025-11-01; validation: < 2026-05-27; holdout: all), so indicators get
their history from earlier periods (past data only, no lookahead) and no run ever loads candles from
a later period. A trade belongs to the period that contains its decision time (close of the signal
candle). A trade still open at the end of a period's data closes at the last close (`END_OF_DATA`,
counted). The engine's prefix audit (lookahead guard) runs on every market that is built.

## 2. Holdout seal and lock

1. Before any result is computed, `candlescope research --seal` writes `reports/holdout-seal.json`:
   boundary (2026-05-27 00:00 UTC), SHA-256 of the holdout candles (BTC and ETH, 1m and 5m/15m/1h),
   SHA-256 of `protocol.md` and `search-space.md`, and the creation time. It refuses to overwrite an
   existing seal.
2. Every research stage except `--final` loads only candles, 1m candles and funding before the
   boundary. `candlescope backtest --open-holdout` and a `--holdout-from` later than the seal are
   refused while the seal exists, so `--final` is the only way to read the holdout.
3. `candlescope research --final` first creates `reports/holdout.lock` (exclusive create; it holds the
   time, the seal hash and the hash of the frozen finalist list) and only then loads the holdout. If the
   lock exists it refuses to run. Only the user may delete the lock.
4. `--final` evaluates only the finalists frozen in `reports/research/finalists.json` by the validation
   stage that passed every validation-stage criterion (S3-S6 below). **If no finalist passes
   validation, the holdout is not opened** and stays sealed for a later phase.

## 3. What a trial is

A trial is one fully specified rule evaluated on BTCUSDT DISCOVERY data: setup x direction x time frame
x trade rule (stop and exit) x filter. Every trial in the pre-registered space is run and written to
the ledger (`reports/trial-ledger.csv`, one row per trial, with its net result at 1x and 2x costs),
whether or not it produces trades. The multiple-testing pool (`m`) is all discovery trials of all
families in `search-space.md`. Later evaluations of frozen rules (validation, ETH, walk-forward,
baselines, holdout) are logged in the same ledger with their stage name; they are not new trials and
are not part of `m`.

All trials use: entry at the next open after the signal candle closes, one open trade per rule at a
time, a minimum stop of 0.2 ATR, 1 % risk per trade, 10x leverage cap, starting equity 10,000, and the
Phase 3 cost model at 1x and at 2x (the stress multiplier doubles fee, spread and slippage; funding is
not scaled). Direction "reversed" flips the direction of every event of the setup before simulation
(the structural stop then sits beyond the other side of the pattern).

## 4. Statistics

* **Expectancy** = mean net R per trade (after all costs). Reported at 1x and 2x, with gross R and
  cost per trade in R next to it. Win = net R > 0.
* **Discovery p-value** (one-sided, H0: expectancy at 1x costs <= 0): t = mean / SE with
  SE = max(Newey-West HAC SE with Bartlett kernel and lag floor(4 (n/100)^(2/9)), i.i.d. SE), over the
  trades in exit order; p = 1 - Phi(t). A trial with fewer than 30 trades gets p = 1.
* **FDR control:** Benjamini-Hochberg step-up at q = 0.10 over all `m` discovery trials.
* **Bootstrap interval:** circular block bootstrap of net R in exit order (cs-stats), block length
  ceil(n^(1/3)), 2000 resamples, seed 1; percentile 80 % interval (95 % also shown).
* **Drawdown:** largest peak-to-trough fall of cumulative net R (exit order); worst losing streak;
  largest single loss in R.

## 5. Survival criteria (all must hold)

| id | stage | criterion |
|---|---|---|
| S1 | discovery | passes Benjamini-Hochberg FDR control at q = 0.10 across all `m` discovery trials |
| S2 | discovery | parameter plateau: every pre-registered neighbour (section 6) has positive net expectancy at 1x costs with at least 30 trades (no isolated spike) |
| S3 | validation | at least 100 trades |
| S4 | validation | net expectancy at 2x costs > 0 |
| S5 | validation | lower bound of the 80 % block-bootstrap interval of net expectancy at 1x costs > 0 |
| S6 | ETH, months 1-18 | not contradicted: upper bound of the 80 % interval of net expectancy at 1x costs >= 0 ("confirmed" if the lower bound > 0 as well) |
| S7 | holdout | net expectancy at 2x costs > 0 (BTCUSDT) |

A setup "survives" only if one of its frozen variants passes S1-S7.

## 6. Parameter plateau (S2)

Target ladder L = {0.25, 0.5, 0.75, 1, 1.5, 2, 3} R. Stop buffers B = {0.25, 1.0} ATR. Neighbours of a
trial (same setup, direction, time frame, filter):
* the same exit with the other stop buffer;
* for a fixed R target: the next lower and next higher target in L that exist in the pre-registered
  space for that filter (all of L without a filter; {1, 2, 3} R with a filter);
* for the next-level target and the trailing exit: the 2 R fixed target with the same stop buffer
  (the closest fixed analogue: both aim beyond 1 R);
* for confluence trials (one rule only): the same setup, direction and time frame with the adjacent
  thresholds (count >= k-1 and >= k+1 that exist in {1, 2, 3}).

A neighbour that does not exist in the space is not required. A trial with no neighbour at all cannot
pass S2.

## 7. Finalists (step 2)

* Candidates = discovery trials passing S1 and S2.
* Ranked by discovery t; at most 3 variants per (setup, direction, time frame); at most 20 finalists.
  Their parameters are frozen in `reports/research/finalists.json` before validation runs.
* If fewer than 5 candidates exist, up to 10 **exploratory** rules are added for information: the best
  discovery t among trials with at least 100 discovery trades and positive net 1x expectancy, at most
  one per (setup, direction, time frame). They go through validation, walk-forward, baselines and
  regime checks, but **cannot survive** (they failed S1 or S2) and never enter the holdout.

## 8. Diagnostics for finalists and exploratory rules (steps 3-5)

* **Walk-forward** (months 1-18 only, never the holdout): rolling 3 months train, 1 month test, step 1
  month (test months 2025-02 to 2026-05, the last one partial). In each window the variant with the
  highest net 1x expectancy (at least 30 train trades) among the finalist's plateau neighbourhood
  (itself plus its neighbours from section 6) is chosen on the train window and traded in the test
  month. Reported: concatenated out-of-sample expectancy (1x, 2x), share of positive test months, and
  the frozen variant's monthly results; test months inside DISCOVERY and inside VALIDATION are shown
  separately (the first group is not independent of the finalist selection). A negative walk-forward
  result is a warning, not a gate.
* **Random-entry baseline:** per symbol, time frame and period, the Phase 3 control (random direction
  at random candles, p = 0.5, stop 1 ATR, target 1 R, seed 1); and per finalist a **matched** random
  control with the finalist's own stop rule, exit, filter and overlap (random events have no zone, so
  the structural stop falls back to the signal candle's low/high plus the buffer). Every finalist is
  reported as net R minus the matched random net R.
* **Regimes:** results split by ADX regime relative to the trade (with / against / range), by
  volatility percentile tercile (< 1/3, 1/3-2/3, > 2/3), by UTC session and by UTC hour of the signal
  candle; the share of total net R from the best three hours is reported. Concentration is a warning,
  not a gate.
* **Maker-fee sensitivity:** for survivors only (if any), every fill charged 2 bps instead of 5 bps,
  spread and slippage unchanged. Not part of the search.

## 9. Confluence (pre-registered, limited)

Confluence count at the signal candle, every component relative to the trade direction and known at
the signal candle's close:
* C1: 4h EMA 20/50/200 stack aligned (EMA ready);
* C2: 1h swing-structure trend aligned (on the 1h time frame: 4h structure);
* C3: at a zone: the close is inside an S/R or supply/demand zone, or the nearest zone edge on the
  supporting side (below for a long, above for a short) is within 0.5 ATR of the close;
* C4: a second trigger: a directional event of a different detector in the trade direction known on
  the signal candle or one of the two candles before it.

Confluence family: setups {sweep, fakeout, retest, role_reversal, pullback, sd_test, fib_touch,
choch} x direction x time frame x one rule (stop buffer 0.25 ATR, 2 R target) x threshold
{count >= 1, >= 2, >= 3} = 144 trials, inside the FDR pool. A descriptive table of expectancy by exact
count (0-4) is also shown for these setups; those buckets are not trials and cannot survive.

## 10. Win-rate frontier

For each setup, win rate versus net expectancy across the target ladder L (stop buffer 0.25 and
1.0 ATR, no filter, discovery data, 1x costs). Every discovery variant with win rate >= 80 % is listed
with its expectancy, worst losing streak and largest single loss (and its frozen validation result,
for information). The frontier targets 0.25 / 0.5 / 0.75 / 1.5 R are a family of the FDR pool.

## 11. Verdict rule (fixed now)

* **PROCEED**: at least one setup survives S1-S7.
* **EXTEND**: no survivor, but at least one discovery trial passed S1 and its frozen rule kept a
  positive gross (pre-cost) expectancy in validation, i.e. there is a pre-cost effect that costs or
  instability removed. Next step would be tier 2/3 patterns or forward-only heatmap/liquidation data,
  not Phase 5.
* **DO NOT PROCEED**: no survivor and the EXTEND condition does not hold.

Heatmap/liquidation tactics (H1, H2) cannot be tested on history (no free liquidation archive) and
are outside this verdict; they can only be forward-tested on recorder data.

## 12. Changes to this protocol

Every change, with date, reason and whether results had been seen.

1. **2026-11-27 ~05:00 UTC, before the seal and before any result.** Section 6 gained the plateau
   neighbours of confluence trials (adjacent thresholds) and the sentence that a trial with no
   neighbour cannot pass S2. Reason: confluence trials have a single rule, so the plateau test had no
   neighbours to check. This version is the one hashed into the seal.
2. **2026-11-27, after the discovery results (not a protocol change).** The "top by t" summary list
   in `discovery.json` sorted trials with an undefined t (0-1 trades) first. Fixed and discovery re-run:
   the ledger is byte-identical (sha256 `ee00c1d0...`), the BH result and the frozen exploratory and
   win-rate lists are unchanged; only the freeze time in `finalists.json` changed. Validation ran after
   the re-run, on that file.
3. **2026-11-27, after the discovery and validation results: descriptive addition.**
   `candlescope research --stage winrates` reports the target-hit rate and gross win rate of every
   fixed-target discovery trial next to the protocol's net win rate (section 4 defines a win as net
   R > 0). Reason: the user's 80 % goal is about wins as a chart reader counts them (target hit). Same
   trials, same data, 1x costs; nothing is selected from it, it is not in the ledger and not in `m`.
4. **2026-11-27, presentation only.** The descriptive confluence table (section 9) is shown per time
   frame instead of pooled, because pooling mixed very different cost levels (5m vs 1h).
5. **Holdout (pre-registered rule applied, no change).** No finalist passed validation, so the
   holdout was not opened (section 2.4); `--final` refused and created no lock.
