use vapor_project::{
    actions, deviation::Deviation, dsl::parse_program, evaluate, observation::Observation,
    policy::ActionPlan, qualification::SentinelState,
};

fn obs(sequence: u64, value: u64) -> Observation {
    Observation::new("SYSTEM_USED_MEMORY_MIB", sequence, value).unwrap()
}

#[test]
fn all_comparison_operators_control_qualification_and_policy_consistently() {
    for (operator, value, expected) in [
        (">", 100, false),
        (">", 101, true),
        (">=", 100, true),
        ("<", 99, true),
        ("<", 100, false),
        ("<=", 100, true),
        ("==", 100, true),
        ("==", 101, false),
        ("!=", 99, true),
        ("!=", 100, false),
    ] {
        let source = format!(
            "vapor s() {{ if(SYSTEM_USED_MEMORY_MIB {operator} 100) {{ send(\"matched\"); }} }}"
        );
        let program = parse_program(&source).unwrap();
        let evaluation = evaluate(Some(&obs(1, 80)), &obs(2, value), &program);
        assert_eq!(
            evaluation.evidence.state(),
            if expected {
                SentinelState::Anomalous
            } else {
                SentinelState::Normal
            },
            "{operator} {value}"
        );
        assert_eq!(
            evaluation.plan,
            if expected {
                ActionPlan::Notify {
                    message: "matched".into(),
                }
            } else {
                ActionPlan::NoAction
            }
        );
        assert!(actions::validate(&evaluation.plan, &evaluation.evidence).is_ok());
    }
}
#[test]
fn startup_duplicates_gaps_reordering_and_metric_changes_cannot_authorize_notifications() {
    let program = parse_program("vapor s(){ send(\"alert\"); }").unwrap();
    let previous = obs(3, 80);
    let cases = [
        (None, obs(1, 200), Deviation::NoBaseline),
        (Some(&previous), obs(3, 200), Deviation::DuplicateSequence),
        (Some(&previous), obs(2, 200), Deviation::OutOfOrderSequence),
        (Some(&previous), obs(5, 200), Deviation::SequenceGap),
        (
            Some(&previous),
            Observation::new("OTHER", 4, 200).unwrap(),
            Deviation::MetricMismatch,
        ),
    ];
    for (previous, current, deviation) in cases {
        let evaluation = evaluate(previous, &current, &program);
        assert_eq!(evaluation.evidence.deviation(), &deviation);
        assert_eq!(evaluation.evidence.state(), SentinelState::Unknown);
        assert_eq!(evaluation.plan, ActionPlan::NoAction);
        assert!(actions::validate(
            &ActionPlan::Notify {
                message: "forged".into()
            },
            &evaluation.evidence
        )
        .is_err());
    }
}
#[test]
fn healthy_decrease_and_unmatched_policy_are_normal() {
    let program =
        parse_program("vapor s(){ if(SYSTEM_USED_MEMORY_MIB >= 100){send(\"high\");} }").unwrap();
    let evaluation = evaluate(Some(&obs(1, 80)), &obs(2, 70), &program);
    assert_eq!(evaluation.evidence.state(), SentinelState::Normal);
    assert_eq!(evaluation.plan, ActionPlan::NoAction);
}
#[test]
fn nested_conditions_require_every_predicate() {
    let program = parse_program("vapor s(){ if(SYSTEM_USED_MEMORY_MIB >= 100){ if(SYSTEM_USED_MEMORY_MIB < 200){send(\"range\");} } }").unwrap();
    for (value, expected) in [(99, false), (100, true), (199, true), (200, false)] {
        let evaluation = evaluate(Some(&obs(1, 80)), &obs(2, value), &program);
        assert_eq!(
            matches!(evaluation.plan, ActionPlan::Notify { .. }),
            expected
        );
    }
}
