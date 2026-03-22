use crate::market_data::{
    ConversionRatesSnapshot, InstrumentIdentity, QuoteSnapshot, TechnicalSummary,
};
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq)]
pub struct FundamentalsSnapshot {
    pub instrument: InstrumentIdentity,
    pub market_cap: Option<f64>,
    pub price_earnings_ttm: Option<f64>,
    pub price_to_book_fq: Option<f64>,
    pub price_to_sales_current: Option<f64>,
    pub total_revenue_ttm: Option<f64>,
    pub net_income_ttm: Option<f64>,
    pub eps_ttm: Option<f64>,
    pub dividend_yield_recent: Option<f64>,
    pub return_on_equity_ttm: Option<f64>,
    pub return_on_assets_ttm: Option<f64>,
    pub debt_to_equity_mrq: Option<f64>,
    pub current_ratio_mrq: Option<f64>,
    pub free_cash_flow_ttm: Option<f64>,
    pub ebitda_ttm: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnalystRecommendations {
    pub buy: Option<u32>,
    pub sell: Option<u32>,
    pub hold: Option<u32>,
    pub outperform: Option<u32>,
    pub underperform: Option<u32>,
    pub total: Option<u32>,
    pub rating: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnalystPriceTargets {
    pub average: Option<f64>,
    pub high: Option<f64>,
    pub low: Option<f64>,
    pub median: Option<f64>,
    pub one_year: Option<f64>,
    pub one_year_delta_percent: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnalystForecasts {
    pub revenue_current_quarter: Option<f64>,
    pub revenue_next_quarter: Option<f64>,
    pub revenue_next_half_year: Option<f64>,
    pub revenue_next_fiscal_year: Option<f64>,
    pub eps_current_quarter: Option<f64>,
    pub eps_next_quarter: Option<f64>,
    pub eps_next_half_year: Option<f64>,
    pub eps_next_fiscal_year: Option<f64>,
    pub eps_surprise_recent_quarter: Option<f64>,
    pub eps_surprise_percent_recent_quarter: Option<f64>,
    pub forward_non_gaap_price_earnings: Option<f64>,
    pub forward_price_earnings_fiscal_year: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EarningsCalendar {
    pub recent_release_at: Option<OffsetDateTime>,
    pub next_release_at: Option<OffsetDateTime>,
    pub recent_calendar_date: Option<OffsetDateTime>,
    pub next_calendar_date: Option<OffsetDateTime>,
    pub current_quarter_trading_date: Option<OffsetDateTime>,
    pub next_quarter_trading_date: Option<OffsetDateTime>,
    pub fiscal_year_trading_date: Option<OffsetDateTime>,
    pub recent_release_time_code: Option<u32>,
    pub next_release_time_code: Option<u32>,
    pub current_quarter_publication_type_code: Option<u32>,
    pub next_quarter_publication_type_code: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct AnalystFxRates {
    pub current: Option<ConversionRatesSnapshot>,
    pub time_series: Option<ConversionRatesSnapshot>,
    pub revenue_current_quarter: Option<ConversionRatesSnapshot>,
    pub revenue_next_half_year: Option<ConversionRatesSnapshot>,
    pub revenue_next_fiscal_year: Option<ConversionRatesSnapshot>,
    pub trailing_twelve_months: Option<ConversionRatesSnapshot>,
    pub cash_flow: Option<ConversionRatesSnapshot>,
    pub price_target: Option<ConversionRatesSnapshot>,
    pub market_cap: Option<ConversionRatesSnapshot>,
    pub earnings_current_quarter: Option<ConversionRatesSnapshot>,
    pub earnings_next_quarter: Option<ConversionRatesSnapshot>,
    pub dividend_recent: Option<ConversionRatesSnapshot>,
    pub dividend_upcoming: Option<ConversionRatesSnapshot>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AnalystSummary {
    pub instrument: InstrumentIdentity,
    pub close: Option<f64>,
    pub recommendations: AnalystRecommendations,
    pub price_targets: AnalystPriceTargets,
    pub forecasts: AnalystForecasts,
    pub earnings: EarningsCalendar,
    pub fx_rates: AnalystFxRates,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EquityOverview {
    pub quote: QuoteSnapshot,
    pub fundamentals: FundamentalsSnapshot,
    pub analyst: AnalystSummary,
    pub technicals: TechnicalSummary,
}
