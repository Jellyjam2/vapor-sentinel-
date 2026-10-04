//! Authority boundary between qualification and external actions.

use crate::evidence::EvidenceRecord;
use crate::qualification::SentinelState;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActionPlan {
    NoAction,
    Notify { message: String },
}

pub fn plan(evidence: &EvidenceRecord) -> ActionPlan {
    match evidence.state {
        SentinelState::Anomalous => ActionPlan::Notify {
            message: format!("{} exceeded sentinel threshold", evidence.metric),
        },
        SentinelState::Normal | SentinelState::Degraded | SentinelState::Unknown => ActionPlan::NoAction,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deviation::Deviation;

    #[test]
    fn anomalous_evidence_produces_plan_without_executing_it() {
        let e = EvidenceRecord {
            sequence: 1,
            metric: "SYSTEM_RAM".into(),
            value: 200,
            deviation: Deviation::Unchanged,
            state: SentinelState::Anomalous,
            reason: "threshold exceeded",
        };
        assert_eq!(
            plan(&e),
            ActionPlan::Notify { message: "SYSTEM_RAM exceeded sentinel threshold".into() }
        );
    }
}
