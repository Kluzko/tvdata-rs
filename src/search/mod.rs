use std::collections::BTreeMap;

use bon::Builder;
use serde::Deserialize;
use serde_json::Value;

#[cfg(test)]
mod tests;

fn default_search_language() -> String {
    "en".to_owned()
}

fn default_search_domain() -> String {
    "production".to_owned()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchAssetClass {
    Equity,
    Forex,
    Crypto,
    Futures,
    Index,
    Bond,
    Cfd,
    Option,
}

impl SearchAssetClass {
    pub const fn api_search_type(self) -> Option<&'static str> {
        match self {
            Self::Equity => Some("stock"),
            Self::Forex => Some("forex"),
            Self::Crypto => Some("crypto"),
            Self::Futures => Some("futures"),
            Self::Index => Some("index"),
            Self::Bond => Some("bond"),
            Self::Cfd => Some("cfd"),
            Self::Option => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Builder)]
pub struct SearchRequest {
    #[builder(into)]
    pub text: String,
    #[builder(into)]
    pub exchange: Option<String>,
    #[builder(into)]
    pub instrument_type: Option<String>,
    #[builder(default)]
    pub start: usize,
    #[builder(default = true)]
    pub highlight: bool,
    #[builder(default = default_search_language(), into)]
    pub language: String,
    #[builder(default = default_search_domain(), into)]
    pub domain: String,
}

impl SearchRequest {
    pub fn new(text: impl Into<String>) -> Self {
        Self::builder().text(text).build()
    }

    pub fn equities(text: impl Into<String>) -> Self {
        Self::new(text).asset_class(SearchAssetClass::Equity)
    }

    pub fn forex(text: impl Into<String>) -> Self {
        Self::new(text).asset_class(SearchAssetClass::Forex)
    }

    pub fn crypto(text: impl Into<String>) -> Self {
        Self::new(text).asset_class(SearchAssetClass::Crypto)
    }

    pub fn futures(text: impl Into<String>) -> Self {
        Self::new(text).asset_class(SearchAssetClass::Futures)
    }

    pub fn indices(text: impl Into<String>) -> Self {
        Self::new(text).asset_class(SearchAssetClass::Index)
    }

    pub fn bonds(text: impl Into<String>) -> Self {
        Self::new(text).asset_class(SearchAssetClass::Bond)
    }

    pub fn cfds(text: impl Into<String>) -> Self {
        Self::new(text).asset_class(SearchAssetClass::Cfd)
    }

    pub fn options(text: impl Into<String>) -> Self {
        Self::new(text)
    }

    pub fn exchange(mut self, exchange: impl Into<String>) -> Self {
        self.exchange = Some(exchange.into());
        self
    }

    pub fn asset_class(mut self, asset_class: SearchAssetClass) -> Self {
        self.instrument_type = asset_class.api_search_type().map(str::to_owned);
        self
    }

    pub fn instrument_type(mut self, instrument_type: impl Into<String>) -> Self {
        self.instrument_type = Some(instrument_type.into());
        self
    }

    pub fn start(mut self, start: usize) -> Self {
        self.start = start;
        self
    }

    pub fn highlight(mut self, highlight: bool) -> Self {
        self.highlight = highlight;
        self
    }

    pub fn language(mut self, language: impl Into<String>) -> Self {
        self.language = language.into();
        self
    }

    pub fn domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = domain.into();
        self
    }

    pub(crate) fn to_query_pairs(&self) -> Vec<(&str, String)> {
        let mut pairs = vec![
            ("text", self.text.clone()),
            ("start", self.start.to_string()),
            ("hl", if self.highlight { "1" } else { "0" }.to_owned()),
            ("lang", self.language.clone()),
            ("domain", self.domain.clone()),
        ];

        if let Some(exchange) = self.exchange.as_ref().filter(|value| !value.is_empty()) {
            pairs.push(("exchange", exchange.clone()));
        }

        if let Some(instrument_type) = self
            .instrument_type
            .as_ref()
            .filter(|value| !value.is_empty())
        {
            pairs.push(("search_type", instrument_type.clone()));
        }

        pairs
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchResponse {
    pub hits: Vec<SearchHit>,
    pub symbols_remaining: usize,
}

impl SearchResponse {
    pub fn filtered<F>(self, predicate: F) -> Self
    where
        F: FnMut(&SearchHit) -> bool,
    {
        Self {
            symbols_remaining: self.symbols_remaining,
            hits: self.hits.into_iter().filter(predicate).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct SearchHit {
    pub symbol: String,
    pub highlighted_symbol: Option<String>,
    pub description: Option<String>,
    pub highlighted_description: Option<String>,
    pub instrument_type: Option<String>,
    pub exchange: Option<String>,
    pub country: Option<String>,
    pub currency_code: Option<String>,
    pub currency_logoid: Option<String>,
    pub provider_id: Option<String>,
    pub source_id: Option<String>,
    pub cik_code: Option<String>,
    pub isin: Option<String>,
    pub cusip: Option<String>,
    pub found_by_isin: Option<bool>,
    pub found_by_cusip: Option<bool>,
    pub is_primary_listing: Option<bool>,
    pub logoid: Option<String>,
    pub logo: Option<SearchLogo>,
    pub source: Option<SearchSource>,
    pub type_specs: Vec<String>,
    pub extra: BTreeMap<String, Value>,
}

impl SearchHit {
    pub fn is_option_like(&self) -> bool {
        matches!(self.instrument_type.as_deref(), Some("option"))
            || self
                .type_specs
                .iter()
                .any(|value| value.eq_ignore_ascii_case("option"))
            || self.extra.contains_key("option-type")
            || self.extra.contains_key("expiration")
            || self.extra.contains_key("strike")
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SearchLogo {
    pub style: Option<String>,
    pub logoid: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct SearchSource {
    pub id: Option<String>,
    pub name: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawSearchHit {
    pub symbol: String,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(rename = "type", default)]
    pub instrument_type: Option<String>,
    #[serde(default)]
    pub exchange: Option<String>,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub currency_code: Option<String>,
    #[serde(default, rename = "currency-logoid")]
    pub currency_logoid: Option<String>,
    #[serde(default)]
    pub provider_id: Option<String>,
    #[serde(default)]
    pub source_id: Option<String>,
    #[serde(default)]
    pub cik_code: Option<String>,
    #[serde(default)]
    pub isin: Option<String>,
    #[serde(default)]
    pub cusip: Option<String>,
    #[serde(default)]
    pub found_by_isin: Option<bool>,
    #[serde(default)]
    pub found_by_cusip: Option<bool>,
    #[serde(default)]
    pub is_primary_listing: Option<bool>,
    #[serde(default)]
    pub logoid: Option<String>,
    #[serde(default)]
    pub logo: Option<SearchLogo>,
    #[serde(default, rename = "source2")]
    pub source: Option<SearchSource>,
    #[serde(default, rename = "typespecs")]
    pub type_specs: Vec<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawSearchResponseV3 {
    #[serde(default)]
    pub symbols_remaining: usize,
    #[serde(default, rename = "symbols")]
    pub hits: Vec<RawSearchHit>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum RawSearchResponse {
    V3(RawSearchResponseV3),
    Legacy(Vec<RawSearchHit>),
}

pub(crate) fn sanitize_response(raw_response: RawSearchResponse) -> SearchResponse {
    match raw_response {
        RawSearchResponse::V3(response) => SearchResponse {
            hits: sanitize_hits(response.hits),
            symbols_remaining: response.symbols_remaining,
        },
        RawSearchResponse::Legacy(hits) => SearchResponse {
            symbols_remaining: 0,
            hits: sanitize_hits(hits),
        },
    }
}

pub(crate) fn sanitize_hits(raw_hits: Vec<RawSearchHit>) -> Vec<SearchHit> {
    raw_hits.into_iter().map(sanitize_hit).collect()
}

fn sanitize_hit(hit: RawSearchHit) -> SearchHit {
    SearchHit {
        symbol: strip_em_tags(&hit.symbol),
        highlighted_symbol: contains_highlight_markup(&hit.symbol).then_some(hit.symbol),
        description: hit.description.as_ref().map(|value| strip_em_tags(value)),
        highlighted_description: hit
            .description
            .filter(|value| contains_highlight_markup(value)),
        instrument_type: hit.instrument_type,
        exchange: hit.exchange,
        country: hit.country,
        currency_code: hit.currency_code,
        currency_logoid: hit.currency_logoid,
        provider_id: hit.provider_id,
        source_id: hit.source_id,
        cik_code: hit.cik_code,
        isin: hit.isin,
        cusip: hit.cusip,
        found_by_isin: hit.found_by_isin,
        found_by_cusip: hit.found_by_cusip,
        is_primary_listing: hit.is_primary_listing,
        logoid: hit.logoid,
        logo: hit.logo,
        source: hit.source,
        type_specs: hit.type_specs,
        extra: hit.extra,
    }
}

fn contains_highlight_markup(value: &str) -> bool {
    value.contains("<em>") || value.contains("</em>")
}

fn strip_em_tags(value: &str) -> String {
    value.replace("<em>", "").replace("</em>", "")
}
