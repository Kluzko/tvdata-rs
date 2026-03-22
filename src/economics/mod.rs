use std::collections::BTreeMap;

use bon::Builder;
use serde::Deserialize;
use serde_json::Value;
use time::{Duration, OffsetDateTime, format_description::well_known::Rfc3339};

use crate::error::Result;

#[cfg(test)]
mod tests;

fn default_calendar_from() -> OffsetDateTime {
    OffsetDateTime::now_utc() - Duration::days(7)
}

fn default_calendar_to() -> OffsetDateTime {
    OffsetDateTime::now_utc() + Duration::days(7)
}

#[derive(Debug, Clone, PartialEq, Eq, Builder)]
pub struct EconomicCalendarRequest {
    #[builder(default = default_calendar_from())]
    pub from: OffsetDateTime,
    #[builder(default = default_calendar_to())]
    pub to: OffsetDateTime,
}

impl EconomicCalendarRequest {
    pub fn new(from: OffsetDateTime, to: OffsetDateTime) -> Self {
        Self::builder().from(from).to(to).build()
    }

    pub fn upcoming(days: i64) -> Self {
        let now = OffsetDateTime::now_utc();
        Self::builder()
            .from(now)
            .to(now + Duration::days(days.max(0)))
            .build()
    }

    pub fn trailing(days: i64) -> Self {
        let now = OffsetDateTime::now_utc();
        Self::builder()
            .from(now - Duration::days(days.max(0)))
            .to(now)
            .build()
    }

    pub fn to_query_pairs(&self) -> Result<Vec<(&'static str, String)>> {
        Ok(vec![
            ("from", self.from.format(&Rfc3339)?),
            ("to", self.to.format(&Rfc3339)?),
        ])
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EconomicCalendarResponse {
    pub status: Option<String>,
    pub events: Vec<EconomicEvent>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EconomicEvent {
    pub id: String,
    pub title: Option<String>,
    pub indicator: Option<String>,
    pub date: OffsetDateTime,
    pub country: Option<String>,
    pub currency: Option<String>,
    pub importance: Option<i32>,
    pub actual: Option<EconomicValue>,
    pub forecast: Option<EconomicValue>,
    pub previous: Option<EconomicValue>,
    pub period: Option<String>,
    pub scale: Option<String>,
    pub unit: Option<String>,
    pub source: Option<String>,
    pub comment: Option<String>,
    pub link: Option<String>,
    pub extra: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum EconomicValue {
    Number(f64),
    Text(String),
    Bool(bool),
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawEconomicCalendarResponse {
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default, rename = "result")]
    pub events: Vec<RawEconomicEvent>,
}

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct RawEconomicEvent {
    pub id: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub indicator: Option<String>,
    #[serde(with = "time::serde::rfc3339")]
    pub date: OffsetDateTime,
    #[serde(default)]
    pub country: Option<String>,
    #[serde(default)]
    pub currency: Option<String>,
    #[serde(default)]
    pub importance: Option<i32>,
    #[serde(default)]
    pub actual: Option<EconomicValue>,
    #[serde(default)]
    pub forecast: Option<EconomicValue>,
    #[serde(default)]
    pub previous: Option<EconomicValue>,
    #[serde(default)]
    pub period: Option<String>,
    #[serde(default)]
    pub scale: Option<String>,
    #[serde(default)]
    pub unit: Option<String>,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub comment: Option<String>,
    #[serde(default)]
    pub link: Option<String>,
    #[serde(flatten)]
    pub extra: BTreeMap<String, Value>,
}

pub(crate) fn sanitize_calendar(
    raw_response: RawEconomicCalendarResponse,
) -> EconomicCalendarResponse {
    EconomicCalendarResponse {
        status: raw_response.status,
        events: raw_response
            .events
            .into_iter()
            .map(|event| EconomicEvent {
                id: event.id,
                title: event.title,
                indicator: event.indicator,
                date: event.date,
                country: event.country,
                currency: event.currency,
                importance: event.importance,
                actual: event.actual,
                forecast: event.forecast,
                previous: event.previous,
                period: event.period,
                scale: event.scale,
                unit: event.unit,
                source: event.source,
                comment: event.comment,
                link: event.link,
                extra: event.extra,
            })
            .collect(),
    }
}
