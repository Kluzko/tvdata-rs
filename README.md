# tvdata-rs

`tvdata-rs` is a modern async Rust library for working with TradingView's unofficial data surfaces.

It is designed as a library, not an application framework. The crate gives you:

- high-level facades for common workflows
- low-level scanner access when you need precise field control
- typed models for quotes, fundamentals, analyst data, calendars, and history
- capability-aware validation against live TradingView scanner metainfo
- configurable HTTP and websocket transport with retry support

## Why Use `tvdata-rs`

Most TradingView wrappers fall into one of two buckets:

- thin endpoint wrappers that expose payloads but leave semantics to the user
- convenience helpers that work for a few cases but become limiting for real scanning and research workflows

`tvdata-rs` aims for a cleaner middle ground:

- ergonomic APIs for the common cases
- low-level access for advanced cases
- typed models instead of ad hoc maps
- a library-first architecture that stays composable inside larger Rust systems

## What The Crate Covers

- Screener queries via `scan`
- Live scanner metainfo via `metainfo`
- Capability-aware scan validation and filtering
- Symbol search via TradingView `symbol_search/v3`
- Economic calendar events
- Market-wide earnings, dividend, and IPO calendars
- OHLCV history over TradingView chart websockets
- High-level `equity()`, `crypto()`, and `forex()` facades
- Equity analyst products, estimate history, and point-in-time fundamentals
- Embedded field registry generated from `deepentropy/tvscreener`
- Optional `sessionid` cookie support for auth-aware HTTP and websocket requests

## Installation

Add the crate to your `Cargo.toml`:

```toml
[dependencies]
tvdata-rs = "0.1.0"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Start Here

If you are new to the crate, use this rule of thumb:

| Goal | Best entry point |
| --- | --- |
| Get a quote, fundamentals, analyst data, or a stock overview | `client.equity()` |
| Work with crypto or FX snapshots and movers | `client.crypto()` / `client.forex()` |
| Download OHLCV series | `client.history(...)` or `client.download_history(...)` |
| Look up symbols and listing metadata | `client.search_response(...)` |
| Pull market-wide calendars | `client.economic_calendar(...)`, `earnings_calendar(...)`, `dividend_calendar(...)`, `ipo_calendar(...)` |
| Build custom screeners | `client.scan(...)` with `ScanQuery` |
| Make custom scans safer | `client.metainfo(...)`, `validate_scan_query(...)`, `scan_validated(...)`, `scan_supported(...)` |

If you only need one thing and want the shortest path, start with the high-level facades first. Drop to the low-level scanner only when you need custom fields or custom filter logic.

## Quick Start

```rust,no_run
use tvdata_rs::{Result, TradingViewClient};

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder().build()?;

    let quote = client.equity().quote("NASDAQ:AAPL").await?;

    println!(
        "{} close: {:?}",
        quote.instrument.ticker.as_str(),
        quote.close
    );

    Ok(())
}
```

## Common Workflows

### Equity: Quotes, Fundamentals, Analysts

This is the best entry point for stock-oriented workflows.

```rust,no_run
use tvdata_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder().build()?;
    let equity = client.equity();

    let quote = equity.quote("NASDAQ:AAPL").await?;
    let fundamentals = equity.fundamentals("NASDAQ:AAPL").await?;
    let analyst = equity.analyst_summary("NASDAQ:AAPL").await?;
    let overview = equity.overview("NASDAQ:AAPL").await?;

    println!("{quote:#?}");
    println!("{fundamentals:#?}");
    println!("{analyst:#?}");
    println!("{overview:#?}");
    Ok(())
}
```

The equity facade also exposes more specific analyst methods:

- `analyst_recommendations()`
- `price_targets()`
- `analyst_forecasts()`
- `earnings_calendar()`
- `analyst_fx_rates()`

And historical analyst / fundamental products:

- `estimate_history()`
- `earnings_history()`
- `fundamentals_point_in_time()`
- `fundamentals_history()`

These historical products use typed `FiscalPeriod` values and a shared `HistoricalObservation<T>` model.

### Crypto And Forex Snapshots

```rust,no_run
use tvdata_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder().build()?;

    let btc = client.crypto().quote("BINANCE:BTCUSDT").await?;
    let eurusd = client.forex().quote("FX:EURUSD").await?;
    let crypto_gainers = client.crypto().top_gainers(10).await?;
    let fx_active = client.forex().most_active(10).await?;

    println!("{btc:#?}");
    println!("{eurusd:#?}");
    println!("{crypto_gainers:#?}");
    println!("{fx_active:#?}");
    Ok(())
}
```

### OHLCV History

Use `history(...)` for a single series and `download_history(...)` / `history_batch(...)` for multiple symbols.

```rust,no_run
use tvdata_rs::{HistoryRequest, Interval, Result, TradingViewClient};

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder().build()?;

    let recent = client
        .history(&HistoryRequest::new("NASDAQ:AAPL", Interval::Day1, 100))
        .await?;

    let maximum = client
        .history(&HistoryRequest::max("NASDAQ:AAPL", Interval::Day1))
        .await?;

    println!("recent bars: {}", recent.bars.len());
    println!("max bars: {}", maximum.bars.len());
    Ok(())
}
```

For multiple symbols:

```rust,no_run
use tvdata_rs::{HistoryBatchRequest, Interval, Result, TradingViewClient};

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder().build()?;
    let request = HistoryBatchRequest::new(["NASDAQ:AAPL", "NASDAQ:MSFT"], Interval::Day1, 30);

    let series = client.history_batch(&request).await?;
    println!("series: {}", series.len());
    Ok(())
}
```

### Symbol Search

`search_response(...)` exposes the richer TradingView `v3` search shape, including listing metadata and identifiers such as `isin`, `cusip`, and `cik_code`.

```rust,no_run
use tvdata_rs::{Result, TradingViewClient};

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder().build()?;
    let response = client.search_equities_response("AAPL").await?;

    println!("remaining: {}", response.symbols_remaining);

    for hit in response.hits.iter().take(3) {
        println!(
            "{} {:?} {:?} {:?}",
            hit.symbol,
            hit.exchange,
            hit.instrument_type,
            hit.isin
        );
    }

    Ok(())
}
```

Typed convenience helpers are available for the most common asset classes:

- `search_equities(...)` / `search_equities_response(...)`
- `search_forex(...)` / `search_forex_response(...)`
- `search_crypto(...)` / `search_crypto_response(...)`
- `search_options(...)` / `search_options_response(...)`

### Macro And Corporate Calendars

`tvdata-rs` exposes both macro events and market-wide corporate calendars.

```rust,no_run
use tvdata_rs::{
    CalendarWindowRequest, DividendCalendarRequest, EconomicCalendarRequest, Result,
    TradingViewClient,
};

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder().build()?;

    let macro_events = client
        .economic_calendar(&EconomicCalendarRequest::upcoming(7))
        .await?;

    let earnings = client
        .earnings_calendar(&CalendarWindowRequest::upcoming("america", 7))
        .await?;

    let dividends = client
        .dividend_calendar(&DividendCalendarRequest::upcoming("america", 14))
        .await?;

    let ipos = client
        .ipo_calendar(&CalendarWindowRequest::trailing("america", 30))
        .await?;

    println!("macro events: {}", macro_events.events.len());
    println!("earnings: {}", earnings.len());
    println!("dividends: {}", dividends.len());
    println!("ipos: {}", ipos.len());
    Ok(())
}
```

### Custom Screener Queries

Use the low-level scanner when you need exact TradingView fields, custom filters, or a query builder that maps closely to scanner payloads.

```rust,no_run
use tvdata_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder().build()?;

    let query = ScanQuery::new()
        .market("america")
        .tickers(["NASDAQ:AAPL"])
        .select([
            fields::core::NAME,
            fields::price::CLOSE,
            fields::technical::RSI,
            fields::analyst::PRICE_TARGET_AVERAGE,
        ]);

    let response = client.scan(&query).await?;
    let record = response.rows[0].as_record(&query.columns);

    println!("{record:#?}");
    Ok(())
}
```

## Capability-Aware Scans

TradingView field support is not uniform across markets and can drift over time. The crate exposes live metainfo and scan validation to help with that.

### Inspect Live Scanner Metainfo

```rust,no_run
use tvdata_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder().build()?;
    let metainfo = client.metainfo("america").await?;

    assert!(metainfo.supports_field("close"));
    println!("market fields: {}", metainfo.fields.len());
    Ok(())
}
```

### Validate Before Executing

```rust,no_run
use tvdata_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder().build()?;
    let query = ScanQuery::new()
        .market("america")
        .select([fields::core::NAME, fields::price::CLOSE]);

    let report = client.validate_scan_query(&query).await?;
    assert!(report.is_strictly_supported());

    let response = client.scan_validated(&query).await?;
    println!("rows: {}", response.rows.len());
    Ok(())
}
```

### Filter Unsupported Columns For Multi-Market Queries

```rust,no_run
use tvdata_rs::prelude::*;

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder().build()?;
    let query = ScanQuery::new()
        .markets(["america", "crypto"])
        .select([fields::price::CLOSE, fields::fundamentals::MARKET_CAP_BASIC]);

    let (filtered, report) = client.filter_scan_query(&query).await?;
    println!("kept columns: {:?}", report.filtered_column_names());

    let response = client.scan_supported(&query).await?;
    println!("rows: {}", response.rows.len());
    assert!(!filtered.columns.is_empty());
    Ok(())
}
```

## Configuration

### Auth-Aware Sessions

If you have a TradingView `sessionid` cookie and want auth-aware requests, pass it through the client builder:

```rust,no_run
use tvdata_rs::{Result, TradingViewClient};

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder()
        .session_id("your-session-id")
        .build()?;

    let quote = client.equity().quote("NASDAQ:AAPL").await?;
    println!("{:?}", quote.close);
    Ok(())
}
```

### Retry And Endpoint Overrides

```rust,no_run
use std::time::Duration;

use tvdata_rs::{Endpoints, Result, RetryConfig, TradingViewClient};

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder()
        .retry(
            RetryConfig::builder()
                .max_retries(4)
                .min_retry_interval(Duration::from_millis(250))
                .max_retry_interval(Duration::from_secs(3))
                .build(),
        )
        .endpoints(
            Endpoints::default()
                .with_scanner_base_url("http://127.0.0.1:8080")?
                .with_symbol_search_base_url("http://127.0.0.1:8081/symbol_search")?,
        )
        .build()?;

    let _ = client.search_equities("AAPL").await?;
    Ok(())
}
```

## Design Notes

The crate intentionally separates:

- low-level TradingView payload composition
- transport concerns such as retries, cookies, and websocket framing
- user-facing typed models and high-level facades

It also intentionally does not include:

- built-in storage or database layers
- a crawler / browser automation layer
- a local persistence framework
- a non-Rust runtime or service wrapper

The goal is to stay useful as a clean Rust library that can be embedded into your own application, research pipeline, or service.

## Important Caveat

TradingView does not provide a public end-user market data API for this use case. This crate works against unofficial, reverse-engineered surfaces.

That means:

- upstream schemas can change without notice
- field support can drift by market and over time
- rate limits and freshness characteristics are not officially documented

If you depend on a specific field set or scanner behavior, prefer capability-aware flows such as `metainfo(...)`, `validate_scan_query(...)`, and `scan_supported(...)`.

## Development

Examples live in [examples/](/Users/jakubkluzniak/dev/tvdata-rs/examples) and cover the main product surfaces:

- quotes and facades
- search
- metainfo and capability-aware scans
- history
- macro and corporate calendars

The field registry is embedded so low-level scanner workflows can still operate with a stable local field catalog even when live metainfo is unavailable.

Contributor workflow and quality gates are documented in [CONTRIBUTING.md](/Users/jakubkluzniak/dev/tvdata-rs/CONTRIBUTING.md), and release-facing changes are tracked in [CHANGELOG.md](/Users/jakubkluzniak/dev/tvdata-rs/CHANGELOG.md).
