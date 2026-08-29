# Dependabot triage — 2026-08-29 (Lane Eleven)

Reviewed all 15 open Dependabot PRs. Ordering: viewer npm → Rust crates → GitHub Actions.

Write path: GitHub MCP as repo owner. `gh` CLI is read-only in the Cloud Agent environment.

## Outcomes

| PR | Title | Risk | Action | Reason |
|----|-------|------|--------|--------|
| #111 | vite 8.0.9 → 8.1.5 | Low; **security** | **Closed** (superseded) | Dependabot branch conflicted. Main lockfile already resolved vite **8.2.2** (newer than 8.1.5); package.json range raised to `^8.2.2` on this triage branch. `npm audit` clean. |
| #109 | typescript 5.9.3 → 7.0.2 | **High** (major) | Left open | TS 5→7 needs a dedicated migration; do not land as Dependabot-only. |
| #108 | three + @types/three 0.169 → 0.185 | **High** | Left open | Large three.js jump; viewer 3D API risk; branch conflicts. |
| #104 | @playwright/test 1.59.1 → 1.61.1 | Low | **Merged** | Dev-dep minor; squash-merged. |
| #102 | pyo3 0.24 → 0.29 (group) | N/A | **Closed** | Superseded: main already on pyo3 0.29. |
| #76 | pyo3 0.24 → 0.28.3 | N/A | **Closed** | Superseded by main (0.29). |
| #74 | thiserror 1 → 2.0.18 | Medium–High | Left open | Major; macro surface looks compatible but needs fresh CI after rebase. |
| #72 | quick-xml 0.36 → 0.39.2 | N/A | **Closed** | Main already on 0.41; merge would downgrade. |
| #71 | criterion 0.5 → 0.7 | Medium | Left open | Conflicts; verify benches against 0.7. |
| #70 | cfb 0.11 → 0.14 | **High** | Left open | Core CFB I/O; multi-minor jump; conflicts. |
| #69 | actions/setup-node 4 → 6 | Low–Med | **Merged** | CI green; squash-merged. |
| #6 | actions/upload-artifact 4 → 7 | Med | **Merged** | CI green; squash-merged (paired with #2). |
| #4 | actions/setup-python 5 → 6 | Low–Med | **Merged** | CI green; squash-merged. |
| #3 | Swatinem/rust-cache pin | Low | **Merged** | v2 SHA pin refresh; squash-merged. |
| #2 | actions/download-artifact 4 → 8 | Med | **Merged** | CI green; squash-merged (paired with #6). |

## Counts

- Merged: 6 (#104, #69, #6, #4, #3, #2)
- Closed: 4 (#111 superseded, #102, #76, #72)
- Left open: 5 (#109, #108, #74, #71, #70)

## Follow-ups

1. Raise viewer `vite` package.json range to `^8.2.2` (this PR) so the declared range matches the secure lockfile resolution.
2. Schedule dedicated migrations for: TypeScript 7, three.js 0.185, cfb 0.14, thiserror 2, criterion 0.7.
3. Optionally `@dependabot recreate` on left-open majors after migrations land.
