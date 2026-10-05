//! Auditable evidence records produced from deterministic qualification.

use crate::deviation::{compare, Deviation};
use crate::dsl::Program;
use crate::observation::Observation;
use crate::qualification::{qualify, qualify_match, Qualification, SentinelState};
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EvidenceRecord {
    sequence: u64,
    metric: String,
    value: u64,
    threshold: Option<u64>,
    matched_messages: Vec<String>,
    deviation: Deviation,
    state: SentinelState,
    reason: &'static str,
}

impl EvidenceRecord {
    /// Evaluate the actual DSL predicate; there is no hidden second threshold.
    pub fn evaluate_policy(
        previous: Option<&Observation>,
        current: &Observation,
        program: &Program,
    ) -> Self {
        let messages = program.requested_messages(current);
        let deviation = compare(previous, current);
        let qualification = qualify_match(Some(current), Some(&deviation), !messages.is_empty());
        Self {
            sequence: current.sequence(),
            metric: current.metric().to_owned(),
            value: current.value(),
            threshold: None,
            matched_messages: messages,
            deviation,
            state: qualification.state,
            reason: qualification.reason,
        }
    }

    pub fn matched_messages(&self) -> &[String] {
        &self.matched_messages
    }

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

    pub fn threshold(&self) -> Option<u64> {
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
            sequence: observation.sequence(),
            metric: observation.metric().to_owned(),
            value: observation.value(),
            threshold: Some(threshold),
            matched_messages: Vec::new(),
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
        assert_eq!(e.threshold(), Some(100));
        assert_eq!(e.deviation(), &Deviation::Increased { delta: 20 });
        assert_eq!(e.state(), SentinelState::Anomalous);
        assert_eq!(e.reason(), "policy condition matched");
    }

    #[test]
    fn first_observation_is_not_normal() {
        let current = Observation::new("SYSTEM_RAM", 1, 80).unwrap();
        let e = EvidenceRecord::evaluate(None, &current, 100);
        assert_eq!(e.deviation(), &Deviation::NoBaseline);
        assert_eq!(e.state(), SentinelState::Unknown);
    }
}
