# Architecture

## Purpose

Vapor Sentinel is an evidence-oriented system monitoring engine. Its core responsibility is to transform observed system measurements into deterministic, inspectable state and policy decisions.

The architecture deliberately separates observation, interpretation, evidence, policy, and external effects.

## Processing boundary

    SYSTEM
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
       +------> Optional external action

### 1. Observation

`Observation` is the canonical input record:

- metric identity;
- sequence number;
- observed value.

Observation does not decide whether a system is healthy.

### 2. Deviation

`deviation::compare` compares the current observation with the previous observation.

It distinguishes:

- no baseline;
- unchanged;
- increased;
- decreased;
- duplicate sequence;
- out-of-order sequence;
- metric mismatch.

This layer is deterministic and has no external side effects.

### 3. Qualification

`qualification::qualify` maps the observation and deviation into a sentinel state:

- `Normal`
- `Degraded`
- `Anomalous`
- `Unknown`

Invalid ordering, identity mismatches, and insufficient baseline evidence become `Unknown`.

This is a deliberate fail-closed boundary.

### 4. Evidence

`EvidenceRecord` packages the observation, deviation, qualification, and threshold into a serializable record.

Evidence describes what the engine concluded from the supplied observations. It does not itself acquire authority or execute an action.

### 5. Policy

`policy::plan` converts evidence plus declarative notification intent into an `ActionPlan`.

The policy layer is side-effect free.

For the current implementation, only anomalous evidence can produce a notification plan. Normal, degraded, and unknown states produce no action.

### 6. Action boundary

`actions::execute` is the only current module responsible for external network effects.

Actions are disabled by default and require explicit environment configuration. Webhook delivery requires HTTPS.

This boundary exists so that deterministic evidence generation can be tested independently of external services.

## DSL boundary

The Vapor DSL is intentionally smaller than a general-purpose programming language.

Current supported forms are:

- `vapor name() { ... }`
- `if(METRIC) { ... }`
- `send("message");`

Loops, assignments, generic statements, filesystem operations, and arbitrary executable constructs are rejected.

The parser produces declarative intent; it does not perform the action.

## Runtime boundary

The current executable samples system memory through `sysinfo`.

Current metric:

`SYSTEM_USED_MEMORY_MB`

Current implementation threshold:

`> 100 MiB`

The threshold is an implementation default and should not be interpreted as a universal system-health rule.

## What this architecture does not claim

Vapor Sentinel is not currently:

- an EDR;
- an intrusion-prevention system;
- a secure enclave;
- a forensic erasure mechanism;
- a host/kernel/hypervisor integrity monitor;
- a guaranteed secure-memory-zeroization system;
- a persistent evidence database.

Those are outside the current implemented assurance boundary.

## Design principle

The central engineering rule is:

> External action must remain downstream of explicit observation, deterministic qualification, evidence construction, and policy.

That ordering makes the core pipeline easier to test, replay, audit, and reason about than a runtime in which observation and side effects are interleaved.
