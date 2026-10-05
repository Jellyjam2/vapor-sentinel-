# Current architecture

The executable samples one configured memory metric and evaluates one local policy. It does not perform filesystem deletion, process remediation, arbitrary code execution, or cryptographic attestation.

## Deterministic core

`Observation → Deviation → Qualification → EvidenceRecord → ActionPlan`

`Observation` validates identity and nonzero sequence. `Deviation` requires matching identities and consecutive sequences; a missing baseline, gap, duplicate, or reversed sequence becomes Unknown. The runtime's `EvidenceRecord::evaluate_policy` evaluates the actual DSL predicates and qualifies valid matching requests as Anomalous, otherwise Normal. The standalone numeric `EvidenceRecord::evaluate` helper uses an inclusive numeric threshold for callers that explicitly choose that API; the executable does not add that threshold to DSL conditions.

The pure policy module authorizes anomaly notifications only from anomalous evidence. All six DSL comparison operators are authoritative. Bare metric selectors match the selected metric without a value test. Source code is parsed once; the parser has no external effects. Available metric names are bound before startup.

## Stateful runtime

`ActionPlan → Lifecycle → Bounded delivery worker → DeliveryResult`

The lifecycle module suppresses repeated alerts, schedules reminders, backs off after failures, and emits a separate recovery plan after consecutive normal samples. Unknown samples neither authorize notifications nor resolve active alerts. The action boundary requires Anomalous evidence for Notify and Normal evidence for Recovered, and rechecks aggregate message bounds.

The worker uses a capacity-one queue; the normal runtime keeps at most one notification pending. Its HTTP client is reused, accepts HTTPS only, follows no redirects, and uses a ten-second timeout. Queue status and delivery outcomes are distinct from qualification. A slow endpoint does not run on the sampling thread. Sampling is scheduled using monotonic deadlines; missed deadlines do not cause an unbounded catch-up loop.

Retry is in-memory and uses the next current observation; this is not a durable delivery guarantee. Changed notification messages may be delivered before the reminder interval. Recovery is sample-based debounce, not numeric hysteresis. Standard termination requests stop sampling and drain outstanding delivery work.

## Evidence boundary

Stdout is versioned JSONL. The optional output file appends the same records. Evaluation events include wall-clock timestamp, source/run/event labels, exact-policy SHA-256, matched messages, evidence, plans, and scheduling reason. Delivery events refer to the evaluation ID. Neither labels nor hashes establish authenticity. Logs require operator-managed rotation and retention.

Replay is an explicit CLI mode using bounded input records. It always disables network actions and advances scheduling time from sample position and configured interval. Replay decisions are reproducible; emitted run IDs and recording timestamps intentionally differ.

The browser viewer imports these recordings locally and adapts the actual schema. It does not fetch a live feed or obtain execution authority.

## Resource and trust boundaries

- Config: 16 KiB maximum, known fields, validated intervals and adapter.
- DSL: 64 KiB maximum, 16 nested conditions, 64 messages, 1,024 bytes per message, 4,096 aggregate bytes including separators.
- A quote-aware iterative preflight limits nesting before Pest is called; token rules are atomic.
- Replay: 1 MiB and at most 10,000 observations.
- Viewer: 10 MiB and at most 20,000 JSONL records.
- Metric sampling refreshes memory only; unrelated process enumeration is removed.

These are engineering boundaries within one process, not a sandbox against malicious in-process code. See [threat-model.md](docs/threat-model.md). Future guarantees described in FUTURE_UPGRADES_ROADMAP.txt remain future work.
