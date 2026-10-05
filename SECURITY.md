# Security policy

Vapor Sentinel is intended for monitoring systems you own or are authorized to administer.

The current implemented boundary is described in [ARCHITECTURE.md](ARCHITECTURE.md) and the [threat model](docs/threat-model.md). Do not treat it as an EDR, secure enclave, forensic erasure tool, signed evidence ledger, or host-compromise prevention system.

## Safe operation

- Network actions default to disabled. Enabling them requires a valid operator-selected HTTPS destination.
- Replay always disables delivery. Use it to inspect decisions without contacting a service.
- Policy/configuration/output locations and the process environment are deployment-controlled. Protect their permissions.
- Unknown measurements are recorded and cannot authorize anomaly or recovery notifications.
- Recovery messages describe a policy condition clearing; they do not claim that a host is secure.
- External effects are limited to HTTPS notifications. No secure shredding or memory-zeroization guarantee is implemented.
- Maintain output retention and monitor process health independently. A process that cannot obtain a metric or write evidence exits with an error.

Webhook errors never include endpoint credentials or response bodies. HTTPS is enforced by the client as well as URL validation, redirects are disabled, and requests have a timeout. Network delivery is isolated in a bounded worker and does not grant a policy arbitrary execution authority.

## Dashboard boundary

The evidence workspace is a local, read-only recording inspector. It validates schema, dates, safe integers, identifiers, delivery types, and size limits before accepting a file. Text sinks and a content security policy prevent imported strings from becoming HTML or initiating network connections through the app. It does not upload recordings or store them persistently. An operator can explicitly export a filtered file to their device.

A displayed state is a claim in the supplied recording, not independent proof that a host is healthy. Policy hashes identify bytes and are not signatures. Example recordings are labeled, and missing delivery evidence is not reported as success. Browser behavior and WCAG checks are covered by automated CI, without claiming a full browser matrix or manual accessibility/security audit.

## Reporting

A maintainer-designated private vulnerability contact is still required before a public production release. The repository does not currently document a verified working private reporting channel. Do not include secrets or sensitive deployment data in public issues. Maintainers should establish and test a private reporting channel before making production support commitments.

## Changes to security claims

Claims must identify their assumptions and refer to implementation and repeatable checks. A green build alone does not prove operational security, and a source hash is not authentication. Future signed-policy, attestation, isolation, and distributed-assurance work remains explicitly unimplemented.
