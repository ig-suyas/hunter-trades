# Phase 4 search space (pre-registered)

Written 2026-11-27 04:55 UTC together with `reports/protocol.md`, before any Phase 4 run. Nothing may be
added to this list after results are seen; every trial below is run on BTCUSDT DISCOVERY data
(2024-11-01 to 2025-11-01) and written to `reports/trial-ledger.csv`. Total pool: **m = 34,584**.

## Dimensions

**Setups (59).** A setup is a detector, or a group of its event kinds where the kinds are different
trades. Events with `dir = 0` are never traded.

| # | setup | detector | kinds |
|---|---|---|---|
| 1 | swing | swing | swing_high, swing_low |
| 2 | structure_label | structure | hh, hl, lh, ll |
| 3 | structure_trend | structure | trend_up, trend_down |
| 4 | bos | bos | bos_bull, bos_bear |
| 5 | choch | choch | choch_bull, choch_bear |
| 6 | sr_break | sr_zone | zone_break_up, zone_break_down |
| 7 | sr_formed | sr_zone | zone_formed |
| 8 | role_reversal | role_reversal | flip_support, flip_resistance |
| 9 | sd_test | sd_zone | demand_test, supply_test |
| 10 | sd_formed | sd_zone | demand_formed, supply_formed |
| 11 | sd_broken | sd_zone | demand_broken, supply_broken |
| 12 | ema_stack | ema | bull_stack, bear_stack |
| 13 | ema_cross | ema | cross_up_fast_mid, cross_down_fast_mid, cross_up_mid_slow, cross_down_mid_slow |
| 14 | tl_touch | trendline | tl_up_touch, tl_down_touch |
| 15 | tl_break | trendline | tl_up_break, tl_down_break |
| 16 | tl_formed | trendline | tl_up_formed, tl_down_formed |
| 17 | fib_touch | fib | fib_zone_touch |
| 18 | fib_ext | fib | fib_ext_1272, fib_ext_1618 |
| 19 | fib_leg | fib | fib_leg_up, fib_leg_down |
| 20 | pullback | pullback | pullback_long, pullback_short |
| 21 | breakout | breakout | breakout_up, breakout_down |
| 22 | retest | retest | retest_long, retest_short |
| 23 | fakeout | fakeout | fakeout_down_failed, fakeout_up_failed |
| 24 | eq_levels | eq_levels | eq_highs, eq_lows |
| 25 | sweep | sweep | sweep_high, sweep_low |
| 26 | h1_stack | htf_trend | h1_bull_stack, h1_bear_stack (not on the 1h time frame) |
| 27 | h1_trend | htf_trend | h1_trend_up, h1_trend_down (not on the 1h time frame) |
| 28 | h4_stack | htf_trend | h4_bull_stack, h4_bear_stack |
| 29 | h4_trend | htf_trend | h4_trend_up, h4_trend_down |
| 30-44 | bullish candles | cdl_hammer, cdl_inverted_hammer, cdl_bullish_engulfing, cdl_piercing_line, cdl_morning_star, cdl_morning_doji_star, cdl_three_white_soldiers, cdl_tweezer_bottom, cdl_bullish_harami, cdl_bullish_harami_cross, cdl_three_inside_up, cdl_three_outside_up, cdl_rising_three_methods, cdl_bullish_marubozu, cdl_dragonfly_doji | all |
| 45-59 | bearish candles | cdl_hanging_man, cdl_shooting_star, cdl_bearish_engulfing, cdl_dark_cloud_cover, cdl_evening_star, cdl_evening_doji_star, cdl_three_black_crows, cdl_tweezer_top, cdl_bearish_harami, cdl_bearish_harami_cross, cdl_three_inside_down, cdl_three_outside_down, cdl_falling_three_methods, cdl_bearish_marubozu, cdl_gravestone_doji | all |

Rare patterns (fewer than 30 trades) stay in the pool and get p = 1; they still count in `m`.

**Direction (2):** `native` (the detector's direction) and `reversed` (every event's direction flipped).

**Time frame (3):** 5m, 15m, 1h. Excluded combinations: setups 26-27 on 1h (the 1h state is not
defined on a 1h series). Base combinations setup x direction x time frame: 118 + 118 + 114 = **350**.

**Trade rules (10)** = stop x exit. All: next-open entry, structural stop (event zone or pattern
extent, else the signal candle's low/high) plus a buffer, one trade at a time, minimum stop 0.2 ATR.

| rule | stop buffer | exit |
|---|---|---|
| b025-t1, b025-t2, b025-t3 | 0.25 ATR | fixed target 1 R, 2 R, 3 R |
| b025-lvl | 0.25 ATR | nearest S/R or supply/demand zone edge at least 1 R away (skip if none) |
| b025-trail | 0.25 ATR | no target; trailing stop 2 ATR behind the best close once 1 R in profit; time stop 96 candles |
| b100-t1, b100-t2, b100-t3, b100-lvl, b100-trail | 1.0 ATR | the same five exits |

**Filters (10, Phase 3 context filters; all relative to the trade direction, read at the signal candle):**

| id | filter | engine filter |
|---|---|---|
| F00 | none | - |
| F01 | 1h EMA 20/50/200 stack with the trade | `HtfStack { slot: 0, With }` |
| F02 | 4h EMA stack with the trade | `HtfStack { slot: 1, With }` |
| F03 | 1h swing structure with the trade | `HtfStructure { slot: 0, With }` |
| F04 | 4h swing structure with the trade | `HtfStructure { slot: 1, With }` |
| F05 | 1h and 4h EMA stacks both with the trade | F01 + F02 |
| F06 | 4h EMA stack against the trade (counter-trend) | `HtfStack { slot: 1, Against }` |
| F07 | ADX regime trending in the trade direction | `Regime(With)` |
| F08 | ADX regime range | `Regime(Neutral)` |
| F09 | session 07-21 UTC (London, overlap, New York) | `Sessions(0b01110)` |
| F10 | healthy pullback (pullback setup only) | `Healthy(true)` |

F01, F03 and F05 do not exist on the 1h time frame (no 1h state on a 1h series) and are excluded there.
Volatility-percentile and hour filters are not in the search (they appear only as regime diagnostics).

## Families and trial counts

| family | definition | trials |
|---|---|---:|
| main | 350 base x 10 rules x filters F00-F09 (7 filters on 1h): 5m 11,800 + 15m 11,800 + 1h 7,980 | 31,580 |
| healthy | pullback x 2 directions x 3 time frames x 10 rules x F10 | 60 |
| frontier | 350 base x stop buffer {0.25, 1.0} x targets {0.25, 0.5, 0.75, 1.5} R, no filter | 2,800 |
| confluence | setups {sweep, fakeout, retest, role_reversal, pullback, sd_test, fib_touch, choch} x 2 directions x 3 time frames x rule b025-t2 x confluence count >= 1 / >= 2 / >= 3 (definition in `protocol.md` section 9) | 144 |
| **total (FDR pool m)** | | **34,584** |

Each trial is run at 1x and 2x costs (same trades; the 2x run doubles fee, spread and slippage). The
p-value and FDR use net R at 1x costs (`protocol.md` section 4).

## Not in the search (stated so nobody adds it later as a "new idea" on the same data)

Limit entries at the zone, other trailing or time-stop settings, detector parameter variants (all
detectors keep their Phase 2 defaults, which were fixed from definitions and event counts only),
volatility or hour filters, spot prices, other symbols, maker fees (survivors only, as a sensitivity
check), tier 2/3 patterns, and heatmap/liquidation tactics (no history).
