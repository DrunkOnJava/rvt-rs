# Dependabot triage — 2026-08-29 (Lane Eleven)

Reviewed all 15 open Dependabot PRs. Ordering: viewer npm → Rust crates → GitHub Actions.

Authenticated write path: GitHub MCP (repo owner). `gh` CLI is read-only in this environment.

## Summary table

| PR | Title | Risk | Action | Reason |
|----|-------|------|--------|--------|
| #111 | vite 8.0.9 → 8.1.5 | Low (patch within 8.x); **security** | Merge (after rebase if needed) | Fixes `npm audit` highs on vite 8.0.0–8.0.15 (GHSA-v6wh-96g9-6wx3, GHSA-fx2h-pf6j-xcff). Conflict with main lockfile expected. |
| #109 | typescript 5.9.3 → 7.0.2 | **High** (major) | Leave open | TS 5→7 is a dedicated migration; Dependabot-only bump is unsafe. |
| #108 | three + @types/three 0.169 → 0.185 | **High** | Leave open | Large three.js jump; viewer 3D API risk; conflicts with main. |
| #104 | @playwright/test 1.59.1 → 1.61.1 | Low (minor) | Merge | Dev-dep only; CI reds are unrelated (stale vite audit / cargo audit). |
| #102 | pyo3 0.24 → 0.29 (group) | N/A | Close | Superseded: `rvt-py` already on pyo3 0.29 on main. |
| #76 | pyo3 0.24 → 0.28.3 | N/A | Close | Superseded by main (0.29) and by #102. |
| #74 | thiserror 1 → 2.0.18 | Medium–High (major) | Leave open | Macro usage looks compatible, but major + needs fresh CI after rebase. |
| #72 | quick-xml 0.36 → 0.39.2 | N/A (regressive) | Close | Main already on quick-xml 0.41; merging would downgrade. CI red. |
| #71 | criterion 0.5 → 0.7 | Medium (major, benches) | Leave open | Conflicts; benches need verify against 0.7 API. |
| #70 | cfb 0.11 → 0.14 | **High** | Leave open | Core CFB I/O; multi-minor jump; conflicts; dedicated migration. |
| #69 | actions/setup-node 4 → 6 | Low–Med (action major) | Merge | CI green on PR; Node setup only. |
| #6 | actions/upload-artifact 4 → 7 | Med (action major) | Merge | CI green; pair with #2 for publish workflow. |
| #4 | actions/setup-python 5 → 6 | Low–Med | Merge | CI green. |
| #3 | Swatinem/rust-cache pin bump | Low | Merge | Same major (v2); SHA pin refresh; CI green. |
| #2 | actions/download-artifact 4 → 8 | Med | Merge | CI green; pair with #6. |

## Notes

- Failed `cargo audit` / `cargo deny` on older Dependabot branches are largely **stale** (e.g. `anyhow` advisory) and do not block merges of npm/GHA-only PRs when the changed surface is unrelated and main CI is green.
- Failed `viewer dependency audit` on older npm PRs is driven by **vite 8.0.x** advisories — landing #111 first is the fix.
- Do **not** force-push Dependabot branches; use GitHub “Update branch” / recreate via Dependabot when conflicted.
