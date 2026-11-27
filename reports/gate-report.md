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

## 1. Survivors

**None.** No setup passed the discovery FDR test (S1), so no rule could be frozen as a finalist and the table is empty:

| setup | rule | expectancy R, validation (80 % interval) | win rate | trades (val) | max drawdown R | holdout result |
|---|---|---|---|---|---|---|
| (none) | - | - | - | - | - | holdout not opened (still sealed) |


## 2. Rejected setups and the reason

All 59 setups are rejected. The reason is the same for every one: **S1 failed**, no discovery trial passed Benjamini-Hochberg FDR control at q = 0.10 over m = 34,584 trials (needed p <= 2.9e-06 for the best trial; the lowest p of all trials was 1.43e-03, q = 1.00 for every trial). The columns show how far each setup was from an edge: its trials, the share of its trials (with at least 30 trades) that were positive after 1x costs, its median gross and cost per trade, and its best discovery trial by t (which, for the 10 exploratory rules, was also run on validation).

| setup | trials | with >= 30 trades | positive at 1x | median gross R | median cost R | best discovery trial (by t) | reason |
|---|---|---|---|---|---|---|---|
| swing | 588 | 588 | 1.9 % | +0.009 | 0.634 | swing native 1h b100-t2 F04: 96 trades, +0.183 R, p 1.15e-01 | S1: no trial passed FDR; exploratory rule native 1h b100-lvl F07 made +0.014 R on 85 validation trades |
| structure_label | 588 | 588 | 0.5 % | +0.004 | 0.747 | structure_label reversed 1h b100-trail F07: 175 trades, +0.142 R, p 2.20e-01 | S1: no trial passed FDR |
| structure_trend | 588 | 588 | 0.2 % | +0.004 | 0.772 | structure_trend reversed 1h b100-trail F02: 71 trades, +0.029 R, p 4.57e-01 | S1: no trial passed FDR |
| bos | 588 | 588 | 4.3 % | +0.022 | 0.527 | bos reversed 1h b100-trail F07: 42 trades, +0.746 R, p 5.73e-02 | S1: no trial passed FDR |
| choch | 606 | 601 | 0.8 % | -0.009 | 0.555 | choch reversed 1h b100-trail F09: 121 trades, +0.172 R, p 2.17e-01 | S1: no trial passed FDR; exploratory rule reversed 1h b100-trail F09 made +0.046 R on 62 validation trades |
| sr_break | 588 | 441 | 1.6 % | +0.013 | 0.449 | sr_break native 1h b100-t3 F04: 89 trades, +0.143 R, p 2.33e-01 | S1: no trial passed FDR |
| sr_formed | 588 | 567 | 2.1 % | -0.008 | 0.648 | sr_formed native 1h b100-t2 F07: 70 trades, +0.235 R, p 9.33e-02 | S1: no trial passed FDR |
| role_reversal | 606 | 546 | 1.1 % | +0.006 | 0.503 | role_reversal native 1h b100-t3 F08: 85 trades, +0.230 R, p 1.32e-01 | S1: no trial passed FDR |
| sd_test | 606 | 363 | 1.1 % | +0.020 | 1.218 | sd_test native 15m b100-lvl F04: 35 trades, +0.197 R, p 2.16e-01 | S1: no trial passed FDR |
| sd_formed | 588 | 179 | 11.7 % | +0.044 | 0.322 | sd_formed native 1h b025-t0.25 F00: 30 trades, +0.081 R, p 8.78e-02 | S1: no trial passed FDR |
| sd_broken | 588 | 252 | 0.0 % | -0.027 | 1.064 | sd_broken native 15m b100-t3 F07: 56 trades, -0.039 R, p 5.62e-01 | S1: no trial passed FDR |
| ema_stack | 588 | 433 | 8.1 % | +0.015 | 0.308 | ema_stack native 1h b025-lvl F07: 43 trades, +0.270 R, p 9.65e-02 | S1: no trial passed FDR |
| ema_cross | 588 | 507 | 8.9 % | +0.023 | 0.404 | ema_cross native 15m b100-t1 F05: 42 trades, +0.383 R, p 1.50e-03 | S1: no trial passed FDR; exploratory rule native 15m b100-t3 F07 made -0.417 R on 84 validation trades |
| tl_touch | 588 | 588 | 2.0 % | +0.019 | 0.706 | tl_touch reversed 1h b025-trail F07: 66 trades, +0.447 R, p 2.12e-01 | S1: no trial passed FDR |
| tl_break | 588 | 588 | 1.4 % | -0.005 | 0.596 | tl_break native 1h b100-t2 F06: 85 trades, +0.106 R, p 2.56e-01 | S1: no trial passed FDR |
| tl_formed | 588 | 588 | 0.7 % | +0.001 | 0.764 | tl_formed reversed 1h b100-trail F07: 122 trades, +0.201 R, p 1.73e-01 | S1: no trial passed FDR; exploratory rule reversed 1h b100-trail F07 made -0.138 R on 79 validation trades |
| fib_touch | 606 | 606 | 3.1 % | +0.006 | 0.596 | fib_touch native 1h b025-trail F09: 150 trades, +0.536 R, p 6.96e-02 | S1: no trial passed FDR; exploratory rule native 1h b025-trail F09 made -0.214 R on 83 validation trades |
| fib_ext | 588 | 247 | 17.8 % | +0.031 | 0.139 | fib_ext native 15m b100-lvl F07: 53 trades, +0.299 R, p 4.28e-02 | S1: no trial passed FDR; exploratory rule native 15m b100-trail F07 made -0.257 R on 108 validation trades |
| fib_leg | 588 | 588 | 2.9 % | +0.011 | 0.523 | fib_leg native 1h b025-trail F09: 143 trades, +0.453 R, p 1.12e-01 | S1: no trial passed FDR; exploratory rule native 1h b025-trail F09 made -0.321 R on 79 validation trades |
| pullback | 666 | 646 | 3.6 % | +0.002 | 0.366 | pullback reversed 1h b100-t1 F08: 180 trades, +0.096 R, p 1.26e-01 | S1: no trial passed FDR; exploratory rule reversed 1h b100-t1 F08 made -0.049 R on 92 validation trades |
| breakout | 588 | 441 | 2.0 % | +0.011 | 0.430 | breakout native 1h b100-t2 F07: 99 trades, +0.169 R, p 1.37e-01 | S1: no trial passed FDR |
| retest | 606 | 413 | 2.7 % | -0.027 | 0.366 | retest native 15m b100-t3 F04: 112 trades, +0.134 R, p 2.28e-01 | S1: no trial passed FDR |
| fakeout | 606 | 569 | 1.6 % | +0.002 | 0.518 | fakeout native 1h b100-t2 F02: 50 trades, +0.202 R, p 1.68e-01 | S1: no trial passed FDR |
| eq_levels | 588 | 558 | 1.4 % | +0.001 | 0.802 | eq_levels native 1h b100-t3 F09: 69 trades, +0.191 R, p 2.05e-01 | S1: no trial passed FDR |
| sweep | 606 | 446 | 2.5 % | -0.007 | 0.599 | sweep native 1h b025-lvl F04: 68 trades, +0.189 R, p 2.27e-01 | S1: no trial passed FDR |
| h1_stack | 432 | 352 | 0.3 % | +0.033 | 0.447 | h1_stack reversed 15m b025-trail F07: 35 trades, +0.038 R, p 4.78e-01 | S1: no trial passed FDR |
| h1_trend | 432 | 388 | 0.8 % | +0.004 | 0.548 | h1_trend reversed 15m b025-lvl F07: 59 trades, +0.160 R, p 3.29e-01 | S1: no trial passed FDR |
| h4_stack | 588 | 337 | 3.0 % | -0.059 | 0.255 | h4_stack reversed 1h b100-t2 F00: 71 trades, +0.143 R, p 2.11e-01 | S1: no trial passed FDR |
| h4_trend | 588 | 371 | 8.6 % | -0.047 | 0.308 | h4_trend reversed 15m b025-t3 F07: 31 trades, +0.456 R, p 1.04e-01 | S1: no trial passed FDR |
| cdl_hammer | 588 | 428 | 0.2 % | -0.030 | 0.776 | cdl_hammer reversed 1h b025-trail F00: 35 trades, +0.136 R, p 4.38e-01 | S1: no trial passed FDR |
| cdl_inverted_hammer | 588 | 398 | 0.0 % | -0.024 | 0.929 | cdl_inverted_hammer reversed 15m b100-trail F06: 37 trades, -0.108 R, p 6.07e-01 | S1: no trial passed FDR |
| cdl_bullish_engulfing | 588 | 570 | 5.3 % | +0.016 | 0.522 | cdl_bullish_engulfing native 1h b100-t1 F04: 57 trades, +0.258 R, p 1.80e-02 | S1: no trial passed FDR |
| cdl_piercing_line | 588 | 453 | 0.7 % | -0.015 | 0.580 | cdl_piercing_line native 1h b100-t1.5 F00: 54 trades, +0.107 R, p 2.68e-01 | S1: no trial passed FDR |
| cdl_morning_star | 588 | 215 | 1.9 % | +0.072 | 0.737 | cdl_morning_star native 5m b100-t3 F04: 42 trades, +0.148 R, p 3.12e-01 | S1: no trial passed FDR |
| cdl_morning_doji_star | 588 | 192 | 0.5 % | +0.028 | 0.845 | cdl_morning_doji_star native 15m b100-lvl F00: 33 trades, +0.122 R, p 3.34e-01 | S1: no trial passed FDR |
| cdl_three_white_soldiers | 588 | 75 | 1.3 % | +0.131 | 1.388 | cdl_three_white_soldiers native 5m b100-t3 F08: 40 trades, +0.015 R, p 4.80e-01 | S1: no trial passed FDR |
| cdl_tweezer_bottom | 588 | 512 | 1.2 % | +0.006 | 0.734 | cdl_tweezer_bottom native 1h b100-t1 F09: 50 trades, +0.113 R, p 2.06e-01 | S1: no trial passed FDR |
| cdl_bullish_harami | 588 | 349 | 1.7 % | -0.005 | 0.719 | cdl_bullish_harami reversed 5m b025-trail F05: 37 trades, +0.285 R, p 3.27e-01 | S1: no trial passed FDR |
| cdl_bullish_harami_cross | 588 | 266 | 0.4 % | -0.011 | 0.801 | cdl_bullish_harami_cross reversed 15m b100-t3 F09: 44 trades, +0.008 R, p 4.90e-01 | S1: no trial passed FDR |
| cdl_three_inside_up | 588 | 106 | 0.0 % | +0.021 | 0.803 | cdl_three_inside_up native 5m b100-lvl F04: 36 trades, -0.170 R, p 7.60e-01 | S1: no trial passed FDR |
| cdl_three_outside_up | 588 | 520 | 6.5 % | +0.025 | 0.447 | cdl_three_outside_up native 1h b100-t3 F00: 59 trades, +0.388 R, p 6.22e-02 | S1: no trial passed FDR; exploratory rule native 1h b025-trail F00 made -0.098 R on 56 validation trades |
| cdl_rising_three_methods | 588 | 0 | - | - | - | no trial with 30 trades | S1: no trial passed FDR (too rare: fewer than 30 trades in every trial) |
| cdl_bullish_marubozu | 588 | 458 | 0.4 % | +0.024 | 0.740 | cdl_bullish_marubozu native 15m b100-lvl F06: 31 trades, +0.222 R, p 2.62e-01 | S1: no trial passed FDR |
| cdl_dragonfly_doji | 588 | 257 | 0.0 % | -0.018 | 1.140 | cdl_dragonfly_doji native 5m b100-t3 F06: 37 trades, -0.454 R, p 9.05e-01 | S1: no trial passed FDR |
| cdl_hanging_man | 588 | 426 | 1.2 % | +0.000 | 0.764 | cdl_hanging_man reversed 1h b100-lvl F00: 31 trades, +0.280 R, p 1.39e-01 | S1: no trial passed FDR |
| cdl_shooting_star | 588 | 458 | 0.2 % | -0.021 | 0.764 | cdl_shooting_star reversed 1h b100-t3 F09: 30 trades, +0.204 R, p 2.83e-01 | S1: no trial passed FDR |
| cdl_bearish_engulfing | 588 | 551 | 0.9 % | -0.003 | 0.524 | cdl_bearish_engulfing reversed 1h b100-t3 F04: 51 trades, +0.166 R, p 2.66e-01 | S1: no trial passed FDR |
| cdl_dark_cloud_cover | 588 | 451 | 1.1 % | -0.041 | 0.568 | cdl_dark_cloud_cover reversed 15m b100-lvl F06: 30 trades, +0.442 R, p 1.64e-01 | S1: no trial passed FDR |
| cdl_evening_star | 588 | 168 | 3.0 % | -0.034 | 0.733 | cdl_evening_star native 15m b100-t1 F00: 38 trades, +0.090 R, p 2.83e-01 | S1: no trial passed FDR |
| cdl_evening_doji_star | 588 | 210 | 5.2 % | -0.035 | 0.754 | cdl_evening_doji_star native 15m b100-t3 F09: 31 trades, +0.614 R, p 4.56e-02 | S1: no trial passed FDR |
| cdl_three_black_crows | 588 | 75 | 0.0 % | +0.000 | 1.587 | cdl_three_black_crows native 5m b025-t3 F09: 42 trades, -0.556 R, p 9.56e-01 | S1: no trial passed FDR |
| cdl_tweezer_top | 588 | 529 | 6.2 % | +0.020 | 0.678 | cdl_tweezer_top reversed 1h b100-t2 F04: 34 trades, +0.742 R, p 1.43e-03 | S1: no trial passed FDR; exploratory rule reversed 1h b025-trail F00 made -0.568 R on 43 validation trades |
| cdl_bearish_harami | 588 | 407 | 1.5 % | -0.010 | 0.631 | cdl_bearish_harami native 5m b100-t3 F07: 31 trades, +0.166 R, p 3.48e-01 | S1: no trial passed FDR |
| cdl_bearish_harami_cross | 588 | 287 | 1.4 % | +0.006 | 0.795 | cdl_bearish_harami_cross reversed 15m b100-t3 F04: 30 trades, +0.278 R, p 2.26e-01 | S1: no trial passed FDR |
| cdl_three_inside_down | 588 | 145 | 0.0 % | +0.043 | 1.019 | cdl_three_inside_down reversed 15m b025-trail F00: 33 trades, -0.184 R, p 5.55e-01 | S1: no trial passed FDR |
| cdl_three_outside_down | 588 | 536 | 1.5 % | -0.014 | 0.472 | cdl_three_outside_down reversed 1h b100-trail F07: 44 trades, +0.274 R, p 2.45e-01 | S1: no trial passed FDR |
| cdl_falling_three_methods | 588 | 0 | - | - | - | no trial with 30 trades | S1: no trial passed FDR (too rare: fewer than 30 trades in every trial) |
| cdl_bearish_marubozu | 588 | 462 | 0.9 % | +0.027 | 0.788 | cdl_bearish_marubozu reversed 15m b100-t2 F06: 46 trades, +0.165 R, p 2.78e-01 | S1: no trial passed FDR |
| cdl_gravestone_doji | 588 | 278 | 0.0 % | -0.022 | 1.096 | cdl_gravestone_doji reversed 5m b100-trail F06: 58 trades, -0.155 R, p 6.49e-01 | S1: no trial passed FDR |


## 3. Equity curves and drawdowns (in R)

There are no survivors or finalists, so these charts show the 10 **exploratory** rules (the best discovery t among rules with at least 100 trades and positive net expectancy; they failed FDR and could never survive). Months 1-12 are the discovery data they were picked on; the shaded months 13-18 are validation. The holdout was not opened, so it is not drawn.

![Cumulative net R of the exploratory rules](charts/equity-overview.svg)

Each rule with its 1x and 2x curve and its drawdown chart:

- [ema_cross native 15m b100-t3 F07](charts/equity-exploratory-15499.svg): discovery +23.6 R over 106 trades, validation -35.0 R over 84 trades; max drawdown months 1-18 50.7 R
- [fib_ext native 15m b100-trail F07](charts/equity-exploratory-16605.svg): discovery +11.5 R over 185 trades, validation -27.7 R over 108 trades; max drawdown months 1-18 30.3 R
- [fib_touch native 1h b025-trail F09](charts/equity-exploratory-28172.svg): discovery +80.4 R over 150 trades, validation -17.8 R over 83 trades; max drawdown months 1-18 33.0 R
- [fib_leg native 1h b025-trail F09](charts/equity-exploratory-28490.svg): discovery +64.8 R over 143 trades, validation -25.4 R over 79 trades; max drawdown months 1-18 29.0 R
- [pullback reversed 1h b100-t1 F08](charts/equity-exploratory-28743.svg): discovery +17.2 R over 180 trades, validation -4.5 R over 92 trades; max drawdown months 1-18 18.9 R
- [cdl_three_outside_up native 1h b025-trail F00](charts/equity-exploratory-31648.svg): discovery +17.1 R over 100 trades, validation -5.5 R over 56 trades; max drawdown months 1-18 12.6 R
- [tl_formed reversed 1h b100-trail F07](charts/equity-exploratory-28127.svg): discovery +24.6 R over 122 trades, validation -10.9 R over 79 trades; max drawdown months 1-18 18.7 R
- [swing native 1h b100-lvl F07](charts/equity-exploratory-25684.svg): discovery +13.3 R over 132 trades, validation +1.2 R over 85 trades; max drawdown months 1-18 14.4 R
- [cdl_tweezer_top reversed 1h b025-trail F00](charts/equity-exploratory-33442.svg): discovery +39.4 R over 100 trades, validation -24.4 R over 43 trades; max drawdown months 1-18 43.1 R
- [choch reversed 1h b100-trail F09](charts/equity-exploratory-26398.svg): discovery +20.1 R over 121 trades, validation +2.8 R over 62 trades; max drawdown months 1-18 15.9 R

Drawdown table (net R at 1x costs):

| rule | max DD discovery | max DD validation | max DD months 1-18 | longest losing streak | largest single loss |
|---|---|---|---|---|---|
| ema_cross native 15m b100-t3 F07 | 24.3 | 50.7 | 50.7 | 16 | -1.44 |
| fib_ext native 15m b100-trail F07 | 10.2 | 28.7 | 30.3 | 8 | -1.39 |
| fib_touch native 1h b025-trail F09 | 18.4 | 33.0 | 33.0 | 9 | -2.62 |
| fib_leg native 1h b025-trail F09 | 27.4 | 28.6 | 29.0 | 10 | -2.48 |
| pullback reversed 1h b100-t1 F08 | 10.1 | 14.5 | 18.9 | 8 | -1.42 |
| cdl_three_outside_up native 1h b025-trail F00 | 10.8 | 12.0 | 12.6 | 9 | -1.65 |
| tl_formed reversed 1h b100-trail F07 | 18.7 | 16.6 | 18.7 | 9 | -1.44 |
| swing native 1h b100-lvl F07 | 12.2 | 14.4 | 14.4 | 7 | -1.35 |
| cdl_tweezer_top reversed 1h b025-trail F00 | 21.5 | 32.3 | 43.1 | 23 | -2.42 |
| choch reversed 1h b100-trail F09 | 15.9 | 8.9 | 15.9 | 8 | -1.53 |


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


## Appendix A. What was run

| item | value |
|---|---|
| data | Binance USDS-M BTCUSDT (search) and ETHUSDT (check), 1m candles 2024-11-01 .. 2026-11-25, strict 5m/15m/1h, historical funding |
| discovery | 2024-11-01 .. 2025-11-01 (12 months) |
| validation | 2025-11-01 .. 2026-05-27 (207 days) |
| holdout | 2026-05-27 00:00 .. 2026-11-26 00:00 (183 days), sealed 2026-11-27 05:05:47 UTC before any result; **not opened**: no finalist passed validation; no lock file; still sealed for a later phase |
| trials (FDR pool m) | 34,584 = main 31,580 + healthy 60 + frontier 2,800 + confluence 144 |
| trials with any trade / with >= 30 trades | 32,258 / 23,749 |
| costs | taker 5 bps per fill, spread 1 bp, slippage 1 bp + 0.02 ATR per market fill, historical funding; 2x doubles fee, spread and slippage |
| engine | cs-backtest unchanged (Phase 3); prefix lookahead audit passed on every market built (BTC discovery and months 1-18, ETH months 1-18; 5m/15m/1h; detector and random-entry sources) |
| runtime | discovery 17.4 s, validation 11.3 s (release build, 6 threads) |
| ledger | `reports/trial-ledger.csv`, 34,676 rows (34,584 discovery trials + baselines + validation-stage evaluations); discovery part sha256 `ee00c1d0858d51a7...` |


## Appendix B. Discovery results

Benjamini-Hochberg at q = 0.10: **0 rejections**. Lowest p-value 1.43e-03; the best trial would have needed p <= 2.9e-06. Trials with p < 0.05: 8 of 23,749 testable; with p < 0.01: 3. Plateau (S2) held for 149 trials, none of which passed S1.

Distribution over all trials with at least 30 trades. `z gross` = gross mean / (sd / sqrt(n)): if the setups had no pre-cost information at all, about 5 % of trials would show |z| > 1.96 (2.5 % on each side).

| tf | trials | median net 1x R | net 1x > 0 | net 2x > 0 | median gross R | gross > 0 | median cost R/trade | z gross > 1.96 | z gross < -1.96 |
|---|---|---|---|---|---|---|---|---|---|
| 5m | 10,792 | -0.895 | 0.2 % | 0.0 % | +0.004 | 51.8 % | 0.893 | 2.1 % | 2.8 % |
| 15m | 8,793 | -0.493 | 1.9 % | 0.4 % | +0.001 | 50.4 % | 0.460 | 1.2 % | 2.9 % |
| 1h | 4,164 | -0.243 | 9.3 % | 2.3 % | +0.008 | 52.0 % | 0.223 | 2.0 % | 2.6 % |
| all | 23,749 | -0.621 | 2.4 % | 0.5 % | +0.004 | 51.3 % | 0.601 | 1.7 % | 2.8 % |

Top 20 discovery trials by t (net R at 1x; none passes FDR):

| trial | trades | win rate | net 1x | net 2x | gross | cost/trade | vs random 1x | t | p | q | plateau |
|---|---|---|---|---|---|---|---|---|---|---|---|
| cdl_tweezer_top reversed 1h b100-t2 F04 | 34 | 64.7 % | +0.742 | +0.556 | +0.941 | 0.199 | +0.974 | 2.98 | 1.43e-03 | 1.00e+00 | no |
| ema_cross native 15m b100-t1 F05 | 42 | 76.2 % | +0.383 | +0.226 | +0.545 | 0.162 | +0.950 | 2.97 | 1.50e-03 | 1.00e+00 | yes |
| cdl_tweezer_top reversed 1h b100-t3 F04 | 30 | 56.7 % | +1.054 | +0.867 | +1.267 | 0.213 | +1.286 | 2.77 | 2.78e-03 | 1.00e+00 | yes |
| cdl_bullish_engulfing native 1h b100-t1 F04 | 57 | 70.2 % | +0.258 | +0.121 | +0.404 | 0.146 | +0.490 | 2.10 | 1.80e-02 | 1.00e+00 | yes |
| ema_cross native 15m b100-t3 F05 | 35 | 45.7 % | +0.681 | +0.521 | +0.854 | 0.172 | +1.248 | 1.91 | 2.80e-02 | 1.00e+00 | yes |
| ema_cross native 15m b100-trail F05 | 46 | 58.7 % | +0.281 | +0.108 | +0.460 | 0.178 | +0.848 | 1.76 | 3.88e-02 | 1.00e+00 | yes |
| fib_ext native 15m b100-lvl F07 | 53 | 60.4 % | +0.299 | +0.188 | +0.411 | 0.111 | +0.866 | 1.72 | 4.28e-02 | 1.00e+00 | no |
| cdl_evening_doji_star native 15m b100-t3 F09 | 31 | 48.4 % | +0.614 | +0.351 | +0.860 | 0.246 | +1.181 | 1.69 | 4.56e-02 | 1.00e+00 | yes |
| bos reversed 1h b100-trail F07 | 42 | 40.5 % | +0.746 | +0.522 | +0.971 | 0.225 | +0.978 | 1.58 | 5.73e-02 | 1.00e+00 | yes |
| cdl_three_outside_up native 1h b100-t3 F00 | 59 | 39.0 % | +0.388 | +0.284 | +0.520 | 0.131 | +0.620 | 1.54 | 6.22e-02 | 1.00e+00 | yes |
| cdl_tweezer_top reversed 1h b100-t2 F07 | 49 | 51.0 % | +0.330 | +0.140 | +0.531 | 0.201 | +0.562 | 1.53 | 6.29e-02 | 1.00e+00 | no |
| ema_cross native 15m b025-trail F05 | 47 | 59.6 % | +0.304 | +0.069 | +0.544 | 0.240 | +0.871 | 1.51 | 6.59e-02 | 1.00e+00 | yes |
| cdl_three_outside_up native 1h b100-t1 F09 | 60 | 65.0 % | +0.187 | +0.081 | +0.300 | 0.113 | +0.419 | 1.49 | 6.78e-02 | 1.00e+00 | no |
| cdl_bullish_engulfing native 1h b025-lvl F04 | 50 | 60.0 % | +0.283 | +0.050 | +0.526 | 0.243 | +0.515 | 1.49 | 6.81e-02 | 1.00e+00 | yes |
| ema_cross reversed 15m b100-trail F05 | 31 | 61.3 % | +0.445 | +0.122 | +0.773 | 0.328 | +1.011 | 1.49 | 6.88e-02 | 1.00e+00 | no |
| fib_touch native 1h b025-trail F09 | 150 | 40.0 % | +0.536 | +0.278 | +0.802 | 0.266 | +0.768 | 1.48 | 6.96e-02 | 1.00e+00 | no |
| cdl_evening_doji_star native 15m b100-t3 F00 | 44 | 43.2 % | +0.424 | +0.159 | +0.674 | 0.250 | +0.991 | 1.42 | 7.76e-02 | 1.00e+00 | yes |
| cdl_three_outside_up native 1h b100-t1 F08 | 33 | 69.7 % | +0.243 | +0.100 | +0.394 | 0.151 | +0.475 | 1.38 | 8.31e-02 | 1.00e+00 | yes |
| fib_ext native 15m b025-t3 F09 | 67 | 35.8 % | +0.319 | +0.216 | +0.433 | 0.114 | +0.885 | 1.37 | 8.60e-02 | 1.00e+00 | no |
| sd_formed native 1h b025-t0.25 F00 | 30 | 93.3 % | +0.081 | -0.005 | +0.167 | 0.086 | +0.312 | 1.35 | 8.78e-02 | 1.00e+00 | no |


## Appendix C. Random-entry baseline (control)

Random direction at random candles (p = 0.5, seed 1), stop 1 ATR, target 1 R, same costs. Net = minus the cost, gross = zero within noise, as it should be.

| symbol | tf | period | trades | net 1x R | net 2x R | gross R | cost R/trade |
|---|---|---|---|---|---|---|---|
| BTCUSDT | 5m | discovery | 26859 | -1.1004 | -2.1949 | -0.0059 | 1.0945 |
| BTCUSDT | 5m | validation | 14804 | -1.0099 | -2.0269 | +0.0071 | 1.0170 |
| ETHUSDT | 5m | discovery | 25793 | -0.5882 | -1.1685 | -0.0079 | 0.5804 |
| ETHUSDT | 5m | validation | 14339 | -0.7437 | -1.4834 | -0.0040 | 0.7397 |
| BTCUSDT | 15m | discovery | 8000 | -0.5668 | -1.1338 | +0.0003 | 0.5671 |
| BTCUSDT | 15m | validation | 4325 | -0.5326 | -1.0545 | -0.0106 | 0.5220 |
| ETHUSDT | 15m | discovery | 7829 | -0.3358 | -0.6606 | -0.0109 | 0.3249 |
| ETHUSDT | 15m | validation | 4249 | -0.3919 | -0.7787 | -0.0051 | 0.3868 |
| BTCUSDT | 1h | discovery | 1864 | -0.2314 | -0.5004 | +0.0376 | 0.2690 |
| BTCUSDT | 1h | validation | 1057 | -0.3296 | -0.5780 | -0.0814 | 0.2482 |
| ETHUSDT | 1h | discovery | 1936 | -0.1167 | -0.2852 | +0.0517 | 0.1684 |
| ETHUSDT | 1h | validation | 1023 | -0.2453 | -0.4378 | -0.0526 | 0.1927 |


## Appendix D. Validation of the exploratory rules (information only, cannot survive)

Frozen before validation in `reports/research/finalists.json`. Pre-registered criteria: S3 >= 100 validation trades, S4 net at 2x > 0, S5 80 % interval lower bound > 0, S6 ETH months 1-18 not contradicted. S1/S2 are failed by construction for exploratory rules.

| rule | discovery trades / net 1x | val trades | val win rate | val net 1x | 80 % interval | val net 2x | val gross | cost/trade | val max DD | matched random 1x | vs matched random | ETH months 1-18 | failed |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| ema_cross native 15m b100-t3 F07 | 106 / +0.223 | 84 | 17.9 % | -0.417 | -0.655 .. -0.164 | -0.547 | -0.286 | 0.131 | 50.7 | -0.363 | -0.054 | not_contradicted (212, +0.026) | S1, S2, S3, S4, S5 |
| fib_ext native 15m b100-trail F07 | 185 / +0.062 | 108 | 37.0 % | -0.257 | -0.372 .. -0.135 | -0.348 | -0.166 | 0.091 | 28.7 | -0.295 | +0.038 | not_contradicted (299, -0.046) | S1, S2, S4, S5 |
| fib_touch native 1h b025-trail F09 | 150 / +0.536 | 83 | 32.5 % | -0.214 | -0.475 .. +0.060 | -0.476 | +0.049 | 0.263 | 33.0 | -0.287 | +0.073 | not_contradicted (223, +0.191) | S1, S2, S3, S4, S5 |
| fib_leg native 1h b025-trail F09 | 143 / +0.453 | 79 | 31.6 % | -0.321 | -0.576 .. -0.040 | -0.536 | -0.107 | 0.214 | 28.6 | -0.287 | -0.034 | not_contradicted (209, +0.128) | S1, S2, S3, S4, S5 |
| pullback reversed 1h b100-t1 F08 | 180 / +0.096 | 92 | 54.3 % | -0.049 | -0.226 .. +0.126 | -0.183 | +0.087 | 0.136 | 14.5 | -0.220 | +0.171 | contradicted (241, -0.144) | S1, S2, S3, S4, S5, S6 |
| cdl_three_outside_up native 1h b025-trail F00 | 100 / +0.171 | 56 | 39.3 % | -0.098 | -0.327 .. +0.127 | -0.236 | +0.042 | 0.140 | 12.0 | -0.506 | +0.407 | not_contradicted (127, -0.073) | S1, S2, S3, S4, S5 |
| tl_formed reversed 1h b100-trail F07 | 122 / +0.201 | 79 | 31.6 % | -0.138 | -0.374 .. +0.107 | -0.329 | +0.050 | 0.188 | 16.6 | -0.154 | +0.016 | not_contradicted (212, -0.052) | S1, S2, S3, S4, S5 |
| swing native 1h b100-lvl F07 | 132 / +0.101 | 85 | 45.9 % | +0.014 | -0.154 .. +0.183 | -0.098 | +0.126 | 0.112 | 14.4 | -0.109 | +0.123 | not_contradicted (234, -0.080) | S1, S2, S3, S4, S5 |
| cdl_tweezer_top reversed 1h b025-trail F00 | 100 / +0.394 | 43 | 18.6 % | -0.568 | -1.153 .. +0.087 | -1.090 | -0.045 | 0.523 | 32.3 | -0.506 | -0.062 | contradicted (110, -0.530) | S1, S2, S3, S4, S5, S6 |
| choch reversed 1h b100-trail F09 | 121 / +0.166 | 62 | 40.3 % | +0.046 | -0.217 .. +0.314 | -0.152 | +0.246 | 0.200 | 8.9 | -0.288 | +0.333 | not_contradicted (139, -0.001) | S1, S2, S3, S4, S5 |

On average the 10 rules made +0.240 R per trade in discovery and -0.200 R in validation: the discovery edge was selection noise.

## Appendix E. Walk-forward (3 months train, 1 month test, months 1-18)

Each month the variant with the best train-window net expectancy (>= 30 train trades) among the rule and its plateau neighbours is traded in the next month. Test months 2025-02..2025-10 lie inside the discovery data the rule was picked from (not independent); 2025-11..2026-05 are validation months.

| rule | variants | OOS trades | OOS net 1x | OOS net 2x | discovery months: trades / net | validation months: trades / net | positive test months |
|---|---|---|---|---|---|---|---|
| ema_cross native 15m b100-t3 F07 | 3 | 186 | -0.020 | -0.200 | 94 / +0.373 | 92 / -0.422 | 8 of 16 |
| fib_ext native 15m b100-trail F07 | 3 | 252 | -0.080 | -0.179 | 143 / +0.034 | 109 / -0.228 | 6 of 16 |
| fib_touch native 1h b025-trail F09 | 3 | 198 | +0.008 | -0.241 | 111 / +0.115 | 87 / -0.129 | 5 of 16 |
| fib_leg native 1h b025-trail F09 | 3 | 188 | -0.008 | -0.197 | 105 / +0.005 | 83 / -0.026 | 9 of 16 |
| pullback reversed 1h b100-t1 F08 | 3 | 206 | +0.048 | -0.099 | 133 / +0.099 | 73 / -0.045 | 8 of 16 |
| cdl_three_outside_up native 1h b025-trail F00 | 3 | 16 | +0.074 | -0.079 | 9 / +0.552 | 7 / -0.541 | 1 of 2 |
| tl_formed reversed 1h b100-trail F07 | 3 | 127 | +0.007 | -0.316 | 57 / +0.420 | 70 / -0.329 | 4 of 12 |
| swing native 1h b100-lvl F07 | 3 | 209 | -0.072 | -0.239 | 118 / -0.103 | 91 / -0.031 | 7 of 16 |
| cdl_tweezer_top reversed 1h b025-trail F00 | 3 | 34 | -0.364 | -0.955 | 34 / -0.364 | 0 / - | 1 of 4 |
| choch reversed 1h b100-trail F09 | 3 | 74 | -0.089 | -0.550 | 55 / -0.030 | 19 / -0.259 | 3 of 7 |


## Appendix F. Regimes, volatility, sessions and UTC hours (validation trades, net R at 1x)

`top-3 hours share` = net R of the best three signal hours divided by the total net R (only defined when the total is positive; above 1 means the other 21 hours lost money in sum).

| rule | ADX regime (trades/net) | volatility tercile | session | top-3 hours share |
|---|---|---|---|---|
| ema_cross native 15m b100-t3 F07 | with_trend 84/-0.42 | high 51/-0.57; low 15/+0.40; mid 18/-0.68 | asia 19/-0.10; late 9/+0.20; london 13/-1.17; newyork 24/-0.45; overlap 19/-0.47 | - |
| fib_ext native 15m b100-trail F07 | with_trend 108/-0.26 | high 53/-0.22; low 31/-0.51; mid 24/-0.02 | asia 25/-0.06; late 9/-0.11; london 17/-0.36; newyork 20/-0.12; overlap 37/-0.45 | - |
| fib_touch native 1h b025-trail F09 | against_trend 23/-0.81; range 28/-0.55; with_trend 32/+0.51 | high 27/+0.29; low 25/-0.48; mid 31/-0.45 | london 18/-0.44; newyork 25/+0.33; overlap 40/-0.45 | - |
| fib_leg native 1h b025-trail F09 | against_trend 12/-0.52; range 26/-1.05; with_trend 41/+0.20 | high 24/+0.30; low 30/-0.65; mid 25/-0.52 | london 19/-0.57; newyork 18/+0.58; overlap 42/-0.60 | - |
| pullback reversed 1h b100-t1 F08 | range 92/-0.05 | high 19/-0.06; low 45/+0.03; mid 28/-0.17 | asia 26/+0.03; late 10/+0.05; london 18/-0.02; newyork 20/-0.24; overlap 18/-0.03 | - |
| cdl_three_outside_up native 1h b025-trail F00 | against_trend 38/+0.01; range 14/-0.23; with_trend 4/-0.63 | high 18/-0.10; low 17/+0.09; mid 21/-0.25 | asia 17/-0.22; late 9/+0.06; london 9/-0.36; newyork 11/-0.27; overlap 10/+0.40 | - |
| tl_formed reversed 1h b100-trail F07 | with_trend 79/-0.14 | high 19/+0.31; low 28/-0.16; mid 32/-0.38 | asia 34/-0.01; late 8/+0.44; london 18/-0.17; newyork 11/-0.35; overlap 8/-0.91 | - |
| swing native 1h b100-lvl F07 | with_trend 85/+0.01 | high 28/+0.10; low 31/-0.03; mid 26/-0.03 | asia 24/-0.35; late 13/+0.76; london 19/+0.03; newyork 21/+0.04; overlap 8/-0.20 | 12.58 |
| cdl_tweezer_top reversed 1h b025-trail F00 | against_trend 8/+0.32; range 17/-0.69; with_trend 18/-0.84 | high 13/-0.99; low 16/-0.22; mid 14/-0.58 | asia 10/-0.36; late 10/+0.64; london 12/-1.48; newyork 6/-1.40; overlap 5/-0.22 | - |
| choch reversed 1h b100-trail F09 | against_trend 15/-0.38; range 25/-0.24; with_trend 22/+0.66 | high 16/+0.21; low 25/+0.24; mid 21/-0.30 | london 24/+0.15; newyork 16/+0.11; overlap 22/-0.11 | 8.15 |


## Appendix G. Confluence (pre-registered, limited)

Confluence family: 144 trials (count >= 1 / 2 / 3, rule b025-t2), 120 with >= 30 trades; positive at 1x: 1; lowest p 4.07e-01.

| threshold | trials (>= 30 trades) | median net 1x | median gross | positive at 1x | median trades |
|---|---|---|---|---|---|
| >= 1 | 42 | -0.629 | +0.011 | 0.0 % | 450 |
| >= 2 | 41 | -0.660 | +0.017 | 0.0 % | 387 |
| >= 3 | 37 | -0.703 | -0.017 | 2.7 % | 186 |

Descriptive (not trials): unfiltered b025-t2 trades of each confluence setup split by the exact confluence count of the signal, discovery period, pooled over both directions, per time frame (trades / net R at 1x / net win rate):

| tf | setup | count 0 | count 1 | count 2 | count 3 | count 4 |
|---|---|---|---|---|---|---|
| 5m | choch | 118 / -2.33 / 14 % | 961 / -2.08 / 20 % | 2070 / -1.97 / 22 % | 1010 / -2.06 / 22 % | 215 / -2.03 / 18 % |
| 5m | fakeout | 57 / -0.90 / 30 % | 581 / -1.20 / 29 % | 1625 / -1.33 / 27 % | 850 / -1.33 / 28 % | 207 / -1.15 / 24 % |
| 5m | fib_touch | 288 / -1.06 / 33 % | 1584 / -1.06 / 29 % | 2433 / -1.08 / 30 % | 1070 / -1.20 / 27 % | 210 / -1.49 / 22 % |
| 5m | pullback | 290 / -0.92 / 29 % | 1432 / -0.82 / 31 % | 2109 / -0.78 / 31 % | 903 / -0.81 / 31 % | 190 / -0.89 / 26 % |
| 5m | retest | 5 / -0.32 / 40 % | 324 / -0.63 / 29 % | 635 / -0.50 / 35 % | 252 / -0.62 / 30 % | 46 / -0.58 / 33 % |
| 5m | role_reversal | 69 / -1.30 / 23 % | 803 / -1.23 / 30 % | 1944 / -0.93 / 33 % | 914 / -1.02 / 31 % | 170 / -1.23 / 24 % |
| 5m | sd_test | 5 / -2.48 / 0 % | 120 / -1.50 / 30 % | 437 / -2.44 / 16 % | 213 / -2.38 / 22 % | 48 / -1.84 / 27 % |
| 5m | sweep | 41 / -0.72 / 29 % | 182 / -1.12 / 24 % | 504 / -1.29 / 29 % | 286 / -1.63 / 24 % | 65 / -1.58 / 20 % |
| 15m | choch | 23 / -0.55 / 39 % | 296 / -1.08 / 28 % | 655 / -0.90 / 32 % | 331 / -1.09 / 25 % | 64 / -0.92 / 28 % |
| 15m | fakeout | 18 / -0.73 / 22 % | 150 / -0.58 / 29 % | 433 / -0.79 / 29 % | 217 / -0.71 / 32 % | 56 / -0.85 / 29 % |
| 15m | fib_touch | 90 / -0.74 / 30 % | 435 / -0.62 / 31 % | 808 / -0.59 / 33 % | 405 / -0.56 / 34 % | 69 / -0.86 / 28 % |
| 15m | pullback | 114 / -0.29 / 36 % | 406 / -0.40 / 34 % | 586 / -0.49 / 31 % | 243 / -0.39 / 34 % | 45 / -0.90 / 20 % |
| 15m | retest | 2 / -1.34 / 0 % | 112 / -0.63 / 21 % | 234 / -0.11 / 38 % | 103 / -0.34 / 31 % | 22 / -0.22 / 36 % |
| 15m | role_reversal | 23 / -1.08 / 17 % | 216 / -0.60 / 31 % | 633 / -0.50 / 33 % | 310 / -0.63 / 28 % | 70 / -0.80 / 29 % |
| 15m | sd_test | 1 / -1.27 / 0 % | 31 / -0.95 / 39 % | 83 / -1.10 / 30 % | 61 / -1.18 / 28 % | 14 / -1.44 / 21 % |
| 15m | sweep | 21 / -0.96 / 14 % | 108 / -0.32 / 38 % | 221 / -0.58 / 34 % | 117 / -0.78 / 31 % | 22 / -1.45 / 14 % |
| 1h | choch | 6 / -1.58 / 0 % | 73 / -0.42 / 36 % | 185 / -0.43 / 34 % | 101 / -0.49 / 33 % | 16 / -0.44 / 31 % |
| 1h | fakeout | 2 / -1.11 / 0 % | 48 / -0.26 / 33 % | 164 / -0.43 / 32 % | 56 / -0.31 / 34 % | 11 / -0.28 / 36 % |
| 1h | fib_touch | 20 / +0.13 / 45 % | 113 / -0.36 / 32 % | 195 / -0.19 / 36 % | 108 / -0.21 / 35 % | 20 / -0.87 / 15 % |
| 1h | pullback | 27 / -0.25 / 30 % | 136 / -0.12 / 37 % | 187 / -0.23 / 33 % | 70 / +0.02 / 40 % | 23 / -0.04 / 39 % |
| 1h | retest | 0 | 32 / -0.28 / 28 % | 69 / -0.23 / 30 % | 38 / -0.29 / 29 % | 4 / -1.19 / 0 % |
| 1h | role_reversal | 5 / -1.48 / 0 % | 58 / -0.28 / 36 % | 157 / -0.27 / 32 % | 85 / -0.26 / 34 % | 16 / -0.34 / 31 % |
| 1h | sd_test | 1 / +0.97 / 100 % | 8 / -1.32 / 12 % | 31 / -0.51 / 32 % | 7 / -0.60 / 29 % | 5 / -1.44 / 0 % |
| 1h | sweep | 16 / -0.36 / 31 % | 58 / -0.19 / 38 % | 107 / -0.43 / 30 % | 44 / -0.25 / 39 % | 3 / +0.77 / 67 % |

See the gate report's confluence note in the warnings for how to read this table.


## Appendix H. Win-rate frontier

Win rate against net expectancy at 1x costs across the target ladder, no filter, discovery data. Averages over all setups, both directions and all time frames (trials with >= 30 trades):

| stop buffer | target | trials | mean target-hit rate | mean net win rate | mean net 1x | mean gross | mean cost | positive | mean largest loss |
|---|---|---|---|---|---|---|---|---|---|
| 0.25 ATR | 0.25 R | 285 | 76.8 % | 15.9 % | -1.186 | -0.040 | 1.146 | 0.4 % | -9.06 |
| 0.25 ATR | 0.5 R | 284 | 65.7 % | 28.3 % | -1.161 | -0.014 | 1.147 | 0.4 % | -9.04 |
| 0.25 ATR | 0.75 R | 284 | 56.8 % | 32.7 % | -1.149 | -0.005 | 1.144 | 0.0 % | -8.98 |
| 0.25 ATR | 1 R | 284 | 50.1 % | 33.6 % | -1.140 | +0.002 | 1.141 | 0.4 % | -8.88 |
| 0.25 ATR | 1.5 R | 284 | 40.2 % | 31.5 % | -1.131 | +0.005 | 1.136 | 0.7 % | -8.71 |
| 0.25 ATR | 2 R | 283 | 33.8 % | 28.5 % | -1.119 | +0.016 | 1.135 | 1.8 % | -8.62 |
| 0.25 ATR | 3 R | 281 | 25.7 % | 23.3 % | -1.105 | +0.028 | 1.133 | 1.8 % | -8.49 |
| 1.0 ATR | 0.25 R | 297 | 78.9 % | 29.8 % | -0.539 | -0.013 | 0.525 | 0.0 % | -4.47 |
| 1.0 ATR | 0.5 R | 296 | 66.4 % | 44.8 % | -0.527 | -0.003 | 0.524 | 0.7 % | -4.45 |
| 1.0 ATR | 0.75 R | 296 | 56.9 % | 46.3 % | -0.524 | -0.003 | 0.521 | 1.4 % | -4.47 |
| 1.0 ATR | 1 R | 295 | 49.8 % | 44.1 % | -0.521 | -0.003 | 0.518 | 1.7 % | -4.38 |
| 1.0 ATR | 1.5 R | 294 | 40.2 % | 38.1 % | -0.507 | +0.006 | 0.513 | 4.4 % | -4.31 |
| 1.0 ATR | 2 R | 294 | 33.6 % | 32.7 % | -0.497 | +0.011 | 0.508 | 4.4 % | -4.24 |
| 1.0 ATR | 3 R | 292 | 25.4 % | 25.2 % | -0.486 | +0.019 | 0.504 | 5.8 % | -4.13 |

Two win rates: the **target-hit rate** (what a chart reader counts as a win) and the **net win rate** (net R > 0 after costs, the protocol's definition). A small target raises the hit rate, but fees are paid on every trade, so many hits are net losses and expectancy does not improve; each full stop-out costs 1 R plus fees. With the 0.25-ATR buffer the structural stop is often only a few basis points away, so fees alone are several R on those trades (hence the large 'largest loss').

Per setup (native direction, 15m, no filter): target-hit rate / net win rate / net R at 1x for each target. Same table for every direction and time frame: `reports/trial-ledger.csv`.

Stop buffer 0.25 ATR:

| setup | 0.25 R | 0.5 R | 0.75 R | 1 R | 1.5 R | 2 R | 3 R |
|---|---|---|---|---|---|---|---|
| swing | 78 % / 29 % / -0.44 | 66 % / 53 % / -0.38 | 56 % / 52 % / -0.36 | 49 % / 48 % / -0.33 | 37 % / 37 % / -0.37 | 34 % / 34 % / -0.30 | 23 % / 23 % / -0.35 |
| structure_label | 78 % / 3 % / -1.07 | 67 % / 16 % / -1.04 | 57 % / 26 % / -1.04 | 50 % / 31 % / -1.04 | 40 % / 32 % / -1.01 | 33 % / 30 % / -1.00 | 25 % / 24 % / -0.97 |
| structure_trend | 77 % / 3 % / -1.10 | 67 % / 15 % / -1.06 | 56 % / 26 % / -1.08 | 49 % / 30 % / -1.08 | 40 % / 32 % / -1.06 | 33 % / 30 % / -1.04 | 25 % / 24 % / -1.04 |
| bos | 78 % / 18 % / -0.52 | 65 % / 43 % / -0.51 | 56 % / 47 % / -0.52 | 47 % / 44 % / -0.54 | 38 % / 37 % / -0.53 | 32 % / 32 % / -0.50 | 25 % / 25 % / -0.45 |
| choch | 79 % / 16 % / -0.52 | 65 % / 41 % / -0.52 | 55 % / 46 % / -0.55 | 48 % / 44 % / -0.53 | 37 % / 36 % / -0.56 | 31 % / 31 % / -0.56 | 23 % / 23 % / -0.57 |
| sr_break | 80 % / 23 % / -0.38 | 67 % / 55 % / -0.36 | 58 % / 55 % / -0.35 | 51 % / 50 % / -0.35 | 42 % / 41 % / -0.32 | 34 % / 34 % / -0.33 | 27 % / 27 % / -0.28 |
| sr_formed | 78 % / 28 % / -0.42 | 65 % / 51 % / -0.41 | 55 % / 50 % / -0.40 | 47 % / 45 % / -0.42 | 38 % / 38 % / -0.39 | 30 % / 30 % / -0.45 | 22 % / 23 % / -0.45 |
| role_reversal | 79 % / 23 % / -0.40 | 66 % / 51 % / -0.39 | 57 % / 54 % / -0.38 | 50 % / 49 % / -0.40 | 40 % / 40 % / -0.39 | 33 % / 33 % / -0.38 | 25 % / 25 % / -0.38 |
| sd_test | 83 % / 9 % / -0.80 | 73 % / 26 % / -0.73 | 60 % / 34 % / -0.76 | 55 % / 38 % / -0.70 | 46 % / 40 % / -0.64 | 39 % / 37 % / -0.62 | 26 % / 25 % / -0.73 |
| sd_formed | 73 % / 54 % / -0.28 | 65 % / 63 % / -0.22 | 56 % / 56 % / -0.20 | 49 % / 49 % / -0.21 | 45 % / 45 % / -0.08 | 36 % / 36 % / -0.10 | 31 % / 31 % / +0.07 |
| sd_broken | 73 % / 12 % / -0.70 | 61 % / 29 % / -0.70 | 53 % / 41 % / -0.69 | 50 % / 44 % / -0.61 | 39 % / 35 % / -0.65 | 30 % / 28 % / -0.74 | 25 % / 25 % / -0.61 |
| ema_stack | 80 % / 63 % / -0.18 | 67 % / 65 % / -0.17 | 56 % / 56 % / -0.19 | 50 % / 50 % / -0.16 | 46 % / 46 % / -0.02 | 33 % / 34 % / -0.18 | 25 % / 26 % / -0.15 |
| ema_cross | 79 % / 44 % / -0.33 | 66 % / 58 % / -0.32 | 57 % / 55 % / -0.29 | 52 % / 51 % / -0.25 | 42 % / 42 % / -0.22 | 36 % / 36 % / -0.19 | 31 % / 31 % / -0.02 |
| tl_touch | 79 % / 3 % / -1.19 | 68 % / 13 % / -1.15 | 59 % / 21 % / -1.16 | 50 % / 25 % / -1.17 | 40 % / 29 % / -1.18 | 33 % / 28 % / -1.19 | 26 % / 24 % / -1.13 |
| tl_break | 80 % / 12 % / -0.60 | 67 % / 35 % / -0.59 | 57 % / 43 % / -0.59 | 51 % / 44 % / -0.56 | 41 % / 39 % / -0.55 | 35 % / 34 % / -0.52 | 25 % / 25 % / -0.55 |
| tl_formed | 77 % / 4 % / -1.08 | 66 % / 17 % / -1.05 | 56 % / 26 % / -1.05 | 49 % / 30 % / -1.05 | 39 % / 31 % / -1.05 | 33 % / 29 % / -1.02 | 26 % / 25 % / -0.95 |
| fib_touch | 80 % / 24 % / -0.68 | 67 % / 39 % / -0.68 | 58 % / 42 % / -0.64 | 51 % / 41 % / -0.61 | 39 % / 36 % / -0.58 | 33 % / 32 % / -0.55 | 24 % / 24 % / -0.54 |
| fib_ext | 84 % / 79 % / -0.07 | 71 % / 71 % / -0.05 | 63 % / 63 % / -0.02 | 48 % / 48 % / -0.15 | 44 % / 44 % / -0.02 | 40 % / 40 % / +0.10 | 24 % / 26 % / -0.11 |
| fib_leg | 81 % / 31 % / -0.59 | 68 % / 45 % / -0.56 | 59 % / 47 % / -0.48 | 52 % / 46 % / -0.42 | 40 % / 38 % / -0.37 | 35 % / 35 % / -0.26 | 25 % / 25 % / -0.27 |
| pullback | 78 % / 28 % / -0.41 | 66 % / 52 % / -0.39 | 56 % / 52 % / -0.38 | 50 % / 49 % / -0.35 | 40 % / 40 % / -0.33 | 32 % / 32 % / -0.37 | 26 % / 26 % / -0.31 |
| breakout | 80 % / 29 % / -0.34 | 68 % / 58 % / -0.32 | 58 % / 56 % / -0.32 | 51 % / 51 % / -0.32 | 41 % / 41 % / -0.32 | 33 % / 33 % / -0.34 | 25 % / 25 % / -0.32 |
| retest | 82 % / 41 % / -0.26 | 68 % / 63 % / -0.25 | 56 % / 55 % / -0.29 | 49 % / 49 % / -0.29 | 39 % / 39 % / -0.30 | 33 % / 33 % / -0.29 | 27 % / 27 % / -0.17 |
| fakeout | 80 % / 25 % / -0.41 | 67 % / 50 % / -0.41 | 59 % / 54 % / -0.36 | 51 % / 49 % / -0.39 | 41 % / 41 % / -0.36 | 33 % / 33 % / -0.37 | 28 % / 28 % / -0.22 |
| eq_levels | 79 % / 3 % / -1.04 | 65 % / 17 % / -1.05 | 56 % / 25 % / -1.05 | 49 % / 30 % / -1.04 | 39 % / 31 % / -1.04 | 33 % / 30 % / -1.03 | 25 % / 24 % / -1.02 |
| sweep | 79 % / 21 % / -0.51 | 67 % / 43 % / -0.49 | 57 % / 46 % / -0.50 | 48 % / 43 % / -0.55 | 40 % / 38 % / -0.51 | 35 % / 34 % / -0.46 | 27 % / 27 % / -0.42 |
| h1_stack | 79 % / 30 % / -0.45 | 67 % / 48 % / -0.42 | 57 % / 47 % / -0.44 | 52 % / 47 % / -0.40 | 44 % / 44 % / -0.32 | 36 % / 36 % / -0.35 | 29 % / 29 % / -0.26 |
| h1_trend | 81 % / 12 % / -0.73 | 68 % / 31 % / -0.72 | 58 % / 39 % / -0.72 | 48 % / 37 % / -0.77 | 38 % / 34 % / -0.78 | 31 % / 30 % / -0.80 | 21 % / 21 % / -0.87 |
| h4_stack | 73 % / 57 % / -0.33 | 61 % / 54 % / -0.34 | 54 % / 52 % / -0.27 | 47 % / 45 % / -0.28 | 28 % / 28 % / -0.52 | 26 % / 26 % / -0.45 | 19 % / 19 % / -0.48 |
| h4_trend | 80 % / 36 % / -0.39 | 62 % / 49 % / -0.46 | 51 % / 44 % / -0.49 | 43 % / 40 % / -0.52 | 33 % / 30 % / -0.57 | 27 % / 25 % / -0.58 | 20 % / 20 % / -0.58 |
| cdl_hammer | 81 % / 10 % / -0.55 | 67 % / 35 % / -0.56 | 57 % / 45 % / -0.56 | 52 % / 46 % / -0.53 | 39 % / 39 % / -0.59 | 32 % / 32 % / -0.62 | 22 % / 22 % / -0.68 |
| cdl_inverted_hammer | 80 % / 0 % / -1.70 | 68 % / 3 % / -1.68 | 60 % / 8 % / -1.64 | 53 % / 18 % / -1.65 | 40 % / 23 % / -1.70 | 34 % / 23 % / -1.70 | 26 % / 24 % / -1.67 |
| cdl_bullish_engulfing | 77 % / 18 % / -0.50 | 64 % / 43 % / -0.49 | 54 % / 49 % / -0.50 | 46 % / 44 % / -0.53 | 37 % / 37 % / -0.51 | 31 % / 31 % / -0.50 | 22 % / 22 % / -0.54 |
| cdl_piercing_line | 79 % / 17 % / -0.49 | 67 % / 43 % / -0.47 | 61 % / 49 % / -0.41 | 54 % / 51 % / -0.39 | 42 % / 42 % / -0.43 | 35 % / 35 % / -0.43 | 22 % / 22 % / -0.58 |
| cdl_morning_star | 85 % / 30 % / -0.37 | 75 % / 48 % / -0.31 | 57 % / 50 % / -0.43 | 57 % / 52 % / -0.29 | 50 % / 50 % / -0.19 | 40 % / 40 % / -0.24 | 35 % / 35 % / -0.05 |
| cdl_morning_doji_star | 85 % / 29 % / -0.35 | 76 % / 54 % / -0.28 | 66 % / 51 % / -0.27 | 56 % / 54 % / -0.30 | 44 % / 44 % / -0.32 | 39 % / 39 % / -0.25 | 32 % / 32 % / -0.16 |
| cdl_three_white_soldiers | 57 % / 29 % / -0.54* | 36 % / 36 % / -0.72* | 36 % / 36 % / -0.63* | 21 % / 21 % / -0.83* | 7 % / 7 % / -1.08* | 7 % / 7 % / -1.04* | 0 % / 0 % / -1.26* |
| cdl_tweezer_bottom | 77 % / 7 % / -0.76 | 68 % / 26 % / -0.70 | 60 % / 39 % / -0.67 | 51 % / 43 % / -0.69 | 40 % / 37 % / -0.70 | 33 % / 31 % / -0.72 | 24 % / 24 % / -0.74 |
| cdl_bullish_harami | 81 % / 5 % / -0.75 | 68 % / 26 % / -0.75 | 60 % / 38 % / -0.73 | 55 % / 44 % / -0.66 | 46 % / 42 % / -0.63 | 34 % / 32 % / -0.74 | 27 % / 26 % / -0.70 |
| cdl_bullish_harami_cross | 79 % / 6 % / -0.79 | 69 % / 26 % / -0.75 | 62 % / 39 % / -0.69 | 55 % / 40 % / -0.69 | 44 % / 42 % / -0.68 | 40 % / 39 % / -0.58 | 29 % / 29 % / -0.64 |
| cdl_three_inside_up | 85 % / 33 % / -0.26* | 78 % / 70 % / -0.16* | 56 % / 56 % / -0.35* | 48 % / 48 % / -0.37* | 33 % / 33 % / -0.50* | 22 % / 22 % / -0.67* | 22 % / 22 % / -0.44* |
| cdl_three_outside_up | 78 % / 32 % / -0.34 | 64 % / 55 % / -0.35 | 53 % / 52 % / -0.38 | 48 % / 47 % / -0.36 | 38 % / 38 % / -0.38 | 32 % / 32 % / -0.36 | 24 % / 24 % / -0.34 |
| cdl_rising_three_methods | 100 % / 50 % / -0.02* | 50 % / 50 % / -0.52* | 50 % / 50 % / -0.39* | 0 % / 0 % / -1.27* | 0 % / 0 % / -1.27* | 0 % / 0 % / -1.27* | 0 % / 0 % / -1.27* |
| cdl_bullish_marubozu | 77 % / 10 % / -0.61 | 62 % / 32 % / -0.64 | 55 % / 42 % / -0.60 | 49 % / 43 % / -0.59 | 40 % / 39 % / -0.57 | 34 % / 33 % / -0.54 | 27 % / 27 % / -0.46 |
| cdl_dragonfly_doji | 87 % / 10 % / -0.59 | 73 % / 30 % / -0.59 | 61 % / 41 % / -0.61 | 53 % / 40 % / -0.62 | 41 % / 41 % / -0.67 | 35 % / 35 % / -0.64 | 25 % / 25 % / -0.71 |
| cdl_hanging_man | 74 % / 0 % / -1.96 | 64 % / 1 % / -1.92 | 57 % / 5 % / -1.88 | 50 % / 14 % / -1.88 | 38 % / 18 % / -1.92 | 29 % / 20 % / -1.98 | 26 % / 23 % / -1.84 |
| cdl_shooting_star | 78 % / 6 % / -0.62 | 68 % / 34 % / -0.59 | 60 % / 47 % / -0.56 | 52 % / 48 % / -0.54 | 39 % / 37 % / -0.61 | 31 % / 31 % / -0.66 | 22 % / 22 % / -0.71 |
| cdl_bearish_engulfing | 82 % / 16 % / -0.46 | 68 % / 46 % / -0.45 | 60 % / 52 % / -0.43 | 52 % / 49 % / -0.43 | 41 % / 41 % / -0.44 | 33 % / 33 % / -0.46 | 25 % / 25 % / -0.44 |
| cdl_dark_cloud_cover | 81 % / 15 % / -0.47 | 71 % / 45 % / -0.42 | 60 % / 51 % / -0.42 | 50 % / 47 % / -0.47 | 40 % / 40 % / -0.47 | 33 % / 33 % / -0.50 | 25 % / 25 % / -0.47 |
| cdl_evening_star | 79 % / 15 % / -0.45 | 69 % / 44 % / -0.41 | 67 % / 54 % / -0.28 | 62 % / 59 % / -0.21 | 53 % / 53 % / -0.13 | 42 % / 42 % / -0.18 | 29 % / 29 % / -0.28 |
| cdl_evening_doji_star | 87 % / 13 % / -0.35 | 72 % / 39 % / -0.36 | 65 % / 63 % / -0.29 | 59 % / 59 % / -0.26 | 52 % / 52 % / -0.13 | 48 % / 50 % / +0.05 | 40 % / 42 % / +0.23 |
| cdl_three_black_crows | 86 % / 43 % / -0.42* | 86 % / 57 % / -0.21* | 29 % / 14 % / -0.99* | 29 % / 29 % / -0.92* | 14 % / 14 % / -1.13* | 14 % / 14 % / -1.06* | 14 % / 14 % / -0.92* |
| cdl_tweezer_top | 80 % / 7 % / -0.70 | 65 % / 28 % / -0.73 | 57 % / 39 % / -0.70 | 49 % / 40 % / -0.72 | 41 % / 38 % / -0.68 | 35 % / 33 % / -0.66 | 25 % / 25 % / -0.68 |
| cdl_bearish_harami | 81 % / 4 % / -0.80 | 69 % / 21 % / -0.76 | 59 % / 34 % / -0.77 | 51 % / 39 % / -0.79 | 44 % / 37 % / -0.71 | 38 % / 36 % / -0.65 | 24 % / 23 % / -0.85 |
| cdl_bearish_harami_cross | 85 % / 4 % / -0.78 | 73 % / 17 % / -0.75 | 64 % / 33 % / -0.72 | 57 % / 37 % / -0.71 | 50 % / 42 % / -0.59 | 39 % / 37 % / -0.66 | 30 % / 30 % / -0.62 |
| cdl_three_inside_down | 76 % / 30 % / -0.45 | 64 % / 52 % / -0.44 | 58 % / 52 % / -0.38 | 48 % / 48 % / -0.42 | 30 % / 30 % / -0.63 | 27 % / 27 % / -0.57 | 18 % / 18 % / -0.66 |
| cdl_three_outside_down | 79 % / 29 % / -0.37 | 66 % / 53 % / -0.36 | 56 % / 52 % / -0.37 | 47 % / 47 % / -0.40 | 38 % / 38 % / -0.38 | 33 % / 33 % / -0.34 | 23 % / 23 % / -0.41 |
| cdl_falling_three_methods | - | - | - | - | - | - | - |
| cdl_bearish_marubozu | 77 % / 10 % / -0.65 | 64 % / 36 % / -0.64 | 56 % / 45 % / -0.63 | 50 % / 45 % / -0.59 | 40 % / 38 % / -0.58 | 34 % / 33 % / -0.58 | 26 % / 26 % / -0.54 |
| cdl_gravestone_doji | 82 % / 6 % / -0.62 | 67 % / 24 % / -0.65 | 62 % / 44 % / -0.55 | 54 % / 47 % / -0.56 | 42 % / 42 % / -0.58 | 31 % / 31 % / -0.71 | 20 % / 20 % / -0.84 |

`*` fewer than 30 trades.

Stop buffer 1.0 ATR:

| setup | 0.25 R | 0.5 R | 0.75 R | 1 R | 1.5 R | 2 R | 3 R |
|---|---|---|---|---|---|---|---|
| swing | 79 % / 49 % / -0.26 | 67 % / 66 % / -0.20 | 59 % / 59 % / -0.15 | 51 % / 51 % / -0.17 | 40 % / 40 % / -0.19 | 31 % / 31 % / -0.24 | 24 % / 24 % / -0.20 |
| structure_label | 79 % / 20 % / -0.42 | 66 % / 51 % / -0.40 | 57 % / 54 % / -0.39 | 50 % / 49 % / -0.39 | 39 % / 39 % / -0.39 | 32 % / 32 % / -0.40 | 24 % / 24 % / -0.39 |
| structure_trend | 80 % / 18 % / -0.42 | 67 % / 51 % / -0.41 | 57 % / 53 % / -0.41 | 52 % / 50 % / -0.37 | 42 % / 42 % / -0.34 | 34 % / 34 % / -0.35 | 25 % / 25 % / -0.39 |
| bos | 79 % / 39 % / -0.31 | 66 % / 59 % / -0.30 | 55 % / 54 % / -0.31 | 48 % / 48 % / -0.32 | 38 % / 38 % / -0.31 | 33 % / 33 % / -0.26 | 27 % / 27 % / -0.18 |
| choch | 80 % / 40 % / -0.29 | 67 % / 60 % / -0.27 | 57 % / 56 % / -0.28 | 51 % / 51 % / -0.24 | 40 % / 40 % / -0.26 | 34 % / 35 % / -0.23 | 24 % / 25 % / -0.28 |
| sr_break | 80 % / 51 % / -0.24 | 68 % / 66 % / -0.21 | 58 % / 58 % / -0.21 | 53 % / 52 % / -0.18 | 41 % / 41 % / -0.18 | 34 % / 34 % / -0.21 | 26 % / 26 % / -0.16 |
| sr_formed | 79 % / 49 % / -0.26 | 65 % / 61 % / -0.26 | 56 % / 55 % / -0.25 | 48 % / 49 % / -0.24 | 37 % / 37 % / -0.30 | 30 % / 31 % / -0.30 | 25 % / 26 % / -0.20 |
| role_reversal | 79 % / 48 % / -0.26 | 67 % / 65 % / -0.25 | 57 % / 57 % / -0.24 | 49 % / 49 % / -0.24 | 39 % / 39 % / -0.25 | 34 % / 34 % / -0.22 | 26 % / 26 % / -0.17 |
| sd_test | 84 % / 31 % / -0.33 | 69 % / 54 % / -0.32 | 64 % / 59 % / -0.24 | 53 % / 52 % / -0.30 | 47 % / 47 % / -0.19 | 39 % / 39 % / -0.16 | 30 % / 30 % / -0.14 |
| sd_formed | 75 % / 64 % / -0.22 | 67 % / 66 % / -0.16 | 58 % / 58 % / -0.14 | 54 % / 54 % / -0.07 | 45 % / 45 % / -0.02 | 42 % / 42 % / +0.11 | 33 % / 33 % / +0.19 |
| sd_broken | 71 % / 32 % / -0.45 | 58 % / 50 % / -0.47 | 52 % / 49 % / -0.42 | 44 % / 42 % / -0.44 | 36 % / 36 % / -0.43 | 32 % / 32 % / -0.36 | 24 % / 24 % / -0.38 |
| ema_stack | 80 % / 75 % / -0.12 | 66 % / 66 % / -0.13 | 56 % / 56 % / -0.12 | 50 % / 50 % / -0.10 | 42 % / 42 % / -0.05 | 34 % / 35 % / -0.07 | 25 % / 27 % / -0.06 |
| ema_cross | 80 % / 63 % / -0.20 | 66 % / 64 % / -0.21 | 58 % / 58 % / -0.17 | 51 % / 51 % / -0.15 | 42 % / 42 % / -0.14 | 36 % / 36 % / -0.10 | 30 % / 30 % / +0.03 |
| tl_touch | 80 % / 16 % / -0.44 | 66 % / 45 % / -0.46 | 55 % / 50 % / -0.46 | 48 % / 47 % / -0.46 | 40 % / 40 % / -0.43 | 34 % / 34 % / -0.40 | 26 % / 26 % / -0.36 |
| tl_break | 79 % / 34 % / -0.32 | 68 % / 59 % / -0.29 | 57 % / 55 % / -0.30 | 50 % / 49 % / -0.30 | 38 % / 38 % / -0.34 | 32 % / 32 % / -0.33 | 22 % / 22 % / -0.39 |
| tl_formed | 79 % / 20 % / -0.43 | 65 % / 49 % / -0.43 | 56 % / 52 % / -0.41 | 49 % / 48 % / -0.41 | 40 % / 39 % / -0.38 | 33 % / 33 % / -0.36 | 24 % / 24 % / -0.39 |
| fib_touch | 80 % / 30 % / -0.58 | 68 % / 43 % / -0.55 | 58 % / 45 % / -0.52 | 52 % / 46 % / -0.45 | 41 % / 40 % / -0.38 | 35 % / 35 % / -0.31 | 24 % / 24 % / -0.38 |
| fib_ext | 84 % / 82 % / -0.05 | 70 % / 70 % / -0.05 | 60 % / 60 % / -0.04 | 48 % / 48 % / -0.13 | 46 % / 46 % / +0.06 | 37 % / 37 % / +0.03 | 30 % / 30 % / +0.11 |
| fib_leg | 80 % / 34 % / -0.53 | 69 % / 47 % / -0.48 | 58 % / 48 % / -0.43 | 54 % / 49 % / -0.30 | 46 % / 45 % / -0.11 | 37 % / 38 % / -0.06 | 28 % / 29 % / -0.05 |
| pullback | 79 % / 51 % / -0.25 | 65 % / 63 % / -0.25 | 56 % / 56 % / -0.23 | 49 % / 49 % / -0.22 | 40 % / 40 % / -0.20 | 34 % / 34 % / -0.19 | 26 % / 26 % / -0.14 |
| breakout | 79 % / 54 % / -0.23 | 67 % / 66 % / -0.21 | 59 % / 59 % / -0.19 | 51 % / 51 % / -0.20 | 39 % / 39 % / -0.25 | 34 % / 34 % / -0.18 | 26 % / 26 % / -0.18 |
| retest | 80 % / 60 % / -0.19 | 65 % / 65 % / -0.21 | 56 % / 56 % / -0.20 | 47 % / 47 % / -0.24 | 40 % / 40 % / -0.19 | 37 % / 37 % / -0.08 | 29 % / 29 % / -0.01 |
| fakeout | 80 % / 48 % / -0.25 | 68 % / 64 % / -0.23 | 57 % / 57 % / -0.24 | 50 % / 50 % / -0.24 | 39 % / 39 % / -0.25 | 34 % / 34 % / -0.21 | 25 % / 25 % / -0.23 |
| eq_levels | 78 % / 19 % / -0.44 | 66 % / 48 % / -0.43 | 56 % / 51 % / -0.44 | 49 % / 48 % / -0.43 | 39 % / 38 % / -0.44 | 33 % / 33 % / -0.42 | 24 % / 24 % / -0.43 |
| sweep | 81 % / 42 % / -0.27 | 66 % / 59 % / -0.28 | 56 % / 54 % / -0.30 | 51 % / 50 % / -0.26 | 42 % / 42 % / -0.22 | 33 % / 33 % / -0.28 | 25 % / 25 % / -0.25 |
| h1_stack | 81 % / 47 % / -0.24 | 65 % / 59 % / -0.28 | 60 % / 59 % / -0.20 | 55 % / 55 % / -0.15 | 41 % / 41 % / -0.22 | 34 % / 34 % / -0.25 | 29 % / 29 % / -0.10 |
| h1_trend | 80 % / 34 % / -0.34 | 66 % / 56 % / -0.34 | 57 % / 54 % / -0.33 | 52 % / 51 % / -0.30 | 39 % / 39 % / -0.35 | 31 % / 31 % / -0.39 | 26 % / 26 % / -0.29 |
| h4_stack | 74 % / 61 % / -0.24 | 61 % / 59 % / -0.25 | 54 % / 54 % / -0.22 | 38 % / 38 % / -0.40 | 32 % / 32 % / -0.36 | 30 % / 30 % / -0.26 | 22 % / 22 % / -0.29 |
| h4_trend | 75 % / 46 % / -0.29 | 60 % / 55 % / -0.33 | 49 % / 47 % / -0.37 | 42 % / 42 % / -0.37 | 32 % / 32 % / -0.43 | 29 % / 29 % / -0.36 | 22 % / 22 % / -0.33 |
| cdl_hammer | 79 % / 36 % / -0.34 | 69 % / 58 % / -0.29 | 56 % / 56 % / -0.35 | 50 % / 49 % / -0.34 | 40 % / 40 % / -0.34 | 31 % / 31 % / -0.40 | 22 % / 22 % / -0.46 |
| cdl_inverted_hammer | 82 % / 8 % / -0.50 | 70 % / 40 % / -0.46 | 60 % / 48 % / -0.47 | 52 % / 49 % / -0.49 | 40 % / 40 % / -0.51 | 29 % / 29 % / -0.63 | 21 % / 21 % / -0.68 |
| cdl_bullish_engulfing | 79 % / 41 % / -0.29 | 64 % / 60 % / -0.31 | 54 % / 53 % / -0.33 | 47 % / 47 % / -0.33 | 38 % / 38 % / -0.32 | 30 % / 30 % / -0.36 | 22 % / 22 % / -0.38 |
| cdl_piercing_line | 82 % / 40 % / -0.26 | 72 % / 62 % / -0.20 | 61 % / 60 % / -0.22 | 56 % / 56 % / -0.17 | 44 % / 44 % / -0.19 | 37 % / 37 % / -0.18 | 26 % / 26 % / -0.25 |
| cdl_morning_star | 90 % / 45 % / -0.15 | 75 % / 68 % / -0.15 | 62 % / 60 % / -0.19 | 60 % / 60 % / -0.08 | 52 % / 52 % / +0.03 | 42 % / 42 % / -0.01 | 35 % / 35 % / +0.10 |
| cdl_morning_doji_star | 90 % / 49 % / -0.14 | 75 % / 65 % / -0.14 | 62 % / 60 % / -0.17 | 55 % / 55 % / -0.17 | 40 % / 40 % / -0.27 | 38 % / 38 % / -0.15 | 30 % / 30 % / -0.07 |
| cdl_three_white_soldiers | 50 % / 43 % / -0.56* | 36 % / 36 % / -0.65* | 21 % / 21 % / -0.82* | 14 % / 14 % / -0.91* | 7 % / 7 % / -1.02* | 0 % / 0 % / -1.19* | 0 % / 0 % / -1.19* |
| cdl_tweezer_bottom | 81 % / 28 % / -0.35 | 69 % / 57 % / -0.33 | 56 % / 53 % / -0.39 | 50 % / 48 % / -0.37 | 38 % / 38 % / -0.40 | 31 % / 31 % / -0.43 | 22 % / 22 % / -0.44 |
| cdl_bullish_harami | 84 % / 25 % / -0.33 | 76 % / 58 % / -0.24 | 65 % / 61 % / -0.24 | 55 % / 52 % / -0.29 | 45 % / 44 % / -0.27 | 36 % / 36 % / -0.29 | 24 % / 24 % / -0.39 |
| cdl_bullish_harami_cross | 79 % / 25 % / -0.37 | 66 % / 52 % / -0.37 | 57 % / 55 % / -0.36 | 50 % / 50 % / -0.37 | 41 % / 41 % / -0.35 | 32 % / 32 % / -0.41 | 27 % / 27 % / -0.28 |
| cdl_three_inside_up | 93 % / 67 % / -0.06* | 74 % / 70 % / -0.11* | 63 % / 63 % / -0.12* | 52 % / 52 % / -0.19* | 26 % / 26 % / -0.58* | 26 % / 26 % / -0.45* | 26 % / 26 % / -0.20* |
| cdl_three_outside_up | 79 % / 55 % / -0.23 | 62 % / 60 % / -0.29 | 54 % / 54 % / -0.26 | 48 % / 48 % / -0.26 | 39 % / 39 % / -0.25 | 33 % / 33 % / -0.23 | 25 % / 25 % / -0.22 |
| cdl_rising_three_methods | 100 % / 50 % / +0.05* | 50 % / 50 % / -0.45* | 0 % / 0 % / -1.20* | 0 % / 0 % / -1.20* | 0 % / 0 % / -1.20* | 0 % / 0 % / -1.20* | 0 % / 0 % / -1.20* |
| cdl_bullish_marubozu | 76 % / 28 % / -0.40 | 66 % / 54 % / -0.36 | 57 % / 54 % / -0.35 | 49 % / 49 % / -0.35 | 41 % / 41 % / -0.31 | 35 % / 35 % / -0.27 | 27 % / 27 % / -0.25 |
| cdl_dragonfly_doji | 86 % / 30 % / -0.31 | 72 % / 53 % / -0.31 | 63 % / 63 % / -0.29 | 52 % / 52 % / -0.35 | 41 % / 41 % / -0.37 | 31 % / 31 % / -0.46 | 24 % / 24 % / -0.45 |
| cdl_hanging_man | 79 % / 6 % / -0.58 | 64 % / 33 % / -0.61 | 57 % / 46 % / -0.57 | 50 % / 46 % / -0.56 | 44 % / 43 % / -0.46 | 36 % / 36 % / -0.48 | 29 % / 29 % / -0.38 |
| cdl_shooting_star | 77 % / 29 % / -0.38 | 68 % / 58 % / -0.32 | 55 % / 53 % / -0.37 | 47 % / 46 % / -0.39 | 38 % / 39 % / -0.37 | 30 % / 30 % / -0.42 | 23 % / 23 % / -0.40 |
| cdl_bearish_engulfing | 81 % / 42 % / -0.26 | 68 % / 63 % / -0.26 | 59 % / 58 % / -0.24 | 51 % / 51 % / -0.25 | 41 % / 41 % / -0.22 | 34 % / 34 % / -0.24 | 26 % / 26 % / -0.19 |
| cdl_dark_cloud_cover | 85 % / 43 % / -0.23 | 69 % / 63 % / -0.24 | 58 % / 58 % / -0.26 | 50 % / 50 % / -0.29 | 39 % / 39 % / -0.31 | 31 % / 31 % / -0.33 | 23 % / 23 % / -0.34 |
| cdl_evening_star | 85 % / 38 % / -0.22 | 74 % / 72 % / -0.16 | 69 % / 67 % / -0.07 | 68 % / 68 % / +0.09 | 53 % / 53 % / +0.04 | 39 % / 39 % / -0.09 | 26 % / 26 % / -0.22 |
| cdl_evening_doji_star | 83 % / 33 % / -0.24 | 72 % / 70 % / -0.19 | 64 % / 64 % / -0.14 | 53 % / 53 % / -0.20 | 51 % / 53 % / +0.05 | 49 % / 51 % / +0.24 | 41 % / 43 % / +0.42 |
| cdl_three_black_crows | 86 % / 57 % / -0.29* | 57 % / 43 % / -0.51* | 43 % / 43 % / -0.61* | 43 % / 43 % / -0.51* | 29 % / 29 % / -0.65* | 29 % / 29 % / -0.50* | 29 % / 29 % / -0.22* |
| cdl_tweezer_top | 79 % / 27 % / -0.36 | 66 % / 56 % / -0.36 | 57 % / 55 % / -0.35 | 50 % / 49 % / -0.35 | 40 % / 40 % / -0.34 | 33 % / 33 % / -0.35 | 22 % / 22 % / -0.42 |
| cdl_bearish_harami | 81 % / 23 % / -0.38 | 67 % / 53 % / -0.38 | 58 % / 54 % / -0.37 | 51 % / 50 % / -0.36 | 41 % / 41 % / -0.36 | 32 % / 32 % / -0.41 | 22 % / 22 % / -0.49 |
| cdl_bearish_harami_cross | 86 % / 23 % / -0.31 | 72 % / 54 % / -0.30 | 63 % / 58 % / -0.28 | 55 % / 54 % / -0.28 | 49 % / 49 % / -0.14 | 39 % / 39 % / -0.21 | 26 % / 27 % / -0.30 |
| cdl_three_inside_down | 85 % / 55 % / -0.21 | 76 % / 70 % / -0.13 | 55 % / 55 % / -0.31 | 39 % / 39 % / -0.48 | 30 % / 30 % / -0.51 | 18 % / 18 % / -0.72 | 15 % / 15 % / -0.65 |
| cdl_three_outside_down | 78 % / 48 % / -0.26 | 65 % / 62 % / -0.24 | 55 % / 55 % / -0.27 | 49 % / 49 % / -0.24 | 38 % / 38 % / -0.26 | 32 % / 32 % / -0.26 | 21 % / 21 % / -0.35 |
| cdl_falling_three_methods | - | - | - | - | - | - | - |
| cdl_bearish_marubozu | 77 % / 26 % / -0.40 | 64 % / 54 % / -0.39 | 57 % / 54 % / -0.35 | 49 % / 47 % / -0.36 | 40 % / 40 % / -0.33 | 33 % / 33 % / -0.32 | 26 % / 26 % / -0.28 |
| cdl_gravestone_doji | 75 % / 21 % / -0.42 | 72 % / 61 % / -0.27 | 54 % / 52 % / -0.42 | 47 % / 47 % / -0.41 | 36 % / 36 % / -0.45 | 26 % / 26 % / -0.55 | 21 % / 21 % / -0.50 |

`*` fewer than 30 trades.

**Every variant with win rate >= 80 % and at least 30 trades (7):** (352 more reach 80 % with fewer than 30 trades; all are listed in `reports/win-rate-80.md`.)

| variant | trades | win rate | net 1x R | net 2x R | worst losing streak | largest single loss R | validation (frozen) |
|---|---|---|---|---|---|---|---|
| sd_formed native 1h b025-t0.25 F00 | 30 | 93.3 % | +0.081 | -0.005 | 1 | -1.16 | 16 trades, 81.2 %, -0.061 R |
| ema_stack native 1h b100-t0.25 F00 | 209 | 83.7 % | -0.016 | -0.083 | 3 | -1.22 | 77 trades, 72.7 %, -0.148 R |
| fib_ext native 15m b100-t0.25 F00 | 473 | 81.6 % | -0.049 | -0.146 | 3 | -1.50 | 248 trades, 81.0 %, -0.071 R |
| fib_ext native 1h b025-t0.25 F00 | 127 | 80.3 % | -0.052 | -0.107 | 3 | -1.26 | 81 trades, 77.8 %, -0.067 R |
| cdl_three_outside_up native 1h b100-t0.25 F00 | 100 | 82.0 % | -0.073 | -0.182 | 4 | -1.24 | 58 trades, 69.0 %, -0.217 R |
| cdl_three_outside_down native 1h b100-t0.25 F00 | 120 | 81.7 % | -0.075 | -0.195 | 2 | -1.22 | 57 trades, 70.2 %, -0.190 R |
| sr_break native 1h b100-t0.25 F00 | 752 | 80.5 % | -0.098 | -0.211 | 3 | -1.23 | 390 trades, 82.1 %, -0.070 R |

**Variants whose target is hit in >= 80 % of trades (at least 30 trades): 175.** Positive after 1x costs: 1; after 2x costs: 0. Median net expectancy -0.344 R, median net win rate 30.0 %, median largest single loss -2.33 R. The 15 best by expectancy (all 175 in `reports/win-rate-80.md`):

| variant | trades | target-hit rate | net win rate | net 1x R | net 2x R | worst losing streak | largest single loss R |
|---|---|---|---|---|---|---|---|
| sd_formed native 1h b025-t0.25 F00 | 30 | 93.3 % | 93.3 % | +0.081 | -0.005 | 1 | -1.16 |
| ema_stack native 1h b100-t0.25 F00 | 209 | 83.7 % | 83.7 % | -0.016 | -0.083 | 3 | -1.22 |
| fib_ext native 15m b100-t0.25 F00 | 473 | 83.9 % | 81.6 % | -0.049 | -0.146 | 3 | -1.50 |
| fib_ext native 1h b025-t0.25 F00 | 127 | 80.3 % | 80.3 % | -0.052 | -0.107 | 3 | -1.26 |
| fib_ext native 15m b025-t0.25 F00 | 522 | 83.9 % | 78.7 % | -0.070 | -0.188 | 5 | -1.59 |
| ema_stack native 1h b025-t0.25 F00 | 239 | 81.2 % | 76.1 % | -0.073 | -0.164 | 4 | -1.28 |
| cdl_three_outside_up native 1h b100-t0.25 F00 | 100 | 83.0 % | 82.0 % | -0.073 | -0.182 | 4 | -1.24 |
| cdl_three_outside_down native 1h b100-t0.25 F00 | 120 | 83.3 % | 81.7 % | -0.075 | -0.195 | 2 | -1.22 |
| cdl_piercing_line native 1h b100-t0.25 F00 | 58 | 84.5 % | 75.9 % | -0.086 | -0.228 | 3 | -1.31 |
| cdl_bearish_harami native 1h b100-t0.25 F00 | 48 | 85.4 % | 75.0 % | -0.091 | -0.251 | 2 | -1.18 |
| h4_stack reversed 1h b100-t0.25 F00 | 98 | 86.7 % | 70.4 % | -0.096 | -0.277 | 2 | -1.30 |
| sr_break native 1h b100-t0.25 F00 | 752 | 81.1 % | 80.5 % | -0.098 | -0.211 | 3 | -1.23 |
| sr_formed native 1h b100-t0.25 F00 | 375 | 81.6 % | 78.9 % | -0.104 | -0.227 | 4 | -1.35 |
| sd_test native 1h b100-t0.25 F00 | 41 | 85.4 % | 75.6 % | -0.104 | -0.275 | 2 | -1.20 |
| cdl_hanging_man reversed 1h b100-t0.25 F00 | 49 | 83.7 % | 75.5 % | -0.113 | -0.270 | 2 | -1.45 |


## Appendix I. Files

| file | content |
|---|---|
| `reports/protocol.md` | pre-registered protocol (written before any run) |
| `reports/search-space.md` | pre-registered search space, m = 34,584 |
| `reports/holdout-seal.json` | seal: boundary, holdout data hashes, protocol hashes, time |
| `reports/trial-ledger.csv` | every trial and every later evaluation (one row each) |
| `reports/win-rate-80.md` | every variant with net win rate >= 80 %, and every variant whose target is hit in >= 80 % of trades |
| `reports/research/winrates-discovery.csv` | target-hit, gross and net win rates of the fixed-target discovery trials (descriptive) |
| `reports/research/discovery.json`, `finalists.json`, `validation.json` | stage outputs (frozen lists, all numbers above) |
| `reports/charts/*.svg` | equity and drawdown charts |
| `scripts/gate_report.py` | builds this report from the files above |
