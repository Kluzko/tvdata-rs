use std::borrow::Cow;
use std::fmt;

use serde::{Serialize, Serializer};

use crate::scanner::filter::{
    FilterCondition, FilterOperator, IntoFilterValue, SortOrder, SortSpec,
};

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Column(Cow<'static, str>);

impl Column {
    pub const fn from_static(name: &'static str) -> Self {
        Self(Cow::Borrowed(name))
    }

    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }

    pub fn with_interval(&self, interval: &str) -> Self {
        Self::new(format!("{}|{interval}", self.as_str()))
    }

    pub fn with_history(&self, periods: u16) -> Self {
        Self::new(format!("{}[{periods}]", self.as_str()))
    }

    pub fn recommendation(&self) -> Self {
        Self::new(format!("Rec.{}", self.as_str()))
    }

    pub fn gt(self, value: impl IntoFilterValue) -> FilterCondition {
        FilterCondition::new(self, FilterOperator::Greater, value.into_filter_value())
    }

    pub fn ge(self, value: impl IntoFilterValue) -> FilterCondition {
        FilterCondition::new(self, FilterOperator::EGreater, value.into_filter_value())
    }

    pub fn lt(self, value: impl IntoFilterValue) -> FilterCondition {
        FilterCondition::new(self, FilterOperator::Less, value.into_filter_value())
    }

    pub fn le(self, value: impl IntoFilterValue) -> FilterCondition {
        FilterCondition::new(self, FilterOperator::ELess, value.into_filter_value())
    }

    pub fn eq(self, value: impl IntoFilterValue) -> FilterCondition {
        FilterCondition::new(self, FilterOperator::Equal, value.into_filter_value())
    }

    pub fn ne(self, value: impl IntoFilterValue) -> FilterCondition {
        FilterCondition::new(self, FilterOperator::NotEqual, value.into_filter_value())
    }

    pub fn between(
        self,
        lower: impl IntoFilterValue,
        upper: impl IntoFilterValue,
    ) -> FilterCondition {
        FilterCondition::new(
            self,
            FilterOperator::InRange,
            vec![lower.into_filter_value(), upper.into_filter_value()].into_filter_value(),
        )
    }

    pub fn not_between(
        self,
        lower: impl IntoFilterValue,
        upper: impl IntoFilterValue,
    ) -> FilterCondition {
        FilterCondition::new(
            self,
            FilterOperator::NotInRange,
            vec![lower.into_filter_value(), upper.into_filter_value()].into_filter_value(),
        )
    }

    pub fn isin<I, V>(self, values: I) -> FilterCondition
    where
        I: IntoIterator<Item = V>,
        V: IntoFilterValue,
    {
        FilterCondition::new(
            self,
            FilterOperator::InRange,
            values
                .into_iter()
                .map(IntoFilterValue::into_filter_value)
                .collect::<Vec<_>>()
                .into_filter_value(),
        )
    }

    pub fn not_in<I, V>(self, values: I) -> FilterCondition
    where
        I: IntoIterator<Item = V>,
        V: IntoFilterValue,
    {
        FilterCondition::new(
            self,
            FilterOperator::NotInRange,
            values
                .into_iter()
                .map(IntoFilterValue::into_filter_value)
                .collect::<Vec<_>>()
                .into_filter_value(),
        )
    }

    pub fn crosses(self, value: impl IntoFilterValue) -> FilterCondition {
        FilterCondition::new(self, FilterOperator::Crosses, value.into_filter_value())
    }

    pub fn crosses_above(self, value: impl IntoFilterValue) -> FilterCondition {
        FilterCondition::new(
            self,
            FilterOperator::CrossesAbove,
            value.into_filter_value(),
        )
    }

    pub fn crosses_below(self, value: impl IntoFilterValue) -> FilterCondition {
        FilterCondition::new(
            self,
            FilterOperator::CrossesBelow,
            value.into_filter_value(),
        )
    }

    pub fn matches(self, value: impl IntoFilterValue) -> FilterCondition {
        FilterCondition::new(self, FilterOperator::Match, value.into_filter_value())
    }

    pub fn empty(self) -> FilterCondition {
        FilterCondition::new(self, FilterOperator::Empty, serde_json::Value::Null)
    }

    pub fn not_empty(self) -> FilterCondition {
        FilterCondition::new(self, FilterOperator::NotEmpty, serde_json::Value::Null)
    }

    pub fn above_pct(
        self,
        base: impl IntoFilterValue,
        pct: impl IntoFilterValue,
    ) -> FilterCondition {
        FilterCondition::new(
            self,
            FilterOperator::AbovePercent,
            vec![base.into_filter_value(), pct.into_filter_value()].into_filter_value(),
        )
    }

    pub fn below_pct(
        self,
        base: impl IntoFilterValue,
        pct: impl IntoFilterValue,
    ) -> FilterCondition {
        FilterCondition::new(
            self,
            FilterOperator::BelowPercent,
            vec![base.into_filter_value(), pct.into_filter_value()].into_filter_value(),
        )
    }

    pub fn sort(self, order: SortOrder) -> SortSpec {
        SortSpec::new(self, order)
    }
}

impl fmt::Display for Column {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for Column {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl From<&'static str> for Column {
    fn from(value: &'static str) -> Self {
        Self::from_static(value)
    }
}

impl From<String> for Column {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<&String> for Column {
    fn from(value: &String) -> Self {
        Self::new(value.clone())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct Market(Cow<'static, str>);

impl Market {
    pub const fn from_static(name: &'static str) -> Self {
        Self(Cow::Borrowed(name))
    }

    pub fn new(name: impl Into<Cow<'static, str>>) -> Self {
        Self(name.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}

impl Serialize for Market {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl From<&'static str> for Market {
    fn from(value: &'static str) -> Self {
        Self::from_static(value)
    }
}

impl From<String> for Market {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct InstrumentRef {
    pub exchange: String,
    pub symbol: String,
}

impl InstrumentRef {
    pub fn new(exchange: impl Into<String>, symbol: impl Into<String>) -> Self {
        Self {
            exchange: exchange.into().trim().to_ascii_uppercase(),
            symbol: symbol.into().trim().to_owned(),
        }
    }

    pub fn from_exchange_symbol(exchange: impl Into<String>, symbol: impl Into<String>) -> Self {
        let exchange = exchange.into().trim().to_ascii_uppercase();
        let symbol = normalize_symbol_for_exchange(&exchange, symbol.into().trim());
        Self { exchange, symbol }
    }

    pub fn from_internal_us_equity(exchange: impl Into<String>, symbol: impl Into<String>) -> Self {
        let exchange = exchange.into().trim().to_ascii_uppercase();
        let symbol = symbol.into().trim().to_ascii_uppercase().replace('-', ".");
        Self { exchange, symbol }
    }

    pub fn to_ticker(&self) -> Ticker {
        Ticker::from_parts(&self.exchange, &self.symbol)
    }
}

fn normalize_symbol_for_exchange(exchange: &str, symbol: &str) -> String {
    let uppercase = symbol.to_ascii_uppercase();

    match exchange {
        "FX" | "FX_IDC" | "FOREX" | "OANDA" | "FOREXCOM" => uppercase
            .chars()
            .filter(|ch| ch.is_ascii_alphanumeric())
            .collect(),
        "NYSE" | "NASDAQ" | "AMEX" | "ARCA" | "BATS" | "TSX" => uppercase.replace('-', "."),
        _ => uppercase,
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Ticker(Cow<'static, str>);

impl Ticker {
    pub fn from_parts(exchange: &str, symbol: &str) -> Self {
        Self(Cow::Owned(format!("{exchange}:{symbol}")))
    }

    pub fn from_exchange_symbol(exchange: &str, symbol: &str) -> Self {
        InstrumentRef::from_exchange_symbol(exchange, symbol).to_ticker()
    }

    pub const fn from_static(raw: &'static str) -> Self {
        Self(Cow::Borrowed(raw))
    }

    pub fn new(raw: impl Into<Cow<'static, str>>) -> Self {
        Self(raw.into())
    }

    pub fn as_str(&self) -> &str {
        self.0.as_ref()
    }
}

impl fmt::Display for Ticker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl Serialize for Ticker {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(self.as_str())
    }
}

impl From<&'static str> for Ticker {
    fn from(value: &'static str) -> Self {
        Self::from_static(value)
    }
}

impl From<String> for Ticker {
    fn from(value: String) -> Self {
        Self::new(value)
    }
}

impl From<InstrumentRef> for Ticker {
    fn from(value: InstrumentRef) -> Self {
        value.to_ticker()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_internal_us_equity_symbols() {
        let ticker = Ticker::from_exchange_symbol("NYSE", "BRK-B");
        assert_eq!(ticker.as_str(), "NYSE:BRK.B");
    }

    #[test]
    fn normalizes_forex_pairs() {
        let instrument = InstrumentRef::from_exchange_symbol("FX", "eur/usd");
        assert_eq!(instrument.to_ticker().as_str(), "FX:EURUSD");
    }
}
