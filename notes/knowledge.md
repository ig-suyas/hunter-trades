# Candlescope knowledge base: testable definitions

Phase 0, 2026-11-27. Every tactic and pattern named in the inputs (`technical_analysis_patterns.txt`,
`Trading_Tactics_Study_Guide.pdf`, `pumpglass_heatmap_trading_guide.pdf`, the two videos) is listed here
with a definition precise enough to code. Sources are in `notes/sources.md` (tags like [TALIB] point there).
Wording is my own; no paid content is copied.

**Classes**
- **OBJECTIVE**: codable with no judgement and at most conventional fixed constants (e.g. engulfing).
- **SEMI**: codable, but the result depends on parameters (pivot strength, tolerances, lookbacks) that
  must be fixed *before* testing and counted in the multiple-testing budget.
- **SUBJECTIVE**: the published definitions disagree or depend on wave counting; can only be
  approximated by a parameterised proxy, and every variant tried counts as a separate hypothesis.

**Testing rule that applies to everything below:** a pattern only becomes a *signal* at the close of
the candle that completes its confirmation. All quantities use data with timestamp <= that close.
Pivots are only known `k` bars after they occur (see S2), so a detector may never use a pivot before it is
confirmed. Entry is at the next bar's open (or a resting stop/limit that is filled by a later bar), never
at the confirming bar's close.

---

## S. Shared building blocks (used by many detectors)

| ID | Name | Definition | Class |
|---|---|---|---|
| S1 | Candle parts | `body = |C-O|`, `range = H-L`, `upper = H-max(O,C)`, `lower = min(O,C)-L`, `mid = (O+C)/2`. Bull if `C>O`, bear if `C<O`. | OBJECTIVE |
| S2 | Swing pivot (fractal) | Bar `i` is a swing high of strength `k` if `H[i] > H[j]` for all `j` in `[i-k, i+k]`, `j != i` (ties: earliest wins). Known only at close of bar `i+k`. Swing low mirrors with `L`. Default `k` in {2,3,5}. | SEMI |
| S3 | ZigZag swing (alternative) | Alternate highs/lows; a new swing is confirmed when price retraces `>= m x ATR(14)` (or `p%`) from the running extreme. Confirmation time = bar where the retrace threshold is crossed. | SEMI |
| S4 | ATR | Wilder ATR(14) on true range. Used to normalise "long body", tolerances, buffers. | OBJECTIVE |
| S5 | "Equal" prices | `|a-b| <= tol`, `tol = q x ATR(14)` at the later point, default `q = 0.1-0.25`. | SEMI |
| S6 | Long / short body (TA-Lib style) | Long: `body > 1.0 x mean(body, 10 prior bars)`; short: `body < 1.0 x mean(body,10)`; doji: `body <= 0.1 x mean(range,10)`. [TALIB] | OBJECTIVE (fixed TA-Lib constants) |
| S7 | Prior trend (for candlesticks) | Down-trend before bar `i` if `C[i-1] < SMA(C,10)[i-1]` and `SMA10` slope over 5 bars < 0 (mirror for up). Alternatives: last 3 swings LH/LL. | SEMI |
| S8 | Market structure trend | Uptrend: last two confirmed swing highs rising (HH) and last two swing lows rising (HL). Downtrend: LH and LL. Otherwise range. Guide section 2. | SEMI (pivot `k`) |
| S9 | Structure break (BOS / MSS) | Close above the most recent confirmed swing high (bull) / below the last swing low (bear). "Market-structure shift" = a BOS against the prevailing S8 trend, usually on a lower time frame. | SEMI |
| S10 | Range | Over the last `N` bars (N in 20-100), `(max H - min L) <= r x ATR(14)` and at least 2 touches (within S5 tol) of each boundary. Boundaries = zone between highest wick and highest body close (top) / lowest wick and lowest body (bottom). Video `trading1.mp4` boxes = this. | SEMI |
| S11 | Level / zone | A horizontal zone built from >= 2 swing highs or lows within S5 tolerance, zone = [min body extreme, max wick extreme] of the touching bars. Guide section 3. | SEMI |
| S12 | Volume expansion | `V[i] > v x mean(V, 20)`, default `v = 1.5`. | SEMI |
| S13 | Costs | Every simulated fill pays taker fee + half-spread + slippage (in bps, per venue), and perps pay funding at each funding timestamp held. | OBJECTIVE (inputs fixed) |
| S14 | R | `R = |entry - stop|` per unit. Outcome in R after costs. | OBJECTIVE |

---

## T. Tactics from the study guide and the PumpGlass guide

| ID | Tactic | Testable definition (long; short is the mirror) | Confirmation / entry / stop / target | Class |
|---|---|---|---|---|
| T1 | Trend filter (MA) | Long allowed only if `C > EMA(n)` and `EMA(n)[i] > EMA(n)[i-s]`. Defaults `n` in {20,50,200}, `s=5`. Guide s5. | Filter only, not a signal. | SEMI |
| T2 | Support/resistance reaction | Price enters an S11 zone from above; a bar closes back above the zone top with `lower >= 0.5 x range` *or* bull close in top third of range. | Entry next open; stop = zone bottom - `b x ATR`; target = next opposing S11 zone or 2R. | SEMI |
| T3 | Role reversal (flip) | Old resistance zone broken by a close above it (T7), later revisited from above and holds (no close back inside). | Same as T8 retest. | SEMI |
| T4 | Demand zone (supply mirror) | "Base" = 1-6 bars with each `range <= 0.6 x ATR` (small, overlapping), followed by a departure leg of >= 2 bars where cumulative move `>= d x ATR` (d=2-3) and each departure bar body >= 0.5 range. Zone = [lowest wick of base, highest body of base]. Fresh until first revisit. Invalid when a close is below zone bottom. Guide s4. | Entry on first revisit + trigger (T2 reaction or lower-TF BOS). Stop = zone bottom - buffer. Target = origin of the departure leg's high or 2R. | SEMI |
| T5 | Trendline | Line through two confirmed swing lows (rising) with a third touch within S5 tol before signal. Break = close below line by > tol. Guide s5. | Bounce on 3rd+ touch with T2-style reaction; break as an exit or reversal input. | SEMI |
| T6 | Pullback continuation | (1) S8 uptrend on higher TF and `C > EMA50 rising`; (2) pullback: price moves down into "value" = within tol of EMA20/EMA50, *or* Fib 38.2-61.8% of last swing (T7f), *or* flipped zone (T3), *or* fresh demand (T4); (3) pullback is "healthy": its size < prior impulse size and its bar count >= impulse bar count; (4) trigger: close above the high of the last lower-high inside the pullback (mini-BOS) or a bull rejection bar. Guide s6, video EMA9/21 study. | Entry next open; stop below pullback low - buffer; target 1 = prior swing high; optional trail. Invalid if close below last swing low. | SEMI |
| T7f | Fibonacci retracement zone | For the last confirmed impulse swing low `A` -> swing high `B`: level(x) = `B - x(B-A)` for x in {0.236,0.382,0.5,0.618,0.65,0.786}. "Golden pocket" = [0.618,0.65]. Idea is cancelled by a close beyond 1.0 (below `A`). Extensions 1.272/1.618 = `B + (x-1)(B-A)` for targets. Guide s7; video `trading.mp4` shows a 4h XAUUSD fib with 0.5/0.618/0.786. | Price touches the level band (+-tol), then trigger as in T6(4). Swing choice via S2/S3 is the parameter. | SEMI |
| T8 | Breakout (aggressive) | Close above a level/range top by `> e x ATR` (e~0.1), with optional quality filters: breakout bar range and volume > 1.5x 20-bar mean (S12), prior compression (`ATR(5)/ATR(20) < c`), direction = higher-TF trend. Guide s8. | Entry next open; stop back inside range (range mid or retest low); fail-exit if a later close is back inside the range. | SEMI |
| T9 | Breakout and retest (conservative) | After T8, within `w` bars price returns to within tol of the broken level without closing below it, then a bull trigger bar closes. | Entry next open; stop below retest low - buffer; target measured move (range height) or next zone. | SEMI |
| T10 | Fakeout / failed breakout | A T8 breakout followed within `w` bars (w<=3) by a close back inside the range. | Trade the opposite direction from the next open; stop beyond the breakout extreme; target opposite range side. | SEMI |
| T11 | Liquidity sweep and reclaim | "Liquidity pool" = >= 2 swing lows within S5 tol (equal lows), or the previous day's low (UTC day), or a range low (S10). Sweep: a bar trades `L < pool - eps` (eps tiny), and **closes back above the pool** (same bar, or within `r` bars, r<=3). Confirm: next bar (or reclaim bar) closes bull, optionally a lower-TF BOS. Guide s9; PumpGlass example B; video `trading1.mp4` sketches. | Entry next open after confirmation; stop = sweep wick low - buffer; target = opposite side of the range / next level / 2R. A close below the pool that holds = breakout, no trade. | SEMI |
| T12 | Entry/stop/target rules | Stop at structural invalidation + buffer `0.1-0.25 x ATR`; never widened; position size = `account x risk% / |entry-stop|`; break-even win rate = `1/(1+RR)` before costs. Guide s10-11. | Used by the simulator for every detector. | OBJECTIVE |
| T13 | Partial exits / trailing | Exit `p%` at 1R or 2R, move stop to entry, trail remainder by last confirmed swing or `ATR x m`. | Variant of T12; each variant is a separate hypothesis. | SEMI |
| T14 | Confluence | Count of independent location reasons within tol of the entry price: S11 level, T4 zone, T7f level, EMA20/50, round number (multiple of 1000 for BTC, 100 for ETH). Signal requires count >= c. Guide s7. | Filter. | SEMI |
| H1 | Liquidation-cluster sweep (heatmap) | Needs an estimated liquidation map: for each recent OI increase at price `p`, assume leverage set {10,25,50,100}x and long/short split, project liquidation prices `p(1 -/+ 1/lev + mm)`. Cluster = price bin whose estimated notional > percentile 95 of bins within +-3% of price. Signal = T11 where the pool is a cluster instead of equal lows. PumpGlass s3-5; guide s12. | Same as T11. **Model estimate, not observed data**; backtest only with recorded OI and live liquidations (see data-sources.md). | SEMI (model) |
| H2 | Order-book wall | Price band (e.g. +-0.1%) whose resting size > `w x` median band size within 1% of mid, persisting >= `t` seconds. PumpGlass s2. Walls can be pulled/spoofed. | Context/target only. Needs recorded book data. | SEMI |
| H3 | OI / price quadrant | Over window `n`: sign(dPrice) x sign(dOI) -> {rising+rising: new longs, rising+falling: short covering, falling+rising: new shorts, falling+falling: long liquidation}. PumpGlass s6. | Filter/context feature. | OBJECTIVE (given n) |
| H4 | CVD | Cumulative `(taker buy volume - taker sell volume)`; from Binance kline field "taker buy base volume" (`V_buy`, `V_sell = V - V_buy`) or aggTrades `is_buyer_maker`. Divergence: price flat/up while CVD falls, or price flat with CVD rising (absorption). PumpGlass s6. | Filter. | SEMI |
| H5 | Funding skew | Funding rate z-score over 30 days `> z` = crowded longs (contrarian short bias), `< -z` = crowded shorts. PumpGlass s5. | Filter. | SEMI |
| H6 | Whale average entries | Platform-specific estimate (PumpGlass); no public definition or data. | Not testable; dropped unless a source is found. | SUBJECTIVE |

---

## R. Reversal chart patterns (bullish; bearish = mirror unless noted)

All use confirmed pivots (S2/S3). "Neckline break" = close beyond neckline by `> e x ATR`. Measured target
= pattern height projected from the break. Each is SEMI because pivot strength, tolerances and minimum
widths are parameters.

| ID | Pattern | Definition | Confirmation | Class |
|---|---|---|---|---|
| R1 | Inverse head and shoulders | Pivots L1 (left shoulder), H1, L2 (head), H2, L3 (right shoulder): `L2 < L1`, `L2 < L3`, `|L1-L3| <= tol_s` (e.g. 0.5 ATR or 10% of height), prior downtrend (S7/S8). Neckline = line H1-H2. [BULK] | Close above neckline; target = neckline - L2 projected up. | SEMI |
| R1b | Head and shoulders (top) | Mirror of R1. | Close below neckline. | SEMI |
| R2 | Double bottom | Two swing lows L1, L2 within tol, separated by >= `m` bars (e.g. 10) with a swing high P between, `P - max(L1,L2) >= h x ATR`. Prior downtrend. | Close above P ("confirmation point"). [BULK] | SEMI |
| R3 | Triple bottom | Three swing lows within tol, two intervening highs; confirmation line = the higher of the two highs. | Close above it. | SEMI |
| R4 | Falling wedge | Over >= 5 pivots, a line through swing highs and a line through swing lows both slope down, they converge (`slope_high < slope_low < 0`, so distance shrinks), >= 2 touches each, each line fit residual <= tol. | Close above upper line. | SEMI |
| R4b | Rising wedge | Both lines rise, converge (`slope_low > slope_high > 0`); bearish. | Close below lower line. | SEMI |
| R5 | Rounding bottom | Over window N (e.g. 30-100 bars), a quadratic fit `C ~ a t^2 + b t + c` has `a > 0`, vertex inside the middle 60% of the window, `R^2 >= r`; rim = max high in first/last 15% of window. | Close above rim. | SEMI |
| R6 | Quasimodo (QM, over-and-under) bullish | Swing sequence in a downtrend: H1, L1, H2 (`H2 < H1`), L2 (`L2 < L1`, new low), H3 (`H3 > H2`, breaks structure). The "QM level" is L1 (the left shoulder). Setup = price returns to the QM level. [CAPQM] | Entry on return to L1 zone with T2 reaction; stop below L2. No fixed formula in sources, so this is one proxy. | SEMI |
| R7 | Adam and Eve bottom | R2 where the first bottom ("Adam") is narrow/V-shaped (low bar's spike: few bars within 1 ATR of the low, e.g. <=3) and the second ("Eve") is wide/rounded (>= 2x as many bars within 1 ATR of the low). Other combos: Adam&Adam, Eve&Eve, Eve&Adam. [BULK-AE] | Close above the intervening peak. | SEMI |
| R8 | Broadening bottom | Swing highs rising and swing lows falling (diverging lines), >= 5 alternating touches, appearing after a downtrend; bullish variant = upward breakout of the upper line. | Close above upper line. | SEMI |
| R9 | Dragon pattern (bullish) | Informal pattern: a "head" swing high, two "legs" = two swing lows (second low roughly equal to or slightly above/below first, i.e. a W) with a "hump" (minor high) between, followed by a break of the trendline from head through hump. Effectively a double bottom variant with a descending trendline break. Definitions vary by author. | Close above head-hump trendline. | SUBJECTIVE |
| R10 | Bump and run reversal (bottom) | Phase 1 "lead-in": a down-trendline over >= 20 bars with slope `s1`; Phase 2 "bump": price accelerates below it, slope `<= 2 x s1` (steeper), distance from line at least double the lead-in's max distance; Phase 3 "run": price returns to and closes above the lead-in trendline. [BULK] | Close above the lead-in trendline. | SEMI |
| R10b | Bump and run (top) | Mirror. | Close below lead-in trendline. | SEMI |
| R11 | Double/triple top, rounding top, broadening top, bearish QM, Adam&Eve top, bearish dragon | Exact mirrors of R2, R3, R5, R8, R6, R7, R9. | Mirror. | as mirror |

## C. Continuation chart patterns (bullish; bearish mirror)

| ID | Pattern | Definition | Confirmation | Class |
|---|---|---|---|---|
| C1 | Bull flag | Pole: move of `>= p x ATR` (p~4) within `<= n` bars (n~10), >= 60% bull bodies. Flag: next 3-20 bars form a parallel channel (two regression lines on highs and lows, slope difference small) sloping flat or down, retracing <= 50% of the pole. | Close above flag upper line; target = pole height from breakout. | SEMI |
| C2 | Bull pennant | Like C1 but the consolidation is a small symmetrical triangle (converging lines, see N1) lasting 3-20 bars. | Close above upper line. | SEMI |
| C3 | Ascending triangle | >= 2 swing highs within tol (flat top) and >= 2 rising swing lows; lines converge. | Close above flat top. | SEMI |
| C4 | Rising rectangle | S10 range forming after an uptrend (S8). | Close above top. | SEMI |
| C5 | Bullish symmetrical triangle | N1 after an uptrend. | Close above upper line. | SEMI |
| C6 | Cup and handle | Cup = R5 rounding bottom between two rims within tol, depth 12-50% of prior advance; handle = small down-drift/flag after the right rim, retracing <= 1/3 of the cup depth, duration << cup. [BULK] | Close above the right rim / handle high. | SEMI |
| C7 | Ascending channel | Parallel rising lines through >= 2 swing highs and >= 2 swing lows (slopes within tol of each other, > 0). Trade bounces off lower line (T2) in trend direction. | Reaction at lower line; break of lower line = exit. | SEMI |
| C8 | Bullish measured move | Leg 1 up (A->B), correction (B->C, 38-62% retrace), leg 2 projected `C + (B-A)`. | Entry on T6 trigger after C; target = projection. | SEMI |
| C9 | Flat base | After an uptrend, >= 25 bars (in daily terms ~5 weeks) with max-min range <= 15% (crypto: <= r x ATR). | Close above base high. | SEMI |
| C10 | High tight flag | Price rise >= 90-100% within <= 40 bars of the TF, then flag that retraces <= 20-25%. Rare on BTC intraday. [BULK] | Close above flag high. | SEMI |
| C11 | Bear flag, bear pennant, descending triangle, falling rectangle, bearish sym. triangle, descending channel, bearish measured move, low tight flag | Mirrors of C1, C2, C3, C4, C5, C7, C8, C10. | Mirror. | SEMI |

## N. Bilateral / neutral patterns

| ID | Pattern | Definition | Confirmation | Class |
|---|---|---|---|---|
| N1 | Symmetrical (contracting) triangle | Lines through >= 2 lower highs and >= 2 higher lows, converging, slopes of opposite sign with similar magnitude (ratio within 0.5-2). "Contracting triangle" = same object. | Close outside either line; direction = break direction. | SEMI |
| N2 | Rectangle (trading range) | S10. | Close outside + optional T9 retest; T10/T11 at the edges. | SEMI |
| N3 | Broadening formation (megaphone), expanding triangle | Higher highs and lower lows, diverging lines, >= 5 alternating touches. Expanding triangle = same object. | Trade the break, or fade touches (each is a hypothesis). | SEMI |
| N4 | Ascending broadening wedge | Both lines rise, diverging (upper slope > lower slope > 0). Usually bearish break. | Close below lower line. | SEMI |
| N5 | Descending broadening wedge | Both lines fall, diverging. Usually bullish break. | Close above upper line. | SEMI |
| N6 | Diamond (top / bottom) | First half: N3 broadening; second half: N1 contracting; total >= ~20 bars. Top after uptrend, bottom after downtrend. | Close outside the lower (top) / upper (bottom) right-side line. | SUBJECTIVE (two-phase fit) |

## K. Candlestick patterns

Definitions follow TA-Lib's open-source rules (S6 thresholds) where TA-Lib has the pattern [TALIB],
otherwise Nison/Bulkowski-style descriptions [NISON-BULK]. Bullish patterns require S7 prior downtrend,
bearish ones prior uptrend (this "context" requirement is part of the definition, not optional).
Signal bar = last bar of the pattern; candlestick patterns are OBJECTIVE except where noted.
Common test: entry next open, stop beyond the pattern extreme, fixed R targets; and as triggers inside T2/T6/T11.

| ID | Pattern | Definition (bar 1 = oldest) |
|---|---|---|
| K1 | Hammer | Small body (S6 short) in upper part of range, `lower >= 2 x body`, `upper <= 0.1 x range` (or TA-Lib "very short" shadow), after downtrend. Body near the low of prior bar (TA-Lib). |
| K2 | Inverted hammer | Small body near low, `upper >= 2 x body`, `lower` very short, after downtrend; gaps/at low of prior bear bar. |
| K3 | Bullish engulfing | Bar 1 bear, bar 2 bull, body2 covers body1: `O2 <= C1` and `C2 >= O1` (at least one strict). Crypto: no gaps, so allow `O2 == C1`. |
| K4 | Piercing line | Bar 1 long bear; bar 2 bull opens below bar 1 low (TA-Lib) — in 24/7 crypto use `O2 <= C1` — and closes above bar 1 midpoint but below `O1`. |
| K5 | Morning star | Bar 1 long bear; bar 2 short body whose body is below bar 1's body; bar 3 bull closing above bar 1's midpoint (TA-Lib penetration 0.3 default). |
| K6 | Morning doji star | K5 with bar 2 = doji (S6). |
| K7 | Three white soldiers | Three bull bars, each closes higher, each opens within previous body, each has short upper shadow, bodies not shrinking sharply. |
| K8 | Tweezer bottom | Two bars, lows equal within `0.05 x ATR`, bar 1 bear, bar 2 bull, after downtrend. |
| K9 | Bullish harami | Bar 1 long bear; bar 2 small body fully inside bar 1's body. |
| K10 | Bullish harami cross | K9 with bar 2 doji. |
| K11 | Three inside up | K9 then bar 3 closes above bar 1's open. |
| K12 | Three outside up | K3 then bar 3 bull closing above bar 2's close. |
| K13 | Rising three methods | Bar 1 long bull; bars 2-4 small, mostly bear, staying within bar 1's range; bar 5 long bull closing above bar 1's close. |
| K14 | Bullish marubozu | Long bull body, `upper` and `lower` each `<= 0.05 x range` (TA-Lib "very short shadow"). |
| K15 | Dragonfly doji | Doji, `upper` ~0, long `lower` (>= 0.6 range). |
| K16 | Hanging man | K1 shape after an uptrend (bearish). |
| K17 | Shooting star | K2 shape after an uptrend, body near the bar's low. |
| K18 | Bearish engulfing | Mirror of K3. |
| K19 | Dark cloud cover | Mirror of K4: bar 1 long bull; bar 2 opens >= C1, closes below bar 1's midpoint but above `O1`. |
| K20 | Evening star / evening doji star | Mirrors of K5/K6. |
| K21 | Three black crows | Mirror of K7. |
| K22 | Tweezer top | Mirror of K8 (equal highs). |
| K23 | Bearish harami / harami cross | Mirrors of K9/K10. |
| K24 | Three inside down / three outside down | Mirrors of K11/K12. |
| K25 | Falling three methods | Mirror of K13. |
| K26 | Bearish marubozu | Mirror of K14. |
| K27 | Gravestone doji | Doji, `lower` ~0, long `upper` (>= 0.6 range). |
| K28 | Doji | `body <= 0.1 x mean(range,10)`. No direction; context only. |
| K29 | Long-legged doji | Doji with both shadows long (each >= 0.3 range) and range > mean range. |
| K30 | Spinning top | Short body (S6) with both shadows longer than the body. |
| K31 | High-wave candle | Spinning top with very long shadows (each >= 2x body) and range > 1.5x mean range. |
| K32 | Rickshaw man | Long-legged doji whose body sits near the middle of the range (|mid - (H+L)/2| <= 0.1 range). |

Gap-based conditions (piercing, stars, harami crosses with gaps) are written for gapped markets; for
24/7 crypto the gap requirement is replaced by the body relation shown. This substitution is a parameter
choice and is recorded as such (SEMI for those patterns on crypto).

## X. Harmonic patterns (all SUBJECTIVE)

Points X, A, B, C, D are alternating confirmed swings (S3 ZigZag). Ratios: `AB/XA`, `BC/AB`, `CD/BC`,
`AD/XA` (D measured as retracement of XA unless noted). Sources disagree; the ranges below are one
public convention [IG-FX], [CS-HARM]; every alternative range set is a separate hypothesis. "D" is a
Potential Reversal Zone; the trade needs a T2-style reaction there.

| ID | Pattern | B (of XA) | C (of AB) | D | Notes |
|---|---|---|---|---|---|
| X1 | Gartley | 0.618 | 0.382-0.886 | 0.786 of XA; CD 1.27-1.618 of BC | |
| X2 | Butterfly | 0.786 | 0.382-0.886 | 1.27-1.618 of XA (beyond X) | |
| X3 | Bat | 0.382-0.50 | 0.382-0.886 | 0.886 of XA; CD 1.618-2.618 of BC | |
| X4 | Alternate bat | <= 0.382 | 0.382-0.886 | 1.13 of XA; CD 2.0-3.618 of BC | |
| X5 | Crab | 0.382-0.618 | 0.382-0.886 | 1.618 of XA; CD 2.24-3.618 of BC | |
| X6 | Deep crab | 0.886 | 0.382-0.886 | 1.618 of XA | |
| X7 | Shark | (0-X-A-B-C naming) | C = 1.13-1.618 of XA-B leg | D = 0.886-1.13 of 0X | Different point naming; sources disagree. |
| X8 | Cypher | 0.382-0.618 | C = 1.272-1.414 of XA (beyond A) | D = 0.786 of XC | XC-based, not XA-based. |
| X9 | AB=CD | — | C 0.618-0.786 of AB | CD = AB in price (and ~time), or 1.27/1.618 x AB | Also listed as "ABCD pattern" (A4). |
| X10 | Three drives | Three pushes, each drive 1.27 or 1.618 extension of the prior correction, corrections 0.618-0.786; ~equal time. | | | Same family as A6. |

## A. Advanced / other price action

| ID | Pattern | Definition | Class |
|---|---|---|---|
| A1 | Quasimodo | See R6 (and bearish mirror). | SEMI |
| A2 | 1-2-3 reversal | In a downtrend: point 1 = lowest low (swing); point 2 = the next swing high after 1; point 3 = a higher swing low (`L3 > L1`). Signal = close above point 2. Stop below 3. | SEMI |
| A3 | 1-2-3 continuation | In an uptrend: 1 = swing low, 2 = swing high, 3 = higher low within 38-78% retrace of 1->2; signal = close above 2. | SEMI |
| A4 | ABCD | X9. | SUBJECTIVE |
| A5 | Wolfe wave (bullish) | Five swings 1-5 in a falling-wedge geometry: 3 below 1, 4 above 1 (inside 1-2 range), 5 below the 1-3 line extended; entry at 5 after a reaction; stop below 5; target = the line through points 1 and 4, extended forward ("estimated price at arrival"). Precise rules come from Bill Wolfe's paid material; public summaries differ. | SUBJECTIVE |
| A6 | Three push | Three successive higher highs (or lower lows) with diminishing extension and momentum (e.g. RSI lower highs), equal-ish spacing; signal = break of the swing low between pushes 2 and 3. | SEMI |
| A7 | Elliott wave patterns | 5-wave impulse with rules: wave 2 never retraces beyond wave 1 start, wave 3 never the shortest of 1/3/5, wave 4 does not overlap wave 1 price territory (cash markets); 3-wave corrections (zigzag 5-3-5, flat 3-3-5, triangle 3-3-3-3-3). The *rules* are codable; the *count* is not unique (degree, alternation, extensions), so any detector is a proxy. | SUBJECTIVE |
| A8 | Failed breakout | T10. | SEMI |
| A9 | Fakeout | T10 (single bar version: wick beyond level, close back inside = T11 sweep). | SEMI |
| A10 | Spring (Wyckoff) | Inside an established S10 range (after a downtrend = accumulation context), price breaks below range support (wick or close), then within `r` bars closes back inside; best on lower volume than the selling climax; a later low-volume "test" holds above the spring low. Mechanically = T11 on a range low with a volume condition. [WYCK] | SEMI |
| A11 | Upthrust / UTAD | Mirror of A10 at range resistance in distribution context. | SEMI |
| A12 | Accumulation | Wyckoff phases A-E: selling climax (SC: wide-range down bar, volume > 2x mean), automatic rally (AR) sets range top, secondary test (ST) near SC low on lower volume, spring/test (A10), sign of strength (break of AR high), last point of support. A detector can label a range "accumulation-like" by the presence of SC + AR + ST + spring; phase labelling is judgement-heavy. | SUBJECTIVE |
| A13 | Distribution | Mirror (BC buying climax, AR down, ST, UTAD, SOW). | SUBJECTIVE |
| A14 | Re-accumulation / re-distribution | A12/A13 structures that form mid-trend (after markup / markdown) and resolve in the trend direction. Same detector plus prior-trend condition. | SUBJECTIVE |

## V. Indicators and ideas seen in the videos (not in the text inputs)

See `notes/video-notes.md`. Only what was visible on screen; no rules were stated in the audio unless
the transcript section there says so.

| ID | Item | Definition | Class |
|---|---|---|---|
| V1 | EMA 9 / EMA 21 | Exponential moving averages of close (shown on XAUUSD 5m and BTCUSDT 15m). Candidate filter or cross signal: EMA9 crosses EMA21. | OBJECTIVE |
| V2 | Supertrend (10, 3) | ATR(10) bands at `hl2 +- 3 x ATR`, flipping on close beyond the active band. | OBJECTIVE |
| V3 | VWAP stdev bands | Session VWAP (UTC day for crypto) with bands at +-1/2/3 standard deviations of price around VWAP. | OBJECTIVE |
| V4 | RSI 14 | Wilder RSI(14). Shown on NVDA and BTCUSD 4h/1W. | OBJECTIVE |
| V5 | Range + sweep boxes | S10 + T11 (trading1.mp4). Narration rule: after a breakout, never enter immediately; wait for the retracement, then enter = T9. | SEMI |
| V6 | Fair value gap (FVG) | Bullish FVG at bar `i`: `H[i-2] < L[i]` (gap between candle 1 high and candle 3 low; candle 2 is the impulse). Zone = `[H[i-2], L[i]]`, known at close of `i`. Filled when a later low <= `H[i-2]`. Bearish mirror. Mentioned (SMC) in trading.mp4. Optional min size `>= g x ATR`. | OBJECTIVE (given g) |
| V7 | Session filter | Trade only when the signal bar's UTC time falls in a session window, e.g. London 07:00-16:00, New York 12:00-21:00 (DST-aware), overlap 12:00-16:00. trading.mp4 says volume is highest in the overlap. | OBJECTIVE (given windows) |
| V8 | News blackout | No new signals from `m` minutes before to `n` minutes after a high-impact (red) or medium-impact (orange) US calendar event (CPI, NFP, FOMC, PCE, GDP). Needs a local event calendar with timestamps, recorded before the event (no lookahead). trading.mp4. | SEMI |

---

## Recommended build order (tier 1 for Phase 2)

Tier 1 (objective or few parameters, directly from the guide): S1-S14, T1, T6, T7f, T8-T11, A2, A10/A11,
K1-K32. Tier 2: T2-T5, T14, R1-R4, R7, C1-C3, C7, N1-N2, H3-H5, V1-V4. Tier 3 (SUBJECTIVE, last, and
only with an explicit multiple-testing budget): R9, N6, X1-X10, A5, A7, A12-A14, H1-H2 (need recorded data).

---

## Phase 2 implementation choices (2026-11-27)

What `crates/cs-patterns` actually codes, where it differs from or pins down the rows above. All of
these were fixed from the definitions and from event counts only (no outcome or P&L data was looked
at), so they do not spend the multiple-testing budget; every *later* change does.

- **Events:** `formed_idx` = bar where the pattern completes, `known_idx` = first bar whose close
  reveals it (always the bar being processed). Usable from the next bar's open. `known_at` /
  `formed_at` are open times.
- **S2 swings:** k = 3 default; ties: earliest bar wins (`>` on the left, `>=` on the right).
- **S3 ZigZag:** 3 x ATR(14) (ATR before the bar); a bar that makes a new extreme never also
  confirms a reversal.
- **S8/S9:** CHoCH = break against the S8 trend in force before the bar; otherwise BOS. Each swing
  level breaks at most once.
- **S11 zones:** pivots of both sides cluster into one zone (band = wick..body extreme, tol 0.25 ATR,
  width <= 1 ATR, >= 2 pivots); overlapping zones merge; a close 0.1 ATR beyond breaks a zone and
  flips its role; a zone broken more than twice is retired; zones expire 1500 bars after their last
  pivot touch. Before merging/retiring were added, one bar could break up to 25 overlapping zones and
  each zone broke ~13 times (absurd counts), see the handoff.
- **T3 role reversal:** within 60 bars of a break, price must first close 0.5 ATR beyond the zone,
  then come back to it and close on the new side.
- **T4 supply/demand:** base 1-6 bars of range <= 0.6 ATR, departure 2-5 same-colour bars with
  body >= 0.5 range, departure close >= 2 ATR beyond the base zone. Tests counted per revisit
  (fresh = first). Broken on a close beyond the far side.
- **T5 trendline:** last two swing lows (rising) 5-300 bars apart, no close below the line between;
  touch = a bar reaches within 0.25 ATR after having been away; break = close 0.25 ATR beyond.
- **T6 pullback:** trend = EMA20 > EMA50 and EMA50 rising (5 bars); value = EMA20/50 band, Fib
  38.2-61.8 of the active ZigZag up-leg, flipped S/R zone, or demand zone tested <= once (touch in
  the last 4 bars); depth >= 1 ATR; trigger = bull close above the prior high or a rejection bar.
  Follow-up (same day): the "healthy pullback" rule is coded as a **flag** (`tags` bit 16), not
  a filter: top B = highest high of the 19 bars before the trigger, impulse origin A = lowest low
  of the 40 bars before B, healthy if the pullback depth < impulse size, pullback speed (depth per
  bar) < impulse speed, and no close since B below the last confirmed swing low before B. "Slower"
  is coded as price per bar; the knowledge-base wording "bar count >= impulse bar count" is a
  stricter variant (a separate hypothesis if tested). The higher-time-frame trend is a context
  feature (next bullet), not yet a pullback condition.
- **T7f:** the leg is the latest ZigZag leg of each direction; cancelled by a close beyond A.
- **S10/T8-T10:** range box = 40 bars within 5 ATR, >= 2 touches of each edge counted >= 3 bars
  apart; levels for breakouts = box edges and S/R zone breaks, **one candidate per bar and direction**
  (the outermost level). Breakout needs bar range and volume >= 1.5 x their 20-bar means. Fakeout =
  any close beyond a level (no expansion needed) and a close back inside within 3 bars.
- **T11 sweep:** pools = equal swing highs/lows (0.15 ATR, <= 200 bars apart, untouched since),
  previous UTC day H/L, previous UTC week (Monday) H/L. Wick beyond by > 0.02 ATR, reclaim close
  within 3 bars (sweep bar included), else the pool is dropped as broken. Range-edge pools (A10/A11
  spring/upthrust) are not separate yet.
- **K candlesticks:** sizes relative to ATR(14) of the bar before the pattern (TA-Lib constants with
  average range -> ATR, average body -> 0.5 ATR): doji <= 0.1, short <= 0.3, long >= 0.6, very short
  shadow <= 0.1, marubozu shadows <= 0.05, hammer-type shadow >= max(2 x body, 0.5 ATR), equal
  <= 0.05, star penetration 0.3, plain doji range >= 0.25. Gap rules replaced by body relations
  (crypto is 24/7). Star / harami variants are exclusive (doji middle bar -> only the doji version).
  Prior trend S7 at the bar before the pattern; bullish reversal patterns need a down-trend, bearish
  ones an up-trend; rising/falling three methods need a trend in their own direction; marubozus and
  the five neutral patterns need none.
- **Higher-time-frame trend (T1/S8 on 1h and 4h for a 5m/15m series):** base candles aggregate into
  UTC buckets; an HTF candle is used only once complete (all base candles present, completed at the
  close of the base candle that ends the bucket; `known_at` = that base candle's open time).
  EMA 20/50/200 stack and slopes and the swing structure trend (k = 3 on HTF candles) update on
  each completed HTF candle and are held in between. Mean lag on a 15m base: 1.5 candles (1h),
  7.5 candles (4h).
- **Context:** regime from ADX(14) (>= 25 trend, < 20 range, hysteresis between), volatility
  percentile = rank of ATR among the previous 500 bars, sessions in fixed UTC hours (no DST): Asia
  00-07, London 07-12, overlap 12-16, New York 16-21, late 21-24.
