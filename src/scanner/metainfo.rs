use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScannerMetainfo {
    #[serde(default)]
    pub financial_currency: Option<String>,
    #[serde(default)]
    pub fields: Vec<ScannerFieldMetainfo>,
}

impl ScannerMetainfo {
    pub fn field(&self, name: &str) -> Option<&ScannerFieldMetainfo> {
        self.fields.iter().find(|field| field.name == name)
    }

    pub fn supports_field(&self, name: &str) -> bool {
        self.field(name).is_some()
    }

    pub fn fields_by_type<'a>(
        &'a self,
        field_type: &'a str,
    ) -> impl Iterator<Item = &'a ScannerFieldMetainfo> + 'a {
        self.fields
            .iter()
            .filter(move |field| field.field_type.as_str() == field_type)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ScannerFieldMetainfo {
    #[serde(rename = "n")]
    pub name: String,
    #[serde(rename = "t")]
    pub field_type: ScannerFieldType,
    #[serde(rename = "r", default)]
    pub range: Option<Value>,
}

impl ScannerFieldMetainfo {
    pub fn enum_values(&self) -> Option<&[Value]> {
        self.range.as_ref()?.as_array().map(Vec::as_slice)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ScannerFieldType(String);

impl ScannerFieldType {
    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_numeric_like(&self) -> bool {
        matches!(
            self.as_str(),
            "number" | "percent" | "price" | "fundamental_price" | "num_slice"
        )
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;

    #[test]
    fn metainfo_looks_up_fields_and_types() {
        let metainfo = ScannerMetainfo {
            financial_currency: Some("USD".to_owned()),
            fields: vec![
                ScannerFieldMetainfo {
                    name: "close".to_owned(),
                    field_type: ScannerFieldType("price".to_owned()),
                    range: None,
                },
                ScannerFieldMetainfo {
                    name: "country".to_owned(),
                    field_type: ScannerFieldType("text".to_owned()),
                    range: Some(json!(["United States", "Canada"])),
                },
            ],
        };

        assert!(metainfo.supports_field("close"));
        assert_eq!(
            metainfo
                .field("country")
                .and_then(ScannerFieldMetainfo::enum_values),
            Some(&[json!("United States"), json!("Canada")][..]),
        );
        assert_eq!(metainfo.fields_by_type("price").count(), 1);
        assert!(
            metainfo
                .field("close")
                .is_some_and(|field| field.field_type.is_numeric_like())
        );
    }
}
