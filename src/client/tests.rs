use std::sync::{
    Arc,
    atomic::{AtomicU32, Ordering},
};
use std::time::Duration;

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
