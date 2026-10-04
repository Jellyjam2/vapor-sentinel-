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
    match evidence.state {
        SentinelState::Anomalous => {
            let message = if requested_messages.is_empty() {
                format!(
                    "{} exceeded sentinel threshold: {} > {}",
                    evidence.metric, evidence.value, evidence.threshold
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
    use crate::deviation::Deviation;

    fn anomaly() -> EvidenceRecord {
        EvidenceRecord {
            sequence: 2,
            metric: "SYSTEM_USED_MEMORY_MB".into(),
            value: 200,
            threshold: 100,
            deviation: Deviation::Increased { delta: 40 },
            state: SentinelState::Anomalous,
            reason: "threshold exceeded",
        }
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
        let mut evidence = anomaly();
        evidence.state = SentinelState::Degraded;

        assert_eq!(plan(&evidence, &[]), ActionPlan::NoAction);
    }
}
