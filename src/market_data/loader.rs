use std::collections::{HashMap, HashSet};

use crate::client::TradingViewClient;
use crate::error::{Error, Result};
use crate::scanner::fields::price;
use crate::scanner::{Column, Market, ScanQuery, ScanRow, SortSpec, Ticker};

use super::columns::quote_columns;
use super::decode::{RowDecoder, decode_quote};
use super::types::QuoteSnapshot;

#[derive(Debug, Clone)]
pub(crate) struct SnapshotLoader<'a> {
    client: &'a TradingViewClient,
    base_query: ScanQuery,
}

impl<'a> SnapshotLoader<'a> {
    pub(crate) fn new(client: &'a TradingViewClient, base_query: ScanQuery) -> Self {
        Self { client, base_query }
    }

    pub(crate) async fn fetch_one(
        &self,
        symbol: impl Into<Ticker>,
        columns: Vec<Column>,
    ) -> Result<ScanRow> {
        let symbol = symbol.into();
        let requested = symbol.as_str().to_owned();
        let mut rows = self.fetch_many([symbol], columns).await?;

        rows.iter()
            .position(|row| row.symbol == requested)
            .map(|index| rows.swap_remove(index))
            .ok_or(Error::SymbolNotFound { symbol: requested })
    }

    pub(crate) async fn fetch_many<I, T>(
        &self,
        symbols: I,
        columns: Vec<Column>,
    ) -> Result<Vec<ScanRow>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Ticker>,
    {
        let requested = symbols.into_iter().map(Into::into).collect::<Vec<Ticker>>();
        if requested.is_empty() {
            return Ok(Vec::new());
        }

        let mut seen = HashSet::new();
        let tickers = requested
            .iter()
            .filter(|ticker| seen.insert(ticker.as_str().to_owned()))
            .cloned()
            .collect::<Vec<_>>();
        let query = self
            .base_query
            .clone()
            .tickers(tickers)
            .select(columns)
            .page(0, seen.len())?;
        let rows = self.client.scan(&query).await?.rows;
        let rows_by_symbol = rows
            .into_iter()
            .map(|row| (row.symbol.clone(), row))
            .collect::<HashMap<_, _>>();

        requested
            .iter()
            .map(|ticker| {
                rows_by_symbol
                    .get(ticker.as_str())
                    .cloned()
                    .ok_or_else(|| Error::SymbolNotFound {
                        symbol: ticker.as_str().to_owned(),
                    })
            })
            .collect()
    }

    pub(crate) async fn fetch_market_quotes(
        &self,
        market: impl Into<Market>,
        limit: usize,
        sort: SortSpec,
    ) -> Result<Vec<QuoteSnapshot>> {
        self.fetch_market_quotes_with_columns(market, limit, sort, quote_columns(), false)
            .await
    }

    pub(crate) async fn fetch_market_active_quotes(
        &self,
        market: impl Into<Market>,
        limit: usize,
        sort: SortSpec,
    ) -> Result<Vec<QuoteSnapshot>> {
        self.fetch_market_quotes_with_columns(market, limit, sort, quote_columns(), true)
            .await
    }

    pub(crate) async fn fetch_market_quotes_with_columns(
        &self,
        market: impl Into<Market>,
        limit: usize,
        sort: SortSpec,
        columns: Vec<Column>,
        require_positive_volume: bool,
    ) -> Result<Vec<QuoteSnapshot>> {
        let decoder = RowDecoder::new(&columns);
        let mut query = self
            .base_query
            .clone()
            .market(market)
            .select(columns)
            .filter(price::CLOSE.clone().gt(0));
        if require_positive_volume {
            query = query.filter(price::VOLUME.clone().gt(0));
        }
        let query = query.sort(sort).page(0, limit)?;
        let response = self.client.scan(&query).await?;

        Ok(response
            .rows
            .iter()
            .map(|row| decode_quote(&decoder, row))
            .collect::<Vec<_>>())
    }
}
