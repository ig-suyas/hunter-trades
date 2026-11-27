# Detector run: um ETHUSDT 5m

Candles: 217440 (2024-11-01 00:00 to 2026-11-25 23:55 UTC, open times). Events: 409671. Detectors: 53.

Throughput (full set + context features, 1 runs): best 286303, median 286303 candles/s.

Prefix invariance on this data (full set + features): cut 210296: PASS, cut 12123: PASS, cut 108835: PASS

Healthy pullbacks (smaller and slower than the impulse, no close beyond the last swing): long 2073 of 8103, short 1753 of 7679.

| HTF | candles with state | EMA ready | bull stack | bear stack | structure up | structure down | mean lag (base candles since the HTF candle completed) |
|---|---:|---:|---:|---:|---:|---:|---:|
| 1h | 217429 | 98.9 % | 20.8 % | 22.1 % | 30.9 % | 28.1 % | 5.50 |
| 4h | 217393 | 95.5 % | 18.7 % | 25.4 % | 32.1 % | 27.7 % | 23.50 |

| group | candles/s alone |
|---|---:|
| swing | 16846542 |
| structure | 14575778 |
| bos_choch | 11934859 |
| sr_zones | 3741264 |
| sd_zones | 11440899 |
| ema | 20047944 |
| trendline | 5527691 |
| fib | 17712466 |
| pullback | 1382042 |
| breakouts | 1910645 |
| sweeps | 5481400 |
| htf_trend | 20272047 |
| candles | 3839986 |
| context features | 1259141 |

| detector | events | per 1000 candles | kinds | flag |
|---|---:|---:|---|---|
| swing | 42983 | 197.7 | swing_high 21635, swing_low 21348 |  |
| structure | 65547 | 301.4 | hh 10235, hl 11376, lh 11399, ll 9971, trend_down 5628, trend_range 11262, trend_up 5676 |  |
| bos | 10678 | 49.1 | bos_bear 5269, bos_bull 5409 |  |
| choch | 4563 | 21.0 | choch_bear 2260, choch_bull 2303 |  |
| sr_zone | 40218 | 185.0 | zone_break_down 14378, zone_break_up 14434, zone_formed 11406 |  |
| role_reversal | 12540 | 57.7 | flip_resistance 6343, flip_support 6197 |  |
| sd_zone | 2878 | 13.2 | demand_broken 392, demand_formed 457, demand_test 585, supply_broken 383, supply_formed 456, supply_test 605 |  |
| ema | 16255 | 74.8 | bear_stack 5059, bull_stack 5359, cross_down_fast_mid 2214, cross_down_mid_slow 704, cross_up_fast_mid 2215, cross_up_mid_slow 704 |  |
| trendline | 45834 | 210.8 | tl_down_break 6589, tl_down_formed 10071, tl_down_touch 6331, tl_up_break 6628, tl_up_formed 9998, tl_up_touch 6217 |  |
| fib | 27338 | 125.7 | fib_ext_1272 2618, fib_ext_1618 1497, fib_leg_down 6141, fib_leg_up 6140, fib_zone_touch 10942 |  |
| pullback | 15782 | 72.6 | pullback_long 8103, pullback_short 7679 |  |
| range | 942 | 4.3 | range_formed 942 |  |
| breakout | 9044 | 41.6 | breakout_down 4719, breakout_up 4325 |  |
| retest | 4013 | 18.5 | retest_long 1863, retest_short 2150 |  |
| fakeout | 11936 | 54.9 | fakeout_down_failed 6006, fakeout_up_failed 5930 |  |
| eq_levels | 3415 | 15.7 | eq_highs 1748, eq_lows 1667 |  |
| sweep | 2446 | 11.2 | sweep_high 1214, sweep_low 1232 |  |
| htf_trend | 4336 | 19.9 | h1_bear_stack 425, h1_bull_stack 370, h1_stack_off 795, h1_trend_down 460, h1_trend_range 942, h1_trend_up 487, h4_bear_stack 108, h4_bull_stack 91, h4_stack_off 199, h4_trend_down 112, h4_trend_range 227, h4_trend_up 120 |  |
| cdl_hammer | 1945 | 8.9 | hammer 1945 |  |
| cdl_inverted_hammer | 1408 | 6.5 | inverted_hammer 1408 |  |
| cdl_bullish_engulfing | 5168 | 23.8 | bullish_engulfing 5168 |  |
| cdl_piercing_line | 1481 | 6.8 | piercing_line 1481 |  |
| cdl_morning_star | 261 | 1.2 | morning_star 261 |  |
| cdl_morning_doji_star | 318 | 1.5 | morning_doji_star 318 |  |
| cdl_three_white_soldiers | 74 | 0.3 | three_white_soldiers 74 |  |
| cdl_tweezer_bottom | 2108 | 9.7 | tweezer_bottom 2108 |  |
| cdl_bullish_harami | 1065 | 4.9 | bullish_harami 1065 |  |
| cdl_bullish_harami_cross | 483 | 2.2 | bullish_harami_cross 483 |  |
| cdl_three_inside_up | 207 | 1.0 | three_inside_up 207 |  |
| cdl_three_outside_up | 2622 | 12.1 | three_outside_up 2622 |  |
| cdl_rising_three_methods | 14 | 0.1 | rising_three_methods 14 | RARE: only 14 events |
| cdl_bullish_marubozu | 2646 | 12.2 | bullish_marubozu 2646 |  |
| cdl_dragonfly_doji | 653 | 3.0 | dragonfly_doji 653 |  |
| cdl_hanging_man | 1588 | 7.3 | hanging_man 1588 |  |
| cdl_shooting_star | 1891 | 8.7 | shooting_star 1891 |  |
| cdl_bearish_engulfing | 5208 | 24.0 | bearish_engulfing 5208 |  |
| cdl_dark_cloud_cover | 1352 | 6.2 | dark_cloud_cover 1352 |  |
| cdl_evening_star | 263 | 1.2 | evening_star 263 |  |
| cdl_evening_doji_star | 298 | 1.4 | evening_doji_star 298 |  |
| cdl_three_black_crows | 64 | 0.3 | three_black_crows 64 |  |
| cdl_tweezer_top | 2377 | 10.9 | tweezer_top 2377 |  |
| cdl_bearish_harami | 1087 | 5.0 | bearish_harami 1087 |  |
| cdl_bearish_harami_cross | 497 | 2.3 | bearish_harami_cross 497 |  |
| cdl_three_inside_down | 197 | 0.9 | three_inside_down 197 |  |
| cdl_three_outside_down | 2657 | 12.2 | three_outside_down 2657 |  |
| cdl_falling_three_methods | 8 | 0.0 | falling_three_methods 8 | RARE: only 8 events |
| cdl_bearish_marubozu | 2372 | 10.9 | bearish_marubozu 2372 |  |
| cdl_gravestone_doji | 653 | 3.0 | gravestone_doji 653 |  |
| cdl_doji | 29765 | 136.9 | doji 29765 |  |
| cdl_long_legged_doji | 1480 | 6.8 | long_legged_doji 1480 |  |
| cdl_spinning_top | 15342 | 70.6 | spinning_top 15342 |  |
| cdl_high_wave | 558 | 2.6 | high_wave 558 |  |
| cdl_rickshaw_man | 813 | 3.7 | rickshaw_man 813 |  |

Flags: ZERO = no events; RARE = fewer than 20 events in the whole history; ABSURD = more than one event per three candles.

Files: `data/events\um\ETHUSDT\5m\events.csv`, `data/events\um\ETHUSDT\5m\features.csv`, `data/events\um\ETHUSDT\5m\gallery.json`.
