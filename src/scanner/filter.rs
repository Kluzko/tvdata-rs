use serde::Serialize;
use serde::ser::Serializer;
use serde_json::Value;

use crate::scanner::field::Column;

pub trait IntoFilterValue {
    fn into_filter_value(self) -> Value;
}

macro_rules! impl_numeric_filter_value {
    ($($ty:ty),+ $(,)?) => {
        $(
            impl IntoFilterValue for $ty {
                fn into_filter_value(self) -> Value {
                    Value::from(self)
                }
            }
        )+
    };
}

impl_numeric_filter_value!(i8, i16, i32, i64, isize, u8, u16, u32, u64);

impl IntoFilterValue for f32 {
    fn into_filter_value(self) -> Value {
        Value::from(self as f64)
    }
}

impl IntoFilterValue for f64 {
    fn into_filter_value(self) -> Value {
        Value::from(self)
    }
}

impl IntoFilterValue for bool {
    fn into_filter_value(self) -> Value {
        Value::from(self)
    }
}

impl IntoFilterValue for Value {
    fn into_filter_value(self) -> Value {
        self
    }
}

impl IntoFilterValue for String {
    fn into_filter_value(self) -> Value {
        Value::from(self)
    }
}

impl IntoFilterValue for &str {
    fn into_filter_value(self) -> Value {
        Value::from(self)
    }
}

impl IntoFilterValue for Column {
    fn into_filter_value(self) -> Value {
        Value::from(self.as_str().to_owned())
    }
}

impl IntoFilterValue for &Column {
    fn into_filter_value(self) -> Value {
        Value::from(self.as_str().to_owned())
    }
}

impl<T> IntoFilterValue for Option<T>
where
    T: IntoFilterValue,
{
    fn into_filter_value(self) -> Value {
        self.map(IntoFilterValue::into_filter_value)
            .unwrap_or(Value::Null)
    }
}

impl<T> IntoFilterValue for Vec<T>
where
    T: IntoFilterValue,
{
    fn into_filter_value(self) -> Value {
        Value::Array(
            self.into_iter()
                .map(IntoFilterValue::into_filter_value)
                .collect(),
        )
    }
}

impl<T, const N: usize> IntoFilterValue for [T; N]
where
    T: IntoFilterValue,
{
    fn into_filter_value(self) -> Value {
        Value::Array(
            self.into_iter()
                .map(IntoFilterValue::into_filter_value)
                .collect(),
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum FilterOperator {
    #[serde(rename = "greater")]
    Greater,
    #[serde(rename = "egreater")]
    EGreater,
    #[serde(rename = "less")]
    Less,
    #[serde(rename = "eless")]
    ELess,
    #[serde(rename = "equal")]
    Equal,
    #[serde(rename = "nequal")]
    NotEqual,
    #[serde(rename = "in_range")]
    InRange,
    #[serde(rename = "not_in_range")]
    NotInRange,
    #[serde(rename = "empty")]
    Empty,
    #[serde(rename = "nempty")]
    NotEmpty,
    #[serde(rename = "crosses")]
    Crosses,
    #[serde(rename = "crosses_above")]
    CrossesAbove,
    #[serde(rename = "crosses_below")]
    CrossesBelow,
    #[serde(rename = "match")]
    Match,
    #[serde(rename = "nmatch")]
    NotMatch,
    #[serde(rename = "has")]
    Has,
    #[serde(rename = "has_none_of")]
    HasNoneOf,
    #[serde(rename = "above%")]
    AbovePercent,
    #[serde(rename = "below%")]
    BelowPercent,
    #[serde(rename = "in_range%")]
    InRangePercent,
    #[serde(rename = "not_in_range%")]
    NotInRangePercent,
    #[serde(rename = "in_day_range")]
    InDayRange,
    #[serde(rename = "in_week_range")]
    InWeekRange,
    #[serde(rename = "in_month_range")]
    InMonthRange,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FilterCondition {
    pub left: Column,
    pub operation: FilterOperator,
    pub right: Value,
}

impl FilterCondition {
    pub fn new(left: Column, operation: FilterOperator, right: Value) -> Self {
        Self {
            left,
            operation,
            right,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum LogicalOperator {
    #[serde(rename = "and")]
    And,
    #[serde(rename = "or")]
    Or,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FilterTree {
    pub operator: LogicalOperator,
    pub operands: Vec<FilterOperand>,
}

impl FilterTree {
    pub fn and<I, O>(operands: I) -> Self
    where
        I: IntoIterator<Item = O>,
        O: Into<FilterOperand>,
    {
        Self {
            operator: LogicalOperator::And,
            operands: operands.into_iter().map(Into::into).collect(),
        }
    }

    pub fn or<I, O>(operands: I) -> Self
    where
        I: IntoIterator<Item = O>,
        O: Into<FilterOperand>,
    {
        Self {
            operator: LogicalOperator::Or,
            operands: operands.into_iter().map(Into::into).collect(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilterOperand {
    Expression(FilterExpressionOperand),
    Operation(FilterOperationOperand),
}

impl Serialize for FilterOperand {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Expression(expression) => expression.serialize(serializer),
            Self::Operation(operation) => operation.serialize(serializer),
        }
    }
}

impl From<FilterCondition> for FilterOperand {
    fn from(value: FilterCondition) -> Self {
        Self::Expression(FilterExpressionOperand { expression: value })
    }
}

impl From<FilterTree> for FilterOperand {
    fn from(value: FilterTree) -> Self {
        Self::Operation(FilterOperationOperand { operation: value })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FilterExpressionOperand {
    pub expression: FilterCondition,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct FilterOperationOperand {
    pub operation: FilterTree,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum SortOrder {
    #[serde(rename = "asc")]
    Asc,
    #[serde(rename = "desc")]
    Desc,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
pub struct SortSpec {
    #[serde(rename = "sortBy")]
    pub sort_by: Column,
    #[serde(rename = "sortOrder")]
    pub sort_order: SortOrder,
    #[serde(skip_serializing_if = "Option::is_none", rename = "nullsFirst")]
    pub nulls_first: Option<bool>,
}

impl SortSpec {
    pub fn new(sort_by: Column, sort_order: SortOrder) -> Self {
        Self {
            sort_by,
            sort_order,
            nulls_first: None,
        }
    }

    pub fn nulls_first(mut self, nulls_first: bool) -> Self {
        self.nulls_first = Some(nulls_first);
        self
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn serializes_nested_filter_tree_like_tradingview() {
        let tree = FilterTree::and(vec![
            FilterOperand::from(Column::from_static("close").ge(200)),
            FilterOperand::from(FilterTree::or(vec![
                FilterOperand::from(Column::from_static("RSI").lt(60)),
                FilterOperand::from(Column::from_static("market_cap_basic").gt(1_000_000_000_u64)),
            ])),
        ]);

        let value = serde_json::to_value(tree).unwrap();
        assert_eq!(
            value,
            json!({
                "operator": "and",
                "operands": [
                    { "expression": { "left": "close", "operation": "egreater", "right": 200 } },
                    {
                        "operation": {
                            "operator": "or",
                            "operands": [
                                { "expression": { "left": "RSI", "operation": "less", "right": 60 } },
                                { "expression": { "left": "market_cap_basic", "operation": "greater", "right": 1_000_000_000_u64 } }
                            ]
                        }
                    }
                ]
            })
        );
    }
}
