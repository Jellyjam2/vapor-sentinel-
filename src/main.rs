use anyhow::{Context, Result};
use serde_json::to_string;
use std::{thread, time::Duration};
use sysinfo::{ProcessExt, System, SystemExt};

use vapor_project::{
    actions::{self, ActionExecution},
    dsl::{self, Program},
    evaluate,
    observation::Observation,
};

const MEMORY_METRIC: &str = "SYSTEM_USED_MEMORY_MB";
const PROCESS_LOG_THRESHOLD_MB: u64 = 50;
const SENTINEL_THRESHOLD_MB: u64 = 100;
const POLL_INTERVAL_SECS: u64 = 4;

struct MetricSource {
    sequence: u64,
}

impl MetricSource {
    fn new() -> Self {
        Self { sequence: 0 }
    }

    fn observe(&mut self, sys: &mut System) -> Result<Observation> {
        sys.refresh_all();

        let used_memory_mb = sys.used_memory() / 1024 / 1024;
        self.sequence = self
            .sequence
            .checked_add(1)
            .context("observation sequence exhausted")?;

        println!("--- SYSTEM MEMORY: {used_memory_mb}MB used ---");
        for (pid, process) in sys.processes() {
            let memory_mb = process.memory() / 1024 / 1024;
            if memory_mb > PROCESS_LOG_THRESHOLD_MB {
                println!(
                    "PROCESS: {} ({}MB) [PID: {}]",
                    process.name(),
                    memory_mb,
                    pid
                );
            }
        }

        Ok(Observation::new(
            MEMORY_METRIC,
            self.sequence,
            used_memory_mb,
        ))
    }
}

fn load_program() -> Result<Program> {
    dsl::parse_program(
        r#"vapor sentinel() {
            if(SYSTEM_USED_MEMORY_MB) {
                send("CRITICAL_MEMORY_THRESHOLD");
            }
        }"#,
    )
}

fn main() -> Result<()> {
    let mut sys = System::new_all();
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
            SENTINEL_THRESHOLD_MB,
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

        if oneshot || std::path::Path::new("EXIT").exists() {
            if std::path::Path::new("EXIT").exists() {
                std::fs::remove_file("EXIT").context("unable to remove EXIT sentinel")?;
            }
            break;
        }

        thread::sleep(Duration::from_secs(POLL_INTERVAL_SECS));
    }

    Ok(())
}
