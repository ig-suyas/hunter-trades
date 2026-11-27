# Detector run: um ETHUSDT 1h

Candles: 18120 (2024-11-01 00:00 to 2026-11-25 23:00 UTC, open times). Events: 34683. Detectors: 53.

Throughput (full set + context features, 1 runs): best 282368, median 282368 candles/s.

Prefix invariance on this data (full set + features): cut 1736: PASS, cut 4203: PASS, cut 13795: PASS

Healthy pullbacks (smaller and slower than the impulse, no close beyond the last swing): long 153 of 610, short 198 of 631.

| HTF | candles with state | EMA ready | bull stack | bear stack | structure up | structure down | mean lag (base candles since the HTF candle completed) |
|---|---:|---:|---:|---:|---:|---:|---:|
| 1h | 0 | 0.0 % | 0.0 % | 0.0 % | 0.0 % | 0.0 % | -0.00 |
| 4h | 18117 | 95.5 % | 18.7 % | 25.4 % | 32.1 % | 27.7 % | 1.50 |

| group | candles/s alone |
|---|---:|
| swing | 16709701 |
| structure | 10218237 |
| bos_choch | 11664735 |
| sr_zones | 3034515 |
| sd_zones | 7546541 |
| ema | 15361139 |
| trendline | 4157107 |
| fib | 20926204 |
| pullback | 1464763 |
| breakouts | 1832580 |
| sweeps | 6262961 |
| htf_trend | 12607849 |
| candles | 4263830 |
| context features | 1158694 |

| detector | events | per 1000 candles | kinds | flag |
|---|---:|---:|---|---|
| swing | 3653 | 201.6 | swing_high 1828, swing_low 1825 |  |
| structure | 5540 | 305.7 | hh 872, hl 995, lh 955, ll 829, trend_down 460, trend_range 942, trend_up 487 |  |
| bos | 902 | 49.8 | bos_bear 431, bos_bull 471 |  |
| choch | 332 | 18.3 | choch_bear 160, choch_bull 172 |  |
| sr_zone | 3093 | 170.7 | zone_break_down 1081, zone_break_up 1090, zone_formed 922 |  |
| role_reversal | 960 | 53.0 | flip_resistance 466, flip_support 494 |  |
| sd_zone | 208 | 11.5 | demand_broken 29, demand_formed 31, demand_test 48, supply_broken 25, supply_formed 35, supply_test 40 |  |
| ema | 1288 | 71.1 | bear_stack 425, bull_stack 369, cross_down_fast_mid 192, cross_down_mid_slow 56, cross_up_fast_mid 191, cross_up_mid_slow 55 |  |
| trendline | 3767 | 207.9 | tl_down_break 520, tl_down_formed 802, tl_down_touch 472, tl_up_break 546, tl_up_formed 859, tl_up_touch 568 |  |
| fib | 2169 | 119.7 | fib_ext_1272 217, fib_ext_1618 125, fib_leg_down 492, fib_leg_up 491, fib_zone_touch 844 |  |
| pullback | 1241 | 68.5 | pullback_long 610, pullback_short 631 |  |
| range | 76 | 4.2 | range_formed 76 |  |
| breakout | 832 | 45.9 | breakout_down 437, breakout_up 395 |  |
| retest | 368 | 20.3 | retest_long 189, retest_short 179 |  |
| fakeout | 798 | 44.0 | fakeout_down_failed 424, fakeout_up_failed 374 |  |
| eq_levels | 312 | 17.2 | eq_highs 163, eq_lows 149 |  |
| sweep | 610 | 33.7 | sweep_high 300, sweep_low 310 |  |
| htf_trend | 857 | 47.3 | h4_bear_stack 108, h4_bull_stack 91, h4_stack_off 199, h4_trend_down 112, h4_trend_range 227, h4_trend_up 120 |  |
| cdl_hammer | 117 | 6.5 | hammer 117 |  |
| cdl_inverted_hammer | 51 | 2.8 | inverted_hammer 51 |  |
| cdl_bullish_engulfing | 383 | 21.1 | bullish_engulfing 383 |  |
| cdl_piercing_line | 116 | 6.4 | piercing_line 116 |  |
| cdl_morning_star | 19 | 1.0 | morning_star 19 | RARE: only 19 events |
| cdl_morning_doji_star | 14 | 0.8 | morning_doji_star 14 | RARE: only 14 events |
| cdl_three_white_soldiers | 0 | 0.0 |  | ZERO: no events |
| cdl_tweezer_bottom | 151 | 8.3 | tweezer_bottom 151 |  |
| cdl_bullish_harami | 85 | 4.7 | bullish_harami 85 |  |
| cdl_bullish_harami_cross | 40 | 2.2 | bullish_harami_cross 40 |  |
| cdl_three_inside_up | 4 | 0.2 | three_inside_up 4 | RARE: only 4 events |
| cdl_three_outside_up | 195 | 10.8 | three_outside_up 195 |  |
| cdl_rising_three_methods | 2 | 0.1 | rising_three_methods 2 | RARE: only 2 events |
| cdl_bullish_marubozu | 45 | 2.5 | bullish_marubozu 45 |  |
| cdl_dragonfly_doji | 29 | 1.6 | dragonfly_doji 29 |  |
| cdl_hanging_man | 93 | 5.1 | hanging_man 93 |  |
| cdl_shooting_star | 97 | 5.4 | shooting_star 97 |  |
| cdl_bearish_engulfing | 449 | 24.8 | bearish_engulfing 449 |  |
| cdl_dark_cloud_cover | 86 | 4.7 | dark_cloud_cover 86 |  |
| cdl_evening_star | 15 | 0.8 | evening_star 15 | RARE: only 15 events |
| cdl_evening_doji_star | 24 | 1.3 | evening_doji_star 24 |  |
| cdl_three_black_crows | 0 | 0.0 |  | ZERO: no events |
| cdl_tweezer_top | 160 | 8.8 | tweezer_top 160 |  |
| cdl_bearish_harami | 89 | 4.9 | bearish_harami 89 |  |
| cdl_bearish_harami_cross | 41 | 2.3 | bearish_harami_cross 41 |  |
| cdl_three_inside_down | 11 | 0.6 | three_inside_down 11 | RARE: only 11 events |
| cdl_three_outside_down | 239 | 13.2 | three_outside_down 239 |  |
| cdl_falling_three_methods | 1 | 0.1 | falling_three_methods 1 | RARE: only 1 events |
| cdl_bearish_marubozu | 45 | 2.5 | bearish_marubozu 45 |  |
| cdl_gravestone_doji | 29 | 1.6 | gravestone_doji 29 |  |
| cdl_doji | 3017 | 166.5 | doji 3017 |  |
| cdl_long_legged_doji | 148 | 8.2 | long_legged_doji 148 |  |
| cdl_spinning_top | 1715 | 94.6 | spinning_top 1715 |  |
| cdl_high_wave | 84 | 4.6 | high_wave 84 |  |
| cdl_rickshaw_man | 83 | 4.6 | rickshaw_man 83 |  |

Flags: ZERO = no events; RARE = fewer than 20 events in the whole history; ABSURD = more than one event per three candles.

Files: `data/events\um\ETHUSDT\1h\events.csv`, `data/events\um\ETHUSDT\1h\features.csv`, `data/events\um\ETHUSDT\1h\gallery.json`.
