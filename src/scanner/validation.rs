use crate::scanner::query::ScanQuery;
use crate::scanner::{Column, Market};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartiallySupportedColumn {
    pub column: Column,
    pub supported_markets: Vec<Market>,
    pub unsupported_markets: Vec<Market>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ScanValidationReport {
    pub route_segment: String,
    pub requested_markets: Vec<Market>,
    pub supported_columns: Vec<Column>,
    pub partially_supported_columns: Vec<PartiallySupportedColumn>,
    pub unsupported_columns: Vec<Column>,
}

impl ScanValidationReport {
    pub fn is_strictly_supported(&self) -> bool {
        self.partially_supported_columns.is_empty() && self.unsupported_columns.is_empty()
    }

    pub fn is_leniently_supported(&self) -> bool {
        self.unsupported_columns.is_empty()
    }

    pub fn strict_violation_column_names(&self) -> Vec<&str> {
        self.partially_supported_columns
            .iter()
            .map(|entry| entry.column.as_str())
            .chain(self.unsupported_columns.iter().map(Column::as_str))
            .collect()
    }

    pub fn filtered_query(&self, query: &ScanQuery) -> ScanQuery {
        let mut filtered = query.clone();
        filtered.columns = self.supported_columns.clone();
        filtered
    }

    pub fn filtered_column_names(&self) -> Vec<&str> {
        self.supported_columns.iter().map(Column::as_str).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn report_distinguishes_strict_and_lenient_support() {
        let report = ScanValidationReport {
            route_segment: "global/scan".to_owned(),
            requested_markets: vec![
                Market::from_static("america"),
                Market::from_static("crypto"),
            ],
            supported_columns: vec![Column::from_static("close")],
            partially_supported_columns: vec![PartiallySupportedColumn {
                column: Column::from_static("market_cap_basic"),
                supported_markets: vec![Market::from_static("america")],
                unsupported_markets: vec![Market::from_static("crypto")],
            }],
            unsupported_columns: vec![Column::from_static("imaginary_field")],
        };

        assert!(!report.is_strictly_supported());
        assert!(!report.is_leniently_supported());
        assert_eq!(
            report.strict_violation_column_names(),
            vec!["market_cap_basic", "imaginary_field"]
        );
        let filtered =
            report.filtered_query(&ScanQuery::new().markets(["america", "crypto"]).select([
                Column::from_static("close"),
                Column::from_static("market_cap_basic"),
            ]));
        assert_eq!(filtered.columns, vec![Column::from_static("close")]);
    }
}
