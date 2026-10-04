# Vapor Sentinel

**Titan Black Swan TECHNOLOGIES · Defensive system assurance**

Vapor Sentinel is a Rust-based monitoring and evidence engine built around one strict idea:

> Observe first. Qualify the evidence. Decide policy. Only then permit an external action.

[![Rust CI](https://github.com/Jellyjam2/vapor-sentinel-/actions/workflows/rust.yml/badge.svg)](https://github.com/Jellyjam2/vapor-sentinel-/actions/workflows/rust.yml)
[![Windows](https://img.shields.io/badge/Windows-0078D4?logo=windows&logoColor=white)](#platform-support) [![macOS](https://img.shields.io/badge/macOS-000000?logo=apple&logoColor=white)](#platform-support) [![Linux](https://img.shields.io/badge/Linux-FCC624?logo=linux&logoColor=black)](#platform-support)

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
| Metric | SYSTEM_USED_MEMORY_MIB |
| Threshold | Configured in `vapor-sentinel.json` (default 100 MiB) |
| Observation | Monotonic sequence number |
| Deviation | unchanged / increased / decreased / invalid ordering |
| Qualification | Normal / Degraded / Anomalous / Unknown |
| Evidence | In-memory EvidenceRecord, JSON serializable |
| Policy | Pure ActionPlan derivation |
| DSL | Restricted Pest grammar with bounded comparisons |
| Actions | Optional HTTPS webhook |
| Bounded mode | VAPOR_SENTINEL_ONESHOT=1 or VAPOR_SENTINEL_EXIT=1 |
| Dashboard | Static, read-only presentation preview |

The memory metric reflects system used memory reported by sysinfo, converted to MiB. The threshold is an implementation default, not a universal safe-operating value.

## DSL boundary

The DSL currently describes declarative notification intent only:

    vapor sentinel() {
        if(SYSTEM_USED_MEMORY_MIB >= 100) {
            send("CRITICAL_MEMORY_THRESHOLD");
        }
    }

The DSL supports metric selectors and bounded numeric comparisons (`>`, `>=`, `<`, `<=`, `==`, `!=`). Loops, assignments, and generic executable statements are rejected rather than silently ignored.

The parser itself performs no filesystem or network side effects. Runtime policy is loaded from an explicit local DSL file.

## Configuration

Runtime configuration is loaded from `vapor-sentinel.json` by default, or from the path in `VAPOR_SENTINEL_CONFIG`. The configuration validates the anomaly threshold, polling interval, and DSL policy path before startup.

The default policy is `policies/default.vapor`.

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
    ├── config.rs         validated runtime configuration
    └── vapor.pest        restricted DSL grammar

    dashboard/
    ├── index.html        read-only operator interface preview
    ├── styles.css        presentation styling
    └── app.js            presentation-only sample evidence

    .github/workflows/
    └── rust.yml          CI gates

    ARCHITECTURE.md        system design boundary
    SECURITY.md            responsible-use + security boundary
    policies/default.vapor default declarative notification policy
    vapor-sentinel.json    runtime configuration

    CHANGELOG.md           implementation history

## Platform support

CI targets all three supported desktop/server families:

**Windows** · **macOS** · **Linux**

The dashboard is a browser-based static preview and currently has no live evidence transport.

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
