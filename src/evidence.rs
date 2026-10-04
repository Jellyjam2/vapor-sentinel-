//! Auditable evidence records produced from deterministic qualification.

use crate::deviation::Deviation;
use crate::observation::Observation;
use crate::qualification::{Qualification, SentinelState};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceRecord {
    pub sequence: u64,
    pub metric: String,
    pub value: u64,
    pub deviation: Deviation,
    pub state: SentinelState,
    pub reason: &'static str,
}

impl EvidenceRecord {
    pub fn from(observation: &Observation, deviation: Deviation, qualification: Qualification) -> Self {
        Self { sequence: observation.sequence, metric: observation.metric.clone(), value: observation.value, deviation, state: qualification.state, reason: qualification.reason }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn evidence_preserves_qualification_and_observation_identity() {
        let obs = Observation::new("SYSTEM_RAM", 4, 120);
        let q = Qualification { state: SentinelState::Anomalous, reason: "threshold exceeded" };
        let e = EvidenceRecord::from(&obs, Deviation::Increased { delta: 20 }, q);
        assert_eq!(e.sequence, 4);
        assert_eq!(e.metric, "SYSTEM_RAM");
        assert_eq!(e.value, 120);
        assert_eq!(e.state, SentinelState::Anomalous);
    }
}
