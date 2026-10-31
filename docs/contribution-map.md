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

## Open Remainders (as of 2026-10-31)

The issue tracker is the source of truth; this is its state today, by
milestone, with each issue's own title. Pick one rather than re-opening a
solved carrier: the reports under `reports/element-framing/` say what each
negative result ruled out, and [`ROADMAP.md`](../ROADMAP.md) says what each
milestone is for.

### 0.12.0: IFC geometry and element data

| Issue | Remainder |
|---|---|
| [#1198](https://github.com/DrunkOnJava/rvt-rs/issues/1198) | Docs: two out-of-date statements in `docs/status.md` and `ROADMAP.md` (a good first issue) |
| [#1197](https://github.com/DrunkOnJava/rvt-rs/issues/1197) | Help wanted: outputs only Revit can produce (a DWG export for the second witness edge, API fixtures for extensible storage) |
| [#1196](https://github.com/DrunkOnJava/rvt-rs/issues/1196) | `Formats/Latest`: classify the residual field encodings and Revit 2027's flagged runtime-state fields from byte evidence |
| [#1195](https://github.com/DrunkOnJava/rvt-rs/issues/1195) | Viewer: show real decode progress, and say clearly when a model is too large for the browser |
| [#1194](https://github.com/DrunkOnJava/rvt-rs/issues/1194) | Geometry and element remainders on Revit 2024 and 2025: wall corners, landing undersides, column sections, Areas, room phases |
| [#1165](https://github.com/DrunkOnJava/rvt-rs/issues/1165) | Unfilled openings Revit's export writes and rvt-rs does not: a door's further cuts, curtain-wall cuts and fire hose cabinets |
| [#1144](https://github.com/DrunkOnJava/rvt-rs/issues/1144) | Saved graphics: mesh cone faces, spline-profile surfaces and cylinder faces with non-rectangular trims (a contributor's proposal) |
| [#1117](https://github.com/DrunkOnJava/rvt-rs/issues/1117) | Viewports and their centres on sheets (a contributor's request) |
| [#1067](https://github.com/DrunkOnJava/rvt-rs/issues/1067) | BIM4LCA office: walls a floor covers in part, which Revit steps and rvt-rs draws at their full height (29 as built, 26 at permit), and one wall as built that Revit cuts although 0.0243 ft² of its plan lies outside the floor; the walls a floor covers whole are cut since RE-267 (RE-257, RE-267) |
| [#885](https://github.com/DrunkOnJava/rvt-rs/issues/885) | Roof cases with no oracle: 2024 gables and hips, a slope arrow stopping short; roofs sloped by several slope arrows (puzzbobb's T and U roofs) are drawn as their record box (RE-199 follow-up) |
| [#884](https://github.com/DrunkOnJava/rvt-rs/issues/884) | Hip roofs over T and U footprints stay level blocks: straight-skeleton split events (RE-199 follow-up) |
| [#869](https://github.com/DrunkOnJava/rvt-rs/issues/869) | MEP ports and pipe cylinders outside Revit 2024 and 2025: no 2026 or 2027 MEP model has a Revit export to measure against |
| [#834](https://github.com/DrunkOnJava/rvt-rs/issues/834) | A port's SystemType (its connector's system classification) and an electrical port's Flow=Source/Sink description are not written (RE-212 follow-up) |
| [#810](https://github.com/DrunkOnJava/rvt-rs/issues/810) | BIM4LCA office: 2 doors' OverallWidth is 0.1137 ft wider than Revit's (1756089, 1765267) |
| [#358](https://github.com/DrunkOnJava/rvt-rs/issues/358) | Wall bodies thicker or thinner than their type's layers: 165 Snowdon walls drawn whole (RE-53) |
| [#357](https://github.com/DrunkOnJava/rvt-rs/issues/357) | Stair flights not yet drawn: monolithic spiral runs, riserless runs with a nosing, and nosing profiles |
| [#323](https://github.com/DrunkOnJava/rvt-rs/issues/323) | Roofs Revit's export decomposes into IfcSlab .ROOF. parts (7 of Snowdon's 20) are written whole |
| [#309](https://github.com/DrunkOnJava/rvt-rs/issues/309) | Snowdon: one record-backed slab (ElementId 2062318) is not in Revit's export |
| [#227](https://github.com/DrunkOnJava/rvt-rs/issues/227) | Opening geometry from the symbol's cut loop on releases other than 2024 and 2025: no Revit export of another release's openings is in CI |
| [#223](https://github.com/DrunkOnJava/rvt-rs/issues/223) | Element record +0x46: the flag word is unread (+0x4a is the schema class, RE-76) |
| [#156](https://github.com/DrunkOnJava/rvt-rs/issues/156) | RE: Reproduce sketch-to-solid geometry reconstruction pipeline |
| [#155](https://github.com/DrunkOnJava/rvt-rs/issues/155) | RE: Decode parameter-store wire formats and exact Revit unit encodings |
| [#96](https://github.com/DrunkOnJava/rvt-rs/issues/96) | CLASS-16: MEP: the duct family, sloped runs, fittings and route geometry left after pipes and ducts |
| [#94](https://github.com/DrunkOnJava/rvt-rs/issues/94) | CLASS-14: structural framing sections still not drawn: 6 channels, 25 angles, one HSS column, 17 I beams |
| [#90](https://github.com/DrunkOnJava/rvt-rs/issues/90) | Room outlines: 7 Snowdon Towers rooms still drawn as their box (room solid edge topology) |
| [#35](https://github.com/DrunkOnJava/rvt-rs/issues/35) | M4-07: Recover common parameters |

### 1.0.0: first-class utility

The requirements these serve, each with its bar, status and evidence, are in
[`release-1.0-criteria.md`](release-1.0-criteria.md).

| Issue | Remainder |
|---|---|
| [#1209](https://github.com/DrunkOnJava/rvt-rs/issues/1209) | W7: coverings' `GrossArea` on ceilings in pieces and cut ceilings, and the roofing coverings (RE-297) |
| [#1208](https://github.com/DrunkOnJava/rvt-rs/issues/1208) | W7: single-layer slabs' `Width` and `Length`, sloped floors' `Depth` and the stair landings' volume (RE-295) |
| [#1207](https://github.com/DrunkOnJava/rvt-rs/issues/1207) | W7: spaces' `GrossVolume`, stepped rooms' `NetVolume` and Revit 25's area quantities |
| [#1203](https://github.com/DrunkOnJava/rvt-rs/issues/1203) | W7: a joined column's `Length` and the round column |
| [#1193](https://github.com/DrunkOnJava/rvt-rs/issues/1193) | W7: the BIM4LCA office's remaining property values without a rule (column Slope, opening Reference, mullion Span, sloped slab PitchAngle) |
| [#1192](https://github.com/DrunkOnJava/rvt-rs/issues/1192) | W10: check that every number in the README and the supported profile names its model |
| [#1191](https://github.com/DrunkOnJava/rvt-rs/issues/1191) | W6: draw family instances from their saved meshes by default (decode the remaining saved-graphics faces) |
| [#1190](https://github.com/DrunkOnJava/rvt-rs/issues/1190) | W2, W6, W7: geometry and properties on Revit 2015 to 2023, 2026 and 2027 projects, beyond each element's box |
| [#1189](https://github.com/DrunkOnJava/rvt-rs/issues/1189) | W3: read family (`.rfa`) and template (`.rte`, `.rft`) files like projects: types, parameters and geometry |
| [#1186](https://github.com/DrunkOnJava/rvt-rs/issues/1186) | W7: the BIM4LCA office's stair values on `IfcStair` and the stair and flight types |
| [#1185](https://github.com/DrunkOnJava/rvt-rs/issues/1185) | W7: type objects' common-set values (`IsExternal`, `LoadBearing`, `FireRating`, `Manufacturer`) on the BIM4LCA office's types |
| [#1075](https://github.com/DrunkOnJava/rvt-rs/issues/1075) | Models with a partition stream over 256 MiB opened empty: reported since 0.11.0, with `--large-model`, not yet measured on a model over the limits |
| [#1046](https://github.com/DrunkOnJava/rvt-rs/issues/1046) | W7: base quantities Revit computes from its joined geometry (wall areas and volumes, mullion surfaces) |
| [#1016](https://github.com/DrunkOnJava/rvt-rs/issues/1016) | W7: Revit's base quantities are not measured (52,038 Qto_ values on the BIM4LCA office) |
| [#879](https://github.com/DrunkOnJava/rvt-rs/issues/879) | Revit 2015 BIMcollab example: 3 air terminals Revit's export leaves out, and 16 footings on no storey (their saved Level is unset, RE-281) |
| [#878](https://github.com/DrunkOnJava/rvt-rs/issues/878) | Revit 2015: the elements' own GlobalIds are not rebuilt (0 of 241 on BIMcollab's structural example; project, site, building and storeys are) |
| [#874](https://github.com/DrunkOnJava/rvt-rs/issues/874) | 1.0: practitioner workflow reports folded into the supported profile |
| [#792](https://github.com/DrunkOnJava/rvt-rs/issues/792) | BIM4LCA office: fixture 3095244 is IfcSanitaryTerminal where Revit writes IfcFlowTerminal, and the permit model writes a column Revit's export leaves out |
| [#1158](https://github.com/DrunkOnJava/rvt-rs/issues/1158) | Native records on Revit 2015 to 2022: saved parameters and the phase order are read there since RE-281, experimentally; the other readers built on them (types, stairs, symbol cuts, mullion hosts, ports, saved meshes) stay on 2023 to 2027 until each is measured on a 4-byte ElementId (RE-280, RE-281) |
| [#421](https://github.com/DrunkOnJava/rvt-rs/issues/421) | Element records beyond 2024/2025: derive the marker from the schema and measure the older envelope (RE-80 follow-up) |
| [#408](https://github.com/DrunkOnJava/rvt-rs/issues/408) | Held-out validation: measure on licensed models not used to develop the decoder |

### Unscheduled

None: every open issue has a user-facing acceptance criterion and a milestone.

### Good first issues

- [#1198](https://github.com/DrunkOnJava/rvt-rs/issues/1198): two out-of-date statements in the docs, each a one-line fix.

The `good first issue` label marks the next ones. Issues labelled `help wanted` are where outside help counts most: openly licensed models with Revit's own export ([#408](https://github.com/DrunkOnJava/rvt-rs/issues/408), [#421](https://github.com/DrunkOnJava/rvt-rs/issues/421)), templates and families ([#1189](https://github.com/DrunkOnJava/rvt-rs/issues/1189)), outputs only Revit can produce ([#1197](https://github.com/DrunkOnJava/rvt-rs/issues/1197)) and practitioners' reports ([#874](https://github.com/DrunkOnJava/rvt-rs/issues/874)).

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
