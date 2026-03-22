use std::convert::Infallible;
use std::fmt;
use std::str::FromStr;

use serde::{Deserialize, Serialize};
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum FiscalPeriod {
    FiscalYear { year: i32 },
    FiscalHalfYear { year: i32, half: u8 },
    FiscalQuarter { year: i32, quarter: u8 },
    Raw(String),
}

impl FiscalPeriod {
    pub fn parse(value: impl AsRef<str>) -> Self {
        let value = value.as_ref().trim();
        if let Some((year, quarter)) = value.split_once("-Q")
            && let (Ok(year), Ok(quarter)) = (year.parse::<i32>(), quarter.parse::<u8>())
        {
            return Self::FiscalQuarter { year, quarter };
        }

        if let Some((year, half)) = value.split_once("-H")
            && let (Ok(year), Ok(half)) = (year.parse::<i32>(), half.parse::<u8>())
        {
            return Self::FiscalHalfYear { year, half };
        }

        if let Ok(year) = value.parse::<i32>() {
            return Self::FiscalYear { year };
        }

        Self::Raw(value.to_owned())
    }
}

impl FromStr for FiscalPeriod {
    type Err = Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::parse(s))
    }
}

impl fmt::Display for FiscalPeriod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::FiscalYear { year } => write!(f, "{year}"),
            Self::FiscalHalfYear { year, half } => write!(f, "{year}-H{half}"),
            Self::FiscalQuarter { year, quarter } => write!(f, "{year}-Q{quarter}"),
            Self::Raw(value) => f.write_str(value),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HistoricalObservation<T> {
    pub fiscal_period: Option<FiscalPeriod>,
    pub release_at: Option<OffsetDateTime>,
    pub value: T,
}

impl<T> HistoricalObservation<T> {
    pub fn new(
        fiscal_period: Option<FiscalPeriod>,
        release_at: Option<OffsetDateTime>,
        value: T,
    ) -> Self {
        Self {
            fiscal_period,
            release_at,
            value,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fiscal_period_variants() {
        assert_eq!(
            FiscalPeriod::parse("2025"),
            FiscalPeriod::FiscalYear { year: 2025 }
        );
        assert_eq!(
            FiscalPeriod::parse("2025-H1"),
            FiscalPeriod::FiscalHalfYear {
                year: 2025,
                half: 1
            }
        );
        assert_eq!(
            FiscalPeriod::parse("2025-Q4"),
            FiscalPeriod::FiscalQuarter {
                year: 2025,
                quarter: 4
            }
        );
        assert_eq!(
            FiscalPeriod::parse("custom"),
            FiscalPeriod::Raw("custom".to_owned())
        );
    }
}
