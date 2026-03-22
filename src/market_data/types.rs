#[cfg(feature = "equity")]
use std::collections::BTreeMap;
#[cfg(feature = "equity")]
use time::OffsetDateTime;

use crate::scanner::Ticker;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InstrumentIdentity {
    pub ticker: Ticker,
    pub name: Option<String>,
    pub market: Option<String>,
    pub exchange: Option<String>,
    pub currency: Option<String>,
    pub country: Option<String>,
    pub instrument_type: Option<String>,
    pub sector: Option<String>,
    pub industry: Option<String>,
}

#[cfg(any(feature = "crypto", feature = "equity", feature = "forex"))]
#[derive(Debug, Clone, PartialEq)]
pub struct QuoteSnapshot {
    pub instrument: InstrumentIdentity,
    pub open: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub close: Option<f64>,
    pub change_percent: Option<f64>,
    pub change_abs: Option<f64>,
    pub volume: Option<f64>,
    pub relative_volume: Option<f64>,
    pub market_cap: Option<f64>,
}

#[cfg(any(feature = "crypto", feature = "equity", feature = "forex"))]
#[derive(Debug, Clone, PartialEq)]
pub struct TechnicalSummary {
    pub instrument: InstrumentIdentity,
    pub close: Option<f64>,
    pub recommend_all: Option<f64>,
    pub recommend_ma: Option<f64>,
    pub recommend_other: Option<f64>,
    pub rsi: Option<f64>,
    pub rsi7: Option<f64>,
    pub macd: Option<f64>,
    pub macd_signal: Option<f64>,
    pub macd_histogram: Option<f64>,
    pub adx: Option<f64>,
    pub atr: Option<f64>,
    pub sma20: Option<f64>,
    pub sma50: Option<f64>,
    pub sma200: Option<f64>,
    pub ema20: Option<f64>,
    pub ema50: Option<f64>,
    pub ema200: Option<f64>,
    pub stoch_k: Option<f64>,
    pub stoch_d: Option<f64>,
    pub williams_r: Option<f64>,
    pub cci20: Option<f64>,
}

#[cfg(feature = "equity")]
#[derive(Debug, Clone, PartialEq)]
pub struct ConversionRatesSnapshot {
    pub effective_at: Option<OffsetDateTime>,
    pub rates: BTreeMap<String, f64>,
}
