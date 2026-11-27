# Variants with win rate >= 80 % (discovery, BTCUSDT, net of 1x costs)

## Net win rate >= 80 % (protocol definition: net R > 0 after costs)

359 variants reach a win rate of at least 80 %; 7 of them have at least 30 trades. A win is a trade with net R > 0 after costs. Sorted by number of trades. Validation results (frozen) exist for the variants with at least 30 trades.

| variant | family | trades | win rate | net 1x R | net 2x R | worst losing streak | largest single loss R | validation trades / win rate / net |
|---|---|---|---|---|---|---|---|---|
| sr_break native 1h b100-t0.25 F00 | frontier | 752 | 80.5 % | -0.098 | -0.211 | 3 | -1.23 | 390 / 82.1 % / -0.070 |
| fib_ext native 15m b100-t0.25 F00 | frontier | 473 | 81.6 % | -0.049 | -0.146 | 3 | -1.50 | 248 / 81.0 % / -0.071 |
| ema_stack native 1h b100-t0.25 F00 | frontier | 209 | 83.7 % | -0.016 | -0.083 | 3 | -1.22 | 77 / 72.7 % / -0.148 |
| fib_ext native 1h b025-t0.25 F00 | frontier | 127 | 80.3 % | -0.052 | -0.107 | 3 | -1.26 | 81 / 77.8 % / -0.067 |
| cdl_three_outside_down native 1h b100-t0.25 F00 | frontier | 120 | 81.7 % | -0.075 | -0.195 | 2 | -1.22 | 57 / 70.2 % / -0.190 |
| cdl_three_outside_up native 1h b100-t0.25 F00 | frontier | 100 | 82.0 % | -0.073 | -0.182 | 4 | -1.24 | 58 / 69.0 % / -0.217 |
| sd_formed native 1h b025-t0.25 F00 | frontier | 30 | 93.3 % | +0.081 | -0.005 | 1 | -1.16 | 16 / 81.2 % / -0.061 |
| sd_formed native 1h b100-t0.25 F00 | frontier | 29 | 93.1 % | +0.095 | +0.027 | 1 | -1.13 | - |
| cdl_dragonfly_doji reversed 1h b100-t0.5 F00 | frontier | 18 | 83.3 % | +0.017 | -0.218 | 1 | -1.25 | - |
| cdl_bearish_harami_cross native 1h b100-t0.5 F00 | frontier | 17 | 82.3 % | +0.009 | -0.219 | 1 | -1.34 | - |
| cdl_bearish_harami native 1h b100-t1 F08 | main | 15 | 86.7 % | +0.560 | +0.384 | 1 | -1.18 | - |
| cdl_bearish_harami native 1h b100-t1 F06 | main | 15 | 80.0 % | +0.458 | +0.306 | 2 | -1.18 | - |
| cdl_bullish_harami_cross native 1h b100-t0.75 F00 | frontier | 13 | 92.3 % | +0.437 | +0.263 | 1 | -1.20 | - |
| cdl_bullish_harami_cross native 1h b100-t0.5 F00 | frontier | 13 | 100.0 % | +0.324 | +0.150 | 0 | 0.15 | - |
| cdl_bullish_harami_cross native 1h b100-t0.25 F00 | frontier | 13 | 92.3 % | +0.074 | -0.100 | 1 | -0.10 | - |
| cdl_evening_star native 1h b100-t0.25 F00 | frontier | 13 | 84.6 % | -0.071 | -0.202 | 1 | -1.19 | - |
| cdl_bearish_marubozu reversed 1h b100-t1 F02 | main | 12 | 83.3 % | +0.325 | -0.006 | 1 | -1.39 | - |
| cdl_morning_star native 15m b100-t1 F08 | main | 12 | 83.3 % | +0.253 | -0.158 | 1 | -1.53 | - |
| cdl_bearish_harami_cross native 15m b100-t1 F02 | main | 11 | 81.8 % | +0.253 | -0.135 | 1 | -1.95 | - |
| cdl_piercing_line reversed 1h b025-t1 F06 | main | 7 | 85.7 % | +0.277 | -0.161 | 1 | -1.29 | - |
| cdl_bearish_harami native 15m b025-t1 F05 | main | 7 | 85.7 % | +0.148 | -0.418 | 1 | -1.35 | - |
| h4_stack reversed 1h b100-t3 F07 | main | 6 | 83.3 % | +2.166 | +1.980 | 1 | -1.24 | - |
| cdl_three_white_soldiers reversed 15m b100-t3 F03 | main | 6 | 83.3 % | +1.788 | +1.241 | 1 | -1.50 | - |
| cdl_three_white_soldiers reversed 15m b100-t3 F08 | main | 6 | 83.3 % | +1.702 | +1.069 | 1 | -1.79 | - |
| cdl_morning_doji_star native 1h b100-t2 F09 | main | 6 | 83.3 % | +1.355 | +1.224 | 1 | -1.17 | - |
| cdl_bearish_harami_cross reversed 1h b100-t2 F04 | main | 6 | 83.3 % | +1.331 | +1.183 | 1 | -1.12 | - |
| h4_stack reversed 1h b100-t2 F07 | main | 6 | 83.3 % | +1.330 | +1.144 | 1 | -1.24 | - |
| sd_formed native 1h b025-lvl F07 | main | 6 | 83.3 % | +1.135 | +1.039 | 1 | -1.08 | - |
| cdl_morning_doji_star native 1h b100-lvl F09 | main | 6 | 83.3 % | +1.048 | +0.917 | 1 | -1.17 | - |
| cdl_three_white_soldiers reversed 15m b100-t2 F03 | main | 6 | 83.3 % | +0.952 | +0.404 | 1 | -1.50 | - |
| sd_formed native 1h b100-lvl F07 | main | 6 | 83.3 % | +0.906 | +0.828 | 1 | -1.07 | - |
| cdl_three_white_soldiers reversed 15m b100-t2 F08 | main | 6 | 83.3 % | +0.867 | +0.233 | 1 | -1.79 | - |
| cdl_bullish_harami reversed 1h b100-lvl F08 | main | 6 | 83.3 % | +0.820 | +0.646 | 1 | -1.10 | - |
| h4_stack reversed 1h b100-t1 F07 | main | 6 | 100.0 % | +0.818 | +0.633 | 0 | 0.71 | - |
| cdl_bearish_harami_cross reversed 1h b100-trail F04 | main | 6 | 83.3 % | +0.808 | +0.660 | 1 | -1.12 | - |
| cdl_three_inside_down native 5m b100-lvl F07 | main | 6 | 83.3 % | +0.577 | +0.002 | 1 | -1.53 | - |
| cdl_morning_doji_star native 1h b100-t1 F09 | main | 6 | 83.3 % | +0.527 | +0.397 | 1 | -1.17 | - |
| cdl_bearish_harami_cross reversed 1h b100-t1 F04 | main | 6 | 83.3 % | +0.507 | +0.359 | 1 | -1.12 | - |
| cdl_bullish_harami reversed 1h b100-t1 F08 | main | 6 | 83.3 % | +0.494 | +0.320 | 1 | -1.10 | - |
| cdl_three_white_soldiers reversed 15m b100-t1 F03 | main | 6 | 100.0 % | +0.450 | -0.100 | 0 | 0.16 | - |
| cdl_gravestone_doji native 15m b100-t1 F01 | main | 6 | 83.3 % | +0.329 | -0.012 | 1 | -1.18 | - |
| cdl_morning_doji_star native 1h b100-trail F09 | main | 6 | 83.3 % | +0.326 | +0.195 | 1 | -1.17 | - |
| cdl_three_white_soldiers reversed 15m b100-t1 F09 | main | 6 | 83.3 % | +0.102 | -0.463 | 1 | -1.79 | - |
| cdl_three_white_soldiers reversed 15m b100-t1 F08 | main | 6 | 83.3 % | +0.031 | -0.604 | 1 | -1.79 | - |
| cdl_dragonfly_doji reversed 1h b025-lvl F08 | main | 5 | 80.0 % | +2.225 | +1.252 | 1 | -2.62 | - |
| cdl_morning_star native 1h b025-trail F04 | main | 5 | 80.0 % | +1.904 | +1.640 | 1 | -1.30 | - |
| cdl_three_white_soldiers reversed 15m b100-lvl F03 | main | 5 | 80.0 % | +1.302 | +0.743 | 1 | -1.47 | - |
| h4_stack reversed 1h b100-lvl F07 | main | 5 | 100.0 % | +1.246 | +1.036 | 0 | 0.97 | - |
| cdl_dragonfly_doji reversed 1h b025-t3 F08 | main | 5 | 80.0 % | +1.220 | +0.239 | 1 | -2.62 | - |
| cdl_morning_star native 1h b100-trail F04 | main | 5 | 80.0 % | +1.197 | +1.030 | 1 | -1.17 | - |
| cdl_dragonfly_doji reversed 1h b100-lvl F08 | main | 5 | 100.0 % | +1.165 | +0.880 | 0 | 0.73 | - |
| cdl_morning_star native 1h b025-lvl F04 | main | 5 | 80.0 % | +1.073 | +0.812 | 1 | -1.30 | - |
| cdl_bullish_harami_cross native 1h b100-t1 F04 | main | 5 | 100.0 % | +0.789 | +0.585 | 0 | 0.65 | - |
| cdl_dragonfly_doji reversed 1h b100-t1 F08 | main | 5 | 100.0 % | +0.713 | +0.426 | 0 | 0.58 | - |
| sd_formed native 15m b100-lvl F02 | main | 5 | 80.0 % | +0.583 | +0.373 | 1 | -1.14 | - |
| cdl_morning_star native 1h b100-t1 F04 | main | 5 | 80.0 % | +0.432 | +0.267 | 1 | -1.17 | - |
| cdl_dragonfly_doji reversed 1h b025-t2 F08 | main | 5 | 80.0 % | +0.418 | -0.564 | 1 | -2.62 | - |
| cdl_dark_cloud_cover native 1h b025-t1 F02 | main | 5 | 80.0 % | +0.379 | +0.154 | 1 | -1.32 | - |
| h4_trend reversed 5m b025-lvl F01 | main | 5 | 80.0 % | +0.370 | -0.235 | 1 | -1.98 | - |
| cdl_bearish_harami_cross native 1h b100-t1 F06 | main | 5 | 80.0 % | +0.363 | +0.114 | 1 | -1.27 | - |
| cdl_morning_star native 1h b025-t1 F04 | main | 5 | 80.0 % | +0.337 | +0.077 | 1 | -1.30 | - |
| cdl_dark_cloud_cover native 1h b100-t1 F02 | main | 5 | 80.0 % | +0.333 | +0.201 | 1 | -1.07 | - |
| cdl_dark_cloud_cover native 1h b025-t1 F07 | main | 5 | 80.0 % | +0.258 | -0.094 | 1 | -1.45 | - |
| cdl_evening_doji_star reversed 1h b025-t1 F04 | main | 5 | 80.0 % | +0.234 | -0.126 | 1 | -1.27 | - |
| cdl_inverted_hammer reversed 1h b025-t1 F06 | main | 5 | 80.0 % | +0.225 | -0.160 | 1 | -1.56 | - |
| cdl_bullish_harami_cross native 15m b025-t2 F01 | main | 5 | 80.0 % | +0.193 | -1.014 | 1 | -2.93 | - |
| cdl_bullish_harami_cross native 15m b025-t2 F05 | main | 5 | 80.0 % | +0.193 | -1.014 | 1 | -2.93 | - |
| cdl_bullish_harami_cross native 1h b025-t1 F04 | main | 5 | 80.0 % | +0.180 | -0.230 | 1 | -1.24 | - |
| cdl_morning_star reversed 1h b025-t2 F02 | main | 4 | 100.0 % | +1.790 | +1.581 | 0 | 1.69 | - |
| cdl_hammer reversed 1h b025-t2 F02 | main | 4 | 100.0 % | +1.413 | +0.827 | 0 | 1.26 | - |
| cdl_evening_star native 15m b025-lvl F07 | main | 4 | 100.0 % | +1.174 | +0.828 | 0 | 0.73 | - |
| cdl_morning_star reversed 1h b100-t1 F02 | main | 4 | 100.0 % | +0.889 | +0.777 | 0 | 0.84 | - |
| cdl_evening_star native 15m b100-t1 F07 | main | 4 | 100.0 % | +0.801 | +0.598 | 0 | 0.69 | - |
| cdl_morning_star reversed 1h b025-t1 F02 | main | 4 | 100.0 % | +0.790 | +0.579 | 0 | 0.69 | - |
| cdl_evening_star native 15m b025-t1 F07 | main | 4 | 100.0 % | +0.660 | +0.314 | 0 | 0.47 | - |
| cdl_hammer reversed 1h b025-t1 F02 | main | 4 | 100.0 % | +0.413 | -0.175 | 0 | 0.26 | - |
| cdl_three_inside_up native 15m b025-lvl F01 | main | 3 | 100.0 % | +3.151 | +2.726 | 0 | 0.57 | - |
| sd_broken reversed 1h b100-t2 F06 | main | 3 | 100.0 % | +1.554 | +1.115 | 0 | 1.44 | - |
| cdl_gravestone_doji native 15m b025-t2 F05 | main | 3 | 100.0 % | +1.547 | +1.088 | 0 | 1.40 | - |
| cdl_gravestone_doji native 15m b100-t1 F05 | main | 3 | 100.0 % | +0.745 | +0.486 | 0 | 0.68 | - |
| cdl_three_inside_up native 15m b025-t1 F07 | main | 3 | 100.0 % | +0.601 | +0.207 | 0 | 0.49 | - |
| cdl_evening_doji_star reversed 1h b025-t1 F02 | main | 3 | 100.0 % | +0.596 | +0.199 | 0 | 0.46 | - |
| sd_broken reversed 1h b100-t1 F06 | main | 3 | 100.0 % | +0.560 | +0.120 | 0 | 0.46 | - |
| cdl_gravestone_doji native 15m b025-t1 F05 | main | 3 | 100.0 % | +0.540 | +0.080 | 0 | 0.40 | - |
| cdl_dragonfly_doji reversed 1h b025-lvl F02 | main | 2 | 100.0 % | +5.822 | +5.272 | 0 | 4.27 | - |
| cdl_three_inside_up native 15m b025-lvl F05 | main | 2 | 100.0 % | +4.441 | +4.059 | 0 | 0.81 | - |
| fib_ext reversed 1h b100-trail F00 | main | 2 | 100.0 % | +4.176 | +3.684 | 0 | 3.03 | - |
| fib_ext reversed 1h b100-trail F06 | main | 2 | 100.0 % | +4.176 | +3.684 | 0 | 3.03 | - |
| cdl_morning_doji_star native 15m b100-lvl F01 | main | 2 | 100.0 % | +3.461 | +3.157 | 0 | 0.77 | - |
| cdl_morning_doji_star native 15m b100-lvl F05 | main | 2 | 100.0 % | +3.461 | +3.157 | 0 | 0.77 | - |
| cdl_gravestone_doji native 1h b100-t3 F07 | main | 2 | 100.0 % | +2.901 | +2.763 | 0 | 2.86 | - |
| cdl_three_white_soldiers native 1h b100-t3 F00 | main | 2 | 100.0 % | +2.891 | +2.789 | 0 | 2.87 | - |
| cdl_three_white_soldiers native 1h b025-t3 F00 | main | 2 | 100.0 % | +2.860 | +2.724 | 0 | 2.83 | - |
| cdl_three_black_crows native 15m b100-t3 F02 | main | 2 | 100.0 % | +2.856 | +2.712 | 0 | 2.85 | - |
| cdl_three_inside_down native 1h b025-t3 F04 | main | 2 | 100.0 % | +2.728 | +2.412 | 0 | 2.65 | - |
| cdl_bullish_harami_cross native 1h b100-t3 F02 | main | 2 | 100.0 % | +2.695 | +2.419 | 0 | 2.65 | - |
| fib_ext reversed 1h b100-t3 F00 | main | 2 | 100.0 % | +2.507 | +2.014 | 0 | 2.38 | - |
| fib_ext reversed 1h b100-t3 F06 | main | 2 | 100.0 % | +2.507 | +2.014 | 0 | 2.38 | - |
| cdl_bullish_harami_cross native 1h b025-trail F02 | main | 2 | 100.0 % | +2.452 | +1.876 | 0 | 0.92 | - |
| cdl_dragonfly_doji reversed 1h b025-t3 F02 | main | 2 | 100.0 % | +2.426 | +1.853 | 0 | 2.26 | - |
| cdl_bullish_harami_cross native 1h b025-t3 F02 | main | 2 | 100.0 % | +2.405 | +1.834 | 0 | 2.26 | - |
| cdl_bullish_harami_cross native 1h b100-lvl F02 | main | 2 | 100.0 % | +2.272 | +1.997 | 0 | 0.94 | - |
| cdl_morning_star reversed 15m b025-lvl F05 | main | 2 | 100.0 % | +1.985 | +1.109 | 0 | 1.14 | - |
| cdl_three_white_soldiers native 1h b100-t2 F00 | main | 2 | 100.0 % | +1.897 | +1.797 | 0 | 1.87 | - |
| cdl_gravestone_doji native 1h b100-t2 F07 | main | 2 | 100.0 % | +1.879 | +1.742 | 0 | 1.82 | - |
| cdl_three_white_soldiers native 1h b025-t2 F00 | main | 2 | 100.0 % | +1.867 | +1.730 | 0 | 1.83 | - |
| cdl_three_black_crows native 15m b100-t2 F02 | main | 2 | 100.0 % | +1.858 | +1.712 | 0 | 1.85 | - |
| cdl_dragonfly_doji reversed 1h b100-t2 F02 | main | 2 | 100.0 % | +1.835 | +1.669 | 0 | 1.77 | - |
| cdl_three_inside_down native 1h b100-t2 F04 | main | 2 | 100.0 % | +1.821 | +1.614 | 0 | 1.78 | - |
| fib_ext reversed 1h b100-lvl F00 | main | 2 | 100.0 % | +1.722 | +1.229 | 0 | 1.18 | - |
| fib_ext reversed 1h b100-lvl F06 | main | 2 | 100.0 % | +1.722 | +1.229 | 0 | 1.18 | - |
| cdl_morning_star native 1h b025-t2 F02 | main | 2 | 100.0 % | +1.716 | +1.449 | 0 | 1.67 | - |
| cdl_three_white_soldiers native 1h b025-lvl F00 | main | 2 | 100.0 % | +1.707 | +1.570 | 0 | 1.01 | - |
| cdl_bullish_harami_cross native 1h b100-t2 F02 | main | 2 | 100.0 % | +1.707 | +1.431 | 0 | 1.65 | - |
| cdl_three_inside_down native 1h b025-t2 F04 | main | 2 | 100.0 % | +1.701 | +1.387 | 0 | 1.65 | - |
| cdl_dragonfly_doji reversed 1h b100-lvl F02 | main | 2 | 100.0 % | +1.678 | +1.512 | 0 | 1.34 | - |
| cdl_three_white_soldiers native 1h b100-lvl F00 | main | 2 | 100.0 % | +1.644 | +1.544 | 0 | 1.51 | - |
| cdl_three_inside_down native 1h b025-lvl F04 | main | 2 | 100.0 % | +1.605 | +1.292 | 0 | 1.00 | - |
| cdl_gravestone_doji native 1h b100-trail F07 | main | 2 | 100.0 % | +1.604 | +1.466 | 0 | 0.34 | - |
| cdl_gravestone_doji native 1h b100-lvl F07 | main | 2 | 100.0 % | +1.568 | +1.431 | 0 | 0.86 | - |
| fib_ext reversed 1h b100-t2 F00 | main | 2 | 100.0 % | +1.506 | +1.011 | 0 | 1.38 | - |
| fib_ext reversed 1h b100-t2 F06 | main | 2 | 100.0 % | +1.506 | +1.011 | 0 | 1.38 | - |
| cdl_three_white_soldiers native 1h b025-trail F00 | main | 2 | 100.0 % | +1.478 | +1.341 | 0 | 0.79 | - |
| cdl_morning_star native 1h b025-trail F02 | main | 2 | 100.0 % | +1.451 | +1.181 | 0 | 0.71 | - |
| cdl_dragonfly_doji reversed 1h b025-t2 F02 | main | 2 | 100.0 % | +1.426 | +0.852 | 0 | 1.26 | - |
| cdl_bullish_harami_cross native 1h b025-t2 F02 | main | 2 | 100.0 % | +1.406 | +0.837 | 0 | 1.26 | - |
| cdl_three_white_soldiers native 1h b100-t1.5 F00 | frontier | 2 | 100.0 % | +1.401 | +1.301 | 0 | 1.38 | - |
| cdl_three_white_soldiers native 1h b025-t1.5 F00 | frontier | 2 | 100.0 % | +1.363 | +1.226 | 0 | 1.34 | - |
| cdl_morning_star native 1h b100-lvl F02 | main | 2 | 100.0 % | +1.286 | +1.113 | 0 | 0.90 | - |
| cdl_morning_star native 15m b100-lvl F01 | main | 2 | 100.0 % | +1.262 | +0.971 | 0 | 1.14 | - |
| cdl_morning_star native 1h b025-lvl F02 | main | 2 | 100.0 % | +1.222 | +0.955 | 0 | 1.06 | - |
| cdl_bullish_harami_cross native 1h b100-trail F02 | main | 2 | 100.0 % | +1.174 | +0.897 | 0 | 0.44 | - |
| cdl_bullish_harami_cross native 1h b025-lvl F02 | main | 2 | 100.0 % | +1.163 | +0.593 | 0 | 0.39 | - |
| cdl_three_inside_down native 1h b100-lvl F04 | main | 2 | 100.0 % | +1.120 | +0.913 | 0 | 0.83 | - |
| cdl_three_white_soldiers native 1h b100-trail F00 | main | 2 | 100.0 % | +1.093 | +0.992 | 0 | 0.58 | - |
| fib_ext reversed 1h b100-t1.5 F00 | frontier | 2 | 100.0 % | +1.006 | +0.512 | 0 | 0.88 | - |
| cdl_morning_star native 1h b100-trail F02 | main | 2 | 100.0 % | +0.942 | +0.767 | 0 | 0.46 | - |
| cdl_three_inside_down native 1h b025-trail F04 | main | 2 | 100.0 % | +0.938 | +0.620 | 0 | 0.81 | - |
| cdl_three_white_soldiers native 1h b100-t1 F00 | main | 2 | 100.0 % | +0.899 | +0.798 | 0 | 0.88 | - |
| cdl_gravestone_doji native 1h b100-t1 F07 | main | 2 | 100.0 % | +0.867 | +0.726 | 0 | 0.80 | - |
| cdl_three_white_soldiers native 1h b025-t1 F00 | main | 2 | 100.0 % | +0.863 | +0.727 | 0 | 0.84 | - |
| cdl_three_black_crows native 15m b100-t1 F02 | main | 2 | 100.0 % | +0.858 | +0.714 | 0 | 0.86 | - |
| cdl_dragonfly_doji reversed 1h b100-t1 F02 | main | 2 | 100.0 % | +0.828 | +0.656 | 0 | 0.77 | - |
| cdl_morning_star native 1h b100-t1 F02 | main | 2 | 100.0 % | +0.824 | +0.651 | 0 | 0.78 | - |
| cdl_three_inside_down native 1h b100-t1 F04 | main | 2 | 100.0 % | +0.800 | +0.595 | 0 | 0.77 | - |
| cdl_morning_star native 1h b025-t1 F02 | main | 2 | 100.0 % | +0.729 | +0.462 | 0 | 0.67 | - |
| cdl_bullish_harami_cross native 1h b100-t1 F02 | main | 2 | 100.0 % | +0.715 | +0.442 | 0 | 0.65 | - |
| cdl_three_inside_down native 1h b025-t1 F04 | main | 2 | 100.0 % | +0.696 | +0.383 | 0 | 0.64 | - |
| cdl_three_inside_up native 15m b025-lvl F07 | main | 2 | 100.0 % | +0.692 | +0.243 | 0 | 0.57 | - |
| cdl_three_white_soldiers native 1h b100-t0.75 F00 | frontier | 2 | 100.0 % | +0.649 | +0.548 | 0 | 0.63 | - |
| cdl_three_inside_down native 1h b100-trail F04 | main | 2 | 100.0 % | +0.617 | +0.408 | 0 | 0.55 | - |
| cdl_three_white_soldiers native 1h b025-t0.75 F00 | frontier | 2 | 100.0 % | +0.615 | +0.479 | 0 | 0.59 | - |
| cdl_falling_three_methods native 5m b100-t0.75 F00 | frontier | 2 | 100.0 % | +0.544 | +0.339 | 0 | 0.52 | - |
| fib_ext reversed 1h b100-t1 F00 | main | 2 | 100.0 % | +0.506 | +0.011 | 0 | 0.38 | - |
| fib_ext reversed 1h b100-t1 F06 | main | 2 | 100.0 % | +0.506 | +0.011 | 0 | 0.38 | - |
| cdl_falling_three_methods native 5m b025-t0.75 F00 | frontier | 2 | 100.0 % | +0.479 | +0.208 | 0 | 0.47 | - |
| cdl_three_black_crows native 15m b100-trail F02 | main | 2 | 100.0 % | +0.455 | +0.309 | 0 | 0.42 | - |
| cdl_dragonfly_doji reversed 1h b025-t1 F02 | main | 2 | 100.0 % | +0.425 | -0.149 | 0 | 0.26 | - |
| cdl_three_inside_up reversed 15m b100-t1 F05 | main | 2 | 100.0 % | +0.421 | -0.178 | 0 | 0.34 | - |
| cdl_bullish_harami_cross native 1h b025-t1 F02 | main | 2 | 100.0 % | +0.407 | -0.162 | 0 | 0.26 | - |
| cdl_three_white_soldiers native 1h b100-t0.5 F00 | frontier | 2 | 100.0 % | +0.400 | +0.299 | 0 | 0.38 | - |
| cdl_three_white_soldiers native 1h b025-t0.5 F00 | frontier | 2 | 100.0 % | +0.365 | +0.228 | 0 | 0.34 | - |
| cdl_falling_three_methods native 5m b100-t0.5 F00 | frontier | 2 | 100.0 % | +0.294 | +0.087 | 0 | 0.27 | - |
| fib_ext reversed 1h b100-t0.75 F00 | frontier | 2 | 100.0 % | +0.255 | -0.241 | 0 | 0.13 | - |
| cdl_falling_three_methods native 5m b025-t0.5 F00 | frontier | 2 | 100.0 % | +0.229 | -0.041 | 0 | 0.22 | - |
| cdl_three_white_soldiers native 1h b100-t0.25 F00 | frontier | 2 | 100.0 % | +0.149 | +0.049 | 0 | 0.13 | - |
| cdl_three_white_soldiers native 1h b025-t0.25 F00 | frontier | 2 | 100.0 % | +0.113 | -0.023 | 0 | 0.09 | - |
| cdl_falling_three_methods native 5m b100-t0.25 F00 | frontier | 2 | 100.0 % | +0.044 | -0.163 | 0 | 0.02 | - |
| cdl_evening_doji_star native 1h b025-trail F07 | main | 1 | 100.0 % | +7.741 | +7.532 | 0 | 7.74 | - |
| cdl_three_inside_up native 15m b100-lvl F05 | main | 1 | 100.0 % | +5.460 | +5.205 | 0 | 5.46 | - |
| cdl_evening_doji_star native 1h b100-trail F07 | main | 1 | 100.0 % | +4.386 | +4.268 | 0 | 4.39 | - |
| cdl_bullish_harami_cross native 1h b025-trail F07 | main | 1 | 100.0 % | +3.987 | +3.287 | 0 | 3.99 | - |
| cdl_bullish_harami_cross native 1h b100-lvl F07 | main | 1 | 100.0 % | +3.607 | +3.275 | 0 | 3.61 | - |
| fib_ext reversed 1h b100-trail F08 | main | 1 | 100.0 % | +3.028 | +2.660 | 0 | 3.03 | - |
| fib_ext reversed 1h b100-trail F09 | main | 1 | 100.0 % | +3.028 | +2.660 | 0 | 3.03 | - |
| cdl_dragonfly_doji native 1h b100-trail F07 | main | 1 | 100.0 % | +2.974 | +2.816 | 0 | 2.97 | - |
| cdl_three_black_crows native 1h b100-t3 F00 | main | 1 | 100.0 % | +2.965 | +2.894 | 0 | 2.96 | - |
| cdl_three_black_crows native 1h b100-t3 F06 | main | 1 | 100.0 % | +2.965 | +2.894 | 0 | 2.96 | - |
| cdl_three_black_crows native 1h b025-t3 F00 | main | 1 | 100.0 % | +2.956 | +2.865 | 0 | 2.96 | - |
| cdl_three_black_crows native 1h b025-t3 F06 | main | 1 | 100.0 % | +2.956 | +2.865 | 0 | 2.96 | - |
| cdl_evening_doji_star native 1h b100-t3 F07 | main | 1 | 100.0 % | +2.893 | +2.779 | 0 | 2.89 | - |
| cdl_evening_doji_star native 1h b100-t3 F08 | main | 1 | 100.0 % | +2.892 | +2.756 | 0 | 2.89 | - |
| cdl_evening_star native 1h b025-lvl F02 | main | 1 | 100.0 % | +2.875 | +2.630 | 0 | 2.88 | - |
| cdl_evening_star native 1h b025-lvl F07 | main | 1 | 100.0 % | +2.875 | +2.630 | 0 | 2.88 | - |
| cdl_three_black_crows native 15m b100-t3 F01 | main | 1 | 100.0 % | +2.860 | +2.717 | 0 | 2.86 | - |
| cdl_three_black_crows native 15m b100-t3 F05 | main | 1 | 100.0 % | +2.860 | +2.717 | 0 | 2.86 | - |
| cdl_morning_doji_star reversed 1h b100-t3 F06 | main | 1 | 100.0 % | +2.859 | +2.712 | 0 | 2.86 | - |
| cdl_evening_doji_star native 1h b025-t3 F08 | main | 1 | 100.0 % | +2.833 | +2.621 | 0 | 2.83 | - |
| cdl_three_black_crows native 15m b025-t3 F01 | main | 1 | 100.0 % | +2.826 | +2.649 | 0 | 2.83 | - |
| cdl_three_black_crows native 15m b025-t3 F05 | main | 1 | 100.0 % | +2.826 | +2.649 | 0 | 2.83 | - |
| cdl_evening_doji_star native 1h b025-t3 F07 | main | 1 | 100.0 % | +2.799 | +2.598 | 0 | 2.80 | - |
| cdl_gravestone_doji native 1h b100-t3 F02 | main | 1 | 100.0 % | +2.783 | +2.562 | 0 | 2.78 | - |
| cdl_morning_doji_star reversed 1h b025-t3 F06 | main | 1 | 100.0 % | +2.779 | +2.545 | 0 | 2.78 | - |
| cdl_dragonfly_doji native 1h b100-t3 F07 | main | 1 | 100.0 % | +2.776 | +2.621 | 0 | 2.78 | - |
| cdl_evening_star native 1h b025-t3 F02 | main | 1 | 100.0 % | +2.764 | +2.521 | 0 | 2.76 | - |
| cdl_evening_star native 1h b025-t3 F07 | main | 1 | 100.0 % | +2.764 | +2.521 | 0 | 2.76 | - |
| cdl_falling_three_methods native 5m b025-t3 F07 | main | 1 | 100.0 % | +2.751 | +2.471 | 0 | 2.75 | - |
| cdl_rising_three_methods reversed 15m b100-t3 F04 | main | 1 | 100.0 % | +2.734 | +2.466 | 0 | 2.73 | - |
| cdl_three_inside_down native 1h b025-t3 F02 | main | 1 | 100.0 % | +2.652 | +2.292 | 0 | 2.65 | - |
| cdl_bullish_harami_cross native 1h b100-t3 F07 | main | 1 | 100.0 % | +2.646 | +2.314 | 0 | 2.65 | - |
| fib_ext reversed 1h b100-t3 F08 | main | 1 | 100.0 % | +2.633 | +2.266 | 0 | 2.63 | - |
| fib_ext reversed 1h b100-t3 F09 | main | 1 | 100.0 % | +2.633 | +2.266 | 0 | 2.63 | - |
| cdl_gravestone_doji native 1h b025-t3 F02 | main | 1 | 100.0 % | +2.592 | +2.184 | 0 | 2.59 | - |
| cdl_rising_three_methods reversed 5m b100-t3 F03 | main | 1 | 100.0 % | +2.402 | +1.821 | 0 | 2.40 | - |
| cdl_evening_doji_star native 1h b025-lvl F07 | main | 1 | 100.0 % | +2.399 | +2.198 | 0 | 2.40 | - |
| cdl_evening_doji_star native 1h b025-lvl F08 | main | 1 | 100.0 % | +2.272 | +2.055 | 0 | 2.27 | - |
| cdl_bullish_harami_cross native 1h b025-t3 F07 | main | 1 | 100.0 % | +2.262 | +1.572 | 0 | 2.26 | - |
| cdl_three_inside_down native 1h b025-lvl F02 | main | 1 | 100.0 % | +2.205 | +1.846 | 0 | 2.21 | - |
| ema_stack reversed 1h b025-t3 F04 | main | 1 | 100.0 % | +2.195 | +1.325 | 0 | 2.19 | - |
| cdl_three_black_crows native 1h b100-t2 F00 | main | 1 | 100.0 % | +1.934 | +1.858 | 0 | 1.93 | - |
| cdl_three_black_crows native 1h b100-t2 F06 | main | 1 | 100.0 % | +1.934 | +1.858 | 0 | 1.93 | - |
| cdl_three_black_crows native 1h b025-t2 F00 | main | 1 | 100.0 % | +1.911 | +1.816 | 0 | 1.91 | - |
| cdl_three_black_crows native 1h b025-t2 F06 | main | 1 | 100.0 % | +1.911 | +1.816 | 0 | 1.91 | - |
| cdl_bullish_harami_cross native 1h b100-trail F07 | main | 1 | 100.0 % | +1.904 | +1.569 | 0 | 1.90 | - |
| cdl_evening_doji_star native 1h b100-t2 F08 | main | 1 | 100.0 % | +1.894 | +1.760 | 0 | 1.89 | - |
| cdl_evening_doji_star native 1h b100-t2 F07 | main | 1 | 100.0 % | +1.892 | +1.778 | 0 | 1.89 | - |
| cdl_three_black_crows native 15m b100-t2 F01 | main | 1 | 100.0 % | +1.861 | +1.719 | 0 | 1.86 | - |
| cdl_three_black_crows native 15m b100-t2 F05 | main | 1 | 100.0 % | +1.861 | +1.719 | 0 | 1.86 | - |
| cdl_morning_doji_star reversed 1h b100-t2 F06 | main | 1 | 100.0 % | +1.860 | +1.712 | 0 | 1.86 | - |
| cdl_evening_star native 1h b100-t2 F02 | main | 1 | 100.0 % | +1.854 | +1.701 | 0 | 1.85 | - |
| cdl_evening_star native 1h b100-t2 F07 | main | 1 | 100.0 % | +1.854 | +1.701 | 0 | 1.85 | - |
| cdl_evening_doji_star native 1h b025-trail F08 | main | 1 | 100.0 % | +1.828 | +1.614 | 0 | 1.83 | - |
| cdl_three_black_crows native 15m b025-t2 F01 | main | 1 | 100.0 % | +1.823 | +1.647 | 0 | 1.82 | - |
| cdl_three_black_crows native 15m b025-t2 F05 | main | 1 | 100.0 % | +1.823 | +1.647 | 0 | 1.82 | - |
| cdl_evening_doji_star native 1h b025-t2 F08 | main | 1 | 100.0 % | +1.805 | +1.588 | 0 | 1.80 | - |
| cdl_evening_doji_star native 1h b025-t2 F07 | main | 1 | 100.0 % | +1.798 | +1.597 | 0 | 1.80 | - |
| cdl_falling_three_methods native 5m b100-t2 F07 | main | 1 | 100.0 % | +1.793 | +1.561 | 0 | 1.79 | - |
| cdl_evening_star native 1h b100-lvl F02 | main | 1 | 100.0 % | +1.792 | +1.639 | 0 | 1.79 | - |
| cdl_evening_star native 1h b100-lvl F07 | main | 1 | 100.0 % | +1.792 | +1.639 | 0 | 1.79 | - |
| cdl_morning_doji_star reversed 1h b025-t2 F06 | main | 1 | 100.0 % | +1.781 | +1.551 | 0 | 1.78 | - |
| cdl_gravestone_doji native 1h b100-t2 F02 | main | 1 | 100.0 % | +1.779 | +1.559 | 0 | 1.78 | - |
| cdl_three_inside_down native 1h b100-t2 F02 | main | 1 | 100.0 % | +1.777 | +1.547 | 0 | 1.78 | - |
| cdl_dragonfly_doji native 1h b100-t2 F07 | main | 1 | 100.0 % | +1.777 | +1.623 | 0 | 1.78 | - |
| cdl_evening_star native 1h b025-t2 F02 | main | 1 | 100.0 % | +1.764 | +1.520 | 0 | 1.76 | - |
| cdl_evening_star native 1h b025-t2 F07 | main | 1 | 100.0 % | +1.764 | +1.520 | 0 | 1.76 | - |
| cdl_rising_three_methods reversed 15m b100-trail F04 | main | 1 | 100.0 % | +1.744 | +1.476 | 0 | 1.74 | - |
| cdl_falling_three_methods native 5m b025-t2 F07 | main | 1 | 100.0 % | +1.737 | +1.453 | 0 | 1.74 | - |
| cdl_rising_three_methods reversed 15m b100-t2 F04 | main | 1 | 100.0 % | +1.734 | +1.466 | 0 | 1.73 | - |
| cdl_rising_three_methods reversed 5m b100-lvl F06 | main | 1 | 100.0 % | +1.718 | +0.328 | 0 | 1.72 | - |
| cdl_three_inside_down native 1h b025-t2 F02 | main | 1 | 100.0 % | +1.653 | +1.294 | 0 | 1.65 | - |
| cdl_morning_doji_star reversed 1h b025-lvl F06 | main | 1 | 100.0 % | +1.649 | +1.418 | 0 | 1.65 | - |
| cdl_bullish_harami_cross native 1h b100-t2 F07 | main | 1 | 100.0 % | +1.646 | +1.315 | 0 | 1.65 | - |
| fib_ext reversed 1h b100-t2 F08 | main | 1 | 100.0 % | +1.632 | +1.265 | 0 | 1.63 | - |
| fib_ext reversed 1h b100-t2 F09 | main | 1 | 100.0 % | +1.632 | +1.265 | 0 | 1.63 | - |
| cdl_rising_three_methods reversed 5m b100-t3 F06 | main | 1 | 100.0 % | +1.611 | +0.221 | 0 | 1.61 | - |
| cdl_gravestone_doji native 1h b025-t2 F02 | main | 1 | 100.0 % | +1.591 | +1.182 | 0 | 1.59 | - |
| cdl_three_black_crows native 1h b025-lvl F00 | main | 1 | 100.0 % | +1.584 | +1.488 | 0 | 1.58 | - |
| cdl_three_black_crows native 1h b025-lvl F06 | main | 1 | 100.0 % | +1.584 | +1.488 | 0 | 1.58 | - |
| cdl_three_black_crows native 1h b025-trail F00 | main | 1 | 100.0 % | +1.536 | +1.442 | 0 | 1.54 | - |
| cdl_three_black_crows native 1h b025-trail F06 | main | 1 | 100.0 % | +1.536 | +1.442 | 0 | 1.54 | - |
| cdl_evening_doji_star native 1h b100-lvl F08 | main | 1 | 100.0 % | +1.440 | +1.303 | 0 | 1.44 | - |
| cdl_three_black_crows native 1h b100-t1.5 F00 | frontier | 1 | 100.0 % | +1.431 | +1.357 | 0 | 1.43 | - |
| cdl_rising_three_methods reversed 5m b100-t2 F03 | main | 1 | 100.0 % | +1.422 | +0.843 | 0 | 1.42 | - |
| cdl_three_black_crows native 1h b025-t1.5 F00 | frontier | 1 | 100.0 % | +1.411 | +1.315 | 0 | 1.41 | - |
| cdl_three_inside_down native 1h b100-lvl F02 | main | 1 | 100.0 % | +1.410 | +1.181 | 0 | 1.41 | - |
| cdl_gravestone_doji native 1h b100-lvl F02 | main | 1 | 100.0 % | +1.389 | +1.169 | 0 | 1.39 | - |
| cdl_morning_star native 15m b100-lvl F05 | main | 1 | 100.0 % | +1.380 | +1.005 | 0 | 1.38 | - |
| cdl_evening_doji_star native 1h b100-lvl F07 | main | 1 | 100.0 % | +1.359 | +1.246 | 0 | 1.36 | - |
| cdl_three_inside_up native 1h b025-t1.5 F00 | frontier | 1 | 100.0 % | +1.350 | +1.220 | 0 | 1.35 | - |
| cdl_falling_three_methods native 5m b025-lvl F00 | main | 1 | 100.0 % | +1.314 | +1.032 | 0 | 1.31 | - |
| cdl_falling_three_methods native 5m b025-lvl F07 | main | 1 | 100.0 % | +1.314 | +1.032 | 0 | 1.31 | - |
| cdl_bullish_harami_cross native 1h b025-t2 F07 | main | 1 | 100.0 % | +1.263 | +0.574 | 0 | 1.26 | - |
| cdl_gravestone_doji native 1h b025-lvl F02 | main | 1 | 100.0 % | +1.230 | +0.821 | 0 | 1.23 | - |
| cdl_three_black_crows native 1h b100-lvl F00 | main | 1 | 100.0 % | +1.226 | +1.152 | 0 | 1.23 | - |
| cdl_three_black_crows native 1h b100-lvl F06 | main | 1 | 100.0 % | +1.226 | +1.152 | 0 | 1.23 | - |
| cdl_three_black_crows native 1h b100-trail F00 | main | 1 | 100.0 % | +1.190 | +1.116 | 0 | 1.19 | - |
| cdl_three_black_crows native 1h b100-trail F06 | main | 1 | 100.0 % | +1.190 | +1.116 | 0 | 1.19 | - |
| fib_ext reversed 1h b100-lvl F08 | main | 1 | 100.0 % | +1.178 | +0.812 | 0 | 1.18 | - |
| fib_ext reversed 1h b100-lvl F09 | main | 1 | 100.0 % | +1.178 | +0.812 | 0 | 1.18 | - |
| fib_ext reversed 5m b100-t3 F07 | main | 1 | 100.0 % | +1.171 | -0.658 | 0 | 1.17 | - |
| cdl_evening_doji_star native 1h b100-trail F08 | main | 1 | 100.0 % | +1.159 | +1.023 | 0 | 1.16 | - |
| ema_stack reversed 1h b025-t2 F04 | main | 1 | 100.0 % | +1.129 | +0.258 | 0 | 1.13 | - |
| cdl_falling_three_methods native 5m b100-lvl F00 | main | 1 | 100.0 % | +1.093 | +0.858 | 0 | 1.09 | - |
| cdl_falling_three_methods native 5m b100-lvl F07 | main | 1 | 100.0 % | +1.093 | +0.858 | 0 | 1.09 | - |
| sd_formed native 15m b100-lvl F05 | main | 1 | 100.0 % | +1.085 | +0.932 | 0 | 1.08 | - |
| cdl_three_inside_down native 1h b025-trail F02 | main | 1 | 100.0 % | +1.070 | +0.706 | 0 | 1.07 | - |
| cdl_morning_doji_star reversed 1h b100-lvl F06 | main | 1 | 100.0 % | +1.043 | +0.897 | 0 | 1.04 | - |
| cdl_rising_three_methods reversed 15m b100-lvl F04 | main | 1 | 100.0 % | +0.969 | +0.696 | 0 | 0.97 | - |
| cdl_rising_three_methods reversed 5m b100-lvl F03 | main | 1 | 100.0 % | +0.954 | +0.375 | 0 | 0.95 | - |
| cdl_evening_star native 1h b025-trail F02 | main | 1 | 100.0 % | +0.944 | +0.694 | 0 | 0.94 | - |
| cdl_evening_star native 1h b025-trail F07 | main | 1 | 100.0 % | +0.944 | +0.694 | 0 | 0.94 | - |
| cdl_three_black_crows native 1h b100-t1 F00 | main | 1 | 100.0 % | +0.931 | +0.857 | 0 | 0.93 | - |
| cdl_three_black_crows native 1h b100-t1 F06 | main | 1 | 100.0 % | +0.931 | +0.857 | 0 | 0.93 | - |
| cdl_three_black_crows native 1h b025-t1 F00 | main | 1 | 100.0 % | +0.910 | +0.814 | 0 | 0.91 | - |
| cdl_three_black_crows native 1h b025-t1 F06 | main | 1 | 100.0 % | +0.910 | +0.814 | 0 | 0.91 | - |
| cdl_three_inside_up native 1h b100-t1 F00 | main | 1 | 100.0 % | +0.893 | +0.801 | 0 | 0.89 | - |
| cdl_three_inside_up native 1h b100-t1 F04 | main | 1 | 100.0 % | +0.893 | +0.801 | 0 | 0.89 | - |
| cdl_three_inside_up native 1h b100-t1 F09 | main | 1 | 100.0 % | +0.893 | +0.801 | 0 | 0.89 | - |
| cdl_evening_doji_star native 1h b100-t1 F07 | main | 1 | 100.0 % | +0.886 | +0.771 | 0 | 0.89 | - |
| cdl_morning_doji_star reversed 1h b025-trail F06 | main | 1 | 100.0 % | +0.877 | +0.645 | 0 | 0.88 | - |
| cdl_evening_doji_star native 1h b100-t1 F08 | main | 1 | 100.0 % | +0.877 | +0.739 | 0 | 0.88 | - |
| cdl_three_black_crows native 15m b100-t1 F01 | main | 1 | 100.0 % | +0.860 | +0.719 | 0 | 0.86 | - |
| cdl_three_black_crows native 15m b100-t1 F05 | main | 1 | 100.0 % | +0.860 | +0.719 | 0 | 0.86 | - |
| cdl_three_inside_up native 1h b025-lvl F00 | main | 1 | 100.0 % | +0.858 | +0.729 | 0 | 0.86 | - |
| cdl_three_inside_up native 1h b025-lvl F04 | main | 1 | 100.0 % | +0.858 | +0.729 | 0 | 0.86 | - |
| cdl_three_inside_up native 1h b025-lvl F09 | main | 1 | 100.0 % | +0.858 | +0.729 | 0 | 0.86 | - |
| cdl_three_inside_up native 1h b025-t1 F00 | main | 1 | 100.0 % | +0.858 | +0.729 | 0 | 0.86 | - |
| cdl_three_inside_up native 1h b025-t1 F04 | main | 1 | 100.0 % | +0.858 | +0.729 | 0 | 0.86 | - |
| cdl_three_inside_up native 1h b025-t1 F09 | main | 1 | 100.0 % | +0.858 | +0.729 | 0 | 0.86 | - |
| cdl_morning_doji_star reversed 1h b100-t1 F06 | main | 1 | 100.0 % | +0.854 | +0.707 | 0 | 0.85 | - |
| cdl_evening_star native 1h b100-t1 F02 | main | 1 | 100.0 % | +0.853 | +0.701 | 0 | 0.85 | - |
| cdl_evening_star native 1h b100-t1 F07 | main | 1 | 100.0 % | +0.853 | +0.701 | 0 | 0.85 | - |
| cdl_three_black_crows native 15m b025-t1 F01 | main | 1 | 100.0 % | +0.825 | +0.650 | 0 | 0.82 | - |
| cdl_three_black_crows native 15m b025-t1 F05 | main | 1 | 100.0 % | +0.825 | +0.650 | 0 | 0.82 | - |
| cdl_evening_doji_star native 1h b025-t1 F08 | main | 1 | 100.0 % | +0.805 | +0.587 | 0 | 0.81 | - |
| cdl_evening_doji_star native 1h b025-t1 F07 | main | 1 | 100.0 % | +0.800 | +0.600 | 0 | 0.80 | - |
| cdl_dragonfly_doji native 1h b100-t1 F07 | main | 1 | 100.0 % | +0.799 | +0.648 | 0 | 0.80 | - |
| cdl_gravestone_doji native 1h b100-t1 F02 | main | 1 | 100.0 % | +0.779 | +0.559 | 0 | 0.78 | - |
| cdl_rising_three_methods reversed 5m b100-trail F03 | main | 1 | 100.0 % | +0.774 | +0.194 | 0 | 0.77 | - |
| cdl_three_inside_down native 1h b100-t1 F02 | main | 1 | 100.0 % | +0.771 | +0.543 | 0 | 0.77 | - |
| cdl_morning_doji_star reversed 1h b025-t1 F06 | main | 1 | 100.0 % | +0.769 | +0.537 | 0 | 0.77 | - |
| cdl_falling_three_methods native 5m b100-t1 F07 | main | 1 | 100.0 % | +0.764 | +0.529 | 0 | 0.76 | - |
| cdl_evening_star native 1h b025-t1 F02 | main | 1 | 100.0 % | +0.757 | +0.513 | 0 | 0.76 | - |
| cdl_evening_star native 1h b025-t1 F07 | main | 1 | 100.0 % | +0.757 | +0.513 | 0 | 0.76 | - |
| cdl_rising_three_methods reversed 15m b100-t1 F04 | main | 1 | 100.0 % | +0.726 | +0.453 | 0 | 0.73 | - |
| cdl_falling_three_methods native 5m b025-t1 F07 | main | 1 | 100.0 % | +0.719 | +0.437 | 0 | 0.72 | - |
| cdl_gravestone_doji native 1h b025-trail F02 | main | 1 | 100.0 % | +0.709 | +0.297 | 0 | 0.71 | - |
| cdl_three_inside_down native 1h b100-trail F02 | main | 1 | 100.0 % | +0.684 | +0.451 | 0 | 0.68 | - |
| cdl_three_black_crows native 1h b100-t0.75 F00 | frontier | 1 | 100.0 % | +0.680 | +0.606 | 0 | 0.68 | - |
| cdl_three_black_crows native 1h b025-t0.75 F00 | frontier | 1 | 100.0 % | +0.660 | +0.564 | 0 | 0.66 | - |
| cdl_evening_doji_star reversed 1h b025-t1 F08 | main | 1 | 100.0 % | +0.658 | +0.335 | 0 | 0.66 | - |
| cdl_three_inside_up native 1h b100-t0.75 F00 | frontier | 1 | 100.0 % | +0.649 | +0.558 | 0 | 0.65 | - |
| cdl_bullish_harami_cross native 1h b100-t1 F07 | main | 1 | 100.0 % | +0.648 | +0.319 | 0 | 0.65 | - |
| cdl_three_inside_down native 1h b025-t1 F02 | main | 1 | 100.0 % | +0.642 | +0.285 | 0 | 0.64 | - |
| fib_ext reversed 1h b100-t1 F08 | main | 1 | 100.0 % | +0.634 | +0.267 | 0 | 0.63 | - |
| fib_ext reversed 1h b100-t1 F09 | main | 1 | 100.0 % | +0.634 | +0.267 | 0 | 0.63 | - |
| cdl_three_inside_up native 1h b025-t0.75 F00 | frontier | 1 | 100.0 % | +0.617 | +0.484 | 0 | 0.62 | - |
| cdl_rising_three_methods reversed 5m b100-t2 F06 | main | 1 | 100.0 % | +0.612 | -0.775 | 0 | 0.61 | - |
| cdl_gravestone_doji native 1h b025-t1 F02 | main | 1 | 100.0 % | +0.591 | +0.181 | 0 | 0.59 | - |
| cdl_evening_star native 1h b100-trail F02 | main | 1 | 100.0 % | +0.588 | +0.433 | 0 | 0.59 | - |
| cdl_evening_star native 1h b100-trail F07 | main | 1 | 100.0 % | +0.588 | +0.433 | 0 | 0.59 | - |
| cdl_morning_doji_star reversed 1h b100-trail F06 | main | 1 | 100.0 % | +0.555 | +0.408 | 0 | 0.56 | - |
| cdl_three_black_crows native 15m b025-trail F01 | main | 1 | 100.0 % | +0.526 | +0.349 | 0 | 0.53 | - |
| cdl_three_black_crows native 15m b025-trail F05 | main | 1 | 100.0 % | +0.526 | +0.349 | 0 | 0.53 | - |
| cdl_three_inside_up native 1h b025-trail F00 | main | 1 | 100.0 % | +0.433 | +0.304 | 0 | 0.43 | - |
| cdl_three_inside_up native 1h b025-trail F04 | main | 1 | 100.0 % | +0.433 | +0.304 | 0 | 0.43 | - |
| cdl_three_inside_up native 1h b025-trail F09 | main | 1 | 100.0 % | +0.433 | +0.304 | 0 | 0.43 | - |
| cdl_three_black_crows native 1h b100-t0.5 F00 | frontier | 1 | 100.0 % | +0.430 | +0.356 | 0 | 0.43 | - |
| cdl_falling_three_methods native 5m b100-trail F07 | main | 1 | 100.0 % | +0.427 | +0.190 | 0 | 0.43 | - |
| cdl_three_black_crows native 15m b100-trail F01 | main | 1 | 100.0 % | +0.421 | +0.279 | 0 | 0.42 | - |
| cdl_three_black_crows native 15m b100-trail F05 | main | 1 | 100.0 % | +0.421 | +0.279 | 0 | 0.42 | - |
| cdl_rising_three_methods reversed 5m b100-t1 F03 | main | 1 | 100.0 % | +0.415 | -0.171 | 0 | 0.41 | - |
| cdl_three_black_crows native 1h b025-t0.5 F00 | frontier | 1 | 100.0 % | +0.410 | +0.314 | 0 | 0.41 | - |
| cdl_three_inside_up native 1h b100-t0.5 F00 | frontier | 1 | 100.0 % | +0.406 | +0.312 | 0 | 0.41 | - |
| cdl_bullish_harami_cross native 1h b025-lvl F07 | main | 1 | 100.0 % | +0.388 | -0.300 | 0 | 0.39 | - |
| cdl_gravestone_doji native 1h b100-trail F02 | main | 1 | 100.0 % | +0.383 | +0.161 | 0 | 0.38 | - |
| cdl_three_inside_up native 1h b025-t0.5 F00 | frontier | 1 | 100.0 % | +0.367 | +0.234 | 0 | 0.37 | - |
| cdl_bullish_harami_cross native 1h b025-t1 F07 | main | 1 | 100.0 % | +0.264 | -0.425 | 0 | 0.26 | - |
| cdl_evening_star reversed 1h b025-t1 F06 | main | 1 | 100.0 % | +0.241 | -0.517 | 0 | 0.24 | - |
| cdl_three_black_crows native 1h b100-t0.25 F00 | frontier | 1 | 100.0 % | +0.175 | +0.100 | 0 | 0.17 | - |
| fib_ext reversed 5m b100-t2 F07 | main | 1 | 100.0 % | +0.170 | -1.659 | 0 | 0.17 | - |
| cdl_three_inside_up native 1h b100-t0.25 F00 | frontier | 1 | 100.0 % | +0.156 | +0.062 | 0 | 0.16 | - |
| cdl_three_black_crows native 1h b025-t0.25 F00 | frontier | 1 | 100.0 % | +0.153 | +0.056 | 0 | 0.15 | - |
| ema_stack reversed 1h b025-t1 F04 | main | 1 | 100.0 % | +0.129 | -0.743 | 0 | 0.13 | - |
| cdl_three_inside_up native 1h b025-t0.25 F00 | frontier | 1 | 100.0 % | +0.117 | -0.015 | 0 | 0.12 | - |
| cdl_falling_three_methods native 5m b025-trail F07 | main | 1 | 100.0 % | +0.078 | -0.204 | 0 | 0.08 | - |
| retest reversed 5m b025-t3 F03 | main | 1 | 100.0 % | +0.055 | -2.890 | 0 | 0.06 | - |
| retest reversed 5m b025-t3 F04 | main | 1 | 100.0 % | +0.055 | -2.890 | 0 | 0.06 | - |
| retest reversed 5m b025-t3 F07 | main | 1 | 100.0 % | +0.055 | -2.890 | 0 | 0.06 | - |


## Target hit in >= 80 % of trades

489 fixed-target variants hit their target in at least 80 % of trades (175 with at least 30 trades). The target-hit rate is what a chart reader counts as a win; the net win rate counts only trades that are positive after costs. Computed by `candlescope research --stage winrates` (descriptive, added after the results were seen; same trials, no selection).

| variant | trades | target-hit rate | net win rate | net 1x R | net 2x R | worst losing streak | largest single loss R |
|---|---|---|---|---|---|---|---|
| sr_break native 5m b100-t0.25 F00 | 10047 | 80.1 % | 16.4 % | -0.457 | -0.914 | 182 | -4.44 |
| tl_formed reversed 15m b100-t0.25 F00 | 2833 | 80.2 % | 17.0 % | -0.430 | -0.864 | 143 | -3.44 |
| ema_stack native 5m b100-t0.25 F00 | 2283 | 80.2 % | 53.0 % | -0.249 | -0.502 | 24 | -2.46 |
| pullback reversed 15m b100-t0.25 F00 | 2003 | 80.1 % | 38.9 % | -0.308 | -0.618 | 21 | -2.59 |
| tl_touch native 15m b100-t0.25 F00 | 1744 | 80.3 % | 15.8 % | -0.440 | -0.884 | 76 | -3.35 |
| bos reversed 15m b100-t0.25 F00 | 1739 | 81.4 % | 11.8 % | -0.455 | -0.928 | 215 | -3.47 |
| structure_trend native 15m b100-t0.25 F00 | 1714 | 80.2 % | 18.1 % | -0.417 | -0.836 | 104 | -3.37 |
| structure_label native 1h b025-t0.25 F00 | 1659 | 80.2 % | 15.3 % | -0.517 | -1.037 | 70 | -3.52 |
| tl_touch reversed 15m b100-t0.25 F00 | 1624 | 80.4 % | 24.7 % | -0.373 | -0.751 | 67 | -2.81 |
| fakeout native 15m b100-t0.25 F00 | 1614 | 80.4 % | 48.3 % | -0.252 | -0.509 | 23 | -2.49 |
| breakout native 15m b025-t0.25 F00 | 1508 | 80.4 % | 28.7 % | -0.341 | -0.687 | 59 | -4.22 |
| fib_leg native 15m b100-t0.25 F00 | 1450 | 80.4 % | 34.3 % | -0.530 | -1.065 | 20 | -6.36 |
| fib_touch native 15m b100-t0.25 F00 | 1442 | 80.2 % | 29.5 % | -0.581 | -1.164 | 22 | -6.36 |
| structure_label reversed 1h b100-t0.25 F00 | 1366 | 81.1 % | 62.9 % | -0.183 | -0.381 | 14 | -1.63 |
| fib_leg native 15m b025-t0.25 F00 | 1070 | 80.6 % | 31.3 % | -0.591 | -1.189 | 15 | -7.17 |
| fib_touch native 15m b025-t0.25 F00 | 1012 | 80.2 % | 24.0 % | -0.681 | -1.365 | 30 | -7.17 |
| sr_break native 1h b025-t0.25 F00 | 910 | 80.9 % | 66.8 % | -0.169 | -0.350 | 8 | -1.64 |
| cdl_bearish_engulfing reversed 15m b100-t0.25 F00 | 867 | 80.7 % | 10.8 % | -0.478 | -0.965 | 127 | -3.16 |
| cdl_bearish_engulfing native 15m b025-t0.25 F00 | 848 | 81.6 % | 15.6 % | -0.457 | -0.935 | 71 | -4.99 |
| cdl_bullish_engulfing reversed 15m b100-t0.25 F00 | 837 | 80.3 % | 11.2 % | -0.477 | -0.958 | 105 | -2.80 |
| cdl_hammer native 5m b100-t0.25 F00 | 833 | 80.3 % | 7.9 % | -0.634 | -1.271 | 111 | -4.81 |
| cdl_shooting_star native 5m b100-t0.25 F00 | 826 | 81.2 % | 5.8 % | -0.623 | -1.262 | 258 | -5.25 |
| ema_stack native 15m b100-t0.25 F00 | 816 | 80.2 % | 75.0 % | -0.119 | -0.241 | 11 | -1.34 |
| cdl_bearish_engulfing native 15m b100-t0.25 F00 | 778 | 81.5 % | 41.6 % | -0.259 | -0.538 | 19 | -1.90 |
| sr_break native 1h b100-t0.25 F00 | 752 | 81.1 % | 80.5 % | -0.098 | -0.211 | 3 | -1.23 |
| choch reversed 15m b100-t0.25 F00 | 743 | 83.0 % | 11.7 % | -0.438 | -0.914 | 166 | -3.23 |
| choch reversed 15m b025-t0.25 F00 | 743 | 80.9 % | 1.6 % | -1.380 | -2.771 | 207 | -9.87 |
| tl_formed native 1h b025-t0.25 F00 | 742 | 80.0 % | 16.6 % | -0.511 | -1.022 | 55 | -2.86 |
| choch native 15m b100-t0.25 F00 | 724 | 80.1 % | 39.8 % | -0.291 | -0.584 | 17 | -2.12 |
| cdl_dark_cloud_cover native 5m b025-t0.25 F00 | 717 | 81.2 % | 3.8 % | -0.886 | -1.788 | 221 | -6.70 |
| cdl_dark_cloud_cover native 5m b100-t0.25 F00 | 705 | 81.0 % | 12.6 % | -0.529 | -1.071 | 132 | -3.48 |
| ema_cross native 15m b100-t0.25 F00 | 701 | 80.2 % | 62.6 % | -0.195 | -0.394 | 9 | -2.13 |
| retest native 15m b025-t0.25 F00 | 668 | 81.6 % | 41.0 % | -0.255 | -0.531 | 29 | -2.36 |
| pullback reversed 1h b100-t0.25 F00 | 550 | 80.0 % | 73.3 % | -0.140 | -0.280 | 5 | -1.39 |
| cdl_bearish_harami native 5m b100-t0.25 F00 | 526 | 82.7 % | 5.3 % | -0.694 | -1.423 | 128 | -4.99 |
| fib_ext native 15m b025-t0.25 F00 | 522 | 83.9 % | 78.7 % | -0.070 | -0.188 | 5 | -1.59 |
| tl_break reversed 1h b025-t0.25 F00 | 520 | 82.1 % | 6.2 % | -0.672 | -1.371 | 107 | -3.65 |
| tl_break reversed 1h b100-t0.25 F00 | 511 | 80.2 % | 48.5 % | -0.238 | -0.479 | 20 | -1.53 |
| sweep native 15m b100-t0.25 F00 | 504 | 80.6 % | 41.9 % | -0.271 | -0.550 | 12 | -2.08 |
| tl_break native 1h b025-t0.25 F00 | 499 | 81.2 % | 40.5 % | -0.287 | -0.589 | 25 | -2.07 |
| cdl_bullish_harami reversed 5m b100-t0.25 F00 | 498 | 80.5 % | 9.6 % | -0.585 | -1.178 | 182 | -3.52 |
| cdl_tweezer_top reversed 15m b100-t0.25 F00 | 479 | 81.0 % | 17.8 % | -0.419 | -0.851 | 71 | -3.16 |
| h1_trend reversed 15m b100-t0.25 F00 | 476 | 81.3 % | 32.8 % | -0.311 | -0.639 | 21 | -2.26 |
| h1_trend reversed 5m b100-t0.25 F00 | 476 | 81.1 % | 21.9 % | -0.469 | -0.953 | 32 | -3.19 |
| structure_trend reversed 1h b025-t0.25 F00 | 476 | 80.5 % | 16.6 % | -0.492 | -0.990 | 32 | -2.71 |
| h1_trend reversed 15m b025-t0.25 F00 | 476 | 80.0 % | 14.1 % | -0.691 | -1.383 | 45 | -4.06 |
| h1_trend native 15m b025-t0.25 F00 | 475 | 80.6 % | 12.4 % | -0.732 | -1.472 | 34 | -8.29 |
| fib_ext native 15m b100-t0.25 F00 | 473 | 83.9 % | 81.6 % | -0.049 | -0.146 | 3 | -1.50 |
| structure_trend reversed 1h b100-t0.25 F00 | 470 | 81.3 % | 61.7 % | -0.182 | -0.383 | 7 | -1.55 |
| tl_break native 1h b100-t0.25 F00 | 452 | 80.1 % | 72.8 % | -0.154 | -0.310 | 5 | -1.45 |
| cdl_tweezer_bottom native 15m b100-t0.25 F00 | 452 | 81.0 % | 27.7 % | -0.353 | -0.718 | 33 | -3.13 |
| role_reversal native 1h b025-t0.25 F00 | 449 | 80.6 % | 65.0 % | -0.183 | -0.374 | 6 | -1.83 |
| cdl_three_outside_down reversed 15m b100-t0.25 F00 | 430 | 82.8 % | 14.2 % | -0.430 | -0.894 | 74 | -3.31 |
| cdl_three_outside_down reversed 15m b025-t0.25 F00 | 430 | 81.2 % | 1.2 % | -1.296 | -2.607 | 232 | -10.25 |
| tl_touch reversed 1h b025-t0.25 F00 | 421 | 83.1 % | 24.0 % | -0.396 | -0.830 | 24 | -2.50 |
| cdl_three_outside_up reversed 15m b100-t0.25 F00 | 415 | 82.4 % | 14.0 % | -0.406 | -0.843 | 107 | -2.01 |
| cdl_bearish_marubozu reversed 15m b100-t0.25 F00 | 408 | 82.1 % | 2.7 % | -0.667 | -1.361 | 238 | -3.38 |
| fakeout reversed 1h b100-t0.25 F00 | 393 | 81.2 % | 18.3 % | -0.397 | -0.808 | 71 | -2.62 |
| h1_stack reversed 5m b100-t0.25 F00 | 389 | 80.2 % | 15.2 % | -0.606 | -1.215 | 34 | -5.10 |
| tl_touch reversed 1h b100-t0.25 F00 | 388 | 82.5 % | 65.5 % | -0.164 | -0.358 | 9 | -1.51 |
| bos native 1h b100-t0.25 F00 | 386 | 80.0 % | 75.9 % | -0.126 | -0.254 | 4 | -1.41 |
| sr_formed native 1h b100-t0.25 F00 | 375 | 81.6 % | 78.9 % | -0.104 | -0.227 | 4 | -1.35 |
| h1_stack native 5m b100-t0.25 F00 | 375 | 80.0 % | 39.2 % | -0.336 | -0.671 | 15 | -2.33 |
| fib_leg native 1h b100-t0.25 F00 | 368 | 80.2 % | 53.5 % | -0.240 | -0.482 | 9 | -2.73 |
| fib_touch native 1h b100-t0.25 F00 | 368 | 81.2 % | 51.4 % | -0.251 | -0.517 | 14 | -2.73 |
| h1_stack native 15m b100-t0.25 F00 | 363 | 80.7 % | 47.1 % | -0.243 | -0.497 | 14 | -1.85 |
| fib_leg reversed 1h b025-t0.25 F00 | 362 | 82.9 % | 55.2 % | -0.225 | -0.485 | 5 | -2.52 |
| fib_touch reversed 1h b025-t0.25 F00 | 350 | 82.9 % | 49.7 % | -0.259 | -0.553 | 8 | -2.52 |
| cdl_gravestone_doji native 5m b100-t0.25 F00 | 331 | 80.7 % | 5.7 % | -0.655 | -1.319 | 142 | -5.25 |
| sweep native 1h b025-t0.25 F00 | 300 | 82.0 % | 48.3 % | -0.238 | -0.501 | 8 | -2.69 |
| cdl_bearish_harami_cross native 5m b100-t0.25 F00 | 286 | 82.5 % | 5.6 % | -0.829 | -1.690 | 112 | -9.97 |
| sweep native 1h b100-t0.25 F00 | 281 | 81.8 % | 76.5 % | -0.119 | -0.264 | 3 | -1.32 |
| fib_leg native 1h b025-t0.25 F00 | 266 | 82.7 % | 53.8 % | -0.244 | -0.521 | 6 | -3.11 |
| fib_touch native 1h b025-t0.25 F00 | 250 | 82.4 % | 49.6 % | -0.285 | -0.600 | 9 | -3.11 |
| cdl_dark_cloud_cover native 15m b025-t0.25 F00 | 240 | 81.2 % | 14.6 % | -0.468 | -0.953 | 34 | -3.12 |
| ema_stack native 1h b025-t0.25 F00 | 239 | 81.2 % | 76.1 % | -0.073 | -0.164 | 4 | -1.28 |
| cdl_bearish_engulfing reversed 1h b025-t0.25 F00 | 239 | 84.5 % | 5.9 % | -0.582 | -1.221 | 41 | -3.65 |
| cdl_bearish_engulfing reversed 1h b100-t0.25 F00 | 235 | 82.5 % | 54.9 % | -0.194 | -0.420 | 12 | -1.38 |
| cdl_dark_cloud_cover native 15m b100-t0.25 F00 | 235 | 84.7 % | 43.0 % | -0.229 | -0.518 | 17 | -2.04 |
| cdl_piercing_line native 15m b100-t0.25 F00 | 220 | 81.8 % | 39.6 % | -0.264 | -0.550 | 17 | -1.73 |
| cdl_hammer native 15m b025-t0.25 F00 | 219 | 81.3 % | 10.1 % | -0.546 | -1.107 | 45 | -2.34 |
| swing reversed 1h b025-t0.25 F00 | 217 | 80.2 % | 6.5 % | -0.717 | -1.436 | 60 | -4.52 |
| cdl_bullish_engulfing reversed 1h b025-t0.25 F00 | 213 | 83.6 % | 8.5 % | -0.600 | -1.245 | 85 | -4.37 |
| cdl_bullish_engulfing reversed 1h b100-t0.25 F00 | 210 | 81.9 % | 58.6 % | -0.198 | -0.421 | 7 | -1.41 |
| ema_stack native 1h b100-t0.25 F00 | 209 | 83.7 % | 83.7 % | -0.016 | -0.083 | 3 | -1.22 |
| choch reversed 1h b100-t0.25 F00 | 206 | 82.0 % | 52.9 % | -0.205 | -0.435 | 15 | -1.53 |
| choch reversed 1h b025-t0.25 F00 | 206 | 81.1 % | 7.3 % | -0.626 | -1.264 | 35 | -2.26 |
| choch native 1h b100-t0.25 F00 | 202 | 80.2 % | 74.8 % | -0.141 | -0.283 | 4 | -1.35 |
| ema_cross native 1h b025-t0.25 F00 | 200 | 82.5 % | 68.5 % | -0.118 | -0.266 | 5 | -1.81 |
| cdl_bearish_harami native 15m b025-t0.25 F00 | 195 | 80.5 % | 3.6 % | -0.795 | -1.597 | 115 | -4.71 |
| cdl_bearish_harami native 15m b100-t0.25 F00 | 194 | 80.9 % | 22.7 % | -0.379 | -0.769 | 25 | -2.54 |
| cdl_inverted_hammer native 15m b100-t0.25 F00 | 192 | 81.8 % | 7.8 % | -0.495 | -1.012 | 54 | -2.40 |
| cdl_bullish_engulfing native 1h b100-t0.25 F00 | 186 | 81.2 % | 78.0 % | -0.118 | -0.250 | 3 | -1.26 |
| cdl_evening_doji_star native 5m b100-t0.25 F00 | 168 | 81.0 % | 15.5 % | -0.464 | -0.943 | 38 | -2.67 |
| cdl_morning_star reversed 5m b100-t0.25 F00 | 168 | 81.0 % | 5.9 % | -0.759 | -1.531 | 56 | -4.26 |
| sd_test native 15m b025-t0.25 F00 | 167 | 82.6 % | 9.0 % | -0.797 | -1.627 | 30 | -4.67 |
| sd_test native 15m b100-t0.25 F00 | 163 | 84.0 % | 30.7 % | -0.326 | -0.702 | 13 | -2.83 |
| cdl_morning_doji_star native 5m b100-t0.25 F00 | 150 | 80.0 % | 13.3 % | -0.542 | -1.084 | 27 | -4.78 |
| cdl_morning_doji_star reversed 5m b100-t0.25 F00 | 150 | 80.0 % | 4.0 % | -0.835 | -1.673 | 54 | -6.05 |
| cdl_morning_doji_star native 5m b025-t0.25 F00 | 150 | 82.7 % | 4.0 % | -0.841 | -1.716 | 77 | -8.77 |
| cdl_bullish_harami native 15m b100-t0.25 F00 | 149 | 83.9 % | 24.8 % | -0.332 | -0.712 | 53 | -1.85 |
| cdl_bullish_harami reversed 15m b025-t0.25 F00 | 149 | 81.2 % | 12.1 % | -0.555 | -1.125 | 38 | -4.17 |
| cdl_bullish_harami native 15m b025-t0.25 F00 | 149 | 81.2 % | 5.4 % | -0.753 | -1.522 | 78 | -3.59 |
| eq_levels native 1h b025-t0.25 F00 | 141 | 85.8 % | 17.7 % | -0.378 | -0.829 | 19 | -2.02 |
| eq_levels native 1h b100-t0.25 F00 | 136 | 85.3 % | 72.8 % | -0.124 | -0.314 | 3 | -1.42 |
| cdl_evening_star native 5m b025-t0.25 F00 | 129 | 82.2 % | 5.4 % | -0.710 | -1.446 | 59 | -3.86 |
| fib_ext native 1h b025-t0.25 F00 | 127 | 80.3 % | 80.3 % | -0.052 | -0.107 | 3 | -1.26 |
| cdl_evening_star native 5m b100-t0.25 F00 | 127 | 81.9 % | 16.5 % | -0.450 | -0.924 | 47 | -2.86 |
| cdl_three_outside_down reversed 1h b025-t0.25 F00 | 127 | 83.5 % | 7.9 % | -0.546 | -1.135 | 59 | -2.18 |
| cdl_three_outside_down native 1h b025-t0.25 F00 | 123 | 81.3 % | 65.8 % | -0.165 | -0.348 | 3 | -1.41 |
| cdl_three_outside_down native 1h b100-t0.25 F00 | 120 | 83.3 % | 81.7 % | -0.075 | -0.195 | 2 | -1.22 |
| cdl_tweezer_top reversed 1h b025-t0.25 F00 | 119 | 81.5 % | 15.1 % | -0.503 | -1.025 | 18 | -2.40 |
| h4_trend reversed 1h b100-t0.25 F00 | 117 | 82.9 % | 75.2 % | -0.119 | -0.274 | 2 | -1.27 |
| h4_trend reversed 15m b100-t0.25 F00 | 117 | 82.0 % | 55.6 % | -0.211 | -0.449 | 6 | -1.85 |
| h4_trend reversed 5m b100-t0.25 F00 | 117 | 81.2 % | 43.6 % | -0.311 | -0.636 | 6 | -2.37 |
| h4_trend reversed 1h b025-t0.25 F00 | 117 | 81.2 % | 37.6 % | -0.320 | -0.655 | 8 | -1.93 |
| h4_trend reversed 15m b025-t0.25 F00 | 117 | 80.3 % | 36.8 % | -0.431 | -0.867 | 8 | -3.11 |
| h4_trend reversed 5m b025-t0.25 F00 | 117 | 80.3 % | 35.0 % | -0.524 | -1.053 | 8 | -4.51 |
| h4_trend native 5m b100-t0.25 F00 | 116 | 80.2 % | 41.4 % | -0.292 | -0.586 | 8 | -1.73 |
| h4_trend native 1h b025-t0.25 F00 | 116 | 80.2 % | 38.8 % | -0.307 | -0.617 | 8 | -1.75 |
| h4_trend native 15m b025-t0.25 F00 | 116 | 80.2 % | 36.2 % | -0.390 | -0.782 | 10 | -2.98 |
| cdl_tweezer_top reversed 1h b100-t0.25 F00 | 115 | 83.5 % | 60.0 % | -0.163 | -0.368 | 5 | -1.37 |
| cdl_three_outside_up reversed 1h b100-t0.25 F00 | 107 | 80.4 % | 57.0 % | -0.212 | -0.429 | 8 | -1.36 |
| cdl_three_outside_up native 1h b025-t0.25 F00 | 106 | 83.0 % | 74.5 % | -0.125 | -0.285 | 3 | -1.43 |
| ema_cross reversed 1h b100-t0.25 F00 | 105 | 81.9 % | 55.2 % | -0.233 | -0.489 | 6 | -1.57 |
| cdl_three_outside_up native 1h b100-t0.25 F00 | 100 | 83.0 % | 82.0 % | -0.073 | -0.182 | 4 | -1.24 |
| h4_stack reversed 1h b100-t0.25 F00 | 98 | 86.7 % | 70.4 % | -0.096 | -0.277 | 2 | -1.30 |
| h4_stack reversed 15m b100-t0.25 F00 | 98 | 84.7 % | 43.9 % | -0.219 | -0.497 | 8 | -1.46 |
| h4_stack reversed 5m b100-t0.25 F00 | 98 | 89.8 % | 35.7 % | -0.267 | -0.656 | 8 | -1.67 |
| h4_stack reversed 1h b025-t0.25 F00 | 98 | 86.7 % | 28.6 % | -0.344 | -0.773 | 14 | -1.70 |
| h4_stack reversed 15m b025-t0.25 F00 | 98 | 86.7 % | 25.5 % | -0.480 | -1.043 | 14 | -3.41 |
| h4_stack reversed 5m b025-t0.25 F00 | 98 | 85.7 % | 25.5 % | -0.632 | -1.336 | 14 | -5.20 |
| retest reversed 1h b100-t0.25 F00 | 97 | 83.5 % | 3.1 % | -0.564 | -1.172 | 73 | -2.09 |
| cdl_bearish_harami_cross native 15m b100-t0.25 F00 | 92 | 85.9 % | 22.8 % | -0.305 | -0.685 | 30 | -1.52 |
| cdl_tweezer_bottom reversed 1h b025-t0.25 F00 | 92 | 82.6 % | 18.5 % | -0.464 | -0.960 | 26 | -2.05 |
| cdl_bearish_harami_cross native 15m b025-t0.25 F00 | 92 | 84.8 % | 4.3 % | -0.782 | -1.623 | 52 | -3.23 |
| cdl_tweezer_bottom reversed 1h b100-t0.25 F00 | 90 | 80.0 % | 64.4 % | -0.196 | -0.392 | 4 | -1.32 |
| cdl_tweezer_bottom native 1h b100-t0.25 F00 | 86 | 81.4 % | 72.1 % | -0.139 | -0.293 | 4 | -1.31 |
| cdl_three_white_soldiers native 5m b100-t0.25 F00 | 80 | 81.2 % | 27.5 % | -0.395 | -0.804 | 13 | -1.83 |
| cdl_bullish_harami_cross reversed 15m b100-t0.25 F00 | 77 | 81.8 % | 46.8 % | -0.253 | -0.529 | 11 | -1.54 |
| cdl_bullish_harami_cross reversed 15m b025-t0.25 F00 | 77 | 80.5 % | 15.6 % | -0.443 | -0.893 | 20 | -2.01 |
| cdl_gravestone_doji native 15m b025-t0.25 F00 | 72 | 81.9 % | 5.6 % | -0.625 | -1.274 | 47 | -2.64 |
| cdl_dragonfly_doji native 15m b025-t0.25 F00 | 71 | 87.3 % | 9.9 % | -0.586 | -1.262 | 27 | -2.34 |
| cdl_three_black_crows reversed 5m b100-t0.25 F00 | 71 | 80.3 % | 0.0 % | -1.587 | -3.177 | 71 | -5.79 |
| cdl_dragonfly_doji native 15m b100-t0.25 F00 | 69 | 85.5 % | 30.4 % | -0.315 | -0.698 | 16 | -1.71 |
| cdl_bullish_marubozu reversed 1h b025-t0.25 F00 | 61 | 83.6 % | 0.0 % | -0.997 | -2.039 | 61 | -2.73 |
| cdl_piercing_line native 1h b025-t0.25 F00 | 59 | 81.4 % | 61.0 % | -0.212 | -0.441 | 3 | -1.56 |
| cdl_piercing_line native 1h b100-t0.25 F00 | 58 | 84.5 % | 75.9 % | -0.086 | -0.228 | 3 | -1.31 |
| cdl_shooting_star native 1h b100-t0.25 F00 | 55 | 80.0 % | 76.4 % | -0.153 | -0.308 | 2 | -1.20 |
| cdl_dark_cloud_cover reversed 1h b100-t0.25 F00 | 53 | 81.1 % | 64.1 % | -0.185 | -0.382 | 4 | -1.35 |
| fakeout reversed 1h b025-t0.25 F00 | 53 | 81.1 % | 0.0 % | -1.149 | -2.312 | 53 | -2.82 |
| cdl_hanging_man reversed 1h b100-t0.25 F00 | 49 | 83.7 % | 75.5 % | -0.113 | -0.270 | 2 | -1.45 |
| cdl_bearish_harami native 1h b100-t0.25 F00 | 48 | 85.4 % | 75.0 % | -0.091 | -0.251 | 2 | -1.18 |
| cdl_evening_doji_star native 15m b100-t0.25 F00 | 46 | 82.6 % | 32.6 % | -0.238 | -0.510 | 5 | -1.31 |
| cdl_evening_doji_star native 15m b025-t0.25 F00 | 46 | 87.0 % | 13.0 % | -0.349 | -0.786 | 16 | -1.52 |
| cdl_bearish_marubozu reversed 1h b025-t0.25 F00 | 45 | 86.7 % | 0.0 % | -1.176 | -2.435 | 45 | -3.40 |
| cdl_bearish_marubozu reversed 1h b100-t0.25 F00 | 44 | 81.8 % | 29.5 % | -0.309 | -0.640 | 22 | -1.39 |
| cdl_bearish_marubozu native 1h b100-t0.25 F00 | 43 | 83.7 % | 74.4 % | -0.122 | -0.293 | 3 | -1.45 |
| sd_test native 1h b100-t0.25 F00 | 41 | 85.4 % | 75.6 % | -0.104 | -0.275 | 2 | -1.20 |
| cdl_morning_doji_star native 15m b100-t0.25 F00 | 41 | 90.2 % | 48.8 % | -0.137 | -0.399 | 6 | -1.24 |
| cdl_morning_doji_star native 15m b025-t0.25 F00 | 41 | 85.4 % | 29.3 % | -0.349 | -0.763 | 10 | -1.55 |
| cdl_morning_star native 15m b100-t0.25 F00 | 40 | 90.0 % | 45.0 % | -0.154 | -0.432 | 7 | -1.43 |
| cdl_morning_star native 15m b025-t0.25 F00 | 40 | 85.0 % | 30.0 % | -0.375 | -0.811 | 7 | -1.94 |
| cdl_evening_star reversed 15m b100-t0.25 F00 | 40 | 80.0 % | 10.0 % | -0.404 | -0.807 | 27 | -2.25 |
| cdl_evening_star reversed 15m b025-t0.25 F00 | 40 | 85.0 % | 2.5 % | -0.820 | -1.702 | 39 | -4.21 |
| cdl_evening_star native 15m b100-t0.25 F00 | 39 | 84.6 % | 38.5 % | -0.222 | -0.504 | 11 | -1.44 |
| cdl_hammer reversed 1h b025-t0.25 F00 | 38 | 86.8 % | 0.0 % | -0.695 | -1.476 | 38 | -2.01 |
| cdl_inverted_hammer native 1h b100-t0.25 F00 | 33 | 87.9 % | 27.3 % | -0.189 | -0.474 | 5 | -1.30 |
| cdl_three_inside_down native 15m b100-t0.25 F00 | 33 | 84.9 % | 54.5 % | -0.209 | -0.481 | 4 | -1.47 |
| cdl_three_inside_down reversed 15m b100-t0.25 F00 | 33 | 87.9 % | 3.0 % | -0.454 | -1.005 | 20 | -1.64 |
| cdl_inverted_hammer native 1h b025-t0.5 F00 | 33 | 81.8 % | 6.1 % | -0.716 | -1.657 | 28 | -2.42 |
| cdl_inverted_hammer native 1h b025-t0.25 F00 | 33 | 87.9 % | 0.0 % | -0.842 | -1.783 | 33 | -2.42 |
| cdl_three_inside_down reversed 15m b025-t0.25 F00 | 33 | 81.8 % | 3.0 % | -1.532 | -3.086 | 31 | -3.84 |
| sd_formed native 1h b025-t0.25 F00 | 30 | 93.3 % | 93.3 % | +0.081 | -0.005 | 1 | -1.16 |
| cdl_inverted_hammer reversed 1h b100-t0.25 F00 | 30 | 83.3 % | 76.7 % | -0.131 | -0.305 | 3 | -1.29 |
| sd_formed native 1h b100-t0.25 F00 | 29 | 93.1 % | 93.1 % | +0.095 | +0.027 | 1 | -1.13 |
| fib_ext reversed 5m b100-t0.25 F00 | 29 | 82.8 % | 0.0 % | -1.915 | -3.864 | 29 | -7.64 |
| sd_broken native 1h b100-t0.25 F00 | 28 | 85.7 % | 78.6 % | -0.069 | -0.210 | 3 | -1.24 |
| sd_broken native 1h b025-t0.25 F00 | 28 | 85.7 % | 50.0 % | -0.197 | -0.466 | 6 | -1.50 |
| cdl_three_inside_up native 15m b100-t0.25 F00 | 27 | 92.6 % | 66.7 % | -0.064 | -0.285 | 2 | -1.38 |
| cdl_bullish_harami native 1h b100-t0.25 F00 | 27 | 85.2 % | 74.1 % | -0.103 | -0.271 | 1 | -1.34 |
| cdl_bullish_harami reversed 1h b025-t0.25 F00 | 27 | 81.5 % | 40.7 % | -0.220 | -0.459 | 6 | -1.35 |
| cdl_three_inside_up native 15m b025-t0.25 F00 | 27 | 85.2 % | 33.3 % | -0.261 | -0.586 | 5 | -1.38 |
| cdl_three_inside_up reversed 15m b025-t0.25 F00 | 27 | 88.9 % | 3.7 % | -1.174 | -2.459 | 17 | -2.90 |
| cdl_three_black_crows reversed 5m b025-t1 F04 | 22 | 81.8 % | 0.0 % | -7.080 | -14.797 | 22 | -22.84 |
| sd_broken reversed 1h b100-t0.25 F00 | 20 | 85.0 % | 15.0 % | -0.288 | -0.640 | 10 | -1.26 |
| sd_test reversed 1h b025-t0.25 F00 | 20 | 80.0 % | 0.0 % | -0.766 | -1.533 | 20 | -1.96 |
| cdl_dragonfly_doji reversed 1h b100-t0.5 F00 | 18 | 83.3 % | 83.3 % | +0.017 | -0.218 | 1 | -1.25 |
| cdl_dragonfly_doji reversed 1h b100-t0.25 F00 | 18 | 88.9 % | 50.0 % | -0.123 | -0.359 | 3 | -1.22 |
| cdl_dragonfly_doji reversed 1h b025-t0.25 F00 | 18 | 94.4 % | 0.0 % | -0.617 | -1.415 | 18 | -1.71 |
| cdl_bearish_harami_cross native 1h b100-t0.5 F00 | 17 | 82.3 % | 82.3 % | +0.009 | -0.219 | 1 | -1.34 |
| cdl_bearish_harami_cross native 1h b100-t0.25 F00 | 17 | 94.1 % | 64.7 % | -0.051 | -0.281 | 2 | -1.21 |
| cdl_morning_star reversed 1h b100-t0.25 F00 | 17 | 82.3 % | 64.7 % | -0.142 | -0.314 | 2 | -1.24 |
| cdl_bearish_harami_cross reversed 1h b100-t0.25 F00 | 17 | 82.3 % | 70.6 % | -0.143 | -0.313 | 2 | -1.26 |
| cdl_bearish_harami_cross native 1h b025-t0.5 F00 | 17 | 82.3 % | 47.1 % | -0.247 | -0.732 | 2 | -1.45 |
| cdl_bearish_harami_cross native 1h b025-t0.25 F00 | 17 | 82.3 % | 5.9 % | -0.455 | -0.939 | 16 | -1.45 |
| cdl_bearish_harami native 1h b100-t1 F08 | 15 | 86.7 % | 86.7 % | +0.560 | +0.384 | 1 | -1.18 |
| cdl_bearish_harami native 1h b100-t1 F06 | 15 | 80.0 % | 80.0 % | +0.458 | +0.306 | 2 | -1.18 |
| cdl_three_white_soldiers reversed 15m b100-t0.5 F00 | 14 | 85.7 % | 42.9 % | -0.249 | -0.785 | 4 | -1.74 |
| cdl_three_white_soldiers reversed 15m b100-t0.25 F00 | 14 | 92.9 % | 7.1 % | -0.375 | -0.910 | 13 | -1.74 |
| cdl_bullish_harami_cross native 1h b100-t0.75 F00 | 13 | 92.3 % | 92.3 % | +0.437 | +0.263 | 1 | -1.20 |
| cdl_bullish_harami_cross native 1h b100-t0.5 F00 | 13 | 100.0 % | 100.0 % | +0.324 | +0.150 | 0 | 0.15 |
| cdl_bullish_harami_cross native 1h b100-t0.25 F00 | 13 | 100.0 % | 92.3 % | +0.074 | -0.100 | 1 | -0.10 |
| cdl_evening_star native 1h b100-t0.25 F00 | 13 | 84.6 % | 84.6 % | -0.071 | -0.202 | 1 | -1.19 |
| cdl_bullish_harami_cross native 1h b025-t0.5 F00 | 13 | 84.6 % | 76.9 % | -0.089 | -0.444 | 1 | -1.58 |
| cdl_evening_star native 1h b025-t0.25 F00 | 13 | 84.6 % | 61.5 % | -0.141 | -0.342 | 2 | -1.27 |
| cdl_evening_star reversed 1h b100-t0.25 F00 | 13 | 84.6 % | 53.8 % | -0.171 | -0.396 | 4 | -1.34 |
| cdl_bullish_harami_cross native 1h b025-t0.25 F00 | 13 | 92.3 % | 23.1 % | -0.205 | -0.560 | 9 | -1.24 |
| cdl_evening_star reversed 1h b025-t0.5 F00 | 13 | 84.6 % | 38.5 % | -0.265 | -0.796 | 3 | -1.36 |
| cdl_evening_star reversed 1h b025-t0.25 F00 | 13 | 100.0 % | 0.0 % | -0.282 | -0.814 | 13 | -0.92 |
| cdl_bearish_marubozu reversed 1h b100-t1 F02 | 12 | 83.3 % | 83.3 % | +0.325 | -0.006 | 1 | -1.39 |
| cdl_morning_star native 15m b100-t1 F08 | 12 | 83.3 % | 83.3 % | +0.253 | -0.158 | 1 | -1.53 |
| cdl_bearish_harami_cross native 15m b100-t1 F02 | 11 | 81.8 % | 81.8 % | +0.253 | -0.135 | 1 | -1.95 |
| cdl_morning_doji_star reversed 1h b025-t0.5 F00 | 11 | 81.8 % | 63.6 % | -0.255 | -0.737 | 2 | -1.69 |
| cdl_morning_doji_star reversed 1h b025-t0.25 F00 | 11 | 81.8 % | 18.2 % | -0.460 | -0.943 | 4 | -1.69 |
| cdl_bullish_harami native 15m b025-t1 F05 | 10 | 80.0 % | 70.0 % | -0.259 | -1.117 | 1 | -2.00 |
| cdl_three_inside_down native 5m b025-t1 F07 | 9 | 88.9 % | 66.7 % | +0.086 | -0.606 | 1 | -1.81 |
| cdl_three_inside_down reversed 15m b025-t1 F06 | 8 | 100.0 % | 12.5 % | -0.325 | -1.651 | 5 | -0.94 |
| cdl_piercing_line reversed 1h b025-t1 F06 | 7 | 85.7 % | 85.7 % | +0.277 | -0.161 | 1 | -1.29 |
| cdl_bearish_harami native 15m b025-t1 F05 | 7 | 85.7 % | 85.7 % | +0.148 | -0.418 | 1 | -1.35 |
| cdl_three_black_crows native 15m b025-t0.5 F00 | 7 | 85.7 % | 57.1 % | -0.205 | -0.701 | 3 | -1.98 |
| cdl_three_black_crows native 15m b100-t0.25 F00 | 7 | 85.7 % | 57.1 % | -0.293 | -0.661 | 3 | -1.75 |
| cdl_three_black_crows native 15m b025-t0.25 F00 | 7 | 85.7 % | 42.9 % | -0.420 | -0.915 | 4 | -1.98 |
| cdl_three_white_soldiers reversed 15m b025-t1 F07 | 7 | 85.7 % | 28.6 % | -0.853 | -2.421 | 5 | -2.54 |
| h4_stack reversed 1h b100-t3 F07 | 6 | 83.3 % | 83.3 % | +2.166 | +1.980 | 1 | -1.24 |
| cdl_three_white_soldiers reversed 15m b100-t3 F03 | 6 | 83.3 % | 83.3 % | +1.788 | +1.241 | 1 | -1.50 |
| cdl_three_white_soldiers reversed 15m b100-t3 F08 | 6 | 83.3 % | 83.3 % | +1.702 | +1.069 | 1 | -1.79 |
| cdl_morning_doji_star native 1h b100-t2 F09 | 6 | 83.3 % | 83.3 % | +1.355 | +1.224 | 1 | -1.17 |
| cdl_bearish_harami_cross reversed 1h b100-t2 F04 | 6 | 83.3 % | 83.3 % | +1.331 | +1.183 | 1 | -1.12 |
| h4_stack reversed 1h b100-t2 F07 | 6 | 83.3 % | 83.3 % | +1.330 | +1.144 | 1 | -1.24 |
| cdl_three_white_soldiers reversed 15m b100-t2 F03 | 6 | 83.3 % | 83.3 % | +0.952 | +0.404 | 1 | -1.50 |
| cdl_three_white_soldiers reversed 15m b100-t2 F08 | 6 | 83.3 % | 83.3 % | +0.867 | +0.233 | 1 | -1.79 |
| h4_stack reversed 1h b100-t1 F07 | 6 | 100.0 % | 100.0 % | +0.818 | +0.633 | 0 | 0.71 |
| cdl_morning_doji_star native 1h b100-t1 F09 | 6 | 83.3 % | 83.3 % | +0.527 | +0.397 | 1 | -1.17 |
| cdl_bearish_harami_cross reversed 1h b100-t1 F04 | 6 | 83.3 % | 83.3 % | +0.507 | +0.359 | 1 | -1.12 |
| cdl_bullish_harami reversed 1h b100-t1 F08 | 6 | 83.3 % | 83.3 % | +0.494 | +0.320 | 1 | -1.10 |
| cdl_three_white_soldiers reversed 15m b100-t1 F03 | 6 | 100.0 % | 100.0 % | +0.450 | -0.100 | 0 | 0.16 |
| cdl_gravestone_doji native 15m b100-t1 F01 | 6 | 83.3 % | 83.3 % | +0.329 | -0.012 | 1 | -1.18 |
| cdl_three_white_soldiers reversed 15m b100-t1 F09 | 6 | 83.3 % | 83.3 % | +0.102 | -0.463 | 1 | -1.79 |
| cdl_three_white_soldiers reversed 15m b100-t1 F08 | 6 | 83.3 % | 83.3 % | +0.031 | -0.604 | 1 | -1.79 |
| cdl_three_white_soldiers reversed 15m b025-t2 F09 | 6 | 83.3 % | 50.0 % | -0.625 | -2.751 | 2 | -2.78 |
| fib_ext reversed 5m b100-t1 F03 | 6 | 83.3 % | 50.0 % | -0.858 | -2.384 | 3 | -3.61 |
| cdl_three_white_soldiers reversed 15m b025-t1 F09 | 6 | 83.3 % | 16.7 % | -1.460 | -3.586 | 3 | -2.78 |
| cdl_dragonfly_doji reversed 1h b025-t3 F08 | 5 | 80.0 % | 80.0 % | +1.220 | +0.239 | 1 | -2.62 |
| cdl_bullish_harami_cross native 1h b100-t1 F04 | 5 | 100.0 % | 100.0 % | +0.789 | +0.585 | 0 | 0.65 |
| cdl_dragonfly_doji reversed 1h b100-t1 F08 | 5 | 100.0 % | 100.0 % | +0.713 | +0.426 | 0 | 0.58 |
| cdl_morning_star native 1h b100-t1 F04 | 5 | 80.0 % | 80.0 % | +0.432 | +0.267 | 1 | -1.17 |
| cdl_dragonfly_doji reversed 1h b025-t2 F08 | 5 | 80.0 % | 80.0 % | +0.418 | -0.564 | 1 | -2.62 |
| cdl_dark_cloud_cover native 1h b025-t1 F02 | 5 | 80.0 % | 80.0 % | +0.379 | +0.154 | 1 | -1.32 |
| cdl_bearish_harami_cross native 1h b100-t1 F06 | 5 | 80.0 % | 80.0 % | +0.363 | +0.114 | 1 | -1.27 |
| cdl_morning_star native 1h b025-t1 F04 | 5 | 80.0 % | 80.0 % | +0.337 | +0.077 | 1 | -1.30 |
| cdl_dark_cloud_cover native 1h b025-t1 F07 | 5 | 80.0 % | 80.0 % | +0.258 | -0.094 | 1 | -1.45 |
| cdl_evening_doji_star reversed 1h b025-t1 F04 | 5 | 80.0 % | 80.0 % | +0.234 | -0.126 | 1 | -1.27 |
| cdl_inverted_hammer reversed 1h b025-t1 F06 | 5 | 80.0 % | 80.0 % | +0.225 | -0.160 | 1 | -1.56 |
| cdl_bullish_harami_cross native 15m b025-t2 F01 | 5 | 80.0 % | 80.0 % | +0.193 | -1.014 | 1 | -2.93 |
| cdl_bullish_harami_cross native 15m b025-t2 F05 | 5 | 80.0 % | 80.0 % | +0.193 | -1.014 | 1 | -2.93 |
| cdl_bullish_harami_cross native 1h b025-t1 F04 | 5 | 80.0 % | 80.0 % | +0.180 | -0.230 | 1 | -1.24 |
| cdl_dragonfly_doji reversed 1h b025-t1 F08 | 5 | 100.0 % | 60.0 % | +0.018 | -0.965 | 2 | -0.62 |
| cdl_bullish_harami_cross native 15m b025-t1 F01 | 5 | 100.0 % | 40.0 % | -0.207 | -1.415 | 1 | -0.93 |
| cdl_bullish_harami_cross native 15m b025-t1 F05 | 5 | 100.0 % | 40.0 % | -0.207 | -1.415 | 1 | -0.93 |
| cdl_morning_star reversed 1h b025-t2 F02 | 4 | 100.0 % | 100.0 % | +1.790 | +1.581 | 0 | 1.69 |
| cdl_hammer reversed 1h b025-t2 F02 | 4 | 100.0 % | 100.0 % | +1.413 | +0.827 | 0 | 1.26 |
| cdl_morning_star reversed 1h b100-t1 F02 | 4 | 100.0 % | 100.0 % | +0.889 | +0.777 | 0 | 0.84 |
| cdl_evening_star native 15m b100-t1 F07 | 4 | 100.0 % | 100.0 % | +0.801 | +0.598 | 0 | 0.69 |
| cdl_morning_star reversed 1h b025-t1 F02 | 4 | 100.0 % | 100.0 % | +0.790 | +0.579 | 0 | 0.69 |
| cdl_evening_star native 15m b025-t1 F07 | 4 | 100.0 % | 100.0 % | +0.660 | +0.314 | 0 | 0.47 |
| cdl_hammer reversed 1h b025-t1 F02 | 4 | 100.0 % | 100.0 % | +0.413 | -0.175 | 0 | 0.26 |
| cdl_rising_three_methods reversed 5m b100-t1 F00 | 4 | 100.0 % | 25.0 % | -0.665 | -2.331 | 3 | -1.74 |
| cdl_rising_three_methods reversed 5m b100-t0.75 F00 | 4 | 100.0 % | 25.0 % | -0.915 | -2.581 | 3 | -1.99 |
| cdl_rising_three_methods reversed 5m b100-t0.5 F00 | 4 | 100.0 % | 0.0 % | -1.166 | -2.832 | 4 | -2.24 |
| cdl_rising_three_methods reversed 5m b100-t0.25 F00 | 4 | 100.0 % | 0.0 % | -1.416 | -3.083 | 4 | -2.49 |
| sd_broken reversed 1h b100-t2 F06 | 3 | 100.0 % | 100.0 % | +1.554 | +1.115 | 0 | 1.44 |
| cdl_gravestone_doji native 15m b025-t2 F05 | 3 | 100.0 % | 100.0 % | +1.547 | +1.088 | 0 | 1.40 |
| cdl_gravestone_doji native 15m b100-t1 F05 | 3 | 100.0 % | 100.0 % | +0.745 | +0.486 | 0 | 0.68 |
| cdl_three_inside_up native 15m b025-t1 F07 | 3 | 100.0 % | 100.0 % | +0.601 | +0.207 | 0 | 0.49 |
| cdl_evening_doji_star reversed 1h b025-t1 F02 | 3 | 100.0 % | 100.0 % | +0.596 | +0.199 | 0 | 0.46 |
| sd_broken reversed 1h b100-t1 F06 | 3 | 100.0 % | 100.0 % | +0.560 | +0.120 | 0 | 0.46 |
| cdl_gravestone_doji native 15m b025-t1 F05 | 3 | 100.0 % | 100.0 % | +0.540 | +0.080 | 0 | 0.40 |
| fib_ext reversed 5m b100-t2 F06 | 3 | 100.0 % | 66.7 % | +0.214 | -1.571 | 1 | -1.77 |
| fib_ext reversed 5m b100-t1 F06 | 3 | 100.0 % | 66.7 % | -0.785 | -2.570 | 1 | -2.77 |
| cdl_gravestone_doji native 1h b100-t3 F07 | 2 | 100.0 % | 100.0 % | +2.901 | +2.763 | 0 | 2.86 |
| cdl_three_white_soldiers native 1h b100-t3 F00 | 2 | 100.0 % | 100.0 % | +2.891 | +2.789 | 0 | 2.87 |
| cdl_three_white_soldiers native 1h b025-t3 F00 | 2 | 100.0 % | 100.0 % | +2.860 | +2.724 | 0 | 2.83 |
| cdl_three_black_crows native 15m b100-t3 F02 | 2 | 100.0 % | 100.0 % | +2.856 | +2.712 | 0 | 2.85 |
| cdl_three_inside_down native 1h b025-t3 F04 | 2 | 100.0 % | 100.0 % | +2.728 | +2.412 | 0 | 2.65 |
| cdl_bullish_harami_cross native 1h b100-t3 F02 | 2 | 100.0 % | 100.0 % | +2.695 | +2.419 | 0 | 2.65 |
| fib_ext reversed 1h b100-t3 F00 | 2 | 100.0 % | 100.0 % | +2.507 | +2.014 | 0 | 2.38 |
| fib_ext reversed 1h b100-t3 F06 | 2 | 100.0 % | 100.0 % | +2.507 | +2.014 | 0 | 2.38 |
| cdl_dragonfly_doji reversed 1h b025-t3 F02 | 2 | 100.0 % | 100.0 % | +2.426 | +1.853 | 0 | 2.26 |
| cdl_bullish_harami_cross native 1h b025-t3 F02 | 2 | 100.0 % | 100.0 % | +2.405 | +1.834 | 0 | 2.26 |
| cdl_three_white_soldiers native 1h b100-t2 F00 | 2 | 100.0 % | 100.0 % | +1.897 | +1.797 | 0 | 1.87 |
| cdl_gravestone_doji native 1h b100-t2 F07 | 2 | 100.0 % | 100.0 % | +1.879 | +1.742 | 0 | 1.82 |
| cdl_three_white_soldiers native 1h b025-t2 F00 | 2 | 100.0 % | 100.0 % | +1.867 | +1.730 | 0 | 1.83 |
| cdl_three_black_crows native 15m b100-t2 F02 | 2 | 100.0 % | 100.0 % | +1.858 | +1.712 | 0 | 1.85 |
| cdl_dragonfly_doji reversed 1h b100-t2 F02 | 2 | 100.0 % | 100.0 % | +1.835 | +1.669 | 0 | 1.77 |
| cdl_three_inside_down native 1h b100-t2 F04 | 2 | 100.0 % | 100.0 % | +1.821 | +1.614 | 0 | 1.78 |
| cdl_morning_star native 1h b025-t2 F02 | 2 | 100.0 % | 100.0 % | +1.716 | +1.449 | 0 | 1.67 |
| cdl_bullish_harami_cross native 1h b100-t2 F02 | 2 | 100.0 % | 100.0 % | +1.707 | +1.431 | 0 | 1.65 |
| cdl_three_inside_down native 1h b025-t2 F04 | 2 | 100.0 % | 100.0 % | +1.701 | +1.387 | 0 | 1.65 |
| fib_ext reversed 1h b100-t2 F00 | 2 | 100.0 % | 100.0 % | +1.506 | +1.011 | 0 | 1.38 |
| fib_ext reversed 1h b100-t2 F06 | 2 | 100.0 % | 100.0 % | +1.506 | +1.011 | 0 | 1.38 |
| cdl_dragonfly_doji reversed 1h b025-t2 F02 | 2 | 100.0 % | 100.0 % | +1.426 | +0.852 | 0 | 1.26 |
| cdl_bullish_harami_cross native 1h b025-t2 F02 | 2 | 100.0 % | 100.0 % | +1.406 | +0.837 | 0 | 1.26 |
| cdl_three_white_soldiers native 1h b100-t1.5 F00 | 2 | 100.0 % | 100.0 % | +1.401 | +1.301 | 0 | 1.38 |
| cdl_three_white_soldiers native 1h b025-t1.5 F00 | 2 | 100.0 % | 100.0 % | +1.363 | +1.226 | 0 | 1.34 |
| fib_ext reversed 1h b100-t1.5 F00 | 2 | 100.0 % | 100.0 % | +1.006 | +0.512 | 0 | 0.88 |
| cdl_three_white_soldiers native 1h b100-t1 F00 | 2 | 100.0 % | 100.0 % | +0.899 | +0.798 | 0 | 0.88 |
| cdl_gravestone_doji native 1h b100-t1 F07 | 2 | 100.0 % | 100.0 % | +0.867 | +0.726 | 0 | 0.80 |
| cdl_three_white_soldiers native 1h b025-t1 F00 | 2 | 100.0 % | 100.0 % | +0.863 | +0.727 | 0 | 0.84 |
| cdl_three_black_crows native 15m b100-t1 F02 | 2 | 100.0 % | 100.0 % | +0.858 | +0.714 | 0 | 0.86 |
| cdl_dragonfly_doji reversed 1h b100-t1 F02 | 2 | 100.0 % | 100.0 % | +0.828 | +0.656 | 0 | 0.77 |
| cdl_morning_star native 1h b100-t1 F02 | 2 | 100.0 % | 100.0 % | +0.824 | +0.651 | 0 | 0.78 |
| cdl_three_inside_down native 1h b100-t1 F04 | 2 | 100.0 % | 100.0 % | +0.800 | +0.595 | 0 | 0.77 |
| cdl_morning_star native 1h b025-t1 F02 | 2 | 100.0 % | 100.0 % | +0.729 | +0.462 | 0 | 0.67 |
| cdl_bullish_harami_cross native 1h b100-t1 F02 | 2 | 100.0 % | 100.0 % | +0.715 | +0.442 | 0 | 0.65 |
| cdl_three_inside_down native 1h b025-t1 F04 | 2 | 100.0 % | 100.0 % | +0.696 | +0.383 | 0 | 0.64 |
| cdl_three_white_soldiers native 1h b100-t0.75 F00 | 2 | 100.0 % | 100.0 % | +0.649 | +0.548 | 0 | 0.63 |
| cdl_three_white_soldiers native 1h b025-t0.75 F00 | 2 | 100.0 % | 100.0 % | +0.615 | +0.479 | 0 | 0.59 |
| cdl_falling_three_methods native 5m b100-t0.75 F00 | 2 | 100.0 % | 100.0 % | +0.544 | +0.339 | 0 | 0.52 |
| fib_ext reversed 1h b100-t1 F00 | 2 | 100.0 % | 100.0 % | +0.506 | +0.011 | 0 | 0.38 |
| fib_ext reversed 1h b100-t1 F06 | 2 | 100.0 % | 100.0 % | +0.506 | +0.011 | 0 | 0.38 |
| cdl_falling_three_methods native 5m b025-t0.75 F00 | 2 | 100.0 % | 100.0 % | +0.479 | +0.208 | 0 | 0.47 |
| cdl_dragonfly_doji reversed 1h b025-t1 F02 | 2 | 100.0 % | 100.0 % | +0.425 | -0.149 | 0 | 0.26 |
| cdl_three_inside_up reversed 15m b100-t1 F05 | 2 | 100.0 % | 100.0 % | +0.421 | -0.178 | 0 | 0.34 |
| cdl_bullish_harami_cross native 1h b025-t1 F02 | 2 | 100.0 % | 100.0 % | +0.407 | -0.162 | 0 | 0.26 |
| cdl_three_white_soldiers native 1h b100-t0.5 F00 | 2 | 100.0 % | 100.0 % | +0.400 | +0.299 | 0 | 0.38 |
| cdl_three_white_soldiers native 1h b025-t0.5 F00 | 2 | 100.0 % | 100.0 % | +0.365 | +0.228 | 0 | 0.34 |
| cdl_falling_three_methods native 5m b100-t0.5 F00 | 2 | 100.0 % | 100.0 % | +0.294 | +0.087 | 0 | 0.27 |
| fib_ext reversed 1h b100-t0.75 F00 | 2 | 100.0 % | 100.0 % | +0.255 | -0.241 | 0 | 0.13 |
| cdl_falling_three_methods native 5m b025-t0.5 F00 | 2 | 100.0 % | 100.0 % | +0.229 | -0.041 | 0 | 0.22 |
| cdl_three_white_soldiers native 1h b100-t0.25 F00 | 2 | 100.0 % | 100.0 % | +0.149 | +0.049 | 0 | 0.13 |
| cdl_three_white_soldiers native 1h b025-t0.25 F00 | 2 | 100.0 % | 100.0 % | +0.113 | -0.023 | 0 | 0.09 |
| cdl_rising_three_methods native 15m b100-t0.25 F00 | 2 | 100.0 % | 50.0 % | +0.052 | -0.146 | 1 | -0.03 |
| cdl_falling_three_methods native 5m b100-t0.25 F00 | 2 | 100.0 % | 100.0 % | +0.044 | -0.163 | 0 | 0.02 |
| fib_ext reversed 1h b100-t0.5 F00 | 2 | 100.0 % | 50.0 % | +0.004 | -0.491 | 1 | -0.12 |
| cdl_falling_three_methods native 5m b025-t0.25 F00 | 2 | 100.0 % | 0.0 % | -0.021 | -0.292 | 2 | -0.03 |
| cdl_rising_three_methods native 15m b025-t0.25 F00 | 2 | 100.0 % | 50.0 % | -0.022 | -0.294 | 1 | -0.12 |
| cdl_rising_three_methods reversed 15m b100-t0.25 F00 | 2 | 100.0 % | 0.0 % | -0.145 | -0.539 | 2 | -0.26 |
| cdl_falling_three_methods reversed 5m b100-t0.25 F00 | 2 | 100.0 % | 0.0 % | -0.206 | -0.663 | 2 | -0.24 |
| fib_ext reversed 1h b100-t0.25 F00 | 2 | 100.0 % | 0.0 % | -0.246 | -0.742 | 2 | -0.37 |
| cdl_rising_three_methods reversed 15m b025-t0.5 F00 | 2 | 100.0 % | 0.0 % | -0.424 | -1.348 | 2 | -0.51 |
| cdl_three_white_soldiers reversed 1h b025-t0.5 F00 | 2 | 100.0 % | 0.0 % | -0.608 | -1.716 | 2 | -0.88 |
| cdl_rising_three_methods reversed 5m b100-t1 F09 | 2 | 100.0 % | 50.0 % | -0.664 | -2.329 | 1 | -1.74 |
| cdl_rising_three_methods reversed 15m b025-t0.25 F00 | 2 | 100.0 % | 0.0 % | -0.674 | -1.598 | 2 | -0.76 |
| cdl_three_black_crows reversed 15m b025-t1 F06 | 2 | 100.0 % | 0.0 % | -0.716 | -2.432 | 2 | -1.10 |
| sd_broken reversed 1h b025-t0.5 F00 | 2 | 100.0 % | 0.0 % | -0.776 | -2.052 | 2 | -0.92 |
| cdl_three_white_soldiers reversed 1h b025-t0.25 F00 | 2 | 100.0 % | 0.0 % | -0.858 | -1.967 | 2 | -1.12 |
| cdl_falling_three_methods reversed 5m b025-t0.25 F00 | 2 | 100.0 % | 0.0 % | -0.932 | -2.114 | 2 | -1.36 |
| sd_broken reversed 1h b025-t0.25 F00 | 2 | 100.0 % | 0.0 % | -1.026 | -2.302 | 2 | -1.17 |
| cdl_rising_three_methods reversed 5m b100-t1 F08 | 2 | 100.0 % | 0.0 % | -1.345 | -3.690 | 2 | -1.74 |
| cdl_rising_three_methods reversed 5m b025-t3 F08 | 2 | 100.0 % | 0.0 % | -5.944 | -14.888 | 2 | -7.97 |
| cdl_rising_three_methods reversed 5m b025-t2 F08 | 2 | 100.0 % | 0.0 % | -6.945 | -15.889 | 2 | -8.97 |
| cdl_rising_three_methods reversed 5m b025-t1 F08 | 2 | 100.0 % | 0.0 % | -7.946 | -16.893 | 2 | -9.97 |
| cdl_three_black_crows native 1h b100-t3 F00 | 1 | 100.0 % | 100.0 % | +2.965 | +2.894 | 0 | 2.96 |
| cdl_three_black_crows native 1h b100-t3 F06 | 1 | 100.0 % | 100.0 % | +2.965 | +2.894 | 0 | 2.96 |
| cdl_three_black_crows native 1h b025-t3 F00 | 1 | 100.0 % | 100.0 % | +2.956 | +2.865 | 0 | 2.96 |
| cdl_three_black_crows native 1h b025-t3 F06 | 1 | 100.0 % | 100.0 % | +2.956 | +2.865 | 0 | 2.96 |
| cdl_evening_doji_star native 1h b100-t3 F07 | 1 | 100.0 % | 100.0 % | +2.893 | +2.779 | 0 | 2.89 |
| cdl_evening_doji_star native 1h b100-t3 F08 | 1 | 100.0 % | 100.0 % | +2.892 | +2.756 | 0 | 2.89 |
| cdl_three_black_crows native 15m b100-t3 F01 | 1 | 100.0 % | 100.0 % | +2.860 | +2.717 | 0 | 2.86 |
| cdl_three_black_crows native 15m b100-t3 F05 | 1 | 100.0 % | 100.0 % | +2.860 | +2.717 | 0 | 2.86 |
| cdl_morning_doji_star reversed 1h b100-t3 F06 | 1 | 100.0 % | 100.0 % | +2.859 | +2.712 | 0 | 2.86 |
| cdl_evening_doji_star native 1h b025-t3 F08 | 1 | 100.0 % | 100.0 % | +2.833 | +2.621 | 0 | 2.83 |
| cdl_three_black_crows native 15m b025-t3 F01 | 1 | 100.0 % | 100.0 % | +2.826 | +2.649 | 0 | 2.83 |
| cdl_three_black_crows native 15m b025-t3 F05 | 1 | 100.0 % | 100.0 % | +2.826 | +2.649 | 0 | 2.83 |
| cdl_evening_doji_star native 1h b025-t3 F07 | 1 | 100.0 % | 100.0 % | +2.799 | +2.598 | 0 | 2.80 |
| cdl_gravestone_doji native 1h b100-t3 F02 | 1 | 100.0 % | 100.0 % | +2.783 | +2.562 | 0 | 2.78 |
| cdl_morning_doji_star reversed 1h b025-t3 F06 | 1 | 100.0 % | 100.0 % | +2.779 | +2.545 | 0 | 2.78 |
| cdl_dragonfly_doji native 1h b100-t3 F07 | 1 | 100.0 % | 100.0 % | +2.776 | +2.621 | 0 | 2.78 |
| cdl_evening_star native 1h b025-t3 F02 | 1 | 100.0 % | 100.0 % | +2.764 | +2.521 | 0 | 2.76 |
| cdl_evening_star native 1h b025-t3 F07 | 1 | 100.0 % | 100.0 % | +2.764 | +2.521 | 0 | 2.76 |
| cdl_falling_three_methods native 5m b025-t3 F07 | 1 | 100.0 % | 100.0 % | +2.751 | +2.471 | 0 | 2.75 |
| cdl_rising_three_methods reversed 15m b100-t3 F04 | 1 | 100.0 % | 100.0 % | +2.734 | +2.466 | 0 | 2.73 |
| cdl_three_inside_down native 1h b025-t3 F02 | 1 | 100.0 % | 100.0 % | +2.652 | +2.292 | 0 | 2.65 |
| cdl_bullish_harami_cross native 1h b100-t3 F07 | 1 | 100.0 % | 100.0 % | +2.646 | +2.314 | 0 | 2.65 |
| fib_ext reversed 1h b100-t3 F08 | 1 | 100.0 % | 100.0 % | +2.633 | +2.266 | 0 | 2.63 |
| fib_ext reversed 1h b100-t3 F09 | 1 | 100.0 % | 100.0 % | +2.633 | +2.266 | 0 | 2.63 |
| cdl_gravestone_doji native 1h b025-t3 F02 | 1 | 100.0 % | 100.0 % | +2.592 | +2.184 | 0 | 2.59 |
| cdl_rising_three_methods reversed 5m b100-t3 F03 | 1 | 100.0 % | 100.0 % | +2.402 | +1.821 | 0 | 2.40 |
| cdl_bullish_harami_cross native 1h b025-t3 F07 | 1 | 100.0 % | 100.0 % | +2.262 | +1.572 | 0 | 2.26 |
| ema_stack reversed 1h b025-t3 F04 | 1 | 100.0 % | 100.0 % | +2.195 | +1.325 | 0 | 2.19 |
| cdl_three_black_crows native 1h b100-t2 F00 | 1 | 100.0 % | 100.0 % | +1.934 | +1.858 | 0 | 1.93 |
| cdl_three_black_crows native 1h b100-t2 F06 | 1 | 100.0 % | 100.0 % | +1.934 | +1.858 | 0 | 1.93 |
| cdl_three_black_crows native 1h b025-t2 F00 | 1 | 100.0 % | 100.0 % | +1.911 | +1.816 | 0 | 1.91 |
| cdl_three_black_crows native 1h b025-t2 F06 | 1 | 100.0 % | 100.0 % | +1.911 | +1.816 | 0 | 1.91 |
| cdl_evening_doji_star native 1h b100-t2 F08 | 1 | 100.0 % | 100.0 % | +1.894 | +1.760 | 0 | 1.89 |
| cdl_evening_doji_star native 1h b100-t2 F07 | 1 | 100.0 % | 100.0 % | +1.892 | +1.778 | 0 | 1.89 |
| cdl_three_black_crows native 15m b100-t2 F01 | 1 | 100.0 % | 100.0 % | +1.861 | +1.719 | 0 | 1.86 |
| cdl_three_black_crows native 15m b100-t2 F05 | 1 | 100.0 % | 100.0 % | +1.861 | +1.719 | 0 | 1.86 |
| cdl_morning_doji_star reversed 1h b100-t2 F06 | 1 | 100.0 % | 100.0 % | +1.860 | +1.712 | 0 | 1.86 |
| cdl_evening_star native 1h b100-t2 F02 | 1 | 100.0 % | 100.0 % | +1.854 | +1.701 | 0 | 1.85 |
| cdl_evening_star native 1h b100-t2 F07 | 1 | 100.0 % | 100.0 % | +1.854 | +1.701 | 0 | 1.85 |
| cdl_three_black_crows native 15m b025-t2 F01 | 1 | 100.0 % | 100.0 % | +1.823 | +1.647 | 0 | 1.82 |
| cdl_three_black_crows native 15m b025-t2 F05 | 1 | 100.0 % | 100.0 % | +1.823 | +1.647 | 0 | 1.82 |
| cdl_evening_doji_star native 1h b025-t2 F08 | 1 | 100.0 % | 100.0 % | +1.805 | +1.588 | 0 | 1.80 |
| cdl_evening_doji_star native 1h b025-t2 F07 | 1 | 100.0 % | 100.0 % | +1.798 | +1.597 | 0 | 1.80 |
| cdl_falling_three_methods native 5m b100-t2 F07 | 1 | 100.0 % | 100.0 % | +1.793 | +1.561 | 0 | 1.79 |
| cdl_morning_doji_star reversed 1h b025-t2 F06 | 1 | 100.0 % | 100.0 % | +1.781 | +1.551 | 0 | 1.78 |
| cdl_gravestone_doji native 1h b100-t2 F02 | 1 | 100.0 % | 100.0 % | +1.779 | +1.559 | 0 | 1.78 |
| cdl_three_inside_down native 1h b100-t2 F02 | 1 | 100.0 % | 100.0 % | +1.777 | +1.547 | 0 | 1.78 |
| cdl_dragonfly_doji native 1h b100-t2 F07 | 1 | 100.0 % | 100.0 % | +1.777 | +1.623 | 0 | 1.78 |
| cdl_evening_star native 1h b025-t2 F02 | 1 | 100.0 % | 100.0 % | +1.764 | +1.520 | 0 | 1.76 |
| cdl_evening_star native 1h b025-t2 F07 | 1 | 100.0 % | 100.0 % | +1.764 | +1.520 | 0 | 1.76 |
| cdl_falling_three_methods native 5m b025-t2 F07 | 1 | 100.0 % | 100.0 % | +1.737 | +1.453 | 0 | 1.74 |
| cdl_rising_three_methods reversed 15m b100-t2 F04 | 1 | 100.0 % | 100.0 % | +1.734 | +1.466 | 0 | 1.73 |
| cdl_three_inside_down native 1h b025-t2 F02 | 1 | 100.0 % | 100.0 % | +1.653 | +1.294 | 0 | 1.65 |
| cdl_bullish_harami_cross native 1h b100-t2 F07 | 1 | 100.0 % | 100.0 % | +1.646 | +1.315 | 0 | 1.65 |
| fib_ext reversed 1h b100-t2 F08 | 1 | 100.0 % | 100.0 % | +1.632 | +1.265 | 0 | 1.63 |
| fib_ext reversed 1h b100-t2 F09 | 1 | 100.0 % | 100.0 % | +1.632 | +1.265 | 0 | 1.63 |
| cdl_rising_three_methods reversed 5m b100-t3 F06 | 1 | 100.0 % | 100.0 % | +1.611 | +0.221 | 0 | 1.61 |
| cdl_gravestone_doji native 1h b025-t2 F02 | 1 | 100.0 % | 100.0 % | +1.591 | +1.182 | 0 | 1.59 |
| cdl_three_black_crows native 1h b100-t1.5 F00 | 1 | 100.0 % | 100.0 % | +1.431 | +1.357 | 0 | 1.43 |
| cdl_rising_three_methods reversed 5m b100-t2 F03 | 1 | 100.0 % | 100.0 % | +1.422 | +0.843 | 0 | 1.42 |
| cdl_three_black_crows native 1h b025-t1.5 F00 | 1 | 100.0 % | 100.0 % | +1.411 | +1.315 | 0 | 1.41 |
| cdl_three_inside_up native 1h b025-t1.5 F00 | 1 | 100.0 % | 100.0 % | +1.350 | +1.220 | 0 | 1.35 |
| cdl_bullish_harami_cross native 1h b025-t2 F07 | 1 | 100.0 % | 100.0 % | +1.263 | +0.574 | 0 | 1.26 |
| fib_ext reversed 5m b100-t3 F07 | 1 | 100.0 % | 100.0 % | +1.171 | -0.658 | 0 | 1.17 |
| ema_stack reversed 1h b025-t2 F04 | 1 | 100.0 % | 100.0 % | +1.129 | +0.258 | 0 | 1.13 |
| cdl_three_black_crows native 1h b100-t1 F00 | 1 | 100.0 % | 100.0 % | +0.931 | +0.857 | 0 | 0.93 |
| cdl_three_black_crows native 1h b100-t1 F06 | 1 | 100.0 % | 100.0 % | +0.931 | +0.857 | 0 | 0.93 |
| cdl_three_black_crows native 1h b025-t1 F00 | 1 | 100.0 % | 100.0 % | +0.910 | +0.814 | 0 | 0.91 |
| cdl_three_black_crows native 1h b025-t1 F06 | 1 | 100.0 % | 100.0 % | +0.910 | +0.814 | 0 | 0.91 |
| cdl_three_inside_up native 1h b100-t1 F00 | 1 | 100.0 % | 100.0 % | +0.893 | +0.801 | 0 | 0.89 |
| cdl_three_inside_up native 1h b100-t1 F04 | 1 | 100.0 % | 100.0 % | +0.893 | +0.801 | 0 | 0.89 |
| cdl_three_inside_up native 1h b100-t1 F09 | 1 | 100.0 % | 100.0 % | +0.893 | +0.801 | 0 | 0.89 |
| cdl_evening_doji_star native 1h b100-t1 F07 | 1 | 100.0 % | 100.0 % | +0.886 | +0.771 | 0 | 0.89 |
| cdl_evening_doji_star native 1h b100-t1 F08 | 1 | 100.0 % | 100.0 % | +0.877 | +0.739 | 0 | 0.88 |
| cdl_three_black_crows native 15m b100-t1 F01 | 1 | 100.0 % | 100.0 % | +0.860 | +0.719 | 0 | 0.86 |
| cdl_three_black_crows native 15m b100-t1 F05 | 1 | 100.0 % | 100.0 % | +0.860 | +0.719 | 0 | 0.86 |
| cdl_three_inside_up native 1h b025-t1 F00 | 1 | 100.0 % | 100.0 % | +0.858 | +0.729 | 0 | 0.86 |
| cdl_three_inside_up native 1h b025-t1 F04 | 1 | 100.0 % | 100.0 % | +0.858 | +0.729 | 0 | 0.86 |
| cdl_three_inside_up native 1h b025-t1 F09 | 1 | 100.0 % | 100.0 % | +0.858 | +0.729 | 0 | 0.86 |
| cdl_morning_doji_star reversed 1h b100-t1 F06 | 1 | 100.0 % | 100.0 % | +0.854 | +0.707 | 0 | 0.85 |
| cdl_evening_star native 1h b100-t1 F02 | 1 | 100.0 % | 100.0 % | +0.853 | +0.701 | 0 | 0.85 |
| cdl_evening_star native 1h b100-t1 F07 | 1 | 100.0 % | 100.0 % | +0.853 | +0.701 | 0 | 0.85 |
| cdl_three_black_crows native 15m b025-t1 F01 | 1 | 100.0 % | 100.0 % | +0.825 | +0.650 | 0 | 0.82 |
| cdl_three_black_crows native 15m b025-t1 F05 | 1 | 100.0 % | 100.0 % | +0.825 | +0.650 | 0 | 0.82 |
| cdl_evening_doji_star native 1h b025-t1 F08 | 1 | 100.0 % | 100.0 % | +0.805 | +0.587 | 0 | 0.81 |
| cdl_evening_doji_star native 1h b025-t1 F07 | 1 | 100.0 % | 100.0 % | +0.800 | +0.600 | 0 | 0.80 |
| cdl_dragonfly_doji native 1h b100-t1 F07 | 1 | 100.0 % | 100.0 % | +0.799 | +0.648 | 0 | 0.80 |
| cdl_gravestone_doji native 1h b100-t1 F02 | 1 | 100.0 % | 100.0 % | +0.779 | +0.559 | 0 | 0.78 |
| cdl_three_inside_down native 1h b100-t1 F02 | 1 | 100.0 % | 100.0 % | +0.771 | +0.543 | 0 | 0.77 |
| cdl_morning_doji_star reversed 1h b025-t1 F06 | 1 | 100.0 % | 100.0 % | +0.769 | +0.537 | 0 | 0.77 |
| cdl_falling_three_methods native 5m b100-t1 F07 | 1 | 100.0 % | 100.0 % | +0.764 | +0.529 | 0 | 0.76 |
| cdl_evening_star native 1h b025-t1 F02 | 1 | 100.0 % | 100.0 % | +0.757 | +0.513 | 0 | 0.76 |
| cdl_evening_star native 1h b025-t1 F07 | 1 | 100.0 % | 100.0 % | +0.757 | +0.513 | 0 | 0.76 |
| cdl_rising_three_methods reversed 15m b100-t1 F04 | 1 | 100.0 % | 100.0 % | +0.726 | +0.453 | 0 | 0.73 |
| cdl_falling_three_methods native 5m b025-t1 F07 | 1 | 100.0 % | 100.0 % | +0.719 | +0.437 | 0 | 0.72 |
| cdl_three_black_crows native 1h b100-t0.75 F00 | 1 | 100.0 % | 100.0 % | +0.680 | +0.606 | 0 | 0.68 |
| cdl_three_black_crows native 1h b025-t0.75 F00 | 1 | 100.0 % | 100.0 % | +0.660 | +0.564 | 0 | 0.66 |
| cdl_evening_doji_star reversed 1h b025-t1 F08 | 1 | 100.0 % | 100.0 % | +0.658 | +0.335 | 0 | 0.66 |
| cdl_three_inside_up native 1h b100-t0.75 F00 | 1 | 100.0 % | 100.0 % | +0.649 | +0.558 | 0 | 0.65 |
| cdl_bullish_harami_cross native 1h b100-t1 F07 | 1 | 100.0 % | 100.0 % | +0.648 | +0.319 | 0 | 0.65 |
| cdl_three_inside_down native 1h b025-t1 F02 | 1 | 100.0 % | 100.0 % | +0.642 | +0.285 | 0 | 0.64 |
| fib_ext reversed 1h b100-t1 F08 | 1 | 100.0 % | 100.0 % | +0.634 | +0.267 | 0 | 0.63 |
| fib_ext reversed 1h b100-t1 F09 | 1 | 100.0 % | 100.0 % | +0.634 | +0.267 | 0 | 0.63 |
| cdl_three_inside_up native 1h b025-t0.75 F00 | 1 | 100.0 % | 100.0 % | +0.617 | +0.484 | 0 | 0.62 |
| cdl_rising_three_methods reversed 5m b100-t2 F06 | 1 | 100.0 % | 100.0 % | +0.612 | -0.775 | 0 | 0.61 |
| cdl_gravestone_doji native 1h b025-t1 F02 | 1 | 100.0 % | 100.0 % | +0.591 | +0.181 | 0 | 0.59 |
| cdl_three_black_crows native 1h b100-t0.5 F00 | 1 | 100.0 % | 100.0 % | +0.430 | +0.356 | 0 | 0.43 |
| cdl_rising_three_methods reversed 5m b100-t1 F03 | 1 | 100.0 % | 100.0 % | +0.415 | -0.171 | 0 | 0.41 |
| cdl_three_black_crows native 1h b025-t0.5 F00 | 1 | 100.0 % | 100.0 % | +0.410 | +0.314 | 0 | 0.41 |
| cdl_three_inside_up native 1h b100-t0.5 F00 | 1 | 100.0 % | 100.0 % | +0.406 | +0.312 | 0 | 0.41 |
| cdl_three_inside_up native 1h b025-t0.5 F00 | 1 | 100.0 % | 100.0 % | +0.367 | +0.234 | 0 | 0.37 |
| cdl_bullish_harami_cross native 1h b025-t1 F07 | 1 | 100.0 % | 100.0 % | +0.264 | -0.425 | 0 | 0.26 |
| cdl_evening_star reversed 1h b025-t1 F06 | 1 | 100.0 % | 100.0 % | +0.241 | -0.517 | 0 | 0.24 |
| cdl_three_black_crows native 1h b100-t0.25 F00 | 1 | 100.0 % | 100.0 % | +0.175 | +0.100 | 0 | 0.17 |
| fib_ext reversed 5m b100-t2 F07 | 1 | 100.0 % | 100.0 % | +0.170 | -1.659 | 0 | 0.17 |
| cdl_three_inside_up native 1h b100-t0.25 F00 | 1 | 100.0 % | 100.0 % | +0.156 | +0.062 | 0 | 0.16 |
| cdl_three_black_crows native 1h b025-t0.25 F00 | 1 | 100.0 % | 100.0 % | +0.153 | +0.056 | 0 | 0.15 |
| ema_stack reversed 1h b025-t1 F04 | 1 | 100.0 % | 100.0 % | +0.129 | -0.743 | 0 | 0.13 |
| cdl_three_inside_up native 1h b025-t0.25 F00 | 1 | 100.0 % | 100.0 % | +0.117 | -0.015 | 0 | 0.12 |
| retest reversed 5m b025-t3 F03 | 1 | 100.0 % | 100.0 % | +0.055 | -2.890 | 0 | 0.06 |
| retest reversed 5m b025-t3 F04 | 1 | 100.0 % | 100.0 % | +0.055 | -2.890 | 0 | 0.06 |
| retest reversed 5m b025-t3 F07 | 1 | 100.0 % | 100.0 % | +0.055 | -2.890 | 0 | 0.06 |
| cdl_rising_three_methods reversed 5m b100-t1 F06 | 1 | 100.0 % | 0.0 % | -0.386 | -1.771 | 1 | -0.39 |
| cdl_falling_three_methods reversed 5m b025-t1 F08 | 1 | 100.0 % | 0.0 % | -0.608 | -2.215 | 1 | -0.61 |
| cdl_falling_three_methods reversed 5m b025-t1 F09 | 1 | 100.0 % | 0.0 % | -0.608 | -2.215 | 1 | -0.61 |
| fib_ext reversed 5m b100-t1 F07 | 1 | 100.0 % | 0.0 % | -0.830 | -2.661 | 1 | -0.83 |
| retest reversed 5m b025-t2 F03 | 1 | 100.0 % | 0.0 % | -0.946 | -3.891 | 1 | -0.95 |
| retest reversed 5m b025-t2 F04 | 1 | 100.0 % | 0.0 % | -0.946 | -3.891 | 1 | -0.95 |
| retest reversed 5m b025-t2 F07 | 1 | 100.0 % | 0.0 % | -0.946 | -3.891 | 1 | -0.95 |
| cdl_rising_three_methods reversed 5m b100-t1 F04 | 1 | 100.0 % | 0.0 % | -1.743 | -4.487 | 1 | -1.74 |
| retest reversed 5m b025-t1 F03 | 1 | 100.0 % | 0.0 % | -1.946 | -4.893 | 1 | -1.95 |
| retest reversed 5m b025-t1 F04 | 1 | 100.0 % | 0.0 % | -1.946 | -4.893 | 1 | -1.95 |
| retest reversed 5m b025-t1 F07 | 1 | 100.0 % | 0.0 % | -1.946 | -4.893 | 1 | -1.95 |
| cdl_rising_three_methods reversed 5m b025-t3 F04 | 1 | 100.0 % | 0.0 % | -7.974 | -18.947 | 1 | -7.97 |
| cdl_rising_three_methods reversed 5m b025-t2 F04 | 1 | 100.0 % | 0.0 % | -8.974 | -19.948 | 1 | -8.97 |
| cdl_rising_three_methods reversed 5m b025-t1 F04 | 1 | 100.0 % | 0.0 % | -9.975 | -20.950 | 1 | -9.97 |
