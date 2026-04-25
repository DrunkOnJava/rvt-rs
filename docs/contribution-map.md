# Contribution Map

This map points contributors to work that helps rvt-rs become useful to BIM and
AEC users without overstating current capability.

## Best First Contributions

| Area | Good work | Start here |
|---|---|---|
| Documentation | Clarify support boundaries, add screenshots, improve non-technical wording | [`docs/status.md`](status.md), [`README.md`](../README.md) |
| Corpus | Submit redistributable files and expected counts | [`docs/corpus.md`](corpus.md), corpus issue form |
| Decoder research | Add a byte probe and evidence table for one class or partition pattern | decoder issue form, [`docs/rvt-moat-break-reconnaissance.md`](rvt-moat-break-reconnaissance.md) |
| Tests | Add fixture assertions that prevent false-positive decode claims | `tests/project_corpus_smoke.rs`, `tests/walker_to_ifc_integration.rs` |
| Viewer UX | Make unsupported-file states clearer and accessible | `viewer/`, [`docs/viewer-privacy-posture.md`](viewer-privacy-posture.md) |

## Work That Needs Design Discussion

Open or comment on an issue before starting:

- Partition-stream record framing or `ElemTable` linkage.
- Changes to the IFC semantic mapping.
- Field-level Revit write APIs.
- Any change that broadens public "supported" claims.
- Any contribution using sample files with unclear redistribution rights.

## Evidence Expectations

Decoder and corpus contributions should include:

- The Revit release and file type.
- The smallest redistributable fixture or a private reproducer note.
- A command that reproduces the observation.
- Expected counts when known, such as walls, floors, doors, windows, levels, or
  `ElemTable` record count.
- A statement of what would falsify the hypothesis.

## Closing Criteria

An issue is ready to close when:

- The implementation is present on `main`.
- The capability is covered by tests or a documented manual verification path.
- README, roadmap, and compatibility claims remain honest.
- `tools/quality.sh` passes locally, with optional audit/deny checks noted if
  the tools are not installed.
