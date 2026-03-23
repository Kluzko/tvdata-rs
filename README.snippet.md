# `tvdata-rs`

`tvdata-rs` is a modern async Rust library for working with TradingView's unofficial data surfaces.

It combines:

- high-level facades for equities, crypto, and FX
- low-level screener access for custom TradingView queries
- typed models for quotes, fundamentals, analyst data, calendars, and history
- capability-aware validation against live scanner metainfo
- simple builder setup plus grouped backend-oriented configuration

## Installation

```toml
[dependencies]
tvdata-rs = "0.1.2"
tokio = { version = "1", features = ["macros", "rt-multi-thread"] }
```

## Feature Flags

Optional product-surface features:

- `search`
- `equity`
- `crypto`
- `forex`
- `calendar`
- `economics`
- `tracing`

Core scanner and history APIs remain available in the base crate.

## Stability Policy

- current MSRV: Rust `1.85`
- patch releases aim to stay non-breaking
- while the crate is `0.x`, minor releases may still contain intentional breaking API changes
- documented public types and methods are the public contract; internal transport and generated registry internals are not

## Start Here

Use these entry points as a rule of thumb:

- `client.equity()` for quotes, fundamentals, analyst data, and stock overviews
- `client.crypto()` and `client.forex()` for market snapshots and movers
- `client.history(...)` or `client.download_history(...)` for OHLCV series
- `client.search_response(...)` for rich symbol lookup metadata
- `client.economic_calendar(...)`, `earnings_calendar(...)`, `dividend_calendar(...)`, and `ipo_calendar(...)` for calendar products
- `client.scan(...)` when you need exact TradingView screener fields and filters
- `client.metainfo(...)`, `validate_scan_query(...)`, and `scan_supported(...)` when you want safer scanner workflows

Initialization paths:

- `TradingViewClient::builder()` for the shortest default setup
- `TradingViewClient::from_config(...)` with `TradingViewClientConfig` for grouped backend configuration
- preset constructors like `for_backend_history()`, `for_research()`, and `for_interactive()` when you want tuned defaults

`for_backend_history()` starts with a tuned chart-history envelope: `6` concurrent
websocket sessions and a matching default batch concurrency for large daily-bar ingestion.
Screener-backed snapshot batch APIs use `SnapshotBatchStrategy::Auto` by default and can be
forced into `SingleRequest` or explicit chunked modes through `SnapshotBatchConfig`.

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

## Low-Level Scanner

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
        ]);

    let response = client.scan(&query).await?;
    let record = response.rows[0].as_record(&query.columns);

    println!("{record:#?}");
    Ok(())
}
```

## Capability-Aware Scans

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

## Search, Calendars, And History

```rust,no_run
use tvdata_rs::{
    CalendarWindowRequest, EconomicCalendarRequest, HistoryRequest, Interval, Result,
    TradingViewClient,
};

#[tokio::main]
async fn main() -> Result<()> {
    let client = TradingViewClient::builder().build()?;

    let search = client.search_equities_response("AAPL").await?;
    println!("remaining: {}", search.symbols_remaining);

    let events = client
        .economic_calendar(&EconomicCalendarRequest::upcoming(7))
        .await?;
    println!("macro events: {}", events.events.len());

    let earnings = client
        .earnings_calendar(&CalendarWindowRequest::upcoming("america", 7))
        .await?;
    println!("earnings events: {}", earnings.len());

    let history = client
        .history(&HistoryRequest::new("NASDAQ:AAPL", Interval::Day1, 30))
        .await?;
    println!("bars: {}", history.bars.len());

    Ok(())
}
```

## Configuration

`TradingViewClient::builder()` supports:

- retry configuration
- endpoint overrides
- explicit `AuthConfig` modes for anonymous, session, token, or combined auth
- auth-aware `sessionid` cookies for HTTP and websocket requests
- grouped `TransportConfig` and `TradingViewClientConfig` for backend-oriented setup
- `SnapshotBatchConfig` with `Auto`, `SingleRequest`, and chunked batch modes for large snapshot workloads
- optional custom websocket connector injection for transport-controlled environments
- optional typed `ClientObserver` hooks for HTTP, websocket, and batch events
- optional `RequestBudget` limits for HTTP pacing and websocket session caps
- injecting a shared HTTP client when your application already owns transport middleware
- optional `tracing` instrumentation for HTTP, scanner, history, and websocket flows
- operational error helpers such as `is_rate_limited()` and `is_auth_error()`

## Caveat

TradingView does not provide a public end-user data API for this use case. This crate works against unofficial, reverse-engineered surfaces, so field support and payload shapes can change over time.
