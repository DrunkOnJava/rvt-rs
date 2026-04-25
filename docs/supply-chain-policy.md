# Supply-Chain Policy

This repository treats parser dependencies and release automation as part of
the trusted computing base. Revit files are untrusted input, so dependency
review has to stay enforceable in CI.

## Rust Dependencies

Required checks:

- `cargo deny check` enforces `deny.toml`:
  - permissive license allowlist
  - yanked crate deny
  - crates.io-only sources
  - wildcard dependency deny
  - advisory policy
- `cargo audit` fails on RustSec advisories against the current dependency set.

`deny.toml` may ignore an advisory only when the ignore entry has:

- a linked GitHub issue
- a short rationale
- an expiry date or explicit re-review date

There are currently no ignored RustSec advisories.

## Viewer JavaScript Dependencies

The viewer has a separate dependency tree under `viewer/package-lock.json`.
CI runs:

```bash
cd viewer
npm ci
npm audit --audit-level=high
```

High and critical npm advisories fail CI. If npm reports a lower-severity
advisory that still affects the zero-upload/privacy posture, treat it as
release-blocking even if `npm audit --audit-level=high` does not fail.

Dependabot checks `/viewer` weekly for npm updates.

## GitHub Actions

Workflow actions must be either:

- pinned to a full commit SHA with a comment naming the human-readable tag, or
- documented as an exception in the table below.

Pinned action examples:

- `actions/checkout@34e114876b0b11c390a56381ad16ebd13914f8d5 # v4`
- `Swatinem/rust-cache@42dc69e1aa15d09112580998cf2ef0119e2e91ae # v2`
- `rustsec/audit-check@69366f33c96575abad1ee0dba8212993eecbe998 # v2.0.0`

Allowed exceptions:

| Action pattern | Rationale | Review trigger |
|---|---|---|
| `dtolnay/rust-toolchain@stable` / `@nightly` / `@master` | This action is a toolchain selector; the value is the requested Rust channel, not a release tag for project logic. | Re-review if replacing the action or pinning a fixed Rust toolchain. |
| `actions/checkout@v4` for external corpus checkout | Used where checkout action features are stable and the source repository is explicitly named in the step. Prefer the pinned SHA for repo checkout steps. | Re-review before adding a new external checkout. |
| `actions/setup-python@v5` | GitHub-owned setup action used to provision a requested Python version in CI/release workflows. | Re-review on major-version bump. |
| `PyO3/maturin-action@v1` | Release/build action for Python wheels; major tag tracks the supported PyO3/maturin interface. | Re-review on major-version bump or publish workflow change. |
| `actions/upload-artifact@v4` / `actions/download-artifact@v4` | GitHub-owned artifact plumbing in CI/release workflows. | Re-review on major-version bump. |
| `pypa/gh-action-pypi-publish@release/v1` | PyPI Trusted Publisher action; upstream documents this release branch for OIDC publishing. | Re-review before changing PyPI publishing mode. |

New third-party actions should not use an exception by default. Pin them to a
SHA and include the source tag in a comment.

## Dependency Update Flow

- Dependabot opens weekly Cargo, GitHub Actions, and viewer npm update PRs.
- Dependency PRs must pass CI before merge.
- Security advisory PRs should mention the advisory ID and affected package.
- If an advisory cannot be fixed immediately, open or link a tracking issue and
  document the temporary ignore with rationale and expiry before merging.
