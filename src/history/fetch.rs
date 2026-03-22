use std::time::Duration;

use std::collections::BTreeMap;

use futures_util::SinkExt;
use futures_util::stream::{self, StreamExt as FuturesStreamExt, TryStreamExt};
use serde_json::{Value, json};
use time::OffsetDateTime;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;

use crate::batch::{BatchResult, SymbolFailure};
use crate::client::Endpoints;
use crate::error::{Error, Result};
use crate::transport::websocket::{
    connect_socket, next_session_id, parse_framed_messages, send_message, send_raw_frame,
};

use super::{Bar, HistoryRequest, HistorySeries};

const HISTORY_SESSION_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_HISTORY_PAGINATION_ROUNDS: usize = 256;

pub(crate) async fn fetch_history(
    endpoints: &Endpoints,
    auth_token: &str,
    user_agent: &str,
    session_id: Option<&str>,
    request: &HistoryRequest,
) -> Result<HistorySeries> {
    fetch_history_with_timeout(
        endpoints,
        auth_token,
        user_agent,
        session_id,
        request,
        HISTORY_SESSION_TIMEOUT,
    )
    .await
}

pub(crate) async fn fetch_history_with_timeout(
    endpoints: &Endpoints,
    auth_token: &str,
    user_agent: &str,
    session_id: Option<&str>,
    request: &HistoryRequest,
    history_timeout: Duration,
) -> Result<HistorySeries> {
    let mut socket = connect_socket(endpoints, user_agent, session_id).await?;
    let requested_chunk_bars = request.bars.max(1);

    let chart_session = next_session_id("cs");
    send_message(&mut socket, "set_auth_token", json!([auth_token])).await?;
    send_message(
        &mut socket,
        "chart_create_session",
        json!([chart_session.as_str(), ""]),
    )
    .await?;
    send_message(
        &mut socket,
        "resolve_symbol",
        json!([
            chart_session.as_str(),
            "symbol_1",
            format!(
                "={{\"symbol\":\"{}\",\"adjustment\":\"{}\",\"session\":\"{}\"}}",
                request.symbol.as_str(),
                request.adjustment.as_code(),
                request.session.as_code()
            )
        ]),
    )
    .await?;
    send_message(
        &mut socket,
        "create_series",
        json!([
            chart_session.as_str(),
            "s1",
            "s1",
            "symbol_1",
            request.interval.as_code(),
            requested_chunk_bars
        ]),
    )
    .await?;
    send_message(
        &mut socket,
        "switch_timezone",
        json!([chart_session.as_str(), "exchange"]),
    )
    .await?;

    timeout(history_timeout, async {
        let mut bars = BTreeMap::new();
        let mut pagination = request
            .fetch_all
            .then(|| HistoryPagination::new(requested_chunk_bars));
        while let Some(message) = socket.next().await {
            let message = message?;
            match message {
                Message::Text(text) => {
                    for payload in parse_framed_messages(&text)? {
                        if let Some(heartbeat) = payload.strip_prefix("~h~") {
                            send_raw_frame(&mut socket, format!("~h~{heartbeat}")).await?;
                            continue;
                        }

                        let envelope: Value = match serde_json::from_str(payload) {
                            Ok(value) => value,
                            Err(_) => continue,
                        };
                        let message_type = envelope
                            .get("m")
                            .and_then(Value::as_str)
                            .unwrap_or_default();

                        match message_type {
                            "timescale_update" => merge_timescale_update(&mut bars, &envelope)?,
                            "series_completed" => {
                                if bars.is_empty() {
                                    return Err(Error::HistoryEmpty {
                                        symbol: request.symbol.as_str().to_owned(),
                                    });
                                }

                                if let Some(pagination) = pagination.as_mut() {
                                    if let Some(chunk_bars) = pagination
                                        .next_chunk_bars(&bars, request.symbol.as_str())?
                                    {
                                        send_message(
                                            &mut socket,
                                            "request_more_data",
                                            json!([chart_session.as_str(), "s1", chunk_bars]),
                                        )
                                        .await?;
                                        continue;
                                    }
                                }

                                return Ok(history_series_from_bars(request, bars));
                            }
                            _ => {}
                        }
                    }
                }
                Message::Ping(payload) => socket.send(Message::Pong(payload)).await?,
                Message::Close(_) => break,
                _ => {}
            }
        }

        Err(Error::HistoryEmpty {
            symbol: request.symbol.as_str().to_owned(),
        })
    })
    .await
    .map_err(|_| Error::Protocol("history session timed out"))?
}

fn history_series_from_bars(request: &HistoryRequest, bars: BTreeMap<i64, Bar>) -> HistorySeries {
    HistorySeries {
        symbol: request.symbol.clone(),
        interval: request.interval,
        bars: bars.into_values().collect(),
    }
}

#[derive(Debug, Clone)]
struct HistoryPagination {
    chunk_bars: u32,
    rounds: usize,
    previous_len: usize,
    previous_oldest_timestamp: Option<i64>,
}

impl HistoryPagination {
    fn new(chunk_bars: u32) -> Self {
        Self {
            chunk_bars,
            rounds: 0,
            previous_len: 0,
            previous_oldest_timestamp: None,
        }
    }

    fn next_chunk_bars(&mut self, bars: &BTreeMap<i64, Bar>, symbol: &str) -> Result<Option<u32>> {
        let current_len = bars.len();
        let current_oldest_timestamp = bars.keys().next().copied();

        let made_progress = self.rounds == 0
            || current_len > self.previous_len
            || current_oldest_timestamp != self.previous_oldest_timestamp;

        if !made_progress {
            return Ok(None);
        }

        if self.rounds >= MAX_HISTORY_PAGINATION_ROUNDS {
            return Err(Error::HistoryPaginationLimitExceeded {
                symbol: symbol.to_owned(),
                rounds: self.rounds,
            });
        }

        self.rounds += 1;
        self.previous_len = current_len;
        self.previous_oldest_timestamp = current_oldest_timestamp;

        Ok(Some(self.chunk_bars))
    }
}

pub(crate) async fn fetch_history_batch_with<F, Fut>(
    requests: Vec<HistoryRequest>,
    concurrency: usize,
    mut fetcher: F,
) -> Result<Vec<HistorySeries>>
where
    F: FnMut(HistoryRequest) -> Fut,
    Fut: std::future::Future<Output = Result<HistorySeries>>,
{
    if concurrency == 0 {
        return Err(Error::InvalidBatchConcurrency);
    }

    if requests.is_empty() {
        return Ok(Vec::new());
    }

    let mut series = stream::iter(requests.into_iter().enumerate().map(|(index, request)| {
        let symbol = request.symbol.clone();
        let future = fetcher(request);

        async move {
            future
                .await
                .map(|series| (index, series))
                .map_err(|source| Error::HistoryDownloadFailed {
                    symbol: symbol.as_str().to_owned(),
                    source: Box::new(source),
                })
        }
    }))
    .buffer_unordered(concurrency)
    .try_collect::<Vec<_>>()
    .await?;

    series.sort_by_key(|(index, _)| *index);
    Ok(series.into_iter().map(|(_, series)| series).collect())
}

pub(crate) async fn fetch_history_batch_detailed_with<F, Fut>(
    requests: Vec<HistoryRequest>,
    concurrency: usize,
    mut fetcher: F,
) -> Result<BatchResult<HistorySeries>>
where
    F: FnMut(HistoryRequest) -> Fut,
    Fut: std::future::Future<Output = Result<HistorySeries>>,
{
    if concurrency == 0 {
        return Err(Error::InvalidBatchConcurrency);
    }

    if requests.is_empty() {
        return Ok(BatchResult::default());
    }

    let mut outcomes = stream::iter(requests.into_iter().enumerate().map(|(index, request)| {
        let symbol = request.symbol.clone();
        let future = fetcher(request);

        async move { (index, symbol, future.await) }
    }))
    .buffer_unordered(concurrency)
    .collect::<Vec<_>>()
    .await;

    outcomes.sort_by_key(|(index, _, _)| *index);

    let mut batch = BatchResult::default();
    for (_, symbol, outcome) in outcomes {
        match outcome {
            Ok(series) => {
                batch.successes.insert(symbol, series);
            }
            Err(error) if error.kind() == crate::error::ErrorKind::SymbolNotFound => {
                batch.missing.push(symbol);
            }
            Err(error) => {
                batch
                    .failures
                    .push(SymbolFailure::from_error(symbol, error));
            }
        }
    }

    Ok(batch)
}

fn merge_timescale_update(bars: &mut BTreeMap<i64, Bar>, envelope: &Value) -> Result<()> {
    let payload = envelope
        .get("p")
        .and_then(Value::as_array)
        .and_then(|parts| parts.get(1))
        .ok_or(Error::Protocol("timescale_update missing payload"))?;

    let Some(series_map) = payload.as_object() else {
        return Err(Error::Protocol("timescale_update payload is not an object"));
    };

    for series in series_map.values() {
        let Some(entries) = series.get("s").and_then(Value::as_array) else {
            continue;
        };

        for entry in entries {
            let Some(values) = entry.get("v").and_then(Value::as_array) else {
                continue;
            };
            if values.len() < 5 {
                continue;
            }
            let timestamp = values[0]
                .as_f64()
                .ok_or(Error::Protocol("history timestamp is not numeric"))?
                as i64;
            let open = as_f64(values.get(1))?;
            let high = as_f64(values.get(2))?;
            let low = as_f64(values.get(3))?;
            let close = as_f64(values.get(4))?;
            let volume = values.get(5).and_then(Value::as_f64);
            let time = OffsetDateTime::from_unix_timestamp(timestamp)
                .map_err(|_| Error::Protocol("invalid unix timestamp in history payload"))?;

            bars.insert(
                timestamp,
                Bar {
                    time,
                    open,
                    high,
                    low,
                    close,
                    volume,
                },
            );
        }
    }

    Ok(())
}

fn as_f64(value: Option<&Value>) -> Result<f64> {
    value
        .and_then(Value::as_f64)
        .ok_or(Error::Protocol("history OHLC value is not numeric"))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;
    use std::time::Duration;

    use futures_util::StreamExt;
    use serde_json::Value;
    use time::macros::datetime;
    use tokio::net::TcpListener;
    use tokio::time::sleep;
    use tokio_tungstenite::{WebSocketStream, accept_async, tungstenite::Message};

    use super::*;
    use crate::client::Endpoints;
    use crate::history::Interval;
    use crate::scanner::Ticker;

    #[test]
    fn merges_history_bars_from_timescale_update() {
        let payload = serde_json::json!({
            "m": "timescale_update",
            "p": [
                "cs_tvrs",
                {
                    "s1": {
                        "s": [
                            { "i": 0, "v": [1773667800.0, 252.105, 253.885, 249.88, 252.82, 32074209.0] },
                            { "i": 1, "v": [1773754200.0, 252.955, 255.13, 252.18, 254.23, 32361607.0] }
                        ]
                    }
                }
            ]
        });

        let mut bars = BTreeMap::new();
        merge_timescale_update(&mut bars, &payload).unwrap();

        assert_eq!(bars.len(), 2);
        assert_eq!(bars.values().next().unwrap().open, 252.105);
    }

    #[tokio::test]
    async fn batch_history_preserves_request_order() {
        let requests = vec![
            HistoryRequest::new("NASDAQ:MSFT", Interval::Day1, 2),
            HistoryRequest::new("NASDAQ:AAPL", Interval::Day1, 2),
        ];

        let series = fetch_history_batch_with(requests, 2, |request| async move {
            Ok(HistorySeries {
                symbol: request.symbol,
                interval: request.interval,
                bars: vec![Bar {
                    time: datetime!(2026-03-20 00:00 UTC),
                    open: 1.0,
                    high: 2.0,
                    low: 0.5,
                    close: 1.5,
                    volume: Some(10.0),
                }],
            })
        })
        .await
        .unwrap();

        assert_eq!(series[0].symbol.as_str(), "NASDAQ:MSFT");
        assert_eq!(series[1].symbol.as_str(), "NASDAQ:AAPL");
    }

    #[tokio::test]
    async fn batch_history_returns_empty_for_empty_requests() {
        let series = fetch_history_batch_with(Vec::new(), 2, |_| async {
            unreachable!("empty batches should not call the fetcher")
        })
        .await
        .unwrap();

        assert!(series.is_empty());
    }

    #[tokio::test]
    async fn batch_history_wraps_symbol_context_on_error() {
        let requests = vec![HistoryRequest::new("NASDAQ:AAPL", Interval::Day1, 2)];

        let error = fetch_history_batch_with(requests, 1, |request| async move {
            Err(Error::HistoryEmpty {
                symbol: request.symbol.as_str().to_owned(),
            })
        })
        .await
        .unwrap_err();

        assert!(matches!(
            error,
            Error::HistoryDownloadFailed { symbol, .. } if symbol == "NASDAQ:AAPL"
        ));
    }

    #[tokio::test]
    async fn batch_history_rejects_zero_concurrency() {
        let requests = vec![HistoryRequest::new("NASDAQ:AAPL", Interval::Day1, 2)];
        let error = fetch_history_batch_with(requests, 0, |_| async {
            unreachable!("validation should run before fetcher")
        })
        .await
        .unwrap_err();

        assert!(matches!(error, Error::InvalidBatchConcurrency));
    }

    #[tokio::test]
    async fn detailed_batch_history_collects_successes_missing_and_failures() {
        let requests = vec![
            HistoryRequest::new("NASDAQ:MSFT", Interval::Day1, 2),
            HistoryRequest::new("NASDAQ:AAPL", Interval::Day1, 2),
            HistoryRequest::new("NASDAQ:NVDA", Interval::Day1, 2),
        ];

        let batch = fetch_history_batch_detailed_with(requests, 2, |request| async move {
            match request.symbol.as_str() {
                "NASDAQ:MSFT" => Ok(HistorySeries {
                    symbol: request.symbol,
                    interval: request.interval,
                    bars: vec![Bar {
                        time: datetime!(2026-03-20 00:00 UTC),
                        open: 1.0,
                        high: 2.0,
                        low: 0.5,
                        close: 1.5,
                        volume: Some(10.0),
                    }],
                }),
                "NASDAQ:AAPL" => Err(Error::HistoryEmpty {
                    symbol: request.symbol.as_str().to_owned(),
                }),
                _ => Err(Error::Protocol("broken payload")),
            }
        })
        .await
        .unwrap();

        assert!(
            batch
                .successes
                .contains_key(&Ticker::from_static("NASDAQ:MSFT"))
        );
        assert_eq!(batch.missing, vec![Ticker::from_static("NASDAQ:AAPL")]);
        assert_eq!(batch.failures.len(), 1);
        assert_eq!(batch.failures[0].symbol.as_str(), "NASDAQ:NVDA");
    }

    #[tokio::test]
    async fn history_fetch_times_out_when_socket_never_completes() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();

            for _ in 0..5 {
                match socket.next().await {
                    Some(Ok(Message::Text(_))) => {}
                    other => panic!("unexpected websocket client message: {other:?}"),
                }
            }

            sleep(Duration::from_millis(100)).await;
            socket.close(None).await.unwrap();
        });

        let endpoints = Endpoints::default()
            .with_websocket_url(format!("ws://{address}"))
            .unwrap();
        let request = HistoryRequest::new("NASDAQ:AAPL", Interval::Day1, 2);

        let error = fetch_history_with_timeout(
            &endpoints,
            "unauthorized_user_token",
            "tvdata-rs/test",
            None,
            &request,
            Duration::from_millis(25),
        )
        .await
        .unwrap_err();

        assert!(matches!(
            error,
            Error::Protocol("history session timed out")
        ));
        server.await.unwrap();
    }

    #[tokio::test]
    async fn history_fetch_all_requests_more_data_until_history_stops_growing() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();

            for _ in 0..5 {
                assert!(read_client_method(&mut socket).await.is_some());
            }

            send_server_message(
                &mut socket,
                serde_json::json!({
                    "m": "timescale_update",
                    "p": [
                        "cs_test",
                        {
                            "s1": {
                                "s": [
                                    { "i": 0, "v": [200.0, 11.0, 12.0, 10.0, 11.5, 100.0] },
                                    { "i": 1, "v": [300.0, 12.0, 13.0, 11.0, 12.5, 150.0] }
                                ]
                            }
                        }
                    ]
                }),
            )
            .await;
            send_server_message(
                &mut socket,
                serde_json::json!({ "m": "series_completed", "p": ["cs_test", "s1"] }),
            )
            .await;

            assert_eq!(
                read_client_method(&mut socket).await.as_deref(),
                Some("request_more_data")
            );

            send_server_message(
                &mut socket,
                serde_json::json!({
                    "m": "timescale_update",
                    "p": [
                        "cs_test",
                        {
                            "s1": {
                                "s": [
                                    { "i": 0, "v": [100.0, 10.0, 11.0, 9.0, 10.5, 80.0] }
                                ]
                            }
                        }
                    ]
                }),
            )
            .await;
            send_server_message(
                &mut socket,
                serde_json::json!({ "m": "series_completed", "p": ["cs_test", "s1"] }),
            )
            .await;

            assert_eq!(
                read_client_method(&mut socket).await.as_deref(),
                Some("request_more_data")
            );

            send_server_message(
                &mut socket,
                serde_json::json!({ "m": "series_completed", "p": ["cs_test", "s1"] }),
            )
            .await;

            socket.close(None).await.unwrap();
        });

        let endpoints = Endpoints::default()
            .with_websocket_url(format!("ws://{address}"))
            .unwrap();
        let request = HistoryRequest::max("NASDAQ:AAPL", Interval::Day1);

        let series = fetch_history_with_timeout(
            &endpoints,
            "unauthorized_user_token",
            "tvdata-rs/test",
            None,
            &request,
            Duration::from_secs(1),
        )
        .await
        .unwrap();

        assert_eq!(series.bars.len(), 3);
        assert_eq!(series.bars[0].time, datetime!(1970-01-01 0:01:40 UTC));
        assert_eq!(series.bars[2].time, datetime!(1970-01-01 0:05:00 UTC));
        server.await.unwrap();
    }

    async fn read_client_method(
        socket: &mut WebSocketStream<tokio::net::TcpStream>,
    ) -> Option<String> {
        while let Some(message) = socket.next().await {
            let message = message.unwrap();
            if let Message::Text(text) = message {
                let payload = parse_framed_messages(&text).unwrap().remove(0).to_owned();
                let envelope: Value = serde_json::from_str(&payload).unwrap();
                return envelope.get("m").and_then(Value::as_str).map(str::to_owned);
            }
        }

        None
    }

    async fn send_server_message(
        socket: &mut WebSocketStream<tokio::net::TcpStream>,
        payload: Value,
    ) {
        let payload = payload.to_string();
        socket
            .send(Message::Text(
                format!("~m~{}~m~{payload}", payload.len()).into(),
            ))
            .await
            .unwrap();
    }
}
