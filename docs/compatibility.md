# Compatibility matrix

What rvt-rs can and cannot do today, grouped by Revit release, file type, decoded element class, and IFC4 export detail. Every row here is checked against current source code. If a claim in this file disagrees with the source, the source wins and this file is wrong. For the shorter user-facing status summary, see [`docs/status.md`](status.md).

Scope sources:

- Version gates: [`src/basic_file_info.rs`](../src/basic_file_info.rs), [`src/streams.rs`](../src/streams.rs), [`src/walker.rs`](../src/walker.rs), [`tests/common/mod.rs`](../tests/common/mod.rs).
- File-type support: [`src/reader.rs`](../src/reader.rs), [`src/lib.rs`](../src/lib.rs), [`src/bin/*.rs`](../src/bin/).
- Decoder registry: [`src/elements/mod.rs`](../src/elements/mod.rs) `all_decoders()`.
- IFC emission: [`src/ifc/category_map.rs`](../src/ifc/category_map.rs), [`src/ifc/from_decoded.rs`](../src/ifc/from_decoded.rs), [`src/ifc/entities.rs`](../src/ifc/entities.rs).

## 1. Format versions supported

Integration corpus: Autodesk's public `rac_basic_sample_family` RFA, one file per Revit release 2016 through 2027, exercised by [`tests/samples.rs`](../tests/samples.rs) via `ALL_YEARS = [2016..=2027]` in [`tests/common/mod.rs`](../tests/common/mod.rs). Revit 2015 has no file in it; it is read from Autodesk's six 2015 sample projects, as 2027 projects are from its five 2027 ones (pinned in [`research/autodesk-sample-projects.tsv`](../research/autodesk-sample-projects.tsv) and read by Measure).

| Revit release | `BasicFileInfo` year | `Partitions/NN` marker | OLE + schema + metadata | Schema-directed walker (ADocument) |
|---|---|---|---|---|
| 2008 to 2014 | 2010 to 2014 stated; 2008 and 2009 the build year plus one (RE-259) | none in the corpus | partial, outside the 1.0 range: the container, streams, schema (no GUID lists before 2014) and `BasicFileInfo` on local closed-source files only (RE-259) | not measured |
| 2015 | 2015 | none in the corpus | partial: the container, streams, schema and `BasicFileInfo` on the sample projects; PartAtom and the preview are not measured | not measured |
| 2016 | 2016 | Partitions/58 | full | partial |
| 2017 | 2017 | Partitions/60 | full | partial |
| 2018 | 2018 | Partitions/61 | full | partial |
| 2019 | 2019 | Partitions/62 | full | partial |
| 2020 | 2020 | Partitions/63 | full | partial |
| 2021 | 2021 | Partitions/64 | full | partial |
| 2022 | 2022 | Partitions/65 | full | partial |
| 2023 | 2023 | Partitions/66 | full | partial |
| 2024 | 2024 | Partitions/67 | full | full |
| 2025 | 2025 | Partitions/68 | full | full |
| 2026 | 2026 | Partitions/69 | full | full |
| 2027 | 2027 | Partitions/70 | full; 325 schema fields carry a flag whose meaning is not measured (RE-264) | not measured |

Column definitions:

- **OLE + schema + metadata**: opens the CFB container, reads all 13 invariant streams, decompresses `Formats/Latest`, parses the whole page-stripped class schema (4,126 classes and 11,562 fields on the 2024 sample, all but 9 to 12 fields per release type-classified, #410; 2027's flagged fields by their base type, RE-264), extracts `BasicFileInfo` (version / build / GUID / path / locale), parses `PartAtom` XML, extracts the `RevitPreview4.0` PNG thumbnail, decodes `Contents`, and decodes the 167-byte `Global/PartitionTable` invariant. This column is `full` on every release from 2016 to 2027; on 2015 it is partial, as its row says.
- **Schema-directed walker**: `walker::read_adocument` locates the `ADocument` entry point in `Global/Latest` and walks all declared fields. Reliable on Revit 2024–2026. Returns `Ok(None)` on 2016–2023 when the entry-point detector can't find a high-confidence offset (see `walker.rs` §366–370 and `docs/rvt-phase4c-session-2026-04-19.md`). This is also the bridge the Layer-5b per-class decoders depend on at runtime against real files.

The same releases, with the support-matrix rows that rate their projects and family files, are the `releases` table of [`docs/support-matrix.json`](support-matrix.json), which `tests/support_matrix.rs` pins to `rvt::capability` and the family corpus.

On the 2015 and 2027 sample projects `rvt-info` reads every file, and their elements export experimentally ([`docs/status.md`](status.md), RE-179, RE-181); 2015 has no family in the corpus, so its family-corpus columns are not measured. Before 2015, a public 2014 family is measured, and Revit 2008 to 2014 files open on local closed-source files only, outside the 1.0 range (2015 to 2027) (RE-259, see the limitations below); after 2027 nothing is. The schema parser and compression path are likely to still work on near-future releases because the truncated-gzip and `Formats/Latest` framing have not changed in a decade, but nothing is claimed outside `[2015, 2027]` beyond that 2014 family.

## 2. File type support

All four extensions share the same Microsoft Compound File Binary container, the same stream layout, and the same schema. [`RevitFile::open`](../src/reader.rs) dispatches purely on the CFB magic `D0 CF 11 E0 A1 B1 1A E1`, not on the extension, so the same code path handles all four.

| Type | Extension | Read | IFC4 export |
|---|---|---|---|
| Project | `.rvt` | yes | partial: spatial tree/diagnostics; typed elements depend on decoded coverage |
| Family | `.rfa` | yes | a valid scaffold (IfcProject and spatial tree); family geometry is not exported and what `rvt-ifc` writes for a family is not measured |
| Project template | `.rte` | same code, not measured: no `.rte` is in any corpus | not measured |
| Family template | `.rft` | same code, not measured: no `.rft` is in any corpus | not measured |

Notes:

- The 11-release family corpus is `.rfa` only. Project files are the synthetic `corpus/tier1/` projects, the licensed models of [`reference-models.md`](reference-models.md) and Autodesk's sample projects; no `.rte` or `.rft` file is measured, and nothing is claimed about `.rvt`-specific streams that do not appear in families (worksharing metadata, linked-model tables, transmission data beyond the already-decoded `TransmissionData`).
- The CLI binaries (`rvt-info`, `rvt-analyze`, `rvt-schema`, `rvt-history`, `rvt-diff`, `rvt-corpus`, `rvt-dump`, `rvt-doc`, `rvt-ifc`) accept any of the four extensions without special-casing.

## 3. Element class decode coverage

**81 classes are registered today, each through `SimpleDecoder`.** Counted directly from [`src/elements/mod.rs`](../src/elements/mod.rs) `all_decoders()` (81 entries; nothing pins the number since #573 deleted the decoders' synthesized-byte tests, and the per-class decoder structs went with the unused typed views). Production `walker::iter_elements` prefers typed decoders for `MVP_TYPED_CLASSES`, merges ArcWall partition recovers, and merges partition MVP Level/Material / 2024 opening-index rows / 2024/2025 element-record Wall/Door/Window/Column/Floor/BuildingPad instances and the RE-33 product categories (furniture, casework, plumbing fixtures, specialty equipment, ceilings, curtain-wall mullions and panels, railings, wall sweeps, ducts, pipes and their fittings); other registered classes still use generic `decode_instance`. On the 76 files Measure reads (the family corpus 2014 to 2026, the reference models and Autodesk's sample projects 2015 to 2027), the `Global/Latest` schema path yields no production row: every element comes from `Partitions/*` records (#852, Measure run [39367864281](https://github.com/DrunkOnJava/rvt-rs/actions/runs/39367864281)). Grouped by domain below.

**Partial on real project files:** ArcWall, Level, Material, and 2024 ArcWallRectOpening index rows can surface from `Partitions/*` via fail-closed recovers. Schema-field Wall instances and typed Door/Window host binding from arbitrary projects remain unsolved (RE-19 negative on magnetar corpora). Floor/Room storey bind via Level ElementIds is idle (RE-20 negative). See [`docs/status.md`](status.md), `reports/element-framing/RE-19-door-window-wall-negative.md`, and `reports/element-framing/RE-20-level-elementid-negative.md`.

Each decoder takes schema-directed instance bytes (from `walker::decode_instance`) and refuses a schema entry for any other class. Typed Rust views over the decoded fields exist for `Level` ([`level.rs`](../src/elements/level.rs)), `Wall` and `WallType` ([`wall.rs`](../src/elements/wall.rs)), `Floor` ([`floor.rs`](../src/elements/floor.rs)), `Door` and `Window` ([`openings.rs`](../src/elements/openings.rs)), `Room`, `Area` and `Space` as one `Zone` ([`zones.rs`](../src/elements/zones.rs)) and `Material` ([`styling.rs`](../src/elements/styling.rs)); `ParameterValue` reads the `AProperty*` carriers ([`parameters.rs`](../src/elements/parameters.rs)). Every other registered class decodes to the generic field list with no typed view. No test exercises the views' projections on synthesized bytes: those suites were deleted in #573. Community-corpus open/scaffold checks (`docs/corpus-hunt-2026-04-21.md`) do not exercise these typed projections on real project instance data.

### Structural (7)

`Column`, `StructuralColumn`, `Beam`, `StructuralFraming`, `StructuralFoundation`, `Rebar`, `ReferencePlane`.

### Architectural — host + opening (14)

`Wall`, `WallType`, `Floor`, `FloorType`, `Roof`, `RoofType`, `Ceiling`, `CeilingType`, `Door`, `Window`, `CurtainWall`, `CurtainGrid`, `CurtainMullion`, `CurtainPanel`.

### Architectural — circulation + zoning (7)

`Stair`, `StairType`, `Railing`, `RailingType`, `Room`, `Area`, `Space`.

### Furnishings + generic (6)

`Furniture`, `FurnitureSystem`, `Casework`, `Mass`, `GenericModel`, `FamilyInstance`.

### Datum + site + levels (6)

`Level`, `Grid`, `GridType`, `BasePoint`, `SurveyPoint`, `ProjectPosition`.

### Styling (4)

`Material`, `FillPattern`, `LinePattern`, `LineStyle`.

### Project organization (7)

`Category`, `Subcategory`, `Phase`, `DesignOption`, `Workset`, `Revision`, `Symbol`.

### Drafting + views (4)

`View`, `Sheet`, `Schedule`, `ScheduleView`. (ReferencePlane is counted under Structural above because its primary consumer is layout, not drafting views.)

### Annotations (4)

`Dimension`, `Tag`, `TextNote`, `Annotation`.

### Parameters and value carriers (11)

- `ParameterElement`, `SharedParameter`.
- `AProperty`, `APropertyBoolean`, `APropertyInteger`, `APropertyEnum`, `APropertyDouble1`, `APropertyDouble3`, `APropertyFloat`, `APropertyFloat3`, `APropertyString` ([`parameters.rs`](../src/elements/parameters.rs)).
- The value-carrier classes are building blocks for parameter extraction: `ParameterValue` types a carrier's value and `StorageType` names its storage kind. Real-file parameter recovery still depends on the object-graph and partition-record linkage described in [`docs/status.md`](status.md). The Global/Latest AProperty carriers are still not joined to host elements (#35); `rvt-elements` and Python's `decoded_elements()` list each element's saved parameters from its native record instead, on Revit 2023 to 2027 files (RE-207), with built-in parameters carrying their id only (#255).

### MEP — mechanical / electrical / plumbing (11)

- Electrical: `ElectricalEquipment`, `ElectricalFixture`, `LightingFixture`, `LightingDevice`.
- Mechanical: `Duct`, `DuctFitting`, `MechanicalEquipment`.
- Plumbing: `Pipe`, `PipeFitting`, `PlumbingFixture`.
- Generic MEP: `SpecialtyEquipment`.

Group sum: 7 + 14 + 7 + 6 + 6 + 4 + 7 + 4 + 4 + 11 + 11 = **81**, matching `all_decoders().len()`.

## 4. IFC4 export coverage

The bridge [`ifc::build_ifc_model`](../src/ifc/from_decoded.rs) maps each decoded element to an `IfcEntity::BuildingElement`, and [`ifc::write_step`](../src/ifc/step_writer.rs) serialises the model as an IFC4 STEP file. A fixed spatial hierarchy (`IfcProject` → `IfcSite` → `IfcBuilding` → `IfcBuildingStorey`) always ships; per-element shape / material / property-set emission is per-class.

Column definitions:

- **IFC entity + placement**: a valid `IfcWall` / `IfcSlab` / … is constructed and wired to the storey via `IfcRelContainedInSpatialStructure`. Always yes for every Revit class that has a mapping in [`category_map.rs`](../src/ifc/category_map.rs); unknown classes fall back to `IfcBuildingElementProxy` rather than being dropped.
- **Extruded geometry**: an `IfcRectangleProfileDef` + `IfcExtrudedAreaSolid` + `IfcShapeRepresentation` + `IfcProductDefinitionShape` chain is emitted and attached to the element's `Representation` slot, iff the caller supplies an `Extrusion` via one of the helpers in [`from_decoded.rs`](../src/ifc/from_decoded.rs) (`wall_extrusion`, `slab_extrusion`). The helpers exist for the classes marked "yes"; they do not exist for the classes marked "no helper". The solid's `Position` is the project-level identity placement: the element translation is carried once, by the element's own `IfcLocalPlacement`, so a consumer composing `ObjectPlacement × Position` lands on the element instead of twice past it (#232).
- **Material association**: an `IfcMaterial` + `IfcRelAssociatesMaterial` is emitted iff the caller populates `BuilderOptions.materials` and sets `ElementInput.material_index`. For compound hosts, `ElementInput.material_layer_set_index` routes through an `IfcMaterialLayerSet` + `IfcMaterialLayerSetUsage` chain; for profile-driven members, `material_profile_set_index` routes through `IfcMaterialProfileSet` + `IfcMaterialProfileSetUsage`. Precedence order is `profile_set > layer_set > single material` — whichever is set wins.
- **Property set**: an `IfcPropertySet` + `IfcPropertySingleValue` + `IfcRelDefinesByProperties` is emitted for each `PropertySet` the caller puts in `ElementInput.property_sets`, typically from a dedicated helper (`wall_property_set`, `window_property_set`). Both helpers write a set of rvt-rs's naming (`RvtWall`, `RvtWindow`): IFC reserves the `Pset_` prefix and each standard set's properties for buildingSMART's definitions (#827). Quantity-typed properties route through `IfcQuantityArea` / `IfcQuantityVolume` / `IfcQuantityCount` / `IfcQuantityTime` / `IfcQuantityWeight`, with Imperial-to-SI conversion done at emission time (square-feet → m², cubic-feet → m³, pounds → kg).
- **Opening / fill**: for doors / windows, iff the caller sets `host_element_index` and `extrusion` on the opening, the writer emits `IfcOpeningElement` + `IfcRelVoidsElement` (host → opening) + `IfcRelFillsElement` (opening → door/window).

| Revit class | IFC entity | Placement + storey | Extrusion helper | Material assoc | Property set | Opening / fill |
|---|---|---|---|---|---|---|
| Level | IfcBuildingStorey | built-in | n/a | n/a | n/a | n/a |
| Grid | IfcGrid | yes | no helper | n/a | n/a | n/a |
| Wall | IfcWall (NOTDEFINED) | yes | yes (`wall_extrusion`) | yes | yes (`wall_property_set`) | n/a |
| CurtainWall | IfcCurtainWall | yes | no helper | no helper | no helper | n/a |
| Door | IfcDoor (DOOR) | yes | caller-supplied | yes | no helper | yes |
| Window | IfcWindow (WINDOW) | yes | caller-supplied | yes | yes (`window_property_set`) | yes |
| Floor | IfcSlab (FLOOR) | yes | yes (`slab_extrusion`) | yes | no helper | n/a |
| Roof | IfcRoof | yes | no helper | yes | no helper | n/a |
| Ceiling | IfcCovering (CEILING) | yes | no helper | yes | no helper | n/a |
| Stair | IfcStair | yes | no helper | yes | no helper | n/a |
| Railing | IfcRailing | yes | no helper | yes | no helper | n/a |
| Ramp | IfcRamp | yes | no helper | yes | no helper | n/a |
| Column | IfcColumn (COLUMN) | yes | no helper | yes | no helper | n/a |
| StructuralColumn | IfcColumn (COLUMN) | yes | no helper | yes | no helper | n/a |
| StructuralFraming | IfcBeam (BEAM) | yes | no helper | yes | no helper | n/a |
| StructuralFoundation | IfcFooting | yes | no helper | yes | no helper | n/a |
| Rebar | IfcReinforcingBar | yes | no helper | yes | no helper | n/a |
| Room | IfcSpace (SPACE) | yes | no helper | n/a | no helper | n/a |
| Area | IfcSpace | yes | no helper | n/a | no helper | n/a |
| Space | IfcSpace | yes | no helper | n/a | no helper | n/a |
| Furniture | IfcFurniture | yes | no helper | yes | no helper | n/a |
| FurnitureSystem | IfcFurniture (USERDEFINED) | yes | no helper | yes | no helper | n/a |
| Casework | IfcFurniture | yes | no helper | yes | no helper | n/a |
| LightingFixture | IfcLightFixture | yes | no helper | yes | no helper | n/a |
| LightingDevice | IfcLightFixture (USERDEFINED) | yes | no helper | yes | no helper | n/a |
| ElectricalEquipment | IfcElectricAppliance | yes | no helper | yes | no helper | n/a |
| ElectricalFixture | IfcLightFixture | yes | no helper | yes | no helper | n/a |
| MechanicalEquipment | IfcFlowController | yes | no helper | yes | no helper | n/a |
| Duct | IfcDuctSegment | yes | no helper | yes | no helper | n/a |
| DuctFitting | IfcDuctFitting | yes | no helper | yes | no helper | n/a |
| Pipe | IfcPipeSegment | yes | no helper | yes | no helper | n/a |
| PipeFitting | IfcPipeFitting | yes | no helper | yes | no helper | n/a |
| PlumbingFixture | IfcSanitaryTerminal | yes | no helper | yes | no helper | n/a |
| SpecialtyEquipment | IfcBuildingElementProxy (USERDEFINED) | yes | no helper | yes | no helper | n/a |
| Mass | IfcBuildingElementProxy (USERDEFINED) | yes | no helper | n/a | no helper | n/a |
| GenericModel | IfcBuildingElementProxy | yes | no helper | n/a | no helper | n/a |
| (unknown class) | IfcBuildingElementProxy | yes | n/a | n/a | n/a | n/a |

Notes:

- The `PredefinedType` values in the IFC-entity column are the mapping's, not the IFC4 enum's closest reading. Where a recorded edge shows what the authoring witness chose, the table follows it (#220): Revit's own export of the 2024 Core Interior project writes `.NOTDEFINED.` on all 360 walls, `.SPACE.` on all 116 spaces, `.DOOR.` on all 132 doors and `.WINDOW.` on all 6 windows, so those are the values rvt-rs writes. `Area` / `Space` / `GenericModel` stay `$` — no edge records the witness's answer for them, and inventing one would be a claim.
- "Yes (mapped; decoder pending)" rows: the class has a category-map entry and would round-trip through the bridge if a decoder existed, but no `ElementDecoder` for the class is registered in `all_decoders()` today, so the bridge never sees a decoded instance to map. See §3 MEP note.
- The units slot is populated via `BuilderOptions.units` (typically from `RvtDocExporter::build_unit_list`) and emits as `IfcUnitAssignment`. Classifications (OmniClass / Uniformat from `PartAtom`) emit as `IfcClassification` + `IfcClassificationReference`.
- The STEP writer is deterministic under `write_step_with_options(model, StepOptions { timestamp })` — a fixed `timestamp` produces byte-identical output across runs. Unicode escaping is ISO-10303-21-correct (BMP `\X2\HHHH\X0\`, supplementary `\X4\HHHHHHHH\X0\`, backslash doubled, control chars `\X\HH`).
- A committed sample IFC output lives at [`tests/fixtures/synthetic-project.ifc`](../tests/fixtures/synthetic-project.ifc), built from synthesized inputs by [`tests/ifc_synthetic_project.rs`](../tests/ifc_synthetic_project.rs) (`DUMP_IFC=1` regenerates it). No IFC schema check runs on the committed file itself: `tests/json_schema_contracts.rs` only compares it with the structural fixture through `rvt-ifc-compare` and validates that report against its schema. `tools/check-local.sh --ifcopenshell` runs the schema-arity check on fresh exports of the tier-1 projects, and CI validates fresh exports of the tier-1 and real projects ([`validation-evidence.md`](validation-evidence.md)).

## 5. Known limitations

- **Typed elements come from the partition element records only.** Revit 2023 to 2025 projects export typed elements from their element records, measured against Revit's own exports, and 2015 to 2022, 2026 and 2027 ones do experimentally ([`docs/status.md`](status.md)). No test exercises the 81 registered schema decoders or their typed views (the synthesized-byte suites were deleted in #573), and schema-field walls and opening-index doors and windows stay unsupported (RE-19).
- **A file that kept an older release's schema is read as that release, unmeasured here.** A project saved by Revit 2024 to 2027 can carry the schema of a release it passed through (#421). The readers take that release when the file's schema matches one measured (a 2024 to 2027 element-record marker, or a 2015 to 2023 release's class tags) and it is older than the saving release (`RevitFile::layout_release`); a schema that matches none still reads nothing. No local file carries an older release's schema, so the counts are a field report's, not this repository's.
- **No semantic write path for Revit files**. Stream-level patching ([`writer::write_with_patches`](../src/writer.rs)) can replace the bytes of a named stream, re-compress with truncated gzip, and re-embed into a byte-preserving sibling file (13/13 streams identical on the 2024 sample round-trip). Field-level semantic writes (edit a specific Wall's unconnected height and round-trip back to a Revit-openable `.rvt`) are not implemented.
- **Before Revit 2015, only local closed-source files, outside the 1.0 range (2015 to 2027)**. Revit 2015 is read from Autodesk's six 2015 sample projects, and their elements export experimentally (RE-181). Of Revit 2014, a public family file (Stertil, pinned in `research/public-families.tsv`) opens and its schema and ElemTable read (RE-147). Revit 2008 to 2013 files open, and 2011 to 2014 projects export their elements experimentally (RE-259), measured on local files that are closed source and cannot be published: the only same-model pair is a 2013 copy of Autodesk's `rac_advanced` sample against its 2015 copy. Revit 2008 to 2010 projects export their elements experimentally too (RE-260), with no copy of the same model to check them against.
- **No IFC2X3 or IFC4.3 export**. The STEP writer targets IFC4 only. The category map is structured to make a future IFC2X3 / IFC4.3 swap a table replacement, but it has not been swapped.
- **No encrypted or password-protected files**. Revit's "Protected" models use a wrapper format this library does not attempt to strip. CFB open will succeed on the outer container but inner streams will be gibberish.
- **No linked-model resolution**. `.rvt` projects can carry references to external `.rvt` / `.rfa` / `.rcp` / `.dwg` / `.ifc` files; this library reads the host file's CFB streams but does not follow or resolve links. Any linked-model metadata is exposed only as raw bytes on whichever stream carries it (`TransmissionData` on project files).
- **MEP geometry**. Pipes are drawn as their cylinders where both ends are proven (RE-131: all 69 of RE1's, Revit 2025, and all 12 of the BIM4LCA office's, Revit 2024, are Revit's, radius, axis ends and length to 0.00001 ft). Ducts, fittings and the other MEP elements keep their element record's bounding box, and the export diagnostics say so ([README, "What does not work yet"](../README.md#what-does-not-work-yet)).
- **Annotation geometry**. The 4 annotation classes (`Dimension`, `Tag`, `TextNote`, `Annotation`) decode as generic field lists, with no typed view, and do not yet render 2D annotation graphics into IFC presentation layers. Drawn sheets therefore export as `IfcAnnotation` shells without the witness-line / leader / dimension-string geometry that a plotted sheet would need.
- **Schema-directed walker is partial on 2016–2023**. See §1. The stream layout and entry-point heuristics differ pre-2024; `read_adocument` returns `Ok(None)` when it can't find a high-confidence offset instead of returning possibly-wrong fields.
- **Geometry is exact only where its bytes are decoded**. On Revit 2024 and 2025 projects, walls, floors, columns, roofs, ceilings, stairs, rooms and pipes are drawn from their records, measured against Revit's own exports ([README, "What rvt-rs reads from real projects"](../README.md#what-rvt-rs-reads-from-real-projects)). Elsewhere a body is the element record's bounding box, and the export diagnostics say so. Doors and windows are drawn from the mesh Revit saved for display by default on Revit 2023 to 2027, where the whole mesh decodes (`--no-saved-meshes` keeps their box); other loadable families are boxes unless `--saved-meshes` draws them the same way (RE-78). The Extrusion helpers in [`from_decoded.rs`](../src/ifc/from_decoded.rs) remain the bridge for caller-supplied dimensions.
- **Parcel of unparsed streams**. `Global/DocumentIncrementTable`, `Global/History`, `Global/ContentDocuments`, and `Partitions/NN` chunks beyond the first are enumerated and decompressed but their internal framing is only partially reverse-engineered (see recon report §Q6 and §Q7).
