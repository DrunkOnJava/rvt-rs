# 1.0.0 release criteria

Measured 2026-10-21, on `main` at `e56245fc`, and the element rows W2, W4,
W5, W7 and W8 again on 2026-10-29 from Measure runs 40927543524 and
40929095759 on `main` at `af500685`; the wall tops of W6 on 2026-10-30
from Measure run 41092512565 against `main` at `ee0398e5`; the stability rows S1, S2 and S4
and the viewer row W9 were measured again on 2026-10-23, in the pull requests
that changed them, and the install row W1, the API documentation row S3 and
W8's large-model evidence on 2026-10-30, from what v0.11.0 published.

1.0.0 is the first release with a stability commitment
([`RELEASE.md`](../RELEASE.md#versioning)). Its milestone states the goal: a
new user installs rvt-rs, opens a model, gets correct elements, geometry and
properties, and can reproduce the measurements. The maintainer set the scope
on 2026-10-21: 1.0 supports every Revit release from 2015 to 2027, and it
ships with no gaps or limitations. Every body is Revit's own shape, every
property Revit's export writes is written, and no release in that range is
experimental. On 2026-10-26 the maintainer excluded the values a project's
own export-time property set file places, which the model does not store
(W7). This page breaks that goal into requirements that can be checked.
1.0.0 is tagged when every requirement has passed.

Each requirement has a bar, the value measured now, a status and its
evidence. A status is one of three values:

- **passed**: measured, and the bar is met.
- **failed**: measured, and the bar is not met.
- **not measured**: no evidence either way, or no bar set yet.

A closed issue or a milestone's completion percentage is not evidence. A bar
marked *proposed* is a draft until the maintainer sets it. A status changes
only in a pull request that links the new evidence.

## Workflows

| | Requirement | Bar | Measured now | Status |
|---|---|---|---|---|
| W1 | A new user installs it | Every channel (crates.io, the PyPI wheels, the CLI archives, the container image, the viewer) is checked from what was published, as [Post-publish install verification](../RELEASE.md#post-publish-install-verification) describes | All five checked from what was published for v0.11.0 ([`install.md`](install.md)) | **passed** for 0.11.0; repeated at the 1.0 tag |
| W2 | Every release from 2015 to 2027 is supported | No release in that range is experimental. Each has at least one project with Revit's own IFC export of that release that meets W4, W6 and W7 | 2024 has exact element sets against a same-release export (Core Interior, RE-21). 2025's same-release evidence is puzzbobb's Revit 2025 models (22 with a Revit export, all exact by ElementId, Measure only) and `teste_export_2025` (a Revit 25.4 export, local only); RE1's exports are by Revit 26.2, a cross-release pair (RE-32). 2023 has 37 of 37 and 37 of 45, the other 8 being window trims nested in families (RE-81). 2026 has 113 of 115 on one project (RE-124). 2015 has one: BIMcollab's structural example against Revit 2015's own export, all 241 of its elements by ElementId, in Revit's class or IFC4's for its IFC2X3 one, plus 3 it leaves out ([RE-198](../reports/element-framing/RE-198-revit-2015-bimcollab-export.md), [#879](https://github.com/DrunkOnJava/rvt-rs/issues/879)). 2016 to 2022 and 2027 have no same-release export, and are measured only against the same models saved by a neighbouring release (RE-179 to RE-181, [#421](https://github.com/DrunkOnJava/rvt-rs/issues/421)); the native readers on 2015 to 2022 are [#1158](https://github.com/DrunkOnJava/rvt-rs/issues/1158), and geometry and properties beyond each element's box on 2015 to 2023, 2026 and 2027 are [#1190](https://github.com/DrunkOnJava/rvt-rs/issues/1190) | **failed** |
| W3 | Family and template files are read like projects | A family (`.rfa`) or template (`.rte`, `.rft`) of every supported release yields its types, parameters and geometry, measured against Revit's export of the family placed in a project | Metadata, previews, schema and OmniClass, and a family's types and type parameters as its `PartAtom` stream lists them, Revit's text at save rather than the family's records, and not compared with an export of the family placed in a project ([supported profile](supported-profile.md#by-input)) ([#1189](https://github.com/DrunkOnJava/rvt-rs/issues/1189)) | **failed** |
| W4 | Correct elements on the models the decoder was developed on | On every reference model, the elements written and their IFC classes equal Revit's export of that model in every category: none missing, none extra | Core Interior (2024): exact in its six instance categories (RE-21, run in CI). RE1 (2025): Architecture 84, Electrical 49 and Mechanical 74 exact by ElementId; Plumbing writes one element Revit's export leaves out, a water closet tank (442378, pinned in `tests/element_records_2025.rs`). puzzbobb's Revit 2025 models: all 22 with an export exact. BIM4LCA office (2024): all 12,974 as built; at building permit one column Revit's export leaves out (1927742), and on both files one fixture in another class (3095244) ([#792](https://github.com/DrunkOnJava/rvt-rs/issues/792)) (Measure run 40927543524, 2026-10-29). Snowdon Towers (2024, local only): 5,945 of 5,968 in Revit's export; the other 23 are curtain panels with no resolved type ([#309](https://github.com/DrunkOnJava/rvt-rs/issues/309)), and the 6 slabs in the Legends phase of [#328](https://github.com/DrunkOnJava/rvt-rs/issues/328) are no longer written (RE-184; Measure run 40930512289: `IfcSlab` 224 to 218, the only change on any CI model; [README](../README.md#what-rvt-rs-reads-from-real-projects)) | **failed** |
| W5 | Correct on models it was not developed on ([#408](https://github.com/DrunkOnJava/rvt-rs/issues/408)) | At least two projects of every supported release, used in no RE report, each with Revit's own IFC export of that release and measured once before any fix is made for them, meet W4, W6 and W7. The results are published as measured | One held-out measurement: Snowdon Towers Plumbing, HVAC and Electrical (2024). It scored against a VIM of the 2027 edition rather than Revit's IFC export of these files: all 10,385 exported elements in the expected category, storeys and GlobalIds equal, every body a box ([report](../reports/validation/held-out-snowdon-mep-2026-09-27.md)). The first held-out project with Revit's own export of its release was BIMcollab's Revit 2015 structural example (RE-198), measured before any fix; RE-208 and RE-220 have since used it, so it no longer counts toward this bar, and neither do the holdout pack's models (`research/holdout-models.tsv`), which later reports used. No model meets this bar today. It fails W4: 3 elements written that Revit's export leaves out, and 16 footings on no storey, whose saved Level is unset ([#879](https://github.com/DrunkOnJava/rvt-rs/issues/879), RE-281). It also fails on identity: the project's, site's, building's and five storeys' GlobalIds are Revit's, from the values the file stores ([RE-220](../reports/element-framing/RE-220-stored-global-ids.md)), but none of its 241 elements' ([#878](https://github.com/DrunkOnJava/rvt-rs/issues/878)) | **failed** |
| W6 | Geometry | Every element's body is Revit's own shape, measured per category against Revit's export of the same model at the tolerance its report states. No body is a bounding box | Below, under [W6](#w6-measured-now) | **failed** |
| W7 | Properties | Every property set, property and quantity Revit's export writes for an element is written, with Revit's value, unit and set, and none Revit's export omits (the maintainer: quantities 2026-10-26, property values 2026-10-29). A value that the export's own user-defined property set file maps into a set is out of scope (the maintainer, 2026-10-26): the model does not store the file, and RE-239 lists those values per reference model | Below, under [W7](#w7-measured-now) | **failed** |
| W8 | Nothing is approximated without saying so | A supported input gets no approximation. An input 1.0 does not support (Revit 2014 and earlier, a damaged file) is refused or reported as such by the diagnostics, `rvt-inspect` and the viewer | The diagnostics count boxed bodies (#409) and unattributed records (`unexported_element_records`). Since [#871](https://github.com/DrunkOnJava/rvt-rs/issues/871), all three say when a file's release is read experimentally. Supported inputs are still approximated: boxed bodies and unread properties (W6, W7). Before 0.11.0 a stream over a resource limit was skipped without a diagnostic, so a large model exported empty ([#1075](https://github.com/DrunkOnJava/rvt-rs/issues/1075)). Since 0.11.0 the export diagnostics name the stream, `rvt-ifc` and `rvt-gltf` exit with status 3 instead of writing an empty model, and the viewer offers a retry with the large-model limits; measured by lowering the limits below the reference files' streams, and no model over the default limits, the reporter's included, has been read with `--large-model` yet | **failed** |
| W9 | The viewer journey | Open, inspect, schedule, export and diagnostics, verified end to end on the reference models | `viewer/tests/journey.spec.ts` runs the whole journey in Chromium on the six redistributable reference models: Einhoven, Core Interior and the four RE1 models (Deploy viewer, run [40001472629](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40001472629)). It opens each, inspects an element, and downloads the schedule, IFC, GLB, plan SVG and diagnostics. The inspected element is in the schedule and in the IFC under its ElementId. The schedule lists exactly the elements the diagnostics count by type. On Einhoven and Core Interior, the export has the counts `tests/project_count_fixtures.rs` pins for the CLI's. The local-only reference models ([reference models](reference-models.md)) are not run | **passed** |
| W10 | The measurements reproduce | *Proposed:* every number in the README and the supported profile names its model and says whether anyone can fetch it. Numbers on redistributable models re-run with one documented command | Redistributable models are fetched by `tools/fetch-reference-models.sh` and scored in CI and Measure, and the witness verdicts are checked in CI. Revit's IFC export of Snowdon Towers, the MIT house and the 255ribeiro projects are local only, so the numbers resting on them cannot be re-run by others ([reference models](reference-models.md)). Nothing yet checks that every number names its model ([#1192](https://github.com/DrunkOnJava/rvt-rs/issues/1192)) | **not measured** |

### W6 measured now

<!-- One sentence or list item per line: a change to one value then touches one line, and pull requests changing different values merge cleanly. -->

Exact where the element's data is decoded, the record's box elsewhere;
the diagnostics count each ([#409](https://github.com/DrunkOnJava/rvt-rs/issues/409)).
Doors and windows are drawn by default from the mesh Revit saved, where all of it decodes (the maintainer,
2026-10-29);
other loadable families are boxes unless `--saved-meshes` draws theirs, which is Revit's own shape where it is complete (every vertex within 0.001 ft on all 138 of Core Interior's doors and windows and 1,294 of 1,709 bodies scored on the BIM4LCA office) but part of the body elsewhere (RE-78).
A face the saved graphics do not decode marks a partial mesh, which keeps its box since RE-224: on the office 1,294 of the 1,352 bodies then drawn are exact.
Some partial meshes record nothing, so every class but doors and windows stays opt-in;
on the office 221 of 269 doors keep their box, their saved graphics not decoding whole (Measure run 40932136360).
Walls: 360 of 360 with Revit's ends on Core Interior, 482 of 607 on the office as built (RE-222,
RE-223).
Wall tops: a wall whose top join or cut pair names a floor, roof or footing that covers its whole plan is cut at its underside, which is Revit's top on 64 of the office's 65 cut tops as built and 70 of 70 at permit (RE-267);
the 29 walls as built and 26 at permit that Revit steps over a floor that covers them in part keep their full height, and so does the one wall as built that Revit cuts although 0.0243 ft² of its plan lies outside the floor.
Per-category results are in the [supported profile](supported-profile.md#geometry-on-revit-2024-and-2025-projects);
the residuals are the issues under the 0.12.0 milestone, and `tools/re/box_bodies_vs_ifc.py` counts in Measure, per class, the bodies still drawn as their box and how much of it Revit's body fills
Open: family instances other than doors and windows drawn from their saved meshes by default ([#1191](https://github.com/DrunkOnJava/rvt-rs/issues/1191)),
and geometry on releases other than 2024 and 2025 ([#1190](https://github.com/DrunkOnJava/rvt-rs/issues/1190)).

### W7 measured now

<!-- One sentence or list item per line: a change to one value then touches one line, and pull requests changing different values merge cleanly. -->

Names,
types,
storeys,
materials,
layers,
GlobalIds,
`IsExternal`,
curtain mullions' and panels' `IsExternal` by their curtain wall's, as exporters from IFC_v24.2.20 on give it,
the exterior members' checked against revit-ifc's source only (RE-288, [#1187](https://github.com/DrunkOnJava/rvt-rs/issues/1187)),
walls' `IsExternal` and the parameters named for a standard property on Revit 2015 to 2023 (RE-283),
curtain mullions' `Span` (RE-226),
beams' `Span` (RE-49),
beams' `Slope` 0 where drawn along a level location line (RE-49),
unrotated beams' `Roll` (RE-276),
walls' `ExtendToStructure` (RE-228),
type objects' `FireRating` and `LoadBearing`, with stair types' and flight types' dimensions
(RE-286, [#1185](https://github.com/DrunkOnJava/rvt-rs/issues/1185); no `IsExternal` on a type object: the office's and the R2025 models' exporters write it, but Revit's newer exporters, those of the held-out models and of RE1 Architecture, write none),
walls' `ExtendToStructure` on Revit 2015 to 2023 (RE-284),
railings' `Height` (RE-231),
level slabs' `PitchAngle` (RE-238),
pipes' `InvertElevation` (RE-157),
storeys' `Reference` and `EntranceLevel` (RE-275),
stairs' `TreadLengthAtInnerSide` and, with `--revit-set-placement`, their `Pset_StairFlightCommon` from their flights (RE-287, [#1186](https://github.com/DrunkOnJava/rvt-rs/issues/1186)),
stair flights' treads,
tread lengths,
nosing and `Reference`,
landings' `Reference` and parameters named for a standard property set are read.
On the BIM4LCA office the export's own property set file places 2,013 values as built and 1,938 at permit that rvt-rs does not write as Revit does (glazing on walls,
curtain walls and coverings,
rooms' `OccupancyType` and the value type of their `OccupancyNumber`,
the `FI_` sets), out of scope;
no other reference model has one (RE-239).
Quantities are measured by `tools/re/qtos_vs_ifc.py` ([#1016](https://github.com/DrunkOnJava/rvt-rs/issues/1016));
those Revit's export computes from its joined geometry, which the file does not store (walls' side areas and volumes,
and the `NetSurfaceArea` of the mullions whose ends the boxes do not say Revit keeps,
RE-248,
RE-268), stay in W7 (the maintainer,
2026-10-26) and are reached by reproducing Revit's joins ([#1046](https://github.com/DrunkOnJava/rvt-rs/issues/1046)): of the 39,388 Revit's export writes on objects rvt-rs writes on the office as built, 29,176 are written equal (curtain mullions' `Length`,
`NetSurfaceArea` where the boxes of their wall's mullions say which ends Revit's solid keeps (6,606 of 10,003, RE-268),
and `NetVolume` where Revit's export draws the mullion as an extrusion of its section (RE-243),
doors' and windows' `Width`,
`Height` and `Area` where the symbol drawing each and its type agree on its size,
walls' `Width` (on a held-out Revit 2025 model one wall's is counted apart: its export, made hours after the file's last save, gives it a type the saved file does not, RE-253, [#1048](https://github.com/DrunkOnJava/rvt-rs/issues/1048), the maintainer, 2026-10-27) and their door openings' `Depth` (RE-244),
walls' `Length` where their curve cut back at their joins is as long as their curve,
the two lengths Revit's export writes by how it draws a wall (RE-252;
where the two differ, nothing in the model says which, and the census counts the wall's `Length` apart, 461 walls as built and 388 at permit, the maintainer, 2026-10-26),
walls' `Height` where nothing joins to or cuts them (RE-266),
walls' `GrossSideArea`,
`NetSideArea`,
`GrossVolume` and `NetVolume` where no step of their record cuts,
embeds or sweeps them and their body is their record's box cut back at their joins,
or they are joined to roofs or floors that lie clear of them and their record confirms their length or their centreline cut back at their joins is as long as their body (RE-270,
RE-290),
walls' `GrossFootprintArea` where nothing joins to or cuts them,
their body is their record's box and they are more than three times as long as thick (RE-292),
curtain walls' `Length`,
their curve's,
beams' `Length` where drawn along their location line,
pipe segments' `Length`,
pipes' `GrossCrossSectionArea` and `OuterSurfaceArea`,
stair flights' `Length` and `NetVolume`,
and the `Length` of columns joined to nothing and,
on those drawn as their type's hollow section,
their `CrossSectionArea`,
`OuterSurfaceArea` and `GrossVolume` (RE-247),
the `CrossSectionArea`,
`OuterSurfaceArea`,
`GrossVolume` and `NetVolume` of columns drawn as their record's box whose type is a plain rectangle,
where no slab,
wall or covering their pairs, clean or cut, name cuts them or crosses an end of them (RE-291, measured on Revit 2024),
floors' `GrossArea` (RE-249) and,
where a floor's type has one layer as deep as its extrusion,
its `Depth` and `GrossVolume`,
floors' `NetVolume` where their record holds no step that cuts them (RE-295, [#1016](https://github.com/DrunkOnJava/rvt-rs/issues/1016); what remains of the slabs, [#1208](https://github.com/DrunkOnJava/rvt-rs/issues/1208)),
and ceilings' `GrossArea` where they are level and their record holds no step that cuts them (RE-297; what remains of the coverings, [#1209](https://github.com/DrunkOnJava/rvt-rs/issues/1209)),
spaces' `Height` where the room's record holds its solid's faces (RE-255),
spaces' `NetVolume` where that solid is a prism of its outline (RE-296, [#1016](https://github.com/DrunkOnJava/rvt-rs/issues/1016); what remains of the spaces, [#1207](https://github.com/DrunkOnJava/rvt-rs/issues/1207)),
stair railings' `Length` where the path is one line along the stair's runs (RE-254)), 1 differs (the mullion whose `Span` differs) and 10,211 are missing (Measure run 41204218380, its 113 removed from `main` at dc9d73ca; the 527 quantities Revit's export writes on openings rvt-rs does not write, a door's further cuts and the cuts of curtain walls and cabinets, are counted apart, [#1165](https://github.com/DrunkOnJava/rvt-rs/issues/1165)), none written that Revit's export leaves out (the maintainer,
2026-10-26: a joined column's `Length` and a curtain panel's `Width` are left out,
since Revit's export gives none on 18 meshed columns and 3 panels that nothing tells apart;
and the maintainer, 2026-10-31, left out too a value on which Revit's exporters disagree: floors', ceilings' and covering walls' `Width`, which Revit 2024's exporter writes and later ones' do not, RE-294;
RE-291 now tells the columns apart,
[#1203](https://github.com/DrunkOnJava/rvt-rs/issues/1203)), and RE1's 94 IFC2X3 pipe and duct lengths all are written;
Core Interior's and RE1 Architecture's exports were made with Revit's "Export base quantities" off, so the census counts the quantities rvt-rs writes on them apart (2,430 and 85,
and RE1 Mechanical's and Plumbing's pipe areas,
12 and 126), and `rvt-ifc --no-base-quantities` leaves them out, base quantities being on by default as in each of Revit's built-in IFC4 setups (RE-242);
the census scores type objects' own sets too ([#1018](https://github.com/DrunkOnJava/rvt-rs/issues/1018)): of the 182 values Revit's export writes on the type objects rvt-rs writes on the office as built, 34 are written equal (`FireRating`, `LoadBearing` and stair types' dimensions, RE-286, [#1185](https://github.com/DrunkOnJava/rvt-rs/issues/1185)), and the other 148 are their `IsExternal` (135), which rvt-rs writes on no type object because Revit's newer exporters write none (the maintainer, 2026-10-31), the door types' `FireRating` (6) and `ThermalTransmittance` (1) and the family types' `Manufacturer` (6, RE-289) (four more,
the mullion types' `IsExternal`,
are counted apart,
RE-269).
Where Revit's export places a property outside the set IFC4's definitions give, W7 is measured with `rvt-ifc --revit-set-placement`, the default output following buildingSMART's definitions (RE-235);
a value Revit's export writes that is known to be wrong (`WalkingLineOffset`,
curtain mullions' `CrossSectionArea`,
door openings' `Width` and `Height`,
which it swaps,
and,
where it is the floor's own outline read as feet,
a floor's `Perimeter`,
which it writes in feet as though they were the file's unit) is left out, listed in RE-235 §5 and counted apart by both censuses (63,
10,003,
382 and 13 values on the office as built).
So are the curtain mullions' and panels' `IsExternal` in the office's exports and their mullion types', which the exporter that made them (24.0.5.432) sets from its own cache of element types, not from the model: the property census counts them apart, 10,267 values on the office as built and 10,304 at permit (RE-269;
the maintainer,
2026-10-30).
rvt-rs writes the members' by their curtain wall's value, as exporters from IFC_v24.2.20 on give it, and none on the member types (RE-288, [#1187](https://github.com/DrunkOnJava/rvt-rs/issues/1187);
the maintainer,
2026-10-30).
That differs from the office's on the members the cache turned false (8,579 of 10,003 mullions and 33 of 260 panels as built, RE-193), and the only model of a newer exporter with curtain members is RE1 Architecture (26.2), whose 12 members, 10 mullions and 2 panels, stand on one interior curtain wall and are false by either rule:
the exterior members' values are checked against revit-ifc's source only, since no newer exporter's model has an exterior curtain wall.
On the office as built, with `--revit-set-placement` (Measure run 41129228138, `main` at b09310a6), 455 of the 45,525 property values Revit's export writes on objects rvt-rs writes are missing and 1 differs;
without it, 1,421 are missing (Measure run 41129224669).
At building permit, 452 of 45,704, and 1,418 without it.
The census counts what the export's own property set file places apart, as out of scope (RE-239): 1,981 values as built and 1,907 at permit, the glazing set on elements that are not doors or windows and the space occupancy set, whose 55 `OccupancyNumber` values, written as `IfcInteger`, were 55 of the 56 that differed;
on the same exports, before that, 2,381 of 47,506 were missing as built with the option and 3,347 without it (Measure runs 41129221110 and 41129217725).
Before the curtain members' `IsExternal` was counted apart, 12,672 of 57,773 were missing with the option and 13,638 without it (Measure runs 41117828966 and 41117835321,
`main` at b9e0ce71).
The held-out and BIMcollab runs keep quantity counts only, so no property census of them is published.
Most instance and type parameters are not ([#35](https://github.com/DrunkOnJava/rvt-rs/issues/35),
[#155](https://github.com/DrunkOnJava/rvt-rs/issues/155),
the 0.12.0 milestone)
Open: type objects' values ([#1185](https://github.com/DrunkOnJava/rvt-rs/issues/1185)),
the BIM4LCA office's stair values ([#1186](https://github.com/DrunkOnJava/rvt-rs/issues/1186)),
curtain members' `IsExternal` by the host rule ([#1187](https://github.com/DrunkOnJava/rvt-rs/issues/1187)),
joined columns' `Length`, the round column and the meshed columns' values ([#1203](https://github.com/DrunkOnJava/rvt-rs/issues/1203)),
and the remaining values with no rule yet ([#1193](https://github.com/DrunkOnJava/rvt-rs/issues/1193)).

## Interfaces 1.0 stabilizes

| | Requirement | Bar | Measured now | Status |
|---|---|---|---|---|
| S1 | The stable surface is named ([#873](https://github.com/DrunkOnJava/rvt-rs/issues/873)) | `RELEASE.md` names the surfaces 1.0 commits to (the crate, the Python package, the CLIs and their JSON, the output formats, the wasm API) and those it does not | [`RELEASE.md`](../RELEASE.md#what-10-keeps-stable) names all five and what each leaves out. The crate keeps 27 modules stable; the other 67 are under `rvt::research`, behind the `unstable-research` feature, and `rvt::cli` is hidden. CI fails if a stable item hands out a research type (`unnameable_types` on the default-features build) | **passed** |
| S2 | Breaking changes are caught (#873) | Each stable surface has a check that fails on an unannounced breaking change | CI's `semver checks` job compares each pull request with its base and fails on a breaking change the title does not mark with `!`: the crate's public API (cargo-semver-checks), the Python type stubs (`tools/ci/python-api-compat.py`; stubtest holds the stubs to the built wheel), the 22 wasm bindings (`tools/ci/wasm-api.py`) and the CLIs' JSON schemas (`tools/ci/schema-compat.py`). All twelve CLIs that print JSON have a schema their output is validated against on Linux with the family corpus. The output formats: IfcOpenShell validates the exports, `rvt-gltf` writes a glTF 2.0 container, the schedules begin with the columns the user guide lists, and every writer gives the same bytes twice on the same file (`tests/reproducible_outputs.rs`, `rvt-ifc` with `SOURCE_DATE_EPOCH`) | **passed** |
| S3 | The Rust API is documented with runnable examples ([#872](https://github.com/DrunkOnJava/rvt-rs/issues/872)) | docs.rs shows the user entry points, each with an example CI runs | On `main`, these entry points have an example CI runs as a doctest against `corpus/tier1/` (`cargo test --profile ci --doc`, #900): `RevitFile::open`, `open_with_limits`, `open_bytes` and `open_bytes_with_limits`, `metadata::read_metadata`, `walker::iter_elements`, `RvtDocExporter::export_with_diagnostics`, `write_step`, `model_to_glb`, `elements_csv`, `sheet::render_plan_svg` and the writer's `copy_file`, and `write_with_patches` with `verify_patches_applied`. Other public items have none yet. `examples/` has three user examples, `basics_inspect`, `basics_elements` and `basics_export`, which CI runs (#904). docs.rs shows 0.11.0 (2026-10-30) with every one of them, as it showed 0.10.0, read from docs.rs's rustdoc JSON for the version | **passed** for 0.11.0 |
| S4 | The Python API is documented with runnable examples (#872) | Every Python block in [`python.md`](python.md) runs in CI | The 3 runnable blocks run in CI against `corpus/tier1/` (`tests/python/test_docs_python.py`, in the wheel job); the other 18 show a signature or a result shape and are fenced `text` | **passed** |

## Evidence from outside the project

| | Requirement | Bar | Measured now | Status |
|---|---|---|---|---|
| P1 | Practitioners' reports are part of the supported profile ([#874](https://github.com/DrunkOnJava/rvt-rs/issues/874)) | At least three BIM or AEC practitioners have run the documented workflows on their own models and reported, and each finding is an issue or a line in the supported profile | One practitioner report so far: hemanth1986's field data on 49 client project files ([#421 comment](https://github.com/DrunkOnJava/rvt-rs/issues/421#issuecomment-6462722973) and [#1075](https://github.com/DrunkOnJava/rvt-rs/issues/1075), 2026-10-28), after the request of 2026-10-25 ([Discussion #982](https://github.com/DrunkOnJava/rvt-rs/discussions/982), pinned, no reply yet). It found two defects, models with a stream over 256 MiB opening empty (#1075) and files carrying an older release's schema reading no elements (#421), and made no comparison with Revit's IFC export. Earlier outside activity: format research (Discussion #112, #421), code contributions (0.5.0, 0.6.0; #835 was found on a private 67 MB model in the viewer) and one show-and-tell (Discussion #393) | **failed** |

## Limitations

None. The maintainer decided on 2026-10-21 that 1.0 ships with no gaps or
limitations, so every row of the [supported profile](supported-profile.md)
has to read as exact before the tag. The work that blocks this is the set of
issues open under the 0.12.0 (geometry and element data) and 1.0.0
milestones.

Out of scope, and so not counted as limitations: Revit 2014 and earlier,
and what [`ROADMAP.md`](../ROADMAP.md#out-of-scope) rules out. That covers
Autodesk SDK internals, a Revit API-compatible surface, field-level writes
before [ADR-002](decisions/ADR-002-semantic-write-api-gate.md)'s gate opens,
and cloud worksharing, licensing and linked-model semantics. Also out of
scope, decided on 2026-10-26: a property value that a project's export-time
user-defined property set file maps into a set. The model stores the
parameter and the export setting that names the file, but not the file, so
which parameter feeds which set on which element is not in the input; [RE-239](../reports/element-framing/RE-239-export-file-values.md)
lists those values per reference model.

## Keeping this page

A pull request that moves a status updates this page and links the new
evidence. W6's and W7's values are written below the table, one sentence or
list item per line, so that pull requests changing different values do not
conflict. The monthly checklist in [`maintaining.md`](maintaining.md) compares
the page with the 1.0.0 milestone, and the release checklist
([`RELEASE.md`](../RELEASE.md#the-10-gate)) does not tag 1.0.0 while any
requirement is not passed.
