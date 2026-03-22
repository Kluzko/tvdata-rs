pub mod analyst;
pub mod calendar;
pub mod core;
pub mod fundamentals;
pub mod performance;
pub mod price;
pub mod technical;

use crate::scanner::field::Column;

pub fn quote_snapshot() -> Vec<Column> {
    vec![
        core::NAME,
        core::MARKET,
        price::CLOSE,
        price::CHANGE_PERCENT,
        price::CHANGE_ABS,
        price::VOLUME,
        price::RELATIVE_VOLUME,
        fundamentals::MARKET_CAP_BASIC,
    ]
}

pub fn technical_snapshot() -> Vec<Column> {
    vec![
        core::NAME,
        price::CLOSE,
        technical::RECOMMEND_ALL,
        technical::RSI,
        technical::MACD,
        technical::SMA50,
        technical::SMA200,
        technical::EMA20,
        technical::ADX,
        technical::ATR,
    ]
}

pub fn analyst_snapshot() -> Vec<Column> {
    vec![
        core::NAME,
        price::CLOSE,
        analyst::PRICE_TARGET_AVERAGE,
        analyst::PRICE_TARGET_HIGH,
        analyst::PRICE_TARGET_LOW,
        analyst::PRICE_TARGET_MEDIAN,
        analyst::RECOMMENDATION_BUY,
        analyst::RECOMMENDATION_HOLD,
        analyst::RECOMMENDATION_SELL,
        analyst::RECOMMENDATION_MARK,
    ]
}

pub fn fundamentals_snapshot() -> Vec<Column> {
    vec![
        core::NAME,
        fundamentals::MARKET_CAP_BASIC,
        fundamentals::PRICE_EARNINGS_TTM,
        fundamentals::PRICE_TO_BOOK_FQ,
        fundamentals::PRICE_TO_SALES_CURRENT,
        fundamentals::TOTAL_REVENUE_TTM,
        fundamentals::NET_INCOME_TTM,
        fundamentals::EPS_TTM,
        fundamentals::RETURN_ON_EQUITY_TTM,
        fundamentals::DEBT_TO_EQUITY_MRQ,
    ]
}
