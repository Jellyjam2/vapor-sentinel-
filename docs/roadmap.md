# Delivery roadmap

Current implementation and exact semantics: [README](../README.md) and [architecture](../ARCHITECTURE.md).

## Immediate release gates

- [ ] Successful hosted CI on the repaired source for Linux, macOS, and Windows.
- [ ] Maintainer-selected licensing and verified private security contact.
- [ ] Representative workload benchmarks and operational pilot acceptance criteria.
- [ ] Installation/service management and output retention guidance validated in a real deployment.

## Implemented in the repair branch

- [x] Deterministic evidence and consistent DSL comparisons.
- [x] Early parser/resource bounds and registered metric validation.
- [x] Notification cooldown, retry scheduling, recovery debounce, and bounded background delivery.
- [x] Versioned evaluation/delivery JSONL, policy hash, and optional recording.
- [x] Safe deterministic replay and executable regression tests.
- [x] Read-only viewer for actual recordings, with no live authority.

Implementation status is not a claim of cross-platform or production verification; see [verification](verification.md).

## Subsequent work, driven by pilot needs

Live evidence transport, durable delivery, retention/rotation, separate remote monitor-health alerts, container-aware adapters, additional metrics, and deployment packaging should be selected from measured user requirements.

Signed evidence/policies, independently verified replay, capability isolation, attestation, and distributed assurance are future trust boundaries. The preserved [future vision](../FUTURE_UPGRADES_ROADMAP.txt) is not current functionality. Add one boundary at a time with a threat model and executable acceptance evidence.
