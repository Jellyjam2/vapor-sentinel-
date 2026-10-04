//! External side effects.
//!
//! Action execution is isolated from observation, qualification, evidence,
//! and policy. Network effects are disabled unless explicitly enabled and
//! configured.

use crate::evidence::EvidenceRecord;
use crate::policy::ActionPlan;
use crate::qualification::SentinelState;
use anyhow::{bail, Context, Result};
use serde::Serialize;
use serde_json::json;
use std::env;
use std::time::Duration;

const ACTION_MESSAGE_MAX_BYTES: usize = 4096;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ActionExecution {
    Executed,
    Skipped,
}

fn actions_enabled() -> bool {
    env::var("VAPOR_SENTINEL_ENABLE_ACTIONS").as_deref() == Ok("1")
}

pub fn execute(plan: &ActionPlan, evidence: &EvidenceRecord) -> Result<ActionExecution> {
    match plan {
        ActionPlan::NoAction => Ok(ActionExecution::Skipped),
        ActionPlan::Notify { message } => {
            if message.is_empty() || message.len() > ACTION_MESSAGE_MAX_BYTES {
                bail!("notification message exceeds action boundary");
            }
            if message.chars().any(char::is_control) {
                bail!("notification message contains control characters");
            }
            if evidence.state() != SentinelState::Anomalous {
                bail!("notification plan requires anomalous evidence");
            }
            send_webhook(message, evidence)
        }
    }
}

fn send_webhook(message: &str, evidence: &EvidenceRecord) -> Result<ActionExecution> {
    if !actions_enabled() {
        return Ok(ActionExecution::Skipped);
    }

    let Some(url) = env::var_os("VAPOR_SENTINEL_WEBHOOK_URL") else {
        return Ok(ActionExecution::Skipped);
    };
    let url = url
        .into_string()
        .map_err(|_| anyhow::anyhow!("webhook URL must be valid UTF-8"))?;

    if !url.starts_with("https://") {
        bail!("VAPOR_SENTINEL_WEBHOOK_URL must use HTTPS");
    }

    let payload = json!({
        "metric": evidence.metric(),
        "sequence": evidence.sequence(),
        "value": evidence.value(),
        "threshold": evidence.threshold(),
        "state": evidence.state(),
        "message": message,
    });

    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .https_only(true)
        .redirects(0)
        .build();

    let response = agent
        .post(&url)
        .set("Content-Type", "application/json")
        .send_string(&payload.to_string())
        .with_context(|| "webhook request failed")?;

    if !(200..300).contains(&response.status()) {
        bail!("webhook returned HTTP {}", response.status());
    }

    Ok(ActionExecution::Executed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observation::Observation;
    use crate::qualification::SentinelState;

    fn evidence_with_state(state: SentinelState) -> EvidenceRecord {
        let previous = Observation::new("SYSTEM_USED_MEMORY_MIB", 1, 80).unwrap();
        let current = Observation::new("SYSTEM_USED_MEMORY_MIB", 2, 120).unwrap();
        let evidence = EvidenceRecord::evaluate(Some(&previous), &current, 100);

        if state == SentinelState::Anomalous {
            evidence
        } else {
            EvidenceRecord::evaluate(
                Some(&previous),
                &Observation::new("SYSTEM_USED_MEMORY_MIB", 2, 80).unwrap(),
                100,
            )
        }
    }

    #[test]
    fn notification_requires_anomalous_evidence() {
        let evidence = evidence_with_state(SentinelState::Normal);
        let plan = ActionPlan::Notify {
            message: "must not fire".into(),
        };

        let error = execute(&plan, &evidence).expect_err("non-anomalous notification must fail");
        assert!(error.to_string().contains("anomalous evidence"));
    }

    #[test]
    fn oversized_or_controlled_notification_is_rejected() {
        let evidence = evidence_with_state(SentinelState::Anomalous);
        let oversized = ActionPlan::Notify { message: "x".repeat(ACTION_MESSAGE_MAX_BYTES + 1) };
        assert!(execute(&oversized, &evidence).is_err());
        let controlled = ActionPlan::Notify { message: "bad\nmessage".into() };
        assert!(execute(&controlled, &evidence).is_err());
    }

    #[test]
    fn no_action_is_always_skipped() {
        let evidence = EvidenceRecord::evaluate(
            None,
            &Observation::new("SYSTEM_USED_MEMORY_MIB", 1, 120).unwrap(),
            100,
        );
        assert_eq!(
            execute(&ActionPlan::NoAction, &evidence).unwrap(),
            ActionExecution::Skipped
        );
    }
}
