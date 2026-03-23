use std::collections::{HashMap, HashSet};
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::time::{Duration, Instant};

use bon::{Builder, bon};
use reqwest::header::{COOKIE, HeaderValue, ORIGIN, REFERER, USER_AGENT};
use reqwest_middleware::{
    ClientBuilder as MiddlewareClientBuilder, ClientWithMiddleware, RequestBuilder,
};
use reqwest_retry::{Jitter, RetryTransientMiddleware, policies::ExponentialBackoff};
use serde::de::DeserializeOwned;
use tokio::sync::{Mutex, OwnedSemaphorePermit, RwLock, Semaphore};
use tokio::time::{Instant as TokioInstant, sleep_until};
#[cfg(feature = "tracing")]
use tracing::{debug, warn};
use url::Url;

#[cfg(feature = "calendar")]
use crate::calendar::{
    CalendarWindowRequest, DividendCalendarEntry, DividendCalendarRequest, EarningsCalendarEntry,
    IpoCalendarEntry,
};
#[cfg(feature = "economics")]
use crate::economics::{
    EconomicCalendarRequest, EconomicCalendarResponse, RawEconomicCalendarResponse,
    sanitize_calendar,
};
use crate::error::{Error, Result};
use crate::history::{
    Adjustment, HistoryRequest, HistorySeries, TradingSession,
    fetch_history_with_timeout_for_client,
};
use crate::scanner::{
    Market, PartiallySupportedColumn, RawScanResponse, ScanQuery, ScanResponse,
    ScanValidationReport, ScannerMetainfo, ScreenerKind, embedded_registry,
};
#[cfg(feature = "search")]
use crate::search::{
    RawSearchResponse, SearchHit, SearchRequest, SearchResponse, sanitize_response,
};
use crate::transport::websocket::{TradingViewWebSocket, connect_socket};

const DEFAULT_USER_AGENT: &str =
    "tvdata-rs/0.1 (+https://github.com/deepentropy/tvscreener reference)";
const DEFAULT_AUTH_TOKEN: &str = "unauthorized_user_token";

fn default_scanner_base_url() -> Url {
    Url::parse("https://scanner.tradingview.com").expect("default scanner endpoint must be valid")
}

fn default_symbol_search_base_url() -> Url {
    Url::parse("https://symbol-search.tradingview.com/symbol_search/v3/")
        .expect("default symbol search endpoint must be valid")
}

fn default_calendar_base_url() -> Url {
    Url::parse("https://chartevents-reuters.tradingview.com/events")
        .expect("default calendar endpoint must be valid")
}

fn default_websocket_url() -> Url {
    Url::parse("wss://data.tradingview.com/socket.io/websocket")
        .expect("default websocket endpoint must be valid")
}

fn default_site_origin() -> Url {
    Url::parse("https://www.tradingview.com").expect("default site origin must be valid")
}

fn default_data_origin() -> Url {
    Url::parse("https://data.tradingview.com").expect("default data origin must be valid")
}

fn default_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_history_session_timeout() -> Duration {
    Duration::from_secs(30)
}

fn default_history_batch_concurrency() -> usize {
    6
}

fn default_backend_http_budget_concurrency() -> usize {
    8
}

fn default_backend_history_batch_concurrency() -> usize {
    6
}

fn default_backend_websocket_budget_concurrency() -> usize {
    6
}

fn default_backend_http_min_interval() -> Duration {
    Duration::from_millis(50)
}

fn default_research_http_budget_concurrency() -> usize {
    4
}

fn default_research_websocket_budget_concurrency() -> usize {
    4
}

fn default_research_http_min_interval() -> Duration {
    Duration::from_millis(25)
}

fn default_interactive_http_budget_concurrency() -> usize {
    2
}

fn default_interactive_websocket_budget_concurrency() -> usize {
    2
}

#[cfg(any(
    feature = "calendar",
    feature = "crypto",
    feature = "equity",
    feature = "forex"
))]
fn default_snapshot_chunk_size() -> usize {
    250
}

#[cfg(any(
    feature = "calendar",
    feature = "crypto",
    feature = "equity",
    feature = "forex"
))]
fn default_snapshot_chunk_concurrency() -> usize {
    4
}

#[cfg(any(
    feature = "calendar",
    feature = "crypto",
    feature = "equity",
    feature = "forex"
))]
fn default_snapshot_auto_single_request_limit() -> usize {
    1_000
}

#[cfg(any(
    feature = "calendar",
    feature = "crypto",
    feature = "equity",
    feature = "forex"
))]
fn default_snapshot_auto_target_cells() -> usize {
    25_000
}

fn default_user_agent() -> String {
    DEFAULT_USER_AGENT.to_owned()
}

fn default_auth_token() -> String {
    DEFAULT_AUTH_TOKEN.to_owned()
}

fn default_anonymous_auth_token() -> String {
    DEFAULT_AUTH_TOKEN.to_owned()
}

fn cookie_header_value(session_id: &str) -> Result<HeaderValue> {
    HeaderValue::from_str(&format!("sessionid={session_id}"))
        .map_err(|_| Error::Protocol("invalid session id configured for cookie header"))
}

fn default_min_retry_interval() -> Duration {
    Duration::from_millis(250)
}

fn default_max_retry_interval() -> Duration {
    Duration::from_secs(2)
}

fn parse_url(value: impl AsRef<str>) -> Result<Url> {
    Url::parse(value.as_ref()).map_err(Into::into)
}

fn referer(origin: &Url) -> String {
    format!("{}/", origin.as_str().trim_end_matches('/'))
}

fn request_preview(request: &RequestBuilder) -> Option<(String, String)> {
    request.try_clone().and_then(|builder| {
        builder
            .build()
            .ok()
            .map(|request| (request.method().to_string(), request.url().to_string()))
    })
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RetryJitter {
    None,
    Full,
    #[default]
    Bounded,
}

impl From<RetryJitter> for Jitter {
    fn from(value: RetryJitter) -> Self {
        match value {
            RetryJitter::None => Self::None,
            RetryJitter::Full => Self::Full,
            RetryJitter::Bounded => Self::Bounded,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Builder)]
pub struct RetryConfig {
    #[builder(default = 2)]
    pub max_retries: u32,
    #[builder(default = default_min_retry_interval())]
    pub min_retry_interval: Duration,
    #[builder(default = default_max_retry_interval())]
    pub max_retry_interval: Duration,
    #[builder(default)]
    pub jitter: RetryJitter,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl RetryConfig {
    pub fn disabled() -> Self {
        Self {
            max_retries: 0,
            ..Self::default()
        }
    }

    fn validate(&self) -> Result<()> {
        if self.min_retry_interval > self.max_retry_interval {
            return Err(Error::InvalidRetryBounds {
                min: self.min_retry_interval,
                max: self.max_retry_interval,
            });
        }

        Ok(())
    }

    fn to_policy(&self) -> ExponentialBackoff {
        ExponentialBackoff::builder()
            .retry_bounds(self.min_retry_interval, self.max_retry_interval)
            .jitter(self.jitter.into())
            .build_with_max_retries(self.max_retries)
    }
}

/// Explicit authentication modes for TradingView HTTP and websocket flows.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AuthMode {
    #[default]
    Anonymous,
    Token,
    Session,
    SessionAndToken,
}

/// Structured authentication configuration for [`TradingViewClient`].
///
/// This is an additive alternative to the legacy `auth_token(...)` and `session_id(...)`
/// builder fields. When provided through `TradingViewClient::builder().auth(...)`, it
/// takes precedence over the legacy auth fields.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthConfig {
    mode: AuthMode,
    auth_token: Option<String>,
    session_id: Option<String>,
}

impl AuthConfig {
    pub fn anonymous() -> Self {
        Self {
            mode: AuthMode::Anonymous,
            auth_token: None,
            session_id: None,
        }
    }

    pub fn token(auth_token: impl Into<String>) -> Self {
        Self {
            mode: AuthMode::Token,
            auth_token: Some(auth_token.into()),
            session_id: None,
        }
    }

    pub fn session(session_id: impl Into<String>) -> Self {
        Self {
            mode: AuthMode::Session,
            auth_token: None,
            session_id: Some(session_id.into()),
        }
    }

    pub fn session_and_token(session_id: impl Into<String>, auth_token: impl Into<String>) -> Self {
        Self {
            mode: AuthMode::SessionAndToken,
            auth_token: Some(auth_token.into()),
            session_id: Some(session_id.into()),
        }
    }

    pub fn mode(&self) -> AuthMode {
        self.mode
    }

    fn resolve(self) -> (String, Option<String>) {
        match self.mode {
            AuthMode::Anonymous => (default_anonymous_auth_token(), None),
            AuthMode::Token => (
                self.auth_token.unwrap_or_else(default_anonymous_auth_token),
                None,
            ),
            AuthMode::Session => (
                default_anonymous_auth_token(),
                self.session_id.filter(|value| !value.is_empty()),
            ),
            AuthMode::SessionAndToken => (
                self.auth_token.unwrap_or_else(default_anonymous_auth_token),
                self.session_id.filter(|value| !value.is_empty()),
            ),
        }
    }
}

impl Default for AuthConfig {
    fn default() -> Self {
        Self::anonymous()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Builder)]
pub struct HistoryClientConfig {
    #[builder(default = default_history_session_timeout())]
    pub session_timeout: Duration,
    #[builder(default = default_history_batch_concurrency())]
    pub default_batch_concurrency: usize,
    #[builder(default)]
    pub default_session: TradingSession,
    #[builder(default)]
    pub default_adjustment: Adjustment,
}

impl Default for HistoryClientConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Builder)]
pub struct RequestBudget {
    pub max_concurrent_http_requests: Option<usize>,
    pub max_concurrent_websocket_sessions: Option<usize>,
    pub min_http_interval: Option<Duration>,
}

impl Default for RequestBudget {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl RequestBudget {
    pub fn disabled() -> Self {
        Self::default()
    }

    fn validate(&self) -> Result<()> {
        if self.max_concurrent_http_requests == Some(0) {
            return Err(Error::InvalidRequestBudget {
                field: "max_concurrent_http_requests",
            });
        }

        if self.max_concurrent_websocket_sessions == Some(0) {
            return Err(Error::InvalidRequestBudget {
                field: "max_concurrent_websocket_sessions",
            });
        }

        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum SnapshotBatchStrategy {
    #[default]
    Auto,
    SingleRequest,
    Chunked {
        chunk_size: usize,
        max_concurrent_requests: usize,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Builder)]
pub struct SnapshotBatchConfig {
    #[builder(default)]
    pub strategy: SnapshotBatchStrategy,
}

impl Default for SnapshotBatchConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl SnapshotBatchConfig {
    fn validate(&self) -> Result<()> {
        match self.strategy {
            SnapshotBatchStrategy::Auto | SnapshotBatchStrategy::SingleRequest => Ok(()),
            SnapshotBatchStrategy::Chunked {
                chunk_size,
                max_concurrent_requests,
            } => {
                if chunk_size == 0 {
                    return Err(Error::InvalidSnapshotBatchConfig {
                        field: "chunk_size",
                    });
                }
                if max_concurrent_requests == 0 {
                    return Err(Error::InvalidSnapshotBatchConfig {
                        field: "max_concurrent_requests",
                    });
                }
                Ok(())
            }
        }
    }
}

#[cfg(any(
    feature = "calendar",
    feature = "crypto",
    feature = "equity",
    feature = "forex"
))]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct SnapshotBatchPlan {
    pub(crate) chunk_size: usize,
    pub(crate) concurrency: usize,
}

pub type WebSocketConnectFuture<'a> =
    Pin<Box<dyn Future<Output = Result<TradingViewWebSocket>> + Send + 'a>>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequestCompletedEvent {
    pub method: String,
    pub url: String,
    pub status: u16,
    pub elapsed_ms: u64,
    pub authenticated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpRequestFailedEvent {
    pub method: String,
    pub url: String,
    pub elapsed_ms: u64,
    pub authenticated: bool,
    pub kind: crate::error::ErrorKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebSocketConnectedEvent {
    pub url: String,
    pub authenticated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WebSocketConnectionFailedEvent {
    pub url: String,
    pub authenticated: bool,
    pub kind: crate::error::ErrorKind,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryBatchMode {
    Strict,
    Detailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryBatchCompletedEvent {
    pub requested: usize,
    pub successes: usize,
    pub missing: usize,
    pub failures: usize,
    pub concurrency: usize,
    pub mode: HistoryBatchMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientEvent {
    HttpRequestCompleted(HttpRequestCompletedEvent),
    HttpRequestFailed(HttpRequestFailedEvent),
    WebSocketConnected(WebSocketConnectedEvent),
    WebSocketConnectionFailed(WebSocketConnectionFailedEvent),
    HistoryBatchCompleted(HistoryBatchCompletedEvent),
}

pub trait ClientObserver: std::fmt::Debug + Send + Sync {
    fn on_event(&self, event: &ClientEvent);
}

pub trait WebSocketConnector: std::fmt::Debug + Send + Sync {
    fn connect<'a>(
        &'a self,
        endpoints: &'a Endpoints,
        user_agent: &'a str,
        session_id: Option<&'a str>,
    ) -> WebSocketConnectFuture<'a>;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct DefaultWebSocketConnector;

impl WebSocketConnector for DefaultWebSocketConnector {
    fn connect<'a>(
        &'a self,
        endpoints: &'a Endpoints,
        user_agent: &'a str,
        session_id: Option<&'a str>,
    ) -> WebSocketConnectFuture<'a> {
        Box::pin(connect_socket(endpoints, user_agent, session_id))
    }
}

#[derive(Debug, Clone, Builder)]
pub struct TransportConfig {
    #[builder(default = default_timeout())]
    pub timeout: Duration,
    #[builder(default = RetryConfig::default())]
    pub retry: RetryConfig,
    #[builder(default = default_user_agent(), into)]
    pub user_agent: String,
    pub http_client: Option<ClientWithMiddleware>,
    pub websocket_connector: Option<Arc<dyn WebSocketConnector>>,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

#[derive(Debug)]
struct RequestBudgetState {
    http_limiter: Option<Arc<Semaphore>>,
    websocket_limiter: Option<Arc<Semaphore>>,
    http_pacer: Option<Arc<Mutex<TokioInstant>>>,
}

impl RequestBudgetState {
    fn new(config: &RequestBudget) -> Self {
        Self {
            http_limiter: config
                .max_concurrent_http_requests
                .map(|limit| Arc::new(Semaphore::new(limit))),
            websocket_limiter: config
                .max_concurrent_websocket_sessions
                .map(|limit| Arc::new(Semaphore::new(limit))),
            http_pacer: config
                .min_http_interval
                .map(|_| Arc::new(Mutex::new(TokioInstant::now()))),
        }
    }
}

#[derive(Debug, Clone, Builder)]
pub struct TradingViewClientConfig {
    #[builder(default = Endpoints::default())]
    pub endpoints: Endpoints,
    #[builder(default = TransportConfig::default())]
    pub transport: TransportConfig,
    #[builder(default = AuthConfig::default())]
    pub auth: AuthConfig,
    #[builder(default = HistoryClientConfig::default())]
    pub history: HistoryClientConfig,
    #[builder(default = SnapshotBatchConfig::default())]
    pub snapshot_batch: SnapshotBatchConfig,
    #[builder(default = RequestBudget::default())]
    pub request_budget: RequestBudget,
    pub observer: Option<Arc<dyn ClientObserver>>,
}

impl Default for TradingViewClientConfig {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl TradingViewClientConfig {
    pub fn backend_history() -> Self {
        Self::builder()
            .transport(
                TransportConfig::builder()
                    .timeout(Duration::from_secs(60))
                    .retry(
                        RetryConfig::builder()
                            .max_retries(4)
                            .min_retry_interval(Duration::from_millis(500))
                            .max_retry_interval(Duration::from_secs(5))
                            .build(),
                    )
                    .build(),
            )
            .history(
                HistoryClientConfig::builder()
                    .session_timeout(Duration::from_secs(60))
                    .default_batch_concurrency(default_backend_history_batch_concurrency())
                    .default_session(TradingSession::Regular)
                    .default_adjustment(Adjustment::Splits)
                    .build(),
            )
            .request_budget(
                RequestBudget::builder()
                    .max_concurrent_http_requests(default_backend_http_budget_concurrency())
                    .max_concurrent_websocket_sessions(
                        default_backend_websocket_budget_concurrency(),
                    )
                    .min_http_interval(default_backend_http_min_interval())
                    .build(),
            )
            .build()
    }

    pub fn research() -> Self {
        Self::builder()
            .transport(
                TransportConfig::builder()
                    .timeout(Duration::from_secs(45))
                    .retry(
                        RetryConfig::builder()
                            .max_retries(2)
                            .min_retry_interval(Duration::from_millis(250))
                            .max_retry_interval(Duration::from_secs(2))
                            .build(),
                    )
                    .build(),
            )
            .request_budget(
                RequestBudget::builder()
                    .max_concurrent_http_requests(default_research_http_budget_concurrency())
                    .max_concurrent_websocket_sessions(
                        default_research_websocket_budget_concurrency(),
                    )
                    .min_http_interval(default_research_http_min_interval())
                    .build(),
            )
            .build()
    }

    pub fn interactive() -> Self {
        Self::builder()
            .transport(
                TransportConfig::builder()
                    .timeout(Duration::from_secs(15))
                    .retry(
                        RetryConfig::builder()
                            .max_retries(1)
                            .min_retry_interval(Duration::from_millis(100))
                            .max_retry_interval(Duration::from_millis(500))
                            .build(),
                    )
                    .build(),
            )
            .request_budget(
                RequestBudget::builder()
                    .max_concurrent_http_requests(default_interactive_http_budget_concurrency())
                    .max_concurrent_websocket_sessions(
                        default_interactive_websocket_budget_concurrency(),
                    )
                    .build(),
            )
            .build()
    }
}

/// Typed endpoint configuration for the TradingView surfaces used by the client.
#[derive(Debug, Clone, PartialEq, Eq, Builder)]
pub struct Endpoints {
    #[builder(default = default_scanner_base_url())]
    scanner_base_url: Url,
    #[builder(default = default_symbol_search_base_url())]
    symbol_search_base_url: Url,
    #[builder(default = default_calendar_base_url())]
    calendar_base_url: Url,
    #[builder(default = default_websocket_url())]
    websocket_url: Url,
    #[builder(default = default_site_origin())]
    site_origin: Url,
    #[builder(default = default_data_origin())]
    data_origin: Url,
}

impl Default for Endpoints {
    fn default() -> Self {
        Self::builder().build()
    }
}

impl Endpoints {
    pub fn scanner_base_url(&self) -> &Url {
        &self.scanner_base_url
    }

    pub fn symbol_search_base_url(&self) -> &Url {
        &self.symbol_search_base_url
    }

    pub fn calendar_base_url(&self) -> &Url {
        &self.calendar_base_url
    }

    pub fn websocket_url(&self) -> &Url {
        &self.websocket_url
    }

    pub fn site_origin(&self) -> &Url {
        &self.site_origin
    }

    pub fn data_origin(&self) -> &Url {
        &self.data_origin
    }

    pub fn with_scanner_base_url(mut self, url: impl AsRef<str>) -> Result<Self> {
        self.scanner_base_url = parse_url(url)?;
        Ok(self)
    }

    pub fn with_symbol_search_base_url(mut self, url: impl AsRef<str>) -> Result<Self> {
        self.symbol_search_base_url = parse_url(url)?;
        Ok(self)
    }

    pub fn with_calendar_base_url(mut self, url: impl AsRef<str>) -> Result<Self> {
        self.calendar_base_url = parse_url(url)?;
        Ok(self)
    }

    pub fn with_websocket_url(mut self, url: impl AsRef<str>) -> Result<Self> {
        self.websocket_url = parse_url(url)?;
        Ok(self)
    }

    pub fn with_site_origin(mut self, url: impl AsRef<str>) -> Result<Self> {
        self.site_origin = parse_url(url)?;
        Ok(self)
    }

    pub fn with_data_origin(mut self, url: impl AsRef<str>) -> Result<Self> {
        self.data_origin = parse_url(url)?;
        Ok(self)
    }

    pub fn scanner_url(&self, route: &str) -> Result<Url> {
        self.scanner_base_url
            .join(route.trim_start_matches('/'))
            .map_err(Into::into)
    }

    pub fn scanner_metainfo_url(&self, market: &Market) -> Result<Url> {
        self.scanner_url(&format!("{}/metainfo", market.as_str()))
    }
}

/// High-level entry point for TradingView screener, search, quote, and history data.
///
/// Most consumers should start with [`TradingViewClient::builder`] and then use one of the
/// product-oriented facades such as [`TradingViewClient::equity`],
/// [`TradingViewClient::crypto`], or [`TradingViewClient::forex`].
///
/// # Examples
///
/// ```no_run
/// use tvdata_rs::{Result, TradingViewClient};
///
/// #[tokio::main]
/// async fn main() -> Result<()> {
///     let client = TradingViewClient::builder().build()?;
///
///     let quote = client.equity().quote("NASDAQ:AAPL").await?;
///     println!("{:?}", quote.close);
///
///     Ok(())
/// }
/// ```
#[derive(Debug, Clone)]
pub struct TradingViewClient {
    http: ClientWithMiddleware,
    endpoints: Endpoints,
    user_agent: String,
    auth_token: String,
    session_id: Option<String>,
    history_config: HistoryClientConfig,
    snapshot_batch_config: SnapshotBatchConfig,
    request_budget: RequestBudget,
    request_budget_state: Arc<RequestBudgetState>,
    websocket_connector: Arc<dyn WebSocketConnector>,
    observer: Option<Arc<dyn ClientObserver>>,
    metainfo_cache: Arc<RwLock<HashMap<String, ScannerMetainfo>>>,
}

#[bon]
impl TradingViewClient {
    /// Builds a [`TradingViewClient`] with validated endpoint configuration and retry settings.
    #[builder]
    pub fn new(
        #[builder(default = Endpoints::default())] endpoints: Endpoints,
        #[builder(default = default_timeout())] timeout: Duration,
        #[builder(default = RetryConfig::default())] retry: RetryConfig,
        #[builder(default = HistoryClientConfig::default())] history_config: HistoryClientConfig,
        #[builder(default = SnapshotBatchConfig::default())]
        snapshot_batch_config: SnapshotBatchConfig,
        #[builder(default = RequestBudget::default())] request_budget: RequestBudget,
        #[builder(default = default_user_agent(), into)] user_agent: String,
        #[builder(default = default_auth_token(), into)] auth_token: String,
        #[builder(into)] session_id: Option<String>,
        auth: Option<AuthConfig>,
        transport_config: Option<TransportConfig>,
        http_client: Option<ClientWithMiddleware>,
        websocket_connector: Option<Arc<dyn WebSocketConnector>>,
        observer: Option<Arc<dyn ClientObserver>>,
    ) -> Result<Self> {
        let transport_config = transport_config.unwrap_or(TransportConfig {
            timeout,
            retry,
            user_agent,
            http_client,
            websocket_connector,
        });
        let TransportConfig {
            timeout,
            retry,
            user_agent,
            http_client,
            websocket_connector,
        } = transport_config;

        let (auth_token, session_id) = auth
            .map(AuthConfig::resolve)
            .unwrap_or((auth_token, session_id));

        request_budget.validate()?;
        snapshot_batch_config.validate()?;

        let http = if let Some(http_client) = http_client {
            http_client
        } else {
            retry.validate()?;

            let base_http = reqwest::Client::builder()
                .timeout(timeout)
                .build()
                .map_err(Error::from)?;

            if retry.max_retries == 0 {
                ClientWithMiddleware::from(base_http)
            } else {
                MiddlewareClientBuilder::new(base_http)
                    .with(RetryTransientMiddleware::new_with_policy(retry.to_policy()))
                    .build()
            }
        };

        Ok(Self {
            http,
            endpoints,
            user_agent,
            auth_token,
            session_id,
            history_config,
            snapshot_batch_config,
            request_budget: request_budget.clone(),
            request_budget_state: Arc::new(RequestBudgetState::new(&request_budget)),
            websocket_connector: websocket_connector
                .unwrap_or_else(|| Arc::new(DefaultWebSocketConnector)),
            observer,
            metainfo_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub fn for_backend_history() -> Result<Self> {
        Self::from_config(TradingViewClientConfig::backend_history())
    }

    /// Builds a client tuned for research-style workflows with moderate retries and timeout.
    pub fn for_research() -> Result<Self> {
        Self::from_config(TradingViewClientConfig::research())
    }

    /// Builds a client tuned for lower-latency interactive usage.
    pub fn for_interactive() -> Result<Self> {
        Self::from_config(TradingViewClientConfig::interactive())
    }

    pub fn from_config(config: TradingViewClientConfig) -> Result<Self> {
        match config.observer {
            Some(observer) => Self::builder()
                .endpoints(config.endpoints)
                .transport_config(config.transport)
                .auth(config.auth)
                .history_config(config.history)
                .snapshot_batch_config(config.snapshot_batch)
                .request_budget(config.request_budget)
                .observer(observer)
                .build(),
            None => Self::builder()
                .endpoints(config.endpoints)
                .transport_config(config.transport)
                .auth(config.auth)
                .history_config(config.history)
                .snapshot_batch_config(config.snapshot_batch)
                .request_budget(config.request_budget)
                .build(),
        }
    }

    pub fn endpoints(&self) -> &Endpoints {
        &self.endpoints
    }

    pub(crate) fn auth_token(&self) -> &str {
        &self.auth_token
    }

    pub(crate) fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
    }

    pub fn history_config(&self) -> &HistoryClientConfig {
        &self.history_config
    }

    pub fn snapshot_batch_config(&self) -> &SnapshotBatchConfig {
        &self.snapshot_batch_config
    }

    pub fn request_budget(&self) -> &RequestBudget {
        &self.request_budget
    }

    pub(crate) fn effective_history_batch_concurrency(&self, requested: usize) -> usize {
        if requested == 0 {
            return 0;
        }

        self.request_budget
            .max_concurrent_websocket_sessions
            .map(|cap| requested.min(cap))
            .unwrap_or(requested)
    }

    #[cfg(any(
        feature = "calendar",
        feature = "crypto",
        feature = "equity",
        feature = "forex"
    ))]
    pub(crate) fn plan_snapshot_batch(&self, symbols: usize, columns: usize) -> SnapshotBatchPlan {
        let effective_http_cap = self
            .request_budget
            .max_concurrent_http_requests
            .unwrap_or(default_snapshot_chunk_concurrency());
        let cap = effective_http_cap.max(1);

        match self.snapshot_batch_config.strategy {
            SnapshotBatchStrategy::SingleRequest => SnapshotBatchPlan {
                chunk_size: symbols.max(1),
                concurrency: 1,
            },
            SnapshotBatchStrategy::Chunked {
                chunk_size,
                max_concurrent_requests,
            } => SnapshotBatchPlan {
                chunk_size: chunk_size.max(1),
                concurrency: max_concurrent_requests.min(cap).max(1),
            },
            SnapshotBatchStrategy::Auto => {
                let cells = symbols.saturating_mul(columns.max(1));
                let auto_chunk_size = (default_snapshot_auto_target_cells() / columns.max(1))
                    .clamp(100, default_snapshot_chunk_size());

                if symbols <= default_snapshot_chunk_size()
                    || (symbols <= default_snapshot_auto_single_request_limit()
                        && cells <= default_snapshot_auto_target_cells())
                {
                    SnapshotBatchPlan {
                        chunk_size: symbols.max(1),
                        concurrency: 1,
                    }
                } else {
                    SnapshotBatchPlan {
                        chunk_size: auto_chunk_size.max(1),
                        concurrency: default_snapshot_chunk_concurrency().min(cap).max(1),
                    }
                }
            }
        }
    }

    /// Executes a low-level TradingView screener query.
    ///
    /// This is the most flexible API in the crate and is useful when you need fields or filters
    /// that are not covered by the higher-level market facades.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tvdata_rs::scanner::fields::{core, price};
    /// use tvdata_rs::scanner::ScanQuery;
    /// use tvdata_rs::{Result, TradingViewClient};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = TradingViewClient::builder().build()?;
    ///     let query = ScanQuery::new()
    ///         .market("america")
    ///         .select([core::NAME, price::CLOSE])
    ///         .page(0, 10)?;
    ///
    ///     let response = client.scan(&query).await?;
    ///     println!("rows: {}", response.rows.len());
    ///
    ///     Ok(())
    /// }
    /// ```
    pub async fn scan(&self, query: &ScanQuery) -> Result<ScanResponse> {
        let route = query.route_segment();
        #[cfg(feature = "tracing")]
        debug!(
            target: "tvdata_rs::scan",
            route = %route,
            columns = query.columns.len(),
            markets = query.markets.len(),
            "executing scanner query",
        );

        let raw: RawScanResponse = self
            .execute_json(
                self.request(self.http.post(self.endpoints.scanner_url(&route)?))?
                    .json(query),
            )
            .await?;

        let response = raw.into_response()?;
        #[cfg(feature = "tracing")]
        debug!(
            target: "tvdata_rs::scan",
            route = %route,
            rows = response.rows.len(),
            "scanner query completed",
        );
        Ok(response)
    }

    /// Validates a scan query against live TradingView metainfo before execution.
    ///
    /// Validation currently requires the query to specify one or more markets so the
    /// client can resolve the corresponding `/{market}/metainfo` endpoints.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tvdata_rs::scanner::fields::{core, price};
    /// use tvdata_rs::scanner::ScanQuery;
    /// use tvdata_rs::{Result, TradingViewClient};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = TradingViewClient::builder().build()?;
    ///     let query = ScanQuery::new()
    ///         .market("america")
    ///         .select([core::NAME, price::CLOSE]);
    ///
    ///     let report = client.validate_scan_query(&query).await?;
    ///     assert!(report.is_strictly_supported());
    ///     Ok(())
    /// }
    /// ```
    pub async fn validate_scan_query(&self, query: &ScanQuery) -> Result<ScanValidationReport> {
        let route_segment = query.route_segment();
        let markets = validation_markets(query)?;
        #[cfg(feature = "tracing")]
        debug!(
            target: "tvdata_rs::scan",
            route = %route_segment,
            columns = query.columns.len(),
            markets = markets.len(),
            "validating scanner query against live metainfo",
        );
        let mut market_metainfo = Vec::with_capacity(markets.len());

        for market in &markets {
            market_metainfo.push((market.clone(), self.cached_metainfo(market).await?));
        }

        let mut supported_columns = Vec::new();
        let mut partially_supported_columns = Vec::new();
        let mut unsupported_columns = Vec::new();
        let mut seen = HashSet::new();

        for column in &query.columns {
            if !seen.insert(column.as_str().to_owned()) {
                continue;
            }

            let mut supported_markets = Vec::new();
            let mut unsupported_markets = Vec::new();

            for (market, metainfo) in &market_metainfo {
                if supports_column_for_market(market, metainfo, column.as_str()) {
                    supported_markets.push(market.clone());
                } else {
                    unsupported_markets.push(market.clone());
                }
            }

            match (supported_markets.is_empty(), unsupported_markets.is_empty()) {
                (true, false) => unsupported_columns.push(column.clone()),
                (false, true) => supported_columns.push(column.clone()),
                (false, false) => partially_supported_columns.push(PartiallySupportedColumn {
                    column: column.clone(),
                    supported_markets,
                    unsupported_markets,
                }),
                (true, true) => {}
            }
        }

        let report = ScanValidationReport {
            route_segment,
            requested_markets: markets,
            supported_columns,
            partially_supported_columns,
            unsupported_columns,
        };

        #[cfg(feature = "tracing")]
        debug!(
            target: "tvdata_rs::scan",
            route = %report.route_segment,
            supported = report.supported_columns.len(),
            partial = report.partially_supported_columns.len(),
            unsupported = report.unsupported_columns.len(),
            "scanner validation completed",
        );

        Ok(report)
    }

    /// Executes a scan only after validating all requested fields against live TradingView
    /// metainfo for the selected markets.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tvdata_rs::scanner::fields::{core, price};
    /// use tvdata_rs::scanner::ScanQuery;
    /// use tvdata_rs::{Result, TradingViewClient};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = TradingViewClient::builder().build()?;
    ///     let query = ScanQuery::new()
    ///         .market("america")
    ///         .select([core::NAME, price::CLOSE]);
    ///
    ///     let response = client.scan_validated(&query).await?;
    ///     println!("rows: {}", response.rows.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn scan_validated(&self, query: &ScanQuery) -> Result<ScanResponse> {
        let report = self.validate_scan_query(query).await?;
        if !report.is_strictly_supported() {
            let fields = report
                .strict_violation_column_names()
                .into_iter()
                .map(str::to_owned)
                .collect();
            return Err(Error::UnsupportedScanFields {
                route: report.route_segment,
                fields,
            });
        }

        self.scan(query).await
    }

    /// Filters a scan query down to columns that are fully supported across the selected
    /// markets according to live TradingView metainfo plus the embedded registry fallback.
    ///
    /// Partially supported columns are removed from the filtered query to keep the result
    /// safe across all requested markets.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tvdata_rs::scanner::fields::{fundamentals, price};
    /// use tvdata_rs::scanner::ScanQuery;
    /// use tvdata_rs::{Result, TradingViewClient};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = TradingViewClient::builder().build()?;
    ///     let query = ScanQuery::new()
    ///         .markets(["america", "crypto"])
    ///         .select([price::CLOSE, fundamentals::MARKET_CAP_BASIC]);
    ///
    ///     let (filtered, report) = client.filter_scan_query(&query).await?;
    ///     println!("filtered columns: {:?}", report.filtered_column_names());
    ///     assert!(!filtered.columns.is_empty());
    ///     Ok(())
    /// }
    /// ```
    pub async fn filter_scan_query(
        &self,
        query: &ScanQuery,
    ) -> Result<(ScanQuery, ScanValidationReport)> {
        let report = self.validate_scan_query(query).await?;
        let filtered = report.filtered_query(query);

        if filtered.columns.is_empty() {
            let fields = report
                .strict_violation_column_names()
                .into_iter()
                .map(str::to_owned)
                .collect();
            return Err(Error::UnsupportedScanFields {
                route: report.route_segment,
                fields,
            });
        }

        Ok((filtered, report))
    }

    /// Executes a scan after dropping columns that are not fully supported across
    /// all selected markets.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tvdata_rs::scanner::fields::{fundamentals, price};
    /// use tvdata_rs::scanner::ScanQuery;
    /// use tvdata_rs::{Result, TradingViewClient};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = TradingViewClient::builder().build()?;
    ///     let query = ScanQuery::new()
    ///         .markets(["america", "crypto"])
    ///         .select([price::CLOSE, fundamentals::MARKET_CAP_BASIC]);
    ///
    ///     let response = client.scan_supported(&query).await?;
    ///     println!("rows: {}", response.rows.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn scan_supported(&self, query: &ScanQuery) -> Result<ScanResponse> {
        let (filtered, _) = self.filter_scan_query(query).await?;
        self.scan(&filtered).await
    }

    /// Fetches TradingView scanner metainfo for a specific market or screener.
    ///
    /// This endpoint returns the currently supported field names and their value types
    /// as exposed by TradingView for the selected screener route.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tvdata_rs::{Result, TradingViewClient};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = TradingViewClient::builder().build()?;
    ///     let metainfo = client.metainfo("america").await?;
    ///
    ///     println!("fields: {}", metainfo.fields.len());
    ///     Ok(())
    /// }
    /// ```
    pub async fn metainfo(&self, market: impl Into<Market>) -> Result<ScannerMetainfo> {
        let market = market.into();
        self.cached_metainfo(&market).await
    }

    /// Searches TradingView symbol metadata using the symbol search endpoint.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tvdata_rs::{Result, SearchRequest, TradingViewClient};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = TradingViewClient::builder().build()?;
    ///     let hits = client
    ///         .search(&SearchRequest::builder().text("AAPL").build())
    ///         .await?;
    ///
    ///     println!("matches: {}", hits.len());
    ///     Ok(())
    /// }
    /// ```
    #[cfg(feature = "search")]
    pub async fn search(&self, request: &SearchRequest) -> Result<Vec<SearchHit>> {
        Ok(self.search_response(request).await?.hits)
    }

    /// Searches equities using TradingView's current `search_type=stock` filter.
    #[cfg(feature = "search")]
    pub async fn search_equities(&self, text: impl Into<String>) -> Result<Vec<SearchHit>> {
        Ok(self.search_equities_response(text).await?.hits)
    }

    /// Searches equities and returns the richer v3 response envelope.
    #[cfg(feature = "search")]
    pub async fn search_equities_response(
        &self,
        text: impl Into<String>,
    ) -> Result<SearchResponse> {
        self.search_response(&SearchRequest::equities(text)).await
    }

    /// Searches forex instruments using TradingView's current `search_type=forex` filter.
    #[cfg(feature = "search")]
    pub async fn search_forex(&self, text: impl Into<String>) -> Result<Vec<SearchHit>> {
        Ok(self.search_forex_response(text).await?.hits)
    }

    /// Searches forex instruments and returns the richer v3 response envelope.
    #[cfg(feature = "search")]
    pub async fn search_forex_response(&self, text: impl Into<String>) -> Result<SearchResponse> {
        self.search_response(&SearchRequest::forex(text)).await
    }

    /// Searches crypto instruments using TradingView's current `search_type=crypto` filter.
    #[cfg(feature = "search")]
    pub async fn search_crypto(&self, text: impl Into<String>) -> Result<Vec<SearchHit>> {
        Ok(self.search_crypto_response(text).await?.hits)
    }

    /// Searches crypto instruments and returns the richer v3 response envelope.
    #[cfg(feature = "search")]
    pub async fn search_crypto_response(&self, text: impl Into<String>) -> Result<SearchResponse> {
        self.search_response(&SearchRequest::crypto(text)).await
    }

    /// Searches option-like instruments.
    ///
    /// As of March 22, 2026, TradingView's live `symbol_search/v3` endpoint rejects
    /// `search_type=option`, so this method performs a broader search and then keeps
    /// hits that look option-related based on the returned payload.
    #[cfg(feature = "search")]
    pub async fn search_options(&self, text: impl Into<String>) -> Result<Vec<SearchHit>> {
        Ok(self.search_options_response(text).await?.hits)
    }

    /// Searches option-like instruments and returns the filtered v3 response envelope.
    #[cfg(feature = "search")]
    pub async fn search_options_response(&self, text: impl Into<String>) -> Result<SearchResponse> {
        let response = self.search_response(&SearchRequest::options(text)).await?;
        Ok(response.filtered(SearchHit::is_option_like))
    }

    /// Searches TradingView symbol metadata and returns the richer v3 search envelope.
    ///
    /// This includes the remaining symbol count reported by TradingView, plus richer
    /// instrument metadata such as identifiers and listing/source information.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tvdata_rs::{Result, SearchRequest, TradingViewClient};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = TradingViewClient::builder().build()?;
    ///     let response = client
    ///         .search_response(&SearchRequest::builder().text("AAPL").build())
    ///         .await?;
    ///
    ///     println!("hits: {}", response.hits.len());
    ///     println!("remaining: {}", response.symbols_remaining);
    ///     Ok(())
    /// }
    /// ```
    #[cfg(feature = "search")]
    pub async fn search_response(&self, request: &SearchRequest) -> Result<SearchResponse> {
        if request.text.trim().is_empty() {
            return Err(Error::EmptySearchQuery);
        }

        #[cfg(feature = "tracing")]
        debug!(
            target: "tvdata_rs::search",
            text_len = request.text.len(),
            exchange = request.exchange.as_deref().unwrap_or(""),
            search_type = request.instrument_type.as_deref().unwrap_or(""),
            start = request.start,
            "executing TradingView symbol search",
        );

        let raw: RawSearchResponse = self
            .execute_json(
                self.request(self.http.get(self.endpoints.symbol_search_base_url.clone()))?
                    .query(&request.to_query_pairs()),
            )
            .await?;

        let response = sanitize_response(raw);
        #[cfg(feature = "tracing")]
        debug!(
            target: "tvdata_rs::search",
            hits = response.hits.len(),
            symbols_remaining = response.symbols_remaining,
            "TradingView symbol search completed",
        );
        Ok(response)
    }

    /// Fetches economic calendar events from TradingView's Reuters-backed calendar feed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tvdata_rs::{EconomicCalendarRequest, Result, TradingViewClient};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = TradingViewClient::builder().build()?;
    ///     let response = client
    ///         .economic_calendar(&EconomicCalendarRequest::upcoming(7))
    ///         .await?;
    ///
    ///     println!("events: {}", response.events.len());
    ///     Ok(())
    /// }
    /// ```
    #[cfg(feature = "economics")]
    pub async fn economic_calendar(
        &self,
        request: &EconomicCalendarRequest,
    ) -> Result<EconomicCalendarResponse> {
        #[cfg(feature = "tracing")]
        debug!(
            target: "tvdata_rs::calendar",
            from = %request.from,
            to = %request.to,
            "executing economic calendar request",
        );

        let raw: RawEconomicCalendarResponse = self
            .execute_json(
                self.request(self.http.get(self.endpoints.calendar_base_url().clone()))?
                    .query(&request.to_query_pairs()?),
            )
            .await?;

        let response = sanitize_calendar(raw);
        #[cfg(feature = "tracing")]
        debug!(
            target: "tvdata_rs::calendar",
            events = response.events.len(),
            status = response.status.as_deref().unwrap_or(""),
            "economic calendar request completed",
        );
        Ok(response)
    }

    /// Fetches an earnings calendar window from TradingView scanner fields.
    ///
    /// This is a market-wide calendar product, distinct from
    /// `client.equity().earnings_calendar("NASDAQ:AAPL")`, which returns
    /// single-symbol analyst earnings metadata.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tvdata_rs::{CalendarWindowRequest, Result, TradingViewClient};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = TradingViewClient::builder().build()?;
    ///     let events = client
    ///         .earnings_calendar(&CalendarWindowRequest::upcoming("america", 7))
    ///         .await?;
    ///
    ///     println!("events: {}", events.len());
    ///     Ok(())
    /// }
    /// ```
    #[cfg(feature = "calendar")]
    pub async fn earnings_calendar(
        &self,
        request: &CalendarWindowRequest,
    ) -> Result<Vec<EarningsCalendarEntry>> {
        self.corporate_earnings_calendar(request).await
    }

    /// Fetches a dividend calendar window from TradingView scanner fields.
    ///
    /// The request can be anchored either on upcoming ex-dates or upcoming
    /// payment dates through [`DividendCalendarRequest::date_kind`].
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tvdata_rs::{DividendCalendarRequest, Result, TradingViewClient};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = TradingViewClient::builder().build()?;
    ///     let events = client
    ///         .dividend_calendar(&DividendCalendarRequest::upcoming("america", 14))
    ///         .await?;
    ///
    ///     println!("events: {}", events.len());
    ///     Ok(())
    /// }
    /// ```
    #[cfg(feature = "calendar")]
    pub async fn dividend_calendar(
        &self,
        request: &DividendCalendarRequest,
    ) -> Result<Vec<DividendCalendarEntry>> {
        self.corporate_dividend_calendar(request).await
    }

    /// Fetches an IPO calendar window from TradingView scanner fields.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tvdata_rs::{CalendarWindowRequest, Result, TradingViewClient};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = TradingViewClient::builder().build()?;
    ///     let events = client
    ///         .ipo_calendar(&CalendarWindowRequest::trailing("america", 30))
    ///         .await?;
    ///
    ///     println!("events: {}", events.len());
    ///     Ok(())
    /// }
    /// ```
    #[cfg(feature = "calendar")]
    pub async fn ipo_calendar(
        &self,
        request: &CalendarWindowRequest,
    ) -> Result<Vec<IpoCalendarEntry>> {
        self.corporate_ipo_calendar(request).await
    }

    /// Downloads a single OHLCV history series over TradingView's chart websocket.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use tvdata_rs::{HistoryRequest, Interval, Result, TradingViewClient};
    ///
    /// #[tokio::main]
    /// async fn main() -> Result<()> {
    ///     let client = TradingViewClient::builder().build()?;
    ///     let request = HistoryRequest::new("NASDAQ:AAPL", Interval::Day1, 30);
    ///     let series = client.history(&request).await?;
    ///
    ///     println!("bars: {}", series.bars.len());
    ///     Ok(())
    /// }
    /// ```
    ///
    /// To fetch the maximum history currently available, construct the request
    /// with `HistoryRequest::max("NASDAQ:AAPL", Interval::Day1)`.
    pub async fn history(&self, request: &HistoryRequest) -> Result<HistorySeries> {
        let _websocket_budget = self.acquire_websocket_slot().await?;

        #[cfg(feature = "tracing")]
        debug!(
            target: "tvdata_rs::history",
            symbol = %request.symbol.as_str(),
            interval = request.interval.as_code(),
            bars = request.bars,
            fetch_all = request.fetch_all,
            session = request.session.as_code(),
            adjustment = request.adjustment.as_code(),
            authenticated = self.session_id().is_some(),
            "fetching TradingView history",
        );

        let series = fetch_history_with_timeout_for_client(
            self,
            request,
            self.history_config.session_timeout,
        )
        .await?;

        #[cfg(feature = "tracing")]
        debug!(
            target: "tvdata_rs::history",
            symbol = %series.symbol.as_str(),
            bars = series.bars.len(),
            authenticated = series.provenance.authenticated,
            "TradingView history fetch completed",
        );

        Ok(series)
    }

    async fn execute_json<T>(&self, request: RequestBuilder) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let body = self.execute_text(request).await?;
        serde_json::from_str(&body).map_err(Into::into)
    }

    async fn execute_text(&self, request: RequestBuilder) -> Result<String> {
        let _http_budget = self.acquire_http_slot().await?;

        let preview = request_preview(&request);
        let started_at = Instant::now();
        let response = match request.send().await {
            Ok(response) => response,
            Err(error) => {
                self.emit_event(ClientEvent::HttpRequestFailed(HttpRequestFailedEvent {
                    method: preview
                        .as_ref()
                        .map(|(method, _)| method.clone())
                        .unwrap_or_else(|| "UNKNOWN".to_owned()),
                    url: preview
                        .as_ref()
                        .map(|(_, url)| url.clone())
                        .unwrap_or_else(|| "<opaque>".to_owned()),
                    elapsed_ms: started_at.elapsed().as_millis() as u64,
                    authenticated: self.session_id().is_some(),
                    kind: crate::error::ErrorKind::Transport,
                }));
                #[cfg(feature = "tracing")]
                warn!(
                    target: "tvdata_rs::http",
                    method = preview.as_ref().map(|(method, _)| method.as_str()).unwrap_or("UNKNOWN"),
                    url = preview.as_ref().map(|(_, url)| url.as_str()).unwrap_or("<opaque>"),
                    elapsed_ms = started_at.elapsed().as_millis() as u64,
                    error = %error,
                    "TradingView HTTP request failed before receiving a response",
                );
                return Err(Error::from(error));
            }
        };
        let status = response.status();
        let body = match response.text().await {
            Ok(body) => body,
            Err(error) => {
                self.emit_event(ClientEvent::HttpRequestFailed(HttpRequestFailedEvent {
                    method: preview
                        .as_ref()
                        .map(|(method, _)| method.clone())
                        .unwrap_or_else(|| "UNKNOWN".to_owned()),
                    url: preview
                        .as_ref()
                        .map(|(_, url)| url.clone())
                        .unwrap_or_else(|| "<opaque>".to_owned()),
                    elapsed_ms: started_at.elapsed().as_millis() as u64,
                    authenticated: self.session_id().is_some(),
                    kind: crate::error::ErrorKind::Transport,
                }));
                #[cfg(feature = "tracing")]
                warn!(
                    target: "tvdata_rs::http",
                    method = preview.as_ref().map(|(method, _)| method.as_str()).unwrap_or("UNKNOWN"),
                    url = preview.as_ref().map(|(_, url)| url.as_str()).unwrap_or("<opaque>"),
                    status = status.as_u16(),
                    elapsed_ms = started_at.elapsed().as_millis() as u64,
                    error = %error,
                    "TradingView HTTP response body could not be read",
                );
                return Err(Error::from(error));
            }
        };

        self.emit_event(ClientEvent::HttpRequestCompleted(
            HttpRequestCompletedEvent {
                method: preview
                    .as_ref()
                    .map(|(method, _)| method.clone())
                    .unwrap_or_else(|| "UNKNOWN".to_owned()),
                url: preview
                    .as_ref()
                    .map(|(_, url)| url.clone())
                    .unwrap_or_else(|| "<opaque>".to_owned()),
                status: status.as_u16(),
                elapsed_ms: started_at.elapsed().as_millis() as u64,
                authenticated: self.session_id().is_some(),
            },
        ));

        #[cfg(feature = "tracing")]
        debug!(
            target: "tvdata_rs::http",
            method = preview.as_ref().map(|(method, _)| method.as_str()).unwrap_or("UNKNOWN"),
            url = preview.as_ref().map(|(_, url)| url.as_str()).unwrap_or("<opaque>"),
            status = status.as_u16(),
            body_bytes = body.len(),
            elapsed_ms = started_at.elapsed().as_millis() as u64,
            "TradingView HTTP request completed",
        );

        if !status.is_success() {
            #[cfg(feature = "tracing")]
            warn!(
                target: "tvdata_rs::http",
                method = preview.as_ref().map(|(method, _)| method.as_str()).unwrap_or("UNKNOWN"),
                url = preview.as_ref().map(|(_, url)| url.as_str()).unwrap_or("<opaque>"),
                status = status.as_u16(),
                elapsed_ms = started_at.elapsed().as_millis() as u64,
                "TradingView HTTP request returned non-success status",
            );
            return Err(Error::ApiStatus { status, body });
        }

        Ok(body)
    }

    fn request(&self, request: RequestBuilder) -> Result<RequestBuilder> {
        let request = request
            .header(
                ORIGIN,
                HeaderValue::from_str(self.endpoints.site_origin.as_str())
                    .map_err(|_| Error::Protocol("invalid site origin configured for request"))?,
            )
            .header(
                REFERER,
                HeaderValue::from_str(&referer(&self.endpoints.site_origin))
                    .map_err(|_| Error::Protocol("invalid referer configured for request"))?,
            )
            .header(
                USER_AGENT,
                HeaderValue::from_str(&self.user_agent)
                    .map_err(|_| Error::Protocol("invalid user agent configured for request"))?,
            );

        let request = if let Some(session_id) = self.session_id.as_deref() {
            request.header(COOKIE, cookie_header_value(session_id)?)
        } else {
            request
        };

        Ok(request)
    }

    async fn acquire_http_slot(&self) -> Result<Option<OwnedSemaphorePermit>> {
        let permit = match self.request_budget_state.http_limiter.as_ref() {
            Some(limiter) => Some(
                limiter
                    .clone()
                    .acquire_owned()
                    .await
                    .map_err(|_| Error::Protocol("http request budget closed"))?,
            ),
            None => None,
        };

        if let (Some(pacer), Some(min_interval)) = (
            self.request_budget_state.http_pacer.as_ref(),
            self.request_budget.min_http_interval,
        ) {
            let mut next_allowed_at = pacer.lock().await;
            let now = TokioInstant::now();
            if *next_allowed_at > now {
                sleep_until(*next_allowed_at).await;
            }
            *next_allowed_at = TokioInstant::now() + min_interval;
        }

        Ok(permit)
    }

    pub(crate) async fn acquire_websocket_slot(&self) -> Result<Option<OwnedSemaphorePermit>> {
        match self.request_budget_state.websocket_limiter.as_ref() {
            Some(limiter) => limiter
                .clone()
                .acquire_owned()
                .await
                .map(Some)
                .map_err(|_| Error::Protocol("websocket request budget closed")),
            None => Ok(None),
        }
    }

    pub(crate) fn emit_event(&self, event: ClientEvent) {
        if let Some(observer) = self.observer.as_ref() {
            observer.on_event(&event);
        }
    }

    pub(crate) async fn connect_socket(&self) -> Result<TradingViewWebSocket> {
        let authenticated = self.session_id().is_some();
        let url = self.endpoints().websocket_url().to_string();
        let result = self
            .websocket_connector
            .connect(self.endpoints(), &self.user_agent, self.session_id())
            .await;

        match &result {
            Ok(_) => self.emit_event(ClientEvent::WebSocketConnected(WebSocketConnectedEvent {
                url,
                authenticated,
            })),
            Err(error) => {
                self.emit_event(ClientEvent::WebSocketConnectionFailed(
                    WebSocketConnectionFailedEvent {
                        url,
                        authenticated,
                        kind: error.kind(),
                    },
                ));
            }
        }

        result
    }

    async fn cached_metainfo(&self, market: &Market) -> Result<ScannerMetainfo> {
        if let Some(cached) = self
            .metainfo_cache
            .read()
            .await
            .get(market.as_str())
            .cloned()
        {
            #[cfg(feature = "tracing")]
            debug!(
                target: "tvdata_rs::metainfo",
                market = market.as_str(),
                "scanner metainfo cache hit",
            );
            return Ok(cached);
        }

        #[cfg(feature = "tracing")]
        debug!(
            target: "tvdata_rs::metainfo",
            market = market.as_str(),
            "scanner metainfo cache miss",
        );

        let metainfo: ScannerMetainfo = self
            .execute_json(
                self.request(self.http.get(self.endpoints.scanner_metainfo_url(market)?))?,
            )
            .await?;

        self.metainfo_cache
            .write()
            .await
            .insert(market.as_str().to_owned(), metainfo.clone());

        #[cfg(feature = "tracing")]
        debug!(
            target: "tvdata_rs::metainfo",
            market = market.as_str(),
            fields = metainfo.fields.len(),
            "scanner metainfo cached",
        );

        Ok(metainfo)
    }
}

fn validation_markets(query: &ScanQuery) -> Result<Vec<Market>> {
    if query.markets.is_empty() {
        return Err(Error::ScanValidationUnavailable {
            reason: "query does not specify any markets".to_owned(),
        });
    }

    Ok(query.markets.clone())
}

fn supports_column_for_market(market: &Market, metainfo: &ScannerMetainfo, column: &str) -> bool {
    metainfo.supports_field(column)
        || market_to_screener_kind(market)
            .and_then(|kind| embedded_registry().find_by_api_name(kind, column))
            .is_some()
}

fn market_to_screener_kind(market: &Market) -> Option<ScreenerKind> {
    match market.as_str() {
        "crypto" => Some(ScreenerKind::Crypto),
        "forex" => Some(ScreenerKind::Forex),
        "bond" | "bonds" => Some(ScreenerKind::Bond),
        "futures" => Some(ScreenerKind::Futures),
        "coin" => Some(ScreenerKind::Coin),
        "options" | "economics2" | "cfd" => None,
        _ => Some(ScreenerKind::Stock),
    }
}

#[cfg(test)]
mod tests;
