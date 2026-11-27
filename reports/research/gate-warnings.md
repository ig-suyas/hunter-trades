## 4. Warnings

What this result does and does not show, and the weak points of this research:

1. **It is a negative result for these definitions, not a proof that no pattern works.** Every
   detector kept its Phase 2 parameters (fixed from definitions and event counts, never tuned);
   other definitions of the same pattern could behave differently. But the pre-cost (gross) result
   is zero across all 34,584 variants, including both directions of every setup, so small
   re-definitions are unlikely to turn this around.
2. **Costs are assumptions.** Taker fee 5 bps per fill (VIP 0, no BNB discount; not re-checked
   online), spread 1 bp, slippage 1 bp + 0.02 ATR per market fill, real funding. Cheaper execution
   (maker fees, a VIP tier) would make every number less negative, but it cannot create the missing
   gross edge: the median gross result is +0.004 R. The maker-fee sensitivity check was pre-registered
   for survivors only and was not run, because there are none. Limit entries were not in the search.
3. **Tight structural stops make costs large in R.** With the 0.25 ATR buffer the stop can be only a
   few basis points away (minimum 0.2 ATR), so the fees alone cost several R on those trades (largest
   single losses around -9 R). That is real arithmetic, not an engine error, but it means the 5m and
   tight-stop rules are negative almost by construction. The conclusion does not rest on them: the
   1h rules with 1.0 ATR buffers (2,155 trials) cost a median 0.19 R per trade and their median
   gross result is +0.013 R, far below that cost.
4. **The two periods were different markets.** BTC rose from about 69k to 110k in the discovery year
   (peak about 126k) and fell from about 110k to 76k in validation (low about 60k). A rule picked in
   a rising year can fail in a falling one for that reason as well as from noise. Both directions
   (native and reversed) were tested to soften this, but only two regimes are in sample.
5. **The trials are not independent.** Filters are nested, rules share events, reversed directions
   mirror native ones. Benjamini-Hochberg is valid under the positive dependence this creates and is
   then conservative; but the conclusion does not depend on the correction: even taken one at a time,
   the best discovery rules failed validation.
6. **p-values are approximate.** They use a normal approximation of a HAC t-statistic; trials with
   30-60 trades (most of the top-t list) and the heavy right tail of trailing exits make them rough.
   The bootstrap intervals in validation are the better guide.
7. **The exploratory rules were never candidates.** They are the best discovery t among rules with
   at least 100 trades and positive net expectancy, shown only to make the regression to the mean
   visible. Their walk-forward months inside the discovery year are not independent of their
   selection; their validation months are.
8. **The holdout stayed sealed, so it confirms nothing here.** The out-of-sample evidence in this
   report is the validation period (and ETH for the exploratory rules). That is enough for a negative
   verdict; a positive one would have needed the holdout.
9. **"Win rate" has two meanings in this report.** The protocol counts a win as net R > 0 after costs;
   a chart reader counts a target hit. Both are shown (appendix H). The second was added after the
   results were seen, as a descriptive column only (protocol section 12).
10. **Confluence did not help.** Requiring 1, 2 or 3 confluence factors (4h trend, 1h structure,
    zone, second trigger) left every threshold negative after costs (1 of 120 testable trials
    positive). In the descriptive table, expectancy by exact confluence count shows no consistent
    rise from 0 to 4 on any time frame; the bucket differences are within the noise of a few hundred
    trades.
11. **Engine checks.** No bug was found in `cs-backtest` (it was not edited). Checks: the engine's own
    metrics match the research statistics on sampled trials; the 1x and 2x runs have identical trade
    paths; the discovery-window rerun inside the validation market reproduces the discovery numbers
    (except two trades cut at the period end); the random control loses exactly its costs; the
    prefix lookahead audit passed on every market. The descriptive win-rate stage reproduces the
    ledger's trades, net win rates and expectancies on all 21,928 fixed-target trials.
12. **Heatmap / liquidation tactics were not tested.** There is no free liquidation history; they can
    only be forward-tested on the recorder's data.
13. **"top-3 hours share" above 1** (appendix F) means the other hours lost money in sum; it is shown
    for two exploratory rules and has no bearing on the verdict.
