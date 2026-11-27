# Backtest benchmark: um BTCUSDT 15m

Full run over all stored history (holdout included for timing only; no outcome numbers were printed or written). Release build, this laptop, single thread.

Candles: 72480 (2024-11-01 00:00 .. 2026-11-26 00:00), 1m candles: 1087200, funding rows: 2265, events: 138430.

| step | seconds |
|---|---|
| load 15m candles | 0.03 |
| load 1m candles | 0.69 |
| load funding | 0.01 |
| prepare: detectors + context + levels | 0.24 |
| lookahead audit (3 cuts) | 0.39 |
| fingerprints | 0.08 |
| 658 rule runs (329 rules x 2 cost levels), 657230 trades, metrics + bootstrap | 3.90 |
| **total** | **5.46** |

Combined results hash: `cf751719a83237785364c7d4b5159bbae83a804063f58950b31090b797ba35e2`
