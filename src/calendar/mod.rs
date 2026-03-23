use bon::Builder;
use time::{Duration, OffsetDateTime};

use crate::client::TradingViewClient;
use crate::error::Result;
use crate::market_data::{InstrumentIdentity, RowDecoder, identity_columns, merge_columns};
use crate::scanner::fields::{analyst, calendar as calendar_fields};
use crate::scanner::filter::SortOrder;
use crate::scanner::{Column, Market, ScanQuery, ScanRow};

const DEFAULT_CALENDAR_LIMIT: usize = 100;
const CALENDAR_PAGE_SIZE: usize = 200;

fn default_calendar_from() -> OffsetDateTime {
    OffsetDateTime::now_utc()
}

fn default_calendar_to() -> OffsetDateTime {
    OffsetDateTime::now_utc() + Duration::days(30)
}

#[derive(Debug, Clone, PartialEq, Eq, Builder)]
pub struct CalendarWindowRequest {
    #[builder(into)]
    pub market: Market,
    #[builder(default = default_calendar_from())]
    pub from: OffsetDateTime,
    #[builder(default = default_calendar_to())]
    pub to: OffsetDateTime,
    #[builder(default = DEFAULT_CALENDAR_LIMIT)]
    pub limit: usize,
}

impl CalendarWindowRequest {
    pub fn new(market: impl Into<Market>, from: OffsetDateTime, to: OffsetDateTime) -> Self {
        Self::builder().market(market).from(from).to(to).build()
    }

    pub fn upcoming(market: impl Into<Market>, days: i64) -> Self {
        let now = OffsetDateTime::now_utc();
        Self::builder()
            .market(market)
            .from(now)
            .to(now + Duration::days(days.max(0)))
            .build()
    }

    pub fn trailing(market: impl Into<Market>, days: i64) -> Self {
        let now = OffsetDateTime::now_utc();
        Self::builder()
            .market(market)
            .from(now - Duration::days(days.max(0)))
            .to(now)
            .build()
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DividendDateKind {
    #[default]
    ExDate,
    PaymentDate,
}

#[derive(Debug, Clone, PartialEq, Eq, Builder)]
pub struct DividendCalendarRequest {
    #[builder(into)]
    pub market: Market,
    #[builder(default = default_calendar_from())]
    pub from: OffsetDateTime,
    #[builder(default = default_calendar_to())]
    pub to: OffsetDateTime,
    #[builder(default = DEFAULT_CALENDAR_LIMIT)]
    pub limit: usize,
    #[builder(default)]
    pub date_kind: DividendDateKind,
}

impl DividendCalendarRequest {
    pub fn new(market: impl Into<Market>, from: OffsetDateTime, to: OffsetDateTime) -> Self {
        Self::builder().market(market).from(from).to(to).build()
    }

    pub fn upcoming(market: impl Into<Market>, days: i64) -> Self {
        let now = OffsetDateTime::now_utc();
        Self::builder()
            .market(market)
            .from(now)
            .to(now + Duration::days(days.max(0)))
            .build()
    }

    pub fn trailing(market: impl Into<Market>, days: i64) -> Self {
        let now = OffsetDateTime::now_utc();
        Self::builder()
            .market(market)
            .from(now - Duration::days(days.max(0)))
            .to(now)
            .build()
    }

    pub fn limit(mut self, limit: usize) -> Self {
        self.limit = limit;
        self
    }

    pub fn date_kind(mut self, date_kind: DividendDateKind) -> Self {
        self.date_kind = date_kind;
        self
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct EarningsCalendarEntry {
    pub instrument: InstrumentIdentity,
    pub release_at: OffsetDateTime,
    pub release_time_code: Option<u32>,
    pub calendar_date: Option<OffsetDateTime>,
    pub eps_forecast_next_fq: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DividendCalendarEntry {
    pub instrument: InstrumentIdentity,
    pub effective_date: OffsetDateTime,
    pub ex_date: Option<OffsetDateTime>,
    pub payment_date: Option<OffsetDateTime>,
    pub amount: Option<f64>,
    pub yield_percent: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IpoCalendarEntry {
    pub instrument: InstrumentIdentity,
    pub offer_date: OffsetDateTime,
    pub offer_time_code: Option<u32>,
    pub announcement_date: Option<OffsetDateTime>,
    pub offer_price_usd: Option<f64>,
    pub deal_amount_usd: Option<f64>,
    pub market_cap_usd: Option<f64>,
    pub price_range_usd_min: Option<f64>,
    pub price_range_usd_max: Option<f64>,
    pub offered_shares: Option<f64>,
    pub offered_shares_primary: Option<f64>,
    pub offered_shares_secondary: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum WindowOrdering {
    Asc,
    Desc,
}

struct CalendarScanSpec<T, Decode, Date>
where
    Decode: Fn(&RowDecoder, &ScanRow) -> Option<T>,
    Date: Fn(&T) -> Option<OffsetDateTime>,
{
    sort_by: Column,
    ordering: WindowOrdering,
    columns: Vec<Column>,
    decode: Decode,
    event_date: Date,
}

impl TradingViewClient {
    pub(crate) async fn corporate_earnings_calendar(
        &self,
        request: &CalendarWindowRequest,
    ) -> Result<Vec<EarningsCalendarEntry>> {
        let columns = earnings_calendar_columns();
        scan_calendar_window(
            self,
            &request.market,
            request.from,
            request.to,
            request.limit,
            CalendarScanSpec {
                sort_by: analyst::EARNINGS_RELEASE_NEXT_DATE,
                ordering: WindowOrdering::Asc,
                columns,
                decode: decode_earnings_entry,
                event_date: EarningsCalendarEntry::event_date,
            },
        )
        .await
    }

    pub(crate) async fn corporate_dividend_calendar(
        &self,
        request: &DividendCalendarRequest,
    ) -> Result<Vec<DividendCalendarEntry>> {
        let columns = dividend_calendar_columns();
        let sort_by = match request.date_kind {
            DividendDateKind::ExDate => calendar_fields::EX_DIVIDEND_DATE_UPCOMING,
            DividendDateKind::PaymentDate => calendar_fields::PAYMENT_DATE_UPCOMING,
        };

        scan_calendar_window(
            self,
            &request.market,
            request.from,
            request.to,
            request.limit,
            CalendarScanSpec {
                sort_by,
                ordering: WindowOrdering::Asc,
                columns,
                decode: |decoder, row| decode_dividend_entry(decoder, row, request.date_kind),
                event_date: DividendCalendarEntry::event_date,
            },
        )
        .await
    }

    pub(crate) async fn corporate_ipo_calendar(
        &self,
        request: &CalendarWindowRequest,
    ) -> Result<Vec<IpoCalendarEntry>> {
        let columns = ipo_calendar_columns();
        scan_calendar_window(
            self,
            &request.market,
            request.from,
            request.to,
            request.limit,
            CalendarScanSpec {
                sort_by: calendar_fields::IPO_OFFER_DATE,
                ordering: WindowOrdering::Desc,
                columns,
                decode: decode_ipo_entry,
                event_date: IpoCalendarEntry::event_date,
            },
        )
        .await
    }
}

fn earnings_calendar_columns() -> Vec<Column> {
    merge_columns([
        identity_columns(),
        vec![
            analyst::EARNINGS_RELEASE_NEXT_DATE,
            analyst::EARNINGS_RELEASE_NEXT_CALENDAR_DATE,
            analyst::EARNINGS_RELEASE_NEXT_TIME,
            analyst::EPS_FORECAST_NEXT_FQ,
        ],
    ])
}

fn dividend_calendar_columns() -> Vec<Column> {
    merge_columns([
        identity_columns(),
        vec![
            calendar_fields::DIVIDEND_AMOUNT_UPCOMING,
            calendar_fields::DIVIDEND_YIELD_UPCOMING,
            calendar_fields::EX_DIVIDEND_DATE_UPCOMING,
            calendar_fields::PAYMENT_DATE_UPCOMING,
        ],
    ])
}

fn ipo_calendar_columns() -> Vec<Column> {
    merge_columns([
        identity_columns(),
        vec![
            calendar_fields::IPO_OFFER_DATE,
            calendar_fields::IPO_OFFER_TIME,
            calendar_fields::IPO_ANNOUNCEMENT_DATE,
            calendar_fields::IPO_OFFER_PRICE_USD,
            calendar_fields::IPO_DEAL_AMOUNT_USD,
            calendar_fields::IPO_MARKET_CAP_USD,
            calendar_fields::IPO_PRICE_RANGE_USD_MIN,
            calendar_fields::IPO_PRICE_RANGE_USD_MAX,
            calendar_fields::IPO_OFFERED_SHARES,
            calendar_fields::IPO_OFFERED_SHARES_PRIMARY,
            calendar_fields::IPO_OFFERED_SHARES_SECONDARY,
        ],
    ])
}

async fn scan_calendar_window<T, Decode, Date>(
    client: &TradingViewClient,
    market: &Market,
    from: OffsetDateTime,
    to: OffsetDateTime,
    limit: usize,
    spec: CalendarScanSpec<T, Decode, Date>,
) -> Result<Vec<T>>
where
    Decode: Fn(&RowDecoder, &ScanRow) -> Option<T>,
    Date: Fn(&T) -> Option<OffsetDateTime>,
{
    if limit == 0 || from > to {
        return Ok(Vec::new());
    }

    let CalendarScanSpec {
        sort_by,
        ordering,
        columns,
        decode,
        event_date,
    } = spec;
    let decoder = RowDecoder::new(&columns);
    let sort_order = match ordering {
        WindowOrdering::Asc => SortOrder::Asc,
        WindowOrdering::Desc => SortOrder::Desc,
    };
    let base_query = ScanQuery::new()
        .market(market.clone())
        .select(columns)
        .filter(sort_by.clone().not_empty())
        .sort(sort_by.sort(sort_order));

    let mut results = Vec::new();
    let mut offset = 0usize;

    loop {
        let query = base_query.clone().page(offset, CALENDAR_PAGE_SIZE)?;
        let response = client.scan(&query).await?;
        if response.rows.is_empty() {
            break;
        }

        let mut reached_window_end = false;
        for row in &response.rows {
            let Some(entry) = decode(&decoder, row) else {
                continue;
            };
            let Some(entry_date) = event_date(&entry) else {
                continue;
            };

            match ordering {
                WindowOrdering::Asc => {
                    if entry_date < from {
                        continue;
                    }
                    if entry_date > to {
                        reached_window_end = true;
                        break;
                    }
                    results.push(entry);
                    if results.len() >= limit {
                        return Ok(results);
                    }
                }
                WindowOrdering::Desc => {
                    if entry_date > to {
                        continue;
                    }
                    if entry_date < from {
                        reached_window_end = true;
                        break;
                    }
                    results.push(entry);
                }
            }
        }

        if reached_window_end {
            break;
        }

        offset += response.rows.len();
        if offset >= response.total_count || response.rows.len() < CALENDAR_PAGE_SIZE {
            break;
        }
    }

    if matches!(ordering, WindowOrdering::Desc) {
        results.reverse();
        if results.len() > limit {
            results.truncate(limit);
        }
    }

    Ok(results)
}

fn decode_earnings_entry(decoder: &RowDecoder, row: &ScanRow) -> Option<EarningsCalendarEntry> {
    Some(EarningsCalendarEntry {
        instrument: decoder.identity(row),
        release_at: decoder.timestamp(row, analyst::EARNINGS_RELEASE_NEXT_DATE.as_str())?,
        release_time_code: decoder.whole_number(row, analyst::EARNINGS_RELEASE_NEXT_TIME.as_str()),
        calendar_date: decoder
            .timestamp(row, analyst::EARNINGS_RELEASE_NEXT_CALENDAR_DATE.as_str()),
        eps_forecast_next_fq: decoder.number(row, analyst::EPS_FORECAST_NEXT_FQ.as_str()),
    })
}

fn decode_dividend_entry(
    decoder: &RowDecoder,
    row: &ScanRow,
    date_kind: DividendDateKind,
) -> Option<DividendCalendarEntry> {
    let ex_date = decoder.timestamp(row, calendar_fields::EX_DIVIDEND_DATE_UPCOMING.as_str());
    let payment_date = decoder.timestamp(row, calendar_fields::PAYMENT_DATE_UPCOMING.as_str());
    let effective_date = match date_kind {
        DividendDateKind::ExDate => ex_date,
        DividendDateKind::PaymentDate => payment_date,
    }?;

    Some(DividendCalendarEntry {
        instrument: decoder.identity(row),
        effective_date,
        ex_date,
        payment_date,
        amount: decoder.number(row, calendar_fields::DIVIDEND_AMOUNT_UPCOMING.as_str()),
        yield_percent: decoder.number(row, calendar_fields::DIVIDEND_YIELD_UPCOMING.as_str()),
    })
}

fn decode_ipo_entry(decoder: &RowDecoder, row: &ScanRow) -> Option<IpoCalendarEntry> {
    Some(IpoCalendarEntry {
        instrument: decoder.identity(row),
        offer_date: decoder.timestamp(row, calendar_fields::IPO_OFFER_DATE.as_str())?,
        offer_time_code: decoder.whole_number(row, calendar_fields::IPO_OFFER_TIME.as_str()),
        announcement_date: decoder.timestamp(row, calendar_fields::IPO_ANNOUNCEMENT_DATE.as_str()),
        offer_price_usd: decoder.number(row, calendar_fields::IPO_OFFER_PRICE_USD.as_str()),
        deal_amount_usd: decoder.number(row, calendar_fields::IPO_DEAL_AMOUNT_USD.as_str()),
        market_cap_usd: decoder.number(row, calendar_fields::IPO_MARKET_CAP_USD.as_str()),
        price_range_usd_min: decoder.number(row, calendar_fields::IPO_PRICE_RANGE_USD_MIN.as_str()),
        price_range_usd_max: decoder.number(row, calendar_fields::IPO_PRICE_RANGE_USD_MAX.as_str()),
        offered_shares: decoder.number(row, calendar_fields::IPO_OFFERED_SHARES.as_str()),
        offered_shares_primary: decoder
            .number(row, calendar_fields::IPO_OFFERED_SHARES_PRIMARY.as_str()),
        offered_shares_secondary: decoder
            .number(row, calendar_fields::IPO_OFFERED_SHARES_SECONDARY.as_str()),
    })
}

impl EarningsCalendarEntry {
    fn event_date(&self) -> Option<OffsetDateTime> {
        Some(self.release_at)
    }
}

impl DividendCalendarEntry {
    fn event_date(&self) -> Option<OffsetDateTime> {
        Some(self.effective_date)
    }
}

impl IpoCalendarEntry {
    fn event_date(&self) -> Option<OffsetDateTime> {
        Some(self.offer_date)
    }
}

#[cfg(test)]
mod tests;
