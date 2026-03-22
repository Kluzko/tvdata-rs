use std::collections::BTreeMap;

use time::macros::datetime;

use super::*;

#[test]
fn calendar_request_formats_rfc3339_bounds() {
    let request = EconomicCalendarRequest::new(
        datetime!(2026-03-01 00:00 UTC),
        datetime!(2026-03-31 00:00 UTC),
    );
    let query = request.to_query_pairs().unwrap();

    assert!(
        query
            .iter()
            .any(|(key, value)| { *key == "from" && value == "2026-03-01T00:00:00Z" })
    );
    assert!(
        query
            .iter()
            .any(|(key, value)| { *key == "to" && value == "2026-03-31T00:00:00Z" })
    );
}

#[test]
fn sanitize_calendar_preserves_typed_values() {
    let response = sanitize_calendar(RawEconomicCalendarResponse {
        status: Some("ok".to_owned()),
        events: vec![RawEconomicEvent {
            id: "event-1".to_owned(),
            title: Some("GDP".to_owned()),
            indicator: Some("GDP Growth Rate".to_owned()),
            date: datetime!(2026-03-22 12:30 UTC),
            country: Some("US".to_owned()),
            currency: Some("USD".to_owned()),
            importance: Some(2),
            actual: Some(EconomicValue::Number(2.1)),
            forecast: Some(EconomicValue::Number(2.0)),
            previous: Some(EconomicValue::Text("1.9".to_owned())),
            period: Some("Q1".to_owned()),
            scale: Some("%".to_owned()),
            unit: Some("USD".to_owned()),
            source: Some("BEA".to_owned()),
            comment: None,
            link: None,
            extra: BTreeMap::new(),
        }],
    });

    assert_eq!(response.status.as_deref(), Some("ok"));
    assert_eq!(response.events[0].importance, Some(2));
    assert_eq!(response.events[0].actual, Some(EconomicValue::Number(2.1)));
}

#[test]
fn economic_calendar_fixture_preserves_extra_fields_and_value_types() {
    let raw: RawEconomicCalendarResponse = serde_json::from_str(include_str!(
        "../../tests/fixtures/economics/calendar_sample.json"
    ))
    .unwrap();
    let response = sanitize_calendar(raw);

    assert_eq!(response.status.as_deref(), Some("ok"));
    assert_eq!(response.events.len(), 2);
    assert_eq!(
        response.events[0].title.as_deref(),
        Some("Consumer Price Index")
    );
    assert_eq!(response.events[0].actual, Some(EconomicValue::Number(2.9)));
    assert_eq!(
        response.events[0].extra.get("revised"),
        Some(&serde_json::Value::Bool(true))
    );
    assert_eq!(
        response.events[1].forecast,
        Some(EconomicValue::Text("0.3".to_owned()))
    );
}
