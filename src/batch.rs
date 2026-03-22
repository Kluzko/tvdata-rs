use std::collections::BTreeMap;

use crate::error::{Error, ErrorKind};
use crate::scanner::Ticker;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolFailure {
    pub symbol: Ticker,
    pub kind: ErrorKind,
    pub message: String,
    pub retryable: bool,
}

impl SymbolFailure {
    pub fn from_error(symbol: Ticker, error: Error) -> Self {
        let kind = error.kind();
        let retryable = error.is_retryable();
        Self {
            symbol,
            kind,
            message: error.to_string(),
            retryable,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BatchResult<T> {
    pub successes: BTreeMap<Ticker, T>,
    pub missing: Vec<Ticker>,
    pub failures: Vec<SymbolFailure>,
}

impl<T> Default for BatchResult<T> {
    fn default() -> Self {
        Self {
            successes: BTreeMap::new(),
            missing: Vec::new(),
            failures: Vec::new(),
        }
    }
}

impl<T> BatchResult<T> {
    pub fn is_empty(&self) -> bool {
        self.successes.is_empty() && self.missing.is_empty() && self.failures.is_empty()
    }

    pub fn len(&self) -> usize {
        self.successes.len() + self.missing.len() + self.failures.len()
    }
}
