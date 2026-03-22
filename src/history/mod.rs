mod fetch;
mod request;

use request::HistorySeriesMap;

use crate::client::TradingViewClient;
use crate::error::Result;
use crate::scanner::Ticker;

pub use request::{
    Adjustment, Bar, HistoryBatchRequest, HistoryRequest, HistorySeries, Interval, TradingSession,
};

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
        fetch::fetch_history_batch_with(
            request.to_requests(),
            request.concurrency,
            |request| async move { self.history(&request).await },
        )
        .await
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
        let request = HistoryBatchRequest::max(symbols, interval);
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
        let request = HistoryBatchRequest::new(symbols, interval, bars);
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
}

pub(crate) use fetch::fetch_history;
