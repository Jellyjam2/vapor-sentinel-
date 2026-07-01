# Changelog

All notable changes to Vapor Sentinel are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

---

## [0.1.0] - 2025-01-XX (Initial Release)

### Added
- **Core Monitoring Engine**: Real-time system RAM tracking with configurable thresholds
- **Custom DSL (Pest Grammar)**: Domain-specific language for defining security actions:
  - `send(message)` — Webhook-based alert delivery
  - `shred(filepath)` — Secure file deletion with memory zeroization
  - `if(VARIABLE) { body }` — Conditional logic based on system metrics
  - `while(VARIABLE) { body }` — Looping constructs for repeated actions
- **HardenedStore Data Structure**: HashMap-based state vault for monitoring metrics
- **Process-Level Insights**: Identifies and reports processes consuming >50 MB RAM
- **Memory Safety**: Integration with `zeroize` crate for secure memory cleanup
- **Webhook Alerting**: JSON-formatted alerts sent to configurable endpoints via HTTPS
- **EXIT Signal Control**: Graceful shutdown via `EXIT` file creation (4-second polling interval)
- **Security-First Documentation**: SECURITY.md policy emphasizing authorized, defensive use only

### Changed
- None (initial release)

### Fixed
- None (initial release)

### Security
- Memory zeroization on exit prevents forensic recovery of sensitive data
- No persistent logging of system state to disk
- Explicit file paths only—no wildcard expansion or recursive deletion
- Transparent operation with full audit trail capability

### Known Limitations
- Webhook URL currently set to generic `https://webhook.site` (configure before production)
- Memory threshold hardcoded to >100 MB (parameterization planned for v0.2.0)
- No built-in encryption for alert transport (rely on HTTPS + TLS)
- Wasmtime sandbox support deferred to v0.2.0

### Roadmap for v0.2.0
- Parameterized configuration file support (TOML format)
- Wasmtime WASM sandbox for isolated plugin execution
- Hardware-level memory protection toggles (region crate enhancement)
- Enhanced alerting with multi-destination support
- Improved process filtering and correlation logic

---

## Contributing

When contributing, please update this CHANGELOG with:
- New features under `Added`
- Breaking changes under `Changed`
- Bug fixes under `Fixed`
- Security-related updates under `Security`

Maintain the format and always document your changes before submitting a pull request.
