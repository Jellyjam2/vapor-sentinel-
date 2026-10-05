//! Pure deterministic observation/deviation primitives.

use crate::observation::Observation;
use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub enum Deviation {
    NoBaseline,
    Unchanged,
    Increased { delta: u64 },
    Decreased { delta: u64 },
    DuplicateSequence,
    OutOfOrderSequence,
    MetricMismatch,
    SequenceGap,
}

pub fn compare(previous: Option<&Observation>, current: &Observation) -> Deviation {
    let Some(previous) = previous else {
        return Deviation::NoBaseline;
    };

    if previous.metric() != current.metric() {
        return Deviation::MetricMismatch;
    }
    if current.sequence() == previous.sequence() {
        return Deviation::DuplicateSequence;
    }
    if current.sequence() < previous.sequence() {
        return Deviation::OutOfOrderSequence;
    }

    if current.sequence() - previous.sequence() != 1 {
        return Deviation::SequenceGap;
    }

    match current.value().cmp(&previous.value()) {
        std::cmp::Ordering::Equal => Deviation::Unchanged,
        std::cmp::Ordering::Greater => Deviation::Increased {
            delta: current.value() - previous.value(),
        },
        std::cmp::Ordering::Less => Deviation::Decreased {
            delta: previous.value() - current.value(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(sequence: u64, value: u64) -> Observation {
        Observation::new("SYSTEM_RAM", sequence, value).unwrap()
    }

    #[test]
    fn missing_baseline_is_reported() {
        assert_eq!(compare(None, &obs(1, 80)), Deviation::NoBaseline);
    }

    #[test]
    fn identical_values_have_no_change() {
        assert_eq!(
            compare(Some(&obs(1, 80)), &obs(2, 80)),
            Deviation::Unchanged
        );
    }

    #[test]
    fn increases_are_reported_with_exact_delta() {
        assert_eq!(
            compare(Some(&obs(1, 80)), &obs(2, 120)),
            Deviation::Increased { delta: 40 }
        );
    }

    #[test]
    fn decreases_are_reported_with_exact_delta() {
        assert_eq!(
            compare(Some(&obs(1, 120)), &obs(2, 80)),
            Deviation::Decreased { delta: 40 }
        );
    }

    #[test]
    fn duplicate_sequences_are_rejected() {
        assert_eq!(
            compare(Some(&obs(7, 80)), &obs(7, 120)),
            Deviation::DuplicateSequence
        );
    }

    #[test]
    fn out_of_order_sequences_are_rejected() {
        assert_eq!(
            compare(Some(&obs(7, 80)), &obs(6, 120)),
            Deviation::OutOfOrderSequence
        );
    }

    #[test]
    fn metric_identity_is_checked() {
        let previous = Observation::new("SYSTEM_RAM", 1, 80).unwrap();
        let current = Observation::new("PROCESS_MEMORY", 2, 80).unwrap();
        assert_eq!(
            compare(Some(&previous), &current),
            Deviation::MetricMismatch
        );
    }
}
