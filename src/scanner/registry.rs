use std::collections::HashMap;

use once_cell::sync::Lazy;
use serde::Deserialize;

use crate::scanner::field::Column;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ScreenerKind {
    Stock,
    Crypto,
    Forex,
    Bond,
    Futures,
    Coin,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct FieldDescriptor {
    pub name: String,
    pub label: String,
    pub field_name: String,
    pub format: Option<String>,
    pub interval: bool,
    pub historical: bool,
}

impl FieldDescriptor {
    pub fn column(&self) -> Column {
        Column::new(self.field_name.clone())
    }

    pub fn recommendation_column(&self) -> Option<Column> {
        (self.format.as_deref() == Some("recommendation")).then(|| self.column().recommendation())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct MarketDescriptor {
    pub name: String,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SymbolTypeDescriptor {
    pub name: String,
    pub value: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct IndexSymbolDescriptor {
    pub name: String,
    pub symbol: String,
    pub symbolset_value: String,
    pub label: String,
}

#[derive(Debug, Clone, Deserialize)]
struct RegistryEnvelope {
    screeners: HashMap<ScreenerKind, Vec<FieldDescriptor>>,
    markets: Vec<MarketDescriptor>,
    symbol_types: Vec<SymbolTypeDescriptor>,
    index_symbols: Vec<IndexSymbolDescriptor>,
}

#[derive(Debug, Clone)]
pub struct FieldRegistry {
    screeners: HashMap<ScreenerKind, Vec<FieldDescriptor>>,
    markets: Vec<MarketDescriptor>,
    symbol_types: Vec<SymbolTypeDescriptor>,
    index_symbols: Vec<IndexSymbolDescriptor>,
}

impl FieldRegistry {
    pub fn from_embedded() -> Self {
        let mut envelope: RegistryEnvelope =
            serde_json::from_str(include_str!("../../assets/field_registry.json"))
                .expect("embedded field registry must be valid JSON");
        normalize_embedded_field_names(&mut envelope.screeners);
        Self {
            screeners: envelope.screeners,
            markets: envelope.markets,
            symbol_types: envelope.symbol_types,
            index_symbols: envelope.index_symbols,
        }
    }

    pub fn fields(&self, screener: ScreenerKind) -> &[FieldDescriptor] {
        self.screeners
            .get(&screener)
            .map(Vec::as_slice)
            .unwrap_or(&[])
    }

    pub fn search(&self, screener: ScreenerKind, query: &str) -> Vec<&FieldDescriptor> {
        let query = query.to_ascii_lowercase();
        self.fields(screener)
            .iter()
            .filter(|field| {
                field.name.to_ascii_lowercase().contains(&query)
                    || field.label.to_ascii_lowercase().contains(&query)
                    || field.field_name.to_ascii_lowercase().contains(&query)
            })
            .collect()
    }

    pub fn find_by_api_name(
        &self,
        screener: ScreenerKind,
        api_name: &str,
    ) -> Option<&FieldDescriptor> {
        self.fields(screener)
            .iter()
            .find(|field| field.field_name == api_name)
    }

    pub fn markets(&self) -> &[MarketDescriptor] {
        &self.markets
    }

    pub fn symbol_types(&self) -> &[SymbolTypeDescriptor] {
        &self.symbol_types
    }

    pub fn index_symbols(&self) -> &[IndexSymbolDescriptor] {
        &self.index_symbols
    }
}

fn normalize_embedded_field_names(screeners: &mut HashMap<ScreenerKind, Vec<FieldDescriptor>>) {
    for fields in screeners.values_mut() {
        for field in fields {
            field.field_name = normalize_field_name(&field.field_name);
        }
    }
}

fn normalize_field_name(field_name: &str) -> String {
    if let Some((prefix, suffix)) = field_name.split_once('.')
        && (prefix == "change" || prefix == "change_abs" || prefix == "relative_volume_intraday")
        && (suffix.chars().all(|c| c.is_ascii_digit()) || matches!(suffix, "1W" | "1M"))
    {
        return format!("{prefix}|{suffix}");
    }

    field_name.to_owned()
}

pub fn embedded_registry() -> &'static FieldRegistry {
    static REGISTRY: Lazy<FieldRegistry> = Lazy::new(FieldRegistry::from_embedded);
    &REGISTRY
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn embedded_registry_matches_reference_counts() {
        let registry = embedded_registry();
        assert_eq!(registry.fields(ScreenerKind::Stock).len(), 3526);
        assert_eq!(registry.fields(ScreenerKind::Crypto).len(), 3108);
        assert_eq!(registry.fields(ScreenerKind::Forex).len(), 2965);
        assert_eq!(registry.markets().len(), 66);
        assert_eq!(registry.index_symbols().len(), 50);
    }

    #[test]
    fn registry_searches_by_name_label_and_api_name() {
        let registry = embedded_registry();
        let matches = registry.search(ScreenerKind::Stock, "dividend");
        assert!(
            matches
                .iter()
                .any(|field| field.field_name == "dividend_yield_recent")
        );
        assert!(
            registry
                .find_by_api_name(ScreenerKind::Stock, "market_cap_basic")
                .is_some()
        );
    }

    #[test]
    fn normalizes_timed_fields_to_api_format() {
        let registry = embedded_registry();
        assert!(
            registry
                .find_by_api_name(ScreenerKind::Stock, "change|1W")
                .is_some()
        );
    }

    #[test]
    fn builds_recommendation_companion_columns() {
        let registry = embedded_registry();
        let field = registry
            .find_by_api_name(ScreenerKind::Stock, "BBPower")
            .expect("BBPower should exist");
        let recommendation = field
            .recommendation_column()
            .expect("BBPower should expose recommendation column");
        assert_eq!(recommendation.as_str(), "Rec.BBPower");
    }
}
