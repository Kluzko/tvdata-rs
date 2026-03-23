use std::collections::{HashMap, HashSet};

use futures_util::stream::{self, StreamExt as FuturesStreamExt, TryStreamExt};

use crate::batch::{BatchResult, SymbolFailure};
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

        let tickers = dedupe_tickers(&requested);
        let rows = self.fetch_rows(tickers, columns).await?;
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

    pub(crate) async fn fetch_many_detailed<I, T>(
        &self,
        symbols: I,
        columns: Vec<Column>,
    ) -> Result<BatchResult<ScanRow>>
    where
        I: IntoIterator<Item = T>,
        T: Into<Ticker>,
    {
        let requested = symbols.into_iter().map(Into::into).collect::<Vec<Ticker>>();
        if requested.is_empty() {
            return Ok(BatchResult::default());
        }

        let tickers = dedupe_tickers(&requested);
        self.fetch_rows_detailed(tickers, columns).await
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

fn dedupe_tickers(requested: &[Ticker]) -> Vec<Ticker> {
    let mut seen = HashSet::new();
    requested
        .iter()
        .filter(|ticker| seen.insert(ticker.as_str().to_owned()))
        .cloned()
        .collect::<Vec<_>>()
}

impl<'a> SnapshotLoader<'a> {
    async fn fetch_rows(&self, tickers: Vec<Ticker>, columns: Vec<Column>) -> Result<Vec<ScanRow>> {
        let plan = self
            .client
            .plan_snapshot_batch(tickers.len(), columns.len());
        if plan.concurrency == 1 || tickers.len() <= plan.chunk_size {
            return self.fetch_rows_single(tickers, columns).await;
        }

        let client = self.client;
        let base_query = self.base_query.clone();
        let mut chunked_rows = stream::iter(
            tickers
                .chunks(plan.chunk_size)
                .map(|chunk| chunk.to_vec())
                .enumerate()
                .map(|(index, chunk)| {
                    let columns = columns.clone();
                    let base_query = base_query.clone();
                    async move {
                        fetch_chunk_rows(client, base_query, chunk, columns)
                            .await
                            .map(|rows| (index, rows))
                    }
                }),
        )
        .buffer_unordered(plan.concurrency)
        .try_collect::<Vec<_>>()
        .await?;

        chunked_rows.sort_by_key(|(index, _)| *index);
        Ok(chunked_rows
            .into_iter()
            .flat_map(|(_, rows)| rows)
            .collect::<Vec<_>>())
    }

    async fn fetch_rows_single(
        &self,
        tickers: Vec<Ticker>,
        columns: Vec<Column>,
    ) -> Result<Vec<ScanRow>> {
        fetch_chunk_rows(self.client, self.base_query.clone(), tickers, columns).await
    }

    async fn fetch_rows_detailed(
        &self,
        tickers: Vec<Ticker>,
        columns: Vec<Column>,
    ) -> Result<BatchResult<ScanRow>> {
        let plan = self
            .client
            .plan_snapshot_batch(tickers.len(), columns.len());
        if plan.concurrency == 1 || tickers.len() <= plan.chunk_size {
            return self.fetch_rows_detailed_single(tickers, columns).await;
        }

        let client = self.client;
        let base_query = self.base_query.clone();
        let mut outcomes = stream::iter(
            tickers
                .chunks(plan.chunk_size)
                .map(|chunk| chunk.to_vec())
                .enumerate()
                .map(|(index, chunk)| {
                    let columns = columns.clone();
                    let base_query = base_query.clone();
                    async move {
                        let outcome =
                            fetch_chunk_rows(client, base_query, chunk.clone(), columns).await;
                        (index, chunk, outcome)
                    }
                }),
        )
        .buffer_unordered(plan.concurrency)
        .collect::<Vec<_>>()
        .await;

        outcomes.sort_by_key(|(index, _, _)| *index);

        let mut batch = BatchResult::default();
        for (_, chunk, outcome) in outcomes {
            match outcome {
                Ok(rows) => {
                    let rows_by_symbol = rows
                        .into_iter()
                        .map(|row| (row.symbol.clone(), row))
                        .collect::<HashMap<_, _>>();
                    for ticker in chunk {
                        match rows_by_symbol.get(ticker.as_str()).cloned() {
                            Some(row) => {
                                batch.successes.insert(ticker, row);
                            }
                            None => batch.missing.push(ticker),
                        }
                    }
                }
                Err(error) => {
                    let kind = error.kind();
                    let retryable = error.is_retryable();
                    let message = error.to_string();
                    batch
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

        Ok(batch)
    }

    async fn fetch_rows_detailed_single(
        &self,
        tickers: Vec<Ticker>,
        columns: Vec<Column>,
    ) -> Result<BatchResult<ScanRow>> {
        let response = match self.fetch_rows_single(tickers.clone(), columns).await {
            Ok(rows) => rows,
            Err(error) => {
                let kind = error.kind();
                let retryable = error.is_retryable();
                let message = error.to_string();
                let failures = tickers
                    .into_iter()
                    .map(|ticker| SymbolFailure {
                        symbol: ticker,
                        kind,
                        message: message.clone(),
                        retryable,
                    })
                    .collect();
                return Ok(BatchResult {
                    failures,
                    ..BatchResult::default()
                });
            }
        };

        let rows_by_symbol = response
            .into_iter()
            .map(|row| (row.symbol.clone(), row))
            .collect::<HashMap<_, _>>();

        let mut batch = BatchResult::default();
        for ticker in tickers {
            match rows_by_symbol.get(ticker.as_str()).cloned() {
                Some(row) => {
                    batch.successes.insert(ticker, row);
                }
                None => batch.missing.push(ticker),
            }
        }

        Ok(batch)
    }
}

async fn fetch_chunk_rows(
    client: &TradingViewClient,
    base_query: ScanQuery,
    tickers: Vec<Ticker>,
    columns: Vec<Column>,
) -> Result<Vec<ScanRow>> {
    let limit = tickers.len();
    let query = base_query.tickers(tickers).select(columns).page(0, limit)?;
    client.scan(&query).await.map(|response| response.rows)
}
