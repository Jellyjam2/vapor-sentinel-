//! Canonical observations emitted by the monitoring boundary.
//!
//! The runtime observation source maintains a monotonically increasing
//! sequence. The observation record carries a validated identity and
//! sequence value; ordering between records is validated by the deviation layer.

use anyhow::{bail, Result};
use serde::Serialize;

const MAX_METRIC_BYTES: usize = 128;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Observation {
    metric: String,
    sequence: u64,
    value: u64,
}

impl Observation {
    pub fn new(metric: impl Into<String>, sequence: u64, value: u64) -> Result<Self> {
        let metric = metric.into();

        if metric.is_empty() {
            bail!("observation metric cannot be empty");
        }
        if metric.len() > MAX_METRIC_BYTES {
            bail!("observation metric exceeds {} bytes", MAX_METRIC_BYTES);
        }
        if metric.chars().any(char::is_control) {
            bail!("observation metric contains control characters");
        }
        if sequence == 0 {
            bail!("observation sequence must be greater than zero");
        }

        Ok(Self {
            metric,
            sequence,
            value,
        })
    }

    pub fn metric(&self) -> &str {
        &self.metric
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn value(&self) -> u64 {
        self.value
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn valid_observation_is_accepted() {
        let observation = Observation::new("SYSTEM_RAM", 1, 80).unwrap();
        assert_eq!(observation.metric(), "SYSTEM_RAM");
        assert_eq!(observation.sequence(), 1);
        assert_eq!(observation.value(), 80);
    }

    #[test]
    fn empty_metric_is_rejected() {
        assert!(Observation::new("", 1, 80).is_err());
    }

    #[test]
    fn zero_sequence_is_rejected() {
        assert!(Observation::new("SYSTEM_RAM", 0, 80).is_err());
    }

    #[test]
    fn control_character_in_metric_is_rejected() {
        assert!(Observation::new("SYSTEM\u{0000}RAM", 1, 80).is_err());
    }

    #[test]
    fn oversized_metric_is_rejected() {
        assert!(Observation::new("X".repeat(MAX_METRIC_BYTES + 1), 1, 80).is_err());
    }
}
