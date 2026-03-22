use std::collections::BTreeMap;

use bon::Builder;
use time::OffsetDateTime;

use crate::scanner::Ticker;

pub(crate) fn default_history_batch_concurrency() -> usize {
    4
}

pub(crate) fn default_history_max_chunk_bars() -> u32 {
    5_000
}

#[derive(Debug, Clone, PartialEq, Eq, Builder)]
pub struct HistoryRequest {
    #[builder(into)]
    pub symbol: Ticker,
    pub interval: Interval,
    pub bars: u32,
    #[builder(default)]
    pub fetch_all: bool,
    #[builder(default)]
    pub session: TradingSession,
    #[builder(default)]
    pub adjustment: Adjustment,
}

impl HistoryRequest {
    pub fn new(symbol: impl Into<Ticker>, interval: Interval, bars: u32) -> Self {
        Self::builder()
            .symbol(symbol)
            .interval(interval)
            .bars(bars)
            .build()
    }

    pub fn max(symbol: impl Into<Ticker>, interval: Interval) -> Self {
        Self::builder()
            .symbol(symbol)
            .interval(interval)
            .bars(default_history_max_chunk_bars())
            .fetch_all(true)
            .build()
    }

    pub fn session(mut self, session: TradingSession) -> Self {
        self.session = session;
        self
    }

    pub fn adjustment(mut self, adjustment: Adjustment) -> Self {
        self.adjustment = adjustment;
        self
    }

    pub fn fetch_all(mut self) -> Self {
        self.fetch_all = true;
        self
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Builder)]
pub struct HistoryBatchRequest {
    pub symbols: Vec<Ticker>,
    pub interval: Interval,
    pub bars: u32,
    #[builder(default)]
    pub fetch_all: bool,
    #[builder(default)]
    pub session: TradingSession,
    #[builder(default)]
    pub adjustment: Adjustment,
    #[builder(default = default_history_batch_concurrency())]
    pub concurrency: usize,
}

impl HistoryBatchRequest {
    pub fn new<I, T>(symbols: I, interval: Interval, bars: u32) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<Ticker>,
    {
        Self {
            symbols: symbols.into_iter().map(Into::into).collect(),
            interval,
            bars,
            fetch_all: false,
            session: TradingSession::Regular,
            adjustment: Adjustment::Splits,
            concurrency: default_history_batch_concurrency(),
        }
    }

    pub fn max<I, T>(symbols: I, interval: Interval) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<Ticker>,
    {
        Self {
            symbols: symbols.into_iter().map(Into::into).collect(),
            interval,
            bars: default_history_max_chunk_bars(),
            fetch_all: true,
            session: TradingSession::Regular,
            adjustment: Adjustment::Splits,
            concurrency: default_history_batch_concurrency(),
        }
    }

    pub fn symbols<I, T>(mut self, symbols: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<Ticker>,
    {
        self.symbols = symbols.into_iter().map(Into::into).collect();
        self
    }

    pub fn push_symbol(mut self, symbol: impl Into<Ticker>) -> Self {
        self.symbols.push(symbol.into());
        self
    }

    pub fn fetch_all(mut self) -> Self {
        self.fetch_all = true;
        self
    }

    pub(crate) fn to_requests(&self) -> Vec<HistoryRequest> {
        self.symbols
            .iter()
            .cloned()
            .map(|symbol| HistoryRequest {
                symbol,
                interval: self.interval,
                bars: self.bars,
                fetch_all: self.fetch_all,
                session: self.session,
                adjustment: self.adjustment,
            })
            .collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Interval {
    Min1,
    Min3,
    Min5,
    Min15,
    Min30,
    Min45,
    Hour1,
    Hour2,
    Hour3,
    Hour4,
    Day1,
    Week1,
    Month1,
    Custom(&'static str),
}

impl Interval {
    pub fn as_code(self) -> &'static str {
        match self {
            Self::Min1 => "1",
            Self::Min3 => "3",
            Self::Min5 => "5",
            Self::Min15 => "15",
            Self::Min30 => "30",
            Self::Min45 => "45",
            Self::Hour1 => "1H",
            Self::Hour2 => "2H",
            Self::Hour3 => "3H",
            Self::Hour4 => "4H",
            Self::Day1 => "1D",
            Self::Week1 => "1W",
            Self::Month1 => "1M",
            Self::Custom(code) => code,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum TradingSession {
    #[default]
    Regular,
    Extended,
}

impl TradingSession {
    pub(crate) fn as_code(self) -> &'static str {
        match self {
            Self::Regular => "regular",
            Self::Extended => "extended",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Adjustment {
    #[default]
    Splits,
    None,
}

impl Adjustment {
    pub(crate) fn as_code(self) -> &'static str {
        match self {
            Self::Splits => "splits",
            Self::None => "none",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bar {
    pub time: OffsetDateTime,
    pub open: f64,
    pub high: f64,
    pub low: f64,
    pub close: f64,
    pub volume: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HistorySeries {
    pub symbol: Ticker,
    pub interval: Interval,
    pub bars: Vec<Bar>,
}

pub type HistorySeriesMap = BTreeMap<Ticker, HistorySeries>;
