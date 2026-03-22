#![forbid(unsafe_code)]
#![deny(missing_debug_implementations)]
#![doc = include_str!("../README.snippet.md")]

pub mod batch;
pub mod calendar;
pub mod client;
pub mod crypto;
pub mod economics;
pub mod equity;
pub mod error;
pub mod forex;
pub mod history;
mod market_data;
pub mod metadata;
pub mod scanner;
pub mod search;
pub mod time_series;
mod transport;

pub use batch::{BatchResult, SymbolFailure};
pub use calendar::{
    CalendarWindowRequest, DividendCalendarEntry, DividendCalendarRequest, DividendDateKind,
    EarningsCalendarEntry, IpoCalendarEntry,
};
pub use client::{Endpoints, HistoryClientConfig, RetryConfig, RetryJitter, TradingViewClient};
pub use crypto::{CryptoClient, CryptoOverview};
pub use economics::{
    EconomicCalendarRequest, EconomicCalendarResponse, EconomicEvent, EconomicValue,
};
pub use equity::{
    AnalystForecasts, AnalystFxRates, AnalystPriceTargets, AnalystRecommendations, AnalystSummary,
    EarningsCalendar, EarningsMetrics, EquityClient, EquityOverview, EstimateHistory,
    EstimateMetrics, EstimateObservation, FundamentalMetrics, FundamentalObservation,
    FundamentalsSnapshot, PointInTimeFundamentals,
};
pub use error::{Error, ErrorKind, Result};
pub use forex::{ForexClient, ForexOverview};
pub use history::{
    Adjustment, Bar, BarSelectionPolicy, DailyBarRangeRequest, DailyBarRequest,
    HistoryBatchRequest, HistoryProvenance, HistoryRequest, HistorySeries, Interval,
    TradingSession,
};
pub use market_data::{
    ConversionRatesSnapshot, InstrumentIdentity, QuoteSnapshot, TechnicalSummary,
};
pub use metadata::{DataLineage, DataSourceKind, HistoryKind};
pub use scanner::{
    HeuristicSymbolNormalizer, InstrumentRef, PartiallySupportedColumn, ScanValidationReport,
    ScannerFieldMetainfo, ScannerFieldType, ScannerMetainfo, SymbolNormalizer,
};
pub use search::{SearchAssetClass, SearchHit, SearchRequest, SearchResponse};
pub use time_series::{FiscalPeriod, HistoricalObservation};

pub mod prelude {
    pub use crate::batch::{BatchResult, SymbolFailure};
    pub use crate::calendar::{
        CalendarWindowRequest, DividendCalendarEntry, DividendCalendarRequest, DividendDateKind,
        EarningsCalendarEntry, IpoCalendarEntry,
    };
    pub use crate::client::{HistoryClientConfig, RetryConfig, RetryJitter, TradingViewClient};
    pub use crate::crypto::{CryptoClient, CryptoOverview};
    pub use crate::economics::{
        EconomicCalendarRequest, EconomicCalendarResponse, EconomicEvent, EconomicValue,
    };
    pub use crate::equity::{
        AnalystForecasts, AnalystFxRates, AnalystPriceTargets, AnalystRecommendations,
        AnalystSummary, EarningsCalendar, EarningsMetrics, EquityClient, EquityOverview,
        EstimateHistory, EstimateMetrics, EstimateObservation, FundamentalMetrics,
        FundamentalObservation, FundamentalsSnapshot, PointInTimeFundamentals,
    };
    pub use crate::forex::{ForexClient, ForexOverview};
    pub use crate::history::{
        Adjustment, Bar, BarSelectionPolicy, DailyBarRangeRequest, DailyBarRequest,
        HistoryBatchRequest, HistoryProvenance, HistoryRequest, HistorySeries, Interval,
        TradingSession,
    };
    pub use crate::market_data::{
        ConversionRatesSnapshot, InstrumentIdentity, QuoteSnapshot, TechnicalSummary,
    };
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
    pub use crate::search::{SearchAssetClass, SearchHit, SearchRequest, SearchResponse};
    pub use crate::time_series::{FiscalPeriod, HistoricalObservation};
    pub use crate::{ErrorKind, Result};
}
