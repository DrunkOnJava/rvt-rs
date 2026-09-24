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

Twelve roadmap and follow-up issues closed on 2026-09-19 against measured
evidence: wall geometry (#30), floor and slab geometry (#31), doors and
windows with host relationships (#32), the end-to-end MVP workflow (#66),
wall instance recovery (#81), the IFCSLAB recall lift (#83), the real slab
boundary profiles that replaced the rectangle (#87), the two viewer and
reader reporting gaps (#187, #188), the `IFCSHADINGDEVICE`
`.NOTDEFINED.` witness mismatch (#235), the `18" Basement` wall-type slot
(#240, an index artifact rather than a second id space — RE-28), and the
80 cut column bodies (#239, RE-29). What is left is narrower and each
remainder is named, so pick one rather than re-opening a solved carrier:

| Issue | Named remainder |
|---|---|
| [#33](https://github.com/DrunkOnJava/rvt-rs/issues/33) / [#219](https://github.com/DrunkOnJava/rvt-rs/issues/219) | Storey containment binds 969 of 970 building elements: #267 / RE-27 reads the Level the element record names, and #90 / RE-29 gave the 116 rooms element records of their own, which removed the 18 name-only spaces that had nothing to read. The one that remains is a wall whose record names no single Level; it is contained in the `IfcBuilding`, not silently filed under the lowest storey. Nothing on this corpus is scored against Revit's own `IfcRelContainedInSpatialStructure` yet — the reference side is NOT MEASURED. |
| [#238](https://github.com/DrunkOnJava/rvt-rs/issues/238) | Wall join trims are down to 9 over-trimmed ends on 9 walls (from 31 on 24) since #274 / RE-29 required the trim candidate to be named in the record's reference list. The 9 are one side of each of two true L corners; no feature in the record orders the two sides, so this needs a new carrier, not a better heuristic. |
| [#88](https://github.com/DrunkOnJava/rvt-rs/issues/88) | Compound layers are read from each type's own data (RE-53, not near the type record where RE-28 swept) and written as `IfcMaterialLayerSetUsage` with material names (RE-58). What remains is elements drawn whole or sloped, which get no set (#358), and Revit's `Family:Type` set names. |
| [#23](https://github.com/DrunkOnJava/rvt-rs/issues/23) | The Revit 2024 ArcWall envelope is still undecoded; RE-21's partition element record is a different carrier and does not close it. |
| [#86](https://github.com/DrunkOnJava/rvt-rs/issues/86) | Partition names are partial and the Level ElementId bind stays blocked by the RE-20 negative. |
| [#156](https://github.com/DrunkOnJava/rvt-rs/issues/156) | The reported sketch-to-solid pipeline is untouched for sweeps and revolves; only the closed-loop plan profile of a slab is recovered (RE-25). |
| [#227](https://github.com/DrunkOnJava/rvt-rs/issues/227) | Doors and windows are bound to their host wall and the `(host wall, filling element)` pair set equals Revit's export on 138 of 138 (#222, RE-23), and the viewer now shows and can jump across that relationship (#269, #272). What is open is the opening cut itself: the `IfcOpeningElement` is still bodied with the door/window bounding box rather than cut from the wall location curve. |
| [#34](https://github.com/DrunkOnJava/rvt-rs/issues/34) | On Revit 2024 and 2025 the IFC's materials are the file's own `OST_Materials` records, named by ElementId (RE-58; 78 of 86 on Core Interior, where the export writes the 9 its elements use, all among them). What is open is joining family instances to their materials, and the names of 8 family materials here. |
| [#90](https://github.com/DrunkOnJava/rvt-rs/issues/90) | Space bodies are still a placeholder; the real room boundary polygon is not recovered. |

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
