# Vapor Sentinel

**Titan Black Swan TECHNOLOGIES · Defensive system assurance**

Vapor Sentinel is a Rust-based monitoring and evidence engine built around one strict idea:

> Observe first. Qualify the evidence. Decide policy. Only then permit an external action.

[![Rust CI](https://github.com/Jellyjam2/vapor-sentinel-/actions/workflows/rust.yml/badge.svg)](https://github.com/Jellyjam2/vapor-sentinel-/actions/workflows/rust.yml)

## Engine

    SYSTEM OBSERVATION
            ↓
      Observation
            ↓
       Deviation
            ↓
     Qualification
            ↓
      EvidenceRecord
            ↓
        ActionPlan
            ↓
    Optional Action

The boundaries are intentional:

- Observation is not qualification.
- Qualification is not authority.
- Evidence is not action.
- Policy is not execution.
- Invalid or insufficient evidence becomes Unknown rather than silently becoming normal.
- External network effects are disabled unless explicitly enabled and configured.

See ARCHITECTURE.md for the design rationale and SECURITY.md for the security boundary.

## Current implementation

| Area | Current behavior |
|---|---|
| Runtime | Rust + sysinfo |
| Metric | SYSTEM_USED_MEMORY_MB |
| Threshold | > 100 MiB implementation default |
| Observation | Monotonic sequence number |
| Deviation | unchanged / increased / decreased / invalid ordering |
| Qualification | Normal / Degraded / Anomalous / Unknown |
| Evidence | In-memory EvidenceRecord, JSON serializable |
| Policy | Pure ActionPlan derivation |
| DSL | Restricted Pest grammar |
| Actions | Optional HTTPS webhook |
| Bounded mode | VAPOR_SENTINEL_ONESHOT=1 |

The memory metric reflects system used memory reported by sysinfo, converted to MiB. The threshold is an implementation default, not a universal safe-operating value.

## DSL boundary

The DSL currently describes declarative notification intent only:

    vapor sentinel() {
        if(SYSTEM_USED_MEMORY_MB) {
            send("CRITICAL_MEMORY_THRESHOLD");
        }
    }

Unsupported constructs such as loops, assignments, and generic executable statements are rejected rather than silently ignored.

The parser itself performs no filesystem or network side effects.

## Action safety

Webhook notification requires both:

    VAPOR_SENTINEL_ENABLE_ACTIONS=1
    VAPOR_SENTINEL_WEBHOOK_URL=https://your-approved-endpoint.example/

Without both conditions, notification is skipped.

The current release path does not implement secure file shredding and does not claim guaranteed memory zeroization.

## Verification

CI covers formatting, Clippy with warnings denied, tests on Linux/macOS/Windows, stable and nightly test matrices, release builds, and dependency auditing.

A green CI run is necessary but not sufficient for a commercial release. Runtime integration, security review, source reinspection, and documentation must agree with the implementation before release claims are promoted.

## Repository map

    src/
    ├── actions.rs        external side-effect boundary
    ├── deviation.rs      observation comparison
    ├── dsl.rs            parser + declarative program model
    ├── evidence.rs       evidence record construction
    ├── observation.rs    canonical observations
    ├── policy.rs         side-effect-free action planning
    ├── qualification.rs  sentinel-state qualification
    ├── main.rs           system metric runtime
    └── vapor.pest        restricted DSL grammar

    .github/workflows/
    └── rust.yml          CI gates

    ARCHITECTURE.md        system design boundary
    SECURITY.md            responsible-use + security boundary
    CHANGELOG.md           implementation history

## Development

PowerShell:

    cargo fmt -- --check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo test
    cargo build --release

For a bounded runtime smoke test:

    $env:VAPOR_SENTINEL_ONESHOT="1"
    cargo run
    Remove-Item Env:VAPOR_SENTINEL_ONESHOT

## Product

**Company:** Titan Black Swan TECHNOLOGIES  
**Product:** Vapor Sentinel  
**Position:** Defensive monitoring and evidence-driven sentinel infrastructure.

## Responsible use

Use Vapor Sentinel only on systems and infrastructure you own or are explicitly authorized to monitor. See SECURITY.md.