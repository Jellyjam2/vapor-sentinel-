# Vapor Sentinel Roadmap

**Company:** Titan Black Swan TECHNOLOGIES
**Product:** Vapor Sentinel

This roadmap reflects the implementation boundary rather than historical target dates. Dates are intentionally omitted until delivery commitments are established.

## Phase A — Core verification

Status: in progress

- [x] Canonical observation type
- [x] Deterministic deviation engine
- [x] Fail-closed qualification
- [x] Evidence record
- [x] Pure policy boundary
- [x] Restricted DSL grammar
- [x] Explicit rejection of unsupported DSL constructs
- [x] Isolated optional notification executor
- [ ] Fully green CI on the current integrated branch
- [ ] Source-level release reinspection
- [ ] Runtime integration test suite

## Phase B — Operator product

Status: planned

- [ ] Read-only operator dashboard
- [ ] Live evidence stream
- [ ] State history and transition display
- [ ] Evidence export
- [ ] Configuration management
- [ ] Clear action status reporting
- [ ] Installation and upgrade workflow

The dashboard must remain an observation/evidence interface. It must not acquire hidden authority or silently perform privileged actions.

## Phase C — Pilot readiness

Status: planned

- [ ] Reproducible release artifact
- [ ] Installation guide
- [ ] Threat model
- [ ] Security review
- [ ] Operational runbook
- [ ] Failure-mode testing
- [ ] Customer demo environment
- [ ] Pilot acceptance criteria

## Phase D — Commercial release

Status: planned

- [ ] Commercial packaging
- [ ] Product website
- [ ] Technical one-pager
- [ ] Sales/demo material
- [ ] Pricing and licensing
- [ ] Support process
- [ ] Customer telemetry/privacy policy
- [ ] Advertising claims tied to verified functionality

## Future research

Potential future work may include:

- additional metrics and adapters;
- sandboxed extensions;
- richer evidence retention;
- enterprise integrations;
- distributed monitoring;
- advanced anomaly models.

These are research/product directions, not current capabilities.

## Release principle

A feature is not considered commercially available merely because its type or interface exists. It must be implemented, tested, documented, and consistent with observable runtime behavior.
