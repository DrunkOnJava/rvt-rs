# Contribution Map

This map points contributors to work that helps rvt-rs become useful to BIM and
AEC users without overstating current capability.

## Best First Contributions

| Area | Good work | Start here |
|---|---|---|
| Documentation | Clarify support boundaries, add screenshots, improve non-technical wording | [`docs/status.md`](status.md), [`README.md`](../README.md) |
| Corpus | Submit redistributable files and expected counts | [`docs/corpus.md`](corpus.md), [`docs/corpus-intake.md`](corpus-intake.md), corpus issue form |
| Decoder research | Add a byte probe and evidence table for one class or partition pattern | decoder issue form, [`docs/format-reconnaissance.md`](format-reconnaissance.md) |
| Tests | Add fixture assertions that prevent false-positive decode claims | `tests/project_corpus_smoke.rs`, `tests/walker_to_ifc_integration.rs` |
| Viewer UX | Make unsupported-file states clearer and accessible | `viewer/`, [`docs/viewer-privacy-posture.md`](viewer-privacy-posture.md) |

## Open Remainders (as of 2026-11-14)

The issue tracker is the source of truth; this is its state today, by
milestone, with each issue's own title. Pick one rather than re-opening a
solved carrier: the reports under `reports/element-framing/` say what each
negative result ruled out, and [`ROADMAP.md`](../ROADMAP.md) says what each
milestone is for.

### 0.25.0: IFC geometry and element data

| Issue | Remainder |
|---|---|
| [#2007](https://github.com/DrunkOnJava/rvt-rs/issues/2007) | IfcRampFlight: Revit's export writes one under a ramp it exports as a container, and rvt-rs writes none (RE-467) |
| [#1987](https://github.com/DrunkOnJava/rvt-rs/issues/1987) | Opening Reference, sloped floor PitchAngle and door FireRating write values Revit leaves out on the BIM4LCA housing (RE-381, RE-384, RE-354) |
| [#1977](https://github.com/DrunkOnJava/rvt-rs/issues/1977) | Multistory stairs: Revit writes the stair once per storey (a ":2" copy with its flights, stringers, landing and railings), rvt-rs writes it once |
| [#1973](https://github.com/DrunkOnJava/rvt-rs/issues/1973) | View graphic overrides: the classes are in the file's schema, no instance is located, and Revit's IFC export gives no oracle |
| [#1962](https://github.com/DrunkOnJava/rvt-rs/issues/1962) | Saved parameters: built-in parameters' names and a family's built-in slots (#35 follow-up, RE-207) |
| [#1947](https://github.com/DrunkOnJava/rvt-rs/issues/1947) | Batch 9 export findings still to fix: 12 confirmed, 6 to settle with a run |
| [#1870](https://github.com/DrunkOnJava/rvt-rs/issues/1870) | BIM4LCA office: wall 1348326's GLB layers start 0.49 ft from Revit's end, held there by a short-edge corner |
| [#1864](https://github.com/DrunkOnJava/rvt-rs/issues/1864) | Autodesk samples, 2025 against 2026 saves: 6 bodies, 1 profile and 12 level binds differ with no known cause |
| [#1856](https://github.com/DrunkOnJava/rvt-rs/issues/1856) | Stair runs drawn from their sketch keep it where Revit's saved mesh exists: same box, 2.55% more volume on rac_advanced 2024 |
| [#1842](https://github.com/DrunkOnJava/rvt-rs/issues/1842) | Revit 2027 structural model: 104 columns, beams, slabs, flights and footings on no storey (#1829) |
| [#1841](https://github.com/DrunkOnJava/rvt-rs/issues/1841) | IFC class: foundation slabs are written as IfcFooting where Revit writes IfcSlab, and one framing member as IfcBeam where Revit writes IfcMember (#1829) |
| [#1839](https://github.com/DrunkOnJava/rvt-rs/issues/1839) | Beams run to the ends of their location line, where Revit's export cuts them back at the columns they frame into (#1829) |
| [#1833](https://github.com/DrunkOnJava/rvt-rs/issues/1833) | Crafted files: eighteen readers do unbounded work or hold unbounded memory, two of them exponential |
| [#1829](https://github.com/DrunkOnJava/rvt-rs/issues/1829) | Report: Revit 2027 structural model (CHL), compared element by element with Revit's IFC4 export and run through a clash engine |
| [#1824](https://github.com/DrunkOnJava/rvt-rs/issues/1824) | Revit 2015 to 2023 exports read every wall record twice for its top join, and GStyleElem twice for category materials |
| [#1814](https://github.com/DrunkOnJava/rvt-rs/issues/1814) | Concrete housing at permit: wall ends 1.28 to 1.38 ft short where the box's join trim meets a partner whose join decides no reach |
| [#1808](https://github.com/DrunkOnJava/rvt-rs/issues/1808) | saved_meshes::attach on 0.14: owner graphs are the peak holder (request for a 0.14-compatible fix) |
| [#1803](https://github.com/DrunkOnJava/rvt-rs/issues/1803) | Timber office at permit: mullions 3028901 and 3028902 lose their NetSurfaceArea once a neighbour keeps its newest frame (RE-444) |
| [#1801](https://github.com/DrunkOnJava/rvt-rs/issues/1801) | Autodesk rme samples, 2025 to 2027 saves: 131 columns in each hold a Length (3.5, 7.3, 10.6 or 7.0 m) that is not their mesh's extent, which the 2024 saves give |
| [#1798](https://github.com/DrunkOnJava/rvt-rs/issues/1798) | Mullion NetSurfaceArea: 120 mullions of the concrete office and 127 of the timber office, as built, are still without a value after RE-441 |
| [#1719](https://github.com/DrunkOnJava/rvt-rs/issues/1719) | Beams with no bounded line in their element data: Revit writes Length, Span, Slope and Roll on 9 BIM4LCA beams that rvt-rs cannot give them (RE-428 residual) |
| [#1712](https://github.com/DrunkOnJava/rvt-rs/issues/1712) | volume_readings: a round bar cut at a slant has elliptical end faces, which the shell reading leaves as another reason |
| [#1706](https://github.com/DrunkOnJava/rvt-rs/issues/1706) | Snowdon Towers Architectural: roofs 1916596 and 964559 stay level blocks (a plane edge across a curved face that is not isoparametric; a cylinder face with several trim loops) |
| [#1698](https://github.com/DrunkOnJava/rvt-rs/issues/1698) | Beams: a body that is not Revit's solid (the office's 8 hollow tubes, 20 of the concrete housing's 22 beams, 10 of the timber housing's 41) |
| [#1635](https://github.com/DrunkOnJava/rvt-rs/issues/1635) | Railings are written as record boxes, about a thousand times Revit's volume, and now sit inside their stairs |
| [#1589](https://github.com/DrunkOnJava/rvt-rs/issues/1589) | research(rooms): read the height a room's area is taken at, so stepped rooms get their section's outline (follow-up of #90, RE-374) |
| [#1583](https://github.com/DrunkOnJava/rvt-rs/issues/1583) | Wall-end rule: a wall in a strip that the pass does not read is found by nothing, so the end is drawn through it with no count (RE-337 section 6) |
| [#1507](https://github.com/DrunkOnJava/rvt-rs/issues/1507) | W7: windows' Pset_WindowCommon.FireRating is read from family-parameter table entries (R2025 models 12 to 17, held-out models) |
| [#1498](https://github.com/DrunkOnJava/rvt-rs/issues/1498) | Value sources: wall rules, door placement, joins and cut loops have none yet, and nothing fails when a new value lands without one |
| [#1495](https://github.com/DrunkOnJava/rvt-rs/issues/1495) | Snowdon wall 1996083 ends 1.03 ft short of Revit's export, and nothing marks it |
| [#1494](https://github.com/DrunkOnJava/rvt-rs/issues/1494) | Doors bound to no plain wall: 49 inverted relative to the axes rule, by IfcDoorType (RE-347 follow-up) |
| [#1480](https://github.com/DrunkOnJava/rvt-rs/issues/1480) | Wall base and top passes: a cutter drawn as a box, or whose level is not read, cuts nothing (RE-351 follow-up) |
| [#1478](https://github.com/DrunkOnJava/rvt-rs/issues/1478) | Walls under a read roof: cut along its sloped underside, as Revit does (RE-351 follow-up) |
| [#1474](https://github.com/DrunkOnJava/rvt-rs/issues/1474) | Walls 2056705 and 2056770: Revit cuts their ends back above 182.087 ft and steps them, and neither office file draws that |
| [#1427](https://github.com/DrunkOnJava/rvt-rs/issues/1427) | Semantic check: recalibrate on real pull requests, and settle fails that noise can cross (#1334 follow-up) |
| [#1407](https://github.com/DrunkOnJava/rvt-rs/issues/1407) | Wall 2177212: its saved mesh is 0.114 ft shorter than Revit's IFC body (RE-340 follow-up) |
| [#1357](https://github.com/DrunkOnJava/rvt-rs/issues/1357) | Doors and windows: decode hand flip and facing flip |
| [#1261](https://github.com/DrunkOnJava/rvt-rs/issues/1261) | Revit 2018 and 2021 projects decode with no rooms (Revit's IFC has 73 and 8) |
| [#1247](https://github.com/DrunkOnJava/rvt-rs/issues/1247) | v0.12.0 vs v0.6.1: some meshes export as bounding boxes; some doors as IfcOpeningElement |
| [#1197](https://github.com/DrunkOnJava/rvt-rs/issues/1197) | Help wanted: outputs only Revit can produce (a DWG export for the second witness edge, API fixtures for extensible storage) |
| [#1195](https://github.com/DrunkOnJava/rvt-rs/issues/1195) | Viewer: show real decode progress, and say clearly when a model is too large for the browser |
| [#1194](https://github.com/DrunkOnJava/rvt-rs/issues/1194) | Geometry and element remainders on Revit 2024 and 2025: wall corners, landing undersides, column sections, Areas, room phases |
| [#1165](https://github.com/DrunkOnJava/rvt-rs/issues/1165) | Unfilled openings Revit's export writes and rvt-rs does not: a door's further cuts, curtain-wall cuts and fire hose cabinets (591 values on the office) |
| [#1117](https://github.com/DrunkOnJava/rvt-rs/issues/1117) | Feature: extract viewports and viewport centers placed on sheets. |
| [#1067](https://github.com/DrunkOnJava/rvt-rs/issues/1067) | Walls drawn taller than Revit's body on 78 of the BIM4LCA office's walls (RE-257) |
| [#885](https://github.com/DrunkOnJava/rvt-rs/issues/885) | Roof cases with no oracle: 2024 gables and hips, a square two-cut with a fascia, a slope arrow stopping short (RE-199 follow-up) |
| [#884](https://github.com/DrunkOnJava/rvt-rs/issues/884) | Hip roofs over T and U footprints stay level blocks: straight-skeleton split events (RE-199 follow-up) |
| [#869](https://github.com/DrunkOnJava/rvt-rs/issues/869) | MEP ports and pipe cylinders outside Revit 2024 and 2025: no 2026 or 2027 MEP model has a Revit export to measure against |
| [#834](https://github.com/DrunkOnJava/rvt-rs/issues/834) | A port's SystemType (its connector's system classification) and an electrical port's Flow=Source/Sink description are not written (RE-212 follow-up) |
| [#358](https://github.com/DrunkOnJava/rvt-rs/issues/358) | Wall bodies thicker or thinner than their type's layers: 165 Snowdon walls drawn whole (RE-53) |
| [#357](https://github.com/DrunkOnJava/rvt-rs/issues/357) | Stair flights not yet drawn: monolithic spiral runs, riserless runs with a nosing, and nosing profiles |
| [#323](https://github.com/DrunkOnJava/rvt-rs/issues/323) | Roofs Revit's export decomposes into IfcSlab .ROOF. parts (7 of Snowdon's 20) are written whole |
| [#309](https://github.com/DrunkOnJava/rvt-rs/issues/309) | Snowdon: one record-backed slab (ElementId 2062318) is not in Revit's export |
| [#227](https://github.com/DrunkOnJava/rvt-rs/issues/227) | Opening geometry from the symbol's cut loop on releases other than 2024 and 2025: no Revit export of another release's openings is in CI |
| [#223](https://github.com/DrunkOnJava/rvt-rs/issues/223) | Element record +0x46: the flag word's bits no Revit export has tested (RE-261) |
| [#156](https://github.com/DrunkOnJava/rvt-rs/issues/156) | RE: Reproduce sketch-to-solid geometry reconstruction pipeline |
| [#155](https://github.com/DrunkOnJava/rvt-rs/issues/155) | RE: Decode parameter-store wire formats and exact Revit unit encodings |
| [#96](https://github.com/DrunkOnJava/rvt-rs/issues/96) | CLASS-16: MEP: the duct family, sloped runs, fittings and route geometry left after pipes and ducts |
| [#94](https://github.com/DrunkOnJava/rvt-rs/issues/94) | CLASS-14: structural framing sections still not drawn: 6 channels, 25 angles, one HSS column, 17 I beams |

### 1.0.0: first-class utility

The requirements these serve, each with its bar, status and evidence, are in
[`release-1.0-criteria.md`](release-1.0-criteria.md).

| Issue | Remainder |
|---|---|
| [#1710](https://github.com/DrunkOnJava/rvt-rs/issues/1710) | W7: columns whose body is the type's rectangle where Revit's section is not one (4 chamfered housing columns, 32 MEP struts), and 2 timber-office Lengths that differ |
| [#1636](https://github.com/DrunkOnJava/rvt-rs/issues/1636) | List the shipped rules that rest on one project, and the second model that could test each |
| [#1565](https://github.com/DrunkOnJava/rvt-rs/issues/1565) | W5: a sealed registry for fresh held-out pairs, read only by the final 1.0 check |
| [#1444](https://github.com/DrunkOnJava/rvt-rs/issues/1444) | rvt-rs reads no reinforcing bars or element assemblies |
| [#1209](https://github.com/DrunkOnJava/rvt-rs/issues/1209) | W7: coverings' GrossArea on ceilings in pieces and cut ceilings, and the roofing coverings (RE-297) |
| [#1208](https://github.com/DrunkOnJava/rvt-rs/issues/1208) | W7: single-layer slabs' Width and Length, sloped floors' Depth and the stair landings' volume (RE-295, RE-298) |
| [#1207](https://github.com/DrunkOnJava/rvt-rs/issues/1207) | W7: spaces' GrossVolume, stepped rooms' NetVolume and Revit 25's area quantities (RE-296) |
| [#1193](https://github.com/DrunkOnJava/rvt-rs/issues/1193) | W7: the BIM4LCA office's remaining property values without a rule (column Slope, opening Reference, mullion Span, sloped slab PitchAngle) |
| [#1192](https://github.com/DrunkOnJava/rvt-rs/issues/1192) | W10: check that every number in the README and the supported profile names its model |
| [#1191](https://github.com/DrunkOnJava/rvt-rs/issues/1191) | W6: draw family instances from their saved meshes by default (decode the remaining saved-graphics faces) |
| [#1190](https://github.com/DrunkOnJava/rvt-rs/issues/1190) | W2, W6, W7: geometry and properties on Revit 2015 to 2023, 2026 and 2027 projects, beyond each element's box |
| [#1189](https://github.com/DrunkOnJava/rvt-rs/issues/1189) | W3: read family (.rfa) and template (.rte, .rft) files like projects: types, parameters and geometry |
| [#1075](https://github.com/DrunkOnJava/rvt-rs/issues/1075) | Bug: models with a partition stream over 256 MiB open empty, with no error |
| [#1046](https://github.com/DrunkOnJava/rvt-rs/issues/1046) | W7: base quantities Revit computes from its joined geometry (wall areas and volumes, mullion surfaces) |
| [#1016](https://github.com/DrunkOnJava/rvt-rs/issues/1016) | W7: Revit's base quantities: the census runs, and what is left is on #1046, #1165, #1203, #1207, #1208 and #1209 |
| [#874](https://github.com/DrunkOnJava/rvt-rs/issues/874) | 1.0: practitioner workflow reports folded into the supported profile |
| [#792](https://github.com/DrunkOnJava/rvt-rs/issues/792) | BIM4LCA office: fixture 3095244 is IfcSanitaryTerminal where Revit writes IfcFlowTerminal, and the permit model writes a column Revit's export leaves out |
| [#421](https://github.com/DrunkOnJava/rvt-rs/issues/421) | Element records beyond 2024/2025: derive the marker from the schema and measure the older envelope (RE-80 follow-up) |
| [#408](https://github.com/DrunkOnJava/rvt-rs/issues/408) | Held-out validation: measure on licensed models not used to develop the decoder |

### 1.x: Revit 2015 to 2023, one release at a time

Work that serves only Revit 2015 to 2023. 1.0 covers Revit 2024 to 2027 at the full bar
([`release-1.0-criteria.md`](release-1.0-criteria.md)); each 1.x release then adds one
earlier year as a supported release.

| Issue | Remainder |
|---|---|
| [#1985](https://github.com/DrunkOnJava/rvt-rs/issues/1985) | The export phase on Revit 2015 rests on one file (RE-372) |
| [#1984](https://github.com/DrunkOnJava/rvt-rs/issues/1984) | Stored storey GlobalIds on Revit 2015 to 2023 rest on one file (RE-220) |
| [#1983](https://github.com/DrunkOnJava/rvt-rs/issues/1983) | Room outlines from their stored solid on Revit 2023 rest on one file (RE-102) |
| [#1982](https://github.com/DrunkOnJava/rvt-rs/issues/1982) | A 2015 to 2023 wall's IsExternal from its type's Function, and parameters named for a property, each rest on one file (RE-283) |
| [#1981](https://github.com/DrunkOnJava/rvt-rs/issues/1981) | Beams cut at the column their list names, on Revit 2023, rest on one file (RE-122) |
| [#1980](https://github.com/DrunkOnJava/rvt-rs/issues/1980) | A framing element's top Level on Revit 2015 to 2023 rests on eight beams of one file (RE-119, RE-461) |
| [#1695](https://github.com/DrunkOnJava/rvt-rs/issues/1695) | Revit 2022 and earlier: the export parameters are not read, so a furniture element carries no PredefinedType where Revit's export writes NOTDEFINED |
| [#1603](https://github.com/DrunkOnJava/rvt-rs/issues/1603) | Revit 2015 to 2023: family instances' materials: 70 windows and 14 railings lack Revit's, the default material is written on 72 elements Revit gives none (Vitoria) |
| [#1584](https://github.com/DrunkOnJava/rvt-rs/issues/1584) | Revit 2015 to 2023 walls: a record's control joins read empty, so Length, side areas, footprint, volumes and angled walls' line bodies stay out (RE-364 section 6, from #1535) |
| [#1335](https://github.com/DrunkOnJava/rvt-rs/issues/1335) | Revit 2015–2023 exports: the remaining time against 0.11.0 after #1237 |
| [#1158](https://github.com/DrunkOnJava/rvt-rs/issues/1158) | Native records on Revit 2015 to 2022: types, materials, stairs, cut loops, mullion hosts, connectors, family forms and saved meshes are read only from 2023 |
| [#879](https://github.com/DrunkOnJava/rvt-rs/issues/879) | Revit 2015 BIMcollab example: 3 air terminals Revit's export leaves out, and 16 footings on no storey |
| [#878](https://github.com/DrunkOnJava/rvt-rs/issues/878) | Revit 2015, BIMcollab's structural example: 241 of 241 element GlobalIds differ from an export that went through another tool; the rule holds on rac_advanced 2015 |

### Unscheduled

None: every open issue has a user-facing acceptance criterion and a milestone.

### Good first issues

None is open now: #1198, the last, was fixed in a first contribution (#1214). The `good first issue` label marks the next ones. Issues labelled `help wanted` are where outside help counts most: openly licensed models with Revit's own export ([#408](https://github.com/DrunkOnJava/rvt-rs/issues/408), [#421](https://github.com/DrunkOnJava/rvt-rs/issues/421)), templates and families ([#1189](https://github.com/DrunkOnJava/rvt-rs/issues/1189)), outputs only Revit can produce ([#1197](https://github.com/DrunkOnJava/rvt-rs/issues/1197)) and practitioners' reports ([#874](https://github.com/DrunkOnJava/rvt-rs/issues/874)).

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
