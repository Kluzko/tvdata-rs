#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]
#![doc = include_str!("../README.snippet.md")]

pub mod batch;
pub mod client;
pub mod error;
pub mod history;
pub mod metadata;
pub mod scanner;
pub mod time_series;
mod transport;

#[cfg(feature = "calendar")]
pub mod calendar;
#[cfg(feature = "crypto")]
pub mod crypto;
#[cfg(feature = "economics")]
pub mod economics;
#[cfg(feature = "equity")]
pub mod equity;
#[cfg(feature = "forex")]
pub mod forex;
#[cfg(any(
    feature = "calendar",
    feature = "crypto",
    feature = "equity",
    feature = "forex"
))]
mod market_data;
#[cfg(feature = "search")]
pub mod search;

pub use batch::{BatchResult, SymbolFailure};
#[cfg(feature = "calendar")]
pub use calendar::{
    CalendarWindowRequest, DividendCalendarEntry, DividendCalendarRequest, DividendDateKind,
    EarningsCalendarEntry, IpoCalendarEntry,
};
pub use client::{
    AuthConfig, AuthMode, ClientEvent, ClientObserver, DefaultWebSocketConnector, Endpoints,
    HistoryBatchCompletedEvent, HistoryBatchMode, HistoryClientConfig, HttpRequestCompletedEvent,
    HttpRequestFailedEvent, RequestBudget, RetryConfig, RetryJitter, SnapshotBatchConfig,
    SnapshotBatchStrategy, TradingViewClient, TradingViewClientConfig, TransportConfig,
    WebSocketConnectFuture, WebSocketConnectedEvent, WebSocketConnectionFailedEvent,
    WebSocketConnector,
};
#[cfg(feature = "crypto")]
pub use crypto::{CryptoClient, CryptoOverview};
#[cfg(feature = "economics")]
pub use economics::{
    EconomicCalendarRequest, EconomicCalendarResponse, EconomicEvent, EconomicValue,
};
#[cfg(feature = "equity")]
pub use equity::{
    AnalystForecasts, AnalystFxRates, AnalystPriceTargets, AnalystRecommendations, AnalystSummary,
    EarningsCalendar, EarningsMetrics, EquityClient, EquityOverview, EstimateHistory,
    EstimateMetrics, EstimateObservation, FundamentalMetrics, FundamentalObservation,
    FundamentalsSnapshot, PointInTimeFundamentals,
};
pub use error::{Error, ErrorKind, Result};
#[cfg(feature = "forex")]
pub use forex::{ForexClient, ForexOverview};
pub use history::{
    Adjustment, Bar, BarSelectionPolicy, DailyBarRangeRequest, DailyBarRequest,
    HistoryBatchRequest, HistoryProvenance, HistoryRequest, HistorySeries, Interval,
    TradingSession,
};
#[cfg(feature = "equity")]
pub use market_data::ConversionRatesSnapshot;
#[cfg(any(
    feature = "calendar",
    feature = "crypto",
    feature = "equity",
    feature = "forex"
))]
pub use market_data::InstrumentIdentity;
#[cfg(any(feature = "crypto", feature = "equity", feature = "forex"))]
pub use market_data::{QuoteSnapshot, TechnicalSummary};
pub use metadata::{DataLineage, DataSourceKind, HistoryKind};
pub use scanner::{
    HeuristicSymbolNormalizer, InstrumentRef, PartiallySupportedColumn, ScanValidationReport,
    ScannerFieldMetainfo, ScannerFieldType, ScannerMetainfo, SymbolNormalizer,
};
#[cfg(feature = "search")]
pub use search::{SearchAssetClass, SearchHit, SearchRequest, SearchResponse};
pub use time_series::{FiscalPeriod, HistoricalObservation};
pub use transport::websocket::TradingViewWebSocket;

pub mod prelude {
    pub use crate::batch::{BatchResult, SymbolFailure};
    #[cfg(feature = "calendar")]
    pub use crate::calendar::{
        CalendarWindowRequest, DividendCalendarEntry, DividendCalendarRequest, DividendDateKind,
        EarningsCalendarEntry, IpoCalendarEntry,
    };
    pub use crate::client::{
        AuthConfig, AuthMode, ClientEvent, ClientObserver, DefaultWebSocketConnector,
        HistoryBatchCompletedEvent, HistoryBatchMode, HistoryClientConfig,
        HttpRequestCompletedEvent, HttpRequestFailedEvent, RequestBudget, RetryConfig, RetryJitter,
        SnapshotBatchConfig, SnapshotBatchStrategy, TradingViewClient, TradingViewClientConfig,
        TransportConfig, WebSocketConnectFuture, WebSocketConnectedEvent,
        WebSocketConnectionFailedEvent, WebSocketConnector,
    };
    #[cfg(feature = "crypto")]
    pub use crate::crypto::{CryptoClient, CryptoOverview};
    #[cfg(feature = "economics")]
    pub use crate::economics::{
        EconomicCalendarRequest, EconomicCalendarResponse, EconomicEvent, EconomicValue,
    };
    #[cfg(feature = "equity")]
    pub use crate::equity::{
        AnalystForecasts, AnalystFxRates, AnalystPriceTargets, AnalystRecommendations,
        AnalystSummary, EarningsCalendar, EarningsMetrics, EquityClient, EquityOverview,
        EstimateHistory, EstimateMetrics, EstimateObservation, FundamentalMetrics,
        FundamentalObservation, FundamentalsSnapshot, PointInTimeFundamentals,
    };
    #[cfg(feature = "forex")]
    pub use crate::forex::{ForexClient, ForexOverview};
    pub use crate::history::{
        Adjustment, Bar, BarSelectionPolicy, DailyBarRangeRequest, DailyBarRequest,
        HistoryBatchRequest, HistoryProvenance, HistoryRequest, HistorySeries, Interval,
        TradingSession,
    };
    #[cfg(feature = "equity")]
    pub use crate::market_data::ConversionRatesSnapshot;
    #[cfg(any(
        feature = "calendar",
        feature = "crypto",
        feature = "equity",
        feature = "forex"
    ))]
    pub use crate::market_data::InstrumentIdentity;
    #[cfg(any(feature = "crypto", feature = "equity", feature = "forex"))]
    pub use crate::market_data::{QuoteSnapshot, TechnicalSummary};
    pub use crate::metadata::{DataLineage, DataSourceKind, HistoryKind};
    pub use crate::scanner::fields;
    pub use crate::scanner::{
        Column, FieldRegistry, FilterCondition, FilterOperator, FilterTree,
        HeuristicSymbolNormalizer, IndexSymbolDescriptor, InstrumentRef, LogicalOperator, Market,
        MarketDescriptor, Page, PartiallySupportedColumn, PriceConversion, ScanQuery, ScanResponse,
        ScanRow, ScanValidationReport, ScannerFieldMetainfo, ScannerFieldType, ScannerMetainfo,
        ScreenerKind, SortOrder, SortSpec, SymbolGroup, SymbolNormalizer, Symbols, Ticker,
        embedded_registry,
    };
    #[cfg(feature = "search")]
    pub use crate::search::{SearchAssetClass, SearchHit, SearchRequest, SearchResponse};
    pub use crate::time_series::{FiscalPeriod, HistoricalObservation};
    pub use crate::{ErrorKind, Result, TradingViewWebSocket};
}
