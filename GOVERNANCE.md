# Governance

Ferrum is a **0.1 alpha**. Governance is intentionally light until 1.0, but
the rules below are already in force.

## Maintainers

- **BDFL model during alpha:** the founding maintainer has final say on
  architecture and releases.
- Before 1.0: grow to 2–3 additional maintainers, each with ≥ 10 substantial
  merged PRs.
- Maintainer decisions are recorded in the RFC/issue thread.

## RFC process

Public API changes and large features require an RFC issue:

1. Problem, motivation, detailed design, drawbacks, alternatives.
2. 14-day comment period (7 days for small changes).
3. Maintainer decision: accepted / rejected / needs-revision.
4. Accepted RFCs are tracked in the issue and summarized in the release notes.

Bug fixes, docs, tests, and internal refactors do not need an RFC.

## Versioning

- **0.1.x:** breaking changes allowed in minor releases; every break gets a
  `CHANGELOG.md` entry and a migration note.
- **1.0+:** strict semver. Breaking changes only in major versions, with a
  codemod/migration guide.
- The `ferrum` facade is the only stability boundary.

## LTS & support

- LTS every 2nd minor after 1.0 (1.2, 1.4, …), 12 months.
- Support window: latest minor + previous minor.
- MSRV: stable Rust, bumped at most once per minor with 6-month notice.

## Security

See [SECURITY.md](SECURITY.md). Private reporting, 72 h acknowledgement,
30-day fix/mitigation target.

## Releases

- Every release runs `scripts/alpha-check.sh` (fmt, tests, conformance,
  wasm size, benches).
- Artifacts: wasm demo, SHA-256 checksums, CycloneDX SBOM.
- Signing is a maintainer key action (pending).

## Code of conduct

See [CODE_OF_CONDUCT.md](CODE_OF_CONDUCT.md).