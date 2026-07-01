# Pull Request Template

## Description
Please include a summary of changes and related context. Link any related issues.

Fixes # (issue)

## Type of Change
- [ ] Bug fix (non-breaking)
- [ ] New feature (non-breaking)
- [ ] Breaking change
- [ ] Documentation update

## Testing
Describe the testing you performed:
- [ ] Unit tests added/updated
- [ ] Integration tests added/updated
- [ ] Manual testing performed
- [ ] All tests pass: `cargo test`

## Formatting & Linting
- [ ] Code formatted: `cargo fmt`
- [ ] No clippy warnings: `cargo clippy`
- [ ] No new unsafe code introduced (or justified)

## Security Checklist
- [ ] Changes comply with [SECURITY.md](../../SECURITY.md)
- [ ] No stealth, persistence, or evasion mechanisms added
- [ ] No unauthorized surveillance features
- [ ] Memory safety preserved (no data leaks)
- [ ] Secure defaults maintained
- [ ] Changes are transparent and auditable

## Documentation
- [ ] README updated (if user-facing)
- [ ] CHANGELOG updated with entry in appropriate section
- [ ] Code comments added for complex logic
- [ ] Architecture doc updated (if applicable)

## Performance Impact
- [ ] No performance regressions
- [ ] Benchmarks run (if applicable)
- [ ] Memory footprint unchanged or improved

## Breaking Changes
If this is a breaking change, describe migration path for users:

---

## Reviewer Checklist
- [ ] Code is readable and well-commented
- [ ] Changes are well-tested
- [ ] No security concerns
- [ ] Documentation is complete
- [ ] Follows project conventions
- [ ] Ready to merge

---

**Note**: PRs that violate the security policy or lack proper testing will not be merged.
