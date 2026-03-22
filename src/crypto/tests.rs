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
async fn quote_uses_crypto_route_and_price_conversion_defaults() {
    let server = MockServer::start().await;
    let columns = quote_columns();
    Mock::given(method("POST"))
        .and(path("/crypto/scan"))
        .respond_with(ResponseTemplate::new(200).set_body_string(encode_response(
            &columns,
            vec![(
                "BINANCE:BTCUSDT",
                vec![
                    ("name", json!("Bitcoin")),
                    ("market", json!("crypto")),
                    ("exchange", json!("BINANCE")),
                    ("currency", json!("USD")),
                    ("type", json!("crypto")),
                    ("close", json!(81_250.0)),
                    ("change", json!(3.1)),
                    ("volume", json!(2_300_000.0)),
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

    let quote = client.crypto().quote("BINANCE:BTCUSDT").await.unwrap();

    assert_eq!(quote.instrument.market.as_deref(), Some("crypto"));
    assert_eq!(quote.close, Some(81_250.0));
}

#[tokio::test]
async fn top_gainers_use_crypto_market_route() {
    let server = MockServer::start().await;
    let columns = quote_columns();
    Mock::given(method("POST"))
        .and(path("/crypto/scan"))
        .respond_with(ResponseTemplate::new(200).set_body_string(encode_response(
            &columns,
            vec![
                (
                    "BINANCE:SOLUSDT",
                    vec![
                        ("name", json!("Solana")),
                        ("market", json!("crypto")),
                        ("exchange", json!("BINANCE")),
                        ("currency", json!("USD")),
                        ("type", json!("crypto")),
                        ("close", json!(180.0)),
                        ("change", json!(8.5)),
                        ("volume", json!(950_000.0)),
                    ],
                ),
                (
                    "BINANCE:ETHUSDT",
                    vec![
                        ("name", json!("Ethereum")),
                        ("market", json!("crypto")),
                        ("exchange", json!("BINANCE")),
                        ("currency", json!("USD")),
                        ("type", json!("crypto")),
                        ("close", json!(4_250.0)),
                        ("change", json!(4.2)),
                        ("volume", json!(1_100_000.0)),
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

    let movers = client.crypto().top_gainers(2).await.unwrap();

    assert_eq!(movers[0].instrument.ticker.as_str(), "BINANCE:SOLUSDT");
    assert_eq!(movers[0].change_percent, Some(8.5));
}
