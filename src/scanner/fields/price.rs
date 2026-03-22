use crate::scanner::field::Column;

pub const OPEN: Column = Column::from_static("open");
pub const HIGH: Column = Column::from_static("high");
pub const LOW: Column = Column::from_static("low");
pub const CLOSE: Column = Column::from_static("close");
pub const VWAP: Column = Column::from_static("VWAP");
pub const CHANGE_PERCENT: Column = Column::from_static("change");
pub const CHANGE_ABS: Column = Column::from_static("change_abs");
pub const CHANGE_FROM_OPEN_PERCENT: Column = Column::from_static("change_from_open");
pub const CHANGE_FROM_OPEN_ABS: Column = Column::from_static("change_from_open_abs");
pub const VOLUME: Column = Column::from_static("volume");
pub const RELATIVE_VOLUME: Column = Column::from_static("relative_volume_10d_calc");
pub const VALUE_TRADED: Column = Column::from_static("Value.Traded");
