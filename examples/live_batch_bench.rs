use std::time::Instant;

use time::{Date, Duration as TimeDuration, OffsetDateTime};
use tvdata_rs::prelude::*;

const SYMBOL_COUNT: usize = 1000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum BenchMode {
    All,
    SnapshotOnly,
    HistoryOnly,
}

#[derive(Clone)]
struct SnapshotCase {
    name: &'static str,
    client: TradingViewClient,
}

#[derive(Clone)]
struct HistoryCase {
    name: &'static str,
    client: TradingViewClient,
}

#[tokio::main]
async fn main() -> Result<()> {
    let mode = bench_mode();
    let symbol_count = benchmark_symbol_count();
    let baseline = TradingViewClient::builder().build()?;
    let symbols = load_symbols(&baseline, symbol_count).await?;

    println!("loaded {} equity symbols for benchmarking", symbols.len());

    if matches!(mode, BenchMode::All | BenchMode::SnapshotOnly) {
        let snapshot_cases = snapshot_cases()?;
        println!("\n== Snapshot batch benchmark ==");
        for case in snapshot_cases {
            bench_snapshot_case(&case, &symbols).await?;
        }
    }

    if matches!(mode, BenchMode::All | BenchMode::HistoryOnly) {
        let history_cases = history_cases()?;
        println!("\n== Daily bar benchmark ==");
        for case in history_cases {
            bench_history_case(&case, &symbols).await?;
        }
    }

    Ok(())
}

fn bench_mode() -> BenchMode {
    match std::env::var("TVDATA_BENCH_MODE")
        .unwrap_or_else(|_| String::from("all"))
        .to_ascii_lowercase()
        .as_str()
    {
        "snapshot" | "snapshots" => BenchMode::SnapshotOnly,
        "history" | "daily" | "daily_bars" => BenchMode::HistoryOnly,
        _ => BenchMode::All,
    }
}

fn benchmark_symbol_count() -> usize {
    std::env::var("TVDATA_BENCH_SYMBOLS")
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|count| *count > 0)
        .unwrap_or(SYMBOL_COUNT)
}

fn snapshot_cases() -> Result<Vec<SnapshotCase>> {
    Ok(vec![
        SnapshotCase {
            name: "auto(default)",
            client: TradingViewClient::builder().build()?,
        },
        SnapshotCase {
            name: "single-request",
            client: TradingViewClient::builder()
                .snapshot_batch_config(
                    SnapshotBatchConfig::builder()
                        .strategy(SnapshotBatchStrategy::SingleRequest)
                        .build(),
                )
                .build()?,
        },
        SnapshotCase {
            name: "chunked-250x4",
            client: TradingViewClient::builder()
                .snapshot_batch_config(
                    SnapshotBatchConfig::builder()
                        .strategy(SnapshotBatchStrategy::Chunked {
                            chunk_size: 250,
                            max_concurrent_requests: 4,
                        })
                        .build(),
                )
                .build()?,
        },
        SnapshotCase {
            name: "chunked-125x4",
            client: TradingViewClient::builder()
                .snapshot_batch_config(
                    SnapshotBatchConfig::builder()
                        .strategy(SnapshotBatchStrategy::Chunked {
                            chunk_size: 125,
                            max_concurrent_requests: 4,
                        })
                        .build(),
                )
                .build()?,
        },
    ])
}

fn history_cases() -> Result<Vec<HistoryCase>> {
    history_concurrencies()
        .into_iter()
        .map(|concurrency| {
            let name = format!("history-{concurrency}");
            let client = TradingViewClient::from_config(
                TradingViewClientConfig::builder()
                    .history(
                        HistoryClientConfig::builder()
                            .default_batch_concurrency(concurrency)
                            .default_session(TradingSession::Regular)
                            .default_adjustment(Adjustment::Splits)
                            .build(),
                    )
                    .request_budget(
                        RequestBudget::builder()
                            .max_concurrent_websocket_sessions(concurrency)
                            .max_concurrent_http_requests(8)
                            .min_http_interval(std::time::Duration::from_millis(50))
                            .build(),
                    )
                    .build(),
            )?;

            Ok(HistoryCase {
                name: Box::leak(name.into_boxed_str()),
                client,
            })
        })
        .collect()
}

fn history_concurrencies() -> Vec<usize> {
    std::env::var("TVDATA_BENCH_HISTORY_CONCURRENCIES")
        .ok()
        .map(|value| {
            value
                .split(',')
                .filter_map(|item| item.trim().parse::<usize>().ok())
                .filter(|value| *value > 0)
                .collect::<Vec<_>>()
        })
        .filter(|values| !values.is_empty())
        .unwrap_or_else(|| vec![2, 4, 6])
}

async fn load_symbols(client: &TradingViewClient, count: usize) -> Result<Vec<String>> {
    let query = ScanQuery::new()
        .market("america")
        .select([fields::core::NAME, fields::fundamentals::MARKET_CAP_BASIC])
        .sort(fields::fundamentals::MARKET_CAP_BASIC.sort(SortOrder::Desc))
        .page(0, count)?;
    let response = client.scan(&query).await?;
    Ok(response.rows.into_iter().map(|row| row.symbol).collect())
}

async fn bench_snapshot_case(case: &SnapshotCase, symbols: &[String]) -> Result<()> {
    let started = Instant::now();
    let quotes = case.client.equity().quotes_batch(symbols.to_vec()).await?;
    let quote_elapsed = started.elapsed();

    let started = Instant::now();
    let fundamentals = case
        .client
        .equity()
        .fundamentals_batch(symbols.to_vec())
        .await?;
    let fundamentals_elapsed = started.elapsed();

    let started = Instant::now();
    let analysts = case
        .client
        .equity()
        .analyst_summaries(symbols.to_vec())
        .await?;
    let analyst_elapsed = started.elapsed();

    let started = Instant::now();
    let technicals = case
        .client
        .equity()
        .technical_summaries(symbols.to_vec())
        .await?;
    let technical_elapsed = started.elapsed();

    let started = Instant::now();
    let overviews = case.client.equity().overviews(symbols.to_vec()).await?;
    let overview_elapsed = started.elapsed();

    println!(
        "{:<18} quotes={:>6}ms ok={} missing={} failed={} fundamentals={:>6}ms rows={} analysts={:>6}ms rows={} technicals={:>6}ms rows={} overviews={:>6}ms rows={}",
        case.name,
        quote_elapsed.as_millis(),
        quotes.successes.len(),
        quotes.missing.len(),
        quotes.failures.len(),
        fundamentals_elapsed.as_millis(),
        fundamentals.len(),
        analyst_elapsed.as_millis(),
        analysts.len(),
        technical_elapsed.as_millis(),
        technicals.len(),
        overview_elapsed.as_millis(),
        overviews.len(),
    );

    Ok(())
}

async fn bench_history_case(case: &HistoryCase, symbols: &[String]) -> Result<()> {
    let asof = benchmark_asof();
    let instruments = symbols
        .iter()
        .map(|ticker| instrument_from_ticker(ticker))
        .collect::<Vec<_>>();
    let request = DailyBarRequest::builder()
        .symbols(instruments)
        .asof(asof)
        .selection(BarSelectionPolicy::LatestOnOrBefore)
        .session(TradingSession::Regular)
        .adjustment(Adjustment::Splits)
        .concurrency(case.client.history_config().default_batch_concurrency)
        .build();

    let started = Instant::now();
    let batch = case.client.daily_bars_on(&request).await?;
    let elapsed = started.elapsed();

    println!(
        "{:<18} daily_bars_on={:>6}ms ok={} missing={} failed={} concurrency={}",
        case.name,
        elapsed.as_millis(),
        batch.successes.len(),
        batch.missing.len(),
        batch.failures.len(),
        case.client.history_config().default_batch_concurrency,
    );

    Ok(())
}

fn instrument_from_ticker(ticker: &str) -> InstrumentRef {
    let (exchange, symbol) = ticker.split_once(':').unwrap_or(("NYSE", ticker));
    InstrumentRef::new(exchange, symbol)
}

fn benchmark_asof() -> Date {
    (OffsetDateTime::now_utc() - TimeDuration::days(1)).date()
}
