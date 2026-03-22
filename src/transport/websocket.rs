use std::sync::atomic::{AtomicU64, Ordering};

use futures_util::SinkExt;
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};

use crate::client::Endpoints;
use crate::error::{Error, Result};

pub(crate) type TradingViewWebSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub(crate) async fn connect_socket(
    endpoints: &Endpoints,
    user_agent: &str,
    session_id: Option<&str>,
) -> Result<TradingViewWebSocket> {
    let mut ws_request = endpoints
        .websocket_url()
        .as_str()
        .into_client_request()
        .map_err(Error::from)?;
    ws_request.headers_mut().insert(
        "Origin",
        endpoints
            .data_origin()
            .as_str()
            .parse()
            .map_err(|_| Error::Protocol("failed to encode websocket origin header"))?,
    );
    ws_request.headers_mut().insert(
        "User-Agent",
        user_agent
            .parse()
            .map_err(|_| Error::Protocol("failed to encode websocket user agent header"))?,
    );
    if let Some(session_id) = session_id {
        ws_request.headers_mut().insert(
            "Cookie",
            format!("sessionid={session_id}")
                .parse()
                .map_err(|_| Error::Protocol("failed to encode websocket cookie header"))?,
        );
    }

    let (socket, _) = connect_async(ws_request).await?;
    Ok(socket)
}

pub(crate) fn next_session_id(prefix: &str) -> String {
    static COUNTER: AtomicU64 = AtomicU64::new(1);
    let id = COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("{prefix}_{id:016x}")
}

pub(crate) async fn send_message(
    socket: &mut TradingViewWebSocket,
    method: &str,
    params: Value,
) -> Result<()> {
    let payload = serde_json::to_string(&json!({ "m": method, "p": params }))?;
    send_raw_frame(socket, payload).await
}

pub(crate) async fn send_raw_frame(
    socket: &mut TradingViewWebSocket,
    payload: String,
) -> Result<()> {
    let framed = format!("~m~{}~m~{payload}", payload.len());
    socket.send(Message::Text(framed.into())).await?;
    Ok(())
}

pub(crate) fn parse_framed_messages(frame: &str) -> Result<Vec<&str>> {
    let mut rest = frame;
    let mut payloads = Vec::new();

    while !rest.is_empty() {
        if let Some(next) = rest.strip_prefix("~m~") {
            let Some((len, tail)) = next.split_once("~m~") else {
                return Err(Error::Protocol("missing websocket frame length separator"));
            };
            let len: usize = len
                .parse()
                .map_err(|_| Error::Protocol("invalid websocket frame length"))?;
            if tail.len() < len {
                return Err(Error::Protocol(
                    "declared websocket frame length exceeds payload",
                ));
            }
            let (payload, remainder) = tail.split_at(len);
            payloads.push(payload);
            rest = remainder;
            continue;
        }

        if let Some((_, remainder)) = rest.split_once("~m~") {
            rest = remainder;
            continue;
        }

        return Err(Error::Protocol("unexpected websocket frame prefix"));
    }

    Ok(payloads)
}

#[cfg(test)]
mod tests {
    use std::sync::{Arc, Mutex};

    use tokio::net::TcpListener;
    use tokio_tungstenite::accept_hdr_async;
    use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};

    use crate::client::Endpoints;

    use super::*;

    #[allow(clippy::result_large_err)]
    fn capture_cookie_callback(
        cookie: Arc<Mutex<Option<String>>>,
    ) -> impl FnOnce(
        &Request,
        Response,
    ) -> std::result::Result<
        Response,
        tokio_tungstenite::tungstenite::http::Response<Option<String>>,
    > {
        move |request: &Request, response: Response| {
            *cookie.lock().unwrap() = request
                .headers()
                .get("cookie")
                .and_then(|value| value.to_str().ok())
                .map(str::to_owned);
            Ok(response)
        }
    }

    #[test]
    fn parses_concatenated_websocket_frames() {
        let frames = parse_framed_messages("~m~9~m~{\"m\":\"a\"}~m~9~m~{\"m\":\"b\"}").unwrap();

        assert_eq!(frames, vec![r#"{"m":"a"}"#, r#"{"m":"b"}"#]);
    }

    #[tokio::test]
    async fn connect_socket_includes_session_cookie_when_configured() {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let address = listener.local_addr().unwrap();
        let cookie = Arc::new(Mutex::new(None::<String>));
        let cookie_clone = Arc::clone(&cookie);

        tokio::spawn(async move {
            let (stream, _) = listener.accept().await.unwrap();
            let mut socket = accept_hdr_async(stream, capture_cookie_callback(cookie_clone))
                .await
                .unwrap();
            let _ = socket.close(None).await;
        });

        let endpoints = Endpoints::default()
            .with_websocket_url(format!("ws://{address}"))
            .unwrap();

        let _socket = connect_socket(&endpoints, "tvdata-rs/test", Some("abc123"))
            .await
            .unwrap();

        assert_eq!(cookie.lock().unwrap().as_deref(), Some("sessionid=abc123"));
    }
}
