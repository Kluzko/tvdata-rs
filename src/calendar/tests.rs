use std::collections::HashMap;

use serde_json::Value;
use serde_json::json;
use time::macros::datetime;
use wiremock::matchers::{body_string_contains, method, path};
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

fn encode_response(
    columns: &[Column],
    total_count: usize,
    rows: Vec<(&str, Vec<(&str, Value)>)>,
) -> String {
    let data = rows
        .into_iter()
        .map(|(symbol, entries)| encode_row(columns, symbol, entries))
        .collect::<Vec<_>>();

    serde_json::to_string(&json!({
        "totalCount": total_count,
        "data": data,
    }))
    .unwrap()
}

#[tokio::test]
async fn earnings_calendar_clips_to_requested_window() {
    let server = MockServer::start().await;
    let columns = earnings_calendar_columns();
    Mock::given(method("POST"))
        .and(path("/america/scan"))
        .and(body_string_contains(r#""range":[0,200]"#))
        .respond_with(ResponseTemplate::new(200).set_body_string(encode_response(
            &columns,
            4,
            vec![
                (
                    "NASDAQ:PAST",
                    vec![
                        ("name", json!("Past")),
                        ("market", json!("america")),
                        ("exchange", json!("NASDAQ")),
                        ("currency", json!("USD")),
                        ("country", json!("US")),
                        ("type", json!("stock")),
                        ("earnings_release_next_date", json!(1769904000i64)),
                    ],
                ),
                (
                    "NASDAQ:IN1",
                    vec![
                        ("name", json!("In One")),
                        ("market", json!("america")),
                        ("exchange", json!("NASDAQ")),
                        ("currency", json!("USD")),
                        ("country", json!("US")),
                        ("type", json!("stock")),
                        ("earnings_release_next_date", json!(1770249600i64)),
                        ("earnings_release_next_time", json!(1)),
                        ("earnings_per_share_forecast_next_fq", json!(1.23)),
                    ],
                ),
                (
                    "NASDAQ:IN2",
                    vec![
                        ("name", json!("In Two")),
                        ("market", json!("america")),
                        ("exchange", json!("NASDAQ")),
                        ("currency", json!("USD")),
                        ("country", json!("US")),
                        ("type", json!("stock")),
                        ("earnings_release_next_date", json!(1770422400i64)),
                    ],
                ),
                (
                    "NASDAQ:FUTURE",
                    vec![
                        ("name", json!("Future")),
                        ("market", json!("america")),
                        ("exchange", json!("NASDAQ")),
                        ("currency", json!("USD")),
                        ("country", json!("US")),
                        ("type", json!("stock")),
                        ("earnings_release_next_date", json!(1771113600i64)),
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

    let request = CalendarWindowRequest::new(
        "america",
        datetime!(2026-02-03 00:00 UTC),
        datetime!(2026-02-10 00:00 UTC),
    );

    let entries = client.corporate_earnings_calendar(&request).await.unwrap();

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].instrument.ticker.as_str(), "NASDAQ:IN1");
    assert_eq!(entries[1].instrument.ticker.as_str(), "NASDAQ:IN2");
    assert_eq!(entries[0].release_time_code, Some(1));
    assert_eq!(entries[0].eps_forecast_next_fq, Some(1.23));
}

#[tokio::test]
async fn dividend_calendar_uses_requested_effective_date_kind() {
    let server = MockServer::start().await;
    let columns = dividend_calendar_columns();
    Mock::given(method("POST"))
        .and(path("/america/scan"))
        .respond_with(ResponseTemplate::new(200).set_body_string(encode_response(
            &columns,
            1,
            vec![(
                "NYSE:ABC",
                vec![
                    ("name", json!("ABC")),
                    ("market", json!("america")),
                    ("exchange", json!("NYSE")),
                    ("currency", json!("USD")),
                    ("country", json!("US")),
                    ("type", json!("stock")),
                    ("dividend_amount_upcoming", json!(0.42)),
                    ("dividend_yield_upcoming", json!(1.9)),
                    ("ex_dividend_date_upcoming", json!(1771286400i64)),
                    ("payment_date_upcoming", json!(1771718400i64)),
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

    let ex_entries = client
        .corporate_dividend_calendar(&DividendCalendarRequest::new(
            "america",
            datetime!(2026-02-10 00:00 UTC),
            datetime!(2026-02-25 00:00 UTC),
        ))
        .await
        .unwrap();
    let payment_entries = client
        .corporate_dividend_calendar(
            &DividendCalendarRequest::new(
                "america",
                datetime!(2026-02-10 00:00 UTC),
                datetime!(2026-02-25 00:00 UTC),
            )
            .date_kind(DividendDateKind::PaymentDate),
        )
        .await
        .unwrap();

    assert_eq!(
        ex_entries[0].effective_date,
        datetime!(2026-02-17 00:00 UTC)
    );
    assert_eq!(
        payment_entries[0].effective_date,
        datetime!(2026-02-22 00:00 UTC)
    );
    assert_eq!(payment_entries[0].amount, Some(0.42));
}

#[tokio::test]
async fn ipo_calendar_scans_descending_but_returns_ascending_window() {
    let server = MockServer::start().await;
    let columns = ipo_calendar_columns();
    Mock::given(method("POST"))
        .and(path("/america/scan"))
        .and(body_string_contains(r#""sortOrder":"desc""#))
        .respond_with(ResponseTemplate::new(200).set_body_string(encode_response(
            &columns,
            4,
            vec![
                (
                    "NASDAQ:FUTURE",
                    vec![
                        ("name", json!("Future")),
                        ("market", json!("america")),
                        ("exchange", json!("NASDAQ")),
                        ("currency", json!("USD")),
                        ("country", json!("US")),
                        ("type", json!("stock")),
                        ("ipo_offer_date", json!(1772323200i64)),
                    ],
                ),
                (
                    "NASDAQ:IN2",
                    vec![
                        ("name", json!("In Two")),
                        ("market", json!("america")),
                        ("exchange", json!("NASDAQ")),
                        ("currency", json!("USD")),
                        ("country", json!("US")),
                        ("type", json!("stock")),
                        ("ipo_offer_date", json!(1771459200i64)),
                        ("ipo_offer_price_usd", json!(15.0)),
                    ],
                ),
                (
                    "NASDAQ:IN1",
                    vec![
                        ("name", json!("In One")),
                        ("market", json!("america")),
                        ("exchange", json!("NASDAQ")),
                        ("currency", json!("USD")),
                        ("country", json!("US")),
                        ("type", json!("stock")),
                        ("ipo_offer_date", json!(1771113600i64)),
                        ("ipo_offer_price_usd", json!(13.5)),
                    ],
                ),
                (
                    "NASDAQ:PAST",
                    vec![
                        ("name", json!("Past")),
                        ("market", json!("america")),
                        ("exchange", json!("NASDAQ")),
                        ("currency", json!("USD")),
                        ("country", json!("US")),
                        ("type", json!("stock")),
                        ("ipo_offer_date", json!(1770595200i64)),
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

    let request = CalendarWindowRequest::new(
        "america",
        datetime!(2026-02-12 00:00 UTC),
        datetime!(2026-02-25 00:00 UTC),
    )
    .limit(10);

    let entries = client.corporate_ipo_calendar(&request).await.unwrap();

    assert_eq!(entries.len(), 2);
    assert_eq!(entries[0].instrument.ticker.as_str(), "NASDAQ:IN1");
    assert_eq!(entries[1].instrument.ticker.as_str(), "NASDAQ:IN2");
    assert_eq!(entries[0].offer_price_usd, Some(13.5));
    assert_eq!(entries[1].offer_price_usd, Some(15.0));
}

#[tokio::test]
async fn calendar_requests_return_empty_for_zero_limit_or_inverted_window() {
    let client = TradingViewClient::builder().build().unwrap();
    let empty = client
        .corporate_earnings_calendar(
            &CalendarWindowRequest::new(
                "america",
                datetime!(2026-03-01 00:00 UTC),
                datetime!(2026-03-10 00:00 UTC),
            )
            .limit(0),
        )
        .await
        .unwrap();
    assert!(empty.is_empty());

    let inverted = client
        .corporate_ipo_calendar(&CalendarWindowRequest::new(
            "america",
            datetime!(2026-03-10 00:00 UTC),
            datetime!(2026-03-01 00:00 UTC),
        ))
        .await
        .unwrap();
    assert!(inverted.is_empty());
}
