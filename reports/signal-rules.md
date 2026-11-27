# Signal view: the 10 frozen rules

**Status: every rule below FAILED the GATE (0 of 59 setups survived; verdict DO NOT PROCEED,
`reports/gate-report.md`).** They are shown in the overlay only because the user asked for a
view for demo trading with the truth next to every alert. Nothing here is a forecast.

## Where the list comes from

The 10 **exploratory** rules of the Phase 4 GATE report, exactly as frozen in
`reports/research/finalists.json` (`exploratory`, frozen 2026-11-27 05:08:14 UTC), in that file's
order. No rule was tuned, added, removed or re-picked from the trial ledger. That order is the
"fixed order of the pre-registered list" used when several rules fire on one candle; it was set
by the protocol's selection step (best discovery t) and is never re-sorted by any later result.

Checks: `cs-live` unit test `matches_the_frozen_file` (labels, trial ids and order equal
`finalists.json`); `cs-cli` unit test `frozen_rules_equal_research_trials` (setup, detector,
kinds, direction, time frame, filter and the complete engine `TradeRule` equal the research
enumeration's trial, field by field; a changed parameter is caught); `candlescope rule-stats`
refuses to write anything unless its trade counts and net R equal
`reports/research/validation.json` for every rule (BTC discovery, validation and months 1-18,
ETH months 1-18): they do.

## The rules and their frozen parameters

All rules share the Phase 3/4 trade template: decision at the close of the `known_at` candle,
**entry at the next candle's open** (market order), structural stop = the event's zone/pattern
extent (else the signal candle's low/high) plus the buffer x ATR(14), minimum stop 0.2 ATR, one
trade per rule at a time, Phase 3 costs (taker 5 bps per fill, spread 1 bp, slippage 1 bp +
0.02 ATR per market fill, funding), pessimistic fills (stop on touch, target only on a trade
through it, 1-minute candles decide a candle that touches both, stop first otherwise).

| # | trial | label | setup (detector: kinds) | direction | tf | stop buffer | exit | filter | hit counted as |
|---|---|---|---|---|---|---|---|---|---|
| 1 | 28172 | fib_touch native 1h b025-trail F09 | fib: fib_zone_touch | native | 1h | 0.25 ATR | trailing (see below) | F09 session 07-21 UTC | ended above 0 R |
| 2 | 28490 | fib_leg native 1h b025-trail F09 | fib: fib_leg_up, fib_leg_down | native | 1h | 0.25 ATR | trailing | F09 session 07-21 UTC | ended above 0 R |
| 3 | 15499 | ema_cross native 15m b100-t3 F07 | ema: cross_up/down_fast_mid, cross_up/down_mid_slow | native | 15m | 1.0 ATR | fixed 3 R target | F07 ADX regime with the trade | target hit |
| 4 | 28743 | pullback reversed 1h b100-t1 F08 | pullback: pullback_long, pullback_short | reversed | 1h | 1.0 ATR | fixed 1 R target | F08 ADX regime range | target hit |
| 5 | 31648 | cdl_three_outside_up native 1h b025-trail F00 | cdl_three_outside_up: all | native | 1h | 0.25 ATR | trailing | none | ended above 0 R |
| 6 | 28127 | tl_formed reversed 1h b100-trail F07 | trendline: tl_up_formed, tl_down_formed | reversed | 1h | 1.0 ATR | trailing | F07 ADX regime with the trade | ended above 0 R |
| 7 | 16605 | fib_ext native 15m b100-trail F07 | fib: fib_ext_1272, fib_ext_1618 | native | 15m | 1.0 ATR | trailing | F07 ADX regime with the trade | ended above 0 R |
| 8 | 25684 | swing native 1h b100-lvl F07 | swing: swing_high, swing_low | native | 1h | 1.0 ATR | nearest S/R or supply/demand zone boundary at least 1 R away (skip if none) | F07 ADX regime with the trade | target hit |
| 9 | 33442 | cdl_tweezer_top reversed 1h b025-trail F00 | cdl_tweezer_top: all | reversed | 1h | 0.25 ATR | trailing | none | ended above 0 R |
| 10 | 26398 | choch reversed 1h b100-trail F09 | choch: choch_bull, choch_bear | reversed | 1h | 1.0 ATR | trailing | F09 session 07-21 UTC | ended above 0 R |

Trailing exit: no fixed target; once the best close is 1 R in profit the stop trails 2 ATR(14)
behind the best close (never loosens, effective from the next candle); time stop 96 candles
(exit at the open of the 96th candle after entry). The panel shows "Target none · trailing from
<price at +1 R>" and "exit by <time stop> at the latest". Rules 3, 4 and 8 have no time stop: the
panel says "this rule has no time exit". "Hold about X" is the rule's median time in trade over
months 1-18.

"Reversed" flips the direction of every event before simulation (the structural stop then sits
beyond the other side of the pattern), exactly as in the research.

## Time-frame coverage

| time frame | rules | the panel shows |
|---|---|---|
| 5m | none | WAIT "no tested rule for this time frame" (context only, in Details) |
| 15m | 3, 7 | alerts of these two rules |
| 1h | 1, 2, 4, 5, 6, 8, 9, 10 | alerts of these eight rules |
| anything else (1m, 4h, 1D ...) | none, no feed | WAIT "Unsupported time frame: ..." |

The rules were picked on BTCUSDT and checked on ETHUSDT (months 1-18) in the validation stage;
the overlay runs them on both symbols and shows each symbol's own past results.

## Past results shown (months 1-18 = discovery + validation; holdout never read)

`candlescope rule-stats` writes `reports/rule-stats.json` (read by the overlay and the journal)
and `reports/rule-stats.md` (all periods, all intervals). It loads data with the research's sealed
access up to the seal boundary (2026-05-27 00:00 UTC), so the sealed holdout is never loaded,
and no `reports/holdout.lock` exists. Per rule and symbol: trades n, hit rate with an 80 %
Beta-Binomial credible interval (uniform prior), expected net R at 1x and 2x costs with 80 %
circular block-bootstrap intervals, mean cost in R, median and 75th percentile time in trade;
for months 1-18, discovery and validation separately. "Low evidence" below 100 trades.

Summary (trades / hit rate / net R 1x / net R 2x; BTC validation = trades / net R 1x):

| # | label | BTC months 1-18 | BTC validation | ETH months 1-18 |
|---|---|---|---|---|
| 1 | fib_touch native 1h b025-trail F09 | 233 / 37.3 % / +0.269 / +0.010 | 83 / -0.214 | 223 / 28.7 % / +0.191 / -0.006 |
| 2 | fib_leg native 1h b025-trail F09 | 222 / 36.0 % / +0.178 / -0.040 | 79 / -0.321 | 209 / 29.2 % / +0.128 / -0.039 |
| 3 | ema_cross native 15m b100-t3 F07 | 190 / 26.8 % / -0.060 / -0.196 | 84 / -0.417 | 212 / 27.8 % / +0.026 / -0.071 |
| 4 | pullback reversed 1h b100-t1 F08 | 272 / 59.6 % / +0.047 / -0.099 | 92 / -0.049 | 241 / 47.3 % / -0.144 / -0.241 |
| 5 | cdl_three_outside_up native 1h b025-trail F00 | 156 / 44.9 % / +0.074 / -0.079 | 56 / -0.098 | 127 / 41.7 % / -0.073 / -0.186 |
| 6 | tl_formed reversed 1h b100-trail F07 | 201 / 35.8 % / +0.068 / -0.124 | 79 / -0.138 | 212 / 33.0 % / -0.052 / -0.179 |
| 7 | fib_ext native 15m b100-trail F07 | 293 / 46.1 % / -0.056 / -0.145 | 108 / -0.257 | 299 / 46.8 % / -0.046 / -0.103 |
| 8 | swing native 1h b100-lvl F07 | 217 / 47.5 % / +0.067 / -0.054 | 85 / +0.014 | 234 / 40.6 % / -0.080 / -0.164 |
| 9 | cdl_tweezer_top reversed 1h b025-trail F00 | 143 / 22.4 % / +0.105 / -0.426 | 43 / -0.568 | 110 / 15.5 % / -0.530 / -0.857 |
| 10 | choch reversed 1h b100-trail F09 | 183 / 38.3 % / +0.125 / -0.097 | 62 / +0.046 | 139 / 33.1 % / -0.001 / -0.130 |

**Read the BTC months 1-18 column with care:** it contains the discovery year on which these
rules were picked, so it flatters them. After picking (validation) 8 of 10 lost money at 1x costs
and all 10 at 2x; at 2x costs months 1-18 is negative for 9 of 10 on BTC and 10 of 10 on ETH.
Every months 1-18 sample has at least 110 trades, so the main view's "low evidence" flag (below
100) does not appear with these numbers; in Details it marks 17 of the 20 validation rows
(BTC 43-92 trades for 9 rules; ETH 39-86 for 8 rules).

## How an alert is decided (one code path)

`cs-live` (`signal::engine`) keeps the live engine's candles, context features and detector
events (bit-identical to `Market::prepare`, proven by the parity tests) and, when the first
in-progress frame of the next candle gives its open, runs the backtest's own
`cs_backtest::sim::simulate` for each rule on that market plus one provisional candle holding
only the real open. The decision at candle k uses candles up to k and the open of k+1, as the
backtest's next-open entry does; the provisional candle is removed again. Support/resistance
levels for rule 8 come from `Market::prepare` itself. Alerts are append-only and never changed;
an alert decided only after its entry candle closed (feed gap, start in mid-candle) is "late" and
never shown. Several rules on one candle: the first in the list above is shown with "+N more";
up and down together: WAIT "rules disagree".

**Parity (one week before the holdout, BTCUSDT 2026-03-02 .. 03-09, replayed as websocket
frames):** 15m 8 alerts = 8 backtest trades and = 8 research trades, bit for bit; 1h 26 alerts =
26 research trades (research market, 1-minute data) bit for bit, and = 26 of 27 trades of a
backtest without 1-minute data. **The one difference is a timing quirk of the backtest engine,
found here:** its one-trade-at-a-time check compares the previous trade's exit *time stamp*
with the decision time; an exit inside the next candle is stamped at that candle's open (no
1-minute data) or at its 1-minute candle's open (with them), so when it happens in the first
minute the backtest lets the new trade through using price action after the decision. The live
path cannot know that and skips such a trade. Over all months 1-18 of these 10 rules this
affected 1 of 2,110 research trades (0 of 483 on 15m, 1 of 1,627 on 1h). The engine was not
edited (task rule); the GATE verdict does not depend on it.

## Panel rules and logging

Daily limits (editable in Details, reset 00:00 UTC): at most 3 shown alerts per day, pause after
3 losses in a row, pause after a daily loss of 3 R (defaults); one shown alert at a time. Every
alert is logged to `data/signals/signals.csv` with the reason it was or was not shown and is
resolved afterwards with the same rules (`simulate` on the same candles from the logged window
start, with 1-minute candles and funding from the exchange's public endpoints). Demo fills are
imported from `data/signals/fills.csv` into `demo-trades.csv`; `candlescope journal-report`
evaluates them (PASS only with >= 100 trades, net R > 0 and the 80 % block-bootstrap lower bound
> 0; 50-99 trades: "checkpoint, not a verdict").
