# Candlescope: handoff

A local indicator written in Rust. It studies two years of price history, tests every trading tactic against it, and shows only the setups that survive honest testing in a small always-on-top overlay next to the chart.

Last updated: 2026-11-27 (IST) after Phase 3 (backtest engine). Stage: Phase 3 done (event-driven backtester with costs, pessimistic fills, metrics and self-checks); the Phase 1 recorder is still running. Project root: the `imp` folder.

---

## 1. Current state

**Stage:** Phase 3 (backtest engine) finished. New code in `crates/cs-backtest` (engine), `crates/cs-stats` (block bootstrap) and a new command `candlescope backtest`. The engine turns detector events into trades: entry at the next open after the `known_at` candle has closed, or a limit order at the event zone with a timeout; stop at the structural level (event `lo`/`hi`, else the signal candle's low/high) plus 0.25 ATR; targets 1R / 1.5R / 2R / 3R or the next S/R or supply/demand zone edge; optional time stop and trailing stop; one trade per rule at a time by default (`OnePerDirection` / `Unlimited` available). Fills are pessimistic: stop on touch, target only on a trade beyond it; a stop gap fills at the worse open, a target gap at the target; a candle touching both is settled with its 1m candles, and the stop is assumed first when one 1m candle touches both or no 1m data exist; limit orders fill only on a trade-through, at the limit, and a target touch in the fill minute does not count. Costs: taker fee on every fill, half the spread plus fixed and ATR slippage on every market fill, historical funding (section 4 and `notes/data-sources.md`), with 1x/2x stress multipliers. Sizing: fixed fractional risk (default 1 %, leverage cap 10x); results in R and in % of account. Filters (HTF 1h/4h EMA stack and structure, healthy pullback, ADX regime, volatility percentile, UTC session/hour) exist and are tested; **none was chosen or tuned**. Metrics: count, win rate, average win/loss in R, expectancy (net and gross), profit factor, max drawdown (R and %), longest losing streak, exposure, time in trade, a circular block-bootstrap 95 % CI for expectancy, breakdowns by ADX regime (with/against/range), UTC entry hour and month; trades CSV + summary JSON with an inputs hash and a results hash. **Lookahead guard:** every run rebuilds the market (features, levels, events) on truncated data at seeded cuts plus the last candle and refuses the run if anything known at a candle changes (a cheating "next close" source is refused). **All 5 self-checks pass** (section 4 has the numbers), plus the `known_at` entry test and the filter-timing test. 90 tests pass (`cargo test --workspace -j 2`); fmt and clippy (`-D warnings`) are clean. **Sealed holdout:** `candlescope backtest` stops before the last 183 days (from 2026-05-27 00:00 UTC) unless `--open-holdout`; Phase 4 decides the final boundary before looking at any result. **No tactic outcome has been looked at:** on real data only the random-entry baseline (a control) was run with printed results; the benchmark ran all detectors x 7 rule templates but prints and writes no outcome numbers. Benchmark: a full run (2 years of 1m data, all 47 directional detectors x 7 templates x 2 cost levels = 658 rule runs, 657,230 trades) takes **5.2-5.5 s** (release build, one thread). Committed locally as "Phase 3: backtest engine".

**Stage:** Phase 2 (pattern engine, tier 1) finished. New code in `crates/cs-patterns`: 12 streaming detector groups emitting **52 detector names** (17 structural + all 35 candlestick patterns from `technical_analysis_patterns.txt`) plus per-candle context features. New command `candlescope detect --symbol BTCUSDT --tf 15m [--market um|spot]` writes `data/events/<market>/<SYM>/<tf>/events.csv` (134,041 events on BTCUSDT USDS-M 15m), `features.csv` (72,480 rows) and `gallery.json`, prints counts per detector with ZERO / RARE / ABSURD flags, measures throughput and re-checks prefix invariance on the real data; the report is `reports/detect-BTCUSDT-15m.md`. `python scripts/gallery.py <gallery.json> reports/detector-gallery` draws 10 random events per detector as SVG sheets (52 sheets + `index.html`, standard-library Python only). **Prefix invariance passes for every detector group and for the context features** (synthetic regime-switching data, 4 seeds, plus the real BTC 15m store; a deliberately cheating detector is caught, so the test has teeth). 140 hand-built candlestick fixtures pass (35 patterns x 2 positive + 2 negative). 61 tests in the workspace pass (`cargo test --workspace -j 2`); `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` are clean. Throughput of the full set + context: ~310-350k candles/s (release build, this laptop). Flags on BTC 15m: rising three methods (4 events), three black crows (13) and falling three methods (1) are RARE; nothing is ZERO or ABSURD. ETHUSDT and the 5m/1h time frames were also run (section 4): no ABSURD counts anywhere; only rare 3- and 5-bar candlestick patterns are ZERO on 1h. **No outcome or P&L of any detector has been looked at yet** (that is Phase 3). Nothing committed.

**Phase 2 state (unchanged):** the next two paragraphs.

**Phase 2 follow-up (same day):** (1) **Higher-time-frame trend state** (`crates/cs-patterns/src/htf.rs`): for a 5m/15m series, 1h and 4h EMA 20/50/200 stack and slopes plus the HTF swing-structure trend, built only from *completed* HTF candles (strict: every base candle of the bucket present; completed at the close of the base candle that ends the bucket, `known_at` = that candle's open time). It is a context feature (`Features::htf`, 16 new `h1_*` / `h4_*` columns in `features.csv`) and a 13th detector group `htf_trend` (4,389 state-change events on BTC 15m). (2) **Healthy-pullback flag** on `pullback` events (`tags` bit 16 = `pullback::HEALTHY`; impulse anchors in `ax/ay`, `bx/by`): smaller and slower than the preceding impulse, and no close beyond the last swing; 687 of 2,639 long and 662 of 2,553 short pullbacks on BTC 15m are healthy. Prefix invariance passes for both (the `htf_trend` and `pullback` groups, and the context features). Separate reference tests: the streaming HTF state equals a batch aggregation at every base candle (15m and 5m bases, data with missing candles), and the healthy flag equals a brute-force recomputation on 6,675 pullback events. 66 tests pass (`cargo test --workspace -j 2`); fmt and clippy clean. The detector gallery was **not** regenerated (no images were requested this time), so it still lacks `htf_trend` and the pullback anchors; run `python scripts/gallery.py data/events/um/BTCUSDT/15m/gallery.json reports/detector-gallery` to refresh it. All six `detect` runs (BTCUSDT, ETHUSDT x 5m/15m/1h) were re-run with the new features (prefix checks PASS); healthy share of pullbacks is 22-31 % in every run.

**Phase 1 state (unchanged):** Phase 1 (data layer and live recorder) finished. `candlescope` now has `fetch`, `validate`, `resample`, `record`, `record-status` and `record-stop`. About 24 months of 1m candles (Binance spot + USDS-M, BTCUSDT and ETHUSDT), funding and 5-minute open interest are downloaded, SHA-256 verified and stored in `data/store/` (1,087,200 one-minute rows per series, i.e. 755 complete UTC days from 2024-11-01 00:00 to 2026-11-25 23:59, for spot and USDS-M x BTCUSDT and ETHUSDT; plus 2,265 funding rows and 217,437 five-minute OI rows per symbol; data/ Phase 1 use ~483 MB: raw zips 210 MB, store 270 MB, live recordings 3 MB so far). `reports/data-quality.md` holds the validation counts. The **live recorder is running in the background** since 2026-11-26 22:14 UTC (see "How to keep the recorder running" below). 24 unit/golden tests pass; `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` are clean. Nothing committed.

**How to keep the recorder running** (heatmap tactics need weeks of it):
- It runs hidden as `data\recorder-bin\candlescope-recorder.exe` (started by `scripts\start-recorder.ps1`, created via WMI so closing a terminal or this session does not stop it). Check it: `data\recorder-bin\candlescope-recorder.exe record-status --data-dir data`. Stop it: `scripts\stop-recorder.ps1`.
- It stops on reboot, sign-out or shutdown, and records nothing while the laptop sleeps. Keep the laptop plugged in with sleep disabled while plugged in (Settings > System > Power), and keep the lid open. **Autostart is installed** (2026-11-27, at the user's request): the per-user scheduled task "Candlescope recorder" runs `scripts\start-recorder.ps1` at sign-in (no admin rights; it exits if a recorder is already running; tested once by hand: result 0, no second process). Remove it with `Unregister-ScheduledTask -TaskName "Candlescope recorder" -Confirm:$false`. At each sign-in it copies the current `target
elease\candlescope.exe` into `data
ecorder-bin`, so keep the release build working.
- **Observed first night:** the laptop entered Modern Standby 2026-11-27 00:10-02:26 UTC (lid or power button; the sleep timeouts are already "never"), so 2 h 16 min are missing from every live stream. The recorder survived and reconnected on wake. Afterwards the `/public` connection (bookTicker + depth20) was reset by the server seconds after each reconnect for several minutes (backoff reached 60 s); it also flapped twice at 22:55-23:00 UTC. Watch `connections.binance_um_public.disconnects` in `record-status`.
- **Fixed the same morning (2026-11-27 ~02:40 UTC):** an independent Python client was also reset on `bookTicker` (after 21 s), so the cause is on the server/network side, not our client. `bookTicker` was the problem load: ~1,200-1,500 msgs/s (~300 kB/s) of which the recorder kept 1 per second. `bbo_um` is now taken from level 1 of `<sym>@depth5@100ms` (~9 frames/s per symbol, ~7 kB/s total); lines keep the `bookTicker` keys (`s b B a A E T u`) plus `"src":"depth5@100ms"`, so files before ~02:40 UTC 2026-11-27 are raw `bookTicker` payloads and later ones are not (no `src` key = old format). Backoff also changed (`next_attempt`): a connection that delivered data and then dropped waits at most ~4 s, not up to 60 s; only connections that deliver nothing grow to the 60 s cap. Recorder restarted on the new build.
- Disk: about 22 MB/day gzipped (measured), capped at 20 GB (`--cap-gb`); the oldest files are deleted only if the cap is reached (about 2.5 years away).

**Previous stage (Phase 0):** Workspace of empty crates exists; `cargo build`, `cargo fmt --check` and `cargo clippy -D warnings` are clean and `target/debug/candlescope.exe` runs (prints a Phase 0 placeholder line). The first build attempt failed at linking on the `gnullvm` toolchain; fixed by installing VS 2022 Build Tools and switching to the msvc toolchain (section 5). No data downloaded except three tiny probe files in `data/probe/`. Nothing committed (no git repository was created; the user does version control).

**Input files** (all reviewed in Phase 0):

| File | What it holds |
|---|---|
| `technical_analysis_patterns.txt` | Pattern names only (reversal, continuation, neutral, candlestick, harmonic, advanced). Every name now has a testable definition in `notes/knowledge.md`. |
| `pumpglass_heatmap_trading_guide.pdf` | 3-page PumpGlass heatmap guide: liquidations, order book, whale entries; workflow = cluster, trend, OI, CVD, funding, wait for sweep + reclaim, define the trade first. Coded as H1-H6 in `knowledge.md`. |
| `Trading_Tactics_Study_Guide.pdf` | 15-page tactics guide (trend, S/R, supply/demand, trendlines, MAs, pullbacks, Fibonacci, breakouts, sweeps, risk). Coded as S1-S14, T1-T14. |
| `trading1.mp4` | 4 min 22 s screen recording, TradingView paper trading, BTCUSD 15m, recorded **26 Nov 2026 ~20:02-20:04 UTC** (01:30 IST on 27 Nov; the earlier handoff said 27 Nov UTC, which was wrong). A range is drawn as two boxes (top ~61,000, bottom ~60,150); the author sketches a spike above the range top and back (sweep), hits TradingView's 30-drawing limit and looks at the pricing page. Narration (Hindi, transcribed locally) states one clear rule: after a breakout never enter straight away, wait for the retracement, then enter (= T9). See `notes/video-notes.md`. |
| `trading.mp4` | **The "second, larger video" is now in the folder** (1.99 GB, 31 min 35 s, 1920x1020). It is an Instagram video call with screen sharing, recorded 26 Nov 2026 ~19:19-19:51 UTC. Charts shown: XAUUSD 4h with a Fibonacci retracement (0.5/0.618/0.786), XAUUSD 5m with EMA 9/21, Supertrend and a paper long with take-profit and stop, a prop-firm dashboard, Forex Factory calendar, BTCUSDT 3m/15m/1D with a paper short, NVDA 5m with VWAP stdev bands and RSI, BTCUSD 4h/1W. Personal data visible (call participant, e-mail): **not copied anywhere**. Narration (Hindi tutorial, transcribed locally) covers Fibonacci "golden zone", trend types, BOS/FVG, Supertrend, EMA 21, news filtering, sessions, VWAP, RSI, and "test on one to two years of old data"; it also suggests selling the indicator, which conflicts with the non-goals. |

**Notes written in Phase 0** (`notes/`): `knowledge.md` (testable definitions, OBJECTIVE/SEMI/SUBJECTIVE), `sources.md`, `data-sources.md` (live probes from India), `video-notes.md` (frames, OCR, transcript).

**Decisions made so far**
- **Phase 3 trade model:** a signal is decided at the close of its `known_idx` candle; the earliest fill is the next candle's open (the engine returns a lookahead error if an entry candle opens before that, and skips the signal across a data gap). Stops and targets come from raw prices only, so costs never change which trades happen (this makes cost monotonicity exact). R = |entry - initial stop| before costs, so a stopped trade loses slightly more than 1 R. Every fill pays the taker fee; target exits are treated as take-profit-market orders (spread + slippage). Trades still open at the end of the data close at the last close (flagged `END_OF_DATA`). Funding is charged for entry < funding time <= exit and is not scaled by the stress multiplier.
- **Phase 3 lookahead guard = prefix audit in the engine** (`market::audit_prefix`): the signal source, the context features and the level tracker are re-run on truncated data and must match. It samples cuts, so it catches systematic lookahead (the canary at any cut), not necessarily a source that cheats only at a few candles.
- **Sealed holdout default:** the last 183 days of the setup candles (now from 2026-05-27 00:00 UTC); the backtest command does not even load them unless `--open-holdout`. `--bench` includes them for timing only and prints no outcomes.
- **Bootstrap:** circular block bootstrap of net R in exit order, block length ceil(n^(1/3)), 2000 resamples, seeded (in `cs-stats`).
- **Cost defaults** (section 4 / `notes/data-sources.md`): taker 5 bps, spread 1 bp (0.5 per fill), slippage 1 bp + 0.02 ATR per market fill, historical funding; spot 10 bps fee, no funding.
- **Phase 2 event contract:** every event has `formed_idx/formed_at` (bar where the pattern completes) and `known_idx/known_at` (first bar whose close reveals it, always the bar being processed; usable from the next open). Detectors are `Detector::update(i, candle, &mut Vec<Event>)` state machines with fixed-size rings and capacity-bounded lists allocated at construction; no end-of-data flush. `Event` lives in `cs-core` (`event.rs`) so the backtester can use it. Each detector owns its building blocks (ATR, swings, zones), so groups are independent and individually testable; the cost is some duplicated work (fine at ~330k candles/s).
- **Phase 2 definitions:** every detector has a `*Params` struct with documented defaults and sensible ranges in its doc comments; the choices that pin down or deviate from `knowledge.md` are listed in its new section "Phase 2 implementation choices". They were fixed from definitions and event counts only, never from outcomes.
- **HTF registry:** `registry()` entries now take the series' candle length (`make(base_ms)`) and `default_set(base_ms)` / `ContextParams::for_base(base_ms)` build for it; `detect` passes the `--tf` length. An HTF that is not a larger multiple of the base (e.g. 1h on a 1h series) stays inactive (empty columns, no events).
- **Healthy pullback is a flag, not a filter**, so Phase 3 can test pullbacks with and without it (two hypotheses). "Slower" is coded as price moved per bar; the knowledge-base bar-count wording is a stricter variant.
- **Gallery images:** no matplotlib / Pillow on this machine and no installs were made, so the gallery is SVG written by a stdlib-only script (open `reports/detector-gallery/index.html` in a browser).
- Name: Candlescope. Language: Rust. Advisory only: never places orders, never holds keys.
- Data first: exchange feeds are the source of truth; the screen only identifies symbol and time frame.
- Start with BTC and ETH; NVDA later.
- Hard GATE after research: if nothing survives honest testing, stop; do not build the overlay on noise.
- **Data (Phase 0 decision):** Binance Vision archive is the primary history source (1m klines, funding, 5-minute OI metrics, ±1-5% book-depth bands); Bybit REST is the fallback. Liquidations and full order books have **no free 2-year history**, so Phase 1 starts a live recorder (Binance `/market` `!forceOrder@arr`, Bybit `allLiquidation`, Binance depth).
- **CVD** comes from the kline "taker buy base volume" column, not from aggTrades (~25 GB per symbol for 2 years).
- **OKX not used** (its terms restrict users by residency; it told Indian users to wind down in 2024).
- **Storage (Phase 1): own compact columnar format "CSC1", not Parquet.** Parquet in Rust means the `parquet` + `arrow` crates: well over 100 transitive crates, minutes of compile time and a much larger binary on a 12 GB laptop, for features not needed here (nested types, predicate push-down). CSC1 is ~300 lines in `cs-data/src/store.rs`: header + metadata (source, file, SHA-256, period) + per-column byte-shuffled values (timestamps delta-encoded) inside one zlib stream with an Adler-32 check. It reuses `flate2`, which the zip reader needs anyway. Cost: pandas/DuckDB cannot read it directly; add a CSV/Parquet export later if needed.
- **Repairs never happen in the store.** Store files are faithful copies of the source; repair rules R1-R5 (`cs-data/src/clean.rs`) run at load time and every touched row is counted in the report.
- **Resampling default is strict:** a 5m/15m/1h/4h/1d candle is emitted only when all its minutes exist (no partial candles, UTC buckets, open-time labels, volume summed). `Policy::AllowGaps` exists and every candle carries its minute count.
- **Phase 1 crates** (54 crates in the release tree, binary ~2 MB): `ureq` 3 (blocking HTTP) and `tungstenite` 0.30 (blocking WebSocket), both on the Windows-native TLS stack (`native-tls` = SChannel), so there is **no async runtime** and no C crypto build; `sha2` (archive checksums), `flate2` with the pure-Rust backend (zip inflate, store compression, gzip of recordings), `serde_json` (exchange JSON). The zip reader is our own (single-entry archives, CRC-32 checked) instead of the `zip` crate. Workspace MSRV raised to 1.89 for `File::try_lock` (recorder lock file).

**Environment (verified 2026-11-27):** Windows 11 Home (build 26200), 12 logical CPUs, 12 GB RAM. rustc/cargo 1.100.0, default toolchain `stable-x86_64-pc-windows-msvc` (installed in Phase 0; `gnullvm` also present but has no linker), Visual Studio Build Tools 2022 17.14 with the C++ workload and Windows SDK 10.0.26100 (installed in Phase 0 with the user's approval), git 2.55.0.windows.5, ffmpeg 9.0.2 (gyan.dev, via winget), Python 3.13, Tesseract 5.4.0.20240606 (installed in Phase 0 with the user's approval), pdftotext (Git for Windows' mingw64), faster-whisper 1.2.1 with the `large-v3-turbo` model cached locally, nvidia-cublas-cu12 12.9 + nvidia-cudnn-cu12 9.28 (so whisper runs on the RTX 3050 6 GB, driver 610.91), websockets (all pip, user site). Nothing required is missing.

---

## 2. Goal

(Unchanged in Phases 2 and 3. Phase 2 produces candidate events only; Phase 3 built the measuring tool; none is a signal until Phase 4 measures it after costs, out of sample.)

Build a local program that:
1. Downloads and checks about two years of candle data (and funding, open interest and liquidations where available).
2. Runs every tactic from the input files as a rule-based detector over that history, with no lookahead.
3. Measures each setup honestly: costs, slippage, walk-forward and a sealed holdout period.
4. Keeps only the setups that survive, and turns them into probabilities with confidence intervals and sample sizes.
5. Runs live beside the chart: reads which symbol and time frame is open, streams the live data, and shows a signal (long, short or wait) with entry, stop, target, expected holding time, probability and the reasons.

**The user's stated wish:** a win rate above 80%, mostly profits, "something nobody ever did".

**What the project will actually optimise, and why:** win rate cannot be chosen separately from reward and risk. A strategy that wins 80% of the time but makes 0.25R on a win and loses 1R on a loss has an expectancy of (0.8 x 0.25) - (0.2 x 1) = 0. After costs it loses money. So the success measure is **expectancy in R after costs, tested out of sample**, together with drawdown and calibration of the probabilities. Win rate is reported, never optimised alone. If the tool honestly finds that no tactic beats costs, that is a valid and valuable result, and the project reports it instead of tuning until the numbers look good.

**Non-goals**
- No automatic order placement, no broker or exchange keys.
- No sending screenshots or data off this computer.
- Not sharing or selling signals to others (in India that may fall under SEBI research-analyst rules; it would also breach the Binance Vision non-commercial data licence).
- No language-model calls in the live signal path (too slow and not repeatable).

---

## 3. Files

**Layout (created in Phase 0; crates are empty placeholders)**

```
imp/
  candlescope-handoff.md        this file
  CLAUDE.md                     standing rules (Phase 0)
  Cargo.toml                    workspace: resolver 3, edition 2024, shared lints and profiles
  rustfmt.toml, clippy.toml     formatting and lint settings
  .gitignore                    target/, data/, large report artefacts, *.mp4, secrets
  technical_analysis_patterns.txt, *.pdf, *.mp4    inputs (left in the root)
  notes/
    knowledge.md                testable definitions of every tactic and pattern
    sources.md                  web and local sources, tagged
    data-sources.md             probed data sources, limits, terms, primary/fallback
    video-notes.md              what the videos show and say
  crates/
    cs-core/                    types: Candle, Event, Signal, Trade; shared maths
    cs-data/                    downloaders, validators, storage, resampling, live recorder
    cs-patterns/                detectors (streaming, no lookahead), Phase 2:
      src/lib.rs                Detector trait, registry (12 groups), Runner (fills formed_at/known_at)
      src/util.rs               Ring, bounded_push, ATR, EMA, RollStats, ADX, candle parts, SplitMix64
      src/swing.rs              fractal swings (S2), ATR ZigZag (S3)
      src/structure.rs          HH/HL/LH/LL + trend (S8), BOS / CHoCH (S9)
      src/zones.rs              S/R zones + role reversal (S11, T3), supply/demand (T4)
      src/trend.rs              EMA 20/50/200 state and slope (T1), trendlines (T5)
      src/htf.rs                higher-time-frame (1h/4h) EMA + structure state from completed HTF candles
      src/fib.rs                Fibonacci retracement/extension from the latest ZigZag leg (T7f)
      src/pullback.rs           pullback to value + trigger (T6)
      src/breakout.rs           range boxes (S10), breakout (T8), retest (T9), fakeout (T10)
      src/sweep.rs              equal highs/lows, prev day/week H/L pools, sweep + reclaim (T11)
      src/candles.rs            all 35 candlestick patterns (K1-K32), ATR-relative
      src/context.rs            ATR, ADX regime, volatility percentile, session, zone distance, volume z
      src/testutil.rs           candle builders + synthetic regime-switching market
      tests/prefix.rs           prefix-invariance + alternative-future tests per group, negative control
      tests/candles.rs          140 hand-built candlestick fixtures
      tests/htf.rs              HTF state vs batch reference (gaps, 15m/5m), healthy flag vs brute force
      tests/scenarios.rs        hand-built sweep / breakout / retest / fakeout / demand / fib / trendline cases
    cs-backtest/                event-driven simulator, costs, metrics (Phase 3):
      src/lib.rs                Error (Lookahead / Input), backtest() = simulate + summarize + hashes
      src/market.rs             SignalSource trait (DetectorSource, RandomSource, FixedSource), Market::prepare
                                (candles + context features + S/R levels + events), audit_prefix (lookahead guard)
      src/rule.rs               TradeRule: Select, Entry (NextOpen / Limit), StopRule, Target, Trail, Overlap, Filter
      src/sim.rs                simulate(): entries, pessimistic fills, 1m drill-down, trailing/time stops,
                                funding, sizing and equity; Trade, Skips, FillStats
      src/cost.rs               CostModel (BINANCE_UM / BINANCE_SPOT / ZERO), Fill, Funding
      src/metrics.rs            Summary, breakdowns, block-bootstrap CI, sanity flags (SUSPECT_LOOKAHEAD ...)
      src/output.rs             trades CSV, summary JSON, market/data fingerprints, inputs and results hashes
      src/testkit.rs            synthetic 1m random walks (martingale) and regime walks, strict aggregation, funding rows
      tests/selfcheck.rs        the 5 self-checks + known_at entry tests + filter-timing test
      tests/fills.rs            1m ambiguity, gaps, limit fills, trailing, time stop, funding window, overlap,
                                leverage cap, cost identity
      tests/common/mod.rs       hand-built candle and 1m helpers
    cs-stats/                   walk-forward, multiple-testing control, bootstrap, probability model
      src/bootstrap.rs          circular block bootstrap CI of a mean (Phase 3); src/lib.rs SplitMix64
    cs-live/                    websocket ingest, incremental engine, signal logger
    cs-ui/                      always-on-top overlay
    cs-vision/                  screen capture and OCR of symbol and time frame (local only)
    cs-cli/                     binary "candlescope": fetch, validate, resample, record, record-status, record-stop (Phase 1),
                                detect (Phase 2), backtest (Phase 3, src/backtest.rs: holdout sealing, rule templates, --bench)
  scripts/
    start-recorder.ps1          start the recorder hidden (copies the exe to data/recorder-bin first)
    stop-recorder.ps1           clean stop (writes data/live/STOP, waits, then kills if needed)
    install-recorder-autostart.ps1   scheduled task at sign-in (installed 2026-11-27)
    gallery.py                  detector gallery: gallery.json -> SVG sheets + index.html (stdlib only)
    spread-stats.py             spread and book-walk cost stats from the live recorder (stdlib only, Phase 3)
  data/                         git-ignored
    raw/binance-vision/...      verified archive zips + .CHECKSUM files, same paths as on data.binance.vision
    store/klines/<spot|um>/<SYM>/1m/<YYYY-MM | YYYY-MM-DD>.csc   1m candles, one file per archive file
    store/funding/um/<SYM>/<YYYY-MM>.csc      funding (current month from REST, marked partial, refreshed each fetch)
    store/metrics/um/<SYM>/<YYYY-MM-DD>.csc   5-minute OI and long/short ratios
    store/resampled/<spot|um>/<SYM>/<tf>.csc  5m, 15m, 1h, 4h, 1d (strict)
    live/<stream>/<YYYY-MM-DD>/<stream>-<date>T<HH>-<HHMMSS>.jsonl(.gz)   recorder output, hourly files
    live/status.json, recorder.log, recorder.lock, STOP (stop request)
    recorder-bin/               copy of the recorder exe that is running
    logs/                       fetch logs
    events/<market>/<SYM>/<tf>/ events.csv, features.csv, gallery.json (Phase 2 `detect`)
    backtest/<market>/<SYM>/<tf>/ runs.csv + <rule>-x<mult>/trades.csv, summary.json (Phase 3 `backtest`)
    probe/, video/              Phase 0
  reports/
    data-quality.md             `candlescope validate` output (Phase 1)
    fetch-summary.md            last `candlescope fetch` run
    detect-<SYM>-<tf>.md        `candlescope detect` counts per detector/kind, flags, throughput (Phase 2;
                                BTCUSDT and ETHUSDT x 5m, 15m, 1h; gallery images only for BTC 15m)
    detector-gallery/           <detector>.svg x 52 + index.html (git-ignored artefacts)
    backtest-baseline-BTCUSDT-15m.md   random-entry baseline on real data (control), in-sample only (Phase 3)
    backtest-bench-BTCUSDT-15m.md      timing of a full run; no outcome numbers (Phase 3)
  crates/cs-data/src/           bybit, clean (R1-R5), fetch, http (polite client), parse, record, resample, store (CSC1), validate, vision, zipread
  crates/cs-data/tests/         golden.rs + fixtures/ (hand-built 90-minute archive with a header, a duplicate, a swap, a gap, a zero-volume candle, a 2% wick and a microsecond row; golden-pipeline.txt)
```

Workspace lints: `unsafe_code = forbid`, clippy `all` + `pedantic` as warnings, `float_cmp` and `dbg_macro` denied, `unwrap`/`expect` warned. Dev profile compiles dependencies at opt-level 2 for faster backtests. Internal crates are wired as used: cs-patterns -> cs-core; cs-backtest -> cs-core, cs-patterns, cs-stats (+ sha2 and serde_json, both already in the tree, so Phase 3 added no new third-party crate); cs-cli -> all of them.

**Data flow**

```
Binance Vision archive / Bybit REST --> cs-data --> candles, funding, OI, depth bands on disk
                                                        |
                  cs-patterns (events) <----------------+--> cs-backtest --> cs-stats --> surviving setups + probabilities
                                                                                               |
Binance /market + Bybit websockets --> cs-live (recorder + incremental detectors) ----------> signal --> cs-ui overlay
screen --> cs-vision (symbol, time frame, price check, local only) --> picks the feed
```

---

## 4. What worked

- **All inputs reviewed:** the pattern list, both PDFs (pdftotext) and **both videos** (frames every 10 s: 26 + 189; OCR of every frame with the Windows OCR engine and with Tesseract 5.4; a sample of frames viewed directly; local speech-to-text with faster-whisper). The inputs agree: patterns are context, confirmation is needed, outcomes must be tested.
- **knowledge.md** gives every named item (about 120) a codable definition with its confirmation rule, classed OBJECTIVE / SEMI / SUBJECTIVE, plus a tier-1 build order for Phase 2.
- **Data from India (live probes, 26 Nov 2026 ~20:50 UTC):** Binance Vision archive (klines, funding, metrics with 5-minute OI, bookDepth), Binance REST, Binance `/market` WebSocket (markPrice, `!forceOrder@arr`), Bybit REST (kline, funding history, OI history back to 2024) and Bybit WebSocket all work. Archive checksums matched.
- **About 1 GB zipped** covers the primary 2-year BTC+ETH set, which fits this laptop.
- **Workspace:** nine crates; `cargo build`, `cargo fmt --check` and `cargo clippy --workspace --all-targets -- -D warnings` all pass with zero warnings on the msvc toolchain.

---

- **Phase 1 data layer:** `fetch` stored all 1,756 archive periods (196 kline files, 48 funding months + the current month from REST, 1,510 daily OI files) in one ~33-minute run: 3,510 requests, 0 failures, 0 retries, 0 rate-limit pauses, 0 checksum mismatches, 0 fallbacks needed; every archive file matched its SHA-256 `.CHECKSUM`. Re-running `fetch` skips everything already stored (resumable) and an interrupted download resumes with HTTP Range (tested: half-written `.part`, server answered 206, checksum matched). The Bybit fallback was exercised with `--source bybit` on one day (all datasets stored, tagged `source=bybit`).
- **Validation** (`reports/data-quality.md`): no gaps, no duplicates and no out-of-order candles in any of the four 1m series. USDS-M has 37 (BTC) / 38 (ETH) zero-volume minutes in runs at the same minutes on both symbols (2025-01-14, 2025-01-29 01:23-02:44, 2025-08-29 06:19, 18 min), most likely exchange outages that the archive fills with flat candles; spot has none. Outlier wicks (kept, flagged): 10 spot BTC, 18 um BTC, 29 spot ETH, 36 um ETH. R1 converted 999,360 spot rows per symbol from microseconds (every spot row since 2025-01-01). Funding: complete, no interval gaps. OI: 3 missing 5-minute slots and 31-33 empty OI values per symbol; R3 sorted ~8,800 unsorted metrics rows per symbol. Resampling: every 5m/15m/1h/4h/1d bucket is complete (217,440 / 72,480 / 18,120 / 4,530 / 755 candles per series).
- **Resampling:** unit tests on hand-built fixtures (OHLCV values, UTC 4h/1d alignment, edge and internal-gap policies) plus the golden pipeline test all pass.
- **Live recorder:** 4 WebSocket connections (Binance USDS-M `/market` and `/public`, Binance spot, Bybit linear) + open-interest polling, 10 data streams, hourly gzipped files, disk cap, status file. Reconnect tested by forcing a drop every 30 s for 80 s: every connection reconnected (3 connects, 2 drops each) with backoff and no lost files. Disk use measured at about 22 MB/day.

- **Phase 2 pattern engine:** all 12 groups pass prefix invariance (8 random cuts on each of 4 synthetic series and 6 on real BTC 15m, plus 2 cuts per series with a different future appended); `detect` repeats the check on the real data for the full set + features (3 cuts, PASS). The negative-control detector that peeks 3 bars ahead is caught at 22 of 79 cuts. Candlestick fixtures 140/140, scenario tests 7/7.
- **Event counts, BTCUSDT USDS-M 15m (72,480 candles, per 1000 candles):** swing 196, structure 298, bos 49, choch 21, sr_zone 184, role_reversal 57, sd_zone 12, ema 72, trendline 204, fib 124, pullback 72, range 5, breakout 45, retest 20, fakeout 54, eq_levels 15, sweep 15; candlesticks from 0.01 (falling three methods) to 133 (doji). Full table: `reports/detect-BTCUSDT-15m.md`.
- **Throughput:** full set + context ~310-350k candles/s; each group alone: fib 23M, ema 19M, swing 16M, structure 12M, sd 12M, bos/choch 12M, sweeps 7M, trendline 6M, sr_zones 4M, candles 4M, breakouts 2.0M, pullback 1.7M, context 1.4M candles/s. Two years of 15m candles take ~0.25 s.
- **All six runs (BTCUSDT, ETHUSDT x 5m / 15m / 1h, USDS-M):** every run exits cleanly and passes the 3 prefix checks on its own data. Events: 406,474 / 134,041 / 34,204 (BTC 5m / 15m / 1h) and 405,335 / 134,690 / 33,826 (ETH). Rates per 1000 candles are stable across symbols and time frames for the structural detectors (e.g. swing 194-202, bos 47-51, choch 18-22, pullback 68-73, breakout 40-49, eq_levels 15-17), so the ATR scaling holds. Two rates move with the time frame, both as expected from the definitions: sweeps rise from ~11 (5m) to ~34 (1h) because a 1h bar covers far more of the previous day's range, and marubozus fall from ~22 (BTC 5m) to ~5 (1h) because the shadow limit (0.05 ATR) is absolute and higher-time-frame bars almost always have wicks. ZERO on 1h (18,120 candles): rising three methods (BTC), three white soldiers and three black crows (ETH); several other multi-bar patterns are RARE on 1h. Not bugs (the fixtures fire), but these patterns cannot be tested on 1h on their own.

- **Phase 2 follow-up:** HTF state on BTC 15m: 1h state on 72,477 candles (EMA ready 98.9 %, bull stack 22.0 %, bear stack 21.9 %, structure up 29.8 % / down 28.7 %), 4h state on 72,465 (EMA ready 95.5 %, bull 20.0 %, bear 22.8 %, up 30.3 % / down 28.8 %); mean lag 1.50 / 7.50 base candles, which is exactly what completed-candle-only use implies (0-3 and 0-15 candles). Spot check: at 2024-11-11 09:30 the 1h state is from the 08:00 candle (known 08:45) and the 4h state from 04:00 (known 07:45). Pullback events are unchanged in number (5,192); the flag only adds information. Throughput of the full set + context ~310-320k candles/s (htf_trend alone 16M candles/s).

- **Phase 3 self-checks (all pass; `cargo test -p cs-backtest --test selfcheck -- --nocapture` prints the numbers):**
  1. *Hand-calculated scenario:* two trades on hand-built candles with ATR exactly 2.0 (fee 5 bps, spread 2 bps, slippage 1 bp + 0.1 ATR, funding 1 bp). Long: entry fill 100.22, target exit fill 103.7792, net **+1.7286002 R**; short: stopped at 106 (fill 106.2212), receives funding at 08:00 (-0.005225 R), net **-1.2682751 R**; account 10,000 -> 10,043.84 (+0.4384 %); every component, the equity path, expectancy 0.23016255 R, PF and drawdown match the hand values to 1e-9.
  2. *Random-entry baseline* (1 year of a driftless 1m random walk, 15m candles, random direction, stop 1 ATR, target 1 R, Binance UM costs): 9,059 trades, gross **+0.0034 R (SE 0.0105)**, net **-0.4142 R**, mean cost 0.4176 R vs 0.4179 R predicted from the cost parameters alone; the 95 % CI [-0.4339, -0.3926] contains minus the costs. On **real data** (control, in-sample only): BTCUSDT 15m 12,325 trades, gross **-0.0036 R (SE 0.0090)**, mean cost 0.5513 R (fee 0.394, spread 0.039, slippage 0.118, funding 0.0001), net **-0.5548 R** (CI -0.5824 .. -0.5262); ETHUSDT 15m 12,078 trades, gross -0.0088 R (SE 0.0091), cost 0.3467 R, net -0.3555 R. Gross is zero within noise and net = minus costs, as it must be.
  3. *Lookahead canary:* a source that decides at candle k from the close of k+1 is refused by the engine's prefix audit ("3404 events known before candle 3404 in the full run, 3403 when the data ends there"); forced through anyway it shows win rate 100 %, t-stat 102.9, PF inf at zero cost (t-stat 23.5 at 1x costs) and is flagged `SUSPECT_LOOKAHEAD`. The random source and the real detector set pass the same audit.
  4. *Cost monotonicity:* 20 rules (5 detectors x 2R / next level / limit / trailing + time stop) x 6 cost levels (0, 0.5, 1, 1.5, 2, 4x): identical trade sets, every trade's net R non-increasing, expectancy non-increasing (e.g. fakeout-2r on synthetic data -0.054 / -0.185 / -0.315 / -0.445 / -0.576 / -1.097 R); raising fee, spread, fixed or ATR slippage alone lowers expectancy.
  5. *Determinism:* the full pipeline (prepare, audit, 20 rules, metrics, bootstrap, CSV, JSON) twice gives identical hashes and text; another bootstrap seed changes the hash. The CLI repeats the same hashes run to run (baseline `d667...1c58`, bench `cf75...35e2`).
  - *known_at entry:* 11,148 trades over 21 rules: none entered before the close of its `known_at` candle; next-open entries are exactly at that close and at candle k+1's open; a hand-built case where entering on the signal candle would win shows entry on k+1 (next open) / k+2 (limit) and a loss; an inconsistent `known_at` or `formed_idx > known_idx` is refused.
  - *Filters:* 20 filter variants x 5,833 events x 4 cuts: identical decisions with the data truncated after the cut and with a completely different future appended; every filter passes some events and rejects others; HTF `known_at` <= the decision candle at every candle.
- **Phase 3 fill tests (10, `tests/fills.rs`):** 1m resolution (target first / stop first / both in one minute -> stop / no 1m -> stop / mismatching 1m -> unused), gaps (stop at the worse open, target at the target), touch vs trade-through, limit fills (no fill on an exact touch, fill at the limit, target ignored in the fill minute, stop counted, expiry blocks overlap, marketable limit at the open), trailing stop effective from the next candle, time stop at the open, funding window (entry-exclusive, exit-inclusive, ms past the minute), overlap policies, leverage cap, and a cost identity on >1,000 detector trades (one half-spread, one slippage, one fee per fill).
- **Real-data intrabar resolution:** on BTCUSDT 15m random trades, 189 exit candles touched both stop and target; 1m candles settled 187, only 2 needed the stop-first fallback (ETH: 140 / 6).
- **Benchmark (release, one thread, BTCUSDT USDS-M 15m, all history):** load 15m 0.03 s, load 1,087,200 1m candles 0.69 s, funding 0.01 s, prepare (all detectors + context + levels) 0.24 s, lookahead audit 3 cuts 0.39 s, fingerprints 0.08 s, 658 rule runs (47 detectors x 7 templates x 2 cost levels, 657,230 trades, metrics + 2000-resample bootstrap each) 3.90 s; **total 5.24-5.46 s** (`reports/backtest-bench-BTCUSDT-15m.md`).
- **Costs measured from the recorder** (`scripts/spread-stats.py`, ~6 h): BTCUSDT spread median 0.017 bps, ETHUSDT 0.059 bps, max 0.96 / 1.59 bps; a $10k market order fits in the top 20 levels 99.4 % / 99.9 % of the time.

## 5. What did not work, or was ruled out

- **Phase 3:** (1) The first hand calculation in self-check 1 was wrong (1.7285002 R instead of 1.7286002 R); the test failed and re-adding by hand confirmed the engine. (2) The synthetic regime walk produces no `breakout` events (no tight range boxes), so the cost-monotonicity rule set uses `fakeout`; a zero-trade rule gives NaN expectancy. (3) A large `serde_json::json!` literal hit the macro recursion limit; the summary JSON is built key by key. (4) LNK1104 (locked test executable) happened once more; an immediate retry linked. (5) The taker fee (5 bps) is from the published Binance schedule as known in mid-2026 and was **not re-checked online** (network use is limited to market data). (6) Spread and depth were measured over one quiet ~6 h window only. (7) **Costs dominate short time frames:** random entries with 1-ATR stops on BTC 15m pay 0.55 R per trade (fee alone 0.39 R); any 15m tactic with stops near 1 ATR needs a gross edge above that.
- **Phase 2 follow-up:** LNK1104 (locked test executable) also happened once with `-j 2`, right after the same test binary had been rebuilt; an immediate retry linked. The healthy-flag reference test first took 45 s (brute-force swings recomputed per event); swings are now computed once per data set (1.5 s).

- **Phase 2, the first S/R zone version gave absurd counts:** zones overlapped and were sliced through repeatedly (one bar broke up to 25 zones; ~13 breaks per zone; role_reversal 12,071 and breakout 7,168 events). Fixed by merging overlapping zones, retiring a zone after more than 2 breaks, expiring on pivot touches only, requiring price to leave the zone (0.5 ATR) before a role-reversal retest, and emitting at most one breakout/fakeout candidate per bar and direction (outermost level). Now role_reversal 4,151, breakout 3,271, fakeout 3,948. These are still high-frequency events; they are candidates, and Phase 3 decides whether any of them is worth anything.
- **Range box touches:** counting touches as runs of touching bars made a flat box count as one touch (a scenario test caught it); touches are now counted at least 3 bars apart.
- **Context features were the bottleneck** (0.44M candles/s, from a 500-bar percentile scan); a sorted window with binary search made them 1.4M candles/s and the full set went from ~190k to ~330k candles/s (checked against brute force in a unit test).
- **`cargo test --workspace` with default parallelism failed to link** (`LNK1104: cannot open file ...out\*.exe`) twice in a row: Windows held freshly linked test executables (most likely antivirus scanning). `cargo test --workspace -j 2` links and passes. Not a code problem.
- **Rare candlestick patterns on BTC 15m:** rising three methods 4, three black crows 13, falling three methods 1 (the 5-bar "three methods" need three small inside bars between two long bars plus the right trend, which 24/7 crypto rarely prints). Too few samples to test alone; use them only as features or pooled.
- **No matplotlib / Pillow** in the Python install; the gallery is SVG instead of PNG (no installs made).

- **Linking binaries (fixed):** the first `cargo build` failed with `linker x86_64-w64-mingw32-clang not found` because Rust was on `stable-x86_64-pc-windows-gnullvm` with no linker. Fixed in Phase 0 by installing VS 2022 Build Tools (C++ workload, winget `Microsoft.VisualStudio.2022.BuildTools`) and running `rustup default stable-x86_64-pc-windows-msvc`.
- **Liquidation history:** Binance `liquidationSnapshot` archive returns 404 (discontinued); REST `allForceOrders` returns 404; CoinGlass needs a key; Tardis is free only for day 1 of each month. **The heatmap tactics (H1, H2) cannot be backtested on 2 years of free data**; forward-test them on data recorded from Phase 1 onward.
- **Binance `openInterestHist` REST** rejects startTime older than ~30 days; use the archive's `metrics` files.
- **Binance futures legacy WebSocket path** (`/ws/...`) connected but delivered nothing for markPrice and forceOrder; the `/market/ws/...` path works. Phase 1 must use the new paths.
- **Binance forceOrder stream** sends only the largest liquidation per symbol per second, so it undercounts; record Bybit `allLiquidation` as well.
- **Kraken** OHLC serves only ~720 recent candles; **Hyperliquid** serves only recent candles; **OKX** funding history ~3 months and doubtful eligibility from India. Not used.
- **Stooq** puts a JavaScript bot check in front of downloads, so it is not used (no bypassing).
- **Yahoo Finance**: 1m data only for the last 30 days; unofficial, personal use only.
- **NVDA** free intraday sources (Alpaca, Polygon/Massive, Alpha Vantage, Twelve Data, Tiingo) all need a free account and key, which the user must create.
- **Speech-to-text:** Whisper `small` mis-transcribed the Hindi narration and stopped after 26 s; PyAV in faster-whisper was incompatible (worked around by reading the WAV directly); `large-v3-turbo` on CPU was too slow for 31 min (~5 min of audio per 45 min). Fixed by running on the GPU (502 s for the whole video). The transcript is still only partly readable; see `video-notes.md`. A Tesseract winget install was cancelled once (the Windows prompt), then installed on retry with the user's approval.
- **Phase 1 notes:** Binance spot archive files switched to **microsecond** timestamps on 2025-01-01 (rule R1 converts them, counted per row). USDS-M kline files have a header line, spot files do not. Daily `metrics` files are unsorted (rule R3). `bookTicker`/`depth` exist only on the futures `/public` path, the other streams only on `/market`. `!forceOrder@arr` covers all symbols. Bybit `allLiquidation` was quiet for BTC/ETH in the first minutes (subscription acknowledged); check its line count after a few days. The laptop clock ran ~1 s behind Binance event times: use exchange timestamps for analysis.
- **Liquidation history (re-checked in Phase 1): still none free** (Binance archive 404, REST 404, Bybit REST 404, CoinGlass needs a key). Recording started 2026-11-26 22:14 UTC; H1/H2 forward tests need weeks to months of it.
- **Recorder gaps are real gaps:** while disconnected (a reconnect takes ~2-3 s, longer after sleep) trades and liquidations are missed; `agg_trade_id_gaps` in `status.json` counts the missed aggTrade ids. Closed 1m candles can be backfilled from the archive later.
- **bookDepth (+-1-5% bands) not downloaded yet:** implemented (`fetch --book-depth`, ~0.7 GB) but left out of the default run to keep Phase 1 lean; fetch it when Phase 2/3 needs depth filters.
- Still ruled out from planning: reading candles from screen pixels, optimising for >80% win rate, scraping PumpGlass, languages "faster than Rust".

---

## 6. What is still to do

Each phase runs in its own Claude Code session (new session, same folder). See `candlescope-phase-prompts.md` for the prompts and effort levels (that file is not in this folder; the user keeps it).

- **At the end of each phase, make one local commit with a short message and no Claude attribution. Never add a remote, never push, never open a PR.** (Git repository created 2026-11-27 on branch `main`; first commit covers Phases 0-2.)

- [x] **Phase 0** Kickoff: read inputs, knowledge notes, workspace, CLAUDE.md, data-source research
- [x] **Phase 1** Data layer and live recorder: `fetch` (Vision primary with SHA-256, Bybit fallback, resumable, rate-limit aware), CSC1 store, `validate` + `reports/data-quality.md`, strict resampling to 5m-1d, liquidation-history check (none), `record` running in the background.
  - Follow-ups: keep the recorder running (sign-in task installed; keep the lid open / laptop awake); after ~1 week check `record-status` (liq_bybit lines, reconnect counts, agg-trade gaps); fetch `--book-depth` if Phase 2/3 wants depth filters; re-run `candlescope fetch` + `validate` before Phase 4 to extend the history (only new days download).
- [x] **Phase 2** Pattern engine, tier 1 detectors: market structure (swings, HH/HL/LH/LL, BOS, CHoCH), S/R zones + role reversal, supply/demand (fresh/tested), EMA 20/50/200 state + slope, trendlines, pullback to value + trigger, Fibonacci retracement/extension, breakout + retest, fakeout, liquidity sweeps (equal H/L, prev day/week H/L), all 35 candlestick patterns, context features; `detect` command; gallery.
  - Follow-ups: **the user should look through `reports/detector-gallery/index.html`** and say which detectors look wrong (orange dashed line = formed, blue line = known). Not yet coded from the knowledge.md tier-1 list: A2 1-2-3 reversal, A10/A11 spring/upthrust as their own detectors (range-edge pools for the sweep), using the higher-time-frame trend as a T6 pullback condition (the healthy-pullback flag and the HTF trend state themselves were added in the follow-up). `detect` has been run on BTCUSDT and ETHUSDT x 5m/15m/1h (reports in `reports/detect-*.md`). Phase 3 must fill at the next open after `known_at` + one candle, and count every parameter variant tried.
- [x] **Phase 3** Backtest engine with costs: `cs-backtest` (trade model, pessimistic fills with 1m resolution, costs with 1x/2x stress, sizing, filters, metrics, block-bootstrap CI, breakdowns, trades CSV + summary JSON + hashes), prefix-audit lookahead guard, all 5 self-checks + known_at + filter tests, `candlescope backtest` with a sealed holdout and `--bench`.
  - Follow-ups for Phase 4: (1) **decide the holdout boundary before looking at any result** (default sealed from 2026-05-27; do not move it earlier into inspected data) and open it once; (2) count every rule x detector x filter x cost variant toward the multiple-testing budget: `candlescope backtest --rules all` runs 7 fixed templates per detector (47 directional detectors on BTC 15m = 329 rules); the Phase 3 bench ran them for timing only and nothing was inspected; (3) expect costs to kill most 15m rules (0.55 R per trade at 1-ATR stops); test 1h and wider stops too; (4) verify the 5 bps taker fee on Binance's fee page and re-run `scripts/spread-stats.py` after a few weeks of recording; (5) walk-forward and multiple-testing control in `cs-stats`; (6) the detector gallery still lacks `htf_trend` (regenerate if images are wanted).
- [ ] **Phase 4** Research, validation and the GATE report
- [ ] **GATE** Do any setups survive? If not, stop or change the hypotheses.
- [ ] **Phase 4b (optional)** Tier 2 and tier 3 patterns, then re-run the research
- [ ] **Phase 5** Probability model and live engine with latency benchmarks
- [ ] **Phase 6** Overlay UI
- [ ] **Phase 7** Screen awareness: symbol, time frame, price check (local OCR only; Tesseract is now installed, Windows OCR is also available)
- [ ] **Phase 8** Forward paper test and final report; optional stocks extension (NVDA: Alpaca free primary, Polygon/Massive free fallback; needs the user's free keys)

**Open questions for the user**
- Account size, risk per trade and fee tier: Phase 3 defaults to a 10,000 account, 1 % risk, a 10x leverage cap (`--equity`, `--risk`, `--max-leverage`) and VIP 0 taker fees without the BNB discount (5 bps on USDS-M). Are these right for you?
- Which time frames matter most (suggested start: 5m, 15m, 1h)? The videos show 15m BTC and 5m XAUUSD/NVDA.
- Account size and risk per trade, for position-size output (the tool will ask, and default to 1%).
- Spot or perpetual-futures prices as the reference for BTC/ETH signals? (Perps carry funding and liquidations; the videos use TradingView's BTCUSD index and Binance BTCUSDT.)
- Gold (XAUUSD) appears in the video: should it be in scope later? (No free 1m history source was checked for it.)
- A check of the Hindi transcript paraphrase in `video-notes.md` by the user (roughly half the segments were garbled), if any rule there should become a detector.
- FVG, session filter and news blackout (from the video, V6-V8 in `knowledge.md`): in scope for tier 2?
