# Changelog

All notable changes to Vapor Sentinel are documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

---

## [Unreleased]

### Added
- Deterministic observation, deviation, qualification, and evidence modules.
- Explicit policy planning with `Normal`, `Degraded`, `Anomalous`, and `Unknown` sentinel states.
- A restricted Vapor DSL parser that produces declarative notification requests without performing filesystem or network operations itself.
- Optional HTTPS webhook notification through an explicit environment-controlled action boundary.
- Bounded one-shot and explicit environment-controlled runtime termination modes for deterministic local execution and smoke testing.
- A static, read-only dashboard preview using explicit sample data.

### Changed
- Runtime processing now follows the explicit assurance path:
  `observation -> deviation -> qualification -> evidence -> policy -> optional action`.
- The runtime memory metric is named `SYSTEM_USED_MEMORY_MIB` and reflects system used memory reported by `sysinfo`.
- Unsupported DSL constructs are rejected rather than silently ignored.
- CI cache keys now include dependency manifests and the Rust matrix entry, and test/build matrices continue on individual failures so platform evidence is not hidden by fail-fast cancellation.

### Removed
- Obsolete checked-in diagnostic process telemetry logs from `tests/`.

### Security
- External network actions are disabled unless `VAPOR_SENTINEL_ENABLE_ACTIONS=1` is set.
- The webhook destination must use `https://`.
- The project does not claim secure file shredding or guaranteed memory zeroization.
- CI includes formatting, Clippy, test, release-build, and dependency-audit gates.

### Known Limitations
- The current runtime uses a fixed 100 MiB anomaly threshold (the metric is measured and reported in MiB).
- Evidence is generated in memory and printed as JSON; persistent evidence storage is not implemented.
- Host, kernel, hypervisor, firmware, and hardware security are outside the engine's current assurance boundary.
- A failing external notification returns an error but does not itself change the sentinel qualification state.
- Reproducible dependency locking and further CI supply-chain hardening remain engineering work.
- The runtime does not use a file-based EXIT sentinel.

## Contributing

Update this file as behavior changes. Security claims must be supported by executable evidence before being documented here.
