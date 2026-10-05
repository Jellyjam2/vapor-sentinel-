use serde_json::{json, Value};
use std::{
    fs,
    process::{Command, Output},
};

fn run(args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_vapor_project"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env_remove("VAPOR_SENTINEL_CONFIG")
        .env_remove("VAPOR_SENTINEL_ONESHOT")
        .env_remove("VAPOR_SENTINEL_EXIT")
        .env("VAPOR_SENTINEL_ENABLE_ACTIONS", "0")
        .args(args)
        .output()
        .unwrap()
}
fn records(output: &Output) -> Vec<Value> {
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|line| serde_json::from_str(line).unwrap())
        .collect()
}
#[test]
fn real_executable_replays_startup_alarm_cooldown_and_recovery_without_network() {
    let output = Command::new(env!("CARGO_BIN_EXE_vapor_project"))
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .env_remove("VAPOR_SENTINEL_CONFIG")
        .env_remove("VAPOR_SENTINEL_ONESHOT")
        .env_remove("VAPOR_SENTINEL_EXIT")
        .env("VAPOR_SENTINEL_ENABLE_ACTIONS", "1")
        .env(
            "VAPOR_SENTINEL_WEBHOOK_URL",
            "deliberately invalid: must never be used in replay",
        )
        .args(["--replay", "tests/fixtures/memory-sequence.json"])
        .output()
        .unwrap();
    let rows = records(&output);
    let evaluations: Vec<_> = rows
        .iter()
        .filter(|r| r["record_type"] == "evaluation")
        .collect();
    assert_eq!(evaluations.len(), 5);
    let states: Vec<_> = evaluations
        .iter()
        .map(|r| r["evidence"]["state"].as_str().unwrap())
        .collect();
    assert_eq!(
        states,
        ["Unknown", "Anomalous", "Anomalous", "Normal", "Normal"]
    );
    assert_eq!(evaluations[2]["scheduling"], "cooldown");
    assert_eq!(evaluations[4]["scheduled_plan"]["kind"], "recovered");
    let deliveries: Vec<_> = rows
        .iter()
        .filter(|r| r["record_type"] == "delivery")
        .collect();
    assert_eq!(deliveries.len(), 2);
    for row in deliveries {
        assert_eq!(
            row["delivery"]["outcome"]["reason"],
            "replay_disables_actions"
        );
    }
    for row in evaluations {
        assert_eq!(row["schema_version"], 1);
        assert_eq!(row["policy_sha256"].as_str().unwrap().len(), 64);
        assert!(row["observed_at_ms"].as_u64().unwrap() > 0);
        assert_eq!(row["run_id"], rows[0]["run_id"]);
    }
}
#[test]
fn output_appends_identical_jsonl_and_replay_is_repeatable() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("evidence.jsonl");
    let first = run(&[
        "--replay",
        "tests/fixtures/memory-sequence.json",
        "--output",
        path.to_str().unwrap(),
    ]);
    let first_rows = records(&first);
    assert_eq!(fs::read(&path).unwrap(), first.stdout);
    let second = run(&[
        "--replay",
        "tests/fixtures/memory-sequence.json",
        "--output",
        path.to_str().unwrap(),
    ]);
    let second_rows = records(&second);
    assert_eq!(
        fs::read(&path).unwrap().len(),
        first.stdout.len() + second.stdout.len()
    );
    for (a, b) in first_rows
        .iter()
        .filter(|r| r["record_type"] == "evaluation")
        .zip(
            second_rows
                .iter()
                .filter(|r| r["record_type"] == "evaluation"),
        )
    {
        assert_eq!(a["evidence"], b["evidence"]);
        assert_eq!(a["plan"], b["plan"]);
        assert_eq!(a["scheduled_plan"], b["scheduled_plan"]);
    }
}
#[test]
fn default_config_validates_and_bad_cli_fails() {
    let rows = records(&run(&["--check"]));
    assert_eq!(rows[0]["valid"], true);
    assert!(!run(&["--samples", "0"]).status.success());
    assert!(!run(&["--unknown"]).status.success());
}
#[test]
fn unknown_metric_and_deep_policy_fail_before_startup() {
    let dir = tempfile::tempdir().unwrap();
    let policy = dir.path().join("policy.vapor");
    let config = dir.path().join("config.json");
    fs::write(
        &config,
        json!({"poll_interval_secs":1,"dsl_path":"policy.vapor"}).to_string(),
    )
    .unwrap();
    for source in [
        "vapor s(){if(TYPO<=10){send(\"bad\");}}".to_owned(),
        format!(
            "vapor s(){{{}send(\"deep\");{}}}",
            "if(M){".repeat(9000),
            "}".repeat(9000)
        ),
    ] {
        fs::write(&policy, source).unwrap();
        let output = run(&["--config", config.to_str().unwrap(), "--check"]);
        assert!(!output.status.success());
        assert!(output.stdout.is_empty());
        assert!(!String::from_utf8_lossy(&output.stderr).contains("stack overflow"));
    }
}
#[test]
fn evidence_cannot_be_appended_into_policy_or_replay_input() {
    let output = run(&[
        "--replay",
        "tests/fixtures/memory-sequence.json",
        "--output",
        "tests/fixtures/memory-sequence.json",
    ]);
    assert!(!output.status.success());
}
