use std::collections::BTreeMap;

use serde::Deserialize;
use serde_json::Value;

use crate::error::Error;
use crate::scanner::field::Column;

#[derive(Debug, Clone, PartialEq)]
pub struct ScanResponse {
    pub total_count: usize,
    pub rows: Vec<ScanRow>,
    pub params: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct ScanRow {
    #[serde(rename = "s")]
    pub symbol: String,
    #[serde(
        rename = "d",
        default = "Vec::new",
        deserialize_with = "deserialize_nullable_vec"
    )]
    pub values: Vec<Value>,
}

impl ScanRow {
    pub fn as_record(&self, columns: &[Column]) -> BTreeMap<String, Value> {
        columns
            .iter()
            .zip(self.values.iter())
            .map(|(column, value)| (column.as_str().to_owned(), value.clone()))
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct RawScanResponse {
    #[serde(rename = "totalCount", default)]
    pub total_count: usize,
    #[serde(default = "Vec::new", deserialize_with = "deserialize_nullable_vec")]
    pub data: Vec<ScanRow>,
    #[serde(default)]
    pub params: Option<Value>,
    #[serde(default)]
    pub error: Option<String>,
}

impl RawScanResponse {
    pub fn into_response(self) -> crate::error::Result<ScanResponse> {
        if let Some(error) = self.error {
            return Err(Error::ApiMessage(error));
        }

        Ok(ScanResponse {
            total_count: self.total_count,
            rows: self.data,
            params: self.params,
        })
    }
}

fn deserialize_nullable_vec<'de, D, T>(deserializer: D) -> std::result::Result<Vec<T>, D::Error>
where
    D: serde::Deserializer<'de>,
    T: Deserialize<'de>,
{
    Ok(Option::<Vec<T>>::deserialize(deserializer)?.unwrap_or_default())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn converts_rows_into_named_records() {
        let row = ScanRow {
            symbol: "NASDAQ:AAPL".to_owned(),
            values: vec![Value::String("AAPL".to_owned()), Value::from(247.99)],
        };
        let record = row.as_record(&[Column::from_static("name"), Column::from_static("close")]);
        assert_eq!(record["name"], Value::String("AAPL".to_owned()));
        assert_eq!(record["close"], Value::from(247.99));
    }

    #[test]
    fn raw_response_handles_null_data() {
        let raw: RawScanResponse =
            serde_json::from_str(r#"{"totalCount":0,"error":"Unknown field","data":null}"#)
                .unwrap();
        assert!(raw.data.is_empty());
    }
}
