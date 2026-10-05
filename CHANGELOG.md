# Changelog

## Unreleased — runtime correctness and verification repairs

### Fixed

- Executable imports/path formatting and stale Observation API calls in tests.
- Recursive-parser crash: quote-aware structural limits are checked before Pest; file reads are capped.
- Atomic identifier/number/string tokens, metric binding, and aggregate notification limits.
- DSL comparisons now directly determine qualification, without a contradictory second threshold.
- Ordinary metric changes/decreases no longer imply degraded health; sequence gaps become Unknown.
- One-shot mode now observes both a baseline and an evaluation.
- Repeated notifications are controlled by reminder and retry intervals, with separate debounced recovery events.
- Slow HTTP delivery is moved off the observation thread; delivery failures are explicit and bounded runs exit unsuccessfully when delivery fails.
- Configuration tests now exercise the actual loader and rejection paths.

### Added

- Bounded replay CLI that always disables external actions, plus a recorded-memory fixture.
- Versioned JSONL evaluation/delivery records, optional append-only recording, timestamps, run/source/event labels, and policy SHA-256.
- Read-only browser import/viewing of real JSONL evidence and recorded delivery status.
- Ctrl-C/SIGTERM shutdown; reusable HTTPS client, no redirects, URL validation, and credential-safe error text.
- Committed dependency lockfile, pinned compiler, pinned CI action commits, least-privilege CI permissions, and executable smoke tests.
- Regression tests for comparisons, nesting, metric typos, oversized aggregates, cooldown/recovery/retry, slow workers, config loading, recording, and replay.
- Current threat model and verification instructions.

### Migration

- Remove `threshold_mib` from JSON; place threshold comparisons in the policy.
- The default metric is now SYSTEM_AVAILABLE_MEMORY_PERCENT with an illustrative <=10% policy. SYSTEM_USED_MEMORY_MIB remains selectable.
- Action plans and execution outcomes use a versioned JSON representation; previous sample dashboard data is not the event contract.
- VAPOR_SENTINEL_ONESHOT and the deprecated VAPOR_SENTINEL_EXIT alias take two samples.

### Limitations

No authenticated telemetry, signed evidence, durable delivery queue, service installer, or live dashboard transport is claimed. Log retention and monitored-workload calibration are deployment responsibilities. License selection, a verified private reporting channel, real-environment benchmarks, pilot validation, and cross-platform CI results remain release requirements. See docs/verification.md for checks actually completed.
