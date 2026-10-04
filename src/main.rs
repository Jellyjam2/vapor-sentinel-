use anyhow::{Context, Result};
use serde_json::to_string;
use std::{env, fs, path::Path, thread, time::Duration};
use sysinfo::{ProcessExt, System, SystemExt};

use vapor_project::{
    actions::{self, ActionExecution},
    config::{self, RuntimeConfig},
    dsl::Program,
    evaluate,
    observation::Observation,
};

const MEMORY_METRIC: &str = "SYSTEM_USED_MEMORY_MIB";
const PROCESS_LOG_THRESHOLD_MIB: u64 = 50;

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

fn load_program(path: &Path) -> Result<Program> {
    let source = fs::read_to_string(path)
        .with_context(|| format!("failed to read DSL policy {}", path.display()))?;
    dsl::parse_program(&source)
        .with_context(|| format!("failed to parse DSL policy {}", path.display()))
}

fn shutdown_requested() -> bool {
    env::var("VAPOR_SENTINEL_EXIT").as_deref() == Ok("1")
}

fn main() -> Result<()> {
    let config_path = config::path_from_env();
    let runtime_config = RuntimeConfig::load(&config_path)?;
    let program = load_program(Path::new(&runtime_config.dsl_path))?;
    let mut sys = System::new();
    let mut source = MetricSource::new();
    let mut previous: Option<Observation> = None;
    let oneshot = env::var("VAPOR_SENTINEL_ONESHOT").as_deref() == Ok("1");

    println!("--- VAPOR SENTINEL ACTIVE ---");
    println!("Assurance path: observation -> deviation -> qualification -> evidence -> policy.");
    println!(
        "Config: threshold={}MiB poll_interval={}s dsl={}",
        runtime_config.threshold_mib,
        runtime_config.poll_interval_secs,
        runtime_config.dsl_path
    );
    println!("Network actions are disabled unless VAPOR_SENTINEL_ENABLE_ACTIONS=1.");

    loop {
        let current = source.observe(&mut sys)?;
        let requested_messages = program.requested_messages(&current);
        let evaluation = evaluate(
            previous.as_ref(),
            &current,
            runtime_config.threshold_mib,
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

        thread::sleep(Duration::from_secs(runtime_config.poll_interval_secs));
    }

    Ok(())
}
