# Vapor Sentinel Architecture

**Product:** Vapor Sentinel
**Company:** Titan Black Swan TECHNOLOGIES

## Purpose

Vapor Sentinel is an evidence-driven defensive monitoring engine. Its central architectural rule is that observation, qualification, evidence, policy, and external execution are separate concerns.

SYSTEM
  |
  v
Metric observation
  |
  v
Observation
  |
  v
Deviation
  |
  v
Qualification
  |
  v
EvidenceRecord
  |
  v
ActionPlan
  |
  +------> Action Executor (optional)
  |
  v
Operator output

## 1. Observation

src/observation.rs defines the canonical observation:

- metric name;
- monotonically increasing sequence;
- numeric value.

The executable currently creates observations from sysinfo's reported used system memory.

The observation layer does not decide whether a value is safe.

## 2. Deviation

src/deviation.rs compares the current observation with the previous observation.

It distinguishes:

- no baseline;
- unchanged value;
- increased value;
- decreased value;
- duplicate sequence;
- out-of-order sequence;
- metric mismatch.

Invalid ordering or identity is not treated as a normal condition.

## 3. Qualification

src/qualification.rs maps evidence into:

- Normal
- Degraded
- Anomalous
- Unknown

The qualifier is deliberately fail-closed. A missing baseline or invalid observation relationship produces Unknown.

A threshold breach is anomalous only after the observation relationship is valid.

## 4. Evidence

src/evidence.rs binds the current observation, deviation, threshold, qualification state, and reason into one serializable EvidenceRecord.

This record is the primary auditable result of one evaluation cycle.

Evidence does not execute actions.

## 5. Policy

src/policy.rs is pure policy logic.

Current behavior:

- Anomalous evidence can produce ActionPlan::Notify;
- Normal, Degraded, and Unknown produce ActionPlan::NoAction.

The policy layer does not perform network or filesystem operations.

## 6. Action execution

src/actions.rs contains the external side-effect boundary.

The current implementation supports optional HTTPS webhook notification.

Actions require explicit opt-in through VAPOR_SENTINEL_ENABLE_ACTIONS=1 and an HTTPS endpoint in VAPOR_SENTINEL_WEBHOOK_URL.

The current implementation intentionally does not provide filesystem deletion or claim secure shredding.

## 7. Vapor DSL

src/vapor.pest defines a deliberately small grammar.

Supported constructs are:

- vapor name() { ... }
- if(METRIC) { ... }
- bounded numeric comparisons such as `if(METRIC >= 100) { ... }`
- send("message");

Unsupported loops, assignments, and generic statements are rejected.

The parser produces a typed declarative program. It does not directly execute network or filesystem effects.

## 8. Runtime integration

src/main.rs is the executable integration point.

Each cycle:

1. obtains a system observation;
2. evaluates deviation and qualification;
3. creates an evidence record;
4. reads matching declarative notification messages from the parsed DSL;
5. derives an action plan;
6. optionally executes the isolated notification action;
7. emits serialized evidence and policy output.

VAPOR_SENTINEL_ONESHOT=1 provides bounded one-cycle execution. VAPOR_SENTINEL_EXIT=1 provides explicit termination control without a file sentinel.

## 9. Explicit non-goals

The current implementation does not claim:

- secure file shredding;
- guaranteed memory zeroization;
- kernel-level protection;
- hardware security;
- stealth or persistence;
- universal anomaly detection;
- automated recovery of arbitrary infrastructure;
- external authority acquisition by the dashboard, DSL, or model.

Those capabilities require separate implementation and verification.

## 10. Verification model

CI covers:

- Rust formatting;
- tests;
- Clippy with warnings denied;
- stable/nightly platform builds;
- release builds;
- dependency auditing.

CI is necessary but not sufficient for a product release. Runtime behavior, security claims, documentation, and customer-facing functionality must be inspected against the same source of truth.
