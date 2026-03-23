use std::time::Duration;

use reqwest::StatusCode;
use thiserror::Error;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorKind {
    RateLimited,
    AuthRequired,
    SymbolNotFound,
    Transport,
    Protocol,
    Unsupported,
    Api,
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("http request failed: {0}")]
    Http(#[source] Box<reqwest_middleware::Error>),

    #[error("websocket request failed: {0}")]
    WebSocket(#[source] Box<tokio_tungstenite::tungstenite::Error>),

    #[error("failed to deserialize tradingview payload: {0}")]
    Json(#[from] serde_json::Error),

    #[error("failed to format time value: {0}")]
    TimeFormat(#[from] time::error::Format),

    #[error("invalid endpoint url: {0}")]
    UrlParse(#[from] url::ParseError),

    #[error("tradingview returned an API error: {0}")]
    ApiMessage(String),

    #[error("tradingview returned HTTP {status}: {body}")]
    ApiStatus { status: StatusCode, body: String },

    #[error("search query cannot be empty")]
    EmptySearchQuery,

    #[error("scan page limit must be greater than zero")]
    InvalidPageLimit,

    #[error("history request returned no bars for {symbol}")]
    HistoryEmpty { symbol: String },

    #[error("scan returned no rows for {symbol}")]
    SymbolNotFound { symbol: String },

    #[error("scan validation is unavailable: {reason}")]
    ScanValidationUnavailable { reason: String },

    #[error("scan query uses fields unsupported for {route}: {fields:?}")]
    UnsupportedScanFields { route: String, fields: Vec<String> },

    #[error("quote session returned no data for {symbol}")]
    QuoteEmpty { symbol: String },

    #[error("quote session returned status {status} for {symbol}")]
    QuoteSymbolFailed { symbol: String, status: String },

    #[error("history batch concurrency must be greater than zero")]
    InvalidBatchConcurrency,

    #[error("history pagination exceeded safe limit for {symbol} after {rounds} rounds")]
    HistoryPaginationLimitExceeded { symbol: String, rounds: usize },

    #[error("history download failed for {symbol}: {source}")]
    HistoryDownloadFailed {
        symbol: String,
        #[source]
        source: Box<Error>,
    },

    #[error("retry min interval {min:?} cannot exceed max interval {max:?}")]
    InvalidRetryBounds { min: Duration, max: Duration },

    #[error("request budget field {field} must be greater than zero")]
    InvalidRequestBudget { field: &'static str },

    #[error("snapshot batch config field {field} must be greater than zero")]
    InvalidSnapshotBatchConfig { field: &'static str },

    #[error("invalid websocket frame: {0}")]
    Protocol(&'static str),
}

impl From<reqwest_middleware::Error> for Error {
    fn from(value: reqwest_middleware::Error) -> Self {
        Self::Http(Box::new(value))
    }
}

impl From<reqwest::Error> for Error {
    fn from(value: reqwest::Error) -> Self {
        let error: reqwest_middleware::Error = value.into();
        Self::Http(Box::new(error))
    }
}

impl From<tokio_tungstenite::tungstenite::Error> for Error {
    fn from(value: tokio_tungstenite::tungstenite::Error) -> Self {
        Self::WebSocket(Box::new(value))
    }
}

impl Error {
    pub fn kind(&self) -> ErrorKind {
        match self {
            Self::Http(_) | Self::WebSocket(_) => ErrorKind::Transport,
            Self::Json(_)
            | Self::TimeFormat(_)
            | Self::UrlParse(_)
            | Self::InvalidPageLimit
            | Self::InvalidBatchConcurrency
            | Self::InvalidRetryBounds { .. }
            | Self::InvalidRequestBudget { .. }
            | Self::InvalidSnapshotBatchConfig { .. }
            | Self::Protocol(_) => ErrorKind::Protocol,
            Self::EmptySearchQuery | Self::ApiMessage(_) => ErrorKind::Api,
            Self::ApiStatus { status, .. } if *status == StatusCode::TOO_MANY_REQUESTS => {
                ErrorKind::RateLimited
            }
            Self::ApiStatus { status, .. }
                if *status == StatusCode::UNAUTHORIZED || *status == StatusCode::FORBIDDEN =>
            {
                ErrorKind::AuthRequired
            }
            Self::ApiStatus { .. } => ErrorKind::Api,
            Self::HistoryEmpty { .. }
            | Self::SymbolNotFound { .. }
            | Self::QuoteEmpty { .. }
            | Self::QuoteSymbolFailed { .. } => ErrorKind::SymbolNotFound,
            Self::ScanValidationUnavailable { .. } | Self::UnsupportedScanFields { .. } => {
                ErrorKind::Unsupported
            }
            Self::HistoryPaginationLimitExceeded { .. } => ErrorKind::Protocol,
            Self::HistoryDownloadFailed { source, .. } => source.kind(),
        }
    }

    pub fn is_retryable(&self) -> bool {
        match self.kind() {
            ErrorKind::RateLimited | ErrorKind::Transport => true,
            ErrorKind::AuthRequired
            | ErrorKind::SymbolNotFound
            | ErrorKind::Protocol
            | ErrorKind::Unsupported => false,
            ErrorKind::Api => matches!(
                self,
                Self::ApiStatus { status, .. } if status.is_server_error()
            ),
        }
    }

    pub fn is_auth_error(&self) -> bool {
        self.kind() == ErrorKind::AuthRequired
    }

    pub fn is_rate_limited(&self) -> bool {
        self.kind() == ErrorKind::RateLimited
    }

    pub fn is_symbol_error(&self) -> bool {
        self.kind() == ErrorKind::SymbolNotFound
    }

    pub fn is_transport_error(&self) -> bool {
        self.kind() == ErrorKind::Transport
    }

    pub fn is_protocol_error(&self) -> bool {
        self.kind() == ErrorKind::Protocol
    }

    pub fn is_unsupported_error(&self) -> bool {
        self.kind() == ErrorKind::Unsupported
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn helper_methods_follow_error_kind_classification() {
        let auth = Error::ApiStatus {
            status: StatusCode::UNAUTHORIZED,
            body: String::from("unauthorized"),
        };
        assert!(auth.is_auth_error());
        assert!(!auth.is_retryable());

        let rate_limited = Error::ApiStatus {
            status: StatusCode::TOO_MANY_REQUESTS,
            body: String::from("rate limited"),
        };
        assert!(rate_limited.is_rate_limited());
        assert!(rate_limited.is_retryable());

        let symbol = Error::SymbolNotFound {
            symbol: String::from("NASDAQ:AAPL"),
        };
        assert!(symbol.is_symbol_error());
        assert!(!symbol.is_retryable());

        let transport = Error::ApiStatus {
            status: StatusCode::BAD_GATEWAY,
            body: String::from("upstream failed"),
        };
        assert_eq!(transport.kind(), ErrorKind::Api);
        assert!(!transport.is_transport_error());
        assert!(transport.is_retryable());

        let protocol = Error::Protocol("bad frame");
        assert!(protocol.is_protocol_error());
        assert!(!protocol.is_retryable());

        let unsupported = Error::UnsupportedScanFields {
            route: String::from("america/scan"),
            fields: vec![String::from("bad_field")],
        };
        assert!(unsupported.is_unsupported_error());
        assert!(!unsupported.is_retryable());
    }

    #[test]
    fn wrapped_history_download_failures_preserve_helper_behavior() {
        let wrapped = Error::HistoryDownloadFailed {
            symbol: String::from("NASDAQ:AAPL"),
            source: Box::new(Error::ApiStatus {
                status: StatusCode::TOO_MANY_REQUESTS,
                body: String::from("rate limited"),
            }),
        };

        assert!(wrapped.is_rate_limited());
        assert!(wrapped.is_retryable());
    }
}
