use std::collections::BTreeMap;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use time::OffsetDateTime;
use tokio::time::timeout;
use tokio_tungstenite::tungstenite::Message;

use crate::client::TradingViewClient;
use crate::error::{Error, Result};
use crate::scanner::Column;
use crate::scanner::Ticker;
use crate::transport::websocket::{
    next_session_id, parse_framed_messages, send_message, send_raw_frame,
};

const QUOTE_SESSION_TIMEOUT: Duration = Duration::from_secs(30);

#[derive(Debug, Clone, Default)]
pub(crate) struct QuoteFieldValues {
    values: BTreeMap<String, Value>,
}

impl QuoteFieldValues {
    pub(crate) fn from_values(values: BTreeMap<String, Value>) -> Self {
        Self { values }
    }

    pub(crate) fn get(&self, field: &str) -> Option<&Value> {
        self.values.get(field)
    }

    pub(crate) fn string_series(&self, field: &str) -> Vec<Option<String>> {
        self.array(field)
            .map(|items| {
                items
                    .iter()
                    .map(|value| match value {
                        Value::String(value) => Some(value.clone()),
                        Value::Number(value) => Some(value.to_string()),
                        Value::Bool(value) => Some(value.to_string()),
                        _ => None,
                    })
                    .collect()
            })
            .unwrap_or_default()
    }

    pub(crate) fn number_series(&self, field: &str) -> Vec<Option<f64>> {
        self.array(field)
            .map(|items| items.iter().map(number_value).collect())
            .unwrap_or_default()
    }

    pub(crate) fn timestamp_series(&self, field: &str) -> Vec<Option<OffsetDateTime>> {
        self.array(field)
            .map(|items| items.iter().map(timestamp_value).collect())
            .unwrap_or_default()
    }

    fn array(&self, field: &str) -> Option<&Vec<Value>> {
        self.get(field).and_then(Value::as_array)
    }
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct QuoteSessionClient<'a> {
    client: &'a TradingViewClient,
}

impl<'a> QuoteSessionClient<'a> {
    pub(crate) const fn new(client: &'a TradingViewClient) -> Self {
        Self { client }
    }

    pub(crate) async fn fetch_fields(
        &self,
        symbol: &Ticker,
        fields: &[Column],
    ) -> Result<QuoteFieldValues> {
        let _websocket_budget = self.client.acquire_websocket_slot().await?;
        let mut socket = self.client.connect_socket().await?;
        let quote_session = next_session_id("qs");
        let requested_symbol = symbol.as_str().to_owned();
        let set_fields_args = quote_set_fields_payload(quote_session.as_str(), fields);

        send_message(
            &mut socket,
            "set_auth_token",
            json!([self.client.auth_token()]),
        )
        .await?;
        send_message(
            &mut socket,
            "quote_create_session",
            json!([quote_session.as_str()]),
        )
        .await?;
        send_message(&mut socket, "quote_set_fields", set_fields_args).await?;
        send_message(
            &mut socket,
            "quote_add_symbols",
            json!([quote_session.as_str(), requested_symbol.as_str()]),
        )
        .await?;

        let mut state = QuoteSymbolState::default();

        timeout(QUOTE_SESSION_TIMEOUT, async {
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

                            match envelope.get("m").and_then(Value::as_str).unwrap_or_default() {
                                "qsd" => {
                                    merge_quote_symbol_state(
                                        &requested_symbol,
                                        &mut state,
                                        &envelope,
                                    )?;
                                }
                                "quote_completed" => {
                                    if completed_symbol(&envelope) == Some(requested_symbol.as_str())
                                    {
                                        if matches!(state.status.as_deref(), Some(status) if status != "ok")
                                        {
                                            return Err(Error::QuoteSymbolFailed {
                                                symbol: requested_symbol.clone(),
                                                status: state.status.unwrap_or_default(),
                                            });
                                        }

                                        if state.values.is_empty() {
                                            return Err(Error::QuoteEmpty {
                                                symbol: requested_symbol.clone(),
                                            });
                                        }

                                        return Ok(QuoteFieldValues::from_values(state.values));
                                    }
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

            Err(Error::QuoteEmpty {
                symbol: requested_symbol,
            })
        })
        .await
        .map_err(|_| Error::Protocol("quote session timed out"))?
    }
}

fn quote_set_fields_payload(session: &str, fields: &[Column]) -> Value {
    let mut args = vec![Value::String(session.to_owned())];
    args.extend(
        fields
            .iter()
            .map(|field| Value::String(field.as_str().to_owned())),
    );
    Value::Array(args)
}

#[derive(Debug, Default)]
struct QuoteSymbolState {
    status: Option<String>,
    values: BTreeMap<String, Value>,
}

fn merge_quote_symbol_state(
    requested_symbol: &str,
    state: &mut QuoteSymbolState,
    envelope: &Value,
) -> Result<()> {
    let payload = envelope
        .get("p")
        .and_then(Value::as_array)
        .and_then(|parts| parts.get(1))
        .ok_or(Error::Protocol("qsd payload missing symbol data"))?;

    let symbol_data = payload
        .as_object()
        .ok_or(Error::Protocol("qsd symbol payload is not an object"))?;

    let Some(symbol_name) = symbol_data.get("n").and_then(Value::as_str) else {
        return Err(Error::Protocol("qsd symbol payload missing symbol name"));
    };

    if symbol_name != requested_symbol {
        return Ok(());
    }

    state.status = symbol_data
        .get("s")
        .and_then(Value::as_str)
        .map(str::to_owned);

    if let Some(values) = symbol_data.get("v").and_then(Value::as_object) {
        state.values.extend(values.clone());
    }

    Ok(())
}

fn completed_symbol(envelope: &Value) -> Option<&str> {
    envelope
        .get("p")
        .and_then(Value::as_array)
        .and_then(|parts| parts.get(1))
        .and_then(Value::as_str)
}

fn number_value(value: &Value) -> Option<f64> {
    match value {
        Value::Number(number) => number.as_f64(),
        Value::String(value) => value.parse::<f64>().ok(),
        _ => None,
    }
}

fn timestamp_value(value: &Value) -> Option<OffsetDateTime> {
    let timestamp = match value {
        Value::Number(number) => number.as_i64().or_else(|| {
            number
                .as_f64()
                .filter(|timestamp| timestamp.fract() == 0.0)
                .map(|timestamp| timestamp as i64)
        }),
        Value::String(value) => value.parse::<i64>().ok(),
        _ => None,
    }?;

    OffsetDateTime::from_unix_timestamp(timestamp).ok()
}

#[cfg(test)]
mod tests {
    use futures_util::{SinkExt, StreamExt};
    use serde_json::json;
    use time::macros::datetime;
    use tokio::net::TcpListener;
    use tokio_tungstenite::{accept_async, tungstenite::Message};

    use super::*;
    use crate::client::Endpoints;
    use crate::error::Error;

    #[test]
    fn quote_series_decoder_reads_strings_numbers_and_timestamps() {
        let values = QuoteFieldValues::from_values(BTreeMap::from([
            ("periods".to_owned(), json!(["2025-Q4", "2025-Q3", null])),
            ("numbers".to_owned(), json!([10.5, "11.25", null])),
            ("dates".to_owned(), json!([1761856320, "1730406900", null])),
        ]));

        assert_eq!(
            values.string_series("periods"),
            vec![Some("2025-Q4".to_owned()), Some("2025-Q3".to_owned()), None]
        );
        assert_eq!(
            values.number_series("numbers"),
            vec![Some(10.5), Some(11.25), None]
        );
        assert_eq!(
            values.timestamp_series("dates"),
            vec![
                Some(datetime!(2025-10-30 20:32:00 UTC)),
                Some(datetime!(2024-10-31 20:35:00 UTC)),
                None,
            ]
        );
    }

    #[test]
    fn merge_quote_symbol_state_accumulates_partial_qsd_payloads() {
        let mut state = QuoteSymbolState::default();

        merge_quote_symbol_state(
            "NASDAQ:AAPL",
            &mut state,
            &json!({
                "m": "qsd",
                "p": [
                    "qs_1",
                    {
                        "n": "NASDAQ:AAPL",
                        "s": "ok",
                        "v": {
                            "field_a": [1, 2]
                        }
                    }
                ]
            }),
        )
        .unwrap();
        merge_quote_symbol_state(
            "NASDAQ:AAPL",
            &mut state,
            &json!({
                "m": "qsd",
                "p": [
                    "qs_1",
                    {
                        "n": "NASDAQ:AAPL",
                        "s": "ok",
                        "v": {
                            "field_b": ["x", "y"]
                        }
                    }
                ]
            }),
        )
        .unwrap();

        assert_eq!(state.status.as_deref(), Some("ok"));
        assert_eq!(state.values.len(), 2);
        assert!(state.values.contains_key("field_a"));
        assert!(state.values.contains_key("field_b"));
    }

    #[test]
    fn merge_quote_symbol_state_rejects_missing_symbol_name() {
        let mut state = QuoteSymbolState::default();
        let error = merge_quote_symbol_state(
            "NASDAQ:AAPL",
            &mut state,
            &json!({
                "m": "qsd",
                "p": [
                    "qs_1",
                    {
                        "s": "ok",
                        "v": {
                            "close": 247.98
                        }
                    }
                ]
            }),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            Error::Protocol("qsd symbol payload missing symbol name")
        ));
    }

    #[tokio::test]
    async fn fetch_fields_returns_quote_empty_when_socket_closes_before_completion() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let server = tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_async(stream).await.unwrap();

            for _ in 0..4 {
                match socket.next().await {
                    Some(Ok(Message::Text(_))) => {}
                    other => panic!("unexpected websocket client message: {other:?}"),
                }
            }

            let payload = serde_json::to_string(&json!({
                "m": "qsd",
                "p": [
                    "qs_1",
                    {
                        "n": "NASDAQ:AAPL",
                        "s": "ok",
                        "v": {
                            "close": [247.98]
                        }
                    }
                ]
            }))
            .unwrap();
            let frame = format!("~m~{}~m~{payload}", payload.len());
            socket.send(Message::Text(frame.into())).await.unwrap();
            socket.close(None).await.unwrap();
        });

        let client = TradingViewClient::builder()
            .endpoints(
                Endpoints::default()
                    .with_websocket_url(format!("ws://{address}"))
                    .unwrap(),
            )
            .build()
            .unwrap();

        let error = QuoteSessionClient::new(&client)
            .fetch_fields(&Ticker::new("NASDAQ:AAPL"), &[Column::from_static("close")])
            .await
            .unwrap_err();

        assert!(matches!(error, Error::QuoteEmpty { symbol } if symbol == "NASDAQ:AAPL"));
        server.await.unwrap();
    }

    #[test]
    fn quote_set_fields_payload_is_flattened() {
        let payload = quote_set_fields_payload(
            "qs_1",
            &[Column::from_static("close"), Column::from_static("volume")],
        );

        assert_eq!(payload, json!(["qs_1", "close", "volume"]));
    }
}
