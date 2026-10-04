use anyhow::{Context, Result};
use serde_json::to_string;
use std::{env, thread, time::Duration};
use sysinfo::{ProcessExt, System, SystemExt};

use vapor_project::{
    actions::{self, ActionExecution},
    dsl::{self, Program},
    evaluate,
    observation::Observation,
};

const MEMORY_METRIC: &str = "SYSTEM_USED_MEMORY_MIB";
const PROCESS_LOG_THRESHOLD_MIB: u64 = 50;
const SENTINEL_THRESHOLD_MIB: u64 = 100;
const POLL_INTERVAL_SECS: u64 = 4;
struct MetricSource {
    sequence: u64,
}

impl MetricSource {
    fn new() -> Self {
        Self { sequence: 0 }
    }

    fn observe(&mut self, sys: &mut System) -> Result<Observation> {
        sys.refresh_memory();
        sys.refresh_processes();

        let used_memory_mib = sys.used_memory() / 1024 / 1024;
        self.sequence = self
            .sequence
            .checked_add(1)
            .context("observation sequence exhausted")?;

        println!("--- SYSTEM MEMORY: {used_memory_mib}MiB used ---");
        for (pid, process) in sys.processes() {
            let memory_mib = process.memory() / 1024 / 1024;
            if memory_mib > PROCESS_LOG_THRESHOLD_MIB {
                println!(
                    "PROCESS: {} ({}MiB) [PID: {}]",
                    process.name(),
                    memory_mib,
                    pid
                );
            }
        }

        Ok(Observation::new(
            MEMORY_METRIC,
            self.sequence,
            used_memory_mib,
        )?)
    }
}

fn load_program() -> Result<Program> {
    dsl::parse_program(
        r#"vapor sentinel() {
            if(SYSTEM_USED_MEMORY_MIB) {
                send("CRITICAL_MEMORY_THRESHOLD");
            }
        }"#,
    )
}

fn shutdown_requested() -> bool {
    env::var("VAPOR_SENTINEL_EXIT").as_deref() == Ok("1")
}

fn main() -> Result<()> {
    let mut sys = System::new();
    let mut source = MetricSource::new();
    let mut previous: Option<Observation> = None;
    let program = load_program()?;
    let oneshot = std::env::var("VAPOR_SENTINEL_ONESHOT").as_deref() == Ok("1");

    println!("--- VAPOR SENTINEL ACTIVE ---");
    println!("Assurance path: observation -> deviation -> qualification -> evidence -> policy.");
    println!("Network actions are disabled unless VAPOR_SENTINEL_ENABLE_ACTIONS=1.");

    loop {
        let current = source.observe(&mut sys)?;
        let requested_messages = program.requested_messages(&current.metric);
        let evaluation = evaluate(
            previous.as_ref(),
            &current,
            SENTINEL_THRESHOLD_MIB,
            &requested_messages,
        );

        println!("EVIDENCE: {}", to_string(&evaluation.evidence)?);
        println!("POLICY: {}", to_string(&evaluation.plan)?);

        match actions::execute(&evaluation.plan, &evaluation.evidence) {
            Ok(ActionExecution::Executed) => println!("ACTION: notification executed"),
            Ok(ActionExecution::Skipped) => println!("ACTION: skipped"),
            Err(error) => eprintln!("ACTION ERROR: {error:#}"),
        }

        previous = Some(current);

        if oneshot || shutdown_requested() {
            break;
        }

        thread::sleep(Duration::from_secs(POLL_INTERVAL_SECS));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shutdown_requires_explicit_environment_value() {
        assert!(!is_shutdown_value(Some("1"), false));
        assert!(is_shutdown_value(Some("1"), true));
        assert!(!is_shutdown_value(Some("0"), true));
        assert!(!is_shutdown_value(None, true));
    }

    fn is_shutdown_value(value: Option<&str>, expected: bool) -> bool {
        (value == Some("1")) == expected
    }
}
