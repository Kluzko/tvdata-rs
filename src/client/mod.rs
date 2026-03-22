use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Duration;

use bon::{Builder, bon};
use reqwest::header::{COOKIE, HeaderMap, HeaderValue, ORIGIN, REFERER};
use reqwest_middleware::{
    ClientBuilder as MiddlewareClientBuilder, ClientWithMiddleware, RequestBuilder,
};
use reqwest_retry::{Jitter, RetryTransientMiddleware, policies::ExponentialBackoff};
use serde::de::DeserializeOwned;
use tokio::sync::RwLock;
use url::Url;

use crate::calendar::{
    CalendarWindowRequest, DividendCalendarEntry, DividendCalendarRequest, EarningsCalendarEntry,
    IpoCalendarEntry,
};
use crate::economics::{
    EconomicCalendarRequest, EconomicCalendarResponse, RawEconomicCalendarResponse,
    sanitize_calendar,
};
use crate::error::{Error, Result};
use crate::history::{HistoryRequest, HistorySeries, fetch_history};
use crate::scanner::{
    Market, PartiallySupportedColumn, RawScanResponse, ScanQuery, ScanResponse,
    ScanValidationReport, ScannerMetainfo, ScreenerKind, embedded_registry,
};
use crate::search::{
    RawSearchResponse, SearchHit, SearchRequest, SearchResponse, sanitize_response,
};

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

fn default_user_agent() -> String {
    DEFAULT_USER_AGENT.to_owned()
}

fn default_auth_token() -> String {
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
        #[builder(default = default_user_agent(), into)] user_agent: String,
        #[builder(default = default_auth_token(), into)] auth_token: String,
        #[builder(into)] session_id: Option<String>,
    ) -> Result<Self> {
        retry.validate()?;

        let mut headers = HeaderMap::new();
        headers.insert(
            ORIGIN,
            HeaderValue::from_str(endpoints.site_origin.as_str()).map_err(|_| {
                Error::Protocol("invalid site origin configured for reqwest client")
            })?,
        );
        headers.insert(
            REFERER,
            HeaderValue::from_str(&referer(&endpoints.site_origin))
                .map_err(|_| Error::Protocol("invalid referer configured for reqwest client"))?,
        );
        if let Some(session_id) = session_id.as_deref() {
            headers.insert(COOKIE, cookie_header_value(session_id)?);
        }

        let base_http = reqwest::Client::builder()
            .default_headers(headers)
            .timeout(timeout)
            .user_agent(&user_agent)
            .build()
            .map_err(Error::from)?;

        let http = if retry.max_retries == 0 {
            ClientWithMiddleware::from(base_http)
        } else {
            MiddlewareClientBuilder::new(base_http)
                .with(RetryTransientMiddleware::new_with_policy(retry.to_policy()))
                .build()
        };

        Ok(Self {
            http,
            endpoints,
            user_agent,
            auth_token,
            session_id,
            metainfo_cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    pub fn endpoints(&self) -> &Endpoints {
        &self.endpoints
    }

    pub(crate) fn user_agent(&self) -> &str {
        &self.user_agent
    }

    pub(crate) fn auth_token(&self) -> &str {
        &self.auth_token
    }

    pub(crate) fn session_id(&self) -> Option<&str> {
        self.session_id.as_deref()
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
        let raw: RawScanResponse = self
            .execute_json(
                self.http
                    .post(self.endpoints.scanner_url(&query.route_segment())?)
                    .json(query),
            )
            .await?;

        raw.into_response()
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

        Ok(ScanValidationReport {
            route_segment,
            requested_markets: markets,
            supported_columns,
            partially_supported_columns,
            unsupported_columns,
        })
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
    pub async fn search(&self, request: &SearchRequest) -> Result<Vec<SearchHit>> {
        Ok(self.search_response(request).await?.hits)
    }

    /// Searches equities using TradingView's current `search_type=stock` filter.
    pub async fn search_equities(&self, text: impl Into<String>) -> Result<Vec<SearchHit>> {
        Ok(self.search_equities_response(text).await?.hits)
    }

    /// Searches equities and returns the richer v3 response envelope.
    pub async fn search_equities_response(
        &self,
        text: impl Into<String>,
    ) -> Result<SearchResponse> {
        self.search_response(&SearchRequest::equities(text)).await
    }

    /// Searches forex instruments using TradingView's current `search_type=forex` filter.
    pub async fn search_forex(&self, text: impl Into<String>) -> Result<Vec<SearchHit>> {
        Ok(self.search_forex_response(text).await?.hits)
    }

    /// Searches forex instruments and returns the richer v3 response envelope.
    pub async fn search_forex_response(&self, text: impl Into<String>) -> Result<SearchResponse> {
        self.search_response(&SearchRequest::forex(text)).await
    }

    /// Searches crypto instruments using TradingView's current `search_type=crypto` filter.
    pub async fn search_crypto(&self, text: impl Into<String>) -> Result<Vec<SearchHit>> {
        Ok(self.search_crypto_response(text).await?.hits)
    }

    /// Searches crypto instruments and returns the richer v3 response envelope.
    pub async fn search_crypto_response(&self, text: impl Into<String>) -> Result<SearchResponse> {
        self.search_response(&SearchRequest::crypto(text)).await
    }

    /// Searches option-like instruments.
    ///
    /// As of March 22, 2026, TradingView's live `symbol_search/v3` endpoint rejects
    /// `search_type=option`, so this method performs a broader search and then keeps
    /// hits that look option-related based on the returned payload.
    pub async fn search_options(&self, text: impl Into<String>) -> Result<Vec<SearchHit>> {
        Ok(self.search_options_response(text).await?.hits)
    }

    /// Searches option-like instruments and returns the filtered v3 response envelope.
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
    pub async fn search_response(&self, request: &SearchRequest) -> Result<SearchResponse> {
        if request.text.trim().is_empty() {
            return Err(Error::EmptySearchQuery);
        }

        let raw: RawSearchResponse = self
            .execute_json(
                self.http
                    .get(self.endpoints.symbol_search_base_url.clone())
                    .query(&request.to_query_pairs()),
            )
            .await?;

        Ok(sanitize_response(raw))
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
    pub async fn economic_calendar(
        &self,
        request: &EconomicCalendarRequest,
    ) -> Result<EconomicCalendarResponse> {
        let raw: RawEconomicCalendarResponse = self
            .execute_json(
                self.http
                    .get(self.endpoints.calendar_base_url().clone())
                    .query(&request.to_query_pairs()?),
            )
            .await?;

        Ok(sanitize_calendar(raw))
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
        fetch_history(
            &self.endpoints,
            &self.auth_token,
            &self.user_agent,
            self.session_id(),
            request,
        )
        .await
    }

    async fn execute_json<T>(&self, request: RequestBuilder) -> Result<T>
    where
        T: DeserializeOwned,
    {
        let body = self.execute_text(request).await?;
        serde_json::from_str(&body).map_err(Into::into)
    }

    async fn execute_text(&self, request: RequestBuilder) -> Result<String> {
        let response = request.send().await?;
        let status = response.status();
        let body = response.text().await?;

        if !status.is_success() {
            return Err(Error::ApiStatus { status, body });
        }

        Ok(body)
    }

    async fn cached_metainfo(&self, market: &Market) -> Result<ScannerMetainfo> {
        if let Some(cached) = self
            .metainfo_cache
            .read()
            .await
            .get(market.as_str())
            .cloned()
        {
            return Ok(cached);
        }

        let metainfo: ScannerMetainfo = self
            .execute_json(self.http.get(self.endpoints.scanner_metainfo_url(market)?))
            .await?;

        self.metainfo_cache
            .write()
            .await
            .insert(market.as_str().to_owned(), metainfo.clone());

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
