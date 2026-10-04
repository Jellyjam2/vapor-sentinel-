use vapor_project::{
    deviation::Deviation,
    dsl::parse_program,
    evidence::EvidenceRecord,
    observation::Observation,
    policy::{self, ActionPlan},
    qualification::SentinelState,
};

#[test]
fn integrated_anomaly_produces_notification_plan() -> anyhow::Result<()> {
    let previous = Observation::new("SYSTEM_USED_MEMORY_MB", 1, 80);
    let current = Observation::new("SYSTEM_USED_MEMORY_MB", 2, 120);
    let evidence = EvidenceRecord::evaluate(Some(&previous), &current, 100);

    assert_eq!(evidence.deviation(), &Deviation::Increased { delta: 40 });
    assert_eq!(evidence.state(), SentinelState::Anomalous);

    let program = parse_program(
        r#"vapor sentinel() {
            if(SYSTEM_USED_MEMORY_MB) {
                send("CRITICAL_MEMORY_THRESHOLD");
            }
        }"#,
    )?;

    let messages = program.requested_messages(&current.metric);
    assert_eq!(messages, vec!["CRITICAL_MEMORY_THRESHOLD"]);

    assert_eq!(
        policy::plan(&evidence, &messages),
        ActionPlan::Notify {
            message: "CRITICAL_MEMORY_THRESHOLD".into()
        }
    );
    Ok(())
}

#[test]
fn first_observation_cannot_trigger_notification() -> anyhow::Result<()> {
    let current = Observation::new("SYSTEM_USED_MEMORY_MB", 1, 120);
    let evidence = EvidenceRecord::evaluate(None, &current, 100);

    assert_eq!(evidence.state(), SentinelState::Unknown);

    let program = parse_program(
        r#"vapor sentinel() {
            if(SYSTEM_USED_MEMORY_MB) {
                send("CRITICAL_MEMORY_THRESHOLD");
            }
        }"#,
    )?;

    assert_eq!(
        policy::plan(&evidence, &program.requested_messages(&current.metric)),
        ActionPlan::NoAction
    );
    Ok(())
}
