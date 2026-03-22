use crate::scanner::field::Column;

pub const PERF_1W: Column = Column::from_static("Perf.1W");
pub const PERF_1M: Column = Column::from_static("Perf.1M");
pub const PERF_3M: Column = Column::from_static("Perf.3M");
pub const PERF_6M: Column = Column::from_static("Perf.6M");
pub const PERF_YTD: Column = Column::from_static("Perf.YTD");
pub const PERF_1Y: Column = Column::from_static("Perf.Y");
pub const HIGH_52W: Column = Column::from_static("High.52W");
pub const LOW_52W: Column = Column::from_static("Low.52W");
pub const VOLATILITY_WEEK: Column = Column::from_static("Volatility.W");
pub const VOLATILITY_MONTH: Column = Column::from_static("Volatility.M");
