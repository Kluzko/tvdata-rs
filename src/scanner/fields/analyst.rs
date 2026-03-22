use crate::scanner::field::Column;

pub const RECOMMENDATION_BUY: Column = Column::from_static("recommendation_buy");
pub const RECOMMENDATION_SELL: Column = Column::from_static("recommendation_sell");
pub const RECOMMENDATION_HOLD: Column = Column::from_static("recommendation_hold");
pub const RECOMMENDATION_OVER: Column = Column::from_static("recommendation_over");
pub const RECOMMENDATION_UNDER: Column = Column::from_static("recommendation_under");
pub const RECOMMENDATION_TOTAL: Column = Column::from_static("recommendation_total");
pub const RECOMMENDATION_MARK: Column = Column::from_static("recommendation_mark");
pub const PRICE_TARGET_AVERAGE: Column = Column::from_static("price_target_average");
pub const PRICE_TARGET_HIGH: Column = Column::from_static("price_target_high");
pub const PRICE_TARGET_LOW: Column = Column::from_static("price_target_low");
pub const PRICE_TARGET_MEDIAN: Column = Column::from_static("price_target_median");
pub const PRICE_TARGET_1Y: Column = Column::from_static("price_target_1y");
pub const PRICE_TARGET_1Y_DELTA: Column = Column::from_static("price_target_1y_delta");
pub const REVENUE_FORECAST_FQ: Column = Column::from_static("revenue_forecast_fq");
pub const REVENUE_FORECAST_NEXT_FQ: Column = Column::from_static("revenue_forecast_next_fq");
pub const REVENUE_FORECAST_NEXT_FH: Column = Column::from_static("revenue_forecast_next_fh");
pub const REVENUE_FORECAST_NEXT_FY: Column = Column::from_static("revenue_forecast_next_fy");
pub const EPS_FORECAST_FQ: Column = Column::from_static("earnings_per_share_forecast_fq");
pub const EPS_FORECAST_NEXT_FQ: Column = Column::from_static("earnings_per_share_forecast_next_fq");
pub const EPS_FORECAST_NEXT_FH: Column = Column::from_static("earnings_per_share_forecast_next_fh");
pub const EPS_FORECAST_NEXT_FY: Column = Column::from_static("earnings_per_share_forecast_next_fy");
pub const EPS_FORECAST_FQ_H: Column = Column::from_static("earnings_per_share_forecast_fq_h");
pub const EPS_FORECAST_FY_H: Column = Column::from_static("earnings_per_share_forecast_fy_h");
pub const EPS_ACTUAL_FQ_H: Column = Column::from_static("earnings_per_share_fq_h");
pub const EPS_ACTUAL_FY_H: Column = Column::from_static("earnings_per_share_fy_h");
pub const EPS_SURPRISE_FQ: Column = Column::from_static("eps_surprise_fq");
pub const EPS_SURPRISE_PERCENT_FQ: Column = Column::from_static("eps_surprise_percent_fq");
pub const REVENUE_FORECAST_FQ_H: Column = Column::from_static("revenue_forecast_fq_h");
pub const REVENUE_FORECAST_FY_H: Column = Column::from_static("revenue_forecast_fy_h");
pub const FORWARD_PE_NON_GAAP_NEXT_FY: Column =
    Column::from_static("non_gaap_price_to_earnings_per_share_forecast_next_fy");
pub const FORWARD_PE_FY: Column = Column::from_static("price_earnings_forward_fy");
pub const EARNINGS_RELEASE_DATE: Column = Column::from_static("earnings_release_date");
pub const EARNINGS_RELEASE_NEXT_DATE: Column = Column::from_static("earnings_release_next_date");
pub const EARNINGS_RELEASE_CALENDAR_DATE: Column =
    Column::from_static("earnings_release_calendar_date");
pub const EARNINGS_RELEASE_NEXT_CALENDAR_DATE: Column =
    Column::from_static("earnings_release_next_calendar_date");
pub const EARNINGS_RELEASE_TRADING_DATE_FQ: Column =
    Column::from_static("earnings_release_trading_date_fq");
pub const EARNINGS_RELEASE_NEXT_TRADING_DATE_FQ: Column =
    Column::from_static("earnings_release_next_trading_date_fq");
pub const EARNINGS_RELEASE_NEXT_TRADING_DATE_FY: Column =
    Column::from_static("earnings_release_next_trading_date_fy");
pub const EARNINGS_RELEASE_TIME: Column = Column::from_static("earnings_release_time");
pub const EARNINGS_RELEASE_NEXT_TIME: Column = Column::from_static("earnings_release_next_time");
pub const EARNINGS_PUBLICATION_TYPE_FQ: Column =
    Column::from_static("earnings_publication_type_fq");
pub const EARNINGS_PUBLICATION_TYPE_NEXT_FQ: Column =
    Column::from_static("earnings_publication_type_next_fq");
pub const EARNINGS_RELEASE_DATE_FQ_H: Column = Column::from_static("earnings_release_date_fq_h");
pub const EARNINGS_RELEASE_DATE_FY_H: Column = Column::from_static("earnings_release_date_fy_h");
pub const EARNINGS_FISCAL_PERIOD_FQ_H: Column = Column::from_static("earnings_fiscal_period_fq_h");
pub const EARNINGS_FISCAL_PERIOD_FY_H: Column = Column::from_static("earnings_fiscal_period_fy_h");
pub const RATES_CURRENT: Column = Column::from_static("rates_current");
pub const RATES_TIME_SERIES: Column = Column::from_static("rates_time_series");
pub const RATES_FQ: Column = Column::from_static("rates_fq");
pub const RATES_FH: Column = Column::from_static("rates_fh");
pub const RATES_FY: Column = Column::from_static("rates_fy");
pub const RATES_TTM: Column = Column::from_static("rates_ttm");
pub const RATES_CF: Column = Column::from_static("rates_cf");
pub const RATES_PT: Column = Column::from_static("rates_pt");
pub const RATES_MC: Column = Column::from_static("rates_mc");
pub const RATES_EARNINGS_FQ: Column = Column::from_static("rates_earnings_fq");
pub const RATES_EARNINGS_NEXT_FQ: Column = Column::from_static("rates_earnings_next_fq");
pub const RATES_DIVIDEND_RECENT: Column = Column::from_static("rates_dividend_recent");
pub const RATES_DIVIDEND_UPCOMING: Column = Column::from_static("rates_dividend_upcoming");
