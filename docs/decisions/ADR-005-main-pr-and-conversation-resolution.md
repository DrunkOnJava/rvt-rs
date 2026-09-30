# ADR-005: Require PRs + resolved conversations on `main` (no external approval deadlock)

## Status

Accepted and applied on 2026-09-30. It was proposed earlier, when the API
update returned HTTP 403 for the automation token; the maintainer's token
applied it, and the live ruleset matches
[`.github/rulesets/main-protection.json`](../../.github/rulesets/main-protection.json),
which also limits merges to squash.

## Context

Audit of ruleset `main-protection` (id `15270879`) found signatures, linear
history, and strict status checks, but **no** pull-request requirement and
**no** conversation-resolution gate. That allowed PR #115 to merge while
Bugbot findings were unresolved on a deleted correction branch.

This repository is currently single-maintainer. Requiring
`required_approving_review_count >= 1` from another human would deadlock
routine work.

## Decision

Update `main-protection` to:

1. **Require a pull request** before landing on `main`.
2. **Require review thread resolution** (`required_review_thread_resolution`).
3. Keep **`required_approving_review_count: 0`** so a solo maintainer is not
   blocked waiting for an external approval.
4. Keep admin **bypass** for emergencies (`RepositoryRole` admin, `always`).
5. Keep existing signature / linear-history / status-check rules.

## Apply payload

See [`.github/rulesets/main-protection.json`](../../.github/rulesets/main-protection.json).

```bash
gh api --method PUT repos/DrunkOnJava/rvt-rs/rulesets/15270879 \
  --input .github/rulesets/main-protection.json
```

## Consequences

- Direct pushes to `main` are blocked for non-bypass actors.
- Unresolved review conversations (including Bot findings treated as review
  threads) block merge until dismissed or addressed.
- Solo maintainer can still merge their own PRs without a second human
  approval; admin bypass remains for break-glass.
