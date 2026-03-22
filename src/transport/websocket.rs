use std::sync::atomic::{AtomicU64, Ordering};

use futures_util::SinkExt;
use serde_json::{Value, json};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::{MaybeTlsStream, WebSocketStream, connect_async, tungstenite::Message};
#[cfg(feature = "tracing")]
use tracing::{debug, warn};

use crate::client::Endpoints;
use crate::error::{Error, Result};

pub(crate) type TradingViewWebSocket = WebSocketStream<MaybeTlsStream<TcpStream>>;

pub(crate) async fn connect_socket(
    endpoints: &Endpoints,
    user_agent: &str,
    session_id: Option<&str>,
) -> Result<TradingViewWebSocket> {
    #[cfg(feature = "tracing")]
    debug!(
        target: "tvdata_rs::transport",
        url = endpoints.websocket_url().as_str(),
        authenticated = session_id.is_some(),
        "opening TradingView websocket",
    );

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

    match connect_async(ws_request).await {
        Ok((socket, _)) => {
            #[cfg(feature = "tracing")]
            debug!(
                target: "tvdata_rs::transport",
                url = endpoints.websocket_url().as_str(),
                authenticated = session_id.is_some(),
                "TradingView websocket connected",
            );
            Ok(socket)
        }
        Err(error) => {
            #[cfg(feature = "tracing")]
            warn!(
                target: "tvdata_rs::transport",
                url = endpoints.websocket_url().as_str(),
                authenticated = session_id.is_some(),
                error = %error,
                "TradingView websocket connection failed",
            );
            Err(Error::from(error))
        }
    }
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

    use tokio::io::{AsyncReadExt, AsyncWriteExt};
    use tokio::net::TcpListener;
    use tokio_tungstenite::tungstenite::handshake::derive_accept_key;

    use crate::client::Endpoints;

    use super::*;

    async fn read_upgrade_request(stream: &mut TcpStream) -> String {
        let mut request = Vec::new();
        let mut buffer = [0_u8; 1024];

        loop {
            let read = stream.read(&mut buffer).await.unwrap();
            assert_ne!(read, 0, "client closed connection before websocket upgrade");
            request.extend_from_slice(&buffer[..read]);
            if request.windows(4).any(|window| window == b"\r\n\r\n") {
                break;
            }
        }

        String::from_utf8(request).unwrap()
    }

    fn header_value<'a>(request: &'a str, name: &str) -> Option<&'a str> {
        request.lines().find_map(|line| {
            let (header, value) = line.split_once(':')?;
            header
                .trim()
                .eq_ignore_ascii_case(name)
                .then_some(value.trim())
        })
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
            let (mut stream, _) = listener.accept().await.unwrap();
            let request = read_upgrade_request(&mut stream).await;
            *cookie_clone.lock().unwrap() = header_value(&request, "cookie").map(str::to_owned);

            let key = header_value(&request, "sec-websocket-key")
                .expect("websocket upgrade request must contain Sec-WebSocket-Key");
            let response = format!(
                "HTTP/1.1 101 Switching Protocols\r\n\
                 Connection: Upgrade\r\n\
                 Upgrade: websocket\r\n\
                 Sec-WebSocket-Accept: {}\r\n\
                 \r\n",
                derive_accept_key(key.as_bytes())
            );

            stream.write_all(response.as_bytes()).await.unwrap();
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
