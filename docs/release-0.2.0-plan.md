# Release plan: 0.2.0 (inspection-focused alpha)

Status: draft (audit A4 / RELEASE-001)  
Baseline tag today: `v0.1.2` (PyPI `rvt==0.1.2` published; **crates.io
crate `rvt` not published** as of 2026-08-29 — docs.rs therefore 404)

## Positioning

**0.2.0 is an inspection-focused alpha with experimental export.**

It is:

- A trustworthy way to open Revit containers, decode streams, read
  metadata / schema, inspect coverage, and triage files without Revit.
- Honest about narrow typed recovers (ArcWall / partition MVP Level /
  Material / Room / Floor plan-loops / 2024 opening-index ids) and
  about negatives (RE-19 Door/Window/Wall; RE-20 Level ElementIds).
- Willing to emit IFC / glTF / SVG **only** as experimental output gated
  by export modes + confidence/provenance — never marketed as
  converter-grade.

It is **not**:

- A general Revit→IFC converter for arbitrary architectural projects.
- A promise that Door / Window / schema-field Wall / full geometry work.
- A claim that crates.io / docs.rs are live until `cargo publish`
  actually succeeds for this crate name.

Canonical user-facing wording lives in [`status.md`](status.md),
[`supported-profile.md`](supported-profile.md), and the README.

## In-scope for the 0.2.0 cut

1. **Docs honesty gate** — README / status / changelog / install align;
   no working-docs.rs claims while the crate is unpublished; PyPI 0.1.2
   called out accurately ([`install.md`](install.md)).
2. **Quality / supply-chain** — `tools/quality.sh` (or equivalent)
   green; cargo-deny / cargo-audit expectations documented; publish
   workflow actions SHA-pinned where practical.
3. **Inspect surface** — `rvt-inspect` + export diagnostics JSON +
   viewer File Status confidence remain the primary UX.
4. **Changelog** — promote the rewritten `[Unreleased]` block into
   `[0.2.0]` at tag time (Keep a Changelog).
5. **Publication plan** — tag `v0.2.0` → `publish.yml`:
   - Prefer publishing **crates.io first**, then PyPI, so docs.rs can
     build from the published crate.
   - If crates.io rejects the `rvt` name or token is missing, do **not**
     claim docs.rs; keep install docs on source + PyPI only and file a
     follow-up issue.

## Explicitly out of scope for 0.2.0

- Converter-grade typed Wall / Door / Window recovery.
- Floor/Room storey binding that depends on Level ElementIds (RE-20).
- Semantic write API (ADR-002) and desktop wrappers (ADR-004).
- Claiming parity with Autodesk `revit-ifc` or commercial converters.

Those belong in later milestones (`0.3.0+` per [`ROADMAP.md`](../ROADMAP.md)).

## Exit criteria (release checklist)

Use [`release-checklist.md`](release-checklist.md) with these 0.2.0
additions:

| Check | Pass condition |
|---|---|
| Positioning | README + status say “inspection alpha / experimental export” |
| PyPI | `pip install rvt==0.2.0` imports; inspect smoke on a sample |
| crates.io | Either `cargo install rvt --version 0.2.0` works **or** release notes explicitly say “Rust crate not on crates.io yet” |
| docs.rs | Linked only if the crate published and the page is HTTP 200 |
| Viewer | Pages build loads; File Status shows confidence / readiness |
| Regression | Finding 1 gated strip + Formats/Latest ungated defaults still hold |

## Changelog prep note

Until the tag lands, keep accumulating under `[Unreleased]` in
[`CHANGELOG.md`](../CHANGELOG.md). At cut time, move that section to
`## [0.2.0] — YYYY-MM-DD` and leave a short empty Unreleased stub.
