# Project Roadmap

## Vision

Vapor Sentinel evolves from a focused real-time monitoring engine into a comprehensive, user-friendly security operations platform with sandboxed extensibility, hardware-level protections, and enterprise-grade alerting.

---

## Current Release: v0.1.0

### Status: ✅ Complete
- ✅ Core DSL parser (Pest grammar)
- ✅ Real-time system monitoring (4-second polling)
- ✅ Webhook-based alerting
- ✅ Secure file deletion (`shred` action)
- ✅ Conditional logic (if/while statements)
- ✅ Memory safety (zeroize integration)
- ✅ EXIT signal for graceful shutdown

### Limitations:
- Hardcoded webhook URL (placeholder)
- Memory threshold hardcoded (100 MB)
- DSL rules embedded in binary
- No configuration file support
- No persistence or logging to disk

---

## Planned: v0.2.0 (Q2 2025)

### Configuration System
- **Goal**: Externalize DSL rules and settings
- **Implementation**:
  - TOML-based configuration file (`vapor.toml`)
  - Define monitoring rules, webhook URLs, thresholds
  - Hot-reload capability (reload config without restart)
- **Example**:
  ```toml
  [monitoring]
  poll_interval_secs = 4
  memory_threshold_mb = 100
  
  [webhook]
  url = "https://alerts.example.com/vapor"
  timeout_secs = 10
  
  [rules]
  rules_file = "security_rules.vapor"
  ```

### Wasmtime Sandbox v1
- **Goal**: Execute untrusted plugins in isolated environment
- **Implementation**:
  - Embed Wasmtime runtime
  - Support WASM-compiled plugins
  - Restrict file system, network access within sandbox
  - Reduce attack surface for third-party integrations
- **Use Cases**:
  - Custom alert formatters
  - Vendor-specific monitoring adapters
  - User-defined decision logic

### Enhanced Alerting
- **Goal**: Support multiple alert destinations
- **Channels**:
  - HTTP webhooks (current)
  - Email (SMTP)
  - Slack integration
  - Discord webhooks
  - Syslog export
- **Smart Routing**: Route alerts based on severity/type

### Persistent Logging (Optional)
- **Goal**: Maintain audit trail
- **Features**:
  - Encrypted log file rotation
  - Tamper detection (HMAC signatures)
  - Log retention policies
  - Structured JSON logging

### Estimated Timeline
- **Alpha**: April 2025 (internal testing)
- **Beta**: May 2025 (contributor feedback)
- **Release**: June 2025

---

## Planned: v0.3.0 (Q4 2025)

### Hardware Protection Integration
- **Goal**: Leverage hardware-level memory defenses
- **Implementation**:
  - Page-level access controls via `region` crate
  - Lock vault memory into physical RAM (prevent swap)
  - Restrict permissions on sensitive memory regions
  - CPU-based DEP/ASLR integration
- **Benefit**: Even if process memory is compromised, sensitive data remains protected

### Advanced Process Monitoring
- **Goal**: Rich process-level visibility
- **Features**:
  - Per-process network connection tracking
  - File descriptor monitoring
  - System call tracing (optional)
  - Process ancestry tree
  - Threat correlation across processes

### Plugin System (v2)
- **Goal**: Native plugin support with security boundaries
- **Features**:
  - Rust plugin API (via cdylib)
  - Dynamic loading at runtime
  - Capability-based permissions model
  - Plugin versioning and dependency resolution

### Estimated Timeline
- **Development**: July–September 2025
- **Release**: October 2025

---

## Planned: v1.0.0 (Q1 2026)

### Production Readiness
- **Goal**: Enterprise deployment support
- **Features**:
  - High-availability cluster mode (primary/replicas)
  - Distributed alert aggregation
  - Web UI for configuration and monitoring
  - Role-based access control (RBAC)
  - Audit logging with cryptographic signing
  - FIPS 140-2 compliance (if applicable)

### Performance Optimization
- **Goal**: Sub-millisecond latency, minimal resource usage
- **Benchmarks**:
  - <10 MB memory footprint
  - <1% CPU usage at rest
  - <100 ms alert delivery time
- **Implementation**:
  - Lock-free data structures (optional)
  - SIMD processing for metrics aggregation
  - Zero-copy webhooks (if supported by ureq)

### Comprehensive Documentation
- **Goal**: Full developer and operator guidance
- **Content**:
  - Deployment playbooks for cloud platforms (AWS, GCP, Azure)
  - Kubernetes manifests and Helm charts
  - Docker image guidelines
  - Troubleshooting guide
  - Performance tuning guide

### Estimated Timeline
- **Development**: October 2025–December 2025
- **Beta Testing**: January 2026
- **Release**: March 2026

---

## Long-Term Vision (v2.0+)

### Distributed Monitoring Network
- Multi-node coordination for cross-system correlation
- Peer-to-peer alert propagation
- Consensus-based threat detection (voting)

### AI/ML Integration
- Anomaly detection using historical baselines
- Automatic rule generation from observational data
- Predictive alerting (forecast issues before they occur)

### Web3 Integration
- On-chain threat feeds integration
- Decentralized alert distribution
- Validator-specific monitoring templates

### Open Ecosystem
- Plugin marketplace for community contributions
- Certification program for approved plugins
- Integration with SIEMs (Splunk, Elastic, etc.)

---

## Development Priorities

### Near-term (Next 2 months)
1. [ ] v0.2.0 configuration system
2. [ ] Basic TOML config parsing
3. [ ] Wasmtime sandbox v1 (basic file I/O restrictions)
4. [ ] Multi-destination alerting (Slack, Discord)

### Medium-term (2–6 months)
5. [ ] Plugin system v1 (Rust API)
6. [ ] Hardware protection (memory locking)
7. [ ] Distributed monitoring v1 (primary/replica)
8. [ ] Web UI (basic dashboard)

### Long-term (6+ months)
9. [ ] ML-based anomaly detection
10. [ ] Web3 integrations
11. [ ] Plugin marketplace
12. [ ] v1.0.0 production release

---

## Community Contributions

We welcome pull requests aligned with this roadmap. Before contributing:
1. **Check the Issues**: See if your feature is already planned
2. **Discuss First**: Open an issue or discussion thread for major features
3. **Follow Security**: Ensure all changes comply with `SECURITY.md`
4. **Test Thoroughly**: Include tests and documentation
5. **Maintain Responsible Use Language**: No stealth, surveillance, or unauthorized access features

For roadmap discussions or ideas, please open a GitHub issue with the `enhancement` label.

---

## Success Metrics

By v1.0.0 release:
- [ ] 10+ production deployments
- [ ] <100 ms median alert delivery latency
- [ ] <10 MB memory footprint
- [ ] 100+ community-contributed plugins
- [ ] 99.95% uptime SLA compliance (if applicable)

---

**Last Updated**: January 2025
**Maintainers**: [Team info to be added]
**Questions?** See [CONTRIBUTING.md](../CONTRIBUTING.md) or open an issue.
