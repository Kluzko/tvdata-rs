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

pub trait SymbolNormalizer {
    fn normalize(&self, instrument: &InstrumentRef) -> InstrumentRef;
}

#[derive(Debug, Clone, Copy, Default)]
pub struct HeuristicSymbolNormalizer;

impl HeuristicSymbolNormalizer {
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
}

impl SymbolNormalizer for HeuristicSymbolNormalizer {
    fn normalize(&self, instrument: &InstrumentRef) -> InstrumentRef {
        InstrumentRef {
            exchange: instrument.exchange.trim().to_ascii_uppercase(),
            symbol: Self::normalize_symbol_for_exchange(&instrument.exchange, &instrument.symbol),
        }
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

    /// Heuristic convenience constructor that normalizes common exchange-specific symbol shapes.
    ///
    /// Prefer [`InstrumentRef::new`] plus [`InstrumentRef::to_ticker`] when you want raw,
    /// non-opinionated construction.
    pub fn from_exchange_symbol(exchange: impl Into<String>, symbol: impl Into<String>) -> Self {
        Self::from_exchange_symbol_normalized(exchange, symbol)
    }

    pub fn from_exchange_symbol_normalized(
        exchange: impl Into<String>,
        symbol: impl Into<String>,
    ) -> Self {
        Self::new(exchange, symbol).normalized_with(&HeuristicSymbolNormalizer)
    }

    pub fn from_internal_us_equity(exchange: impl Into<String>, symbol: impl Into<String>) -> Self {
        let exchange = exchange.into().trim().to_ascii_uppercase();
        let symbol = symbol.into().trim().to_ascii_uppercase().replace('-', ".");
        Self { exchange, symbol }
    }

    pub fn to_ticker(&self) -> Ticker {
        Ticker::from_parts(&self.exchange, &self.symbol)
    }

    pub fn normalized_with<N>(&self, normalizer: &N) -> Self
    where
        N: SymbolNormalizer + ?Sized,
    {
        normalizer.normalize(self)
    }

    pub fn to_ticker_with<N>(&self, normalizer: &N) -> Ticker
    where
        N: SymbolNormalizer + ?Sized,
    {
        self.normalized_with(normalizer).to_ticker()
    }

    pub fn to_normalized_ticker(&self) -> Ticker {
        self.to_ticker_with(&HeuristicSymbolNormalizer)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Ord, PartialOrd)]
pub struct Ticker(Cow<'static, str>);

impl Ticker {
    pub fn from_parts(exchange: &str, symbol: &str) -> Self {
        Self(Cow::Owned(format!("{exchange}:{symbol}")))
    }

    /// Heuristic convenience constructor that normalizes common exchange-specific symbol shapes.
    pub fn from_exchange_symbol(exchange: &str, symbol: &str) -> Self {
        Self::from_exchange_symbol_normalized(exchange, symbol)
    }

    pub fn from_exchange_symbol_normalized(exchange: &str, symbol: &str) -> Self {
        InstrumentRef::from_exchange_symbol_normalized(exchange, symbol).to_ticker()
    }

    pub fn from_exchange_symbol_with<N>(exchange: &str, symbol: &str, normalizer: &N) -> Self
    where
        N: SymbolNormalizer + ?Sized,
    {
        InstrumentRef::new(exchange, symbol).to_ticker_with(normalizer)
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

    pub fn split(&self) -> Option<(&str, &str)> {
        self.as_str().split_once(':')
    }

    pub fn exchange(&self) -> Option<&str> {
        self.split().map(|(exchange, _)| exchange)
    }

    pub fn symbol(&self) -> Option<&str> {
        self.split().map(|(_, symbol)| symbol)
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

    #[derive(Debug, Clone, Copy)]
    struct CustomNormalizer;

    impl SymbolNormalizer for CustomNormalizer {
        fn normalize(&self, instrument: &InstrumentRef) -> InstrumentRef {
            InstrumentRef::new(&instrument.exchange, instrument.symbol.replace('/', "-"))
        }
    }

    #[test]
    fn raw_ticker_construction_preserves_symbol_shape() {
        let instrument = InstrumentRef::new("NYSE", "BRK-B");
        assert_eq!(instrument.to_ticker().as_str(), "NYSE:BRK-B");
    }

    #[test]
    fn heuristic_normalizer_handles_common_us_equity_symbols() {
        let ticker = InstrumentRef::new("NYSE", "BRK-B").to_normalized_ticker();
        assert_eq!(ticker.as_str(), "NYSE:BRK.B");
    }

    #[test]
    fn heuristic_normalizer_handles_common_forex_pairs() {
        let instrument = InstrumentRef::new("FX", "eur/usd");
        assert_eq!(instrument.to_normalized_ticker().as_str(), "FX:EURUSD");
    }

    #[test]
    fn custom_normalizer_can_override_symbol_rules() {
        let ticker = Ticker::from_exchange_symbol_with("FX", "eur/usd", &CustomNormalizer);
        assert_eq!(ticker.as_str(), "FX:eur-usd");
    }
}
