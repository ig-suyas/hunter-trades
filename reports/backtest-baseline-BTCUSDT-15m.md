# Backtest: um BTCUSDT 15m (rules: random)

In-sample candles 2024-11-01 00:00 .. 2026-05-27 00:00; holdout from 2026-05-27 00:00 (sealed).
Costs: CostModel { taker_fee_bps: 5.0, spread_bps: 1.0, slip_bps: 1.0, slip_atr: 0.02, mult: 1.0, funding: true }; sizing: Sizing { equity0: 10000.0, risk_frac: 0.01, max_leverage: 10.0 }.
Lookahead audit: PASS (3 cuts).

```
rule,cost_mult,signals,trades,win_rate,avg_win_r,avg_loss_r,expectancy_r,ci_lo,ci_hi,gross_r,cost_r,profit_factor,max_dd_r,max_dd_pct,longest_losing_streak,exposure,mean_hold_bars,results_hash
random-1r,1,27527,12325,0.4640,0.5061,-1.4733,-0.5548,-0.5824,-0.5262,-0.0036,0.5513,0.297,6841.02,100.00,54,0.659,2.94,2fb8876f141bc19a05cf7c56ced531eeca24d1915dd72d2d9212bdc314ade2ef
random-1r,2,27527,12325,0.2639,0.2833,-1.6039,-1.1060,-1.1513,-1.0597,-0.0036,1.1024,0.063,13631.86,100.00,184,0.659,2.94,a27ae14f3abed08f00b1367f36a97980dbe1772f7532f46b941d47da35e37a98
```

| step | seconds |
|---|---|
| load 15m candles | 0.02 |
| load 1m candles | 0.70 |
| load funding | 0.02 |
| prepare: detectors + context + levels | 0.07 |
| lookahead audit (3 cuts) | 0.15 |
| fingerprints | 0.05 |
| 2 rule runs (1 rules x 2 cost levels), 24650 trades, metrics + bootstrap | 0.07 |
| **total** | **1.14** |

Combined results hash: `d667407176fcbc2669defc6c25d773b4de8f6fa27d435a12a297115d82231c58`
