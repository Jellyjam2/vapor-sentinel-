//! Auditable evidence records produced from deterministic qualification.

use crate::deviation::{compare, Deviation};
use crate::observation::Observation;
use crate::qualification::{qualify, Qualification, SentinelState};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EvidenceRecord {
    sequence: u64,
    metric: String,
    value: u64,
    threshold: u64,
    deviation: Deviation,
    state: SentinelState,
    reason: &'static str,
}

impl EvidenceRecord {
    pub fn evaluate(previous: Option<&Observation>, current: &Observation, threshold: u64) -> Self {
        let deviation = compare(previous, current);
        let qualification = qualify(Some(current), Some(&deviation), threshold);
        Self::from(current, deviation, qualification, threshold)
    }

    pub fn sequence(&self) -> u64 {
        self.sequence
    }

    pub fn metric(&self) -> &str {
        &self.metric
    }

    pub fn value(&self) -> u64 {
        self.value
    }

    pub fn threshold(&self) -> u64 {
        self.threshold
    }

    pub fn deviation(&self) -> &Deviation {
        &self.deviation
    }

    pub fn state(&self) -> SentinelState {
        self.state
    }

    pub fn reason(&self) -> &'static str {
        self.reason
    }

    fn from(
        observation: &Observation,
        deviation: Deviation,
        qualification: Qualification,
        threshold: u64,
    ) -> Self {
        Self {
            sequence: observation.sequence,
            metric: observation.metric.clone(),
            value: observation.value,
            threshold,
            deviation,
            state: qualification.state,
            reason: qualification.reason,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evaluation_binds_observation_deviation_and_qualification() {
        let previous = Observation::new("SYSTEM_RAM", 3, 100).unwrap();
        let current = Observation::new("SYSTEM_RAM", 4, 120).unwrap();
        let e = EvidenceRecord::evaluate(Some(&previous), &current, 100);
        assert_eq!(e.sequence(), 4);
        assert_eq!(e.metric(), "SYSTEM_RAM");
        assert_eq!(e.value(), 120);
        assert_eq!(e.threshold(), 100);
        assert_eq!(e.deviation(), &Deviation::Increased { delta: 20 });
        assert_eq!(e.state(), SentinelState::Anomalous);
        assert_eq!(e.reason(), "threshold exceeded");
    }

    #[test]
    fn first_observation_is_not_normal() {
        let current = Observation::new("SYSTEM_RAM", 1, 80).unwrap();
        let e = EvidenceRecord::evaluate(None, &current, 100);
        assert_eq!(e.deviation(), &Deviation::NoBaseline);
        assert_eq!(e.state(), SentinelState::Unknown);
    }
}
