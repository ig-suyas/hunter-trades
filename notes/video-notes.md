# Video notes

Phase 0, 2026-11-27. What the two videos show and say. Nothing here is invented: screen content comes
from extracted frames (one every 10 s) and OCR; speech comes from local speech-to-text. Where the
transcript is garbled, these notes say so instead of guessing.

**Method (all local, nothing sent off the machine except the frames viewed by the assistant model in this session, see "Privacy" below)**
- Frames: `ffmpeg -vf fps=1/10` -> `data/video/trading1/` (26 frames) and `data/video/trading/` (189 frames).
- OCR: every frame with the built-in Windows OCR engine (`Windows.Media.Ocr`) -> `data/video/*.ocr.txt`, and with
  Tesseract 5.4.0 (`--psm 11`) -> `data/video/*.tess.txt`. About 9 frames were also viewed directly.
- Audio: 16 kHz mono WAV, transcribed with **faster-whisper 1.2.1, model `large-v3-turbo`, language forced to Hindi**,
  VAD on, beam 5 -> `data/video/*.transcript.txt`. trading1 on CPU (725 s), trading on the RTX 3050 GPU (502 s).
  A first pass with model `small` was unusable (it stopped after 26 s and garbled the words) and was discarded.
- **Transcript quality:** the speech is casual Hindi with English trading words, over a call. Roughly half the
  segments are readable; the rest are partly garbled. Whisper also inserted the word "subscribe" (सब्सक्राइब)
  many times; that is a known Whisper hallucination on noise, not something said. The summaries below are my
  English paraphrase of the readable parts only. Timestamps are mm:ss into each video.
- **Privacy:** `trading.mp4` is a screen-shared video call. A call participant's camera image, names in the call
  window, a call URL, and an e-mail address on a prop-firm dashboard are visible. None of these are copied into
  these notes or any other project file. The frames stay in git-ignored `data/video/`.

---

## trading1.mp4 (4 min 22 s, 1920x1080)

**Screen:** TradingView (free plan, Paper Trading), `BTCUSD` CRYPTO index, 15-minute chart. On-screen clock
20:02-20:04 UTC, taskbar 01:30-01:34 IST on 27-11-2026, so it was **recorded 26 Nov 2026 ~20:00 UTC**. Price ~60,600-60,660.
Other tabs: Forex Factory calendar, a prop-firm trader dashboard, a second BTCUSD tab.
- 00:00-00:20: a sharp rally from ~57,000 (25 Nov) to ~61,000 (26 Nov 00:00 UTC), then sideways.
- ~01:30: opens TradingView's Black Friday pricing page (Essential / Plus / Premium / Ultimate plans in INR).
- 02:20-03:30: draws two horizontal boxes: range top ~61,000, range bottom ~60,150-60,180 (a wick dips below the
  bottom at ~15:00 UTC on 26 Nov and closes back inside). Sketches with the brush a spike above the top box that
  comes back (sweep-and-return picture) and a move down.
- 03:40: TradingView pop-up "Add more drawings - you've reached the limit of 30 drawings" (free plan limit).

**Speech (paraphrase of readable parts):**
- 00:00-00:24: on the 15-minute chart buying came in and it went straight up without taking a single pullback,
  never made a (higher) low, kept rising, then went sideways.
- 00:24-00:44: when its low breaks the market can go into selling, otherwise it goes up; liquidity sits on both
  sides (buyers' and sellers'), and now this has broken.
- 00:44-01:10: when the market moves, it will pull back and come back like this; **then take the trade from here**.
- 01:10-01:45: complains that drawing is blocked and shows the subscription prices ("who pays this much").
- 02:30-02:56: explains candles: a buying candle opens low and closes high, a selling candle opens high and closes
  low; "now the market is moving in a level" (range).
- 02:56-04:05: the key rule, said twice: when the market breaks out, wait for it to come back (pull back /
  **retracement**) and then enter. "**Never enter a sell straight away; always let it retrace first, then sell.**"
- 04:05-04:21: mostly garbled (mentions Bitcoin).

**Maps to:** S10 range, T8/T9 breakout + retest (the explicit rule), T11 sweep and reclaim (the sketch).

---

## trading.mp4 (31 min 35 s, 1920x1020) - the "second, larger video"

**Screen:** an Instagram video call; one participant shares a Chrome window with TradingView (Paper Trading).
On-screen clocks show **26 Nov 2026 ~19:19-19:51 UTC** (00:50-01:21 IST on 27 Nov). Sequence from OCR + frames:

| Video time | What is on screen |
|---|---|
| 00:00-03:40 | `XAUUSD` (OANDA) 4h. Fibonacci retracement drawn on the fall that starts late Aug 2026: levels 0 (4,284.648), 0.5 (4,455.861), 0.618 (4,496.267), 0.786 (4,553.795), 1 (4,627.074), also 1.618 (4,838.693) at 02:50. Price rallies back into 0.5-0.618 and turns down. Rectangle boxes drawn. Volume (ticks) pane. |
| 04:30-06:10 | Symbol search; `DXY` 4h (Tickmill); Forex Factory home and **calendar** for the week of 22-28 Nov and next week (Core PCE, Prelim GDP, ISM PMI, JOLTS, NFP, unemployment claims, OPEC meetings). |
| 06:20-09:50 | `XAUUSD` 5m. Adds **Supertrend (10, 3)**, then **EMA 9** and **EMA 21**; settings dialog for EMA length 21. Trendline and boxes drawn. |
| 10:10-11:10 | Paper trading panel (balance ~102,192 USD). **Market buy 2 XAUUSD at ~3,566.6**; take-profit sell 2 at **3,586.864** (+40.53 USD shown), stop at **3,558.642** (-15.92 USD shown). Reward:risk ~2.5:1. Outcome not shown. |
| 11:50-13:10 | A prop-firm dashboard (session win rates, market hours by session, account allocation); not described further for privacy. |
| 13:20-18:50 | `BTCUSDT` (Binance) 3m, 15m, 1D, with EMA 9/21. **Market sell 1.1813 BTCUSDT at 60,628.78**, take-profit buy at **60,398.27** (+~272 USD shown); later **buy 1.1813 at 60,636.93** (closed at a small loss, ~-9.6 USD by these prices). Draws a box at the range bottom ~60,170 on 15m. Forex Factory calendar again. |
| 19:10-25:30 | `NVDA` (NASDAQ) 5m/15m/30m/1m on a US holiday. Adds **VWAP (auto-anchored, hlc3) with stdev bands 1/2/3**, searches VWEMA, then **RSI 14**. |
| 25:40-31:20 | `BTCUSD` 5m, 4h, 1W with RSI 14: weekly candles from ~107k (Oct 2026) down to ~57-61k; RSI ~27-35 on the 4h/weekly lows. |

**Speech (paraphrase of readable parts):**
- 00:00-00:51 (gold 4h): the market has been falling since 27 Aug; it pulled back, and with Fibonacci the
  pullback reached the 0.5-0.618 ratio, "this is the **golden zone**", it recovered 50% and then fell again.
- 00:51-02:01: in a downtrend most of it is price action; points at a **double top**; if the market breaks this
  level it goes further down "because big players sit here".
- 02:20-03:20: **three kinds of trend: uptrend, downtrend and sideways** (the rectangle boxes = sideways). Mentions
  SMC terms: **break of structure** and **FVG** (fair value gap).
- 03:45-05:42: many charts exist (gold, EUR/USD, Bitcoin); claims gold tends to rise when the crude-oil price rises
  (stated as fact by the speaker, not checked).
- 05:42-06:17: must trade with the news: Federal Reserve news, employment data; identify the trend.
- 06:17-08:18: adds volume, then **Supertrend**: "it tells you the trend is buy-side from here to here, sell-side
  there"; suggests building exactly this kind of trend identification; adds a moving average and sets it to
  **21** ("it tells you about the previous 21 candles").
- 08:18-10:23: gold lot-size maths: with **0.1 lot a $1 move in gold = $10**; describes a ~$3-5 move and a ~$12 fall.
- 10:23-11:04: shows the paper buy: "this much, ~40 dollars, will be booked here"; once TP and SL are set "you have no
  tension, just set it and leave it; the market will hit SL or TP by itself".
- 11:32-13:03: today the US market is closed (holiday); explains sessions: Sydney (transcribed as "Sweden"), Tokyo,
  London, New York; the market is open 24 hours with different sessions; **most volume when London and New York overlap**.
- 13:56-14:51: sets up a BTC trade on the paper account; time frames used: **5 and 15 minutes**, also 1 day, 4 hours, 1 hour.
- 14:51-18:09: news moves the market a lot; on Forex Factory filter to the **red and orange (high/medium impact)**
  items; around news time the market moves a lot, then returns to normal.
- 18:31-19:28: calls some of it gambling; says the market often does the opposite of the news ("positive news, the
  market falls"); NVDA example: high on the event day then fell back within three days.
- 19:28-21:15 (**most relevant to this project**): "test it by building it"; take **one to two years of old data**
  (NVDA or anything) and analyse **where buying comes in again and again, where price repeatedly stops, where big
  selling sits**.
- 21:38-23:17: **VWAP** "tells you about the whole market, the market's 50%"; when price comes up to the level we
  sell, at the down side we buy (VWAP bands as mean reversion levels).
- 23:17-25:56: shows **RSI**, "a very good indicator"; reads overbought ("no more buying will come") and
  oversold ("no more selling") levels and reactions at them.
- 26:21-27:15: for the higher time frame take 2,000-3,000 old 4h candles to see where the market rose and fell;
  higher-time-frame data is more useful.
- 28:06-28:30: big funds look at higher time frames; on the weekly the market keeps falling and is trying to pull back.
- 29:30-30:03: suggests **posting / selling the indicator on TradingView** once it works.
- 30:03-31:32: on 4h/weekly BTC fell, then went sideways for about a month; a trend change could come; rest garbled.

**Maps to:** T7f Fibonacci (golden pocket), R2 double top, S8 trend types, S9 BOS, V1 EMA 9/21, V2 Supertrend,
V3 VWAP bands, V4 RSI, T12 fixed TP/SL. New ideas not in the text inputs, now listed as candidate filters for later
phases (not yet in `knowledge.md` detectors): **FVG** (three-candle gap: candle 1 high < candle 3 low for bullish),
**session filter** (London/New York overlap), **news blackout** around high-impact calendar events.

**Conflict with project rules:** the idea of selling the indicator on TradingView conflicts with the non-goals
(SEBI research-analyst rules in India, and the Binance Vision data licence is non-commercial). Recorded, not adopted.
