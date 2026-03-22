use crate::market_data::{RowDecoder, decode_quote, decode_technical};
use crate::scanner::fields::{analyst, fundamentals, price};

use super::types::{
    AnalystForecasts, AnalystFxRates, AnalystPriceTargets, AnalystRecommendations, AnalystSummary,
    EarningsCalendar, EquityOverview, FundamentalsSnapshot,
};

pub(crate) fn decode_fundamentals(
    decoder: &RowDecoder,
    row: &crate::scanner::ScanRow,
) -> FundamentalsSnapshot {
    FundamentalsSnapshot {
        instrument: decoder.identity(row),
        market_cap: decoder.number(row, fundamentals::MARKET_CAP_BASIC.as_str()),
        price_earnings_ttm: decoder.number(row, fundamentals::PRICE_EARNINGS_TTM.as_str()),
        price_to_book_fq: decoder.number(row, fundamentals::PRICE_TO_BOOK_FQ.as_str()),
        price_to_sales_current: decoder.number(row, fundamentals::PRICE_TO_SALES_CURRENT.as_str()),
        total_revenue_ttm: decoder.number(row, fundamentals::TOTAL_REVENUE_TTM.as_str()),
        net_income_ttm: decoder.number(row, fundamentals::NET_INCOME_TTM.as_str()),
        eps_ttm: decoder.number(row, fundamentals::EPS_TTM.as_str()),
        dividend_yield_recent: decoder.number(row, fundamentals::DIVIDEND_YIELD_RECENT.as_str()),
        return_on_equity_ttm: decoder.number(row, fundamentals::RETURN_ON_EQUITY_TTM.as_str()),
        return_on_assets_ttm: decoder.number(row, fundamentals::RETURN_ON_ASSETS_TTM.as_str()),
        debt_to_equity_mrq: decoder.number(row, fundamentals::DEBT_TO_EQUITY_MRQ.as_str()),
        current_ratio_mrq: decoder.number(row, fundamentals::CURRENT_RATIO_MRQ.as_str()),
        free_cash_flow_ttm: decoder.number(row, fundamentals::FREE_CASH_FLOW_TTM.as_str()),
        ebitda_ttm: decoder.number(row, fundamentals::EBITDA_TTM.as_str()),
    }
}

pub(crate) fn decode_analyst_recommendations(
    decoder: &RowDecoder,
    row: &crate::scanner::ScanRow,
) -> AnalystRecommendations {
    AnalystRecommendations {
        buy: decoder.whole_number(row, analyst::RECOMMENDATION_BUY.as_str()),
        sell: decoder.whole_number(row, analyst::RECOMMENDATION_SELL.as_str()),
        hold: decoder.whole_number(row, analyst::RECOMMENDATION_HOLD.as_str()),
        outperform: decoder.whole_number(row, analyst::RECOMMENDATION_OVER.as_str()),
        underperform: decoder.whole_number(row, analyst::RECOMMENDATION_UNDER.as_str()),
        total: decoder.whole_number(row, analyst::RECOMMENDATION_TOTAL.as_str()),
        rating: decoder.number(row, analyst::RECOMMENDATION_MARK.as_str()),
    }
}

pub(crate) fn decode_analyst_price_targets(
    decoder: &RowDecoder,
    row: &crate::scanner::ScanRow,
) -> AnalystPriceTargets {
    AnalystPriceTargets {
        average: decoder.number(row, analyst::PRICE_TARGET_AVERAGE.as_str()),
        high: decoder.number(row, analyst::PRICE_TARGET_HIGH.as_str()),
        low: decoder.number(row, analyst::PRICE_TARGET_LOW.as_str()),
        median: decoder.number(row, analyst::PRICE_TARGET_MEDIAN.as_str()),
        one_year: decoder.number(row, analyst::PRICE_TARGET_1Y.as_str()),
        one_year_delta_percent: decoder.number(row, analyst::PRICE_TARGET_1Y_DELTA.as_str()),
    }
}

pub(crate) fn decode_analyst_forecasts(
    decoder: &RowDecoder,
    row: &crate::scanner::ScanRow,
) -> AnalystForecasts {
    AnalystForecasts {
        revenue_current_quarter: decoder.number(row, analyst::REVENUE_FORECAST_FQ.as_str()),
        revenue_next_quarter: decoder.number(row, analyst::REVENUE_FORECAST_NEXT_FQ.as_str()),
        revenue_next_half_year: decoder.number(row, analyst::REVENUE_FORECAST_NEXT_FH.as_str()),
        revenue_next_fiscal_year: decoder.number(row, analyst::REVENUE_FORECAST_NEXT_FY.as_str()),
        eps_current_quarter: decoder.number(row, analyst::EPS_FORECAST_FQ.as_str()),
        eps_next_quarter: decoder.number(row, analyst::EPS_FORECAST_NEXT_FQ.as_str()),
        eps_next_half_year: decoder.number(row, analyst::EPS_FORECAST_NEXT_FH.as_str()),
        eps_next_fiscal_year: decoder.number(row, analyst::EPS_FORECAST_NEXT_FY.as_str()),
        eps_surprise_recent_quarter: decoder.number(row, analyst::EPS_SURPRISE_FQ.as_str()),
        eps_surprise_percent_recent_quarter: decoder
            .number(row, analyst::EPS_SURPRISE_PERCENT_FQ.as_str()),
        forward_non_gaap_price_earnings: decoder
            .number(row, analyst::FORWARD_PE_NON_GAAP_NEXT_FY.as_str()),
        forward_price_earnings_fiscal_year: decoder.number(row, analyst::FORWARD_PE_FY.as_str()),
    }
}

pub(crate) fn decode_earnings_calendar(
    decoder: &RowDecoder,
    row: &crate::scanner::ScanRow,
) -> EarningsCalendar {
    EarningsCalendar {
        recent_release_at: decoder.timestamp(row, analyst::EARNINGS_RELEASE_DATE.as_str()),
        next_release_at: decoder.timestamp(row, analyst::EARNINGS_RELEASE_NEXT_DATE.as_str()),
        recent_calendar_date: decoder
            .timestamp(row, analyst::EARNINGS_RELEASE_CALENDAR_DATE.as_str()),
        next_calendar_date: decoder
            .timestamp(row, analyst::EARNINGS_RELEASE_NEXT_CALENDAR_DATE.as_str()),
        current_quarter_trading_date: decoder
            .timestamp(row, analyst::EARNINGS_RELEASE_TRADING_DATE_FQ.as_str()),
        next_quarter_trading_date: decoder
            .timestamp(row, analyst::EARNINGS_RELEASE_NEXT_TRADING_DATE_FQ.as_str()),
        fiscal_year_trading_date: decoder
            .timestamp(row, analyst::EARNINGS_RELEASE_NEXT_TRADING_DATE_FY.as_str()),
        recent_release_time_code: decoder
            .whole_number(row, analyst::EARNINGS_RELEASE_TIME.as_str()),
        next_release_time_code: decoder
            .whole_number(row, analyst::EARNINGS_RELEASE_NEXT_TIME.as_str()),
        current_quarter_publication_type_code: decoder
            .whole_number(row, analyst::EARNINGS_PUBLICATION_TYPE_FQ.as_str()),
        next_quarter_publication_type_code: decoder
            .whole_number(row, analyst::EARNINGS_PUBLICATION_TYPE_NEXT_FQ.as_str()),
    }
}

pub(crate) fn decode_analyst_fx_rates(
    decoder: &RowDecoder,
    row: &crate::scanner::ScanRow,
) -> AnalystFxRates {
    AnalystFxRates {
        current: decoder.conversion_rates(row, analyst::RATES_CURRENT.as_str()),
        time_series: decoder.conversion_rates(row, analyst::RATES_TIME_SERIES.as_str()),
        revenue_current_quarter: decoder.conversion_rates(row, analyst::RATES_FQ.as_str()),
        revenue_next_half_year: decoder.conversion_rates(row, analyst::RATES_FH.as_str()),
        revenue_next_fiscal_year: decoder.conversion_rates(row, analyst::RATES_FY.as_str()),
        trailing_twelve_months: decoder.conversion_rates(row, analyst::RATES_TTM.as_str()),
        cash_flow: decoder.conversion_rates(row, analyst::RATES_CF.as_str()),
        price_target: decoder.conversion_rates(row, analyst::RATES_PT.as_str()),
        market_cap: decoder.conversion_rates(row, analyst::RATES_MC.as_str()),
        earnings_current_quarter: decoder
            .conversion_rates(row, analyst::RATES_EARNINGS_FQ.as_str()),
        earnings_next_quarter: decoder
            .conversion_rates(row, analyst::RATES_EARNINGS_NEXT_FQ.as_str()),
        dividend_recent: decoder.conversion_rates(row, analyst::RATES_DIVIDEND_RECENT.as_str()),
        dividend_upcoming: decoder.conversion_rates(row, analyst::RATES_DIVIDEND_UPCOMING.as_str()),
    }
}

pub(crate) fn decode_analyst(
    decoder: &RowDecoder,
    row: &crate::scanner::ScanRow,
) -> AnalystSummary {
    AnalystSummary {
        instrument: decoder.identity(row),
        close: decoder.number(row, price::CLOSE.as_str()),
        recommendations: decode_analyst_recommendations(decoder, row),
        price_targets: decode_analyst_price_targets(decoder, row),
        forecasts: decode_analyst_forecasts(decoder, row),
        earnings: decode_earnings_calendar(decoder, row),
        fx_rates: decode_analyst_fx_rates(decoder, row),
    }
}

pub(crate) fn decode_overview(
    decoder: &RowDecoder,
    row: &crate::scanner::ScanRow,
) -> EquityOverview {
    EquityOverview {
        quote: decode_quote(decoder, row),
        fundamentals: decode_fundamentals(decoder, row),
        analyst: decode_analyst(decoder, row),
        technicals: decode_technical(decoder, row),
    }
}
