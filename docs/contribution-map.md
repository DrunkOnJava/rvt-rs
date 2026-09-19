# Contribution Map

This map points contributors to work that helps rvt-rs become useful to BIM and
AEC users without overstating current capability.

## Best First Contributions

| Area | Good work | Start here |
|---|---|---|
| Documentation | Clarify support boundaries, add screenshots, improve non-technical wording | [`docs/status.md`](status.md), [`README.md`](../README.md) |
| Corpus | Submit redistributable files and expected counts | [`docs/corpus.md`](corpus.md), [`docs/corpus-intake.md`](corpus-intake.md), corpus issue form |
| Decoder research | Add a byte probe and evidence table for one class or partition pattern | decoder issue form, [`docs/rvt-moat-break-reconnaissance.md`](rvt-moat-break-reconnaissance.md) |
| Tests | Add fixture assertions that prevent false-positive decode claims | `tests/project_corpus_smoke.rs`, `tests/walker_to_ifc_integration.rs` |
| Viewer UX | Make unsupported-file states clearer and accessible | `viewer/`, [`docs/viewer-privacy-posture.md`](viewer-privacy-posture.md) |

## Open Remainders (as of 2026-09-19)

Six roadmap issues closed on 2026-09-19 against measured evidence: wall
geometry (#30), floor and slab geometry (#31), the IFCSLAB recall lift
(#83), the real slab boundary profiles that replaced the rectangle (#87),
wall instance recovery (#81), and the end-to-end MVP workflow (#66). What
is left is narrower and each remainder is named, so pick one rather than
re-opening a solved carrier:

| Issue | Named remainder |
|---|---|
| [#32](https://github.com/DrunkOnJava/rvt-rs/issues/32) | Doors and windows are bound to their host wall and the void/fill chain is exact (#222); the viewer does not yet display the host relationship. The opening cut itself is [#227](https://github.com/DrunkOnJava/rvt-rs/issues/227). |
| [#33](https://github.com/DrunkOnJava/rvt-rs/issues/33) / [#219](https://github.com/DrunkOnJava/rvt-rs/issues/219) | Storey containment binds 969 of 970 building elements after #267 (RE-27) and #90 (RE-29). The one unbound element is a wall whose record names no single Level and whose base sits mid-storey. |
| [#23](https://github.com/DrunkOnJava/rvt-rs/issues/23) | The Revit 2024 ArcWall envelope is still undecoded; RE-21's partition element record is a different carrier and does not close it. |
| [#86](https://github.com/DrunkOnJava/rvt-rs/issues/86) | Partition names are partial and the Level ElementId bind stays blocked by the RE-20 negative. |
| [#156](https://github.com/DrunkOnJava/rvt-rs/issues/156) | The reported sketch-to-solid pipeline is untouched for sweeps and revolves; only the closed-loop plan profile of a slab is recovered (RE-25). |

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
