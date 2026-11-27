# Data sources

Phase 0, 2026-11-27. Every "probe" row below is a small real request made from this laptop. The
request came from an Indian IP (ipinfo country = IN) through a generic client name; no keys, accounts or
personal data were used. The probes ran around 20:45-20:55 UTC on 2026-11-26 (02:15-02:25 IST, 27 Nov; server timestamps in the responses confirm it). Rate limits marked "docs"
come from the vendors' documentation and were not measured. Sources are listed in `sources.md`.

## Summary: primary and fallback

| Data type (BTC, ETH) | Primary | Fallback | Depth of free history |
|---|---|---|---|
| 1-minute candles, 2 years | **Binance Vision archive** `data.binance.vision`, monthly + daily zips, spot and USDS-M futures (`klines/1m`), with SHA-256 checksums | **Bybit REST** `/v5/market/kline` (linear), then Binance REST `/fapi/v1/klines` for gaps | Years (BTC spot from 2017). Probed 2024-11 and 2026-10 monthly zips plus 2026-11-25 daily: all 200 OK. |
| Funding history | **Binance Vision** `futures/um/monthly/fundingRate` (90 rows/month at 8 h) | **Bybit REST** `/v5/market/funding/history` (paged by endTime; 2024-11 returned OK) | Full history. OKX REST gives only ~3 months (probe returned empty for 2024-11). |
| Open-interest history | **Binance Vision** `futures/um/daily/metrics`: 5-minute OI, OI value, top-trader and global long/short ratios, taker buy/sell ratio (288 rows/day) | **Bybit REST** `/v5/market/open-interest?intervalTime=5min` with startTime (2024-11 returned OK) | 2+ years. Binance REST `openInterestHist` rejects startTime older than ~30 days (probe: code -1130), so use the archive, not REST. |
| Liquidation history | **No free 2-year history exists.** Record live from now: **Binance** `wss://fstream.binance.com/market/ws/!forceOrder@arr` (works; got 2 events in 60 s) | **Bybit** `wss://stream.bybit.com/v5/public/linear` topic `allLiquidation.{symbol}` (subscribe OK; no BTC/ETH event in the 60 s window); OKX `liquidation-orders` (subscribe OK, see India caveat) | Binance archive `liquidationSnapshot` returns 404 for 2023 and 2024 dates (discontinued). Binance REST `allForceOrders` returns 404. Binance's stream sends only the *largest* liquidation per symbol per second [BN-WS], so it undercounts; Bybit's stream is meant to carry all of them. Tardis.dev has the history but is free only for the first day of each month; CoinGlass needs a key (paid tiers). |
| Order-book history | **Binance Vision** `futures/um/daily/bookDepth`: cumulative depth and notional at +-1..5% from mid, about every 30 s (28,800 rows/day, ~0.5 MB/day zipped) | **Bybit order-book archive** (`ob500` daily file, 500 levels; one day for BTCUSDT 2024-11-27 = 82 MB zipped) for selected days only | bookDepth is coarse (bands, no price levels) but covers 2 years cheaply. Full depth: record live from Binance `@depth@100ms` / `@depth20` (probe OK). Binance `bookTicker` archive stops around March 2024 (2024-04 onward = 404). |
| CVD / taker flow | **Binance kline column "taker buy base volume"** (in every 1m kline): `buy = Vb`, `sell = V - Vb` | Binance Vision `aggTrades` (exact, but ~25-40 MB/day per symbol, so ~25 GB per symbol for 2 years; too big for routine use) | Same as candles. |
| Live candles | Binance spot `wss://stream.binance.com:9443/ws/<sym>@kline_1m` and futures `/market` path (both probed OK) | Bybit `kline.1.{symbol}` | — |

**Estimated disk for the primary set (2 years, BTC+ETH, spot+futures 1m, funding, metrics, bookDepth):**
about 0.2 GB of 1m kline zips + ~20 MB metrics + ~0.7 GB bookDepth, roughly **1 GB zipped**. That fits this
laptop comfortably; aggTrades and the Bybit full book should be pulled only for chosen days.

## Per-source detail

### Binance Vision archive (`data.binance.vision`)
- **Availability from India:** works (all probes 200; SHA-256 of three downloaded samples matched the `.CHECKSUM` files).
- **Content probed:** spot and UM-futures `klines/1m` monthly and daily; `fundingRate` monthly (`calc_time, funding_interval_hours, last_funding_rate`); `metrics` daily (`create_time, symbol, sum_open_interest, sum_open_interest_value, count_toptrader_long_short_ratio, sum_toptrader_long_short_ratio, count_long_short_ratio, sum_taker_long_short_vol_ratio`); `bookDepth` daily (`timestamp, percentage, depth, notional`); `aggTrades`, `trades` daily (large).
- **Missing:** `liquidationSnapshot` (404), `bookTicker` after ~Mar 2024 (404).
- **Limits:** static files, no stated rate limit; download politely (sequential, verify checksums).
- **Terms:** Binance Vision Dataset Terms v1.0 (updated 2026-08-26): **CC BY-NC-SA 4.0 plus Binance-specific terms**; derived charts, models and indicators fall under those terms too [BV-TERMS]. **Personal, non-commercial, local research use fits. Selling or sharing signals built on it would not** (that also matches the handoff's non-goal on SEBI rules).

### Binance REST and WebSocket (`api.binance.com`, `fapi.binance.com`, `fstream.binance.com`)
- **Availability from India:** REST works (klines, fundingRate, depth 200 OK in ~0.25 s). Binance reportedly holds FIU-IND registration since 2024 [BN-IN] (secondary sources; not checked on the FIU portal).
- **WebSocket path change (important for Phase 1):** futures market streams now live under `/market` (and `/public`); the legacy `wss://fstream.binance.com/ws/...` connected but delivered **nothing** for `markPrice` and `!forceOrder@arr` in this probe, while `/market/ws/...` delivered both [BN-WS]. `@depth5@100ms` still worked on the legacy path at probe time.
- **Limits (docs):** request-weight budgets per IP per minute (spot ~6000, futures ~2400); klines cost more weight at higher limits; 1000-1500 candles per request. Use the archive for history, REST only for the last day.
- **Terms:** Binance Terms of Use + API terms. Older Binance FAQ text lists commercial data uses that need written consent. Private, advisory use only.

### Bybit (`api.bybit.com`, `stream.bybit.com`, Bybit archive)
- **Availability from India:** REST kline, funding history and OI history all 200 OK; WebSocket subscribe OK. Bybit was blocked in India in early 2025, registered with FIU-IND after a ~$1M fine, and reports say full access returned later in 2025 [BY-IN]. Treat that as likely, but not verified for today.
- **Limits (docs):** per-IP limit on the order of hundreds of requests per 5 s window; 1000 candles per kline request.
- **Terms:** Bybit API Agreement: API clients "shall not commercially exploit the APIs"; per-endpoint limits are in each API's docs. Local, private use fits; redistribution was not confirmed either way.
- **Role:** fallback for candles, funding and OI; **second live liquidation recorder**; source of full-depth book snapshots for selected days.

### OKX (`www.okx.com`)
- **Availability from India:** public REST and WebSocket answered from this IP. **But** OKX's API Agreement limits access to users who meet its residency/jurisdiction rules, and OKX told Indian customers in 2024 to close positions and withdraw (TechCrunch report). **Eligibility from India is doubtful, so OKX is not used as a primary or fallback.** Mention only.
- **Probed limits:** funding history only ~3 months; `rubik` OI history returned empty for 2024-11; `history-candles` 1m returned 2024-11 data.

### Coinbase Exchange, Kraken, Deribit, Hyperliquid (checked as extra fallbacks)
- Coinbase `products/BTC-USD/candles?granularity=60`: 200 OK with 2024-11 data (300 candles per request). Market-data terms limit use to personal or research purposes.
- Kraken `OHLC`: 200, but returns only the most recent ~720 candles regardless of `since` (the probe asked for 2024-11 and got 2026-11 data), so it is **not usable for 2-year 1m history**.
- Deribit `get_tradingview_chart_data` BTC-PERPETUAL 1m: 200 OK with 2024-11 data.
- Hyperliquid `candleSnapshot` for 2024-11: 200 but empty (only recent candles are served).

### Paid or keyed aggregators
- **CoinGlass** (liquidation heatmaps, aggregated liquidations/OI): `401 API key missing`. Needs an account; useful history is on paid tiers. **Not used** (needs an account and key; scraping PumpGlass/CoinGlass pages is ruled out).
- **Tardis.dev:** `api.tardis.dev` metadata open; datasets without a key are free **only for the first day of each month** (probe: 2024-11-01 -> 302 to file; 2024-11-02 -> 401 with that message). Good for checking the live recorder's format against history, not for backtests.

## Gaps this creates (feeds the Phase 4 GATE)
- The heatmap tactics (H1 liquidation-cluster sweep, H2 walls) **cannot be backtested on 2 years of free data**. They can only be forward-tested on data recorded from Phase 1 onward. With ~6 months of recording, sample sizes will be small; report them as such.
- OI, funding, taker flow and coarse depth (+-1..5% bands) **are** available for 2 years, so H3-H5 filters can be tested now.

## NVDA intraday (for a later phase)

| Source | Probe result from India | Free limits | Notes |
|---|---|---|---|
| **Alpaca Market Data (free plan)** | `401` without key (expected) | Needs free account + key. Historical bars **since 2016**, free plan = **IEX feed only** (~2-3% of US volume) and no bars from the latest 15 min. | Best free depth for minute history, but IEX-only OHLCV differs from consolidated tape. **Primary candidate.** Account/key required. Note: Alpaca also has an India GIFT-City arm; check which entity serves Indian residents. |
| **Polygon / Massive (free tier)** | `401 API Key was not provided` | Free key; docs-level limits ~5 requests/min and ~2 years of minute history on the free tier. | Consolidated data. **Fallback candidate.** Account/key required. |
| Alpha Vantage | demo key rejected for NVDA | Free key, ~25 requests/day; intraday by month (`month=YYYY-MM`) back ~20 years. | Slow (one month per request, 24+ requests for 2 years, about 1 day of quota). Account/key required. |
| Twelve Data | `401` demo key | Free key, ~800 requests/day, 8/min. | Account/key required. |
| Tiingo IEX | `403 Please supply a token` | Free key; IEX intraday. | Account/key required. |
| Yahoo Finance chart endpoint | 200 OK without key | **1m only for the last 30 days** (its own 422 error says so), 5m for ~60 days. | Unofficial; personal-use only; Yahoo's terms restrict automated access. Live sanity checks only, not 2-year research. |
| Stooq | Returns a JavaScript bot check | — | Not used (would mean bypassing a bot check). |
| NSE/BSE | not probed | — | NVDA is not listed in India; Indian investors reach it via LRS/brokers, so no local exchange feed. |

**NVDA decision for later:** Alpaca free (IEX, since 2016) primary, Polygon/Massive free fallback; both need
a free account and key, which the user must create (the project will not create accounts). US equities
data has no liquidation or funding analogues, so only the price-based tactics carry over.

## Phase 1 findings (2026-11-26 ~22:15 UTC, from this laptop)

- **Liquidation history re-checked: still none free.** Binance Vision `liquidationSnapshot` daily
  (2026-11-20, 2023-06-01) and monthly (2026-10): 404. Binance REST `/fapi/v1/allForceOrders`: 404.
  Bybit REST `/v5/market/liquidation`: 404 (no such public endpoint). CoinGlass `liquidation_history`:
  HTTP 200 but body `API key missing` (needs an account). Conclusion unchanged: liquidations exist only
  from the day `candlescope record` started (2026-11-26 22:14 UTC).
- **WebSocket paths (probed again):** Binance USDS-M `/market/stream` delivers `aggTrade`, `kline_1m`,
  `markPrice@1s` and `!forceOrder@arr`, but **not** `bookTicker` or `depth`; `/public/stream` delivers
  `bookTicker` and `depth20@500ms` only. The legacy `/stream` path delivered only bookTicker/depth.
  The recorder therefore opens one `/market` and one `/public` connection.
- **`bookTicker` is too heavy to keep subscribed (2026-11-27):** BTC + ETH sent ~1,200-1,500
  msgs/s (~300 kB/s), and `/public` connections carrying it were reset by the server repeatedly
  (also seen with an independent Python client). `<sym>@depth5@100ms` works on `/public` (~9 frames/s
  per symbol, ~7 kB/s, fields `E T s U u pu b a`); the recorder now samples top of book from its
  level 1.
- **`!forceOrder@arr` is market-wide** (all symbols, e.g. TRBUSDT, RIFUSDT), not only BTC/ETH. The
  recorder keeps all of it (a few KB per hour); filter by `o.s` when reading.
- **Bybit `allLiquidation.<sym>`**: subscription acknowledged (`success: true`), but no BTC/ETH event in
  the first test minutes; it may simply be quiet. Check `liq_bybit` line counts after a few days.
- **Binance spot archive timestamps are in microseconds from 2025-01-01** (16-digit `open_time` and
  `close_time`); USDS-M files stay in milliseconds. `clean` rule R1 converts them and counts the rows.
- **USDS-M kline files carry a CSV header line; spot files do not.** Both are handled.
- **Daily `metrics` files are not sorted by time** (rows jump around within the day). `clean` rule R3
  sorts them and counts the rows; nothing is lost.
- **Current-month funding** has no archive file: it comes from Binance REST `/fapi/v1/fundingRate`
  (one request per symbol, refreshed on every fetch). Its `fundingTime` values can sit a few ms past
  the hour; flagged in the report, not changed.
- **Local clock vs exchange:** in the recorder test the laptop clock was about 1 s behind Binance event
  times (`E`). Use exchange timestamps (`E`, `T`, `k.t`) for analysis; the recorder's own `t` is the
  local receive time and is used only to name files.
