use std::collections::HashMap;

use serde_json::Value;
use serde_json::json;
use time::macros::datetime;
use wiremock::matchers::{body_string_contains, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

use super::*;
use crate::client::Endpoints;
use crate::error::Error;
use crate::market_data::quote_columns;

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

fn fx_rates(time: i64, rates: &[(&str, f64)]) -> Value {
    let mut payload = serde_json::Map::from_iter([("time".to_owned(), json!(time))]);
    payload.extend(
        rates
            .iter()
            .map(|(currency, value)| ((*currency).to_owned(), json!(value))),
    );
    Value::Object(payload)
}

#[tokio::test]
async fn quote_decodes_typed_snapshot() {
    let server = MockServer::start().await;
    let columns = equity_quote_columns();
    Mock::given(method("POST"))
        .and(path("/global/scan"))
        .respond_with(ResponseTemplate::new(200).set_body_string(encode_response(
            &columns,
            vec![(
                "NASDAQ:AAPL",
                vec![
                    ("name", json!("Apple")),
                    ("market", json!("america")),
                    ("exchange", json!("NASDAQ")),
                    ("currency", json!("USD")),
                    ("country", json!("US")),
                    ("type", json!("stock")),
                    ("open", json!(190.0)),
                    ("high", json!(196.0)),
                    ("low", json!(189.5)),
                    ("close", json!(195.4)),
                    ("change", json!(2.7)),
                    ("change_abs", json!(5.12)),
                    ("volume", json!(1_200_000.0)),
                    ("relative_volume_10d_calc", json!(1.4)),
                    ("market_cap_basic", json!(3_200_000_000_000.0)),
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

    let quote = client.equity().quote("NASDAQ:AAPL").await.unwrap();

    assert_eq!(quote.instrument.ticker.as_str(), "NASDAQ:AAPL");
    assert_eq!(quote.instrument.name.as_deref(), Some("Apple"));
    assert_eq!(quote.close, Some(195.4));
    assert_eq!(quote.change_percent, Some(2.7));
    assert_eq!(quote.market_cap, Some(3_200_000_000_000.0));
}

#[tokio::test]
async fn quote_rejects_scan_rows_for_the_wrong_symbol() {
    let server = MockServer::start().await;
    let columns = equity_quote_columns();
    Mock::given(method("POST"))
        .and(path("/global/scan"))
        .respond_with(ResponseTemplate::new(200).set_body_string(encode_response(
            &columns,
            vec![(
                "NASDAQ:MSFT",
                vec![
                    ("name", json!("Microsoft")),
                    ("market", json!("america")),
                    ("exchange", json!("NASDAQ")),
                    ("currency", json!("USD")),
                    ("country", json!("US")),
                    ("type", json!("stock")),
                    ("close", json!(415.2)),
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

    let error = client.equity().quote("NASDAQ:AAPL").await.unwrap_err();

    assert!(matches!(error, Error::SymbolNotFound { symbol } if symbol == "NASDAQ:AAPL"));
}

#[tokio::test]
async fn quotes_error_when_any_requested_symbol_is_missing() {
    let server = MockServer::start().await;
    let columns = equity_quote_columns();
    Mock::given(method("POST"))
        .and(path("/global/scan"))
        .respond_with(ResponseTemplate::new(200).set_body_string(encode_response(
            &columns,
            vec![(
                "NASDAQ:AAPL",
                vec![
                    ("name", json!("Apple")),
                    ("market", json!("america")),
                    ("exchange", json!("NASDAQ")),
                    ("currency", json!("USD")),
                    ("country", json!("US")),
                    ("type", json!("stock")),
                    ("close", json!(195.4)),
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

    let error = client
        .equity()
        .quotes(["NASDAQ:AAPL", "NASDAQ:MSFT"])
        .await
        .unwrap_err();

    assert!(matches!(error, Error::SymbolNotFound { symbol } if symbol == "NASDAQ:MSFT"));
}

#[tokio::test]
async fn quotes_preserve_requested_order() {
    let server = MockServer::start().await;
    let columns = equity_quote_columns();
    Mock::given(method("POST"))
        .and(path("/global/scan"))
        .respond_with(ResponseTemplate::new(200).set_body_string(encode_response(
            &columns,
            vec![
                (
                    "NASDAQ:AAPL",
                    vec![
                        ("name", json!("Apple")),
                        ("market", json!("america")),
                        ("exchange", json!("NASDAQ")),
                        ("currency", json!("USD")),
                        ("country", json!("US")),
                        ("type", json!("stock")),
                        ("open", json!(190.0)),
                        ("high", json!(196.0)),
                        ("low", json!(189.5)),
                        ("close", json!(195.4)),
                        ("change", json!(2.7)),
                        ("change_abs", json!(5.12)),
                        ("volume", json!(1_200_000.0)),
                        ("relative_volume_10d_calc", json!(1.4)),
                        ("market_cap_basic", json!(3_200_000_000_000.0)),
                    ],
                ),
                (
                    "NASDAQ:MSFT",
                    vec![
                        ("name", json!("Microsoft")),
                        ("market", json!("america")),
                        ("exchange", json!("NASDAQ")),
                        ("currency", json!("USD")),
                        ("country", json!("US")),
                        ("type", json!("stock")),
                        ("open", json!(410.0)),
                        ("high", json!(418.0)),
                        ("low", json!(409.0)),
                        ("close", json!(415.2)),
                        ("change", json!(1.1)),
                        ("change_abs", json!(4.5)),
                        ("volume", json!(900_000.0)),
                        ("relative_volume_10d_calc", json!(1.1)),
                        ("market_cap_basic", json!(3_100_000_000_000.0)),
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

    let quotes = client
        .equity()
        .quotes(["NASDAQ:MSFT", "NASDAQ:AAPL"])
        .await
        .unwrap();

    assert_eq!(quotes[0].instrument.ticker.as_str(), "NASDAQ:MSFT");
    assert_eq!(quotes[1].instrument.ticker.as_str(), "NASDAQ:AAPL");
}

#[tokio::test]
async fn batch_requests_return_empty_for_empty_inputs() {
    let server = MockServer::start().await;
    let client = TradingViewClient::builder()
        .endpoints(
            Endpoints::default()
                .with_scanner_base_url(server.uri())
                .unwrap(),
        )
        .build()
        .unwrap();

    assert!(
        client
            .equity()
            .quotes(Vec::<&str>::new())
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        client
            .equity()
            .fundamentals_batch(Vec::<&str>::new())
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        client
            .equity()
            .estimate_histories(Vec::<&str>::new())
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        client
            .equity()
            .fundamentals_point_in_time_batch(Vec::<&str>::new())
            .await
            .unwrap()
            .is_empty()
    );
}

#[tokio::test]
async fn quotes_preserve_requested_order_with_duplicate_symbols() {
    let server = MockServer::start().await;
    let columns = equity_quote_columns();
    Mock::given(method("POST"))
        .and(path("/global/scan"))
        .respond_with(ResponseTemplate::new(200).set_body_string(encode_response(
            &columns,
            vec![
                (
                    "NASDAQ:AAPL",
                    vec![
                        ("name", json!("Apple")),
                        ("market", json!("america")),
                        ("exchange", json!("NASDAQ")),
                        ("currency", json!("USD")),
                        ("country", json!("US")),
                        ("type", json!("stock")),
                        ("close", json!(195.4)),
                    ],
                ),
                (
                    "NASDAQ:MSFT",
                    vec![
                        ("name", json!("Microsoft")),
                        ("market", json!("america")),
                        ("exchange", json!("NASDAQ")),
                        ("currency", json!("USD")),
                        ("country", json!("US")),
                        ("type", json!("stock")),
                        ("close", json!(415.2)),
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

    let quotes = client
        .equity()
        .quotes(["NASDAQ:AAPL", "NASDAQ:MSFT", "NASDAQ:AAPL"])
        .await
        .unwrap();

    assert_eq!(quotes.len(), 3);
    assert_eq!(quotes[0].instrument.ticker.as_str(), "NASDAQ:AAPL");
    assert_eq!(quotes[1].instrument.ticker.as_str(), "NASDAQ:MSFT");
    assert_eq!(quotes[2].instrument.ticker.as_str(), "NASDAQ:AAPL");
    assert_eq!(quotes[2].close, Some(195.4));
}

#[tokio::test]
async fn quotes_expand_page_to_match_large_batch_size() {
    let server = MockServer::start().await;
    let columns = equity_quote_columns();
    let symbols = (0..60)
        .map(|index| format!("NASDAQ:TST{index:03}"))
        .collect::<Vec<_>>();
    let rows = symbols
        .iter()
        .map(|symbol| {
            (
                symbol.as_str(),
                vec![
                    ("name", json!(symbol)),
                    ("market", json!("america")),
                    ("exchange", json!("NASDAQ")),
                    ("currency", json!("USD")),
                    ("country", json!("US")),
                    ("type", json!("stock")),
                    ("sector", json!("Technology")),
                    ("industry", json!("Software")),
                    ("open", json!(10.0)),
                    ("high", json!(11.0)),
                    ("low", json!(9.5)),
                    ("close", json!(10.5)),
                    ("change", json!(1.0)),
                    ("change_abs", json!(0.1)),
                    ("volume", json!(1_000.0)),
                    ("relative_volume_10d_calc", json!(1.0)),
                    ("market_cap_basic", json!(1_000_000.0)),
                ],
            )
        })
        .collect::<Vec<_>>();

    Mock::given(method("POST"))
        .and(path("/global/scan"))
        .and(body_string_contains("\"range\":[0,60]"))
        .respond_with(ResponseTemplate::new(200).set_body_string(encode_response(&columns, rows)))
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

    let quotes = client.equity().quotes(symbols.clone()).await.unwrap();

    assert_eq!(quotes.len(), 60);
    assert_eq!(quotes[0].instrument.ticker.as_str(), symbols[0]);
    assert_eq!(quotes[59].instrument.ticker.as_str(), symbols[59]);
}

#[tokio::test]
async fn quote_tolerates_partial_rows_and_missing_columns() {
    let server = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/global/scan"))
        .respond_with(
            ResponseTemplate::new(200).set_body_string(
                serde_json::to_string(&json!({
                    "totalCount": 1,
                    "data": [
                        {
                            "s": "NASDAQ:AAPL",
                            "d": ["Apple"]
                        }
                    ]
                }))
                .unwrap(),
            ),
        )
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

    let quote = client.equity().quote("NASDAQ:AAPL").await.unwrap();

    assert_eq!(quote.instrument.name.as_deref(), Some("Apple"));
    assert_eq!(quote.instrument.market, None);
    assert_eq!(quote.close, None);
    assert_eq!(quote.market_cap, None);
}

#[tokio::test]
async fn quote_tolerates_malformed_scalar_values() {
    let server = MockServer::start().await;
    let columns = equity_quote_columns();
    Mock::given(method("POST"))
        .and(path("/global/scan"))
        .respond_with(ResponseTemplate::new(200).set_body_string(encode_response(
            &columns,
            vec![(
                "NASDAQ:AAPL",
                vec![
                    ("name", json!("Apple")),
                    ("market", json!("america")),
                    ("exchange", json!("NASDAQ")),
                    ("currency", json!("USD")),
                    ("country", json!("US")),
                    ("type", json!("stock")),
                    ("close", json!("not-a-number")),
                    ("change", json!({"unexpected": true})),
                    ("volume", json!(false)),
                    ("market_cap_basic", json!("also-bad")),
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

    let quote = client.equity().quote("NASDAQ:AAPL").await.unwrap();

    assert_eq!(quote.instrument.ticker.as_str(), "NASDAQ:AAPL");
    assert_eq!(quote.close, None);
    assert_eq!(quote.change_percent, None);
    assert_eq!(quote.volume, None);
    assert_eq!(quote.market_cap, None);
}

#[tokio::test]
async fn analyst_summary_decodes_rich_payload() {
    let server = MockServer::start().await;
    let columns = analyst_columns();
    Mock::given(method("POST"))
        .and(path("/global/scan"))
        .respond_with(ResponseTemplate::new(200).set_body_string(encode_response(
            &columns,
            vec![(
                "NASDAQ:AAPL",
                vec![
                    ("name", json!("Apple")),
                    ("market", json!("america")),
                    ("exchange", json!("NASDAQ")),
                    ("currency", json!("USD")),
                    ("country", json!("US")),
                    ("type", json!("stock")),
                    ("close", json!(247.98)),
                    ("recommendation_buy", json!(24)),
                    ("recommendation_sell", json!(2)),
                    ("recommendation_hold", json!(15)),
                    ("recommendation_over", json!(11)),
                    ("recommendation_under", json!(0)),
                    ("recommendation_total", json!(52)),
                    ("recommendation_mark", json!(1.471154)),
                    ("price_target_average", json!(298.89875)),
                    ("price_target_high", json!(350.0)),
                    ("price_target_low", json!(205.0)),
                    ("price_target_median", json!(302.5)),
                    ("price_target_1y", json!(298.89875)),
                    ("price_target_1y_delta", json!(20.528549538287844)),
                    ("revenue_forecast_fq", json!(138_391_007_589.0)),
                    ("revenue_forecast_next_fq", json!(108_883_744_747.0)),
                    ("revenue_forecast_next_fh", json!(210_868_433_513.0)),
                    ("revenue_forecast_next_fy", json!(463_998_856_866.0)),
                    ("earnings_per_share_forecast_fq", json!(2.673324)),
                    ("earnings_per_share_forecast_next_fq", json!(1.946418)),
                    ("earnings_per_share_forecast_next_fh", json!(3.675343)),
                    ("earnings_per_share_forecast_next_fy", json!(8.483484)),
                    ("eps_surprise_fq", json!(0.16667599999999982)),
                    ("eps_surprise_percent_fq", json!(6.234784859598006)),
                    (
                        "non_gaap_price_to_earnings_per_share_forecast_next_fy",
                        json!(29.2320937954265),
                    ),
                    ("price_earnings_forward_fy", Value::Null),
                    ("earnings_release_date", json!(1769722200)),
                    ("earnings_release_next_date", json!(1777550400)),
                    ("earnings_release_calendar_date", json!(1767139200)),
                    ("earnings_release_next_calendar_date", json!(1774915200)),
                    ("earnings_release_trading_date_fq", json!(1769644800)),
                    ("earnings_release_next_trading_date_fq", json!(1777507200)),
                    (
                        "earnings_release_next_trading_date_fy",
                        json!(1793232000_u64),
                    ),
                    ("earnings_release_time", json!(1)),
                    ("earnings_release_next_time", json!(0)),
                    ("earnings_publication_type_fq", json!(22)),
                    ("earnings_publication_type_next_fq", json!(10)),
                    (
                        "rates_current",
                        fx_rates(
                            1774051200,
                            &[("to_market", 1.0), ("to_symbol", 1.0), ("to_usd", 1.0)],
                        ),
                    ),
                    (
                        "rates_fq",
                        fx_rates(
                            1767139200,
                            &[("to_market", 1.0), ("to_eur", 0.851419), ("to_usd", 1.0)],
                        ),
                    ),
                    (
                        "rates_fy",
                        fx_rates(
                            1759190400,
                            &[("to_market", 1.0), ("to_jpy", 147.928994), ("to_usd", 1.0)],
                        ),
                    ),
                    (
                        "rates_earnings_next_fq",
                        fx_rates(
                            1774915200,
                            &[("to_market", 1.0), ("to_gbp", 0.744823), ("to_usd", 1.0)],
                        ),
                    ),
                    ("rates_dividend_recent", Value::Null),
                    ("rates_dividend_upcoming", Value::Null),
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

    let analyst = client
        .equity()
        .analyst_summary("NASDAQ:AAPL")
        .await
        .unwrap();

    assert_eq!(analyst.close, Some(247.98));
    assert_eq!(analyst.recommendations.buy, Some(24));
    assert_eq!(analyst.recommendations.total, Some(52));
    assert_eq!(analyst.price_targets.one_year, Some(298.89875));
    assert_eq!(
        analyst.price_targets.one_year_delta_percent,
        Some(20.528549538287844)
    );
    assert_eq!(
        analyst.forecasts.revenue_next_fiscal_year,
        Some(463_998_856_866.0)
    );
    assert_eq!(analyst.forecasts.eps_next_fiscal_year, Some(8.483484));
    assert_eq!(
        analyst.forecasts.eps_surprise_percent_recent_quarter,
        Some(6.234784859598006)
    );
    assert_eq!(
        analyst.earnings.recent_release_at,
        Some(datetime!(2026-01-29 21:30:00 UTC))
    );
    assert_eq!(
        analyst.earnings.next_release_at,
        Some(datetime!(2026-04-30 12:00:00 UTC))
    );
    assert_eq!(
        analyst.earnings.fiscal_year_trading_date,
        Some(datetime!(2026-10-29 00:00:00 UTC))
    );
    assert_eq!(
        analyst.earnings.current_quarter_publication_type_code,
        Some(22)
    );
    assert_eq!(analyst.earnings.next_release_time_code, Some(0));
    assert_eq!(
        analyst
            .fx_rates
            .current
            .as_ref()
            .and_then(|rates| rates.effective_at),
        Some(datetime!(2026-03-21 00:00:00 UTC))
    );
    assert_eq!(
        analyst
            .fx_rates
            .revenue_current_quarter
            .as_ref()
            .and_then(|rates| rates.rates.get("to_eur"))
            .copied(),
        Some(0.851419)
    );
    assert_eq!(
        analyst
            .fx_rates
            .earnings_next_quarter
            .as_ref()
            .and_then(|rates| rates.rates.get("to_gbp"))
            .copied(),
        Some(0.744823)
    );
}

#[tokio::test]
async fn price_targets_use_dedicated_columns() {
    let server = MockServer::start().await;
    let columns = analyst_price_target_columns();
    Mock::given(method("POST"))
        .and(path("/global/scan"))
        .respond_with(ResponseTemplate::new(200).set_body_string(encode_response(
            &columns,
            vec![(
                "NASDAQ:AAPL",
                vec![
                    ("price_target_average", json!(298.89875)),
                    ("price_target_high", json!(350.0)),
                    ("price_target_low", json!(205.0)),
                    ("price_target_median", json!(302.5)),
                    ("price_target_1y", json!(298.89875)),
                    ("price_target_1y_delta", json!(20.528549538287844)),
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

    let targets = client.equity().price_targets("NASDAQ:AAPL").await.unwrap();

    assert_eq!(targets.average, Some(298.89875));
    assert_eq!(targets.high, Some(350.0));
    assert_eq!(targets.low, Some(205.0));
    assert_eq!(targets.one_year_delta_percent, Some(20.528549538287844));
}

#[tokio::test]
async fn earnings_calendar_uses_dedicated_columns() {
    let server = MockServer::start().await;
    let columns = earnings_calendar_columns();
    Mock::given(method("POST"))
        .and(path("/global/scan"))
        .respond_with(ResponseTemplate::new(200).set_body_string(encode_response(
            &columns,
            vec![(
                "NASDAQ:AAPL",
                vec![
                    ("earnings_release_date", json!(1769722200)),
                    ("earnings_release_next_date", json!(1777550400)),
                    ("earnings_release_calendar_date", json!(1767139200)),
                    ("earnings_release_next_calendar_date", json!(1774915200)),
                    ("earnings_release_trading_date_fq", json!(1769644800)),
                    ("earnings_release_next_trading_date_fq", json!(1777507200)),
                    (
                        "earnings_release_next_trading_date_fy",
                        json!(1793232000_u64),
                    ),
                    ("earnings_release_time", json!(1)),
                    ("earnings_release_next_time", json!(0)),
                    ("earnings_publication_type_fq", json!(22)),
                    ("earnings_publication_type_next_fq", json!(10)),
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

    let earnings = client
        .equity()
        .earnings_calendar("NASDAQ:AAPL")
        .await
        .unwrap();

    assert_eq!(
        earnings.recent_release_at,
        Some(datetime!(2026-01-29 21:30:00 UTC))
    );
    assert_eq!(
        earnings.next_release_at,
        Some(datetime!(2026-04-30 12:00:00 UTC))
    );
    assert_eq!(
        earnings.fiscal_year_trading_date,
        Some(datetime!(2026-10-29 00:00:00 UTC))
    );
    assert_eq!(earnings.current_quarter_publication_type_code, Some(22));
    assert_eq!(earnings.next_release_time_code, Some(0));
}

#[tokio::test]
async fn overview_maps_all_sections() {
    let server = MockServer::start().await;
    let columns = overview_columns();
    Mock::given(method("POST"))
        .and(path("/global/scan"))
        .respond_with(ResponseTemplate::new(200).set_body_string(encode_response(
            &columns,
            vec![(
                "NASDAQ:AAPL",
                vec![
                    ("name", json!("Apple")),
                    ("market", json!("america")),
                    ("exchange", json!("NASDAQ")),
                    ("currency", json!("USD")),
                    ("country", json!("US")),
                    ("type", json!("stock")),
                    ("sector", json!("Technology")),
                    ("industry", json!("Consumer Electronics")),
                    ("open", json!(190.0)),
                    ("high", json!(193.0)),
                    ("low", json!(188.0)),
                    ("close", json!(195.4)),
                    ("change", json!(2.7)),
                    ("change_abs", json!(5.12)),
                    ("volume", json!(1_200_000.0)),
                    ("relative_volume_10d_calc", json!(1.4)),
                    ("market_cap_basic", json!(3_200_000_000_000.0)),
                    ("price_earnings_ttm", json!(28.5)),
                    ("price_book_fq", json!(35.2)),
                    ("price_sales_current", json!(7.8)),
                    ("total_revenue", json!(400_000_000_000.0)),
                    ("net_income", json!(95_000_000_000.0)),
                    ("earnings_per_share_basic_ttm", json!(6.43)),
                    ("dividend_yield_recent", json!(0.52)),
                    ("return_on_equity", json!(175.0)),
                    ("return_on_assets", json!(28.0)),
                    ("debt_to_equity", json!(120.0)),
                    ("current_ratio", json!(0.95)),
                    ("free_cash_flow", json!(120_000_000_000.0)),
                    ("ebitda", json!(140_000_000_000.0)),
                    ("recommendation_buy", json!(24)),
                    ("recommendation_sell", json!(3)),
                    ("recommendation_hold", json!(7)),
                    ("recommendation_over", json!(2)),
                    ("recommendation_under", json!(1)),
                    ("recommendation_total", json!(35)),
                    ("recommendation_mark", json!(1.8)),
                    ("price_target_average", json!(210.0)),
                    ("price_target_high", json!(230.0)),
                    ("price_target_low", json!(180.0)),
                    ("price_target_median", json!(205.0)),
                    ("revenue_forecast_next_fq", json!(42_000_000_000.0)),
                    ("earnings_per_share_forecast_next_fq", json!(1.9)),
                    ("Recommend.All", json!(0.62)),
                    ("Recommend.MA", json!(0.55)),
                    ("Recommend.Other", json!(0.71)),
                    ("RSI", json!(57.0)),
                    ("RSI7", json!(61.0)),
                    ("MACD.macd", json!(1.7)),
                    ("MACD.signal", json!(1.5)),
                    ("MACD.hist", json!(0.2)),
                    ("ADX", json!(25.0)),
                    ("ATR", json!(4.0)),
                    ("SMA20", json!(185.0)),
                    ("SMA50", json!(190.0)),
                    ("SMA200", json!(172.0)),
                    ("EMA20", json!(186.0)),
                    ("EMA50", json!(181.0)),
                    ("EMA200", json!(170.0)),
                    ("Stoch.K", json!(82.0)),
                    ("Stoch.D", json!(78.0)),
                    ("W.R", json!(-18.0)),
                    ("CCI20", json!(110.0)),
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

    let overview = client.equity().overview("NASDAQ:AAPL").await.unwrap();

    assert_eq!(
        overview.quote.instrument.sector.as_deref(),
        Some("Technology")
    );
    assert_eq!(overview.fundamentals.price_earnings_ttm, Some(28.5));
    assert_eq!(overview.analyst.recommendations.buy, Some(24));
    assert_eq!(overview.technicals.rsi, Some(57.0));
}

#[tokio::test]
async fn top_gainers_use_market_route() {
    let server = MockServer::start().await;
    let columns = quote_columns();
    Mock::given(method("POST"))
        .and(path("/america/scan"))
        .respond_with(ResponseTemplate::new(200).set_body_string(encode_response(
            &columns,
            vec![
                (
                    "NASDAQ:NVDA",
                    vec![
                        ("name", json!("NVIDIA")),
                        ("market", json!("america")),
                        ("exchange", json!("NASDAQ")),
                        ("currency", json!("USD")),
                        ("country", json!("US")),
                        ("type", json!("stock")),
                        ("open", json!(120.0)),
                        ("high", json!(132.0)),
                        ("low", json!(119.0)),
                        ("close", json!(132.0)),
                        ("change", json!(10.0)),
                        ("change_abs", json!(12.0)),
                        ("volume", json!(2_500_000.0)),
                        ("relative_volume_10d_calc", json!(2.3)),
                        ("market_cap_basic", json!(2_900_000_000_000.0)),
                    ],
                ),
                (
                    "NASDAQ:AMD",
                    vec![
                        ("name", json!("AMD")),
                        ("market", json!("america")),
                        ("exchange", json!("NASDAQ")),
                        ("currency", json!("USD")),
                        ("country", json!("US")),
                        ("type", json!("stock")),
                        ("open", json!(150.0)),
                        ("high", json!(158.0)),
                        ("low", json!(148.0)),
                        ("close", json!(158.0)),
                        ("change", json!(4.6)),
                        ("change_abs", json!(7.0)),
                        ("volume", json!(1_800_000.0)),
                        ("relative_volume_10d_calc", json!(1.8)),
                        ("market_cap_basic", json!(260_000_000_000.0)),
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

    let movers = client.equity().top_gainers("america", 2).await.unwrap();
    let requests = server.received_requests().await.unwrap();
    let body = String::from_utf8_lossy(&requests[0].body);

    assert_eq!(movers.len(), 2);
    assert_eq!(movers[0].instrument.ticker.as_str(), "NASDAQ:NVDA");
    assert_eq!(movers[0].change_percent, Some(10.0));
    assert!(!body.contains(r#""left":"volume""#));
}

#[tokio::test]
async fn most_active_requires_positive_volume() {
    let server = MockServer::start().await;
    let columns = quote_columns();
    Mock::given(method("POST"))
        .and(path("/america/scan"))
        .respond_with(ResponseTemplate::new(200).set_body_string(encode_response(
            &columns,
            vec![(
                "NASDAQ:NVDA",
                vec![
                    ("name", json!("NVIDIA")),
                    ("market", json!("america")),
                    ("exchange", json!("NASDAQ")),
                    ("currency", json!("USD")),
                    ("country", json!("US")),
                    ("type", json!("stock")),
                    ("close", json!(132.0)),
                    ("change", json!(10.0)),
                    ("change_abs", json!(12.0)),
                    ("volume", json!(2_500_000.0)),
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

    let movers = client.equity().most_active("america", 1).await.unwrap();
    let requests = server.received_requests().await.unwrap();
    let body = String::from_utf8_lossy(&requests[0].body);

    assert_eq!(movers.len(), 1);
    assert!(body.contains(r#""left":"volume""#));
}
