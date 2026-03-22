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
}
