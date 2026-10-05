//! Stateful delivery scheduling is separate from evidence and pure policy.
use crate::{
    actions::ActionExecution, delivery::DeliveryResult, policy::ActionPlan,
    qualification::SentinelState,
};

pub struct Lifecycle {
    repeat_secs: u64,
    retry_secs: u64,
    recovery_samples: u32,
    normal_samples: u32,
    active: bool,
    pending: bool,
    next_attempt: u64,
    last_plan: ActionPlan,
}
impl Lifecycle {
    pub fn new(repeat_secs: u64, retry_secs: u64, recovery_samples: u32) -> Self {
        Self {
            repeat_secs,
            retry_secs,
            recovery_samples,
            normal_samples: 0,
            active: false,
            pending: false,
            next_attempt: 0,
            last_plan: ActionPlan::NoAction,
        }
    }
    pub fn candidate(
        &mut self,
        state: SentinelState,
        plan: &ActionPlan,
        now: u64,
    ) -> (ActionPlan, &'static str) {
        if state == SentinelState::Unknown {
            self.normal_samples = 0;
            return (ActionPlan::NoAction, "monitoring_unknown");
        }
        if state == SentinelState::Normal {
            self.normal_samples = self.normal_samples.saturating_add(1);
        } else {
            self.normal_samples = 0;
        }
        if self.pending {
            return (ActionPlan::NoAction, "delivery_pending");
        }
        let candidate = match plan {
            ActionPlan::Notify { .. } => plan.clone(),
            _ if self.active && self.normal_samples >= self.recovery_samples => {
                ActionPlan::Recovered {
                    message: "Policy condition cleared".into(),
                }
            }
            _ => return (ActionPlan::NoAction, "no_action"),
        };
        if candidate == self.last_plan && now < self.next_attempt {
            return (ActionPlan::NoAction, "cooldown");
        }
        (candidate, "ready")
    }
    pub fn submitted(&mut self, plan: &ActionPlan) {
        self.pending = true;
        self.last_plan = plan.clone();
        if matches!(plan, ActionPlan::Notify { .. }) {
            self.active = true;
        }
    }
    pub fn completed(&mut self, result: &DeliveryResult, now: u64) {
        self.pending = false;
        let failed = matches!(result.outcome, ActionExecution::Failed { .. });
        self.next_attempt = now.saturating_add(if failed {
            self.retry_secs
        } else {
            self.repeat_secs
        });
        if !failed && matches!(result.plan, ActionPlan::Recovered { .. }) {
            self.active = false;
            self.last_plan = ActionPlan::NoAction;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn alert() -> ActionPlan {
        ActionPlan::Notify {
            message: "high".into(),
        }
    }
    fn finish(tracker: &mut Lifecycle, plan: ActionPlan, outcome: ActionExecution, now: u64) {
        tracker.completed(
            &DeliveryResult {
                event_id: "event".into(),
                plan,
                outcome,
            },
            now,
        );
    }
    #[test]
    fn persistent_alerts_cool_down_and_recovery_is_debounced() {
        let mut tracker = Lifecycle::new(300, 30, 2);
        assert_eq!(
            tracker.candidate(SentinelState::Anomalous, &alert(), 0).0,
            alert()
        );
        tracker.submitted(&alert());
        assert_eq!(
            tracker.candidate(SentinelState::Anomalous, &alert(), 1).1,
            "delivery_pending"
        );
        finish(&mut tracker, alert(), ActionExecution::Delivered, 1);
        assert_eq!(
            tracker.candidate(SentinelState::Anomalous, &alert(), 100).1,
            "cooldown"
        );
        assert_eq!(
            tracker.candidate(SentinelState::Anomalous, &alert(), 301).0,
            alert()
        );
        assert_eq!(
            tracker
                .candidate(SentinelState::Normal, &ActionPlan::NoAction, 302)
                .0,
            ActionPlan::NoAction
        );
        let recovery = tracker
            .candidate(SentinelState::Normal, &ActionPlan::NoAction, 303)
            .0;
        assert!(matches!(recovery, ActionPlan::Recovered { .. }));
        tracker.submitted(&recovery);
        finish(&mut tracker, recovery, ActionExecution::Delivered, 304);
        assert_eq!(
            tracker
                .candidate(SentinelState::Normal, &ActionPlan::NoAction, 305)
                .0,
            ActionPlan::NoAction
        );
        assert_eq!(
            tracker.candidate(SentinelState::Anomalous, &alert(), 306).0,
            alert()
        );
    }
    #[test]
    fn failures_back_off_and_unknown_does_not_resolve_an_alert() {
        let mut tracker = Lifecycle::new(300, 30, 2);
        tracker.submitted(&alert());
        finish(
            &mut tracker,
            alert(),
            ActionExecution::Failed {
                reason: "timeout".into(),
            },
            0,
        );
        assert_eq!(
            tracker.candidate(SentinelState::Anomalous, &alert(), 29).1,
            "cooldown"
        );
        assert_eq!(
            tracker.candidate(SentinelState::Anomalous, &alert(), 30).0,
            alert()
        );
        assert_eq!(
            tracker
                .candidate(SentinelState::Unknown, &ActionPlan::NoAction, 31)
                .1,
            "monitoring_unknown"
        );
        assert_eq!(
            tracker
                .candidate(SentinelState::Normal, &ActionPlan::NoAction, 32)
                .0,
            ActionPlan::NoAction
        );
    }
}
