//! Auditable evidence records produced from deterministic qualification.

use crate::deviation::{compare, Deviation};
use crate::observation::Observation;
use crate::qualification::{qualify, Qualification, SentinelState};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EvidenceRecord {
    pub sequence: u64,
    pub metric: String,
    pub value: u64,
    pub threshold: u64,
    pub deviation: Deviation,
    pub state: SentinelState,
    pub reason: &'static str,
}

impl EvidenceRecord {
    pub fn evaluate(previous: Option<&Observation>, current: &Observation, threshold: u64) -> Self {
        let deviation = compare(previous, current);
        let qualification = qualify(Some(current), Some(&deviation), threshold);
        Self::from(current, deviation, qualification, threshold)
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
        let previous = Observation::new("SYSTEM_RAM", 3, 100);
        let current = Observation::new("SYSTEM_RAM", 4, 120);
        let e = EvidenceRecord::evaluate(Some(&previous), &current, 100);
        assert_eq!(e.sequence, 4);
        assert_eq!(e.metric, "SYSTEM_RAM");
        assert_eq!(e.value, 120);
        assert_eq!(e.threshold, 100);
        assert_eq!(e.deviation, Deviation::Increased { delta: 20 });
        assert_eq!(e.state, SentinelState::Anomalous);
    }

    #[test]
    fn first_observation_is_not_normal() {
        let current = Observation::new("SYSTEM_RAM", 1, 80);
        let e = EvidenceRecord::evaluate(None, &current, 100);
        assert_eq!(e.deviation, Deviation::NoBaseline);
        assert_eq!(e.state, SentinelState::Unknown);
    }
}
