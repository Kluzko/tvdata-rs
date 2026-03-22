use time::OffsetDateTime;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DataSourceKind {
    Scanner,
    QuoteWebSocket,
    HistoryWebSocket,
    Composed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HistoryKind {
    Snapshot,
    Native,
    Derived,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DataLineage {
    pub source: DataSourceKind,
    pub history_kind: HistoryKind,
    pub as_of: OffsetDateTime,
    pub effective_at: Option<OffsetDateTime>,
}

impl DataLineage {
    pub fn new(
        source: DataSourceKind,
        history_kind: HistoryKind,
        as_of: OffsetDateTime,
        effective_at: Option<OffsetDateTime>,
    ) -> Self {
        Self {
            source,
            history_kind,
            as_of,
            effective_at,
        }
    }
}
