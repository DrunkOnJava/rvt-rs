# ES ElementId remapping research (`research/es-remap`)

Governing doc: [`docs/research/unified-research-report.md`](../../docs/research/unified-research-report.md) (rev 1.1).

## Honesty

- **Discovery = owned synthetics.** Production Autodesk / magnetar corpora are **regression only**.
- **Revit API oracle required** for Phase 2 file generation (N1–N4, R1/R2, C1/C2, C3a/C4a). The Cursor Cloud VM has **no Revit** — fixture bytes are not invented here.
- Presence of contracts / schemas / manifests does **not** mean ES remapping works.
- ES reference edges stay **outside** [#152](https://github.com/DrunkOnJava/rvt-rs/issues/152) ElemTable ownership parent scoring.
- Default IFC **omits** ES edges.

## Layout

| Path | Role |
|------|------|
| `manifest.yaml` | Fixture law + ES-remap-00 sketch |
| `observation.schema.json` | Mirror of `docs/schemas/es-observation.schema.json` |
| `capability.schema.json` | Mirror of `docs/schemas/es-capability.schema.json` (§15.16) |
| `README.md` | This file |

## Phase 1 (in-repo, no Revit)

Rust contracts in `rvt::identity`, `rvt::evidence`, `rvt::es_refs`:

- `DocumentIdentity` / `ScopedElementRef` / `SourceSpan`
- `EvidenceTier` + evidence / edge ledgers
- `EsReferenceOccurrence`, path segments, `FixtureTransition` / no-op baselines

## Phase 2 (blocked)

See [`tools/oracle/README.md`](../../tools/oracle/README.md) for the external runner checklist.
