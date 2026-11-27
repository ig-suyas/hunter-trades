# Detector run: um ETHUSDT 15m

Candles: 72480 (2024-11-01 00:00 to 2026-11-25 23:45 UTC, open times). Events: 139026. Detectors: 53.

Throughput (full set + context features, 1 runs): best 276290, median 276290 candles/s.

Prefix invariance on this data (full set + features): cut 51416: PASS, cut 603: PASS, cut 14335: PASS

Healthy pullbacks (smaller and slower than the impulse, no close beyond the last swing): long 710 of 2653, short 653 of 2551.

| HTF | candles with state | EMA ready | bull stack | bear stack | structure up | structure down | mean lag (base candles since the HTF candle completed) |
|---|---:|---:|---:|---:|---:|---:|---:|
| 1h | 72477 | 98.9 % | 20.8 % | 22.1 % | 30.9 % | 28.1 % | 1.50 |
| 4h | 72465 | 95.5 % | 18.7 % | 25.4 % | 32.1 % | 27.7 % | 7.50 |

| group | candles/s alone |
|---|---:|
| swing | 9751766 |
| structure | 13894640 |
| bos_choch | 11614082 |
| sr_zones | 3046300 |
| sd_zones | 10776253 |
| ema | 22148882 |
| trendline | 4570131 |
| fib | 14737099 |
| pullback | 1252588 |
| breakouts | 1801412 |
| sweeps | 4273837 |
| htf_trend | 13295179 |
| candles | 3994951 |
| context features | 1038571 |

| detector | events | per 1000 candles | kinds | flag |
|---|---:|---:|---|---|
| swing | 14567 | 201.0 | swing_high 7289, swing_low 7278 |  |
| structure | 22239 | 306.8 | hh 3474, hl 3955, lh 3814, ll 3322, trend_down 1897, trend_range 3829, trend_up 1948 |  |
| bos | 3446 | 47.5 | bos_bear 1661, bos_bull 1785 |  |
| choch | 1534 | 21.2 | choch_bear 797, choch_bull 737 |  |
| sr_zone | 13071 | 180.3 | zone_break_down 4625, zone_break_up 4680, zone_formed 3766 |  |
| role_reversal | 4054 | 55.9 | flip_resistance 1994, flip_support 2060 |  |
| sd_zone | 658 | 9.1 | demand_broken 89, demand_formed 109, demand_test 154, supply_broken 77, supply_formed 97, supply_test 132 |  |
| ema | 5320 | 73.4 | bear_stack 1713, bull_stack 1700, cross_down_fast_mid 725, cross_down_mid_slow 228, cross_up_fast_mid 726, cross_up_mid_slow 228 |  |
| trendline | 15306 | 211.2 | tl_down_break 2157, tl_down_formed 3304, tl_down_touch 1995, tl_up_break 2273, tl_up_formed 3457, tl_up_touch 2120 |  |
| fib | 8796 | 121.4 | fib_ext_1272 901, fib_ext_1618 508, fib_leg_down 1970, fib_leg_up 1970, fib_zone_touch 3447 |  |
| pullback | 5204 | 71.8 | pullback_long 2653, pullback_short 2551 |  |
| range | 359 | 5.0 | range_formed 359 |  |
| breakout | 3392 | 46.8 | breakout_down 1803, breakout_up 1589 |  |
| retest | 1497 | 20.7 | retest_long 694, retest_short 803 |  |
| fakeout | 3757 | 51.8 | fakeout_down_failed 1846, fakeout_up_failed 1911 |  |
| eq_levels | 1197 | 16.5 | eq_highs 629, eq_lows 568 |  |
| sweep | 1151 | 15.9 | sweep_high 584, sweep_low 567 |  |
| htf_trend | 4336 | 59.8 | h1_bear_stack 425, h1_bull_stack 370, h1_stack_off 795, h1_trend_down 460, h1_trend_range 942, h1_trend_up 487, h4_bear_stack 108, h4_bull_stack 91, h4_stack_off 199, h4_trend_down 112, h4_trend_range 227, h4_trend_up 120 |  |
| cdl_hammer | 488 | 6.7 | hammer 488 |  |
| cdl_inverted_hammer | 315 | 4.3 | inverted_hammer 315 |  |
| cdl_bullish_engulfing | 1624 | 22.4 | bullish_engulfing 1624 |  |
| cdl_piercing_line | 453 | 6.2 | piercing_line 453 |  |
| cdl_morning_star | 69 | 1.0 | morning_star 69 |  |
| cdl_morning_doji_star | 78 | 1.1 | morning_doji_star 78 |  |
| cdl_three_white_soldiers | 7 | 0.1 | three_white_soldiers 7 | RARE: only 7 events |
| cdl_tweezer_bottom | 666 | 9.2 | tweezer_bottom 666 |  |
| cdl_bullish_harami | 358 | 4.9 | bullish_harami 358 |  |
| cdl_bullish_harami_cross | 153 | 2.1 | bullish_harami_cross 153 |  |
| cdl_three_inside_up | 53 | 0.7 | three_inside_up 53 |  |
| cdl_three_outside_up | 828 | 11.4 | three_outside_up 828 |  |
| cdl_rising_three_methods | 1 | 0.0 | rising_three_methods 1 | RARE: only 1 events |
| cdl_bullish_marubozu | 422 | 5.8 | bullish_marubozu 422 |  |
| cdl_dragonfly_doji | 148 | 2.0 | dragonfly_doji 148 |  |
| cdl_hanging_man | 362 | 5.0 | hanging_man 362 |  |
| cdl_shooting_star | 507 | 7.0 | shooting_star 507 |  |
| cdl_bearish_engulfing | 1754 | 24.2 | bearish_engulfing 1754 |  |
| cdl_dark_cloud_cover | 417 | 5.8 | dark_cloud_cover 417 |  |
| cdl_evening_star | 71 | 1.0 | evening_star 71 |  |
| cdl_evening_doji_star | 89 | 1.2 | evening_doji_star 89 |  |
| cdl_three_black_crows | 11 | 0.2 | three_black_crows 11 | RARE: only 11 events |
| cdl_tweezer_top | 782 | 10.8 | tweezer_top 782 |  |
| cdl_bearish_harami | 336 | 4.6 | bearish_harami 336 |  |
| cdl_bearish_harami_cross | 156 | 2.2 | bearish_harami_cross 156 |  |
| cdl_three_inside_down | 58 | 0.8 | three_inside_down 58 |  |
| cdl_three_outside_down | 890 | 12.3 | three_outside_down 890 |  |
| cdl_falling_three_methods | 4 | 0.1 | falling_three_methods 4 | RARE: only 4 events |
| cdl_bearish_marubozu | 373 | 5.1 | bearish_marubozu 373 |  |
| cdl_gravestone_doji | 165 | 2.3 | gravestone_doji 165 |  |
| cdl_doji | 10311 | 142.3 | doji 10311 |  |
| cdl_long_legged_doji | 536 | 7.4 | long_legged_doji 536 |  |
| cdl_spinning_top | 6103 | 84.2 | spinning_top 6103 |  |
| cdl_high_wave | 237 | 3.3 | high_wave 237 |  |
| cdl_rickshaw_man | 317 | 4.4 | rickshaw_man 317 |  |

Flags: ZERO = no events; RARE = fewer than 20 events in the whole history; ABSURD = more than one event per three candles.

Files: `data/events\um\ETHUSDT\15m\events.csv`, `data/events\um\ETHUSDT\15m\features.csv`, `data/events\um\ETHUSDT\15m\gallery.json`.
