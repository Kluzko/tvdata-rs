use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU32, AtomicUsize, Ordering},
};
use std::time::{Duration, Instant};

use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use serde_json::json;
use time::macros::datetime;
use tokio::net::TcpListener;
use tokio_tungstenite::{accept_async, tungstenite::Message};
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, Respond, ResponseTemplate};

use super::*;
use crate::scanner::Column;
use crate::scanner::ScanQuery;
use crate::scanner::fields::{analyst, core, price};
use crate::search::SearchRequest;

#[derive(Clone)]
struct EventuallySuccessfulScan {
    attempts: Arc<AtomicU32>,
}

impl EventuallySuccessfulScan {
    fn new() -> Self {
        Self {
            attempts: Arc::new(AtomicU32::new(0)),
        }
    }
}

impl Respond for EventuallySuccessfulScan {
    fn respond(&self, _request: &wiremock::Request) -> ResponseTemplate {
        let attempt = self.attempts.fetch_add(1, Ordering::SeqCst);
        if attempt == 0 {
            ResponseTemplate::new(503)
        } else {
            ResponseTemplate::new(200).set_body_string(
                r#"{"totalCount":1,"data":[{"s":"NASDAQ:AAPL","d":["AAPL",247.99]}]}"#,
            )
        }
    }
}

#[derive(Clone)]
struct DelayedSearchResponder {
    in_flight: Arc<AtomicUsize>,
    max_in_flight: Arc<AtomicUsize>,
}

impl DelayedSearchResponder {
    fn new() -> Self {
        Self {
            in_flight: Arc::new(AtomicUsize::new(0)),
            max_in_flight: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn max_in_flight(&self) -> usize {
        self.max_in_flight.load(Ordering::SeqCst)
    }
}

impl Respond for DelayedSearchResponder {
    fn respond(&self, _request: &wiremock::Request) -> ResponseTemplate {
        let current = self.in_flight.fetch_add(1, Ordering::SeqCst) + 1;
        self.max_in_flight.fetch_max(current, Ordering::SeqCst);
        std::thread::sleep(Duration::from_millis(60));
        self.in_flight.fetch_sub(1, Ordering::SeqCst);

        ResponseTemplate::new(200).set_body_string(r#"{"symbols_remaining":0,"symbols":[]}"#)
    }
}

#[derive(Clone)]
struct DynamicScanResponder {
    calls: Arc<AtomicUsize>,
}

impl DynamicScanResponder {
    fn new() -> Self {
        Self {
            calls: Arc::new(AtomicUsize::new(0)),
        }
    }

    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl Respond for DynamicScanResponder {
    fn respond(&self, request: &wiremock::Request) -> ResponseTemplate {
        self.calls.fetch_add(1, Ordering::SeqCst);

        let payload: Value = serde_json::from_slice(&request.body).unwrap();
        let columns = payload
            .get("columns")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let tickers = payload
            .get("symbols")
            .and_then(|symbols| symbols.get("tickers"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();

        let rows = tickers
            .into_iter()
            .filter_map(|ticker| ticker.as_str().map(str::to_owned))
            .map(|ticker| {
                let symbol = ticker.clone();
                let values = columns
                    .iter()
                    .map(|column| {
                        scan_value_for_column(column.as_str().unwrap_or_default(), &ticker)
                    })
                    .collect::<Vec<_>>();
                json!({
                    "s": symbol,
                    "d": values,
                })
            })
            .collect::<Vec<_>>();

        ResponseTemplate::new(200).set_body_json(json!({
            "totalCount": rows.len(),
            "data": rows,
        }))
    }
}

fn scan_value_for_column(column: &str, ticker: &str) -> Value {
    match column {
        "name" => json!(ticker.rsplit(':').next().unwrap_or(ticker)),
        "market" => json!("america"),
        "exchange" => json!(ticker.split(':').next().unwrap_or("NASDAQ")),
        "currency" => json!("USD"),
        "country" => json!("US"),
        "type" => json!("stock"),
        "open" => json!(100.0),
        "high" => json!(101.0),
        "low" => json!(99.0),
        "close" => json!(100.5),
        "change" => json!(1.25),
        "change_abs" => json!(1.0),
        "volume" => json!(1_000_000.0),
        "relative_volume_10d_calc" => json!(1.1),
        "market_cap_basic" => json!(1_000_000_000.0),
        "Recommend.All" => json!(0.4),
        "RSI" => json!(57.0),
        "MACD.macd" => json!(1.2),
        "SMA50" => json!(98.0),
        "SMA200" => json!(91.0),
        "EMA20" => json!(99.5),
        "ADX" => json!(24.0),
        "ATR" => json!(2.0),
        "price_target_average" => json!(120.0),
        "price_target_high" => json!(130.0),
        "price_target_low" => json!(110.0),
        "price_target_median" => json!(119.0),
        "recommendation_buy" => json!(12),
        "recommendation_hold" => json!(8),
        "recommendation_sell" => json!(1),
        "recommendation_mark" => json!(1.8),
        "recommendation_over" => json!(4),
        "recommendation_under" => json!(1),
        "recommendation_total" => json!(26),
        "price_earnings_ttm" => json!(24.0),
        "price_book_fq" => json!(8.0),
        "price_sales_current" => json!(7.5),
        "total_revenue" => json!(420_000_000_000.0),
        "net_income" => json!(95_000_000_000.0),
        "earnings_per_share_diluted_ttm" => json!(6.2),
        "return_on_equity" => json!(0.33),
        "debt_to_equity" => json!(0.55),
        _ => Value::Null,
    }
}

#[derive(Debug, Clone, Default)]
struct CountingWebSocketConnector {
    calls: Arc<AtomicUsize>,
}

impl CountingWebSocketConnector {
    fn calls(&self) -> usize {
        self.calls.load(Ordering::SeqCst)
    }
}

impl WebSocketConnector for CountingWebSocketConnector {
    fn connect<'a>(
        &'a self,
        endpoints: &'a Endpoints,
        user_agent: &'a str,
        session_id: Option<&'a str>,
    ) -> WebSocketConnectFuture<'a> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Box::pin(crate::transport::websocket::connect_socket(
            endpoints, user_agent, session_id,
        ))
    }
}

#[derive(Debug, Clone, Default)]
struct RecordingObserver {
    events: Arc<Mutex<Vec<ClientEvent>>>,
}

impl RecordingObserver {
    fn events(&self) -> Vec<ClientEvent> {
        self.events.lock().unwrap().clone()
    }
}

impl ClientObserver for RecordingObserver {
    fn on_event(&self, event: &ClientEvent) {
        self.events.lock().unwrap().push(event.clone());
    }
}

async fn spawn_history_batch_server(
    expected_connections: usize,
    response_delay: Duration,
) -> (
    std::net::SocketAddr,
    Arc<AtomicUsize>,
    tokio::task::JoinHandle<()>,
) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let max_in_flight = Arc::new(AtomicUsize::new(0));
    let in_flight = Arc::new(AtomicUsize::new(0));
    let server_max_in_flight = Arc::clone(&max_in_flight);
    let server_in_flight = Arc::clone(&in_flight);

    let server = tokio::spawn(async move {
        let mut connection_tasks = Vec::with_capacity(expected_connections);

        for _ in 0..expected_connections {
            let (stream, _) = listener.accept().await.unwrap();
            let in_flight = Arc::clone(&server_in_flight);
            let max_in_flight = Arc::clone(&server_max_in_flight);
            connection_tasks.push(tokio::spawn(async move {
                let mut socket = accept_async(stream).await.unwrap();
                let current = in_flight.fetch_add(1, Ordering::SeqCst) + 1;
                max_in_flight.fetch_max(current, Ordering::SeqCst);

                while let Some(message) = socket.next().await {
                    let message = message.unwrap();
                    if let Message::Text(text) = message {
                        let payload = crate::transport::websocket::parse_framed_messages(&text)
                            .unwrap()
                            .remove(0)
                            .to_owned();
                        let envelope: Value = serde_json::from_str(&payload).unwrap();
                        let method = envelope
                            .get("m")
                            .and_then(Value::as_str)
                            .unwrap_or_default();

                        if method == "create_series" {
                            tokio::time::sleep(response_delay).await;
                            send_ws_message(
                                &mut socket,
                                serde_json::json!({
                                    "m": "timescale_update",
                                    "p": [
                                        "cs_test",
                                        {
                                            "s1": {
                                                "s": [
                                                    { "i": 0, "v": [1773667800.0, 252.105, 253.885, 249.88, 252.82, 32074209.0] }
                                                ]
                                            }
                                        }
                                    ]
                                }),
                            )
                            .await;
                            send_ws_message(
                                &mut socket,
                                serde_json::json!({ "m": "series_completed", "p": ["cs_test", "s1"] }),
                            )
                            .await;
                            break;
                        }
                    }
                }

                in_flight.fetch_sub(1, Ordering::SeqCst);
            }));
        }

        for task in connection_tasks {
            task.await.unwrap();
        }
    });

    (address, max_in_flight, server)
}

#[tokio::test]
async fn scan_uses_market_route() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/america/scan"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"totalCount":1,"data":[{"s":"NASDAQ:AAPL","d":["AAPL",247.99]}]}"#,
        ))
        .mount(&server)
        .await;

    let endpoints = Endpoints::default()
        .with_scanner_base_url(server.uri())
        .unwrap();
    let client = TradingViewClient::builder()
        .endpoints(endpoints)
        .build()
        .unwrap();

    let query = ScanQuery::new()
        .market("america")
        .select([core::NAME, price::CLOSE]);
    let response = client.scan(&query).await.unwrap();

    assert_eq!(response.total_count, 1);
    assert_eq!(response.rows[0].symbol, "NASDAQ:AAPL");
}

#[tokio::test]
async fn search_sanitizes_highlight_markup() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/symbol_search/v3"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"symbols_remaining":0,"symbols":[{"symbol":"<em>AAPL</em>","description":"Apple <em>Inc.</em>","exchange":"NASDAQ","type":"stock","cik_code":"0000320193"}]}"#,
        ))
        .mount(&server)
        .await;

    let endpoints = Endpoints::default()
        .with_symbol_search_base_url(format!("{}/symbol_search/v3", server.uri()))
        .unwrap();
    let client = TradingViewClient::builder()
        .endpoints(endpoints)
        .build()
        .unwrap();

    let hits = client.search(&SearchRequest::new("AAPL")).await.unwrap();
    assert_eq!(hits[0].symbol, "AAPL");
    assert_eq!(hits[0].description.as_deref(), Some("Apple Inc."));
    assert_eq!(hits[0].highlighted_symbol.as_deref(), Some("<em>AAPL</em>"));
    assert_eq!(hits[0].cik_code.as_deref(), Some("0000320193"));
}

#[tokio::test]
async fn search_response_decodes_remaining_symbols_and_source_metadata() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/symbol_search/v3"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{
                "symbols_remaining": 4,
                "symbols": [
                    {
                        "symbol": "<em>AAPL</em>",
                        "description": "Apple <em>Inc.</em>",
                        "exchange": "NASDAQ",
                        "type": "stock",
                        "source2": {
                            "id": "NASDAQ",
                            "name": "Nasdaq Stock Market",
                            "description": "Primary listing"
                        },
                        "logo": {
                            "style": "single",
                            "logoid": "apple"
                        }
                    }
                ]
            }"#,
        ))
        .mount(&server)
        .await;

    let endpoints = Endpoints::default()
        .with_symbol_search_base_url(format!("{}/symbol_search/v3", server.uri()))
        .unwrap();
    let client = TradingViewClient::builder()
        .endpoints(endpoints)
        .build()
        .unwrap();

    let response = client
        .search_response(&SearchRequest::new("AAPL"))
        .await
        .unwrap();

    assert_eq!(response.symbols_remaining, 4);
    assert_eq!(
        response.hits[0]
            .source
            .as_ref()
            .and_then(|s| s.id.as_deref()),
        Some("NASDAQ")
    );
    assert_eq!(
        response.hits[0]
            .logo
            .as_ref()
            .and_then(|logo| logo.logoid.as_deref()),
        Some("apple")
    );
}

#[tokio::test]
async fn search_equities_response_uses_stock_search_type() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/symbol_search/v3"))
        .and(query_param("search_type", "stock"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(r#"{"symbols_remaining":0,"symbols":[]}"#),
        )
        .mount(&server)
        .await;

    let endpoints = Endpoints::default()
        .with_symbol_search_base_url(format!("{}/symbol_search/v3", server.uri()))
        .unwrap();
    let client = TradingViewClient::builder()
        .endpoints(endpoints)
        .build()
        .unwrap();

    let response = client.search_equities_response("AAPL").await.unwrap();
    assert!(response.hits.is_empty());
}

#[tokio::test]
async fn snapshot_batch_single_request_strategy_uses_one_scan_call() {
    let server = MockServer::start().await;
    let responder = DynamicScanResponder::new();
    Mock::given(method("POST"))
        .and(path("/global/scan"))
        .respond_with(responder.clone())
        .mount(&server)
        .await;

    let endpoints = Endpoints::default()
        .with_scanner_base_url(server.uri())
        .unwrap();
    let client = TradingViewClient::builder()
        .endpoints(endpoints)
        .snapshot_batch_config(
            SnapshotBatchConfig::builder()
                .strategy(SnapshotBatchStrategy::SingleRequest)
                .build(),
        )
        .build()
        .unwrap();

    let symbols = (0..1000)
        .map(|index| format!("NASDAQ:S{index:04}"))
        .collect::<Vec<_>>();
    let batch = client.equity().quotes_batch(symbols).await.unwrap();

    assert_eq!(batch.successes.len(), 1000);
    assert_eq!(responder.calls(), 1);
}

#[tokio::test]
async fn snapshot_batch_chunked_strategy_slices_large_batches() {
    let server = MockServer::start().await;
    let responder = DynamicScanResponder::new();
    Mock::given(method("POST"))
        .and(path("/global/scan"))
        .respond_with(responder.clone())
        .mount(&server)
        .await;

    let endpoints = Endpoints::default()
        .with_scanner_base_url(server.uri())
        .unwrap();
    let client = TradingViewClient::builder()
        .endpoints(endpoints)
        .snapshot_batch_config(
            SnapshotBatchConfig::builder()
                .strategy(SnapshotBatchStrategy::Chunked {
                    chunk_size: 250,
                    max_concurrent_requests: 4,
                })
                .build(),
        )
        .build()
        .unwrap();

    let symbols = (0..600)
        .map(|index| format!("NASDAQ:C{index:04}"))
        .collect::<Vec<_>>();
    let batch = client.equity().quotes_batch(symbols).await.unwrap();

    assert_eq!(batch.successes.len(), 600);
    assert_eq!(responder.calls(), 3);
}

#[tokio::test]
async fn snapshot_batch_auto_strategy_keeps_1000_quote_batches_single_request() {
    let server = MockServer::start().await;
    let responder = DynamicScanResponder::new();
    Mock::given(method("POST"))
        .and(path("/global/scan"))
        .respond_with(responder.clone())
        .mount(&server)
        .await;

    let endpoints = Endpoints::default()
        .with_scanner_base_url(server.uri())
        .unwrap();
    let client = TradingViewClient::builder()
        .endpoints(endpoints)
        .build()
        .unwrap();

    let symbols = (0..1000)
        .map(|index| format!("NASDAQ:A{index:04}"))
        .collect::<Vec<_>>();
    let batch = client.equity().quotes_batch(symbols).await.unwrap();

    assert_eq!(batch.successes.len(), 1000);
    assert_eq!(responder.calls(), 1);
}

#[tokio::test]
async fn snapshot_batch_auto_strategy_preserves_order_across_chunks() {
    let server = MockServer::start().await;
    let responder = DynamicScanResponder::new();
    Mock::given(method("POST"))
        .and(path("/global/scan"))
        .respond_with(responder.clone())
        .mount(&server)
        .await;

    let endpoints = Endpoints::default()
        .with_scanner_base_url(server.uri())
        .unwrap();
    let client = TradingViewClient::builder()
        .endpoints(endpoints)
        .build()
        .unwrap();

    let symbols = (0..1500)
        .rev()
        .map(|index| format!("NASDAQ:O{index:04}"))
        .collect::<Vec<_>>();
    let quotes = client.equity().quotes(symbols.clone()).await.unwrap();

    assert_eq!(quotes.len(), symbols.len());
    assert_eq!(
        quotes.first().unwrap().instrument.ticker.as_str(),
        symbols[0]
    );
    assert_eq!(
        quotes.last().unwrap().instrument.ticker.as_str(),
        symbols[symbols.len() - 1]
    );
    assert!(responder.calls() > 1);
}

#[tokio::test]
async fn search_options_response_filters_to_option_like_hits() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/symbol_search/v3"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{
                "symbols_remaining": 0,
                "symbols": [
                    {
                        "symbol": "NASDAQ:AAPL",
                        "description": "Apple Inc.",
                        "exchange": "NASDAQ",
                        "type": "stock"
                    },
                    {
                        "symbol": "AAPL240621C00195000",
                        "description": "Apple call",
                        "exchange": "OPRA",
                        "type": "structured",
                        "option-type": "call"
                    }
                ]
            }"#,
        ))
        .mount(&server)
        .await;

    let endpoints = Endpoints::default()
        .with_symbol_search_base_url(format!("{}/symbol_search/v3", server.uri()))
        .unwrap();
    let client = TradingViewClient::builder()
        .endpoints(endpoints)
        .build()
        .unwrap();

    let response = client.search_options_response("AAPL").await.unwrap();

    assert_eq!(response.hits.len(), 1);
    assert_eq!(response.hits[0].symbol, "AAPL240621C00195000");
}

#[tokio::test]
async fn metainfo_uses_market_route_and_decodes_fields() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/america/metainfo"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{
                "financial_currency":"USD",
                "fields":[
                    {"n":"close","t":"price","r":null},
                    {"n":"country","t":"text","r":["United States","Canada"]}
                ]
            }"#,
        ))
        .mount(&server)
        .await;

    let endpoints = Endpoints::default()
        .with_scanner_base_url(server.uri())
        .unwrap();
    let client = TradingViewClient::builder()
        .endpoints(endpoints)
        .build()
        .unwrap();

    let metainfo = client.metainfo("america").await.unwrap();

    assert_eq!(metainfo.financial_currency.as_deref(), Some("USD"));
    assert!(metainfo.supports_field("close"));
    assert_eq!(
        metainfo
            .field("country")
            .and_then(|field| field.enum_values())
            .map(|values| values.len()),
        Some(2)
    );
}

#[tokio::test]
async fn metainfo_uses_cache_across_calls() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/america/metainfo"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"financial_currency":"USD","fields":[{"n":"close","t":"price","r":null}]}"#,
        ))
        .expect(1)
        .mount(&server)
        .await;

    let endpoints = Endpoints::default()
        .with_scanner_base_url(server.uri())
        .unwrap();
    let client = TradingViewClient::builder()
        .endpoints(endpoints)
        .build()
        .unwrap();

    let first = client.metainfo("america").await.unwrap();
    let second = client.metainfo("america").await.unwrap();

    assert_eq!(first, second);
}

#[tokio::test]
async fn economic_calendar_decodes_typed_events() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/events"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{
                "status": "ok",
                "result": [
                    {
                        "id": "event-1",
                        "title": "GDP",
                        "indicator": "GDP Growth Rate",
                        "country": "US",
                        "currency": "USD",
                        "date": "2026-03-22T12:30:00Z",
                        "importance": 2,
                        "actual": 2.1,
                        "forecast": 2.0,
                        "previous": "1.9",
                        "period": "Q1",
                        "source": "BEA"
                    }
                ]
            }"#,
        ))
        .mount(&server)
        .await;

    let endpoints = Endpoints::default()
        .with_calendar_base_url(format!("{}/events", server.uri()))
        .unwrap();
    let client = TradingViewClient::builder()
        .endpoints(endpoints)
        .build()
        .unwrap();

    let response = client
        .economic_calendar(&crate::economics::EconomicCalendarRequest::trailing(1))
        .await
        .unwrap();

    assert_eq!(response.status.as_deref(), Some("ok"));
    assert_eq!(response.events.len(), 1);
    assert_eq!(response.events[0].country.as_deref(), Some("US"));
}

#[tokio::test]
async fn search_uses_session_cookie_when_configured() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/symbol_search/v3"))
        .and(header("cookie", "sessionid=test-session"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(r#"{"symbols_remaining":0,"symbols":[]}"#),
        )
        .mount(&server)
        .await;

    let endpoints = Endpoints::default()
        .with_symbol_search_base_url(format!("{}/symbol_search/v3", server.uri()))
        .unwrap();
    let client = TradingViewClient::builder()
        .endpoints(endpoints)
        .session_id("test-session")
        .build()
        .unwrap();

    let response = client
        .search_response(&SearchRequest::new("AAPL"))
        .await
        .unwrap();

    assert!(response.hits.is_empty());
}

#[tokio::test]
async fn search_uses_session_cookie_from_auth_config() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/symbol_search/v3"))
        .and(header("cookie", "sessionid=test-session"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(r#"{"symbols_remaining":0,"symbols":[]}"#),
        )
        .mount(&server)
        .await;

    let endpoints = Endpoints::default()
        .with_symbol_search_base_url(format!("{}/symbol_search/v3", server.uri()))
        .unwrap();
    let client = TradingViewClient::builder()
        .endpoints(endpoints)
        .auth(AuthConfig::session("test-session"))
        .build()
        .unwrap();

    let response = client
        .search_response(&SearchRequest::new("AAPL"))
        .await
        .unwrap();

    assert!(response.hits.is_empty());
}

#[tokio::test]
async fn search_uses_injected_http_client_and_applies_default_headers() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/symbol_search/v3"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(r#"{"symbols_remaining":0,"symbols":[]}"#),
        )
        .mount(&server)
        .await;

    let endpoints = Endpoints::default()
        .with_symbol_search_base_url(format!("{}/symbol_search/v3", server.uri()))
        .unwrap();
    let shared_http =
        reqwest_middleware::ClientWithMiddleware::from(reqwest::Client::builder().build().unwrap());
    let client = TradingViewClient::builder()
        .endpoints(endpoints)
        .session_id("test-session")
        .http_client(shared_http)
        .build()
        .unwrap();

    let response = client
        .search_response(&SearchRequest::new("AAPL"))
        .await
        .unwrap();

    assert!(response.hits.is_empty());
    let requests = server.received_requests().await.unwrap();
    let request = &requests[0];
    assert_eq!(
        request
            .headers
            .get("origin")
            .and_then(|value| value.to_str().ok()),
        Some("https://www.tradingview.com/")
    );
    assert_eq!(
        request
            .headers
            .get("referer")
            .and_then(|value| value.to_str().ok()),
        Some("https://www.tradingview.com/")
    );
    assert_eq!(
        request
            .headers
            .get("cookie")
            .and_then(|value| value.to_str().ok()),
        Some("sessionid=test-session")
    );
}

#[tokio::test]
async fn observer_receives_http_request_completed_event() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/symbol_search/v3"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(r#"{"symbols_remaining":0,"symbols":[]}"#),
        )
        .mount(&server)
        .await;

    let observer = RecordingObserver::default();
    let endpoints = Endpoints::default()
        .with_symbol_search_base_url(format!("{}/symbol_search/v3", server.uri()))
        .unwrap();
    let client = TradingViewClient::builder()
        .endpoints(endpoints)
        .observer(Arc::new(observer.clone()))
        .build()
        .unwrap();

    let response = client
        .search_response(&SearchRequest::new("AAPL"))
        .await
        .unwrap();

    assert!(response.hits.is_empty());
    assert!(observer.events().iter().any(|event| matches!(
        event,
        ClientEvent::HttpRequestCompleted(HttpRequestCompletedEvent { method, status, .. })
            if method == "GET" && *status == 200
    )));
}

#[test]
fn auth_config_overrides_legacy_auth_fields() {
    let client = TradingViewClient::builder()
        .auth_token("legacy-token")
        .session_id("legacy-session")
        .auth(AuthConfig::session_and_token(
            "fresh-session",
            "fresh-token",
        ))
        .build()
        .unwrap();

    assert_eq!(client.session_id(), Some("fresh-session"));
    assert_eq!(client.auth_token(), "fresh-token");
}

#[test]
fn anonymous_auth_config_clears_legacy_session_fields() {
    let client = TradingViewClient::builder()
        .auth_token("legacy-token")
        .session_id("legacy-session")
        .auth(AuthConfig::anonymous())
        .build()
        .unwrap();

    assert_eq!(client.session_id(), None);
    assert_eq!(client.auth_token(), "unauthorized_user_token");
}

#[test]
fn builder_accepts_injected_http_client_with_invalid_retry_bounds() {
    let shared_http =
        reqwest_middleware::ClientWithMiddleware::from(reqwest::Client::builder().build().unwrap());

    let client = TradingViewClient::builder()
        .http_client(shared_http)
        .retry(
            RetryConfig::builder()
                .min_retry_interval(Duration::from_secs(2))
                .max_retry_interval(Duration::from_millis(500))
                .build(),
        )
        .build();

    assert!(client.is_ok());
}

#[test]
fn transport_config_overrides_flat_transport_fields() {
    let shared_http =
        reqwest_middleware::ClientWithMiddleware::from(reqwest::Client::builder().build().unwrap());

    let client = TradingViewClient::builder()
        .retry(
            RetryConfig::builder()
                .min_retry_interval(Duration::from_secs(2))
                .max_retry_interval(Duration::from_millis(500))
                .build(),
        )
        .http_client(shared_http.clone())
        .transport_config(
            TransportConfig::builder()
                .retry(RetryConfig::default())
                .http_client(shared_http)
                .user_agent("tvdata-rs/grouped-test")
                .build(),
        )
        .build();

    assert!(client.is_ok());
}

#[tokio::test]
async fn request_budget_serializes_http_requests_when_configured() {
    let server = MockServer::start().await;
    let responder = DelayedSearchResponder::new();
    Mock::given(method("GET"))
        .and(path("/symbol_search/v3"))
        .respond_with(responder.clone())
        .mount(&server)
        .await;

    let endpoints = Endpoints::default()
        .with_symbol_search_base_url(format!("{}/symbol_search/v3", server.uri()))
        .unwrap();
    let client = TradingViewClient::builder()
        .endpoints(endpoints)
        .request_budget(
            RequestBudget::builder()
                .max_concurrent_http_requests(1)
                .build(),
        )
        .build()
        .unwrap();

    let first_request = SearchRequest::new("AAPL");
    let second_request = SearchRequest::new("MSFT");
    let started_at = Instant::now();
    let first = client.search_response(&first_request);
    let second = client.search_response(&second_request);
    let (first, second) = tokio::join!(first, second);
    let elapsed = started_at.elapsed();

    first.unwrap();
    second.unwrap();

    assert_eq!(responder.max_in_flight(), 1);
    assert!(elapsed >= Duration::from_millis(100));
}

#[test]
fn builder_rejects_zero_request_budget_limits() {
    let error = TradingViewClient::builder()
        .request_budget(
            RequestBudget::builder()
                .max_concurrent_http_requests(0)
                .build(),
        )
        .build()
        .unwrap_err();

    assert!(matches!(error, Error::InvalidRequestBudget { .. }));
}

#[test]
fn builder_rejects_invalid_snapshot_batch_config() {
    let error = TradingViewClient::builder()
        .snapshot_batch_config(
            SnapshotBatchConfig::builder()
                .strategy(SnapshotBatchStrategy::Chunked {
                    chunk_size: 0,
                    max_concurrent_requests: 4,
                })
                .build(),
        )
        .build()
        .unwrap_err();

    assert!(matches!(error, Error::InvalidSnapshotBatchConfig { .. }));
}

#[test]
fn backend_history_preset_applies_request_budget_defaults() {
    let client = TradingViewClient::for_backend_history().unwrap();

    assert_eq!(
        client.request_budget().max_concurrent_http_requests,
        Some(8)
    );
    assert_eq!(
        client.request_budget().max_concurrent_websocket_sessions,
        Some(6)
    );
    assert_eq!(
        client.request_budget().min_http_interval,
        Some(Duration::from_millis(50))
    );
    assert_eq!(client.history_config().default_batch_concurrency, 6);
}

#[tokio::test]
async fn from_config_uses_grouped_auth_and_transport_settings() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/symbol_search/v3"))
        .and(header("cookie", "sessionid=config-session"))
        .and(header("user-agent", "tvdata-rs/config-test"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(r#"{"symbols_remaining":0,"symbols":[]}"#),
        )
        .mount(&server)
        .await;

    let config = TradingViewClientConfig::builder()
        .endpoints(
            Endpoints::default()
                .with_symbol_search_base_url(format!("{}/symbol_search/v3", server.uri()))
                .unwrap(),
        )
        .transport(
            TransportConfig::builder()
                .user_agent("tvdata-rs/config-test")
                .build(),
        )
        .auth(AuthConfig::session("config-session"))
        .build();
    let client = TradingViewClient::from_config(config).unwrap();

    let response = client
        .search_response(&SearchRequest::new("AAPL"))
        .await
        .unwrap();

    assert!(response.hits.is_empty());
}

#[test]
fn grouped_profile_config_matches_backend_history_preset() {
    let client =
        TradingViewClient::from_config(TradingViewClientConfig::backend_history()).unwrap();

    assert_eq!(client.history_config().default_batch_concurrency, 6);
    assert_eq!(
        client.request_budget().max_concurrent_http_requests,
        Some(8)
    );
    assert_eq!(
        client.request_budget().max_concurrent_websocket_sessions,
        Some(6)
    );
}

#[tokio::test]
async fn history_batch_caps_effective_concurrency_to_websocket_budget() {
    let expected_connections = 8;
    let (address, max_in_flight, server) =
        spawn_history_batch_server(expected_connections, Duration::from_millis(60)).await;

    let endpoints = Endpoints::default()
        .with_websocket_url(format!("ws://{address}"))
        .unwrap();
    let request = crate::HistoryBatchRequest::new(
        (0..expected_connections).map(|idx| format!("NASDAQ:SYM{idx}")),
        crate::Interval::Day1,
        1,
    )
    .concurrency(expected_connections);
    let client = TradingViewClient::builder()
        .endpoints(endpoints)
        .request_budget(
            RequestBudget::builder()
                .max_concurrent_websocket_sessions(3)
                .build(),
        )
        .build()
        .unwrap();

    let batch = client.history_batch_detailed(&request).await.unwrap();

    assert_eq!(batch.successes.len(), expected_connections);
    assert!(batch.missing.is_empty());
    assert!(batch.failures.is_empty());
    assert!(max_in_flight.load(Ordering::SeqCst) <= 3);
    server.await.unwrap();
}

#[tokio::test]
async fn backend_history_preset_uses_safe_daily_ingestion_envelope() {
    let expected_connections = 8;
    let (address, max_in_flight, server) =
        spawn_history_batch_server(expected_connections, Duration::from_millis(80)).await;

    let endpoints = Endpoints::default()
        .with_websocket_url(format!("ws://{address}"))
        .unwrap();
    let mut config = TradingViewClientConfig::backend_history();
    config.endpoints = endpoints;
    let client = TradingViewClient::from_config(config).unwrap();

    let series = client
        .download_history(
            (0..expected_connections).map(|idx| format!("NASDAQ:BAR{idx}")),
            crate::Interval::Day1,
            1,
        )
        .await
        .unwrap();

    assert_eq!(series.len(), expected_connections);
    assert!(max_in_flight.load(Ordering::SeqCst) <= 6);
    server.await.unwrap();
}

#[tokio::test]
async fn custom_websocket_connector_is_used_for_history_requests() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();

        while let Some(message) = socket.next().await {
            let message = message.unwrap();
            if let Message::Text(text) = message {
                let payload = crate::transport::websocket::parse_framed_messages(&text)
                    .unwrap()
                    .remove(0)
                    .to_owned();
                let envelope: Value = serde_json::from_str(&payload).unwrap();
                let method = envelope
                    .get("m")
                    .and_then(Value::as_str)
                    .unwrap_or_default();

                if method == "create_series" {
                    send_ws_message(
                        &mut socket,
                        serde_json::json!({
                            "m": "timescale_update",
                            "p": [
                                "cs_test",
                                {
                                    "s1": {
                                        "s": [
                                            { "i": 0, "v": [1773667800.0, 252.105, 253.885, 249.88, 252.82, 32074209.0] }
                                        ]
                                    }
                                }
                            ]
                        }),
                    )
                    .await;
                    send_ws_message(
                        &mut socket,
                        serde_json::json!({ "m": "series_completed", "p": ["cs_test", "s1"] }),
                    )
                    .await;
                    break;
                }
            }
        }
    });

    let connector = CountingWebSocketConnector::default();
    let endpoints = Endpoints::default()
        .with_websocket_url(format!("ws://{address}"))
        .unwrap();
    let client = TradingViewClient::from_config(
        TradingViewClientConfig::builder()
            .endpoints(endpoints)
            .transport(
                TransportConfig::builder()
                    .websocket_connector(Arc::new(connector.clone()))
                    .build(),
            )
            .build(),
    )
    .unwrap();

    let series = client
        .history(&crate::HistoryRequest::new(
            "NASDAQ:AAPL",
            crate::Interval::Day1,
            1,
        ))
        .await
        .unwrap();

    assert_eq!(connector.calls(), 1);
    assert_eq!(series.bars.len(), 1);
    assert_eq!(series.bars[0].time, datetime!(2026-03-16 13:30:00 UTC));
    server.await.unwrap();
}

#[tokio::test]
async fn observer_receives_websocket_and_history_batch_events() {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let server = tokio::spawn(async move {
        let (stream, _) = listener.accept().await.unwrap();
        let mut socket = accept_async(stream).await.unwrap();

        while let Some(message) = socket.next().await {
            let message = message.unwrap();
            if let Message::Text(text) = message {
                let payload = crate::transport::websocket::parse_framed_messages(&text)
                    .unwrap()
                    .remove(0)
                    .to_owned();
                let envelope: Value = serde_json::from_str(&payload).unwrap();
                let method = envelope
                    .get("m")
                    .and_then(Value::as_str)
                    .unwrap_or_default();

                if method == "create_series" {
                    send_ws_message(
                        &mut socket,
                        serde_json::json!({
                            "m": "timescale_update",
                            "p": [
                                "cs_test",
                                {
                                    "s1": {
                                        "s": [
                                            { "i": 0, "v": [1773667800.0, 252.105, 253.885, 249.88, 252.82, 32074209.0] }
                                        ]
                                    }
                                }
                            ]
                        }),
                    )
                    .await;
                    send_ws_message(
                        &mut socket,
                        serde_json::json!({ "m": "series_completed", "p": ["cs_test", "s1"] }),
                    )
                    .await;
                    break;
                }
            }
        }
    });

    let observer = RecordingObserver::default();
    let endpoints = Endpoints::default()
        .with_websocket_url(format!("ws://{address}"))
        .unwrap();
    let client = TradingViewClient::builder()
        .endpoints(endpoints)
        .observer(Arc::new(observer.clone()))
        .build()
        .unwrap();
    let request = crate::HistoryBatchRequest::new(["NASDAQ:AAPL"], crate::Interval::Day1, 1);

    let batch = client.history_batch_detailed(&request).await.unwrap();

    assert_eq!(batch.successes.len(), 1);
    let events = observer.events();
    assert!(events.iter().any(|event| matches!(
        event,
        ClientEvent::WebSocketConnected(WebSocketConnectedEvent { .. })
    )));
    assert!(events.iter().any(|event| matches!(
        event,
        ClientEvent::HistoryBatchCompleted(HistoryBatchCompletedEvent {
            requested,
            successes,
            missing,
            failures,
            mode,
            ..
        }) if *requested == 1
            && *successes == 1
            && *missing == 0
            && *failures == 0
            && *mode == HistoryBatchMode::Detailed
    )));
    server.await.unwrap();
}

async fn send_ws_message(
    socket: &mut tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
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

#[tokio::test]
async fn scan_validated_rejects_unsupported_fields_before_scan_request() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/america/metainfo"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"financial_currency":"USD","fields":[{"n":"close","t":"price","r":null}]}"#,
        ))
        .mount(&server)
        .await;

    let endpoints = Endpoints::default()
        .with_scanner_base_url(server.uri())
        .unwrap();
    let client = TradingViewClient::builder()
        .endpoints(endpoints)
        .build()
        .unwrap();

    let query = ScanQuery::new()
        .market("america")
        .select([price::CLOSE, Column::from_static("imaginary_field")]);

    let error = client.scan_validated(&query).await.unwrap_err();

    assert!(matches!(
        error,
        Error::UnsupportedScanFields { fields, .. }
            if fields == vec![String::from("imaginary_field")]
    ));
}

#[tokio::test]
async fn validate_scan_query_keeps_registry_backed_interface_fields() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/america/metainfo"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"financial_currency":"USD","fields":[{"n":"close","t":"price","r":null}]}"#,
        ))
        .mount(&server)
        .await;

    let endpoints = Endpoints::default()
        .with_scanner_base_url(server.uri())
        .unwrap();
    let client = TradingViewClient::builder()
        .endpoints(endpoints)
        .build()
        .unwrap();

    let query = ScanQuery::new()
        .market("america")
        .select([core::NAME, price::CLOSE]);

    let report = client.validate_scan_query(&query).await.unwrap();

    assert!(report.is_strictly_supported());
    assert!(report.unsupported_columns.is_empty());
}

#[tokio::test]
async fn validate_scan_query_marks_partial_multi_market_support() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/america/metainfo"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{
                "fields":[
                    {"n":"close","t":"price","r":null},
                    {"n":"market_cap_basic","t":"number","r":null}
                ]
            }"#,
        ))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/crypto/metainfo"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{
                "fields":[
                    {"n":"close","t":"price","r":null}
                ]
            }"#,
        ))
        .mount(&server)
        .await;

    let endpoints = Endpoints::default()
        .with_scanner_base_url(server.uri())
        .unwrap();
    let client = TradingViewClient::builder()
        .endpoints(endpoints)
        .build()
        .unwrap();

    let query = ScanQuery::new().markets(["america", "crypto"]).select([
        price::CLOSE,
        crate::scanner::fields::fundamentals::MARKET_CAP_BASIC,
    ]);

    let report = client.validate_scan_query(&query).await.unwrap();

    assert!(!report.is_strictly_supported());
    assert!(report.is_leniently_supported());
    assert_eq!(report.partially_supported_columns.len(), 1);
    assert_eq!(
        report.partially_supported_columns[0].column.as_str(),
        "market_cap_basic"
    );
}

#[tokio::test]
async fn validate_scan_query_requires_markets() {
    let client = TradingViewClient::builder().build().unwrap();
    let query = ScanQuery::new().tickers(["NASDAQ:AAPL"]);

    let error = client.validate_scan_query(&query).await.unwrap_err();

    assert!(matches!(error, Error::ScanValidationUnavailable { .. }));
}

#[tokio::test]
async fn filter_scan_query_drops_partially_supported_columns() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/america/metainfo"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{
                "fields":[
                    {"n":"close","t":"price","r":null},
                    {"n":"market_cap_basic","t":"number","r":null}
                ]
            }"#,
        ))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/crypto/metainfo"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{
                "fields":[
                    {"n":"close","t":"price","r":null}
                ]
            }"#,
        ))
        .mount(&server)
        .await;

    let endpoints = Endpoints::default()
        .with_scanner_base_url(server.uri())
        .unwrap();
    let client = TradingViewClient::builder()
        .endpoints(endpoints)
        .build()
        .unwrap();

    let query = ScanQuery::new().markets(["america", "crypto"]).select([
        price::CLOSE,
        crate::scanner::fields::fundamentals::MARKET_CAP_BASIC,
    ]);

    let (filtered, report) = client.filter_scan_query(&query).await.unwrap();

    assert_eq!(report.filtered_column_names(), vec!["close"]);
    assert_eq!(filtered.columns, vec![price::CLOSE]);
}

#[tokio::test]
async fn scan_supported_executes_filtered_query() {
    let server = MockServer::start().await;
    Mock::given(method("GET"))
        .and(path("/america/metainfo"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{
                "fields":[
                    {"n":"close","t":"price","r":null},
                    {"n":"market_cap_basic","t":"number","r":null}
                ]
            }"#,
        ))
        .mount(&server)
        .await;
    Mock::given(method("GET"))
        .and(path("/crypto/metainfo"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{
                "fields":[
                    {"n":"close","t":"price","r":null}
                ]
            }"#,
        ))
        .mount(&server)
        .await;
    Mock::given(method("POST"))
        .and(path("/global/scan"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(
                r#"{"totalCount":1,"data":[{"s":"BINANCE:BTCUSDT","d":[65000.0]}]}"#,
            ),
        )
        .mount(&server)
        .await;

    let endpoints = Endpoints::default()
        .with_scanner_base_url(server.uri())
        .unwrap();
    let client = TradingViewClient::builder()
        .endpoints(endpoints)
        .build()
        .unwrap();

    let query = ScanQuery::new().markets(["america", "crypto"]).select([
        price::CLOSE,
        crate::scanner::fields::fundamentals::MARKET_CAP_BASIC,
    ]);

    let response = client.scan_supported(&query).await.unwrap();
    let requests = server.received_requests().await.unwrap();
    let scan_body = String::from_utf8_lossy(&requests[2].body);

    assert_eq!(response.total_count, 1);
    assert!(scan_body.contains(r#""columns":["close"]"#));
    assert!(!scan_body.contains("market_cap_basic"));
}

#[tokio::test]
async fn scan_returns_api_message_for_payload_errors() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/america/scan"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{"totalCount":0,"error":"Unknown field \"bad_field\"","data":null}"#,
        ))
        .mount(&server)
        .await;

    let endpoints = Endpoints::default()
        .with_scanner_base_url(server.uri())
        .unwrap();
    let client = TradingViewClient::builder()
        .endpoints(endpoints)
        .build()
        .unwrap();

    let query = ScanQuery::new().market("america").select([
        analyst::PRICE_TARGET_AVERAGE,
        Column::from_static("bad_field"),
    ]);
    let error = client.scan(&query).await.unwrap_err();

    assert!(matches!(error, Error::ApiMessage(message) if message.contains("bad_field")));
}

#[tokio::test]
async fn scan_retries_transient_failures() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/america/scan"))
        .respond_with(EventuallySuccessfulScan::new())
        .expect(2)
        .mount(&server)
        .await;

    let endpoints = Endpoints::default()
        .with_scanner_base_url(server.uri())
        .unwrap();
    let retry = RetryConfig::builder()
        .max_retries(1)
        .min_retry_interval(Duration::from_millis(1))
        .max_retry_interval(Duration::from_millis(5))
        .jitter(RetryJitter::None)
        .build();
    let client = TradingViewClient::builder()
        .endpoints(endpoints)
        .retry(retry)
        .build()
        .unwrap();

    let query = ScanQuery::new()
        .market("america")
        .select([core::NAME, price::CLOSE]);
    let response = client.scan(&query).await.unwrap();

    assert_eq!(response.total_count, 1);
}

#[test]
fn builder_rejects_invalid_retry_bounds() {
    let error = TradingViewClient::builder()
        .retry(
            RetryConfig::builder()
                .min_retry_interval(Duration::from_secs(2))
                .max_retry_interval(Duration::from_millis(500))
                .build(),
        )
        .build()
        .unwrap_err();

    assert!(matches!(error, Error::InvalidRetryBounds { .. }));
}
