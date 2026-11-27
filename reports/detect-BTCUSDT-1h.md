# Detector run: um BTCUSDT 1h

Candles: 18120 (2024-11-01 00:00 to 2026-11-25 23:00 UTC, open times). Events: 35053. Detectors: 53.

Throughput (full set + context features, 1 runs): best 287919, median 287919 candles/s.

Prefix invariance on this data (full set + features): cut 1736: PASS, cut 4203: PASS, cut 13795: PASS

Healthy pullbacks (smaller and slower than the impulse, no close beyond the last swing): long 191 of 644, short 181 of 650.

| HTF | candles with state | EMA ready | bull stack | bear stack | structure up | structure down | mean lag (base candles since the HTF candle completed) |
|---|---:|---:|---:|---:|---:|---:|---:|
| 1h | 0 | 0.0 % | 0.0 % | 0.0 % | 0.0 % | 0.0 % | -0.00 |
| 4h | 18117 | 95.5 % | 20.0 % | 22.8 % | 30.3 % | 28.8 % | 1.50 |

| group | candles/s alone |
|---|---:|
| swing | 18158132 |
| structure | 16251121 |
| bos_choch | 14591722 |
| sr_zones | 2844182 |
| sd_zones | 10075063 |
| ema | 15966164 |
| trendline | 5580708 |
| fib | 20525600 |
| pullback | 1363760 |
| breakouts | 1730312 |
| sweeps | 7116487 |
| htf_trend | 19788140 |
| candles | 4113041 |
| context features | 1247230 |

| detector | events | per 1000 candles | kinds | flag |
|---|---:|---:|---|---|
| swing | 3648 | 201.3 | swing_high 1831, swing_low 1817 |  |
| structure | 5569 | 307.3 | hh 876, hl 966, lh 954, ll 850, trend_down 487, trend_range 959, trend_up 477 |  |
| bos | 875 | 48.3 | bos_bear 431, bos_bull 444 |  |
| choch | 401 | 22.1 | choch_bear 198, choch_bull 203 |  |
| sr_zone | 3336 | 184.1 | zone_break_down 1201, zone_break_up 1180, zone_formed 955 |  |
| role_reversal | 1030 | 56.8 | flip_resistance 537, flip_support 493 |  |
| sd_zone | 220 | 12.1 | demand_broken 25, demand_formed 29, demand_test 50, supply_broken 33, supply_formed 39, supply_test 44 |  |
| ema | 1258 | 69.4 | bear_stack 419, bull_stack 389, cross_down_fast_mid 171, cross_down_mid_slow 55, cross_up_fast_mid 170, cross_up_mid_slow 54 |  |
| trendline | 3631 | 200.4 | tl_down_break 526, tl_down_formed 792, tl_down_touch 471, tl_up_break 547, tl_up_formed 813, tl_up_touch 482 |  |
| fib | 2195 | 121.1 | fib_ext_1272 228, fib_ext_1618 124, fib_leg_down 501, fib_leg_up 500, fib_zone_touch 842 |  |
| pullback | 1294 | 71.4 | pullback_long 644, pullback_short 650 |  |
| range | 74 | 4.1 | range_formed 74 |  |
| breakout | 887 | 49.0 | breakout_down 456, breakout_up 431 |  |
| retest | 442 | 24.4 | retest_long 214, retest_short 228 |  |
| fakeout | 903 | 49.8 | fakeout_down_failed 474, fakeout_up_failed 429 |  |
| eq_levels | 306 | 16.9 | eq_highs 162, eq_lows 144 |  |
| sweep | 621 | 34.3 | sweep_high 297, sweep_low 324 |  |
| htf_trend | 849 | 46.9 | h4_bear_stack 94, h4_bull_stack 97, h4_stack_off 191, h4_trend_down 119, h4_trend_range 230, h4_trend_up 118 |  |
| cdl_hammer | 101 | 5.6 | hammer 101 |  |
| cdl_inverted_hammer | 61 | 3.4 | inverted_hammer 61 |  |
| cdl_bullish_engulfing | 442 | 24.4 | bullish_engulfing 442 |  |
| cdl_piercing_line | 110 | 6.1 | piercing_line 110 |  |
| cdl_morning_star | 26 | 1.4 | morning_star 26 |  |
| cdl_morning_doji_star | 19 | 1.0 | morning_doji_star 19 | RARE: only 19 events |
| cdl_three_white_soldiers | 4 | 0.2 | three_white_soldiers 4 | RARE: only 4 events |
| cdl_tweezer_bottom | 170 | 9.4 | tweezer_bottom 170 |  |
| cdl_bullish_harami | 73 | 4.0 | bullish_harami 73 |  |
| cdl_bullish_harami_cross | 33 | 1.8 | bullish_harami_cross 33 |  |
| cdl_three_inside_up | 6 | 0.3 | three_inside_up 6 | RARE: only 6 events |
| cdl_three_outside_up | 225 | 12.4 | three_outside_up 225 |  |
| cdl_rising_three_methods | 0 | 0.0 |  | ZERO: no events |
| cdl_bullish_marubozu | 88 | 4.9 | bullish_marubozu 88 |  |
| cdl_dragonfly_doji | 36 | 2.0 | dragonfly_doji 36 |  |
| cdl_hanging_man | 89 | 4.9 | hanging_man 89 |  |
| cdl_shooting_star | 109 | 6.0 | shooting_star 109 |  |
| cdl_bearish_engulfing | 452 | 24.9 | bearish_engulfing 452 |  |
| cdl_dark_cloud_cover | 100 | 5.5 | dark_cloud_cover 100 |  |
| cdl_evening_star | 20 | 1.1 | evening_star 20 |  |
| cdl_evening_doji_star | 16 | 0.9 | evening_doji_star 16 | RARE: only 16 events |
| cdl_three_black_crows | 1 | 0.1 | three_black_crows 1 | RARE: only 1 events |
| cdl_tweezer_top | 211 | 11.6 | tweezer_top 211 |  |
| cdl_bearish_harami | 82 | 4.5 | bearish_harami 82 |  |
| cdl_bearish_harami_cross | 36 | 2.0 | bearish_harami_cross 36 |  |
| cdl_three_inside_down | 15 | 0.8 | three_inside_down 15 | RARE: only 15 events |
| cdl_three_outside_down | 238 | 13.1 | three_outside_down 238 |  |
| cdl_falling_three_methods | 1 | 0.1 | falling_three_methods 1 | RARE: only 1 events |
| cdl_bearish_marubozu | 95 | 5.2 | bearish_marubozu 95 |  |
| cdl_gravestone_doji | 33 | 1.8 | gravestone_doji 33 |  |
| cdl_doji | 2794 | 154.2 | doji 2794 |  |
| cdl_long_legged_doji | 148 | 8.2 | long_legged_doji 148 |  |
| cdl_spinning_top | 1519 | 83.8 | spinning_top 1519 |  |
| cdl_high_wave | 75 | 4.1 | high_wave 75 |  |
| cdl_rickshaw_man | 86 | 4.7 | rickshaw_man 86 |  |

Flags: ZERO = no events; RARE = fewer than 20 events in the whole history; ABSURD = more than one event per three candles.

Files: `data/events\um\BTCUSDT\1h\events.csv`, `data/events\um\BTCUSDT\1h\features.csv`, `data/events\um\BTCUSDT\1h\gallery.json`.
