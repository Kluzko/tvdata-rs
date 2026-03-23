# tvdata-rs

`tvdata-rs` is a modern async Rust library for working with TradingView's unofficial data surfaces.

It is designed as a library, not an application framework. The crate gives you:

- high-level facades for common workflows
- low-level scanner access when you need precise field control
- typed models for quotes, fundamentals, analyst data, calendars, and history
- capability-aware validation against live TradingView scanner metainfo
- backend-friendly initialization with grouped config, request budgets, and transport injection
- configurable HTTP and websocket transport with retry support and typed observer hooks

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
tvdata-rs = "0.1.2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Feature Flags

`tvdata-rs` now supports slimming higher-level product surfaces at compile time.

Core scanner and history APIs remain available in the base crate. Optional features currently
control the add-on surfaces layered on top:

- `search`
- `equity`
- `crypto`
- `forex`
- `calendar`
- `economics`
- `tracing`

Default features enable all of them.

Example:

```toml
[dependencies]
tvdata-rs = { version = "0.1.2", default-features = false, features = ["search"] }
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

The optional `tracing` feature adds structured instrumentation for request execution,
scanner validation, batch history flows, and websocket lifecycle events without changing
the default dependency footprint.

## Stability Policy

`tvdata-rs` targets Rust `1.85` as its current minimum supported Rust version (MSRV).

Versioning expectations:

- patch releases focus on fixes, docs, and non-breaking behavior improvements
- while the crate is still `0.x`, minor releases may contain breaking API changes
- when a breaking change is intentional, it should be called out in `CHANGELOG.md`

Public contract:

- documented public types, methods, and re-exports in the crate root are treated as the stable API surface
- internal transport details, generated field-registry internals, and non-public modules are not stable
- examples and docs should describe supported behavior, but unofficial upstream TradingView payloads can still drift over time

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

## Initialization Paths

`tvdata-rs` currently supports three initialization styles:

- `TradingViewClient::builder()` for the shortest path and library-default setup
- `TradingViewClient::from_config(...)` with `TradingViewClientConfig` when you want grouped backend-oriented configuration
- preset constructors like `TradingViewClient::for_backend_history()` when you want a tuned starting point without wiring the config manually

Use the flat builder when you are exploring or embedding the crate in a small tool. Use grouped config when you care about explicit auth, transport ownership, pacing, websocket behavior, or observer hooks.

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

### Backend-Oriented Initialization

For service and ingestion code, the grouped config path is usually the cleaner entry point:

```rust,no_run
use std::time::Duration;

use tvdata_rs::{
    AuthConfig, RequestBudget, Result, SnapshotBatchConfig, SnapshotBatchStrategy,
    TradingViewClient, TradingViewClientConfig, TransportConfig,
};

#[tokio::main]
async fn main() -> Result<()> {
    let config = TradingViewClientConfig::builder()
        .transport(
            TransportConfig::builder()
                .timeout(Duration::from_secs(45))
                .user_agent("my-backend/1.0")
                .build(),
        )
        .auth(AuthConfig::session("your-session-id"))
        .snapshot_batch(
            SnapshotBatchConfig::builder()
                .strategy(SnapshotBatchStrategy::Auto)
                .build(),
        )
        .request_budget(
            RequestBudget::builder()
                .max_concurrent_http_requests(8)
                .max_concurrent_websocket_sessions(6)
                .build(),
        )
        .build();

    let client = TradingViewClient::from_config(config)?;
    let _ = client.search_equities("AAPL").await?;
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

### Simple Builder vs Grouped Config

`TradingViewClient::builder()` is still fully supported and remains the best simple entry point.

For more structured environments, `TradingViewClientConfig` and `TransportConfig` group the initialization story into:

- transport ownership and retry policy
- auth mode
- history defaults
- request pacing and websocket/session caps
- optional observer hooks

If `transport_config(...)` is provided on the flat builder, it takes precedence over the flat transport fields like `timeout(...)`, `retry(...)`, `user_agent(...)`, and `http_client(...)`.

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

For backend code that wants a more explicit auth shape, use `AuthConfig` instead of wiring
legacy auth fields separately:

```rust,no_run
use tvdata_rs::{AuthConfig, Result, TradingViewClient};

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder()
        .auth(AuthConfig::session("your-session-id"))
        .build()?;

    let _ = client.search_equities("AAPL").await?;
    Ok(())
}
```

`TransportConfig` can also carry a custom `websocket_connector` for deterministic integration
tests, tunneled websocket setups, or proxy-aware environments where you want to own the
connection strategy instead of always using the crate default.

For backend metrics and structured operational accounting without parsing logs, attach a
typed `ClientObserver` through the flat builder or grouped client config. Current events cover
HTTP request completion/failure, websocket connection success/failure, and history-batch
completion summaries.

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

### Shared HTTP Clients

If your backend already owns a shared HTTP stack with custom middleware, proxy/TLS settings,
or request telemetry, inject it directly instead of letting `tvdata-rs` construct a new one.

This fits both initialization styles:

- flat builder with `.http_client(...)`
- grouped config with `TransportConfig::builder().http_client(...)`

Example with the flat builder:

```rust,no_run
use reqwest_middleware::ClientWithMiddleware;
use tvdata_rs::{RequestBudget, Result, TradingViewClient};

#[tokio::main]
async fn main() -> Result<()> {
    let shared_http = ClientWithMiddleware::from(reqwest::Client::new());

    let client = TradingViewClient::builder()
        .http_client(shared_http)
        .request_budget(
            RequestBudget::builder()
                .max_concurrent_http_requests(8)
                .max_concurrent_websocket_sessions(6)
                .min_http_interval(std::time::Duration::from_millis(50))
                .build(),
        )
        .build()?;

    let _ = client.metainfo("america").await?;
    Ok(())
}
```

When a shared HTTP client is injected, `tvdata-rs` still applies TradingView-specific
request headers such as `Origin`, `Referer`, `User-Agent`, and the optional `sessionid`
cookie. HTTP retry and timeout behavior should be configured on the shared client itself.

### Request Budgets And Backpressure

For backend jobs that need lightweight pacing without building a separate limiter layer,
configure a `RequestBudget` on the client:

```rust,no_run
use std::time::Duration;

use tvdata_rs::{RequestBudget, Result, TradingViewClient};

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder()
        .request_budget(
            RequestBudget::builder()
                .max_concurrent_http_requests(8)
                .max_concurrent_websocket_sessions(6)
                .min_http_interval(Duration::from_millis(50))
                .build(),
        )
        .build()?;

    let budget = client.request_budget();
    assert_eq!(budget.max_concurrent_http_requests, Some(8));
    Ok(())
}
```

This budget currently applies:

- HTTP concurrency caps across scan, search, metainfo, and calendar requests
- HTTP request pacing through `min_http_interval`
- websocket session caps across chart-history and quote-session flows

For screener-backed snapshot batches (`quotes_batch`, `fundamentals_batch`, `technical_summaries`,
`analyst_summaries`, `overviews`), the client also applies a snapshot batch planner. The planner
can run as:

- `SnapshotBatchStrategy::Auto`
- `SnapshotBatchStrategy::SingleRequest`
- `SnapshotBatchStrategy::Chunked { chunk_size, max_concurrent_requests }`

`Auto` is the default and is tuned to keep current 1000-symbol public snapshot surfaces on a
single request when that is the better overall tradeoff, while switching to sliced concurrent
requests for larger payloads.

Preset constructors already use sensible request-budget defaults:

- `TradingViewClient::for_backend_history()`
- `TradingViewClient::for_research()`
- `TradingViewClient::for_interactive()`

`for_backend_history()` is tuned for chart-history workloads: it starts with `6` concurrent
websocket sessions and a matching default history batch concurrency.

In live benchmarking on `2026-03-23`, the current targeted `daily_bars_on` path stayed stable at
websocket concurrency `6` on both `250` and `1000` U.S. equities, while `7`, `8`, and `10`
started introducing symbol-level failures. That makes `6` the highest concurrency that this crate
currently treats as the safe default for backend daily-bar ingestion.

The grouped equivalents are:

- `TradingViewClientConfig::backend_history()`
- `TradingViewClientConfig::research()`
- `TradingViewClientConfig::interactive()`

### Typed Observer Hooks

If you need metrics or operational accounting without scraping logs, attach a `ClientObserver`.

The observer currently receives typed events for:

- HTTP request completion
- HTTP request failure
- websocket connection success
- websocket connection failure
- history batch completion summaries

This is additive to `tracing`: use `tracing` for logs/spans, use `ClientObserver` for typed counters or metrics pipelines.

### Observability With `tracing`

Backend services that want request- and history-level telemetry can enable the optional
`tracing` feature:

```toml
[dependencies]
tvdata-rs = { version = "0.1.2", features = ["tracing"] }
tracing-subscriber = "0.3"
```

```rust,no_run
use tvdata_rs::{Result, TradingViewClient};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();

    let client = TradingViewClient::for_backend_history()?;
    let _ = client.metainfo("america").await?;
    Ok(())
}
```

Current instrumentation targets include:

- `tvdata_rs::http`
- `tvdata_rs::scan`
- `tvdata_rs::search`
- `tvdata_rs::calendar`
- `tvdata_rs::history`
- `tvdata_rs::transport`

### Preset Clients

For common workload profiles, you can start from a prebuilt client preset:

- `TradingViewClient::for_backend_history()`
- `TradingViewClient::for_research()`
- `TradingViewClient::for_interactive()`

If you want the same preset shape in grouped form, use:

- `TradingViewClientConfig::backend_history()`
- `TradingViewClientConfig::research()`
- `TradingViewClientConfig::interactive()`

### Failure Classification

Backend workers can classify failures without string matching:

```rust,no_run
use tvdata_rs::{Error, Result, TradingViewClient};

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder().build()?;

    match client.search_equities("AAPL").await {
        Ok(_) => {}
        Err(error) if error.is_rate_limited() => {
            // back off and retry later
        }
        Err(error) if error.is_auth_error() => {
            // refresh credentials or disable auth-aware mode
        }
        Err(error) if error.is_symbol_error() => {
            // suppress or mark the symbol as unavailable
        }
        Err(error) => return Err(error),
    }

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
- live batch strategy benchmarking via [live_batch_bench.rs](/Users/jakubkluzniak/dev/tvdata-rs/examples/live_batch_bench.rs)

The field registry is embedded so low-level scanner workflows can still operate with a stable local field catalog even when live metainfo is unavailable.

Contributor workflow and quality gates are documented in [CONTRIBUTING.md](/Users/jakubkluzniak/dev/tvdata-rs/CONTRIBUTING.md), and release-facing changes are tracked in [CHANGELOG.md](/Users/jakubkluzniak/dev/tvdata-rs/CHANGELOG.md).
