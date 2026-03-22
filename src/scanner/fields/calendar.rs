use crate::scanner::field::Column;

pub const DIVIDEND_AMOUNT_UPCOMING: Column = Column::from_static("dividend_amount_upcoming");
pub const DIVIDEND_YIELD_UPCOMING: Column = Column::from_static("dividend_yield_upcoming");
pub const EX_DIVIDEND_DATE_UPCOMING: Column = Column::from_static("ex_dividend_date_upcoming");
pub const PAYMENT_DATE_UPCOMING: Column = Column::from_static("payment_date_upcoming");
pub const IPO_OFFER_DATE: Column = Column::from_static("ipo_offer_date");
pub const IPO_OFFER_TIME: Column = Column::from_static("ipo_offer_time");
pub const IPO_OFFER_PRICE_USD: Column = Column::from_static("ipo_offer_price_usd");
pub const IPO_DEAL_AMOUNT_USD: Column = Column::from_static("ipo_deal_amount_usd");
pub const IPO_MARKET_CAP_USD: Column = Column::from_static("ipo_market_cap_usd");
pub const IPO_PRICE_RANGE_USD_MIN: Column = Column::from_static("ipo_price_range_usd_min");
pub const IPO_PRICE_RANGE_USD_MAX: Column = Column::from_static("ipo_price_range_usd_max");
pub const IPO_ANNOUNCEMENT_DATE: Column = Column::from_static("ipo_announcement_date");
pub const IPO_OFFERED_SHARES: Column = Column::from_static("ipo_offered_shares");
pub const IPO_OFFERED_SHARES_PRIMARY: Column = Column::from_static("ipo_offered_shares_primary");
pub const IPO_OFFERED_SHARES_SECONDARY: Column =
    Column::from_static("ipo_offered_shares_secondary");
