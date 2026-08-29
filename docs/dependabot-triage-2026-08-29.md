# Dependabot triage — 2026-08-29 (Lane Eleven)

Reviewed all 15 open Dependabot PRs. Ordering: viewer npm → Rust crates → GitHub Actions.

Write path: GitHub MCP as repo owner. `gh` CLI is read-only in the Cloud Agent environment.

## Outcomes

| PR | Title | Risk | Action | Reason |
|----|-------|------|--------|--------|
| #111 | vite 8.0.9 → 8.1.5 | Low; **security** | **Closed** (superseded) | Dependabot branch conflicted. Main lockfile already resolved vite **8.2.2** (newer than 8.1.5); package.json range raised to `^8.2.2` on this triage branch. `npm audit` clean. |
| #109 | typescript 5.9.3 → 7.0.2 | **High** (major) | Left open | TS 5→7 needs a dedicated migration; do not land as Dependabot-only. |
| #108 | three + @types/three 0.169 → 0.185 | **High** | **Closed** (superseded by #135) | Dedicated migration: `three`/`@types/three` → 0.185.1 + `three/addons` imports; typecheck + production build green. |
| #104 | @playwright/test 1.59.1 → 1.61.1 | Low | **Merged** | Dev-dep minor; squash-merged. |
| #102 | pyo3 0.24 → 0.29 (group) | N/A | **Closed** | Superseded: main already on pyo3 0.29. |
| #76 | pyo3 0.24 → 0.28.3 | N/A | **Closed** | Superseded by main (0.29). |
| #74 | thiserror 1 → 2.0.18 | Medium–High | **Closed** (superseded by #132) | Dedicated migration to thiserror 2.0.20; 823 lib tests + clippy `-D warnings` green. |
| #72 | quick-xml 0.36 → 0.39.2 | N/A | **Closed** | Main already on 0.41; merge would downgrade. |
| #71 | criterion 0.5 → 0.7 | Medium | **Closed** (superseded by #133) | Dedicated migration; `cargo bench --no-run` green. |
| #70 | cfb 0.11 → 0.14 | **High** | Left open | Core CFB I/O; multi-minor jump; conflicts. Needs patch-corpus + roundtrip evidence. |
| #69 | actions/setup-node 4 → 6 | Low–Med | **Merged** | CI green; squash-merged. |
| #6 | actions/upload-artifact 4 → 7 | Med | **Merged** | CI green; squash-merged (paired with #2). |
| #4 | actions/setup-python 5 → 6 | Low–Med | **Merged** | CI green; squash-merged. |
| #3 | Swatinem/rust-cache pin | Low | **Merged** | v2 SHA pin refresh; squash-merged. |
| #2 | actions/download-artifact 4 → 8 | Med | **Merged** | CI green; squash-merged (paired with #6). |

## Counts (Lane Eleven initial)

- Merged: 6 (#104, #69, #6, #4, #3, #2)
- Closed: 4 (#111 superseded, #102, #76, #72)
- Left open: 5 (#109, #108, #74, #71, #70)

## Follow-up (same day) — dedicated migrations

| Action | Result |
|--------|--------|
| #132 `thiserror` 1 → 2 | **Merged**; closed Dependabot #74 |
| #133 `criterion` 0.5 → 0.7 | **Merged**; closed Dependabot #71 |
| #135 `three` 0.169 → 0.185 | **Merged**; closed Dependabot #108 |
| #134 viewer a11y / responsive (M6-05) | **Merged**; closed #46 |
| #98 OPS-02 parent-dir `CLAUDE.md` | **Closed** not_planned (file not in-repo; `AGENTS.md` is the in-tree brief) |

### Still open Dependabot majors

1. **#109 TypeScript 7** — dedicated migration still required.
2. **#70 cfb 0.14** — dedicated CFB I/O migration with corpus/patch coverage still required.

### Remaining follow-ups

1. ~~Raise viewer `vite` package.json range to `^8.2.2`~~ — done in Lane Eleven.
2. Schedule remaining dedicated migrations: TypeScript 7, cfb 0.14.
3. Optionally `@dependabot recreate` on #109 / #70 after those migrations land (or ignore major if we intentionally stay).
