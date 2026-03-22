mod columns;
mod decode;
mod history;
#[cfg(test)]
mod tests;
mod types;

use futures_util::stream::{self, StreamExt as FuturesStreamExt, TryStreamExt};

use self::columns::{
    analyst_columns, analyst_forecast_columns, analyst_fx_rate_columns,
    analyst_price_target_columns, analyst_recommendation_columns, earnings_calendar_columns,
    equity_identity_columns, equity_quote_columns, equity_technical_columns, fundamentals_columns,
    overview_columns,
};
use self::decode::{
    decode_analyst, decode_analyst_forecasts, decode_analyst_fx_rates,
    decode_analyst_price_targets, decode_analyst_recommendations, decode_earnings_calendar,
    decode_fundamentals, decode_overview,
};
use self::history::{
    decode_estimate_history, decode_point_in_time_fundamentals, estimate_history_fields,
    fundamentals_history_fields,
};
use crate::batch::BatchResult;
use crate::client::TradingViewClient;
use crate::error::Result;
use crate::market_data::{
    InstrumentIdentity, QuoteSnapshot, RowDecoder, SnapshotLoader, TechnicalSummary, decode_quote,
    decode_technical,
};
use crate::scanner::fields::price;
use crate::scanner::{Column, Market, ScanQuery, SortOrder, Ticker};
use crate::transport::quote_session::QuoteSessionClient;

pub use history::{
    EarningsMetrics, EstimateHistory, EstimateMetrics, EstimateObservation, FundamentalMetrics,
    FundamentalObservation, PointInTimeFundamentals,
};
pub use types::{
    AnalystForecasts, AnalystFxRates, AnalystPriceTargets, AnalystRecommendations, AnalystSummary,
    EarningsCalendar, EquityOverview, FundamentalsSnapshot,
};

const HISTORY_BATCH_CONCURRENCY: usize = 4;

/// High-level equity data facade built on top of TradingView screener and quote sessions.
///
/// This facade exposes typed products for common stock workflows such as:
///
/// - quotes and market movers
/// - fundamentals snapshots
/// - analyst summaries, targets, forecasts, and earnings metadata
/// - native estimate history and point-in-time fundamentals
///
/// # Examples
///
/// ```no_run
/// use tvdata_rs::{Result, TradingViewClient};
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     let client = TradingViewClient::builder().build()?;
///     let equity = client.equity();
///
///     let quote = equity.quote("NASDAQ:AAPL").await?;
///     let analyst = equity.analyst_summary("NASDAQ:AAPL").await?;
///
///     println!("{:?} {:?}", quote.close, analyst.price_targets.average);
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct EquityClient<'a> {
    client: &'a TradingViewClient,
}

impl<'a> EquityClient<'a> {
    pub const fn new(client: &'a TradingViewClient) -> Self {
        Self { client }
    }

    pub fn client(&self) -> &'a TradingViewClient {
        self.client
    }

    /// Fetches a typed equity quote snapshot for a single symbol.
    pub async fn quote(&self, symbol: impl Into<Ticker>) -> Result<QuoteSnapshot> {
        let columns = equity_quote_columns();
        let decoder = RowDecoder::new(&columns);
        let row = self.loader().fetch_one(symbol, columns).await?;
        Ok(decode_quote(&decoder, &row))
    }

    /// Fetches typed quote snapshots for multiple symbols while preserving the requested order.
    pub async fn quotes<I, T>(&self, symbols: I) -> Result<Vec<QuoteSnapshot>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Ticker>,
    {
        let columns = equity_quote_columns();
        let decoder = RowDecoder::new(&columns);
        let rows = self.loader().fetch_many(symbols, columns).await?;

        Ok(rows
            .iter()
            .map(|row| decode_quote(&decoder, row))
            .collect::<Vec<_>>())
    }

    pub async fn quotes_batch<I, T>(&self, symbols: I) -> Result<BatchResult<QuoteSnapshot>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Ticker>,
    {
        let columns = equity_quote_columns();
        let decoder = RowDecoder::new(&columns);
        let rows = self.loader().fetch_many_detailed(symbols, columns).await?;

        Ok(BatchResult {
            successes: rows
                .successes
                .into_iter()
                .map(|(ticker, row)| (ticker, decode_quote(&decoder, &row)))
                .collect(),
            missing: rows.missing,
            failures: rows.failures,
        })
    }

    /// Fetches a fundamentals snapshot for a single equity symbol.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tvdata_rs::{Result, TradingViewClient};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = TradingViewClient::builder().build()?;
    ///     let fundamentals = client.equity().fundamentals("NASDAQ:AAPL").await?;
    ///
    ///     println!("{:?}", fundamentals.market_cap);
    ///     Ok(())
    /// }
    /// ```
    pub async fn fundamentals(&self, symbol: impl Into<Ticker>) -> Result<FundamentalsSnapshot> {
        let columns = fundamentals_columns();
        let decoder = RowDecoder::new(&columns);
        let row = self.loader().fetch_one(symbol, columns).await?;
        Ok(decode_fundamentals(&decoder, &row))
    }

    pub async fn fundamentals_history(
        &self,
        symbol: impl Into<Ticker>,
    ) -> Result<PointInTimeFundamentals> {
        self.fundamentals_point_in_time(symbol).await
    }

    /// Fetches native point-in-time fundamentals history from TradingView quote sessions.
    pub async fn fundamentals_point_in_time(
        &self,
        symbol: impl Into<Ticker>,
    ) -> Result<PointInTimeFundamentals> {
        let symbol = symbol.into();
        let instrument = self.fetch_identity(&symbol).await?;
        let values = self
            .quote_session()
            .fetch_fields(&symbol, &fundamentals_history_fields())
            .await?;

        Ok(decode_point_in_time_fundamentals(instrument, &values))
    }

    pub async fn fundamentals_histories<I, T>(
        &self,
        symbols: I,
    ) -> Result<Vec<PointInTimeFundamentals>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Ticker>,
    {
        self.fetch_many_history_products(symbols, |symbol| async move {
            self.fundamentals_point_in_time(symbol).await
        })
        .await
    }

    pub async fn fundamentals_point_in_time_batch<I, T>(
        &self,
        symbols: I,
    ) -> Result<Vec<PointInTimeFundamentals>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Ticker>,
    {
        self.fundamentals_histories(symbols).await
    }

    pub async fn fundamentals_batch<I, T>(&self, symbols: I) -> Result<Vec<FundamentalsSnapshot>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Ticker>,
    {
        let columns = fundamentals_columns();
        let decoder = RowDecoder::new(&columns);
        let rows = self.loader().fetch_many(symbols, columns).await?;

        Ok(rows
            .iter()
            .map(|row| decode_fundamentals(&decoder, row))
            .collect::<Vec<_>>())
    }

    /// Fetches a rich analyst snapshot combining recommendations, targets, forecasts, earnings,
    /// and FX conversion metadata.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tvdata_rs::{Result, TradingViewClient};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = TradingViewClient::builder().build()?;
    ///     let analyst = client.equity().analyst_summary("NASDAQ:AAPL").await?;
    ///
    ///     println!("{:?}", analyst.recommendations.rating);
    ///     Ok(())
    /// }
    /// ```
    pub async fn analyst_summary(&self, symbol: impl Into<Ticker>) -> Result<AnalystSummary> {
        let columns = analyst_columns();
        let decoder = RowDecoder::new(&columns);
        let row = self.loader().fetch_one(symbol, columns).await?;
        Ok(decode_analyst(&decoder, &row))
    }

    /// Fetches native analyst estimate history from TradingView quote sessions.
    ///
    /// The returned model is point-in-time aware and distinguishes annual and quarterly
    /// observations.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tvdata_rs::{Result, TradingViewClient};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = TradingViewClient::builder().build()?;
    ///     let history = client.equity().estimate_history("NASDAQ:AAPL").await?;
    ///
    ///     println!(
    ///         "quarterly observations: {}",
    ///         history.quarterly.len()
    ///     );
    ///     Ok(())
    /// }
    /// ```
    pub async fn estimate_history(&self, symbol: impl Into<Ticker>) -> Result<EstimateHistory> {
        let symbol = symbol.into();
        let instrument = self.fetch_identity(&symbol).await?;
        let values = self
            .quote_session()
            .fetch_fields(&symbol, &estimate_history_fields())
            .await?;

        Ok(decode_estimate_history(instrument, &values))
    }

    pub async fn earnings_history(&self, symbol: impl Into<Ticker>) -> Result<EstimateHistory> {
        self.estimate_history(symbol).await
    }

    pub async fn estimate_histories<I, T>(&self, symbols: I) -> Result<Vec<EstimateHistory>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Ticker>,
    {
        self.fetch_many_history_products(symbols, |symbol| async move {
            self.estimate_history(symbol).await
        })
        .await
    }

    pub async fn earnings_histories<I, T>(&self, symbols: I) -> Result<Vec<EstimateHistory>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Ticker>,
    {
        self.estimate_histories(symbols).await
    }

    pub async fn analyst_recommendations(
        &self,
        symbol: impl Into<Ticker>,
    ) -> Result<AnalystRecommendations> {
        let columns = analyst_recommendation_columns();
        self.fetch_analyst_section(symbol, columns, decode_analyst_recommendations)
            .await
    }

    pub async fn price_targets(&self, symbol: impl Into<Ticker>) -> Result<AnalystPriceTargets> {
        let columns = analyst_price_target_columns();
        self.fetch_analyst_section(symbol, columns, decode_analyst_price_targets)
            .await
    }

    pub async fn analyst_forecasts(&self, symbol: impl Into<Ticker>) -> Result<AnalystForecasts> {
        let columns = analyst_forecast_columns();
        self.fetch_analyst_section(symbol, columns, decode_analyst_forecasts)
            .await
    }

    pub async fn earnings_calendar(&self, symbol: impl Into<Ticker>) -> Result<EarningsCalendar> {
        let columns = earnings_calendar_columns();
        self.fetch_analyst_section(symbol, columns, decode_earnings_calendar)
            .await
    }

    pub async fn earnings_events(&self, symbol: impl Into<Ticker>) -> Result<EarningsCalendar> {
        self.earnings_calendar(symbol).await
    }

    pub async fn analyst_fx_rates(&self, symbol: impl Into<Ticker>) -> Result<AnalystFxRates> {
        let columns = analyst_fx_rate_columns();
        self.fetch_analyst_section(symbol, columns, decode_analyst_fx_rates)
            .await
    }

    pub async fn analyst_summaries<I, T>(&self, symbols: I) -> Result<Vec<AnalystSummary>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Ticker>,
    {
        let columns = analyst_columns();
        let decoder = RowDecoder::new(&columns);
        let rows = self.loader().fetch_many(symbols, columns).await?;

        Ok(rows
            .iter()
            .map(|row| decode_analyst(&decoder, row))
            .collect::<Vec<_>>())
    }

    pub async fn technical_summary(&self, symbol: impl Into<Ticker>) -> Result<TechnicalSummary> {
        let columns = equity_technical_columns();
        let decoder = RowDecoder::new(&columns);
        let row = self.loader().fetch_one(symbol, columns).await?;
        Ok(decode_technical(&decoder, &row))
    }

    pub async fn technical_summaries<I, T>(&self, symbols: I) -> Result<Vec<TechnicalSummary>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Ticker>,
    {
        let columns = equity_technical_columns();
        let decoder = RowDecoder::new(&columns);
        let rows = self.loader().fetch_many(symbols, columns).await?;

        Ok(rows
            .iter()
            .map(|row| decode_technical(&decoder, row))
            .collect::<Vec<_>>())
    }

    pub async fn overview(&self, symbol: impl Into<Ticker>) -> Result<EquityOverview> {
        let columns = overview_columns();
        let decoder = RowDecoder::new(&columns);
        let row = self.loader().fetch_one(symbol, columns).await?;
        Ok(decode_overview(&decoder, &row))
    }

    pub async fn overviews<I, T>(&self, symbols: I) -> Result<Vec<EquityOverview>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Ticker>,
    {
        let columns = overview_columns();
        let decoder = RowDecoder::new(&columns);
        let rows = self.loader().fetch_many(symbols, columns).await?;

        Ok(rows
            .iter()
            .map(|row| decode_overview(&decoder, row))
            .collect::<Vec<_>>())
    }

    /// Fetches the strongest equity movers in a market by percentage change.
    pub async fn top_gainers(
        &self,
        market: impl Into<Market>,
        limit: usize,
    ) -> Result<Vec<QuoteSnapshot>> {
        self.loader()
            .fetch_market_quotes(market, limit, price::CHANGE_PERCENT.sort(SortOrder::Desc))
            .await
    }

    pub async fn top_losers(
        &self,
        market: impl Into<Market>,
        limit: usize,
    ) -> Result<Vec<QuoteSnapshot>> {
        self.loader()
            .fetch_market_quotes(market, limit, price::CHANGE_PERCENT.sort(SortOrder::Asc))
            .await
    }

    pub async fn most_active(
        &self,
        market: impl Into<Market>,
        limit: usize,
    ) -> Result<Vec<QuoteSnapshot>> {
        self.loader()
            .fetch_market_active_quotes(market, limit, price::VOLUME.sort(SortOrder::Desc))
            .await
    }

    fn loader(&self) -> SnapshotLoader<'_> {
        SnapshotLoader::new(self.client, ScanQuery::new())
    }

    fn quote_session(&self) -> QuoteSessionClient<'_> {
        QuoteSessionClient::new(self.client)
    }

    async fn fetch_analyst_section<T, F>(
        &self,
        symbol: impl Into<Ticker>,
        columns: Vec<Column>,
        decode: F,
    ) -> Result<T>
    where
        F: FnOnce(&RowDecoder, &crate::scanner::ScanRow) -> T,
    {
        let decoder = RowDecoder::new(&columns);
        let row = self.loader().fetch_one(symbol, columns).await?;
        Ok(decode(&decoder, &row))
    }

    async fn fetch_identity(&self, symbol: &Ticker) -> Result<InstrumentIdentity> {
        let columns = equity_identity_columns();
        let decoder = RowDecoder::new(&columns);
        let row = self.loader().fetch_one(symbol.clone(), columns).await?;
        Ok(decoder.identity(&row))
    }

    async fn fetch_many_history_products<I, T, O, F, Fut>(
        &self,
        symbols: I,
        fetcher: F,
    ) -> Result<Vec<O>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Ticker>,
        F: Fn(Ticker) -> Fut + Copy,
        Fut: std::future::Future<Output = Result<O>>,
    {
        let mut products =
            stream::iter(symbols.into_iter().map(Into::into).enumerate())
                .map(|(index, symbol)| async move {
                    fetcher(symbol).await.map(|product| (index, product))
                })
                .buffered(HISTORY_BATCH_CONCURRENCY)
                .try_collect::<Vec<_>>()
                .await?;
        products.sort_by_key(|(index, _)| *index);
        Ok(products.into_iter().map(|(_, product)| product).collect())
    }
}

impl TradingViewClient {
    /// Returns the high-level equity facade.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tvdata_rs::{Result, TradingViewClient};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = TradingViewClient::builder().build()?;
    ///     let movers = client.equity().top_gainers("america", 5).await?;
    ///
    ///     println!("movers: {}", movers.len());
    ///     Ok(())
    /// }
    /// ```
    pub fn equity(&self) -> EquityClient<'_> {
        EquityClient::new(self)
    }
}
