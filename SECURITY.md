# Security Policy & Responsible Use

**Vapor Sentinel is developed and maintained by Titan Black Swan TECHNOLOGIES.**

Vapor Sentinel is a defensive monitoring product. It is intended for authorized administration, monitoring, testing, and security research.

## Authorized use

Use the software only on systems for which you have appropriate authorization.

Examples include:

- systems you own;
- infrastructure operated by your organization;
- controlled test environments;
- security research performed with explicit permission.

Do not use Vapor Sentinel for unauthorized monitoring, covert surveillance, persistence, access-control bypass, credential theft, or data exfiltration.

## Current security boundaries

### Observation and evidence

The monitoring path is designed around explicit typed observations:

Observation -> Deviation -> Qualification -> EvidenceRecord -> ActionPlan

Qualification is fail-closed. Missing baselines and invalid observation ordering are represented as Unknown rather than silently treated as normal.

### External actions

External notification side effects are isolated in src/actions.rs.

Webhook notification requires both:

1. VAPOR_SENTINEL_ENABLE_ACTIONS=1
2. an HTTPS VAPOR_SENTINEL_WEBHOOK_URL

Action errors are surfaced rather than discarded.

The current implementation does not provide a destructive filesystem action or file-based shutdown control. The bounded runtime uses VAPOR_SENTINEL_ONESHOT=1 or VAPOR_SENTINEL_EXIT=1 for explicit process termination; it does not inspect or delete arbitrary files. Vapor Sentinel does not claim secure shredding or guaranteed unrecoverable file destruction.

### Memory handling

The current runtime does not claim cryptographic memory zeroization. The previous zeroization dependency has been removed because the implemented runtime does not provide that guarantee.

Do not treat ordinary Rust ownership, collection clearing, or process termination as proof of secure memory erasure.

### Runtime configuration

Runtime configuration is explicit rather than compiled into the executable. Startup validates the threshold, polling interval, and configured DSL path. The default configuration is `vapor-sentinel.json`; `VAPOR_SENTINEL_CONFIG` may select another configuration file.

Treat the configuration and DSL policy files as deployment-controlled inputs; they must not be accepted from untrusted network sources.

### DSL boundary

The Vapor DSL intentionally supports a small set of declarative notification constructs, including bounded numeric comparisons. Unsupported constructs such as loops, assignments, and generic executable statements are rejected rather than silently ignored.

The DSL itself does not obtain external authority.

## Security assumptions

This project does not claim to prevent compromise of the host operating system, kernel, hypervisor, firmware, or physical environment.

It is not a secure enclave, EDR replacement, forensic guarantee, or universal intrusion-prevention mechanism.

Security claims must be tied to behavior that is implemented and tested in the repository.

## Deployment guidance

Before deployment:

1. establish authorization for the monitored environment;
2. review the configured metric and threshold;
3. keep external actions disabled until the destination and operational behavior are reviewed;
4. test the one-shot path in a controlled environment;
5. retain resulting evidence according to your organization's policy.

## Reporting vulnerabilities

Please report security vulnerabilities privately to the project maintainers rather than publishing sensitive exploit details in a public issue.

A dedicated security contact will be published when the commercial release process establishes one.

## Legal

Vapor Sentinel is provided for lawful, authorized defensive use. Operators are responsible for compliance with applicable laws, contracts, policies, and monitoring-consent requirements.
