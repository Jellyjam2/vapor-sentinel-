//! A single bounded worker keeps a slow webhook off the observation thread.
use crate::{actions::ActionExecution, evidence::EvidenceRecord, policy::ActionPlan};
use anyhow::{Context, Result};
use serde::Serialize;
use std::{
    sync::mpsc::{self, Receiver, SyncSender, TrySendError},
    thread::{self, JoinHandle},
};

#[derive(Debug, Clone, Serialize)]
pub struct Notification {
    pub schema_version: u32,
    pub event_id: String,
    pub run_id: String,
    pub source_id: String,
    pub observed_at_ms: u64,
    pub policy_sha256: String,
    pub evidence: EvidenceRecord,
    pub plan: ActionPlan,
}
#[derive(Debug, Clone, Serialize)]
pub struct DeliveryResult {
    pub event_id: String,
    pub plan: ActionPlan,
    pub outcome: ActionExecution,
}
pub struct DeliveryWorker {
    sender: SyncSender<Notification>,
    results: Receiver<DeliveryResult>,
    handle: JoinHandle<()>,
}
impl DeliveryWorker {
    pub fn spawn<F>(mut deliver: F) -> Self
    where
        F: FnMut(&Notification) -> Result<ActionExecution> + Send + 'static,
    {
        let (sender, jobs) = mpsc::sync_channel::<Notification>(1);
        let (completed, results) = mpsc::sync_channel(2);
        let handle = thread::spawn(move || {
            while let Ok(job) = jobs.recv() {
                let outcome = deliver(&job).unwrap_or_else(|error| ActionExecution::Failed {
                    reason: error.to_string(),
                });
                if completed
                    .send(DeliveryResult {
                        event_id: job.event_id,
                        plan: job.plan,
                        outcome,
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            sender,
            results,
            handle,
        }
    }
    pub fn enqueue(&self, notification: Notification) -> Result<bool> {
        match self.sender.try_send(notification) {
            Ok(()) => Ok(true),
            Err(TrySendError::Full(_)) => Ok(false),
            Err(TrySendError::Disconnected(_)) => anyhow::bail!("delivery worker stopped"),
        }
    }
    pub fn completed(&self) -> impl Iterator<Item = DeliveryResult> + '_ {
        self.results.try_iter()
    }
    pub fn finish(self) -> Result<Vec<DeliveryResult>> {
        drop(self.sender);
        let results = self.results.into_iter().collect();
        self.handle
            .join()
            .map_err(|_| anyhow::anyhow!("delivery worker panicked"))
            .context("shutdown delivery")?;
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{actions::Webhook, observation::Observation};
    use std::sync::mpsc;
    fn job(id: &str) -> Notification {
        Notification {
            schema_version: 1,
            event_id: id.into(),
            run_id: "run".into(),
            source_id: "test".into(),
            observed_at_ms: 1,
            policy_sha256: "digest".into(),
            evidence: EvidenceRecord::evaluate(
                Some(&Observation::new("RAM", 1, 80).unwrap()),
                &Observation::new("RAM", 2, 120).unwrap(),
                100,
            ),
            plan: ActionPlan::Notify {
                message: "high".into(),
            },
        }
    }
    #[test]
    fn slow_delivery_does_not_block_producer_and_queue_is_bounded() {
        let (started_tx, started_rx) = mpsc::channel();
        let (release_tx, release_rx) = mpsc::channel();
        let worker = DeliveryWorker::spawn(move |_| {
            started_tx.send(()).unwrap();
            release_rx.recv().unwrap();
            Ok(ActionExecution::Delivered)
        });
        assert!(worker.enqueue(job("one")).unwrap());
        started_rx
            .recv_timeout(std::time::Duration::from_secs(5))
            .unwrap();
        assert!(worker.enqueue(job("two")).unwrap());
        assert!(!worker.enqueue(job("three")).unwrap());
        release_tx.send(()).unwrap();
        release_tx.send(()).unwrap();
        let results = worker.finish().unwrap();
        assert_eq!(results.len(), 2);
        assert!(results
            .iter()
            .all(|r| r.outcome == ActionExecution::Delivered));
    }
    #[test]
    fn failures_and_disabled_delivery_are_explicit() {
        let worker = DeliveryWorker::spawn(|_| anyhow::bail!("simulated timeout"));
        worker.enqueue(job("failure")).unwrap();
        let results = worker.finish().unwrap();
        assert!(
            matches!(&results[0].outcome,ActionExecution::Failed { reason } if reason=="simulated timeout")
        );
        assert_eq!(
            Webhook::disabled().deliver(&job("disabled")).unwrap(),
            ActionExecution::Skipped {
                reason: "actions_disabled".into()
            }
        );
    }
}
