# Vapor Sentinel

**Vapor Sentinel is a Titan Black Swan TECHNOLOGIES product.**

Vapor Sentinel is a Rust-based defensive monitoring and evidence engine. It observes a system metric, produces a deterministic observation sequence, compares observations, qualifies the resulting state, records evidence, and derives a side-effect-free policy plan.

## Current architecture

SYSTEM OBSERVATION
        |
        v
Observation
        |
        v
DEVIATION
        |
        v
QUALIFICATION
        |
        v
EVIDENCE RECORD
        |
        v
POLICY PLAN
        |
        +----> OPTIONAL ACTION EXECUTOR
        |
        v
OPERATOR OUTPUT

The core boundaries are deliberate:

- Observation is not qualification.
- Qualification is not authority.
- Evidence is not an action.
- Policy is not execution.
- External network effects are disabled unless explicitly enabled and configured.

## Product ownership

**Company:** Titan Black Swan TECHNOLOGIES
**Product:** Vapor Sentinel
**Position:** Defensive system monitoring and evidence-driven sentinel infrastructure.

## What is implemented

- Rust monitoring runtime using sysinfo
- Canonical Observation records with sequence numbers
- Deterministic deviation classification
- Fail-closed qualification (Unknown when evidence is insufficient or invalid)
- Serializable EvidenceRecord
- Pure ActionPlan policy boundary
- Restricted Vapor DSL parsed with Pest
- Explicit rejection of unsupported DSL constructs
- Optional HTTPS webhook notification through an isolated action module
- One-shot execution mode for bounded demonstrations and testing

## Current monitoring metric

The executable currently observes SYSTEM_USED_MEMORY_MB.

This represents system memory currently reported as used by sysinfo, converted to MiB.

The current sentinel threshold is 100 MiB. This is an implementation default, not a claim about a universal safe operating threshold.

## Action safety

Network notification is opt-in:

VAPOR_SENTINEL_ENABLE_ACTIONS=1
VAPOR_SENTINEL_WEBHOOK_URL=https://example.invalid/endpoint

Without the enable flag and HTTPS endpoint, notification is skipped.

Vapor Sentinel does not currently implement secure file shredding or guaranteed memory zeroization. The current release path intentionally avoids destructive filesystem actions.

## Verification status

The repository uses CI for formatting, tests, Clippy, release builds, and dependency auditing.

A green CI run is necessary but not sufficient for a commercial release. Runtime integration, source reinspection, security review, and product documentation must also agree with the implementation.

## Authorized defensive use

Use Vapor Sentinel only on systems and infrastructure you own or are explicitly authorized to monitor. See SECURITY.md for the responsible-use boundary.
