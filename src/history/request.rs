use std::collections::BTreeMap;

use bon::Builder;
use time::{Date, OffsetDateTime};

use crate::metadata::DataLineage;
use crate::scanner::{InstrumentRef, Ticker};

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

    pub fn session(mut self, session: TradingSession) -> Self {
        self.session = session;
        self
    }

    pub fn adjustment(mut self, adjustment: Adjustment) -> Self {
        self.adjustment = adjustment;
        self
    }

    pub fn concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency;
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
    pub provenance: HistoryProvenance,
}

impl HistorySeries {
    pub fn latest(&self) -> Option<&Bar> {
        self.bars.last()
    }

    pub fn bar_on(&self, date: Date) -> Option<&Bar> {
        self.bars.iter().find(|bar| bar.time.date() == date)
    }

    pub fn latest_on_or_before(&self, date: Date) -> Option<&Bar> {
        self.bars.iter().rev().find(|bar| bar.time.date() <= date)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BarSelectionPolicy {
    ExactDate,
    #[default]
    LatestOnOrBefore,
}

#[derive(Debug, Clone, PartialEq, Eq, Builder)]
pub struct DailyBarRequest {
    pub symbols: Vec<InstrumentRef>,
    pub asof: Date,
    #[builder(default)]
    pub adjustment: Adjustment,
    #[builder(default)]
    pub session: TradingSession,
    #[builder(default)]
    pub selection: BarSelectionPolicy,
    #[builder(default = default_history_batch_concurrency())]
    pub concurrency: usize,
}

impl DailyBarRequest {
    pub fn new<I>(symbols: I, asof: Date) -> Self
    where
        I: IntoIterator<Item = InstrumentRef>,
    {
        Self {
            symbols: symbols.into_iter().collect(),
            asof,
            adjustment: Adjustment::default(),
            session: TradingSession::default(),
            selection: BarSelectionPolicy::default(),
            concurrency: default_history_batch_concurrency(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Builder)]
pub struct DailyBarRangeRequest {
    pub symbols: Vec<InstrumentRef>,
    pub start: Date,
    pub end: Date,
    #[builder(default)]
    pub adjustment: Adjustment,
    #[builder(default)]
    pub session: TradingSession,
    #[builder(default = default_history_batch_concurrency())]
    pub concurrency: usize,
}

impl DailyBarRangeRequest {
    pub fn new<I>(symbols: I, start: Date, end: Date) -> Self
    where
        I: IntoIterator<Item = InstrumentRef>,
    {
        Self {
            symbols: symbols.into_iter().collect(),
            start,
            end,
            adjustment: Adjustment::default(),
            session: TradingSession::default(),
            concurrency: default_history_batch_concurrency(),
        }
    }
}

pub type HistorySeriesMap = BTreeMap<Ticker, HistorySeries>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryProvenance {
    pub requested_symbol: Ticker,
    pub resolved_symbol: Ticker,
    pub exchange: Option<String>,
    pub session: TradingSession,
    pub adjustment: Adjustment,
    pub authenticated: bool,
    pub lineage: DataLineage,
}

#[cfg(test)]
mod tests {
    use time::macros::datetime;

    use super::*;
    use crate::metadata::{DataSourceKind, HistoryKind};

    #[test]
    fn selects_bars_by_date() {
        let series = HistorySeries {
            symbol: Ticker::from_static("NASDAQ:AAPL"),
            interval: Interval::Day1,
            bars: vec![
                Bar {
                    time: datetime!(2026-03-18 00:00 UTC),
                    open: 1.0,
                    high: 2.0,
                    low: 0.5,
                    close: 1.5,
                    volume: Some(10.0),
                },
                Bar {
                    time: datetime!(2026-03-20 00:00 UTC),
                    open: 2.0,
                    high: 3.0,
                    low: 1.5,
                    close: 2.5,
                    volume: Some(12.0),
                },
            ],
            provenance: HistoryProvenance {
                requested_symbol: Ticker::from_static("NASDAQ:AAPL"),
                resolved_symbol: Ticker::from_static("NASDAQ:AAPL"),
                exchange: Some("NASDAQ".to_owned()),
                session: TradingSession::Regular,
                adjustment: Adjustment::Splits,
                authenticated: false,
                lineage: DataLineage::new(
                    DataSourceKind::HistoryWebSocket,
                    HistoryKind::Native,
                    datetime!(2026-03-22 00:00 UTC),
                    Some(datetime!(2026-03-20 00:00 UTC)),
                ),
            },
        };

        assert_eq!(
            series
                .bar_on(datetime!(2026-03-18 00:00 UTC).date())
                .unwrap()
                .close,
            1.5
        );
        assert_eq!(
            series
                .latest_on_or_before(datetime!(2026-03-19 00:00 UTC).date())
                .unwrap()
                .close,
            1.5
        );
        assert_eq!(series.latest().unwrap().close, 2.5);
    }
}
