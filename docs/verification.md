# Verification of the runtime repair

Measured locally on 5 October 2026 using Linux x86_64, rustc 1.99.0 and the committed Cargo.lock. These results apply to the repair source; they do not certify production security or other operating systems.

| Check | Local result |
|---|---|
| `cargo fmt --all -- --check` | Pass |
| `cargo clippy --locked --all-targets --all-features -- -D warnings` | Pass |
| `cargo test --locked --all-targets` | 63 Rust tests pass: 54 unit, 4 assurance integration, 5 executable integration |
| `node --test tests/dashboard.test.cjs` | 14 recording-model tests pass against actual Rust replay output and the bundled example |
| `cargo build --locked --release` | Pass |
| Release executable, live observation, `VAPOR_SENTINEL_ONESHOT=1`, actions disabled | Two observations; Unknown then Normal on this host; successful exit |
| Release executable replay fixture | Expected five evaluations, one scheduled alert, cooldown, and recovery; no network delivery |
| cargo-audit 0.22.2 against committed Cargo.lock | 120 dependencies; zero known vulnerabilities and no advisory warnings at time checked |

The audit used RustSec database commit `ef6173cbc5c50ec8166f9a5b28f07834144373ee`, last updated 3 October 2026. This result is time dependent and is not a guarantee against undisclosed flaws.

## Concrete regressions exercised

- A 9,000-level policy below the source byte limit returns an error without entering the recursive parser. The exact supported depth remains accepted.
- Every comparison operator has matching qualifier and notification semantics, including equality boundaries and low-value alerts.
- Startup, duplicate, reversed, skipped, and mismatched observations remain Unknown and cannot authorize a notification.
- Ordinary below-threshold changes are Normal; nested conditions require all predicates.
- Unknown metrics, spaced identifiers, excessive aggregate messages, oversized files, bad intervals, and unknown config fields fail before monitoring.
- Repeated alarms cool down; failed delivery backs off; recovery requires consecutive normal samples; Unknown does not count as recovery.
- A blocked fake delivery worker does not block enqueueing/sampling work, and its queue has a tested capacity limit.
- Actual CLI replay ignores even deliberately enabled/malformed webhook environment configuration and records skipped delivery.
- Recording appends the same JSONL emitted to stdout; repeated replay reproduces evidence and decisions.
- The viewer consumes the actual runtime schema, navigates recorded states, displays recorded outcomes, rejects malformed/oversized input, uses text sinks, and prevents stale file-load races.

## Still unverified

The runtime repair passed Linux, macOS, Windows, and dependency-audit jobs on main at `19e4bd2`: [CI run 37248623682](https://github.com/Jellyjam2/vapor-sentinel-/actions/runs/37248623682). The earlier billing-related block did not affect that run.

The refined dashboard was subsequently merged through PR #5. Post-merge `main` at `ef896804fb7466a7be6694709622efd940b216a6` passed Linux, macOS, Windows and dependency-audit jobs, including the Linux browser suite: [CI run 37254736772](https://github.com/Jellyjam2/vapor-sentinel-/actions/runs/37254736772).

No real external webhook was contacted during repair. Delivery behavior was tested with disabled clients and controlled fake transports; live DNS/TLS/provider behavior still requires a deployment-specific test.

The refined dashboard adds `tests/dashboard.browser.cjs`, which runs Chromium against actual files, exports, keyboard controls, stream selection, malformed inputs, file-load races, and 320–1440px layouts. It also runs axe-core checks against empty, loaded, and mobile views. Linux CI uploads screenshots and reports as `dashboard-browser-evidence`; consult the check result for the exact revision. Local Chrome launch was unavailable in the repair environment. This test suite does not establish Safari/Firefox support or constitute a manual accessibility audit.

No resource-overhead benchmark, authenticated evidence proof, service installation pilot, log-rotation system, or durable-delivery guarantee is claimed. Maintainers still need to choose licensing and establish a private vulnerability reporting channel before a production release.

## Pilot measurement tooling

The [pilot guide](operational-pilot.md) defines a repeatable, actions-disabled Linux measurement and environment-specific acceptance criteria. It does not close the representative workload, service installation, live webhook or retention gates.

The initial tool checks used the Linux release binary from the successful `ef896804` main CI run. The downloaded archive matched its GitHub artifact SHA-256 (`55dbbec2b1c4438d7f97b2f40b24a5acd826b109f8c72ac953ee8407c6da1bf8`); the extracted executable's SHA-256 was `82199052599d605b8ce2a29ec8a4e17f60f0319b198c6e0c3dca949856e974b5`. No local Rust compiler was available for this tooling-only change; the workflow runs the full Rust matrix on the proposed source.

Local process checks exercised a deliberately enabled/invalid inherited webhook configuration, copied inputs, real live observations, skipped delivery, incomplete evidence, nonzero exit, timeout termination and existing-output protection. A 30-observation run at one-second intervals completed, but the environment denied access to `/proc/PID/exe`. The tool correctly marked the measurement incomplete and did not claim a post-exec Rust memory footprint. The resource-dependent integration test explicitly skips that restricted case; the separate CI smoke command must produce a complete report to pass.

Linux CI runs six measurement regression cases against its release executable, followed by a three-observation smoke measurement, and uploads `pilot-measurement-evidence`. Consult the exact revision's check result and artifact for measured values. Shared-runner smoke results establish tool operation, not representative workload overhead or a production performance budget.
