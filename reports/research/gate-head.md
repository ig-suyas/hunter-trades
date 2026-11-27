# Gate report: Phase 4 research (BTCUSDT and ETHUSDT perpetuals, tier-1 setups)

**0 of 59 setups survived. Verdict: DO NOT PROCEED.** Do not start Phase 5: an overlay built on
these setups would show noise.

In plain language:

- **Nothing passed the first test.** 34,584 pre-registered variants were tested on 12 months of
  BTCUSDT (59 tier-1 setups x both directions x 5m / 15m / 1h x 10 stop-and-target rules x 10
  context filters, plus win-rate, healthy-pullback and confluence families). None passed the
  multiple-testing check (Benjamini-Hochberg, q = 0.10). The best p-value was 0.0014; with this
  many trials it would have needed about 0.000003. No rule could be frozen as a finalist.
- **The setups carry no information even before costs.** Across the 23,749 variants with at least
  30 trades, the median gross result is +0.004 R per trade, 51 % are positive before costs, and the
  share of "significant" gross results (4.6 % beyond +-1.96) is what pure chance gives (5 %).
- **Costs decide the sign.** The median cost per trade is 0.89 R on 5m, 0.46 R on 15m and 0.22 R on
  1h. A random-entry control loses exactly its costs (BTC: -1.10 / -0.57 / -0.23 R on 5m / 15m /
  1h). The setups do no better than random entries.
- **The best-looking rules did not hold up.** The 10 best rules from the discovery year (shown for
  information only, they had failed the test) made +0.24 R per trade there and **-0.20 R per trade**
  on the next 6.8 months of validation. That is what selection noise looks like.
- **High win rates do not help.** 175 variants hit their target in at least 80 % of trades. 174 of
  them lose money after costs; the one exception had 30 trades and lost in validation. None is
  positive at 2x costs. Small targets buy a high hit rate and pay for it with fees on every trade
  and full 1 R (plus fees) losses on the misses.
- **The holdout was not opened.** No finalist existed, so under the pre-registered rule the last
  183 days (2026-05-27 to 2026-11-25) stay sealed: there is no `reports/holdout.lock`, and the
  holdout can still serve one later, different hypothesis.

Why DO NOT PROCEED and not EXTEND: the rule fixed before the run (protocol section 11) says EXTEND
only if at least one trial passed the FDR test and kept a positive pre-cost result in validation.
No trial passed the FDR test. Your options from here (your decision, not part of the verdict):
stop; or test tier 2/3 patterns (little reason for optimism, since tier 1 shows no pre-cost signal
at all); or collect forward-only heatmap / liquidation data with the recorder, which is a different
information source and the one idea this result does not speak to (it needs months of recording).

| key number | value |
|---|---|
| setups tested | 59 (25 structural, 4 higher-time-frame, 30 candlestick) |
| trials in the multiple-testing pool | 34,584 (23,749 with at least 30 trades) |
| passed FDR (q = 0.10) | 0 |
| lowest p-value / needed | 1.4e-3 / 2.9e-6 |
| finalists / survivors | 0 / 0 |
| exploratory rules, net R per trade: discovery -> validation | +0.240 -> -0.200 |
| variants with target hit >= 80 % (>= 30 trades) / positive after 1x costs / after 2x | 175 / 1 / 0 |
| holdout | sealed, not opened, no lock file |

The protocol, search space and seal were written before any run (`reports/protocol.md`,
`reports/search-space.md`, `reports/holdout-seal.json`); every trial is in
`reports/trial-ledger.csv`. Changes to the protocol are listed in its section 12 (one before the
seal, the rest descriptive or presentation only).
