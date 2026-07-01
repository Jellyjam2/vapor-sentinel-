# Security Policy & Responsible Use

## Overview

Vapor Sentinel is a **defensive system monitoring engine** designed for authorized security research and protective system administration. This policy outlines core responsibilities for all users and contributors.

---

## Authorized Use Only

✅ **Permitted Applications:**
- Local system monitoring on devices you own or have explicit written authorization to monitor
- Defensive research in controlled environments (test VMs, isolated networks)
- Automated incident response for validator nodes and infrastructure you operate
- Security testing with prior consent from all stakeholders

❌ **Explicitly Prohibited:**
- Unauthorized surveillance or monitoring of any system or user
- Covert installation or stealth persistence mechanisms
- Monitoring without informed consent from affected parties
- Use on third-party systems without explicit written permission
- Bypassing access controls or authentication mechanisms
- Data exfiltration or unauthorized data collection

---

## Core Security Principles

### 1. **No Stealth, No Persistence**
- Vapor Sentinel operates transparently with clear logging
- No mechanisms for hiding execution, suppressing logs, or establishing persistence
- System administrators and users must always be aware of monitoring activity
- Clean exit via `EXIT` file signal—no background daemons maintained without consent

### 2. **Explicit Approval Required**
- Obtain written authorization before deploying on any system
- Clearly document stakeholder consent for each monitored environment
- Provide users/operators with disable/exit mechanisms
- Honor user requests to cease monitoring immediately

### 3. **Memory Safety & Data Protection**
- All sensitive data (keys, memory contents) are securely zeroized on exit
- `zeroize` crate with derive features ensure memory is overwritten
- No sensitive data persisted to disk without encryption
- Regular security audits of memory access patterns

### 4. **Bounded Destruction**
- File shredding (`shred` action) only targets explicitly named files
- No recursive directory deletion or destructive defaults
- User provides explicit file paths; no wildcard expansion
- Verification of file existence before shredding operations

### 5. **Transparent Alerting**
- Webhook alerts contain only metadata (thresholds, timestamps, process names)
- No user data, system secrets, or sensitive credentials in alert payloads
- Webhook endpoints must use HTTPS with certificate validation
- Users can audit all generated alerts before sending

---

## Deployment Guidelines

### Before Deployment:
1. Review the monitoring configuration and expected behavior
2. Obtain written authorization from all affected stakeholders
3. Document retention policies for alerts and logs
4. Establish clear incident response procedures

### During Deployment:
1. Operate with full transparency—log all monitoring actions
2. Make exit mechanisms easily accessible
3. Periodically validate that monitoring is still authorized
4. Monitor alert delivery and effectiveness

### After Deployment:
1. Maintain audit logs of all monitoring activities
2. Promptly respond to authorization revocations
3. Securely delete all retained data upon exit
4. Document lessons learned for future deployments

---

## Contribution Standards

Contributors must:
- Acknowledge that Vapor Sentinel is for **authorized, defensive use only**
- Avoid adding stealth, persistence, or evasion features
- Document security implications of all changes
- Follow secure coding practices (input validation, bounds checking, safe memory handling)
- Pass security review before merge

### Pull Request Checklist:
- [ ] No unauthorized surveillance features
- [ ] No persistence mechanisms added
- [ ] Memory safety verified (no data leaks)
- [ ] Secure defaults enforced
- [ ] Security implications documented

---

## Incident Reporting

**To report a security vulnerability or misuse concern:**
1. Email: [security contact to be added]
2. Do not open public issues for security problems
3. Allow 30 days for response and fix deployment
4. Coordinate disclosure with maintainers

---

## Legal Disclaimer

**Vapor Sentinel is provided as-is for authorized, lawful uses only.** Users are solely responsible for ensuring compliance with all applicable laws and regulations in their jurisdiction. Unauthorized surveillance, wiretapping, or unauthorized system access may violate criminal and civil law.

The maintainers assume no liability for misuse of this tool. Users accept full responsibility for all consequences arising from their use of Vapor Sentinel.

---

## Questions?

If you have questions about responsible use or deployment authorization, please contact the maintainers before proceeding.
