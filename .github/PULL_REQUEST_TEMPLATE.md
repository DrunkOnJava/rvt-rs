<!--
  rvt-rs pull request template. Every section below is expected to be
  filled in. A PR that self-certifies against this checklist lets
  reviewers focus on substance instead of mechanics. Delete sections
  that genuinely do not apply, and say so ("N/A: docs-only PR").
-->

## Summary

<!-- 1-3 sentences describing the change. -->

## Motivation

<!-- Why this change? Link to the issue if applicable. -->

## Changes

<!-- Bulleted list of the files / subsystems touched. -->

-
-

## Testing

- [ ] `cargo test --all` passes locally
- [ ] `cargo clippy --all-targets --all-features -- -D warnings` passes
- [ ] `cargo fmt --check` passes
- [ ] New / changed code has unit tests
- [ ] If this touches the IFC exporter: `tests/fixtures/synthetic-project.ifc` still produces byte-identical output (or the diff is documented in the PR body with rationale)
- [ ] If this adds an element decoder: `empty_tolerance`, `wrong_schema_rejected`, and `from_decoded_happy_path` tests are present
- [ ] If this is a performance change: reran `tools/bench.sh` and attached the before / after

## Audit-honesty checklist

<!--
  rvt-rs is built on the principle that we state what works AND what
  does not. Every PR is expected to meet this bar.
-->

- [ ] README / docs reflect the current state after this PR — no claims of features that are not actually shipped in this PR
- [ ] If a limitation was resolved, the "Known limitations" / "What works today" section in the relevant docs is updated
- [ ] If a new limitation was introduced, it is documented in the PR body and in-code
- [ ] Commit messages describe WHAT changed and WHY, not "fix things" or "updates"
- [ ] No emojis in code, docs, or commit messages
- [ ] No private paths, personal email addresses, or internal jargon in committed files
- [ ] If this is a reverse-engineering finding: a reproducible probe under `examples/` and a dated addendum to `docs/rvt-moat-break-reconnaissance.md` are included

## Legal and contribution hygiene

- [ ] I agree this work is licensable under Apache-2.0 (see `CONTRIBUTING.md`)
- [ ] This PR contains no Autodesk proprietary source, decompiled internals, or NDA'd SDK content (see `CLEANROOM.md`)
- [ ] Any new test fixtures use synthetic values (`testuser`, `111111`, `FY-20XX`, etc.) and contain no real-world PII

## Breaking changes

<!-- "None" or list them with the migration path. -->

## Screenshots or output samples

<!-- Required if UI or IFC output changed. Paste or attach. -->

## Related issues

<!-- "Closes #N" / "Refs #M" / "None". -->

---

See [`CONTRIBUTING.md`](../CONTRIBUTING.md) for deeper guidance on coding conventions, reverse-engineering findings, and the clean-room posture.
