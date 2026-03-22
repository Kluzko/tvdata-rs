use std::collections::HashSet;

use crate::scanner::Column;
use crate::scanner::fields::core;
#[cfg(any(feature = "crypto", feature = "equity", feature = "forex"))]
use crate::scanner::fields::{fundamentals, price, technical};

pub(crate) fn identity_columns() -> Vec<Column> {
    vec![
        core::NAME,
        core::MARKET,
        core::EXCHANGE,
        core::CURRENCY,
        core::COUNTRY,
        core::TYPE,
    ]
}

#[cfg(feature = "equity")]
pub(crate) fn classification_columns() -> Vec<Column> {
    vec![core::SECTOR, core::INDUSTRY]
}

#[cfg(any(feature = "crypto", feature = "equity", feature = "forex"))]
pub(crate) fn quote_columns() -> Vec<Column> {
    merge_columns([
        identity_columns(),
        vec![
            price::OPEN,
            price::HIGH,
            price::LOW,
            price::CLOSE,
            price::CHANGE_PERCENT,
            price::CHANGE_ABS,
            price::VOLUME,
            price::RELATIVE_VOLUME,
            fundamentals::MARKET_CAP_BASIC,
        ],
    ])
}

#[cfg(any(feature = "crypto", feature = "equity", feature = "forex"))]
pub(crate) fn technical_columns() -> Vec<Column> {
    merge_columns([
        identity_columns(),
        vec![
            price::CLOSE,
            technical::RECOMMEND_ALL,
            technical::RECOMMEND_MA,
            technical::RECOMMEND_OTHER,
            technical::RSI,
            technical::RSI7,
            technical::MACD,
            technical::MACD_SIGNAL,
            technical::MACD_HISTOGRAM,
            technical::ADX,
            technical::ATR,
            technical::SMA20,
            technical::SMA50,
            technical::SMA200,
            technical::EMA20,
            technical::EMA50,
            technical::EMA200,
            technical::STOCH_K,
            technical::STOCH_D,
            technical::WILLIAMS_R,
            technical::CCI20,
        ],
    ])
}

pub(crate) fn merge_columns<const N: usize>(groups: [Vec<Column>; N]) -> Vec<Column> {
    let mut seen = HashSet::new();
    let mut merged = Vec::new();

    for group in groups {
        for column in group {
            if seen.insert(column.as_str().to_owned()) {
                merged.push(column);
            }
        }
    }

    merged
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scanner::fields::core;

    #[test]
    fn shared_identity_columns_are_cross_market_safe() {
        let columns = identity_columns();

        assert!(columns.contains(&core::NAME));
        assert!(columns.contains(&core::TYPE));
        assert!(!columns.contains(&core::SECTOR));
        assert!(!columns.contains(&core::INDUSTRY));
    }

    #[cfg(feature = "equity")]
    #[test]
    fn classification_columns_are_opt_in() {
        let columns = classification_columns();

        assert_eq!(columns, vec![core::SECTOR, core::INDUSTRY]);
    }
}
