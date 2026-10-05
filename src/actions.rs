//! HTTPS delivery only. Endpoint configuration is validated once; credentials
//! and response bodies are never included in diagnostic errors.
use crate::{
    evidence::EvidenceRecord,
    policy::{ActionPlan, MAX_NOTIFICATION_BYTES},
    qualification::SentinelState,
};
use anyhow::{bail, Result};
use serde::{Deserialize, Serialize};
use std::{env, time::Duration};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum ActionExecution {
    Delivered,
    Skipped { reason: String },
    Failed { reason: String },
}

pub fn validate(plan: &ActionPlan, evidence: &EvidenceRecord) -> Result<()> {
    let message = match plan {
        ActionPlan::NoAction => return Ok(()),
        ActionPlan::Notify { message } => {
            if evidence.state() != SentinelState::Anomalous {
                bail!("notification requires anomalous evidence");
            }
            message
        }
        ActionPlan::Recovered { message } => {
            if evidence.state() != SentinelState::Normal {
                bail!("recovery requires normal evidence");
            }
            message
        }
    };
    if message.is_empty()
        || message.len() > MAX_NOTIFICATION_BYTES
        || message.chars().any(char::is_control)
    {
        bail!("invalid notification message");
    }
    Ok(())
}

pub struct Webhook {
    endpoint: Option<url::Url>,
    agent: ureq::Agent,
}
impl Webhook {
    pub fn disabled() -> Self {
        Self {
            endpoint: None,
            agent: agent(),
        }
    }
    pub fn configured(endpoint: &str) -> Result<Self> {
        let url = url::Url::parse(endpoint).map_err(|_| anyhow::anyhow!("invalid webhook URL"))?;
        if url.scheme() != "https"
            || url.host_str().is_none()
            || !url.username().is_empty()
            || url.password().is_some()
            || url.fragment().is_some()
        {
            bail!("webhook must be HTTPS with a host, without embedded credentials or fragment");
        }
        Ok(Self {
            endpoint: Some(url),
            agent: agent(),
        })
    }
    pub fn from_env() -> Result<Self> {
        match env::var("VAPOR_SENTINEL_ENABLE_ACTIONS").as_deref() {
            Err(env::VarError::NotPresent) | Ok("0") => Ok(Self::disabled()),
            Ok("1") => {
                let endpoint = env::var("VAPOR_SENTINEL_WEBHOOK_URL").map_err(|_| {
                    anyhow::anyhow!("enabled actions require VAPOR_SENTINEL_WEBHOOK_URL")
                })?;
                Self::configured(&endpoint)
            }
            _ => bail!("VAPOR_SENTINEL_ENABLE_ACTIONS must be 0 or 1"),
        }
    }
    pub fn deliver(
        &mut self,
        notification: &crate::delivery::Notification,
    ) -> Result<ActionExecution> {
        validate(&notification.plan, &notification.evidence)?;
        if matches!(notification.plan, ActionPlan::NoAction) {
            return Ok(ActionExecution::Skipped {
                reason: "no_action".into(),
            });
        }
        let Some(endpoint) = &self.endpoint else {
            return Ok(ActionExecution::Skipped {
                reason: "actions_disabled".into(),
            });
        };
        let payload = serde_json::to_string(notification)?;
        match self
            .agent
            .post(endpoint.as_str())
            .set("Content-Type", "application/json")
            .send_string(&payload)
        {
            Ok(response) if (200..300).contains(&response.status()) => {
                Ok(ActionExecution::Delivered)
            }
            Ok(response) | Err(ureq::Error::Status(_, response)) => {
                bail!("webhook returned HTTP {}", response.status())
            }
            Err(ureq::Error::Transport(_)) => {
                bail!("webhook transport failed (timeout, DNS, connection, or TLS)")
            }
        }
    }
}
fn agent() -> ureq::Agent {
    ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(10))
        .https_only(true)
        .redirects(0)
        .build()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::observation::Observation;
    fn evidence(value: u64) -> EvidenceRecord {
        EvidenceRecord::evaluate(
            Some(&Observation::new("RAM", 1, 80).unwrap()),
            &Observation::new("RAM", 2, value).unwrap(),
            100,
        )
    }
    #[test]
    fn action_types_require_matching_evidence() {
        let notify = ActionPlan::Notify {
            message: "alert".into(),
        };
        let recovered = ActionPlan::Recovered {
            message: "recovered".into(),
        };
        assert!(validate(&notify, &evidence(120)).is_ok());
        assert!(validate(&notify, &evidence(80)).is_err());
        assert!(validate(&recovered, &evidence(80)).is_ok());
        assert!(validate(&recovered, &evidence(120)).is_err());
    }
    #[test]
    fn oversized_and_control_messages_are_rejected() {
        for message in [
            "".into(),
            "x".repeat(MAX_NOTIFICATION_BYTES + 1),
            "bad\nmessage".into(),
        ] {
            assert!(validate(&ActionPlan::Notify { message }, &evidence(120)).is_err());
        }
    }
    #[test]
    fn endpoint_validation_does_not_expose_secrets() {
        for url in [
            "http://example.com/token",
            "https://user:secret@example.com",
            "https://example.com/#secret",
            "not a URL",
        ] {
            let error = Webhook::configured(url).err().unwrap().to_string();
            assert!(!error.contains("secret"));
        }
        assert!(Webhook::configured("https://example.com/webhook?token=secret").is_ok());
    }
}
