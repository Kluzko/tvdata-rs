use crate::market_data::{
    classification_columns, identity_columns, merge_columns, quote_columns, technical_columns,
};
use crate::scanner::Column;
use crate::scanner::fields::{analyst, fundamentals, price};

pub(crate) fn fundamentals_columns() -> Vec<Column> {
    merge_columns([
        equity_identity_columns(),
        vec![
            fundamentals::MARKET_CAP_BASIC,
            fundamentals::PRICE_EARNINGS_TTM,
            fundamentals::PRICE_TO_BOOK_FQ,
            fundamentals::PRICE_TO_SALES_CURRENT,
            fundamentals::TOTAL_REVENUE_TTM,
            fundamentals::NET_INCOME_TTM,
            fundamentals::EPS_TTM,
            fundamentals::DIVIDEND_YIELD_RECENT,
            fundamentals::RETURN_ON_EQUITY_TTM,
            fundamentals::RETURN_ON_ASSETS_TTM,
            fundamentals::DEBT_TO_EQUITY_MRQ,
            fundamentals::CURRENT_RATIO_MRQ,
            fundamentals::FREE_CASH_FLOW_TTM,
            fundamentals::EBITDA_TTM,
        ],
    ])
}

pub(crate) fn analyst_recommendation_columns() -> Vec<Column> {
    vec![
        analyst::RECOMMENDATION_BUY,
        analyst::RECOMMENDATION_SELL,
        analyst::RECOMMENDATION_HOLD,
        analyst::RECOMMENDATION_OVER,
        analyst::RECOMMENDATION_UNDER,
        analyst::RECOMMENDATION_TOTAL,
        analyst::RECOMMENDATION_MARK,
    ]
}

pub(crate) fn analyst_price_target_columns() -> Vec<Column> {
    vec![
        analyst::PRICE_TARGET_AVERAGE,
        analyst::PRICE_TARGET_HIGH,
        analyst::PRICE_TARGET_LOW,
        analyst::PRICE_TARGET_MEDIAN,
        analyst::PRICE_TARGET_1Y,
        analyst::PRICE_TARGET_1Y_DELTA,
    ]
}

pub(crate) fn analyst_forecast_columns() -> Vec<Column> {
    vec![
        analyst::REVENUE_FORECAST_FQ,
        analyst::REVENUE_FORECAST_NEXT_FQ,
        analyst::REVENUE_FORECAST_NEXT_FH,
        analyst::REVENUE_FORECAST_NEXT_FY,
        analyst::EPS_FORECAST_FQ,
        analyst::EPS_FORECAST_NEXT_FQ,
        analyst::EPS_FORECAST_NEXT_FH,
        analyst::EPS_FORECAST_NEXT_FY,
        analyst::EPS_SURPRISE_FQ,
        analyst::EPS_SURPRISE_PERCENT_FQ,
        analyst::FORWARD_PE_NON_GAAP_NEXT_FY,
        analyst::FORWARD_PE_FY,
    ]
}

pub(crate) fn earnings_calendar_columns() -> Vec<Column> {
    vec![
        analyst::EARNINGS_RELEASE_DATE,
        analyst::EARNINGS_RELEASE_NEXT_DATE,
        analyst::EARNINGS_RELEASE_CALENDAR_DATE,
        analyst::EARNINGS_RELEASE_NEXT_CALENDAR_DATE,
        analyst::EARNINGS_RELEASE_TRADING_DATE_FQ,
        analyst::EARNINGS_RELEASE_NEXT_TRADING_DATE_FQ,
        analyst::EARNINGS_RELEASE_NEXT_TRADING_DATE_FY,
        analyst::EARNINGS_RELEASE_TIME,
        analyst::EARNINGS_RELEASE_NEXT_TIME,
        analyst::EARNINGS_PUBLICATION_TYPE_FQ,
        analyst::EARNINGS_PUBLICATION_TYPE_NEXT_FQ,
    ]
}

pub(crate) fn analyst_fx_rate_columns() -> Vec<Column> {
    vec![
        analyst::RATES_CURRENT,
        analyst::RATES_TIME_SERIES,
        analyst::RATES_FQ,
        analyst::RATES_FH,
        analyst::RATES_FY,
        analyst::RATES_TTM,
        analyst::RATES_CF,
        analyst::RATES_PT,
        analyst::RATES_MC,
        analyst::RATES_EARNINGS_FQ,
        analyst::RATES_EARNINGS_NEXT_FQ,
        analyst::RATES_DIVIDEND_RECENT,
        analyst::RATES_DIVIDEND_UPCOMING,
    ]
}

pub(crate) fn analyst_columns() -> Vec<Column> {
    merge_columns([
        equity_identity_columns(),
        vec![price::CLOSE],
        analyst_recommendation_columns(),
        analyst_price_target_columns(),
        analyst_forecast_columns(),
        earnings_calendar_columns(),
        analyst_fx_rate_columns(),
    ])
}

pub(crate) fn overview_columns() -> Vec<Column> {
    merge_columns([
        equity_quote_columns(),
        fundamentals_columns(),
        analyst_columns(),
        equity_technical_columns(),
    ])
}

pub(crate) fn equity_identity_columns() -> Vec<Column> {
    merge_columns([identity_columns(), classification_columns()])
}

pub(crate) fn equity_quote_columns() -> Vec<Column> {
    merge_columns([equity_identity_columns(), quote_columns()])
}

pub(crate) fn equity_technical_columns() -> Vec<Column> {
    merge_columns([equity_identity_columns(), technical_columns()])
}
