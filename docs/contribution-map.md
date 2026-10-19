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

## Open Remainders (as of 2026-10-19)

The issue tracker is the source of truth; this is its state today, by
milestone, with each issue's own title. Pick one rather than re-opening a
solved carrier: the reports under `reports/element-framing/` say what each
negative result ruled out, and [`ROADMAP.md`](../ROADMAP.md) says what each
milestone is for.

### 0.6.x: IFC geometry, remaining slices

| Issue | Remainder |
|---|---|
| [#358](https://github.com/DrunkOnJava/rvt-rs/issues/358) | Wall bodies thicker or thinner than their type's layers: 165 Snowdon walls drawn whole (RE-53) |
| [#357](https://github.com/DrunkOnJava/rvt-rs/issues/357) | Stair flights not yet drawn: monolithic (end-with-riser flag), riserless, spiral, and nosing profiles |
| [#356](https://github.com/DrunkOnJava/rvt-rs/issues/356) | Sloped roofs need a Revit 2024/2025 pitched-roof oracle |
| [#355](https://github.com/DrunkOnJava/rvt-rs/issues/355) | Element materials: shading colours decode exactly; names and the element-to-material link do not yet |
| [#328](https://github.com/DrunkOnJava/rvt-rs/issues/328) | Snowdon: 6 slabs in the Legends phase are not in Revit's export (phase filtering) |
| [#323](https://github.com/DrunkOnJava/rvt-rs/issues/323) | Stairs, stair runs, landings, stringers and roofs as IFC aggregates |
| [#309](https://github.com/DrunkOnJava/rvt-rs/issues/309) | Snowdon: one record-backed slab (ElementId 2062318) is not in Revit's export |
| [#227](https://github.com/DrunkOnJava/rvt-rs/issues/227) | Opening geometry: cut the IfcOpeningElement from the wall location curve instead of the door/window bbox; recover the 63 slab/shading-device penetrations |
| [#156](https://github.com/DrunkOnJava/rvt-rs/issues/156) | RE: Reproduce sketch-to-solid geometry reconstruction pipeline |
| [#96](https://github.com/DrunkOnJava/rvt-rs/issues/96) | CLASS-16: MEP: the duct family, sloped runs, fittings and route geometry left after pipes and ducts |
| [#94](https://github.com/DrunkOnJava/rvt-rs/issues/94) | CLASS-14: structural framing sections still not drawn: 6 channels, 25 angles, one HSS column, 17 I beams |
| [#90](https://github.com/DrunkOnJava/rvt-rs/issues/90) | Room outlines: 7 Snowdon Towers rooms still drawn as their box (room solid edge topology) |

### 0.7.0: element data (parameters) and viewer journey

| Issue | Remainder |
|---|---|
| [#844](https://github.com/DrunkOnJava/rvt-rs/issues/844) | Core Interior's six windows and one RE1 door write no OverallWidth/OverallHeight: their size is not where RE-187 reads it |
| [#223](https://github.com/DrunkOnJava/rvt-rs/issues/223) | Element record +0x46: the flag word is unread (+0x4a is the schema class, RE-76) |
| [#155](https://github.com/DrunkOnJava/rvt-rs/issues/155) | RE: Decode parameter-store wire formats and exact Revit unit encodings |
| [#35](https://github.com/DrunkOnJava/rvt-rs/issues/35) | M4-07: Recover common parameters |

### 1.0.0: first-class utility

| Issue | Remainder |
|---|---|
| [#421](https://github.com/DrunkOnJava/rvt-rs/issues/421) | Element records beyond 2024/2025: derive the marker from the schema and measure the older envelope (RE-80 follow-up) |
| [#408](https://github.com/DrunkOnJava/rvt-rs/issues/408) | Held-out validation: measure on licensed models not used to develop the decoder |

### Unscheduled

| Issue | Remainder |
|---|---|
| [#834](https://github.com/DrunkOnJava/rvt-rs/issues/834) | A port's description, system classification and HVAC or piping direction are not read (#828 follow-up) |
| [#827](https://github.com/DrunkOnJava/rvt-rs/issues/827) | from_decoded's property set builders put Revit fields in IFC4 standard sets (Pset_WallCommon.BaseOffset, Pset_WindowCommon.SillHeight, ...) |
| [#824](https://github.com/DrunkOnJava/rvt-rs/issues/824) | BIM4LCA office: 31 of 63 slabs' top surfaces differ from Revit's (8 at twice the area, 20 at +0.35%, holes missing) |
| [#823](https://github.com/DrunkOnJava/rvt-rs/issues/823) | BIM4LCA office: 19 ceilings written as separate pieces where Revit's export writes one IfcCovering each |
| [#810](https://github.com/DrunkOnJava/rvt-rs/issues/810) | BIM4LCA office doors: Revit's OverallWidth is the door's own extent, 10 mm over its type's Width; 0 of 268 written as Revit's |
| [#792](https://github.com/DrunkOnJava/rvt-rs/issues/792) | Nordic LCA office: 323 elements exported as a different IFC class than Revit's export |
| [#766](https://github.com/DrunkOnJava/rvt-rs/issues/766) | corpus: put Snowdon Towers, BIM4LCA and the R2025 test models in front of CI |
| [#763](https://github.com/DrunkOnJava/rvt-rs/issues/763) | Nordic LCA office: Revit's export has 44 family instances as IfcSpace (their own Export to IFC As) that rvt-rs writes as proxies |
| [#528](https://github.com/DrunkOnJava/rvt-rs/issues/528) | MEP connectivity: ports and port-to-port connections (IfcDistributionPort, IfcRelNests, IfcRelConnectsPorts) |
| [#154](https://github.com/DrunkOnJava/rvt-rs/issues/154) | RE: Reproduce complete Formats/Latest parsing and serialization-tag assignment |
| [#153](https://github.com/DrunkOnJava/rvt-rs/issues/153) | RE: Validate ElementHeader framing for ElementId and class-tag recovery |
| [#152](https://github.com/DrunkOnJava/rvt-rs/issues/152) | RE: Validate Global/ElemTable body as a versioned ownership tree |
| [#85](https://github.com/DrunkOnJava/rvt-rs/issues/85) | RE-15-05: Symmetric-tuple probe sweep — find Sheet/View/Phase/DesignOption/Workset/ParameterElement resource tuples |

### Good first issues

None is open today: #501 and #502 are done. The `good first issue` label
marks the next one.

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
