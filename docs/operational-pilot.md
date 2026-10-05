# Operational pilot

The next release gate is a representative deployment with recorded acceptance results. The tools below prepare that work; passing a short local or CI run does not complete the pilot.

## Agree on acceptance criteria first

Record the operator, workload, operating system, hardware or VM allocation, container limits if any, monitoring period, policy, expected alert destination, and success budgets before testing. Keep deployment identifiers and recordings private unless they are safe to share.

| Area | Required acceptance evidence |
|---|---|
| Policy usefulness | The selected host-memory signal corresponds to the workload's actual failure mode; choose thresholds using observations. Container memory limits are not measured by the current adapter. |
| Overhead | Agreed CPU and peak-RSS budgets, measured at the intended interval during idle and representative normal/peak workload. Record workload latency or throughput with and without the monitor separately. |
| Sampling | Consecutive evaluation sequences and observed sample gaps within an agreed tolerance. Investigate clock steps and scheduler delays rather than silently averaging them away. |
| Notifications | One opening alert, controlled repeats, explicit failure/retry, and debounced recovery at an operator-approved test destination. Match event IDs against actual provider receipts. |
| Shutdown and restart | Normal termination drains outstanding work within an agreed timeout. Restart creates a new baseline/run; unsent notifications and cooldown state do not survive restart. |
| Recording | Restrict access, budget disk space, retain evaluation and delivery records together, and verify the chosen rotation/backup process without losing or mixing runs. |
| Installation | An operator can install, start, stop, upgrade, and roll back the chosen deployment without editing source. Record the commands and observed results for that environment. |
| Dashboard | Open the retained recording, inspect an alert and recovery, verify missing outcomes stay explicit, and export/reimport a filtered subset. |

Choose numeric budgets appropriate to the deployment. This repository does not yet establish universal CPU, memory, latency, or availability guarantees.

## Establish the baseline

Use the pinned toolchain and committed lockfile. From a fresh checkout:

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cargo build --locked --release
```

Retain the source commit, build environment, binary digest, configuration, exact policy bytes, and output records. A source revision label and a hash identify inputs; they do not attest how a binary was built.

## Measure the monitor on Linux

The standard-library-only measurement tool requires Linux and Python 3.9+. It measures an already-built executable; compilation is outside the timed interval.

```sh
python3 scripts/measure_runtime.py \
  --binary target/release/vapor_project \
  --config vapor-sentinel.json \
  --samples 60 --interval-secs 4 \
  --label 'idle baseline on pilot host' \
  --source-revision "$(git rev-parse HEAD)" \
  --output pilot-results/idle-01
```

Repeat at least three times for both the idle case and the operator's representative workload, using a new output directory each time. Run the workload using its normal test procedure; the tool does not create memory pressure or alter the workload. Use the same binary, policy and interval when comparing runs. A 60-sample run at four-second intervals takes approximately 236 seconds plus startup/shutdown.

The tool copies configuration and policy into the new output directory, changes only the copied policy path and polling interval, removes inherited `VAPOR_SENTINEL_*` settings, and explicitly disables notification delivery. The original files are unchanged. The source label is fixed to `pilot-measurement`; no webhook URL is recorded. Treat custom policy text and evidence as deployment data.

Each run retains:

- `report.json`: binary/config/policy hashes, an optional operator-supplied revision label, platform context, child resource usage, state/outcome counts, and sample gaps.
- `config.json` and `policy.vapor`: the exact effective inputs.
- `evidence.jsonl`: raw live evaluation/delivery records that the dashboard can open.
- Separate configuration-check and runtime stderr logs.

The output directory must not exist. A failed configuration, nonzero runtime exit, incomplete recording, missing delivery outcome, or timeout produces a nonzero tool exit. Once the output directory and report are initialized, failures retain a report marked `valid_measurement: false`. This flag means the measurement completed and its basic recording checks passed; it does not mean the deployment met its performance budgets.

The default timeout is the expected sample span plus 30 seconds. `--timeout-secs` overrides it. Timeout or Ctrl-C sends SIGTERM to the child process group, then SIGKILL after two seconds if necessary, and marks the run invalid. This bounded harness timeout is not evidence of the runtime's normal graceful-shutdown behavior.

### Interpret the numbers

`wall_seconds` is measured with the launcher's monotonic clock and includes process startup, observation waits, recording, and shutdown. Polling for process completion adds approximately one 20 ms polling interval plus scheduling delay. CPU user/system time comes from Linux `wait4` for that child, not cumulative usage across benchmark runs. Average CPU percentage is `100 × (user + system CPU seconds) / wall seconds`; 100% means one CPU core, and multithreaded work can exceed it. Sleeping between observations is deliberately part of the monitoring cost.

Memory has two deliberately separate fields. `child_lifetime_peak_rss_kib` is Linux `wait4`'s peak and can include the forked Python launcher's memory before exec. It must not be presented as the Rust executable's steady footprint. `runtime_high_water_rss_kib_observed` is the largest `/proc/PID/status` `VmHWM` seen after `/proc/PID/exe` matches the requested executable, sampled approximately every 20 ms. It is an observed high-water mark in KiB; a final spike between the last read and exit can be missed. The report records the number of successful reads. A completed run without any post-exec RSS reading is invalid. Pass the actual executable, not a wrapper script. Host `/proc` access is required.

Recorded sample gaps use `observed_at_ms`, which is wall-clock time. Clock corrections can distort or reverse those gaps; the report exposes nonpositive gaps. These are not monotonic deadline or webhook latency measurements. The report's CPU counts describe visible CPUs/affinity and do not establish a container's CPU quota. Record VM/cgroup constraints separately.

Results cover the selected memory adapter, policy, interval and actions-disabled execution. They do not measure enabled HTTPS delivery, the browser, build resource usage, disk retention, application slowdown, or behavior under an out-of-memory condition. Short shared-runner measurements are smoke checks, not deployment performance claims.

## Complete deployment-specific checks

Use the deterministic replay fixture first to inspect the baseline, opening alert, suppressed repeat, and recovery without network delivery. The replay command and expected sequence are in the README.

For live delivery, the operator must select an HTTPS test endpoint and explicitly enable actions using the README instructions. Use a staging workload or a temporary test policy with a safely induced match; do not force the host toward memory exhaustion. Retain provider receipts and matching delivery records. Exercise slow/error responses using a controlled destination and confirm observations continue. Do not send test notifications to production recipients without their authorization.

For retention, the runtime keeps its output file open and does not reopen it on rename. Do not assume rename-based log rotation is automatically supported. Test a supervised stop/drain, archive, restart procedure and verify the downtime and baseline reset are acceptable. A successful write is not an fsync-backed durable-delivery promise.

Document installation, service identity, filesystem permissions, disk monitoring, shutdown timeout, upgrade and rollback for the chosen OS. No service installer or durable notification queue is delivered by this pilot tooling.

## Close the gate with evidence

Attach results to the pilot record: expected budgets, commands, workload description, repeated measurements, alert/recovery receipts, interruption/restart behavior, retention exercise, dashboard inspection, and named operator acceptance. Mark any unmet criterion explicitly.

Keep the roadmap's pilot/deployment boxes open until those environment-specific results exist. A maintainer-selected license and a tested private vulnerability reporting channel are separate release requirements.
