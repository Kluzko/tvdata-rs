#[cfg(feature = "equity")]
use std::collections::BTreeMap;
use std::collections::HashMap;

use serde_json::Value;
use time::OffsetDateTime;

use crate::scanner::ScanRow;
use crate::scanner::Ticker;
use crate::scanner::fields::core;
#[cfg(any(feature = "crypto", feature = "equity", feature = "forex"))]
use crate::scanner::fields::{fundamentals, price, technical};

#[cfg(feature = "equity")]
use super::types::ConversionRatesSnapshot;
use super::types::InstrumentIdentity;
#[cfg(any(feature = "crypto", feature = "equity", feature = "forex"))]
use super::types::{QuoteSnapshot, TechnicalSummary};

#[derive(Debug, Clone)]
pub(crate) struct RowDecoder {
    index: HashMap<String, usize>,
}

impl RowDecoder {
    pub(crate) fn new(columns: &[crate::scanner::Column]) -> Self {
        let index = columns
            .iter()
            .enumerate()
            .map(|(position, column)| (column.as_str().to_owned(), position))
            .collect::<HashMap<_, _>>();
        Self { index }
    }

    pub(crate) fn value<'a>(&self, row: &'a ScanRow, column: &str) -> Option<&'a Value> {
        self.index
            .get(column)
            .and_then(|position| row.values.get(*position))
            .filter(|value| !value.is_null())
    }

    pub(crate) fn string(&self, row: &ScanRow, column: &str) -> Option<String> {
        match self.value(row, column)? {
            Value::String(value) => Some(value.clone()),
            Value::Number(value) => Some(value.to_string()),
            Value::Bool(value) => Some(value.to_string()),
            _ => None,
        }
    }

    pub(crate) fn number(&self, row: &ScanRow, column: &str) -> Option<f64> {
        match self.value(row, column)? {
            Value::Number(value) => value.as_f64(),
            Value::String(value) => value.parse::<f64>().ok(),
            _ => None,
        }
    }

    pub(crate) fn timestamp(&self, row: &ScanRow, column: &str) -> Option<OffsetDateTime> {
        Self::timestamp_value(self.value(row, column)?)
    }

    pub(crate) fn whole_number(&self, row: &ScanRow, column: &str) -> Option<u32> {
        let value = self.number(row, column)?;
        (value >= 0.0 && value.fract() == 0.0).then_some(value as u32)
    }

    #[cfg(feature = "equity")]
    pub(crate) fn conversion_rates(
        &self,
        row: &ScanRow,
        column: &str,
    ) -> Option<ConversionRatesSnapshot> {
        let Value::Object(object) = self.value(row, column)? else {
            return None;
        };

        let effective_at = object.get("time").and_then(Self::timestamp_value);
        let rates = object
            .iter()
            .filter_map(|(key, value)| {
                (key != "time")
                    .then(|| Self::number_value(value).map(|rate| (key.clone(), rate)))
                    .flatten()
            })
            .collect::<BTreeMap<_, _>>();

        Some(ConversionRatesSnapshot {
            effective_at,
            rates,
        })
    }

    pub(crate) fn identity(&self, row: &ScanRow) -> InstrumentIdentity {
        InstrumentIdentity {
            ticker: Ticker::new(row.symbol.clone()),
            name: self.string(row, core::NAME.as_str()),
            market: self.string(row, core::MARKET.as_str()),
            exchange: self.string(row, core::EXCHANGE.as_str()),
            currency: self.string(row, core::CURRENCY.as_str()),
            country: self.string(row, core::COUNTRY.as_str()),
            instrument_type: self.string(row, core::TYPE.as_str()),
            sector: self.string(row, core::SECTOR.as_str()),
            industry: self.string(row, core::INDUSTRY.as_str()),
        }
    }

    fn timestamp_value(value: &Value) -> Option<OffsetDateTime> {
        let timestamp = match value {
            Value::Number(number) => number.as_i64().or_else(|| {
                number
                    .as_f64()
                    .filter(|timestamp| timestamp.fract() == 0.0)
                    .map(|timestamp| timestamp as i64)
            }),
            Value::String(value) => value.parse::<i64>().ok(),
            _ => None,
        }?;

        OffsetDateTime::from_unix_timestamp(timestamp).ok()
    }

    #[cfg(feature = "equity")]
    fn number_value(value: &Value) -> Option<f64> {
        match value {
            Value::Number(number) => number.as_f64(),
            Value::String(value) => value.parse::<f64>().ok(),
            _ => None,
        }
    }
}

#[cfg(any(feature = "crypto", feature = "equity", feature = "forex"))]
pub(crate) fn decode_quote(decoder: &RowDecoder, row: &ScanRow) -> QuoteSnapshot {
    QuoteSnapshot {
        instrument: decoder.identity(row),
        open: decoder.number(row, price::OPEN.as_str()),
        high: decoder.number(row, price::HIGH.as_str()),
        low: decoder.number(row, price::LOW.as_str()),
        close: decoder.number(row, price::CLOSE.as_str()),
        change_percent: decoder.number(row, price::CHANGE_PERCENT.as_str()),
        change_abs: decoder.number(row, price::CHANGE_ABS.as_str()),
        volume: decoder.number(row, price::VOLUME.as_str()),
        relative_volume: decoder.number(row, price::RELATIVE_VOLUME.as_str()),
        market_cap: decoder.number(row, fundamentals::MARKET_CAP_BASIC.as_str()),
    }
}

#[cfg(any(feature = "crypto", feature = "equity", feature = "forex"))]
pub(crate) fn decode_technical(decoder: &RowDecoder, row: &ScanRow) -> TechnicalSummary {
    TechnicalSummary {
        instrument: decoder.identity(row),
        close: decoder.number(row, price::CLOSE.as_str()),
        recommend_all: decoder.number(row, technical::RECOMMEND_ALL.as_str()),
        recommend_ma: decoder.number(row, technical::RECOMMEND_MA.as_str()),
        recommend_other: decoder.number(row, technical::RECOMMEND_OTHER.as_str()),
        rsi: decoder.number(row, technical::RSI.as_str()),
        rsi7: decoder.number(row, technical::RSI7.as_str()),
        macd: decoder.number(row, technical::MACD.as_str()),
        macd_signal: decoder.number(row, technical::MACD_SIGNAL.as_str()),
        macd_histogram: decoder.number(row, technical::MACD_HISTOGRAM.as_str()),
        adx: decoder.number(row, technical::ADX.as_str()),
        atr: decoder.number(row, technical::ATR.as_str()),
        sma20: decoder.number(row, technical::SMA20.as_str()),
        sma50: decoder.number(row, technical::SMA50.as_str()),
        sma200: decoder.number(row, technical::SMA200.as_str()),
        ema20: decoder.number(row, technical::EMA20.as_str()),
        ema50: decoder.number(row, technical::EMA50.as_str()),
        ema200: decoder.number(row, technical::EMA200.as_str()),
        stoch_k: decoder.number(row, technical::STOCH_K.as_str()),
        stoch_d: decoder.number(row, technical::STOCH_D.as_str()),
        williams_r: decoder.number(row, technical::WILLIAMS_R.as_str()),
        cci20: decoder.number(row, technical::CCI20.as_str()),
    }
}
