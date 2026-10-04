//! Pure policy boundary between evidence and external effects.

use crate::evidence::EvidenceRecord;
use crate::qualification::SentinelState;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum ActionPlan {
    NoAction,
    Notify { message: String },
}

pub fn plan(evidence: &EvidenceRecord, requested_messages: &[String]) -> ActionPlan {
    match evidence.state() {
        SentinelState::Anomalous => {
            let message = if requested_messages.is_empty() {
                format!(
                    "{} exceeded sentinel threshold: {} > {}",
                    evidence.metric(),
                    evidence.value(),
                    evidence.threshold()
                )
            } else {
                requested_messages.join(" | ")
            };

            ActionPlan::Notify { message }
        }
        SentinelState::Normal | SentinelState::Degraded | SentinelState::Unknown => {
            ActionPlan::NoAction
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observation::Observation;

    fn anomaly() -> EvidenceRecord {
        let previous = Observation::new("SYSTEM_USED_MEMORY_MB", 1, 80);
        let current = Observation::new("SYSTEM_USED_MEMORY_MB", 2, 200);
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
    fn non_anomalous_evidence_produces_no_action() {
        let previous = Observation::new("SYSTEM_USED_MEMORY_MB", 1, 80);
        let current = Observation::new("SYSTEM_USED_MEMORY_MB", 2, 90);
        let evidence = EvidenceRecord::evaluate(Some(&previous), &current, 100);

        assert_eq!(plan(&evidence, &[]), ActionPlan::NoAction);
    }

    #[test]
    fn unknown_evidence_cannot_trigger_action() {
        let current = Observation::new("SYSTEM_USED_MEMORY_MB", 1, 200);
        let evidence = EvidenceRecord::evaluate(None, &current, 100);

        assert_eq!(plan(&evidence, &["must not fire".into()]), ActionPlan::NoAction);
    }
}
