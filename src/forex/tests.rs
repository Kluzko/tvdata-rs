use std::collections::HashMap;

use serde_json::Value;
use serde_json::json;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::client::Endpoints;

fn encode_row(columns: &[Column], symbol: &str, entries: Vec<(&str, Value)>) -> Value {
    let values = entries
        .into_iter()
        .map(|(column, value)| (column.to_owned(), value))
        .collect::<HashMap<_, _>>();
    let ordered = columns
        .iter()
        .map(|column| values.get(column.as_str()).cloned().unwrap_or(Value::Null))
        .collect::<Vec<_>>();

    json!({
        "s": symbol,
        "d": ordered,
    })
}

fn encode_response(columns: &[Column], rows: Vec<(&str, Vec<(&str, Value)>)>) -> String {
    let data = rows
        .into_iter()
        .map(|(symbol, entries)| encode_row(columns, symbol, entries))
        .collect::<Vec<_>>();

    serde_json::to_string(&json!({
        "totalCount": data.len(),
        "data": data,
    }))
    .unwrap()
}

#[tokio::test]
async fn quote_uses_forex_route_and_symbol_type_defaults() {
    let server = MockServer::start().await;
    let columns = forex_quote_columns();
    Mock::given(method("POST"))
        .and(path("/forex/scan"))
        .respond_with(ResponseTemplate::new(200).set_body_string(encode_response(
            &columns,
            vec![(
                "FX:EURUSD",
                vec![
                    ("name", json!("EURUSD")),
                    ("market", json!("forex")),
                    ("exchange", json!("FX_IDC")),
                    ("currency", json!("USD")),
                    ("type", json!("forex")),
                    ("close", json!(1.0845)),
                    ("change", json!(0.3)),
                    ("volume", json!(125_000.0)),
                ],
            )],
        )))
        .mount(&server)
        .await;

    let client = TradingViewClient::builder()
        .endpoints(
            Endpoints::default()
                .with_scanner_base_url(server.uri())
                .unwrap(),
        )
        .build()
        .unwrap();

    let quote = client.forex().quote("FX:EURUSD").await.unwrap();

    assert_eq!(quote.instrument.market.as_deref(), Some("forex"));
    assert_eq!(quote.close, Some(1.0845));
}

#[tokio::test]
async fn most_active_uses_forex_market_route() {
    let server = MockServer::start().await;
    let columns = forex_quote_columns();
    Mock::given(method("POST"))
        .and(path("/forex/scan"))
        .respond_with(ResponseTemplate::new(200).set_body_string(encode_response(
            &columns,
            vec![
                (
                    "FX:EURUSD",
                    vec![
                        ("name", json!("EURUSD")),
                        ("market", json!("forex")),
                        ("exchange", json!("FX_IDC")),
                        ("currency", json!("USD")),
                        ("type", json!("forex")),
                        ("close", json!(1.0845)),
                        ("change", json!(0.3)),
                        ("volume", json!(125_000.0)),
                    ],
                ),
                (
                    "FX:USDJPY",
                    vec![
                        ("name", json!("USDJPY")),
                        ("market", json!("forex")),
                        ("exchange", json!("FX_IDC")),
                        ("currency", json!("JPY")),
                        ("type", json!("forex")),
                        ("close", json!(151.2)),
                        ("change", json!(0.1)),
                        ("volume", json!(95_000.0)),
                    ],
                ),
            ],
        )))
        .mount(&server)
        .await;

    let client = TradingViewClient::builder()
        .endpoints(
            Endpoints::default()
                .with_scanner_base_url(server.uri())
                .unwrap(),
        )
        .build()
        .unwrap();

    let movers = client.forex().most_active(2).await.unwrap();

    assert_eq!(movers[0].instrument.ticker.as_str(), "FX:EURUSD");
    assert_eq!(movers[0].volume, Some(125_000.0));
}

#[test]
fn forex_quote_columns_exclude_equity_only_market_cap() {
    let columns = forex_quote_columns();

    assert!(!columns.contains(&crate::scanner::fields::fundamentals::MARKET_CAP_BASIC));
}
