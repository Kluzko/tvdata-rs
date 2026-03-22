use std::collections::BTreeMap;

use serde_json::Value;

use super::*;

#[test]
fn search_request_uses_tradingview_defaults() {
    let request = SearchRequest::new("AAPL").exchange("NASDAQ");
    let query = request.to_query_pairs();

    assert!(
        query
            .iter()
            .any(|(key, value)| *key == "hl" && value == "1")
    );
    assert!(
        query
            .iter()
            .any(|(key, value)| *key == "exchange" && value == "NASDAQ")
    );
    assert!(
        query
            .iter()
            .any(|(key, value)| *key == "domain" && value == "production")
    );
    assert!(!query.iter().any(|(key, _)| *key == "type"));
}

#[test]
fn asset_class_builders_map_to_current_search_types() {
    assert_eq!(
        SearchRequest::equities("AAPL").instrument_type.as_deref(),
        Some("stock")
    );
    assert_eq!(
        SearchRequest::forex("EURUSD").instrument_type.as_deref(),
        Some("forex")
    );
    assert_eq!(
        SearchRequest::crypto("BTC").instrument_type.as_deref(),
        Some("crypto")
    );
    assert_eq!(SearchRequest::options("AAPL").instrument_type, None);
}

#[test]
fn strip_markup_only_changes_highlight_tags() {
    assert_eq!(strip_em_tags("Apple <em>Inc.</em>"), "Apple Inc.");
}

#[test]
fn sanitize_response_supports_v3_envelopes() {
    let response = sanitize_response(RawSearchResponse::V3(RawSearchResponseV3 {
        symbols_remaining: 3,
        hits: vec![RawSearchHit {
            symbol: "<em>AAPL</em>".to_owned(),
            description: Some("Apple <em>Inc.</em>".to_owned()),
            instrument_type: Some("stock".to_owned()),
            exchange: Some("NASDAQ".to_owned()),
            country: Some("US".to_owned()),
            currency_code: Some("USD".to_owned()),
            currency_logoid: Some("country/US".to_owned()),
            provider_id: Some("ice".to_owned()),
            source_id: Some("NASDAQ".to_owned()),
            cik_code: Some("0000320193".to_owned()),
            isin: Some("US0378331005".to_owned()),
            cusip: Some("037833100".to_owned()),
            found_by_isin: Some(false),
            found_by_cusip: Some(false),
            is_primary_listing: Some(true),
            logoid: Some("apple".to_owned()),
            logo: Some(SearchLogo {
                style: Some("single".to_owned()),
                logoid: Some("apple".to_owned()),
            }),
            source: Some(SearchSource {
                id: Some("NASDAQ".to_owned()),
                name: Some("Nasdaq Stock Market".to_owned()),
                description: None,
            }),
            type_specs: vec!["common".to_owned()],
            extra: BTreeMap::new(),
        }],
    }));

    assert_eq!(response.symbols_remaining, 3);
    assert_eq!(response.hits[0].symbol, "AAPL");
    assert_eq!(response.hits[0].description.as_deref(), Some("Apple Inc."));
    assert_eq!(response.hits[0].cik_code.as_deref(), Some("0000320193"));
}

#[test]
fn option_like_detection_uses_multiple_clues() {
    let hit = SearchHit {
        symbol: "AAPL240621C00195000".to_owned(),
        highlighted_symbol: None,
        description: None,
        highlighted_description: None,
        instrument_type: Some("structured".to_owned()),
        exchange: None,
        country: None,
        currency_code: None,
        currency_logoid: None,
        provider_id: None,
        source_id: None,
        cik_code: None,
        isin: None,
        cusip: None,
        found_by_isin: None,
        found_by_cusip: None,
        is_primary_listing: None,
        logoid: None,
        logo: None,
        source: None,
        type_specs: Vec::new(),
        extra: BTreeMap::from([("option-type".to_owned(), Value::String("call".to_owned()))]),
    };

    assert!(hit.is_option_like());
}

#[test]
fn search_fixture_preserves_identifiers_and_extra_fields() {
    let raw: RawSearchResponse =
        serde_json::from_str(include_str!("../../tests/fixtures/search/aapl_v3.json")).unwrap();
    let response = sanitize_response(raw);

    assert_eq!(response.symbols_remaining, 14);
    assert_eq!(response.hits.len(), 2);
    assert_eq!(response.hits[0].symbol, "AAPL");
    assert_eq!(
        response.hits[0].highlighted_symbol.as_deref(),
        Some("<em>AAPL</em>")
    );
    assert_eq!(response.hits[0].isin.as_deref(), Some("US0378331005"));
    assert_eq!(
        response.hits[0]
            .source
            .as_ref()
            .and_then(|source| source.id.as_deref()),
        Some("NASDAQ")
    );
    assert_eq!(
        response.hits[0].extra.get("sector").and_then(Value::as_str),
        Some("Technology")
    );
    assert!(response.hits[1].is_option_like());
}
