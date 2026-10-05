# Threat model for the current runtime

## Trusted deployment inputs

The operator controls the binary, configuration, local policy, output location, source label, process environment, and optional HTTPS destination. The OS, sysinfo measurements, wall clock, Rust toolchain, and dependencies are trusted inputs. Protect those inputs and run as an ordinary user; administrator privileges are not required for the intended workflow.

## Inputs that receive defensive validation

Policy/config/replay files may be malformed or accidentally excessive. File reads and parser structure are bounded before recursive interpretation. Unknown fields, unavailable metric identifiers, invalid sequence relations, invalid endpoints, and inconsistent action/evidence pairs are rejected. Replay never enables networking. The viewer treats file contents as untrusted data and uses textContent rather than HTML insertion.

The external endpoint can fail, be slow, return non-success status, or redirect. HTTPS validation, no redirects, bounded requests, a bounded worker, explicit outcomes, and repeat/retry intervals constrain this interaction. Error text excludes the endpoint and response body. The allowed destination itself is chosen by the trusted operator; there is no multi-tenant destination allowlist or SSRF defense against an operator authorized to configure arbitrary endpoints.

## Protected properties

- Parser structure cannot exceed the checked depth before recursive parsing starts.
- Unknown evidence does not authorize anomaly notifications or recovery.
- Notification and recovery are separate action types with corresponding state checks.
- Unavailable/failed delivery does not become a false success record.
- The monitoring thread does not wait synchronously for webhook delivery.
- Replay produces no external notification, regardless of action environment variables.

Executable evidence for these properties is in the unit and runtime tests. Cross-platform behavior must still be established by successful platform CI; this document does not substitute for those runs.

## Exclusions and residual risks

Compromised hosts, malicious in-process code, forged telemetry, manipulated clocks, replaced binaries, and modified logs are not detected by a cryptographic trust system. Modules are not process isolation. Hashes identify policy bytes but do not authenticate them. Run/source labels are not attestation.

Sampling can miss short spikes; host-level available memory may not represent a container's limits. Unknown is visible in local evidence but has no independent remote health-alert channel. Output files can consume disk without operator retention. Output paths must be private to the deployment; path prechecks are not a defense against a hostile actor concurrently replacing files or hard links. JSONL flushes are not transactional storage or an fsync durability guarantee. Queue, retry, cooldown, and baseline state are lost on restart.

The project supports no destructive remediation or arbitrary plugins. Expanding authority requires a new threat model and independent verification of the new boundary.
