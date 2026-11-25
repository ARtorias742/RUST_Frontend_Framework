# Support

Ferrum is a **0.1.0-alpha**. There is no production support promise yet.

## Where to ask

- **Questions / ideas:** GitHub Discussions.
- **Bugs:** GitHub Issues (use the bug template).
- **Security:** private advisory — see [SECURITY.md](SECURITY.md).
- **Chat:** link will be added to the README when the community space exists.

## What to expect

- Alpha: best-effort responses; no SLA.
- Maintainer bandwidth is limited; a minimal reproduction is the fastest path
  to a fix.
- The API may change between 0.1.x releases; check `CHANGELOG.md` before
  upgrading.

## Before filing a bug

1. Confirm you are on the latest `main` (the alpha moves quickly).
2. Reduce to a minimal component/`rsx!`/command.
3. Include `rustc --version`, target (native/wasm), browser + version, and
   the full error.
4. For browser-only issues, include the console output and whether the
   native/headless test reproduces it.

## Commercial use

Alpha is Apache-2.0 and free to use, but it is **not production-ready**.
If you need guarantees, wait for 1.0 or contact the maintainers about
sponsorship/support.