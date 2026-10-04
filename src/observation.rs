//! Canonical observations emitted by the monitoring boundary.

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Observation {
    pub metric: String,
    pub sequence: u64,
    pub value: u64,
}

impl Observation {
    pub fn new(metric: impl Into<String>, sequence: u64, value: u64) -> Self {
        Self {
            metric: metric.into(),
            sequence,
            value,
        }
    }
}
