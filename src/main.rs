use anyhow::{bail, Context, Result};
use serde::Deserialize;
use serde_json::json;
use std::{
    env,
    path::{Path, PathBuf},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
    time::{Duration, Instant},
};
use sysinfo::{System, SystemExt};
use vapor_project::{
    actions::{ActionExecution, Webhook},
    config::{self, Metric, RuntimeConfig},
    delivery::{DeliveryResult, DeliveryWorker, Notification},
    dsl::{self, Program},
    evaluate,
    events::{self, EventWriter, RunContext},
    lifecycle::Lifecycle,
    observation::Observation,
    policy::ActionPlan,
};

#[derive(Default)]
struct Options {
    config: Option<PathBuf>,
    replay: Option<PathBuf>,
    output: Option<PathBuf>,
    samples: Option<usize>,
    check: bool,
    help: bool,
}
impl Options {
    fn parse() -> Result<Self> {
        let mut result = Self::default();
        let mut args = env::args_os().skip(1);
        while let Some(arg) = args.next() {
            match arg.to_str() {
                Some("--config") => {
                    result.config = Some(args.next().context("--config requires a path")?.into())
                }
                Some("--replay") => {
                    result.replay =
                        Some(args.next().context("--replay requires a JSON path")?.into())
                }
                Some("--output") => {
                    result.output = Some(
                        args.next()
                            .context("--output requires a JSONL path")?
                            .into(),
                    )
                }
                Some("--samples") => {
                    let value = args.next().context("--samples requires a count")?;
                    let count = value.to_str().context("invalid count")?.parse::<usize>()?;
                    if !(1..=10_000).contains(&count) {
                        bail!("--samples must be 1..10000");
                    }
                    result.samples = Some(count);
                }
                Some("--check") => result.check = true,
                Some("--help" | "-h") => result.help = true,
                _ => bail!("unknown argument; use --help"),
            }
        }
        Ok(result)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ReplayRow {
    metric: String,
    sequence: u64,
    value: u64,
}
fn replay(path: &Path, metric: Metric) -> Result<Vec<Observation>> {
    let rows: Vec<ReplayRow> = serde_json::from_str(&config::read_bounded(path, 1024 * 1024)?)?;
    if rows.is_empty() || rows.len() > 10_000 {
        bail!("replay must contain 1..10000 observations");
    }
    rows.into_iter()
        .map(|row| {
            if row.metric != metric.name() {
                bail!("replay metric differs from configured metric");
            }
            if metric == Metric::AvailableMemoryPercent && row.value > 100 {
                bail!("memory percentage must be 0..100");
            }
            Observation::new(row.metric, row.sequence, row.value)
        })
        .collect()
}
fn observe(sys: &mut System, metric: Metric, sequence: u64) -> Result<Observation> {
    sys.refresh_memory();
    let total = sys.total_memory();
    if total == 0 {
        bail!("system memory measurement unavailable");
    }
    let value = match metric {
        Metric::UsedMemoryMib => sys.used_memory() / 1024 / 1024,
        Metric::AvailableMemoryPercent => {
            let available = sys.available_memory();
            if available > total {
                bail!("invalid system memory measurement");
            }
            ((u128::from(available) * 100) / u128::from(total)) as u64
        }
    };
    Observation::new(metric.name(), sequence, value)
}
fn report_delivery(
    writer: &mut EventWriter,
    context: &RunContext,
    lifecycle: &mut Lifecycle,
    result: DeliveryResult,
    elapsed: u64,
) -> Result<bool> {
    lifecycle.completed(&result, elapsed);
    let failed = matches!(result.outcome, ActionExecution::Failed { .. });
    writer.emit(&json!({"schema_version": 1, "record_type": "delivery", "run_id": context.run_id, "recorded_at_ms": events::unix_ms()?, "delivery": result}))?;
    Ok(failed)
}

fn main() -> Result<()> {
    let options = Options::parse()?;
    if options.help {
        println!("Vapor Sentinel\nUsage: vapor_project [--config PATH] [--check] [--samples N] [--output events.jsonl]\n       vapor_project [--config PATH] --replay observations.json [--output events.jsonl]\nReplay always disables network effects. Ctrl-C/SIGTERM requests graceful shutdown.\nVAPOR_SENTINEL_ONESHOT=1 takes two samples (baseline + evaluation).");
        return Ok(());
    }
    let config_path = options.config.unwrap_or_else(config::path_from_env);
    let config = RuntimeConfig::load(&config_path)?;
    let policy_source = config::read_bounded(&config.dsl_path, dsl::MAX_SOURCE_BYTES)?;
    let program: Program = dsl::parse_program(&policy_source)?;
    program.validate_metrics(&[config.metric.name()])?;
    let replay_rows = options
        .replay
        .as_deref()
        .map(|path| replay(path, config.metric))
        .transpose()?;
    let is_replay = replay_rows.is_some();
    let mut webhook = if is_replay {
        Webhook::disabled()
    } else {
        Webhook::from_env()?
    };
    if options.check {
        println!("{}", json!({"valid": true, "metric": config.metric.name()}));
        return Ok(());
    }
    // Do not allow evidence output to append into any of the inputs being read.
    if let Some(output) = &options.output {
        if output.exists() {
            let output = output.canonicalize()?;
            for input in [
                Some(config_path.as_path()),
                Some(config.dsl_path.as_path()),
                options.replay.as_deref(),
            ]
            .into_iter()
            .flatten()
            {
                if output == input.canonicalize()? {
                    bail!("evidence output must not overwrite an input file");
                }
            }
        }
    }
    let context = RunContext::new(&policy_source, is_replay)?;
    let mut writer = EventWriter::new(options.output.as_deref())?;
    let worker = DeliveryWorker::spawn(move |notification| webhook.deliver(notification));
    let mut lifecycle = Lifecycle::new(
        config.notification_repeat_secs,
        config.notification_retry_secs,
        config.recovery_samples,
    );
    let stop = Arc::new(AtomicBool::new(false));
    let handler_stop = Arc::clone(&stop);
    ctrlc::set_handler(move || handler_stop.store(true, Ordering::SeqCst))?;
    let oneshot = env::var("VAPOR_SENTINEL_ONESHOT").as_deref() == Ok("1")
        || env::var("VAPOR_SENTINEL_EXIT").as_deref() == Ok("1");
    let limit = options
        .samples
        .or_else(|| oneshot.then_some(2))
        .unwrap_or(usize::MAX);
    let started = Instant::now();
    let mut deadline = started;
    let interval = Duration::from_secs(config.poll_interval_secs);
    let mut previous = None;
    let mut sys = System::new();
    let mut samples = 0usize;
    let mut delivery_failed = false;
    eprintln!(
        "Vapor Sentinel: {}. JSONL on stdout; policy {}. Replay={is_replay}",
        config.metric.name(),
        config.dsl_path.display()
    );
    while !stop.load(Ordering::SeqCst) && samples < limit {
        let elapsed = if is_replay {
            samples as u64 * config.poll_interval_secs
        } else {
            started.elapsed().as_secs()
        };
        for result in worker.completed() {
            delivery_failed |=
                report_delivery(&mut writer, &context, &mut lifecycle, result, elapsed)?;
        }
        let current = if let Some(rows) = &replay_rows {
            let Some(row) = rows.get(samples) else {
                break;
            };
            row.clone()
        } else {
            observe(
                &mut sys,
                config.metric,
                (samples as u64)
                    .checked_add(1)
                    .context("sequence exhausted")?,
            )?
        };
        let evaluation = evaluate(previous.as_ref(), &current, &program);
        let (candidate, scheduling) =
            lifecycle.candidate(evaluation.evidence.state(), &evaluation.plan, elapsed);
        let event_id = format!("{}:{}", context.run_id, samples + 1);
        let observed_at_ms = events::unix_ms()?;
        writer.emit(&json!({"schema_version": 1, "record_type": "evaluation", "run_id": context.run_id, "source_id": context.source_id, "event_id": event_id, "observed_at_ms": observed_at_ms, "policy_sha256": context.policy_sha256, "unit": config.metric.unit(), "replay": is_replay, "evidence": evaluation.evidence, "plan": evaluation.plan, "scheduled_plan": candidate, "scheduling": scheduling}))?;
        if candidate != ActionPlan::NoAction {
            if is_replay {
                lifecycle.submitted(&candidate);
                let result = DeliveryResult {
                    event_id,
                    plan: candidate,
                    outcome: ActionExecution::Skipped {
                        reason: "replay_disables_actions".into(),
                    },
                };
                report_delivery(&mut writer, &context, &mut lifecycle, result, elapsed)?;
            } else {
                let job = Notification {
                    schema_version: 1,
                    event_id: event_id.clone(),
                    run_id: context.run_id.clone(),
                    source_id: context.source_id.clone(),
                    observed_at_ms,
                    policy_sha256: context.policy_sha256.clone(),
                    evidence: evaluation.evidence,
                    plan: candidate.clone(),
                };
                if worker.enqueue(job)? {
                    lifecycle.submitted(&candidate);
                } else {
                    writer.emit(&json!({"schema_version": 1, "record_type": "delivery", "run_id": context.run_id, "recorded_at_ms": events::unix_ms()?, "delivery": {"event_id": event_id, "plan": candidate, "outcome": {"status": "skipped", "reason": "queue_full"}}}))?;
                }
            }
        }
        previous = Some(current);
        samples += 1;
        if samples >= limit || is_replay {
            continue;
        }
        deadline += interval;
        if deadline < Instant::now() {
            deadline = Instant::now() + interval;
        }
        while !stop.load(Ordering::SeqCst) && Instant::now() < deadline {
            for result in worker.completed() {
                delivery_failed |= report_delivery(
                    &mut writer,
                    &context,
                    &mut lifecycle,
                    result,
                    started.elapsed().as_secs(),
                )?;
            }
            thread::sleep(
                deadline
                    .saturating_duration_since(Instant::now())
                    .min(Duration::from_millis(100)),
            );
        }
    }
    for result in worker.finish()? {
        delivery_failed |= report_delivery(
            &mut writer,
            &context,
            &mut lifecycle,
            result,
            started.elapsed().as_secs(),
        )?;
    }
    if delivery_failed {
        bail!("one or more notifications failed; inspect delivery records");
    }
    Ok(())
}
