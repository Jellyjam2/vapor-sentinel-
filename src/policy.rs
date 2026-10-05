//! Pure policy boundary between evidence and external effects.

use crate::evidence::EvidenceRecord;
use crate::qualification::SentinelState;
use serde::Serialize;

pub const MAX_NOTIFICATION_BYTES: usize = 4096;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ActionPlan {
    NoAction,
    Notify { message: String },
    Recovered { message: String },
}

pub fn plan(evidence: &EvidenceRecord, requested_messages: &[String]) -> ActionPlan {
    match evidence.state() {
        SentinelState::Anomalous if !requested_messages.is_empty() => ActionPlan::Notify {
            message: requested_messages.join(" | "),
        },
        SentinelState::Anomalous
        | SentinelState::Normal
        | SentinelState::Degraded
        | SentinelState::Unknown => ActionPlan::NoAction,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observation::Observation;

    fn anomaly() -> EvidenceRecord {
        let previous = Observation::new("SYSTEM_USED_MEMORY_MIB", 1, 80).unwrap();
        let current = Observation::new("SYSTEM_USED_MEMORY_MIB", 2, 200).unwrap();
        EvidenceRecord::evaluate(Some(&previous), &current, 100)
    }

    #[test]
    fn anomalous_evidence_produces_notification_plan() {
        assert_eq!(
            plan(&anomaly(), &["configured alert".into()]),
            ActionPlan::Notify {
                message: "configured alert".into()
            }
        );
    }

    #[test]
    fn anomalous_evidence_without_intent_produces_no_action() {
        assert_eq!(plan(&anomaly(), &[]), ActionPlan::NoAction);
    }

    #[test]
    fn non_anomalous_evidence_produces_no_action() {
        let previous = Observation::new("SYSTEM_USED_MEMORY_MIB", 1, 80).unwrap();
        let current = Observation::new("SYSTEM_USED_MEMORY_MIB", 2, 90).unwrap();
        let evidence = EvidenceRecord::evaluate(Some(&previous), &current, 100);

        assert_eq!(plan(&evidence, &[]), ActionPlan::NoAction);
    }

    #[test]
    fn unknown_evidence_cannot_trigger_action() {
        let current = Observation::new("SYSTEM_USED_MEMORY_MIB", 1, 200).unwrap();
        let evidence = EvidenceRecord::evaluate(None, &current, 100);

        assert_eq!(
            plan(&evidence, &["must not fire".into()]),
            ActionPlan::NoAction
        );
    }
}
