# Detector run: um BTCUSDT 15m

Candles: 72480 (2024-11-01 00:00 to 2026-11-25 23:45 UTC, open times). Events: 138430. Detectors: 53.

Throughput (full set + context features, 3 runs): best 318163, median 309462 candles/s.

Prefix invariance on this data (full set + features): cut 51416: PASS, cut 603: PASS, cut 14335: PASS

Healthy pullbacks (smaller and slower than the impulse, no close beyond the last swing): long 687 of 2639, short 662 of 2553.

| HTF | candles with state | EMA ready | bull stack | bear stack | structure up | structure down | mean lag (base candles since the HTF candle completed) |
|---|---:|---:|---:|---:|---:|---:|---:|
| 1h | 72477 | 98.9 % | 22.0 % | 21.9 % | 29.8 % | 28.7 % | 1.50 |
| 4h | 72465 | 95.5 % | 20.0 % | 22.8 % | 30.3 % | 28.8 % | 7.50 |

| group | candles/s alone |
|---|---:|
| swing | 16737484 |
| structure | 13956713 |
| bos_choch | 13049584 |
| sr_zones | 3854766 |
| sd_zones | 10775292 |
| ema | 22026378 |
| trendline | 6238703 |
| fib | 20547712 |
| pullback | 1593272 |
| breakouts | 2122097 |
| sweeps | 7680814 |
| htf_trend | 16052801 |
| candles | 4376600 |
| context features | 1349587 |

| detector | events | per 1000 candles | kinds | flag |
|---|---:|---:|---|---|
| swing | 14185 | 195.7 | swing_high 7113, swing_low 7072 |  |
| structure | 21568 | 297.6 | hh 3433, hl 3775, lh 3679, ll 3296, trend_down 1833, trend_range 3686, trend_up 1866 |  |
| bos | 3567 | 49.2 | bos_bear 1745, bos_bull 1822 |  |
| choch | 1527 | 21.1 | choch_bear 772, choch_bull 755 |  |
| sr_zone | 13318 | 183.7 | zone_break_down 4789, zone_break_up 4755, zone_formed 3774 |  |
| role_reversal | 4151 | 57.3 | flip_resistance 2105, flip_support 2046 |  |
| sd_zone | 858 | 11.8 | demand_broken 112, demand_formed 130, demand_test 166, supply_broken 131, supply_formed 151, supply_test 168 |  |
| ema | 5208 | 71.9 | bear_stack 1663, bull_stack 1684, cross_down_fast_mid 714, cross_down_mid_slow 216, cross_up_fast_mid 715, cross_up_mid_slow 216 |  |
| trendline | 14778 | 203.9 | tl_down_break 2111, tl_down_formed 3221, tl_down_touch 2041, tl_up_break 2190, tl_up_formed 3278, tl_up_touch 1937 |  |
| fib | 8962 | 123.6 | fib_ext_1272 890, fib_ext_1618 505, fib_leg_down 2023, fib_leg_up 2023, fib_zone_touch 3521 |  |
| pullback | 5192 | 71.6 | pullback_long 2639, pullback_short 2553 |  |
| range | 357 | 4.9 | range_formed 357 |  |
| breakout | 3271 | 45.1 | breakout_down 1731, breakout_up 1540 |  |
| retest | 1483 | 20.5 | retest_long 687, retest_short 796 |  |
| fakeout | 3948 | 54.5 | fakeout_down_failed 1943, fakeout_up_failed 2005 |  |
| eq_levels | 1107 | 15.3 | eq_highs 571, eq_lows 536 |  |
| sweep | 1071 | 14.8 | sweep_high 528, sweep_low 543 |  |
| htf_trend | 4389 | 60.6 | h1_bear_stack 419, h1_bull_stack 389, h1_stack_off 809, h1_trend_down 487, h1_trend_range 959, h1_trend_up 477, h4_bear_stack 94, h4_bull_stack 97, h4_stack_off 191, h4_trend_down 119, h4_trend_range 230, h4_trend_up 118 |  |
| cdl_hammer | 468 | 6.5 | hammer 468 |  |
| cdl_inverted_hammer | 428 | 5.9 | inverted_hammer 428 |  |
| cdl_bullish_engulfing | 1770 | 24.4 | bullish_engulfing 1770 |  |
| cdl_piercing_line | 475 | 6.6 | piercing_line 475 |  |
| cdl_morning_star | 83 | 1.1 | morning_star 83 |  |
| cdl_morning_doji_star | 86 | 1.2 | morning_doji_star 86 |  |
| cdl_three_white_soldiers | 23 | 0.3 | three_white_soldiers 23 |  |
| cdl_tweezer_bottom | 856 | 11.8 | tweezer_bottom 856 |  |
| cdl_bullish_harami | 323 | 4.5 | bullish_harami 323 |  |
| cdl_bullish_harami_cross | 158 | 2.2 | bullish_harami_cross 158 |  |
| cdl_three_inside_up | 50 | 0.7 | three_inside_up 50 |  |
| cdl_three_outside_up | 891 | 12.3 | three_outside_up 891 |  |
| cdl_rising_three_methods | 4 | 0.1 | rising_three_methods 4 | RARE: only 4 events |
| cdl_bullish_marubozu | 787 | 10.9 | bullish_marubozu 787 |  |
| cdl_dragonfly_doji | 150 | 2.1 | dragonfly_doji 150 |  |
| cdl_hanging_man | 446 | 6.2 | hanging_man 446 |  |
| cdl_shooting_star | 510 | 7.0 | shooting_star 510 |  |
| cdl_bearish_engulfing | 1741 | 24.0 | bearish_engulfing 1741 |  |
| cdl_dark_cloud_cover | 428 | 5.9 | dark_cloud_cover 428 |  |
| cdl_evening_star | 78 | 1.1 | evening_star 78 |  |
| cdl_evening_doji_star | 95 | 1.3 | evening_doji_star 95 |  |
| cdl_three_black_crows | 13 | 0.2 | three_black_crows 13 | RARE: only 13 events |
| cdl_tweezer_top | 932 | 12.9 | tweezer_top 932 |  |
| cdl_bearish_harami | 375 | 5.2 | bearish_harami 375 |  |
| cdl_bearish_harami_cross | 165 | 2.3 | bearish_harami_cross 165 |  |
| cdl_three_inside_down | 71 | 1.0 | three_inside_down 71 |  |
| cdl_three_outside_down | 848 | 11.7 | three_outside_down 848 |  |
| cdl_falling_three_methods | 1 | 0.0 | falling_three_methods 1 | RARE: only 1 events |
| cdl_bearish_marubozu | 767 | 10.6 | bearish_marubozu 767 |  |
| cdl_gravestone_doji | 154 | 2.1 | gravestone_doji 154 |  |
| cdl_doji | 9623 | 132.8 | doji 9623 |  |
| cdl_long_legged_doji | 556 | 7.7 | long_legged_doji 556 |  |
| cdl_spinning_top | 5574 | 76.9 | spinning_top 5574 |  |
| cdl_high_wave | 250 | 3.4 | high_wave 250 |  |
| cdl_rickshaw_man | 311 | 4.3 | rickshaw_man 311 |  |

Flags: ZERO = no events; RARE = fewer than 20 events in the whole history; ABSURD = more than one event per three candles.

Files: `data/events\um\BTCUSDT\15m\events.csv`, `data/events\um\BTCUSDT\15m\features.csv`, `data/events\um\BTCUSDT\15m\gallery.json`.
