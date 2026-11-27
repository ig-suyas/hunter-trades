# Sources

Phase 0, 2026-11-27. Tags are used in `knowledge.md` and `data-sources.md`. Definitions in this project
are paraphrased; no paid content was copied. Where sources disagree, `knowledge.md` says so and treats
each variant as a separate hypothesis.

## Project inputs (local)

| Tag | Source |
|---|---|
| [GUIDE] | `Trading_Tactics_Study_Guide.pdf` (15 pages, written in this project's chat) |
| [PUMP] | `pumpglass_heatmap_trading_guide.pdf` (3 pages) |
| [LIST] | `technical_analysis_patterns.txt` (names only) |
| [VID1] | `trading1.mp4`, see `video-notes.md` |
| [VID2] | `trading.mp4`, see `video-notes.md` |

## Pattern and tactic definitions (web)

| Tag | Source | Used for |
|---|---|---|
| [TALIB] | TA-Lib candle settings, https://ta-lib.org/api/candle-settings/ ; function pages e.g. https://ta-lib.org/functions/cdlshortline ; R port vignette with default thresholds, https://cran.r-project.org/web/packages/talib/vignettes/candlestick.html | Candlestick thresholds (long/short body, doji, shadows). TA-Lib is open source (BSD), so its rules are a reproducible baseline. |
| [BULK] | Thomas Bulkowski, The Pattern Site, https://thepatternsite.com (e.g. Adam & Eve double bottom https://www.thepatternsite.com/aedb.html , Eve & Eve https://www.thepatternsite.com/eedb.html) | Chart pattern identification guidelines, confirmation points, bump-and-run, cup and handle, high tight flag. Only freely published guideline summaries were used; statistics from his paid books were not copied. |
| [BULK-AE] | Same site, Adam & Eve pages above | Adam (narrow) vs Eve (wide) bottoms. |
| [NISON-BULK] | General candlestick descriptions from Nison's published work as summarised on public pages; plus [BULK] candlestick pages | Patterns TA-Lib names differently. |
| [IG-FX] | IG, "Top 7 harmonic patterns", https://www.ig.com/uk/trading-strategies/top-7-harmonic-patterns-every-trader-should-know-210608 ; FOREX.com harmonic cheat sheet, https://www.forex.com/en-au/news-and-analysis/harmonic-patterns/ ; Capital.com harmonic page https://capital.com/en-ke/learn/technical-analysis/harmonic-pattern | Harmonic ratio ranges. Sources conflict (Butterfly D, Cypher and Shark naming). |
| [CS-HARM] | StockCharts ChartSchool, Harmonic Patterns, https://chartschool.stockcharts.com/table-of-contents/trading-strategies-and-models/trading-strategies/harmonic-patterns | Harmonic overview. |
| [CAPQM] | Capital.com, "Quasimodo trading: how the QM pattern works", https://capital.com/en-int/learn/trading-strategies/quasimodo-pattern | QM structure; source says it has no fixed formula. |
| [WYCK] | StockCharts ChartSchool, "The Wyckoff Method: A Tutorial", https://chartschool.stockcharts.com/table-of-contents/market-analysis/wyckoff-analysis-articles/the-wyckoff-method-a-tutorial ; "Wyckoff stock analysis", https://chartschool.stockcharts.com/table-of-contents/market-analysis/wyckoff-analysis-articles/wyckoff-stock-analysis | Phases, spring, upthrust, tests. |
| [GEN] | Standard definitions (Wilder ATR/RSI 1978; EMA; Supertrend as published in TradingView built-ins; VWAP) | Shared building blocks and indicators seen in the videos. |

Not found in this session with a reliable public source: a precise definition of the **Dragon pattern**
and **Wolfe wave** rules (Wolfe's own material is paid). Both are marked SUBJECTIVE with a proxy definition.

## Data sources and terms (web + live probes)

| Tag | Source |
|---|---|
| [BV-TERMS] | Binance Vision Dataset Terms (CC BY-NC-SA 4.0 plus extra terms), https://data.binance.vision/Binance_Vision-Terms_of_Use.pdf |
| [BV-GH] | binance-public-data repo (file layout), https://github.com/binance/binance-public-data/ |
| [BN-WS] | Binance USDS-M futures "Important WebSocket Change Notice" (/public, /market, /private paths), https://developers.binance.com/docs/derivatives/usds-margined-futures/websocket-market-streams/Important-WebSocket-Change-Notice ; change log https://developers.binance.com/docs/derivatives/change-log |
| [BN-MD] | Binance market-data-only URLs, https://developers.binance.com/docs/binance-spot-api-docs/faqs/market_data_only |
| [BN-IN] | Binance FIU-IND status (secondary reports, not verified on the FIU portal): https://www.cryptowire.in/crypto/fiu-registered-crypto-exchanges-india/ , https://openrate.live/blog/is-binance-legal-india-2026 |
| [BY-IN] | Bybit India FIU registration and return: https://www.coindesk.com/policy/2025/02/06/crypto-exchange-bybit-receives-india-clearance-after-settling-usd1m-fine , https://www.ccn.com/news/crypto/bybit-india-regains-app-access-web-access/ |
| [TARDIS] | Tardis.dev datasets (free only for the first day of each month without a key), https://docs.tardis.dev |
| [CG] | CoinGlass API (key required), https://docs.coinglass.com |
| [YF] | Yahoo Finance chart endpoint (unofficial; 1m only last 30 days per its own error message) |
| [AV], [TD], [POLY], [ALP], [TII] | Alpha Vantage, Twelve Data, Polygon (Massive), Alpaca, Tiingo: all need a free key (probed, see data-sources.md) |
