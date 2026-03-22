use crate::scanner::field::Column;

pub const MARKET_CAP_BASIC: Column = Column::from_static("market_cap_basic");
pub const PRICE_EARNINGS_TTM: Column = Column::from_static("price_earnings_ttm");
pub const PRICE_TO_BOOK_FQ: Column = Column::from_static("price_book_fq");
pub const PRICE_TO_SALES_CURRENT: Column = Column::from_static("price_sales_current");
pub const TOTAL_REVENUE_TTM: Column = Column::from_static("total_revenue");
pub const NET_INCOME_TTM: Column = Column::from_static("net_income");
pub const EPS_TTM: Column = Column::from_static("earnings_per_share_basic_ttm");
pub const DIVIDEND_YIELD_RECENT: Column = Column::from_static("dividend_yield_recent");
pub const RETURN_ON_EQUITY_TTM: Column = Column::from_static("return_on_equity");
pub const RETURN_ON_ASSETS_TTM: Column = Column::from_static("return_on_assets");
pub const DEBT_TO_EQUITY_MRQ: Column = Column::from_static("debt_to_equity");
pub const CURRENT_RATIO_MRQ: Column = Column::from_static("current_ratio");
pub const FREE_CASH_FLOW_TTM: Column = Column::from_static("free_cash_flow");
pub const EBITDA_TTM: Column = Column::from_static("ebitda");
pub const TOTAL_REVENUE_FQ_H: Column = Column::from_static("total_revenue_fq_h");
pub const TOTAL_REVENUE_FY_H: Column = Column::from_static("total_revenue_fy_h");
pub const NET_INCOME_FQ_H: Column = Column::from_static("net_income_fq_h");
pub const NET_INCOME_FY_H: Column = Column::from_static("net_income_fy_h");
pub const TOTAL_ASSETS_FQ_H: Column = Column::from_static("total_assets_fq_h");
pub const TOTAL_ASSETS_FY_H: Column = Column::from_static("total_assets_fy_h");
pub const TOTAL_LIABILITIES_FQ_H: Column = Column::from_static("total_liabilities_fq_h");
pub const TOTAL_LIABILITIES_FY_H: Column = Column::from_static("total_liabilities_fy_h");
pub const CASH_FROM_OPERATIONS_FQ_H: Column =
    Column::from_static("cash_f_operating_activities_fq_h");
pub const CASH_FROM_OPERATIONS_FY_H: Column =
    Column::from_static("cash_f_operating_activities_fy_h");
pub const FISCAL_PERIOD_FQ_H: Column = Column::from_static("fiscal_period_fq_h");
pub const FISCAL_PERIOD_FY_H: Column = Column::from_static("fiscal_period_fy_h");
