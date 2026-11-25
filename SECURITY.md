# Security Policy

## Supported versions

| Version | Supported |
|---|---|
| 0.1.x (alpha) | Best effort until 1.0 |
| 1.0+ | Per the LTS policy (every 2nd minor, 12 months) |

## Reporting a vulnerability

**Do not open a public issue.**

Report privately via GitHub's
[private security advisory](https://docs.github.com/en/code-security/security-advisories/guidance-on-reporting-and-writing/privately-reporting-a-security-vulnerability)
form, or email the maintainers at `security@ferrum.rs`.

Please include:

- affected version and commit;
- a minimal reproduction;
- impact assessment;
- any suggested fix.

## Response targets

- Acknowledge within **72 hours**.
- Fix or mitigation within **30 days**.
- Credit the reporter in the advisory unless they prefer otherwise.

## Scope

In scope:

- XSS via the sanitizer, SSR, or hydration.
- CSP bypasses.
- Code execution via `eval`/`new Function` paths.
- Panics/DoS in the reconciler or parser from untrusted input.

Out of scope:

- Vulnerabilities in third-party crates (report upstream).
- Issues that require a compromised host or malicious browser extension.
- The alpha's documented gaps (no native window, no audited fuzzing yet).