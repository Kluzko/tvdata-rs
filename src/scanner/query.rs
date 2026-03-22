use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::Value;

use crate::error::{Error, Result as TvResult};
use crate::scanner::field::{Column, Market, Ticker};
use crate::scanner::fields::{core, fundamentals, price};
use crate::scanner::filter::{FilterCondition, FilterTree, SortSpec};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SymbolGroup {
    #[serde(rename = "type")]
    pub kind: String,
    pub values: Vec<String>,
}

impl SymbolGroup {
    pub fn new(
        kind: impl Into<String>,
        values: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self {
        Self {
            kind: kind.into(),
            values: values.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Watchlist {
    pub id: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct SymbolQuery {
    pub types: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Default)]
pub struct Symbols {
    pub query: SymbolQuery,
    pub tickers: Vec<Ticker>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub symbolset: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub watchlist: Option<Watchlist>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub groups: Vec<SymbolGroup>,
}

impl Symbols {
    pub fn with_tickers<I, T>(mut self, tickers: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<Ticker>,
    {
        self.tickers = tickers.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_symbolset<I, S>(mut self, symbolset: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.symbolset = symbolset.into_iter().map(Into::into).collect();
        self
    }

    pub fn with_watchlist(mut self, id: i64) -> Self {
        self.watchlist = Some(Watchlist { id });
        self
    }

    pub fn with_group(mut self, group: SymbolGroup) -> Self {
        self.groups.push(group);
        self
    }

    pub fn with_types<I, S>(mut self, types: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.query.types = types.into_iter().map(Into::into).collect();
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct Page([usize; 2]);

impl Page {
    pub fn new(offset: usize, limit: usize) -> TvResult<Self> {
        if limit == 0 {
            return Err(Error::InvalidPageLimit);
        }
        Ok(Self([offset, limit]))
    }
}

impl Default for Page {
    fn default() -> Self {
        Self([0, 50])
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PriceConversion {
    SymbolCurrency,
    MarketCurrency,
    Specific(String),
}

impl Serialize for PriceConversion {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use serde::ser::SerializeMap;

        let mut map = serializer.serialize_map(Some(1))?;
        match self {
            Self::SymbolCurrency => map.serialize_entry("to_symbol", &true)?,
            Self::MarketCurrency => map.serialize_entry("to_symbol", &false)?,
            Self::Specific(currency) => {
                map.serialize_entry("to_currency", &currency.to_lowercase())?
            }
        }
        map.end()
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScanQuery {
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub markets: Vec<Market>,
    pub symbols: Symbols,
    #[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
    pub options: BTreeMap<String, Value>,
    pub columns: Vec<Column>,
    #[serde(rename = "filter", skip_serializing_if = "Vec::is_empty", default)]
    pub filters: Vec<FilterCondition>,
    #[serde(rename = "filter2", skip_serializing_if = "Option::is_none")]
    pub filter_tree: Option<FilterTree>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sort: Option<SortSpec>,
    #[serde(rename = "range")]
    pub page: Page,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ignore_unknown_fields: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preset: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub price_conversion: Option<PriceConversion>,
}

impl Default for ScanQuery {
    fn default() -> Self {
        Self {
            markets: Vec::new(),
            symbols: Symbols::default(),
            options: BTreeMap::from([(String::from("lang"), Value::String(String::from("en")))]),
            columns: vec![
                core::NAME,
                price::CLOSE,
                price::VOLUME,
                fundamentals::MARKET_CAP_BASIC,
            ],
            filters: Vec::new(),
            filter_tree: None,
            sort: None,
            page: Page::default(),
            ignore_unknown_fields: None,
            preset: None,
            price_conversion: None,
        }
    }
}

impl ScanQuery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn select<I, C>(mut self, columns: I) -> Self
    where
        I: IntoIterator<Item = C>,
        C: Into<Column>,
    {
        self.columns = columns.into_iter().map(Into::into).collect();
        self
    }

    pub fn push_column(mut self, column: impl Into<Column>) -> Self {
        self.columns.push(column.into());
        self
    }

    pub fn market(mut self, market: impl Into<Market>) -> Self {
        self.markets = vec![market.into()];
        self
    }

    pub fn markets<I, M>(mut self, markets: I) -> Self
    where
        I: IntoIterator<Item = M>,
        M: Into<Market>,
    {
        self.markets = markets.into_iter().map(Into::into).collect();
        self
    }

    pub fn symbols(mut self, symbols: Symbols) -> Self {
        self.symbols = symbols;
        self
    }

    pub fn tickers<I, T>(mut self, tickers: I) -> Self
    where
        I: IntoIterator<Item = T>,
        T: Into<Ticker>,
    {
        self.symbols = self.symbols.with_tickers(tickers);
        self
    }

    pub fn symbolset<I, S>(mut self, symbolset: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.symbols = self.symbols.with_symbolset(symbolset);
        self
    }

    pub fn watchlist(mut self, id: i64) -> Self {
        self.symbols = self.symbols.with_watchlist(id);
        self
    }

    pub fn group(mut self, group: SymbolGroup) -> Self {
        self.symbols = self.symbols.with_group(group);
        self
    }

    pub fn symbol_types<I, S>(mut self, types: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.symbols = self.symbols.with_types(types);
        self
    }

    pub fn filter(mut self, filter: FilterCondition) -> Self {
        self.filters.push(filter);
        self
    }

    pub fn filters<I>(mut self, filters: I) -> Self
    where
        I: IntoIterator<Item = FilterCondition>,
    {
        self.filters.extend(filters);
        self
    }

    pub fn filter_tree(mut self, filter_tree: FilterTree) -> Self {
        self.filter_tree = Some(filter_tree);
        self
    }

    pub fn sort(mut self, sort: SortSpec) -> Self {
        self.sort = Some(sort);
        self
    }

    pub fn page(mut self, offset: usize, limit: usize) -> TvResult<Self> {
        self.page = Page::new(offset, limit)?;
        Ok(self)
    }

    pub fn language(mut self, language: impl Into<String>) -> Self {
        self.options
            .insert(String::from("lang"), Value::String(language.into()));
        self
    }

    pub fn option(mut self, key: impl Into<String>, value: impl Into<Value>) -> Self {
        self.options.insert(key.into(), value.into());
        self
    }

    pub fn preset(mut self, preset: impl Into<String>) -> Self {
        self.preset = Some(preset.into());
        self
    }

    pub fn price_conversion(mut self, price_conversion: PriceConversion) -> Self {
        self.price_conversion = Some(price_conversion);
        self
    }

    pub fn ignore_unknown_fields(mut self, ignore_unknown_fields: bool) -> Self {
        self.ignore_unknown_fields = Some(ignore_unknown_fields);
        self
    }

    pub fn route_segment(&self) -> String {
        let requires_global_route = !self.symbols.symbolset.is_empty()
            || self.symbols.watchlist.is_some()
            || !self.symbols.groups.is_empty();

        match (requires_global_route, self.markets.as_slice()) {
            (false, [market]) => format!("{}/scan", market.as_str()),
            _ => String::from("global/scan"),
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::scanner::fields::{analyst, technical};

    #[test]
    fn uses_global_route_for_multi_market_or_symbolset_queries() {
        let query = ScanQuery::new()
            .markets(["america", "crypto"])
            .select([core::NAME, price::CLOSE]);
        assert_eq!(query.route_segment(), "global/scan");

        let query = ScanQuery::new()
            .symbolset(["SYML:SP;SPX"])
            .preset("index_components_market_pages");
        assert_eq!(query.route_segment(), "global/scan");

        let query = ScanQuery::new()
            .market("america")
            .symbolset(["SYML:SP;SPX"])
            .preset("index_components_market_pages");
        assert_eq!(query.route_segment(), "global/scan");
    }

    #[test]
    fn uses_global_route_for_watchlist_and_group_queries() {
        let query = ScanQuery::new().market("america").watchlist(42);
        assert_eq!(query.route_segment(), "global/scan");

        let query = ScanQuery::new()
            .market("america")
            .group(SymbolGroup::new("index", ["SPX"]));
        assert_eq!(query.route_segment(), "global/scan");
    }

    #[test]
    fn serializes_tradingview_scan_payload() {
        let query = ScanQuery::new()
            .market("america")
            .tickers(["NASDAQ:AAPL"])
            .select([
                core::NAME,
                price::CLOSE,
                analyst::PRICE_TARGET_AVERAGE,
                technical::RSI.with_interval("1W"),
            ])
            .filter(price::CLOSE.clone().gt(100))
            .sort(price::CLOSE.clone().sort(crate::scanner::SortOrder::Desc));

        let value = serde_json::to_value(query).unwrap();
        assert_eq!(value["columns"][0], "name");
        assert_eq!(value["columns"][3], "RSI|1W");
        assert_eq!(value["filter"][0]["operation"], "greater");
        assert_eq!(value["markets"], json!(["america"]));
    }
}
