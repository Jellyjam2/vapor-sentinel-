# Contributing

## Engineering standard

Vapor Sentinel is developed as an evidence-oriented system. Changes should preserve the separation between:

`Observation -> Deviation -> Qualification -> Evidence -> Policy -> Action`

Do not introduce external side effects into the observation, deviation, qualification, evidence, or policy layers. Stateful scheduling belongs in lifecycle; external work belongs in delivery/actions.

## Before opening a pull request

Run locally:

```powershell
cargo fmt -- --check
cargo clippy --locked --all-targets --all-features -- -D warnings
cargo test --locked --all-targets
cargo build --locked --release
```

If the change affects runtime behavior, also run the bounded smoke test:

```powershell
$env:VAPOR_SENTINEL_ONESHOT="1"
cargo run --locked
Remove-Item Env:VAPOR_SENTINEL_ONESHOT
```

## Evidence rules

Do not document a security property unless the repository contains implementation and executable evidence supporting it.

Use precise status language:

- **MEASURED** — directly observed output.
- **VERIFIED** — supported by repeatable implementation and test evidence.
- **INFERRED** — reasoned from verified facts but not directly tested.
- **NOT PROVEN** — evidence is insufficient.

A passing CI run does not by itself prove a security claim.

## DSL changes

The DSL is intentionally constrained.

If adding syntax, define its semantics before implementing it and add tests for:

1. accepted syntax;
2. rejected syntax;
3. nested behavior;
4. malformed input;
5. side-effect isolation.

Do not silently accept syntax that the executor does not implement.

## Action changes

External actions must remain behind the action boundary.

New actions should:

- be disabled by default;
- validate configuration;
- surface errors;
- have deterministic policy inputs;
- avoid being invoked directly from observation or qualification code.

## Documentation

Update README.md, ARCHITECTURE.md, SECURITY.md, and CHANGELOG.md when behavior or security boundaries change.

Security-sensitive claims must be supported by executable evidence before they are promoted into documentation.

## Pull requests

Keep pull requests focused and reviewable.

Describe:

- what changed;
- why it changed;
- what was tested;
- what remains unproven.

Do not merge or publish a release without explicit maintainer approval.
