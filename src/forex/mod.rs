use crate::batch::BatchResult;
use crate::client::TradingViewClient;
use crate::error::Result;
use crate::market_data::{
    QuoteSnapshot, RowDecoder, SnapshotLoader, TechnicalSummary, decode_quote, decode_technical,
    identity_columns, merge_columns, technical_columns,
};
use crate::scanner::fields::price;
use crate::scanner::{Column, ScanQuery, SortOrder, Ticker};

#[cfg(test)]
mod tests;

/// High-level FX market facade for quote snapshots, technicals, and movers.
///
/// # Examples
///
/// ```no_run
/// use tvdata_rs::{Result, TradingViewClient};
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     let client = TradingViewClient::builder().build()?;
///     let quote = client.forex().quote("FX:EURUSD").await?;
///
///     println!("{:?}", quote.close);
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone, Copy)]
pub struct ForexClient<'a> {
    client: &'a TradingViewClient,
}

impl<'a> ForexClient<'a> {
    pub const fn new(client: &'a TradingViewClient) -> Self {
        Self { client }
    }

    pub fn client(&self) -> &'a TradingViewClient {
        self.client
    }

    /// Fetches a typed FX quote snapshot for a single symbol.
    pub async fn quote(&self, symbol: impl Into<Ticker>) -> Result<QuoteSnapshot> {
        let columns = forex_quote_columns();
        let decoder = RowDecoder::new(&columns);
        let row = self.loader().fetch_one(symbol, columns).await?;
        Ok(decode_quote(&decoder, &row))
    }

    /// Fetches typed FX quote snapshots for multiple symbols while preserving the requested
    /// order.
    pub async fn quotes<I, T>(&self, symbols: I) -> Result<Vec<QuoteSnapshot>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Ticker>,
    {
        let columns = forex_quote_columns();
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
        let columns = forex_quote_columns();
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

    pub async fn overview(&self, symbol: impl Into<Ticker>) -> Result<ForexOverview> {
        let columns = overview_columns();
        let decoder = RowDecoder::new(&columns);
        let row = self.loader().fetch_one(symbol, columns).await?;
        Ok(ForexOverview {
            quote: decode_quote(&decoder, &row),
            technicals: decode_technical(&decoder, &row),
        })
    }

    pub async fn overviews<I, T>(&self, symbols: I) -> Result<Vec<ForexOverview>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Ticker>,
    {
        let columns = overview_columns();
        let decoder = RowDecoder::new(&columns);
        let rows = self.loader().fetch_many(symbols, columns).await?;

        Ok(rows
            .iter()
            .map(|row| ForexOverview {
                quote: decode_quote(&decoder, row),
                technicals: decode_technical(&decoder, row),
            })
            .collect::<Vec<_>>())
    }

    pub async fn top_gainers(&self, limit: usize) -> Result<Vec<QuoteSnapshot>> {
        self.loader()
            .fetch_market_quotes_with_columns(
                "forex",
                limit,
                price::CHANGE_PERCENT.sort(SortOrder::Desc),
                forex_quote_columns(),
                false,
            )
            .await
    }

    pub async fn top_losers(&self, limit: usize) -> Result<Vec<QuoteSnapshot>> {
        self.loader()
            .fetch_market_quotes_with_columns(
                "forex",
                limit,
                price::CHANGE_PERCENT.sort(SortOrder::Asc),
                forex_quote_columns(),
                false,
            )
            .await
    }

    /// Fetches the most active FX instruments by TradingView volume.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tvdata_rs::{Result, TradingViewClient};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = TradingViewClient::builder().build()?;
    ///     let movers = client.forex().most_active(10).await?;
    ///
    ///     println!("movers: {}", movers.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn most_active(&self, limit: usize) -> Result<Vec<QuoteSnapshot>> {
        self.loader()
            .fetch_market_quotes_with_columns(
                "forex",
                limit,
                price::VOLUME.sort(SortOrder::Desc),
                forex_quote_columns(),
                true,
            )
            .await
    }

    fn loader(&self) -> SnapshotLoader<'_> {
        SnapshotLoader::new(
            self.client,
            ScanQuery::new().market("forex").symbol_types(["forex"]),
        )
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForexOverview {
    pub quote: QuoteSnapshot,
    pub technicals: TechnicalSummary,
}

impl TradingViewClient {
    /// Returns the high-level FX facade.
    pub fn forex(&self) -> ForexClient<'_> {
        ForexClient::new(self)
    }
}

fn overview_columns() -> Vec<Column> {
    merge_columns([forex_quote_columns(), technical_columns()])
}

fn forex_quote_columns() -> Vec<Column> {
    merge_columns([
        identity_columns(),
        vec![
            price::OPEN,
            price::HIGH,
            price::LOW,
            price::CLOSE,
            price::CHANGE_PERCENT,
            price::CHANGE_ABS,
            price::VOLUME,
            price::RELATIVE_VOLUME,
        ],
    ])
}
