//! Deterministic sentinel-state qualification.

use crate::deviation::Deviation;
use crate::observation::Observation;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum SentinelState {
    Normal,
    Degraded,
    Anomalous,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Qualification {
    pub state: SentinelState,
    pub reason: &'static str,
}

pub fn qualify(
    observation: Option<&Observation>,
    deviation: Option<&Deviation>,
    threshold: u64,
) -> Qualification {
    let Some(observation) = observation else {
        return Qualification {
            state: SentinelState::Unknown,
            reason: "no observation",
        };
    };

    match deviation {
        Some(Deviation::DuplicateSequence | Deviation::OutOfOrderSequence | Deviation::MetricMismatch) => {
            return Qualification {
                state: SentinelState::Unknown,
                reason: "invalid observation ordering or identity",
            };
        }
        Some(Deviation::NoBaseline) | None => {
            return Qualification {
                state: SentinelState::Unknown,
                reason: "baseline unavailable",
            };
        }
        Some(Deviation::Unchanged | Deviation::Increased { .. } | Deviation::Decreased { .. }) => {}
    }

    if observation.value > threshold {
        return Qualification {
            state: SentinelState::Anomalous,
            reason: "threshold exceeded",
        };
    }

    match deviation {
        Some(Deviation::Increased { .. } | Deviation::Decreased { .. }) => Qualification {
            state: SentinelState::Degraded,
            reason: "metric changed",
        },
        Some(Deviation::Unchanged) => Qualification {
            state: SentinelState::Normal,
            reason: "evidence within threshold",
        },
        _ => Qualification {
            state: SentinelState::Unknown,
            reason: "insufficient evidence",
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deviation::compare;

    #[test]
    fn missing_observation_is_unknown() {
        assert_eq!(qualify(None, None, 100).state, SentinelState::Unknown);
    }

    #[test]
    fn first_observation_is_unknown_until_baseline_exists() {
        let current = Observation::new("SYSTEM_RAM", 1, 80);
        assert_eq!(
            qualify(Some(&current), Some(&Deviation::NoBaseline), 100).state,
            SentinelState::Unknown
        );
    }

    #[test]
    fn threshold_exceeded_is_anomalous() {
        let previous = Observation::new("SYSTEM_RAM", 1, 80);
        let current = Observation::new("SYSTEM_RAM", 2, 101);
        let d = compare(Some(&previous), &current);
        assert_eq!(qualify(Some(&current), Some(&d), 100).state, SentinelState::Anomalous);
    }

    #[test]
    fn changed_but_below_threshold_is_degraded() {
        let previous = Observation::new("SYSTEM_RAM", 1, 80);
        let current = Observation::new("SYSTEM_RAM", 2, 90);
        let d = compare(Some(&previous), &current);
        assert_eq!(qualify(Some(&current), Some(&d), 100).state, SentinelState::Degraded);
    }

    #[test]
    fn invalid_ordering_is_unknown_even_above_threshold() {
        let previous = Observation::new("SYSTEM_RAM", 2, 80);
        let current = Observation::new("SYSTEM_RAM", 1, 101);
        let d = compare(Some(&previous), &current);
        assert_eq!(qualify(Some(&current), Some(&d), 100).state, SentinelState::Unknown);
    }

    #[test]
    fn unchanged_below_threshold_is_normal() {
        let previous = Observation::new("SYSTEM_RAM", 1, 80);
        let current = Observation::new("SYSTEM_RAM", 2, 80);
        let d = compare(Some(&previous), &current);
        assert_eq!(qualify(Some(&current), Some(&d), 100).state, SentinelState::Normal);
    }
}
