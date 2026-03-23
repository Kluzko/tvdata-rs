mod fetch;
mod request;

use futures_util::stream::{self, StreamExt as FuturesStreamExt};
use request::HistorySeriesMap;
use time::{Date, OffsetDateTime};
#[cfg(feature = "tracing")]
use tracing::debug;

use crate::batch::{BatchResult, SymbolFailure};
use crate::client::{ClientEvent, HistoryBatchCompletedEvent, HistoryBatchMode, TradingViewClient};
use crate::error::Result;
use crate::scanner::{InstrumentRef, Ticker};

pub use request::{
    Adjustment, Bar, BarSelectionPolicy, DailyBarRangeRequest, DailyBarRequest,
    HistoryBatchRequest, HistoryProvenance, HistoryRequest, HistorySeries, Interval,
    TradingSession,
};

fn estimated_daily_bars_since(date: Date) -> u32 {
    let today = OffsetDateTime::now_utc().date();
    let days = if date <= today {
        (today - date).whole_days().max(0) as u32
    } else {
        0
    };

    days.saturating_add(32).max(64)
}

fn daily_batch_request(
    symbols: &[InstrumentRef],
    start: Date,
    session: TradingSession,
    adjustment: Adjustment,
    concurrency: usize,
) -> HistoryBatchRequest {
    HistoryBatchRequest::new(
        symbols.iter().cloned().map(Into::<Ticker>::into),
        Interval::Day1,
        estimated_daily_bars_since(start),
    )
    .session(session)
    .adjustment(adjustment)
    .concurrency(concurrency)
}

fn daily_bar_socket_chunk_size(symbols: usize, socket_concurrency: usize) -> usize {
    if symbols == 0 {
        return 0;
    }

    let socket_concurrency = socket_concurrency.max(1);
    symbols.div_ceil(socket_concurrency).clamp(16, 64)
}

impl TradingViewClient {
    /// Downloads multiple OHLCV history series with bounded concurrency.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tvdata_rs::{HistoryBatchRequest, Interval, Result, TradingViewClient};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = TradingViewClient::builder().build()?;
    ///     let request = HistoryBatchRequest::new(["NASDAQ:AAPL", "NASDAQ:MSFT"], Interval::Day1, 30);
    ///     let series = client.history_batch(&request).await?;
    ///
    ///     println!("series: {}", series.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn history_batch(&self, request: &HistoryBatchRequest) -> Result<Vec<HistorySeries>> {
        let effective_concurrency = self.effective_history_batch_concurrency(request.concurrency);

        #[cfg(feature = "tracing")]
        debug!(
            target: "tvdata_rs::history",
            requested = request.symbols.len(),
            interval = request.interval.as_code(),
            bars = request.bars,
            requested_concurrency = request.concurrency,
            effective_concurrency,
            "starting history batch",
        );

        let series = fetch::fetch_history_batch_with(
            request.to_requests(),
            effective_concurrency,
            |request| async move { self.history(&request).await },
        )
        .await?;

        self.emit_event(ClientEvent::HistoryBatchCompleted(
            HistoryBatchCompletedEvent {
                requested: request.symbols.len(),
                successes: series.len(),
                missing: 0,
                failures: 0,
                concurrency: effective_concurrency,
                mode: HistoryBatchMode::Strict,
            },
        ));

        Ok(series)
    }

    /// Downloads multiple OHLCV history series and returns successes, missing symbols, and
    /// failures separately.
    pub async fn history_batch_detailed(
        &self,
        request: &HistoryBatchRequest,
    ) -> Result<BatchResult<HistorySeries>> {
        let effective_concurrency = self.effective_history_batch_concurrency(request.concurrency);

        #[cfg(feature = "tracing")]
        debug!(
            target: "tvdata_rs::history",
            requested = request.symbols.len(),
            interval = request.interval.as_code(),
            bars = request.bars,
            requested_concurrency = request.concurrency,
            effective_concurrency,
            "starting detailed history batch",
        );

        let batch = fetch::fetch_history_batch_detailed_with(
            request.to_requests(),
            effective_concurrency,
            |request| async move { self.history(&request).await },
        )
        .await?;

        self.emit_event(ClientEvent::HistoryBatchCompleted(
            HistoryBatchCompletedEvent {
                requested: request.symbols.len(),
                successes: batch.successes.len(),
                missing: batch.missing.len(),
                failures: batch.failures.len(),
                concurrency: effective_concurrency,
                mode: HistoryBatchMode::Detailed,
            },
        ));

        Ok(batch)
    }

    /// Downloads the maximum history currently available for multiple symbols.
    ///
    /// The crate keeps requesting older bars over the chart websocket until
    /// TradingView stops returning new history.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tvdata_rs::{Interval, Result, TradingViewClient};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = TradingViewClient::builder().build()?;
    ///     let series = client
    ///         .download_history_max(["NASDAQ:AAPL", "NASDAQ:MSFT"], Interval::Day1)
    ///         .await?;
    ///
    ///     println!("series: {}", series.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn download_history_max<I, T>(
        &self,
        symbols: I,
        interval: Interval,
    ) -> Result<Vec<HistorySeries>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Ticker>,
    {
        let defaults = self.history_config();
        let request = HistoryBatchRequest::max(symbols, interval)
            .session(defaults.default_session)
            .adjustment(defaults.default_adjustment)
            .concurrency(defaults.default_batch_concurrency);
        self.history_batch(&request).await
    }

    /// Convenience wrapper around [`TradingViewClient::history_batch`] for a list of symbols.
    pub async fn download_history<I, T>(
        &self,
        symbols: I,
        interval: Interval,
        bars: u32,
    ) -> Result<Vec<HistorySeries>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Ticker>,
    {
        let defaults = self.history_config();
        let request = HistoryBatchRequest::new(symbols, interval, bars)
            .session(defaults.default_session)
            .adjustment(defaults.default_adjustment)
            .concurrency(defaults.default_batch_concurrency);
        self.history_batch(&request).await
    }

    /// Downloads multiple history series and returns them keyed by symbol.
    pub async fn download_history_map<I, T>(
        &self,
        symbols: I,
        interval: Interval,
        bars: u32,
    ) -> Result<HistorySeriesMap>
    where
        I: IntoIterator<Item = T>,
        T: Into<Ticker>,
    {
        let series = self.download_history(symbols, interval, bars).await?;
        Ok(series
            .into_iter()
            .map(|series| (series.symbol.clone(), series))
            .collect())
    }

    /// Downloads the maximum history available and returns it keyed by symbol.
    pub async fn download_history_map_max<I, T>(
        &self,
        symbols: I,
        interval: Interval,
    ) -> Result<HistorySeriesMap>
    where
        I: IntoIterator<Item = T>,
        T: Into<Ticker>,
    {
        let series = self.download_history_max(symbols, interval).await?;
        Ok(series
            .into_iter()
            .map(|series| (series.symbol.clone(), series))
            .collect())
    }

    /// Downloads daily bars for a set of instruments and selects the best bar for the requested
    /// trading date.
    pub async fn daily_bars_on(&self, request: &DailyBarRequest) -> Result<BatchResult<Bar>> {
        if request.symbols.is_empty() {
            return Ok(BatchResult::default());
        }

        let effective_concurrency = self.effective_history_batch_concurrency(request.concurrency);

        #[cfg(feature = "tracing")]
        debug!(
            target: "tvdata_rs::history",
            symbols = request.symbols.len(),
            asof = %request.asof,
            selection = ?request.selection,
            requested_concurrency = request.concurrency,
            effective_concurrency,
            "starting daily bar selection",
        );

        let tickers = request
            .symbols
            .iter()
            .cloned()
            .map(Into::<Ticker>::into)
            .collect::<Vec<_>>();
        let chunk_size = daily_bar_socket_chunk_size(tickers.len(), effective_concurrency);

        let mut outcomes = stream::iter(
            tickers
                .chunks(chunk_size)
                .map(|chunk| chunk.to_vec())
                .enumerate()
                .map(|(index, chunk)| async move {
                    let outcome = fetch::fetch_daily_bars_batch_with_timeout_for_client(
                        self,
                        &chunk,
                        request.asof,
                        request.selection,
                        request.session,
                        request.adjustment,
                        self.history_config().session_timeout,
                    )
                    .await;
                    (index, chunk, outcome)
                }),
        )
        .buffer_unordered(effective_concurrency)
        .collect::<Vec<_>>()
        .await;

        outcomes.sort_by_key(|(index, _, _)| *index);

        let mut selected = BatchResult::default();
        for (_, chunk, outcome) in outcomes {
            match outcome {
                Ok(batch) => {
                    selected.successes.extend(batch.successes);
                    selected.missing.extend(batch.missing);
                    selected.failures.extend(batch.failures);
                }
                Err(error) if error.is_symbol_error() => selected.missing.extend(chunk),
                Err(error) => {
                    let kind = error.kind();
                    let retryable = error.is_retryable();
                    let message = error.to_string();
                    selected
                        .failures
                        .extend(chunk.into_iter().map(|ticker| SymbolFailure {
                            symbol: ticker,
                            kind,
                            message: message.clone(),
                            retryable,
                        }));
                }
            }
        }

        self.emit_event(ClientEvent::HistoryBatchCompleted(
            HistoryBatchCompletedEvent {
                requested: request.symbols.len(),
                successes: selected.successes.len(),
                missing: selected.missing.len(),
                failures: selected.failures.len(),
                concurrency: effective_concurrency,
                mode: HistoryBatchMode::Detailed,
            },
        ));

        #[cfg(feature = "tracing")]
        debug!(
            target: "tvdata_rs::history",
            asof = %request.asof,
            successes = selected.successes.len(),
            missing = selected.missing.len(),
            failures = selected.failures.len(),
            "daily bar selection completed",
        );

        Ok(selected)
    }

    /// Downloads daily history and trims each successful series to the requested date window.
    pub async fn daily_bars_range(
        &self,
        request: &DailyBarRangeRequest,
    ) -> Result<BatchResult<HistorySeries>> {
        if request.start > request.end {
            return Ok(BatchResult::default());
        }

        #[cfg(feature = "tracing")]
        debug!(
            target: "tvdata_rs::history",
            symbols = request.symbols.len(),
            start = %request.start,
            end = %request.end,
            concurrency = request.concurrency,
            "starting daily history range selection",
        );

        let history_request = daily_batch_request(
            &request.symbols,
            request.start,
            request.session,
            request.adjustment,
            request.concurrency,
        );
        let batch = self.history_batch_detailed(&history_request).await?;

        let mut selected = BatchResult {
            missing: batch.missing,
            failures: batch.failures,
            ..BatchResult::default()
        };

        for (ticker, mut series) in batch.successes {
            series
                .bars
                .retain(|bar| bar.time.date() >= request.start && bar.time.date() <= request.end);

            if series.bars.is_empty() {
                selected.missing.push(ticker);
            } else {
                selected.successes.insert(ticker, series);
            }
        }

        #[cfg(feature = "tracing")]
        debug!(
            target: "tvdata_rs::history",
            start = %request.start,
            end = %request.end,
            successes = selected.successes.len(),
            missing = selected.missing.len(),
            failures = selected.failures.len(),
            "daily history range selection completed",
        );

        Ok(selected)
    }
}

pub(crate) use fetch::fetch_history_with_timeout_for_client;
