use crate::batch::BatchResult;
use crate::client::TradingViewClient;
use crate::error::Result;
use crate::market_data::{
    QuoteSnapshot, RowDecoder, SnapshotLoader, TechnicalSummary, decode_quote, decode_technical,
    quote_columns, technical_columns,
};
use crate::scanner::{Column, PriceConversion, ScanQuery, SortOrder, Ticker};

#[cfg(test)]
mod tests;

/// High-level crypto market facade for quote snapshots, technicals, and movers.
///
/// # Examples
///
/// ```no_run
/// use tvdata_rs::{Result, TradingViewClient};
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     let client = TradingViewClient::builder().build()?;
///     let quote = client.crypto().quote("BINANCE:BTCUSDT").await?;
///
///     println!("{:?}", quote.close);
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct CryptoClient<'a> {
    client: &'a TradingViewClient,
}

impl<'a> CryptoClient<'a> {
    pub const fn new(client: &'a TradingViewClient) -> Self {
        Self { client }
    }

    pub fn client(&self) -> &'a TradingViewClient {
        self.client
    }

    /// Fetches a typed crypto quote snapshot for a single symbol.
    pub async fn quote(&self, symbol: impl Into<Ticker>) -> Result<QuoteSnapshot> {
        let columns = quote_columns();
        let decoder = RowDecoder::new(&columns);
        let row = self.loader().fetch_one(symbol, columns).await?;
        Ok(decode_quote(&decoder, &row))
    }

    /// Fetches typed crypto quote snapshots for multiple symbols while preserving the requested
    /// order.
    pub async fn quotes<I, T>(&self, symbols: I) -> Result<Vec<QuoteSnapshot>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Ticker>,
    {
        let columns = quote_columns();
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
        let columns = quote_columns();
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

    pub async fn technical_summary(&self, symbol: impl Into<Ticker>) -> Result<TechnicalSummary> {
        let columns = technical_columns();
        let decoder = RowDecoder::new(&columns);
        let row = self.loader().fetch_one(symbol, columns).await?;
        Ok(decode_technical(&decoder, &row))
    }

    pub async fn technical_summaries<I, T>(&self, symbols: I) -> Result<Vec<TechnicalSummary>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Ticker>,
    {
        let columns = technical_columns();
        let decoder = RowDecoder::new(&columns);
        let rows = self.loader().fetch_many(symbols, columns).await?;

        Ok(rows
            .iter()
            .map(|row| decode_technical(&decoder, row))
            .collect::<Vec<_>>())
    }

    pub async fn overview(&self, symbol: impl Into<Ticker>) -> Result<CryptoOverview> {
        let columns = overview_columns();
        let decoder = RowDecoder::new(&columns);
        let row = self.loader().fetch_one(symbol, columns).await?;
        Ok(CryptoOverview {
            quote: decode_quote(&decoder, &row),
            technicals: decode_technical(&decoder, &row),
        })
    }

    pub async fn overviews<I, T>(&self, symbols: I) -> Result<Vec<CryptoOverview>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Ticker>,
    {
        let columns = overview_columns();
        let decoder = RowDecoder::new(&columns);
        let rows = self.loader().fetch_many(symbols, columns).await?;

        Ok(rows
            .iter()
            .map(|row| CryptoOverview {
                quote: decode_quote(&decoder, row),
                technicals: decode_technical(&decoder, row),
            })
            .collect::<Vec<_>>())
    }

    /// Fetches the strongest crypto movers by percentage change.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tvdata_rs::{Result, TradingViewClient};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = TradingViewClient::builder().build()?;
    ///     let movers = client.crypto().top_gainers(10).await?;
    ///
    ///     println!("movers: {}", movers.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn top_gainers(&self, limit: usize) -> Result<Vec<QuoteSnapshot>> {
        self.loader()
            .fetch_market_quotes(
                "crypto",
                limit,
                crate::scanner::fields::price::CHANGE_PERCENT.sort(SortOrder::Desc),
            )
            .await
    }

    pub async fn top_losers(&self, limit: usize) -> Result<Vec<QuoteSnapshot>> {
        self.loader()
            .fetch_market_quotes(
                "crypto",
                limit,
                crate::scanner::fields::price::CHANGE_PERCENT.sort(SortOrder::Asc),
            )
            .await
    }

    pub async fn most_active(&self, limit: usize) -> Result<Vec<QuoteSnapshot>> {
        self.loader()
            .fetch_market_active_quotes(
                "crypto",
                limit,
                crate::scanner::fields::price::VOLUME.sort(SortOrder::Desc),
            )
            .await
    }

    fn loader(&self) -> SnapshotLoader<'_> {
        SnapshotLoader::new(
            self.client,
            ScanQuery::new()
                .market("crypto")
                .price_conversion(PriceConversion::MarketCurrency),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CryptoOverview {
    pub quote: QuoteSnapshot,
    pub technicals: TechnicalSummary,
}

impl TradingViewClient {
    /// Returns the high-level crypto facade.
    pub fn crypto(&self) -> CryptoClient<'_> {
        CryptoClient::new(self)
    }
}

fn overview_columns() -> Vec<Column> {
    crate::market_data::merge_columns([quote_columns(), technical_columns()])
}
