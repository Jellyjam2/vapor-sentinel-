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
    qualify_match(
        observation,
        deviation,
        observation.is_some_and(|o| o.value() >= threshold),
    )
}

/// A valid sample is anomalous precisely when the configured policy matches.
/// Changing values alone do not imply unhealthy behavior.
pub fn qualify_match(
    observation: Option<&Observation>,
    deviation: Option<&Deviation>,
    matched: bool,
) -> Qualification {
    let reason = match (observation, deviation) {
        (None, _) => Some("no observation"),
        (
            _,
            Some(
                Deviation::DuplicateSequence
                | Deviation::OutOfOrderSequence
                | Deviation::MetricMismatch
                | Deviation::SequenceGap,
            ),
        ) => Some("invalid observation ordering or identity"),
        (_, Some(Deviation::NoBaseline) | None) => Some("baseline unavailable"),
        _ => None,
    };
    if let Some(reason) = reason {
        return Qualification {
            state: SentinelState::Unknown,
            reason,
        };
    }
    if matched {
        Qualification {
            state: SentinelState::Anomalous,
            reason: "policy condition matched",
        }
    } else {
        Qualification {
            state: SentinelState::Normal,
            reason: "no policy condition matched",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::deviation::compare;

    fn obs(sequence: u64, value: u64) -> Observation {
        Observation::new("SYSTEM_RAM", sequence, value).unwrap()
    }

    #[test]
    fn missing_observation_is_unknown() {
        assert_eq!(qualify(None, None, 100).state, SentinelState::Unknown);
    }

    #[test]
    fn first_observation_is_unknown_until_baseline_exists() {
        let current = obs(1, 80);
        assert_eq!(
            qualify(Some(&current), Some(&Deviation::NoBaseline), 100).state,
            SentinelState::Unknown
        );
    }

    #[test]
    fn threshold_exceeded_is_anomalous() {
        let previous = obs(1, 80);
        let current = obs(2, 101);
        let d = compare(Some(&previous), &current);
        assert_eq!(
            qualify(Some(&current), Some(&d), 100).state,
            SentinelState::Anomalous
        );
    }

    #[test]
    fn changed_but_below_threshold_is_normal() {
        let previous = obs(1, 80);
        let current = obs(2, 90);
        let d = compare(Some(&previous), &current);
        assert_eq!(
            qualify(Some(&current), Some(&d), 100).state,
            SentinelState::Normal
        );
    }

    #[test]
    fn invalid_ordering_is_unknown_even_above_threshold() {
        let previous = obs(2, 80);
        let current = obs(1, 101);
        let d = compare(Some(&previous), &current);
        assert_eq!(
            qualify(Some(&current), Some(&d), 100).state,
            SentinelState::Unknown
        );
    }

    #[test]
    fn unchanged_below_threshold_is_normal() {
        let previous = obs(1, 80);
        let current = obs(2, 80);
        let d = compare(Some(&previous), &current);
        assert_eq!(
            qualify(Some(&current), Some(&d), 100).state,
            SentinelState::Normal
        );
    }
}
