# Unified Research, Architecture, and Implementation Report

| Field | Value |
|-------|--------|
| Revision | **1.1** |
| Session date | **2026-08-29** |
| Frozen audit baseline | `86a43b9` (`docs(#151): Wave 2 evidence matrix + writer audit`) |
| Main at ingest | **advanced past** that baseline (audit P0 #165–#169, Actions SHA pins #170, Finding 1 narrow strip, etc.) |
| Credit | Research posture and Discussion [#112](https://github.com/DrunkOnJava/rvt-rs/discussions/112) findings credited to [@STE1200](https://github.com/STE1200) and team; repository-side reproduction and product gates remain maintainer-owned |
| Honesty | This document does **not** claim ES ElementId remapping, Door/Window discriminators, schema-field Wall, Level ElementIds, AProperty joins, or converter-grade IFC are solved |

> **Note on completeness.** The full session paste (rev 1.1) is preserved here as a governing structured mirror. Where the live paste exceeded practical verbatim reconstruction, every required control surface is captured: evidence labels, governing decisions (§3), executive assessment, ES oracle priority (§15 + §30), lanes A–F (§19), phases 0–8 (§29), release/capability gates (§23), non-goals (§28), risks (§27), and the immediate action list. Discussion #112 comments often 403 for automation — this doc mirror is the durable record.

---

## 1. Executive assessment

**Primary research target:** ES (`ElementSettings` / Extensible Storage–adjacent) **ElementId remapping** under hypothesis **H-ES5**, driven by **owned synthetic fixtures** and a **Revit API oracle**. Production Autodesk corpora are **regression only**, never discovery oracles.

**Why this ordering:** Typed BIM recovery (Door/Window, schema-field Wall, Level ElementIds, AProperty host joins) hit documented negatives (RE-19 / RE-20). Continuing to invent those paths erodes trust. ES remapping is a separable, instrumentable research surface: mutate one semantic edge → observe before/after bytes → localize references → promote typed edges only after evidence gates.

**Pipeline (fail-closed):**

```text
owned fixture → API truth → one semantic mutation → before/after files
  → normalize → localize → typed edge → validate → optional IFC
```

**Product readiness (Phase 0 / Lane E)** proceeds in parallel and is largely landed on `main` after `86a43b9`. **Phase 1** (this continuum) ships foundational contracts without Revit. **Phase 2** (oracle fixture generation N1–N4, R1/R2, C1/C2, …) is **blocked on a Revit-hosted oracle** — not available on the Cloud VM.

**Target architecture (documented only):** a future multi-crate workspace (`rvt-core`, `rvt-es`, `rvt-oracle`, …) remains a **target**, not an immediate split. Phase 1 types live in the existing root crate layout.

---

## 2. Evidence labels

| Label | Meaning |
|-------|---------|
| **E0** | Speculative / untested hypothesis |
| **E1** | Single-environment observation (one file / one release) |
| **E2** | Multi-file or multi-release observation; still no independent oracle |
| **E3** | Independently reproduced on redistributable / owned fixtures |
| **E4** | Oracle-backed (Revit API or equivalent) + automated regression |
| **E5** | Promoted capability with release gate + support-matrix row |

| Hypothesis / workstream | Current ceiling | Notes |
|-------------------------|-----------------|-------|
| H-ES5 ES ElementId remapping | **E0–E1** (research) | No production claim; Phase 2 requires oracle |
| Finding 1 checksum-paged inflate (#151) | **E4** (narrow gate) | Formats/Latest strip excluded |
| ElemTable ownership tree (#152) | **E0–E1** reported | Outside ES parent scoring |
| ElementHeader framing (#153) | **E0–E1** reported | Evidence-only |
| Door/Window discriminator (RE-19) | **Negative** | Do not invent |
| Level ElementId map (RE-20) | **Negative** | Do not invent |
| Audit P0 SEC/PARSE/support/changelog | **Shipped** | #165–#169 |

---

## 3. Governing decisions

1. **Typed edges, not universal parent.** Relationships are explicit typed edges with provenance; there is no single “parent pointer” that collapses BIM topology, ES graphs, and ElemTable ownership.
2. **ES value trees + ES ref graph are separate from BIM topology.** Do not merge ES reference graphs into wall/floor/host topology without an evidence gate.
3. **Default IFC omits ES.** Extensible-storage / ES-derived edges are not emitted on the default IFC path; optional diagnostic modes may surface them later under explicit flags.
4. **No generic host→`IfcRelConnects`.** Host/opening/connect relations require typed, validated evidence — never a catch-all connector.
5. **One mutation per fixture transition.** Each before→after pair encodes exactly one semantic change so localization stays attributable.
6. **Unsupported versions fail closed.** Unknown or ungated Revit years refuse decode/export claims rather than guessing layouts.
7. **Discovery = owned synthetics; production = regression only.** Magnetar / Autodesk samples validate non-regression; they do not define ES remap semantics.
8. **ES refs stay outside #152 parent scoring.** ElemTable ownership-tree work and ES remapping do not cross-credit each other.
9. **Hard walls unchanged:** no invented Door/Window, schema-field Wall, Level ElementIds, AProperty joins, compound-opening decoder names, or converter-grade IFC. Formats strip stays disabled for Formats/Latest.
10. **Honesty over velocity.** Capabilities promote only after evidence gates (§23); docs and support-matrix stay fail-closed.

---

## 4–14. Context summary (condensed)

- Clean-room Apache-2 toolkit: Rust core + CLIs, Python (`maturin`/`pyo3`), WASM viewer.
- Strong layers 1–4c; partial ElemTable body, typed MVP, geometry, IFC.
- Discussion #112 findings #151–#156: Finding 1 largely closed on narrow gate; #152–#156 open evidence-only.
- RE-19 / RE-20 negatives bound product claims on Door/Window/Wall/Level ElementId.
- Audit P0 after frozen commit `86a43b9` raised product honesty (security, Formats diagnostics, support matrix, changelog/0.2.0 plan, CI pins).

---

## 15. Immediate ES oracle priority (H-ES5)

### 15.1 Goal

Establish whether ES-held ElementId (and related) references remapped under controlled mutations, and if so, **localize** every remapped occurrence with byte-accurate `SourceSpan` + path segments, then promote **typed edges** only after oracle agreement.

### 15.2 Why first

Instrumentable with owned fixtures; orthogonal to blocked BIM discriminators; feeds future IFC/diagnostic honesty without claiming converter-grade export.

### 15.3 Method sketch

1. Build minimal owned project fixture (Phase A / `S_All` seed).
2. Capture Revit API truth (element ids, unique ids, ES entity/field values).
3. Apply **one** semantic mutation (id remap / copy / delete / null).
4. Save before/after `.rvt` pairs under fixture law.
5. Normalize streams (identity-preserving; no Formats strip).
6. Diff + localize candidate reference occurrences.
7. Classify typed edges; validate against API truth.
8. Optional: diagnostic IFC annotation (never default path).

### 15.4–15.15 Fixture families (Phase 2 — Revit required)

| Family | Intent | Cloud status |
|--------|--------|--------------|
| **N1–N4** | Null / no-op / identity baselines | Blocked on oracle |
| **R1/R2** | Remap single / multi ElementId refs | Blocked on oracle |
| **C1/C2** | Copy / duplicate semantics | Blocked on oracle |
| **C3a/C4a** | Constrained compound cases | Blocked on oracle |
| **ES-remap-00** | Manifest sketch + contracts only | Scaffolded in-repo |

### 15.16 Capability schema (promotion stub)

Capabilities under this program use a versioned JSON schema (`research/es-remap/capability.schema.json` and `docs/schemas/es-capability.schema.json`) with at least:

- `schema_version`
- `capability_id` (e.g. `es.elementid_remap`)
- `status`: `unsupported` | `research` | `partial` | `verified`
- `evidence_tier` (E0–E5)
- `fixture_ids[]`
- `oracle`: `{ "required": true, "kind": "revit_api" }`
- `claims[]` / `non_claims[]`
- `support_matrix_row` (optional link)

**Promotion rule:** status may move to `verified` only with E4+ evidence and a support-matrix update in the same change set. Until then, status stays `research` or `unsupported`.

---

## 16–18. Architecture notes (target vs now)

**Now (Phase 1):** single crate `rvt` gains public contract types: `DocumentIdentity`, `ScopedElementRef`, `SourceSpan`, `EvidenceTier`, ledger stubs, `EsReferenceOccurrence` (+ path segments), transition types. No ES byte decoder invents layouts.

**Target (later):** multi-crate workspace separating core parse, ES graph, oracle harness, and export — documented only; do not explode the workspace in this continuum.

---

## 19. Lanes A–F

| Lane | Name | Role |
|------|------|------|
| **A** | ES remap oracle | Primary research; owned fixtures + Revit API |
| **B** | ElemTable / headers (#152/#153) | Evidence-only; separate from ES scoring |
| **C** | Typed BIM recovery | Bound by RE-19/RE-20; no invented successes |
| **D** | Geometry / openings | Fail-closed; no compound-opening decoder names without evidence |
| **E** | Product readiness | Security, diagnostics, support matrix, changelog, CI pins |
| **F** | Export honesty | IFC modes, diagnostics, omit ES by default |

Lanes A and E are the near-term parallel tracks. Lane A Phase 2 is Revit-blocked; Lane E Phase 0 is mostly done on `main`.

---

## 20–22. Coordination

- Discussion #112: automation often cannot comment (403). Prefer `docs/disc-112-coordination.md` + this report.
- Sibling Cloud agents may land P1 chores (Actions pins, `source_coverage`); rebase and skip duplicates.
- Hard walls apply to all lanes.

---

## 23. Release / capability gates

| Gate | Requirement |
|------|-------------|
| **G-docs** | No user-facing claim without matching support-matrix / schema status |
| **G-E3** | Multi-fixture reproduction before “partial” |
| **G-E4** | Oracle agreement + automated regression before “verified” |
| **G-IFC** | Default IFC must not emit ES edges |
| **G-Formats** | Formats/Latest production strip remains disabled |
| **G-ES-#152** | ES refs do not count toward ElemTable ownership parent scoring |
| **G-version** | Unsupported Revit years fail closed |
| **G-mutation** | One semantic mutation per fixture transition |

---

## 24–26. Observability & schemas

Phase 1 freezes:

- Observation schema for scalar/copy/remap localization attempts (`observation.schema.json`)
- Capability schema stub (§15.16)
- Fixture manifest law (`manifest.yaml`)

Observations are research artifacts; they are not production decode success proofs.

---

## 27. Risks

| Risk | Mitigation |
|------|------------|
| Inventing ES byte layouts on Cloud without Revit | Scaffold contracts only; block Phase 2 checklist |
| Cross-contaminating ES with #152 ownership claims | Explicit non-scoring rule; separate evidence ledgers |
| Over-claiming IFC from ES edges | Default omit; capability gates |
| Fixture law drift across agents | Versioned `manifest.yaml` + one-mutation rule |
| Production corpus used as discovery oracle | Documented: regression only |
| Parallel agents fighting same files | Fetch/rebase; skip merged pins/coverage work |
| Honesty erosion under schedule pressure | Fail-closed statuses; RE-19/RE-20 walls |

---

## 28. Explicit non-goals (this continuum)

- Inventing Door/Window discriminators or typed Door/Window success
- Inventing schema-field Wall or 2024 ArcWall envelope claims beyond evidence
- Inventing Level ElementId recovery / Floor–Room storey binds
- Inventing AProperty\* host joins
- Naming/shipping compound opening decoders without oracle evidence
- Converter-grade IFC for arbitrary projects
- Enabling Formats/Latest checksum-page strip in production
- Splitting the crate into the full multi-crate workspace
- Claiming ES ElementId remapping works
- Using production corpora as discovery oracles for ES remap

---

## 29. Phases 0–8

| Phase | Focus | Status (2026-08-29 ingest) |
|-------|--------|----------------------------|
| **0** | Product readiness (Lane E): security, Formats integrity diagnostics, support matrix, changelog/0.2.0 honesty, CI/deploy pins | **Mostly done** on `main` (#165–#170; residual chores may remain with sibling agents) |
| **1** | Foundational contracts: DocumentIdentity, ScopedElementRef, SourceSpan, EvidenceTier, ES occurrence types, observation/capability schemas, `research/es-remap` scaffold | **Next shippable without Revit** |
| **2** | ES-remap-00 + N1–N4, R1/R2, C1/C2, C3a/C4a fixture generation via Revit API oracle | **Blocked on Revit-hosted oracle** |
| **3** | Localization automation + typed edge promotion under G-E3/G-E4 | After Phase 2 |
| **4** | Regression harness on production corpora (non-discovery) | After Phase 3 |
| **5** | Optional diagnostic IFC surfacing (non-default) | Gated |
| **6** | ElemTable/#152/#153 evidence lanes (parallel, non-blocking) | Evidence-only |
| **7** | BIM typed recovery only where new evidence overturns RE-19/RE-20 | Fail-closed |
| **8** | Multi-crate workspace split + release channel publish decisions | Human/process gated |

---

## 30. Immediate action list

1. Persist this governing report under `docs/research/` + Cloud artifacts.
2. Align TODO / disc-112 / status pointers — Phase 0 done → Phase 1 contracts → Phase 2 ES oracle (Revit-blocked).
3. **Add `DocumentIdentity` / `ScopedElementRef`** (document-scoped ElementId + UniqueId).
4. **Add `SourceSpan` + record identity hooks** as practical.
5. **Named `EvidenceTier` + lightweight evidence/edge ledger types.**
6. **`EsReferenceOccurrence` + path segment types** (public API; decoder stubs OK).
7. **Transition / no-op baseline contract types** for fixture pairs.
8. **Phase A `S_All` fixture manifest** sketch under `research/es-remap/`.
9. **Freeze scalar/copy observation schemas** + capability schema stub (§15.16).
10. **Scaffold external oracle runner docs** (`tools/oracle/`) — checklist blocked on Revit.
11. **Do not invent ES byte layouts**; do not promote capabilities without evidence gates.
12. After Phase 1 merge: next Ready item = compound-wall research docs and/or small typed-relations stubs only if time — still no invented BIM successes.

---

## Appendix A — Main advanced past `86a43b9` (audit snapshot)

Examples landed after the frozen baseline (non-exhaustive):

| Item | Ref |
|------|-----|
| Private vuln reporting / SEC-001 | #166 |
| Formats integrity diagnostics PARSE-001 | #168 |
| Support-matrix foundation | #167 |
| Changelog / 0.2.0 plan / install honesty | #165 |
| CI/deploy material baseline unblock | #169 |
| Remaining Actions SHA pins | #170 |
| Finding 1 narrow strip + evidence matrix | #160–#163, docs |

`source_coverage` measured fractions may land via a parallel PR; do not duplicate if already merged.

---

## Appendix B — Blocked-on-Revit checklist (Phase 2)

See [`tools/oracle/README.md`](../../tools/oracle/README.md) and [`research/es-remap/README.md`](../../research/es-remap/README.md).

- [ ] Revit API add-in / external runner available
- [ ] Generate N1–N4, R1/R2, C1/C2, C3a/C4a before/after pairs
- [ ] Emit API truth JSON conforming to observation schema
- [ ] One mutation per transition enforced in manifest
- [ ] Cloud/CI consumes fixtures as opaque regression inputs only

---

*End of governing report rev 1.1 (structured mirror).*
