use crate::market_data::InstrumentIdentity;
use crate::metadata::{DataLineage, DataSourceKind, HistoryKind};
use crate::scanner::Column;
use crate::scanner::fields::{analyst, fundamentals};
use crate::time_series::{FiscalPeriod, HistoricalObservation};
use crate::transport::quote_session::QuoteFieldValues;
use time::OffsetDateTime;

#[derive(Debug, Clone, PartialEq, Default)]
pub struct EstimateMetrics {
    pub revenue_forecast: Option<f64>,
    pub revenue_actual: Option<f64>,
    pub eps_forecast: Option<f64>,
    pub eps_actual: Option<f64>,
}

pub type EstimateObservation = HistoricalObservation<EstimateMetrics>;
pub type EarningsMetrics = EstimateMetrics;

#[derive(Debug, Clone, PartialEq)]
pub struct EstimateHistory {
    pub instrument: InstrumentIdentity,
    pub quarterly: Vec<EstimateObservation>,
    pub annual: Vec<EstimateObservation>,
    pub lineage: DataLineage,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct FundamentalMetrics {
    pub total_revenue: Option<f64>,
    pub net_income: Option<f64>,
    pub total_assets: Option<f64>,
    pub total_liabilities: Option<f64>,
    pub cash_from_operations: Option<f64>,
}

pub type FundamentalObservation = HistoricalObservation<FundamentalMetrics>;

#[derive(Debug, Clone, PartialEq)]
pub struct PointInTimeFundamentals {
    pub instrument: InstrumentIdentity,
    pub quarterly: Vec<FundamentalObservation>,
    pub annual: Vec<FundamentalObservation>,
    pub lineage: DataLineage,
}

pub(crate) fn decode_estimate_history(
    instrument: InstrumentIdentity,
    values: &QuoteFieldValues,
) -> EstimateHistory {
    let quarterly = build_estimate_observations(
        values.string_series(analyst::EARNINGS_FISCAL_PERIOD_FQ_H.as_str()),
        values.timestamp_series(analyst::EARNINGS_RELEASE_DATE_FQ_H.as_str()),
        values.number_series(analyst::REVENUE_FORECAST_FQ_H.as_str()),
        Vec::new(),
        values.number_series(analyst::EPS_FORECAST_FQ_H.as_str()),
        values.number_series(analyst::EPS_ACTUAL_FQ_H.as_str()),
    );
    let annual = build_estimate_observations(
        values.string_series(analyst::EARNINGS_FISCAL_PERIOD_FY_H.as_str()),
        values.timestamp_series(analyst::EARNINGS_RELEASE_DATE_FY_H.as_str()),
        values.number_series(analyst::REVENUE_FORECAST_FY_H.as_str()),
        Vec::new(),
        values.number_series(analyst::EPS_FORECAST_FY_H.as_str()),
        values.number_series(analyst::EPS_ACTUAL_FY_H.as_str()),
    );

    EstimateHistory {
        instrument,
        quarterly,
        annual,
        lineage: native_history_lineage([
            latest_release(values, analyst::EARNINGS_RELEASE_DATE_FQ_H.as_str()),
            latest_release(values, analyst::EARNINGS_RELEASE_DATE_FY_H.as_str()),
        ]),
    }
}

pub(crate) fn decode_point_in_time_fundamentals(
    instrument: InstrumentIdentity,
    values: &QuoteFieldValues,
) -> PointInTimeFundamentals {
    let quarterly = build_fundamental_observations(
        values.string_series(fundamentals::FISCAL_PERIOD_FQ_H.as_str()),
        values.timestamp_series(analyst::EARNINGS_RELEASE_DATE_FQ_H.as_str()),
        values.number_series(fundamentals::TOTAL_REVENUE_FQ_H.as_str()),
        values.number_series(fundamentals::NET_INCOME_FQ_H.as_str()),
        values.number_series(fundamentals::TOTAL_ASSETS_FQ_H.as_str()),
        values.number_series(fundamentals::TOTAL_LIABILITIES_FQ_H.as_str()),
        values.number_series(fundamentals::CASH_FROM_OPERATIONS_FQ_H.as_str()),
    );
    let annual = build_fundamental_observations(
        values.string_series(fundamentals::FISCAL_PERIOD_FY_H.as_str()),
        values.timestamp_series(analyst::EARNINGS_RELEASE_DATE_FY_H.as_str()),
        values.number_series(fundamentals::TOTAL_REVENUE_FY_H.as_str()),
        values.number_series(fundamentals::NET_INCOME_FY_H.as_str()),
        values.number_series(fundamentals::TOTAL_ASSETS_FY_H.as_str()),
        values.number_series(fundamentals::TOTAL_LIABILITIES_FY_H.as_str()),
        values.number_series(fundamentals::CASH_FROM_OPERATIONS_FY_H.as_str()),
    );

    PointInTimeFundamentals {
        instrument,
        quarterly,
        annual,
        lineage: native_history_lineage([
            latest_release(values, analyst::EARNINGS_RELEASE_DATE_FQ_H.as_str()),
            latest_release(values, analyst::EARNINGS_RELEASE_DATE_FY_H.as_str()),
        ]),
    }
}

pub(crate) fn estimate_history_fields() -> Vec<Column> {
    vec![
        analyst::REVENUE_FORECAST_FQ_H,
        analyst::REVENUE_FORECAST_FY_H,
        analyst::EPS_FORECAST_FQ_H,
        analyst::EPS_FORECAST_FY_H,
        analyst::EPS_ACTUAL_FQ_H,
        analyst::EPS_ACTUAL_FY_H,
        analyst::EARNINGS_RELEASE_DATE_FQ_H,
        analyst::EARNINGS_RELEASE_DATE_FY_H,
        analyst::EARNINGS_FISCAL_PERIOD_FQ_H,
        analyst::EARNINGS_FISCAL_PERIOD_FY_H,
    ]
}

pub(crate) fn fundamentals_history_fields() -> Vec<Column> {
    vec![
        fundamentals::TOTAL_REVENUE_FQ_H,
        fundamentals::TOTAL_REVENUE_FY_H,
        fundamentals::NET_INCOME_FQ_H,
        fundamentals::NET_INCOME_FY_H,
        fundamentals::TOTAL_ASSETS_FQ_H,
        fundamentals::TOTAL_ASSETS_FY_H,
        fundamentals::TOTAL_LIABILITIES_FQ_H,
        fundamentals::TOTAL_LIABILITIES_FY_H,
        fundamentals::CASH_FROM_OPERATIONS_FQ_H,
        fundamentals::CASH_FROM_OPERATIONS_FY_H,
        fundamentals::FISCAL_PERIOD_FQ_H,
        fundamentals::FISCAL_PERIOD_FY_H,
        analyst::EARNINGS_RELEASE_DATE_FQ_H,
        analyst::EARNINGS_RELEASE_DATE_FY_H,
    ]
}

fn build_estimate_observations(
    fiscal_periods: Vec<Option<String>>,
    release_dates: Vec<Option<OffsetDateTime>>,
    revenue_forecasts: Vec<Option<f64>>,
    revenue_actuals: Vec<Option<f64>>,
    eps_forecasts: Vec<Option<f64>>,
    eps_actuals: Vec<Option<f64>>,
) -> Vec<EstimateObservation> {
    let len = [
        fiscal_periods.len(),
        release_dates.len(),
        revenue_forecasts.len(),
        revenue_actuals.len(),
        eps_forecasts.len(),
        eps_actuals.len(),
    ]
    .into_iter()
    .max()
    .unwrap_or(0);
    let fiscal_periods = parse_fiscal_period_series(fiscal_periods);

    (0..len)
        .map(|index| {
            let value = EstimateMetrics {
                revenue_forecast: series_value(&revenue_forecasts, index),
                revenue_actual: series_value(&revenue_actuals, index),
                eps_forecast: series_value(&eps_forecasts, index),
                eps_actual: series_value(&eps_actuals, index),
            };

            EstimateObservation::new(
                series_value(&fiscal_periods, index),
                series_value(&release_dates, index),
                value,
            )
        })
        .filter(|observation| {
            observation.fiscal_period.is_some()
                || observation.release_at.is_some()
                || observation.value.revenue_forecast.is_some()
                || observation.value.revenue_actual.is_some()
                || observation.value.eps_forecast.is_some()
                || observation.value.eps_actual.is_some()
        })
        .collect()
}

fn build_fundamental_observations(
    fiscal_periods: Vec<Option<String>>,
    release_dates: Vec<Option<OffsetDateTime>>,
    total_revenue: Vec<Option<f64>>,
    net_income: Vec<Option<f64>>,
    total_assets: Vec<Option<f64>>,
    total_liabilities: Vec<Option<f64>>,
    cash_from_operations: Vec<Option<f64>>,
) -> Vec<FundamentalObservation> {
    let len = [
        fiscal_periods.len(),
        release_dates.len(),
        total_revenue.len(),
        net_income.len(),
        total_assets.len(),
        total_liabilities.len(),
        cash_from_operations.len(),
    ]
    .into_iter()
    .max()
    .unwrap_or(0);
    let fiscal_periods = parse_fiscal_period_series(fiscal_periods);

    (0..len)
        .map(|index| {
            let value = FundamentalMetrics {
                total_revenue: series_value(&total_revenue, index),
                net_income: series_value(&net_income, index),
                total_assets: series_value(&total_assets, index),
                total_liabilities: series_value(&total_liabilities, index),
                cash_from_operations: series_value(&cash_from_operations, index),
            };

            FundamentalObservation::new(
                series_value(&fiscal_periods, index),
                series_value(&release_dates, index),
                value,
            )
        })
        .filter(|observation| {
            observation.fiscal_period.is_some()
                || observation.release_at.is_some()
                || observation.value.total_revenue.is_some()
                || observation.value.net_income.is_some()
                || observation.value.total_assets.is_some()
                || observation.value.total_liabilities.is_some()
                || observation.value.cash_from_operations.is_some()
        })
        .collect()
}

fn latest_release(values: &QuoteFieldValues, field: &str) -> Option<OffsetDateTime> {
    values.timestamp_series(field).into_iter().flatten().next()
}

fn native_history_lineage<const N: usize>(
    effective_candidates: [Option<OffsetDateTime>; N],
) -> DataLineage {
    DataLineage::new(
        DataSourceKind::Composed,
        HistoryKind::Native,
        OffsetDateTime::now_utc(),
        effective_candidates.into_iter().flatten().max(),
    )
}

fn series_value<T: Clone>(series: &[Option<T>], index: usize) -> Option<T> {
    series.get(index).cloned().flatten()
}

fn parse_fiscal_period_series(series: Vec<Option<String>>) -> Vec<Option<FiscalPeriod>> {
    series
        .into_iter()
        .map(|value| value.map(FiscalPeriod::parse))
        .collect()
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use serde_json::json;

    use super::*;
    use crate::scanner::Ticker;

    fn instrument() -> InstrumentIdentity {
        InstrumentIdentity {
            ticker: Ticker::new("NASDAQ:AAPL"),
            name: Some("Apple".to_owned()),
            market: Some("america".to_owned()),
            exchange: Some("NASDAQ".to_owned()),
            currency: Some("USD".to_owned()),
            country: Some("US".to_owned()),
            instrument_type: Some("stock".to_owned()),
            sector: None,
            industry: None,
        }
    }

    #[test]
    fn estimate_history_decodes_native_quote_series() {
        let values = QuoteFieldValues::from_values(BTreeMap::from([
            (
                analyst::EARNINGS_FISCAL_PERIOD_FQ_H.as_str().to_owned(),
                json!(["2026-Q1", "2025-Q4"]),
            ),
            (
                analyst::EARNINGS_RELEASE_DATE_FQ_H.as_str().to_owned(),
                json!([1769722200, 1761856320]),
            ),
            (
                analyst::REVENUE_FORECAST_FQ_H.as_str().to_owned(),
                json!([138391007589.0, 102227074560.0]),
            ),
            (
                analyst::EPS_FORECAST_FQ_H.as_str().to_owned(),
                json!([2.673324, 1.777147]),
            ),
            (
                analyst::EPS_ACTUAL_FQ_H.as_str().to_owned(),
                json!([2.84, 1.85]),
            ),
            (
                analyst::EARNINGS_FISCAL_PERIOD_FY_H.as_str().to_owned(),
                json!(["2025", "2024"]),
            ),
            (
                analyst::EARNINGS_RELEASE_DATE_FY_H.as_str().to_owned(),
                json!([1761856320, 1730406900i64]),
            ),
            (
                analyst::REVENUE_FORECAST_FY_H.as_str().to_owned(),
                json!([415406882375.0, 390480701773.0]),
            ),
            (
                analyst::EPS_FORECAST_FY_H.as_str().to_owned(),
                json!([7.381826, 6.708209]),
            ),
            (
                analyst::EPS_ACTUAL_FY_H.as_str().to_owned(),
                json!([7.46, 6.75]),
            ),
        ]));

        let history = decode_estimate_history(instrument(), &values);

        assert_eq!(history.instrument.ticker.as_str(), "NASDAQ:AAPL");
        assert_eq!(history.quarterly.len(), 2);
        assert_eq!(
            history.quarterly[0].fiscal_period,
            Some(FiscalPeriod::FiscalQuarter {
                year: 2026,
                quarter: 1,
            })
        );
        assert_eq!(history.quarterly[0].value.eps_actual, Some(2.84));
        assert_eq!(
            history.annual[0].value.revenue_forecast,
            Some(415406882375.0)
        );
        assert_eq!(history.lineage.source, DataSourceKind::Composed);
        assert_eq!(history.lineage.history_kind, HistoryKind::Native);
    }

    #[test]
    fn point_in_time_fundamentals_decodes_native_quote_series() {
        let values = QuoteFieldValues::from_values(BTreeMap::from([
            (
                fundamentals::FISCAL_PERIOD_FQ_H.as_str().to_owned(),
                json!(["2026-Q1", "2025-Q4"]),
            ),
            (
                analyst::EARNINGS_RELEASE_DATE_FQ_H.as_str().to_owned(),
                json!([1769722200, 1761856320]),
            ),
            (
                fundamentals::TOTAL_REVENUE_FQ_H.as_str().to_owned(),
                json!([143756000000.0, 102466000000.0]),
            ),
            (
                fundamentals::NET_INCOME_FQ_H.as_str().to_owned(),
                json!([42097000000.0, 27466000000.0]),
            ),
            (
                fundamentals::TOTAL_ASSETS_FQ_H.as_str().to_owned(),
                json!([379297000000.0, 359241000000.0]),
            ),
            (
                fundamentals::TOTAL_LIABILITIES_FQ_H.as_str().to_owned(),
                json!([290437000000.0, 308030000000.0]),
            ),
            (
                fundamentals::CASH_FROM_OPERATIONS_FQ_H.as_str().to_owned(),
                json!([53925000000.0, 29728000000.0]),
            ),
            (
                fundamentals::FISCAL_PERIOD_FY_H.as_str().to_owned(),
                json!(["2025", "2024"]),
            ),
            (
                analyst::EARNINGS_RELEASE_DATE_FY_H.as_str().to_owned(),
                json!([1761856320, 1730406900i64]),
            ),
            (
                fundamentals::TOTAL_REVENUE_FY_H.as_str().to_owned(),
                json!([416161000000.0, 391035000000.0]),
            ),
            (
                fundamentals::NET_INCOME_FY_H.as_str().to_owned(),
                json!([112010000000.0, 93736000000.0]),
            ),
            (
                fundamentals::TOTAL_ASSETS_FY_H.as_str().to_owned(),
                json!([359241000000.0, 364980000000.0]),
            ),
            (
                fundamentals::TOTAL_LIABILITIES_FY_H.as_str().to_owned(),
                json!([264090000000.0, 308030000000.0]),
            ),
            (
                fundamentals::CASH_FROM_OPERATIONS_FY_H.as_str().to_owned(),
                json!([111482000000.0, 118254000000.0]),
            ),
        ]));

        let history = decode_point_in_time_fundamentals(instrument(), &values);

        assert_eq!(history.quarterly.len(), 2);
        assert_eq!(
            history.quarterly[0].fiscal_period,
            Some(FiscalPeriod::FiscalQuarter {
                year: 2026,
                quarter: 1,
            })
        );
        assert_eq!(history.quarterly[0].value.net_income, Some(42097000000.0));
        assert_eq!(
            history.annual[0].value.cash_from_operations,
            Some(111482000000.0)
        );
        assert_eq!(history.lineage.source, DataSourceKind::Composed);
        assert_eq!(history.lineage.history_kind, HistoryKind::Native);
    }
}
