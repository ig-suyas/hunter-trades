# Detector run: um BTCUSDT 5m

Candles: 217440 (2024-11-01 00:00 to 2026-11-25 23:55 UTC, open times). Events: 410863. Detectors: 53.

Throughput (full set + context features, 1 runs): best 304900, median 304900 candles/s.

Prefix invariance on this data (full set + features): cut 210296: PASS, cut 12123: PASS, cut 108835: PASS

Healthy pullbacks (smaller and slower than the impulse, no close beyond the last swing): long 2091 of 8079, short 1748 of 7788.

| HTF | candles with state | EMA ready | bull stack | bear stack | structure up | structure down | mean lag (base candles since the HTF candle completed) |
|---|---:|---:|---:|---:|---:|---:|---:|
| 1h | 217429 | 98.9 % | 22.0 % | 21.9 % | 29.8 % | 28.7 % | 5.50 |
| 4h | 217393 | 95.5 % | 20.0 % | 22.8 % | 30.3 % | 28.8 % | 23.50 |

| group | candles/s alone |
|---|---:|
| swing | 17631748 |
| structure | 16430158 |
| bos_choch | 11522747 |
| sr_zones | 3498283 |
| sd_zones | 10445610 |
| ema | 19716369 |
| trendline | 6148156 |
| fib | 20938093 |
| pullback | 1488064 |
| breakouts | 1990172 |
| sweeps | 5725164 |
| htf_trend | 21483831 |
| candles | 4096790 |
| context features | 1332342 |

| detector | events | per 1000 candles | kinds | flag |
|---|---:|---:|---|---|
| swing | 42169 | 193.9 | swing_high 21270, swing_low 20899 |  |
| structure | 63862 | 293.7 | hh 10303, hl 10817, lh 10966, ll 10081, trend_down 5419, trend_range 10828, trend_up 5448 |  |
| bos | 11055 | 50.8 | bos_bear 5485, bos_bull 5570 |  |
| choch | 4810 | 22.1 | choch_bear 2389, choch_bull 2421 |  |
| sr_zone | 41001 | 188.6 | zone_break_down 14692, zone_break_up 14813, zone_formed 11496 |  |
| role_reversal | 12516 | 57.6 | flip_resistance 6263, flip_support 6253 |  |
| sd_zone | 3908 | 18.0 | demand_broken 571, demand_formed 642, demand_test 705, supply_broken 585, supply_formed 654, supply_test 751 |  |
| ema | 16081 | 74.0 | bear_stack 5002, bull_stack 5230, cross_down_fast_mid 2237, cross_down_mid_slow 687, cross_up_fast_mid 2238, cross_up_mid_slow 687 |  |
| trendline | 43232 | 198.8 | tl_down_break 6325, tl_down_formed 9600, tl_down_touch 5852, tl_up_break 6307, tl_up_formed 9399, tl_up_touch 5749 |  |
| fib | 28079 | 129.1 | fib_ext_1272 2686, fib_ext_1618 1480, fib_leg_down 6357, fib_leg_up 6356, fib_zone_touch 11200 |  |
| pullback | 15867 | 73.0 | pullback_long 8079, pullback_short 7788 |  |
| range | 911 | 4.2 | range_formed 911 |  |
| breakout | 8624 | 39.7 | breakout_down 4404, breakout_up 4220 |  |
| retest | 3768 | 17.3 | retest_long 1794, retest_short 1974 |  |
| fakeout | 12412 | 57.1 | fakeout_down_failed 6243, fakeout_up_failed 6169 |  |
| eq_levels | 3305 | 15.2 | eq_highs 1738, eq_lows 1567 |  |
| sweep | 2432 | 11.2 | sweep_high 1207, sweep_low 1225 |  |
| htf_trend | 4389 | 20.2 | h1_bear_stack 419, h1_bull_stack 389, h1_stack_off 809, h1_trend_down 487, h1_trend_range 959, h1_trend_up 477, h4_bear_stack 94, h4_bull_stack 97, h4_stack_off 191, h4_trend_down 119, h4_trend_range 230, h4_trend_up 118 |  |
| cdl_hammer | 1942 | 8.9 | hammer 1942 |  |
| cdl_inverted_hammer | 1857 | 8.5 | inverted_hammer 1857 |  |
| cdl_bullish_engulfing | 5497 | 25.3 | bullish_engulfing 5497 |  |
| cdl_piercing_line | 1494 | 6.9 | piercing_line 1494 |  |
| cdl_morning_star | 337 | 1.5 | morning_star 337 |  |
| cdl_morning_doji_star | 320 | 1.5 | morning_doji_star 320 |  |
| cdl_three_white_soldiers | 136 | 0.6 | three_white_soldiers 136 |  |
| cdl_tweezer_bottom | 2670 | 12.3 | tweezer_bottom 2670 |  |
| cdl_bullish_harami | 1085 | 5.0 | bullish_harami 1085 |  |
| cdl_bullish_harami_cross | 549 | 2.5 | bullish_harami_cross 549 |  |
| cdl_three_inside_up | 229 | 1.1 | three_inside_up 229 |  |
| cdl_three_outside_up | 2733 | 12.6 | three_outside_up 2733 |  |
| cdl_rising_three_methods | 7 | 0.0 | rising_three_methods 7 | RARE: only 7 events |
| cdl_bullish_marubozu | 4934 | 22.7 | bullish_marubozu 4934 |  |
| cdl_dragonfly_doji | 707 | 3.3 | dragonfly_doji 707 |  |
| cdl_hanging_man | 1913 | 8.8 | hanging_man 1913 |  |
| cdl_shooting_star | 1937 | 8.9 | shooting_star 1937 |  |
| cdl_bearish_engulfing | 5248 | 24.1 | bearish_engulfing 5248 |  |
| cdl_dark_cloud_cover | 1429 | 6.6 | dark_cloud_cover 1429 |  |
| cdl_evening_star | 293 | 1.3 | evening_star 293 |  |
| cdl_evening_doji_star | 338 | 1.6 | evening_doji_star 338 |  |
| cdl_three_black_crows | 137 | 0.6 | three_black_crows 137 |  |
| cdl_tweezer_top | 2837 | 13.0 | tweezer_top 2837 |  |
| cdl_bearish_harami | 1047 | 4.8 | bearish_harami 1047 |  |
| cdl_bearish_harami_cross | 577 | 2.7 | bearish_harami_cross 577 |  |
| cdl_three_inside_down | 219 | 1.0 | three_inside_down 219 |  |
| cdl_three_outside_down | 2652 | 12.2 | three_outside_down 2652 |  |
| cdl_falling_three_methods | 8 | 0.0 | falling_three_methods 8 | RARE: only 8 events |
| cdl_bearish_marubozu | 4799 | 22.1 | bearish_marubozu 4799 |  |
| cdl_gravestone_doji | 710 | 3.3 | gravestone_doji 710 |  |
| cdl_doji | 27896 | 128.3 | doji 27896 |  |
| cdl_long_legged_doji | 1483 | 6.8 | long_legged_doji 1483 |  |
| cdl_spinning_top | 13008 | 59.8 | spinning_top 13008 |  |
| cdl_high_wave | 591 | 2.7 | high_wave 591 |  |
| cdl_rickshaw_man | 823 | 3.8 | rickshaw_man 823 |  |

Flags: ZERO = no events; RARE = fewer than 20 events in the whole history; ABSURD = more than one event per three candles.

Files: `data/events\um\BTCUSDT\5m\events.csv`, `data/events\um\BTCUSDT\5m\features.csv`, `data/events\um\BTCUSDT\5m\gallery.json`.
