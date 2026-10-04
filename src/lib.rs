pub mod actions;
pub mod config;
pub mod deviation;
pub mod dsl;
pub mod evidence;
pub mod observation;
pub mod policy;
pub mod qualification;

use evidence::EvidenceRecord;
use observation::Observation;
use policy::ActionPlan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evaluation {
    pub evidence: EvidenceRecord,
    pub plan: ActionPlan,
}

pub fn evaluate(
    previous: Option<&Observation>,
    current: &Observation,
    threshold: u64,
    requested_messages: &[String],
) -> Evaluation {
    let evidence = EvidenceRecord::evaluate(previous, current, threshold);
    let plan = policy::plan(&evidence, requested_messages);

    Evaluation { evidence, plan }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsl::parse_program;
    use crate::qualification::SentinelState;

    #[test]
    fn end_to_end_anomaly_becomes_notification_plan() -> anyhow::Result<()> {
        let previous = Observation::new("SYSTEM_USED_MEMORY_MIB", 1, 80).unwrap();
        let current = Observation::new("SYSTEM_USED_MEMORY_MIB", 2, 150).unwrap();

        let program = parse_program(
            r#"vapor sentinel() {
                if(SYSTEM_USED_MEMORY_MIB >= 100) {
                    send("CRITICAL_MEMORY_THRESHOLD");
                }
            }"#,
        )?;
        let messages = program.requested_messages(&current);
        let evaluation = evaluate(Some(&previous), &current, 100, &messages);

        assert_eq!(evaluation.evidence.state(), SentinelState::Anomalous);
        assert_eq!(evaluation.evidence.sequence(), 2);
        assert_eq!(
            evaluation.plan,
            ActionPlan::Notify {
                message: "CRITICAL_MEMORY_THRESHOLD".into()
            }
        );
        Ok(())
    }

    #[test]
    fn end_to_end_invalid_ordering_stays_unknown_and_cannot_act() -> anyhow::Result<()> {
        let previous = Observation::new("SYSTEM_USED_MEMORY_MIB", 2, 80).unwrap();
        let current = Observation::new("SYSTEM_USED_MEMORY_MIB", 1, 150).unwrap();

        let evaluation = evaluate(
            Some(&previous),
            &current,
            100,
            &["must not fire".into()],
        );

        assert_eq!(evaluation.evidence.state(), SentinelState::Unknown);
        assert_eq!(evaluation.plan, ActionPlan::NoAction);
        Ok(())
    }
}
