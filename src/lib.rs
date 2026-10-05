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
    program: &dsl::Program,
) -> Evaluation {
    let evidence = EvidenceRecord::evaluate_policy(previous, current, program);
    let plan = policy::plan(&evidence, evidence.matched_messages());

    Evaluation { evidence, plan }
}

pub mod delivery;
pub mod events;
pub mod lifecycle;
