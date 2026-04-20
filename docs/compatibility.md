# Compatibility matrix

What rvt-rs can and cannot do today, grouped by Revit release, file type, decoded element class, and IFC4 export detail. Every row here is checked against current source code. If a claim in this file disagrees with the source, the source wins and this file is wrong.

Scope sources:

- Version gates: [`src/basic_file_info.rs`](../src/basic_file_info.rs), [`src/streams.rs`](../src/streams.rs), [`src/walker.rs`](../src/walker.rs), [`tests/common/mod.rs`](../tests/common/mod.rs).
- File-type support: [`src/reader.rs`](../src/reader.rs), [`src/lib.rs`](../src/lib.rs), [`src/bin/*.rs`](../src/bin/).
- Decoder registry: [`src/elements/mod.rs`](../src/elements/mod.rs) `all_decoders()`.
- IFC emission: [`src/ifc/category_map.rs`](../src/ifc/category_map.rs), [`src/ifc/from_decoded.rs`](../src/ifc/from_decoded.rs), [`src/ifc/entities.rs`](../src/ifc/entities.rs).

## 1. Format versions supported

Integration corpus: Autodesk's public `rac_basic_sample_family` RFA, one file per Revit release 2016 through 2026, exercised by [`tests/samples.rs`](../tests/samples.rs) via `ALL_YEARS = [2016..=2026]` in [`tests/common/mod.rs`](../tests/common/mod.rs).

| Revit release | `BasicFileInfo` year | `Partitions/NN` marker | OLE + schema + metadata | Schema-directed walker (ADocument) |
|---|---|---|---|---|
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

Column definitions:

- **OLE + schema + metadata**: opens the CFB container, reads all 13 invariant streams, decompresses `Formats/Latest`, parses the full class schema (395 classes on the 2024 sample, 13,570 fields at 100% type classification across the 11-release corpus), extracts `BasicFileInfo` (version / build / GUID / path / locale), parses `PartAtom` XML, extracts the `RevitPreview4.0` PNG thumbnail, decodes `Contents`, and decodes the 167-byte `Global/PartitionTable` invariant. This row is `full` for every supported release.
- **Schema-directed walker**: `walker::read_adocument` locates the `ADocument` entry point in `Global/Latest` and walks all declared fields. Reliable on Revit 2024–2026. Returns `Ok(None)` on 2016–2023 when the entry-point detector can't find a high-confidence offset (see `walker.rs` §366–370 and `docs/rvt-phase4c-session-2026-04-19.md`). This is also the bridge the Layer-5b per-class decoders depend on at runtime against real files.

Before 2016 and after 2026: unverified. The schema parser and compression path are likely to still work on near-future releases because the truncated-gzip and `Formats/Latest` framing have not changed in a decade, but no corpus entry exists outside `[2016, 2026]`, so nothing is claimed.

## 2. File type support

All four extensions share the same Microsoft Compound File Binary container, the same stream layout, and the same schema. [`RevitFile::open`](../src/reader.rs) dispatches purely on the CFB magic `D0 CF 11 E0 A1 B1 1A E1`, not on the extension, so the same code path handles all four.

| Type | Extension | Read | IFC4 export |
|---|---|---|---|
| Project | `.rvt` | yes | yes (spatial tree + per-element entities; see §4) |
| Family | `.rfa` | yes | yes (same pipeline) |
| Project template | `.rte` | yes | yes (same pipeline) |
| Family template | `.rft` | yes | yes (same pipeline) |

Notes:

- The entire 11-release reference corpus is `.rfa` (family) files. `.rvt` / `.rte` / `.rft` read the same OLE container and are handled by the same code, but the corpus does not yet include project or template fixtures, so nothing is claimed about `.rvt`-specific streams that do not appear in families (worksharing metadata, linked-model tables, transmission data beyond the already-decoded `TransmissionData`).
- The CLI binaries (`rvt-info`, `rvt-analyze`, `rvt-schema`, `rvt-history`, `rvt-diff`, `rvt-corpus`, `rvt-dump`, `rvt-doc`, `rvt-ifc`) accept any of the four extensions without special-casing.

## 3. Element class decode coverage

**72 classes decoded today.** Counted directly from [`src/elements/mod.rs`](../src/elements/mod.rs) `all_decoders()` — the walker dispatch table built at runtime. Grouped by domain below.

Each decoder takes schema-directed instance bytes (from `walker::decode_instance`) and projects them into a typed Rust struct. Decoders are validated on synthesized schema+bytes fixtures; real-file corpus validation is tracked as Q-01 in the recon report.

### Structural (7)

- `Column` ([`structural.rs`](../src/elements/structural.rs))
- `StructuralColumn` ([`structural.rs`](../src/elements/structural.rs))
- `Beam` ([`structural.rs`](../src/elements/structural.rs))
- `StructuralFraming` ([`structural.rs`](../src/elements/structural.rs))
- `StructuralFoundation` ([`foundation_and_furnishings.rs`](../src/elements/foundation_and_furnishings.rs))
- `Rebar` ([`foundation_and_furnishings.rs`](../src/elements/foundation_and_furnishings.rs))
- `ReferencePlane` ([`reference_planes.rs`](../src/elements/reference_planes.rs))

### Architectural — host + opening (13)

- `Wall`, `WallType` ([`wall.rs`](../src/elements/wall.rs))
- `Floor`, `FloorType` ([`floor.rs`](../src/elements/floor.rs))
- `Roof`, `RoofType` ([`roof.rs`](../src/elements/roof.rs))
- `Ceiling`, `CeilingType` ([`ceiling.rs`](../src/elements/ceiling.rs))
- `Door`, `Window` ([`openings.rs`](../src/elements/openings.rs))
- `CurtainWall`, `CurtainGrid`, `CurtainMullion`, `CurtainPanel` ([`curtain_wall.rs`](../src/elements/curtain_wall.rs))

### Architectural — circulation + zoning (7)

- `Stair`, `StairType` ([`circulation.rs`](../src/elements/circulation.rs))
- `Railing`, `RailingType` ([`circulation.rs`](../src/elements/circulation.rs))
- `Room`, `Area`, `Space` ([`zones.rs`](../src/elements/zones.rs))

### Furnishings + generic (6)

- `Furniture`, `FurnitureSystem`, `Casework` ([`foundation_and_furnishings.rs`](../src/elements/foundation_and_furnishings.rs))
- `Mass`, `GenericModel` ([`generic.rs`](../src/elements/generic.rs))
- `FamilyInstance` ([`family.rs`](../src/elements/family.rs))

### Datum + site + levels (6)

- `Level` ([`level.rs`](../src/elements/level.rs))
- `Grid`, `GridType` ([`grid.rs`](../src/elements/grid.rs))
- `BasePoint`, `SurveyPoint`, `ProjectPosition` ([`reference_points.rs`](../src/elements/reference_points.rs))

### Styling (4)

- `Material` ([`styling.rs`](../src/elements/styling.rs))
- `FillPattern` ([`styling.rs`](../src/elements/styling.rs))
- `LinePattern` ([`styling.rs`](../src/elements/styling.rs))
- `LineStyle` ([`styling.rs`](../src/elements/styling.rs))

### Project organization (7)

- `Category`, `Subcategory` ([`category.rs`](../src/elements/category.rs))
- `Phase`, `DesignOption`, `Workset`, `Revision` ([`project_organization.rs`](../src/elements/project_organization.rs))
- `Symbol` ([`family.rs`](../src/elements/family.rs))

### Drafting + views (5)

- `View`, `Sheet`, `Schedule`, `ScheduleView` ([`drafting.rs`](../src/elements/drafting.rs))
- (ReferencePlane is counted under Structural above because its primary consumer is layout, not drafting views.)

### Annotations (4)

- `Dimension`, `Tag`, `TextNote`, `Annotation` ([`annotations.rs`](../src/elements/annotations.rs))
- Dimensions include linear / angular / radial / arc-length subtypes via the `DimensionKind` discriminator projected from the schema; tag orientation and horizontal alignment enums are exposed on `TagOrientation` and `HorizontalAlignment`.

### Parameters (2)

- `ParameterElement`, `SharedParameter` ([`parameters.rs`](../src/elements/parameters.rs))
- Both expose the `StorageType` enum (`None`, `Integer`, `Double`, `String`, `ElementId`, `Other`) so callers can route parameter values into property-set emission (see §4 IFC-31).

### MEP — mechanical / electrical / plumbing (11)

- Electrical: `ElectricalEquipment`, `ElectricalFixture`, `LightingFixture`, `LightingDevice` ([`mep.rs`](../src/elements/mep.rs))
- Mechanical: `Duct`, `DuctFitting`, `MechanicalEquipment` ([`mep.rs`](../src/elements/mep.rs))
- Plumbing: `Pipe`, `PipeFitting`, `PlumbingFixture` ([`mep.rs`](../src/elements/mep.rs))
- Generic MEP: `SpecialtyEquipment` ([`mep.rs`](../src/elements/mep.rs))
- All MEP instances project onto a shared `MepInstance` typed view with an optional `MepSystemClassification` (Supply / Return / Exhaust / …) so IFC distribution-system emission (IFC-10, future) can key off it without re-reading the schema.

Group sum: 7 + 13 + 7 + 6 + 6 + 4 + 7 + 5 + 4 + 2 + 11 = **72**, matching `all_decoders().len()`.

## 4. IFC4 export coverage

The bridge [`ifc::build_ifc_model`](../src/ifc/from_decoded.rs) maps each decoded element to an `IfcEntity::BuildingElement`, and [`ifc::write_step`](../src/ifc/step_writer.rs) serialises the model as an IFC4 STEP file. A fixed spatial hierarchy (`IfcProject` → `IfcSite` → `IfcBuilding` → `IfcBuildingStorey`) always ships; per-element shape / material / property-set emission is per-class.

Column definitions:

- **IFC entity + placement**: a valid `IfcWall` / `IfcSlab` / … is constructed and wired to the storey via `IfcRelContainedInSpatialStructure`. Always yes for every Revit class that has a mapping in [`category_map.rs`](../src/ifc/category_map.rs); unknown classes fall back to `IfcBuildingElementProxy` rather than being dropped.
- **Extruded geometry**: an `IfcRectangleProfileDef` + `IfcExtrudedAreaSolid` + `IfcShapeRepresentation` + `IfcProductDefinitionShape` chain is emitted and attached to the element's `Representation` slot, iff the caller supplies an `Extrusion` via one of the helpers in [`from_decoded.rs`](../src/ifc/from_decoded.rs) (`wall_extrusion`, `slab_extrusion`, `roof_extrusion`, `ceiling_extrusion`, `column_extrusion`). The helpers exist for the classes marked "yes"; they do not exist yet for the classes marked "helper not yet".
- **Material association**: an `IfcMaterial` + `IfcRelAssociatesMaterial` is emitted iff the caller populates `BuilderOptions.materials` (see [`materials_from_revit`](../src/ifc/from_decoded.rs)) and sets `ElementInput.material_index`. For compound hosts, `ElementInput.material_layer_set_index` routes through an `IfcMaterialLayerSet` + `IfcMaterialLayerSetUsage` chain (IFC-28 / IFC-29); for profile-driven members, `material_profile_set_index` routes through `IfcMaterialProfileSet` + `IfcMaterialProfileSetUsage` (IFC-30). Precedence order is `profile_set > layer_set > single material` — whichever is set wins.
- **Property set**: an `IfcPropertySet` + `IfcPropertySingleValue` + `IfcRelDefinesByProperties` (IFC-31 / IFC-33) is emitted iff the caller supplies a `PropertySet` via a dedicated helper (`wall_property_set`, `door_property_set`, `window_property_set`, `stair_property_set` exist today). Quantity-typed properties route through `IfcQuantityArea` / `IfcQuantityVolume` / `IfcQuantityCount` / `IfcQuantityTime` / `IfcQuantityWeight` (IFC-32), with Imperial-to-SI conversion done at emission time (square-feet → m², cubic-feet → m³, pounds → kg).
- **Opening / fill**: for doors / windows, iff the caller sets `host_element_index` and `extrusion` on the opening, the writer emits `IfcOpeningElement` + `IfcRelVoidsElement` (host → opening, IFC-37) + `IfcRelFillsElement` (opening → door/window, IFC-38).

| Revit class | IFC entity | Placement + storey | Extrusion helper | Material assoc | Property set | Opening / fill |
|---|---|---|---|---|---|---|
| Level | IfcBuildingStorey | built-in | n/a | n/a | n/a | n/a |
| Grid | IfcGrid | yes | helper not yet | n/a | n/a | n/a |
| Wall | IfcWall (STANDARD) | yes | yes (`wall_extrusion`) | yes | yes (`wall_property_set`) | n/a |
| CurtainWall | IfcCurtainWall | yes | helper not yet | helper not yet | helper not yet | n/a |
| Door | IfcDoor | yes | caller-supplied | yes | yes (`door_property_set`) | yes |
| Window | IfcWindow | yes | caller-supplied | yes | yes (`window_property_set`) | yes |
| Floor | IfcSlab (FLOOR) | yes | yes (`slab_extrusion`) | yes | helper not yet | n/a |
| Roof | IfcRoof | yes | yes (`roof_extrusion`) | yes | helper not yet | n/a |
| Ceiling | IfcCovering (CEILING) | yes | yes (`ceiling_extrusion`) | yes | helper not yet | n/a |
| Stair | IfcStair | yes | helper not yet | yes | yes (`stair_property_set`) | n/a |
| Railing | IfcRailing | yes | helper not yet | yes | helper not yet | n/a |
| Ramp | IfcRamp | yes | helper not yet | yes | helper not yet | n/a |
| Column | IfcColumn (COLUMN) | yes | yes (`column_extrusion`) | yes | helper not yet | n/a |
| StructuralColumn | IfcColumn (COLUMN) | yes | yes (`column_extrusion`) | yes | helper not yet | n/a |
| StructuralFraming | IfcBeam (BEAM) | yes | helper not yet | yes | helper not yet | n/a |
| StructuralFoundation | IfcFooting | yes | helper not yet | yes | helper not yet | n/a |
| Rebar | IfcReinforcingBar | yes | helper not yet | yes | helper not yet | n/a |
| Room | IfcSpace (INTERNAL) | yes | helper not yet | n/a | helper not yet | n/a |
| Area | IfcSpace | yes | helper not yet | n/a | helper not yet | n/a |
| Space | IfcSpace | yes | helper not yet | n/a | helper not yet | n/a |
| Furniture | IfcFurniture | yes | helper not yet | yes | helper not yet | n/a |
| FurnitureSystem | IfcFurniture (USERDEFINED) | yes | helper not yet | yes | helper not yet | n/a |
| Casework | IfcFurniture | yes | helper not yet | yes | helper not yet | n/a |
| LightingFixture | IfcLightFixture | yes | helper not yet | yes | helper not yet | n/a |
| LightingDevice | IfcLightFixture (USERDEFINED) | yes | helper not yet | yes | helper not yet | n/a |
| ElectricalEquipment | IfcElectricAppliance | yes | helper not yet | yes | helper not yet | n/a |
| ElectricalFixture | IfcLightFixture | yes | helper not yet | yes | helper not yet | n/a |
| MechanicalEquipment | IfcFlowController | yes | helper not yet | yes | helper not yet | n/a |
| Duct | IfcDuctSegment | yes | helper not yet | yes | helper not yet | n/a |
| DuctFitting | IfcDuctFitting | yes | helper not yet | yes | helper not yet | n/a |
| Pipe | IfcPipeSegment | yes | helper not yet | yes | helper not yet | n/a |
| PipeFitting | IfcPipeFitting | yes | helper not yet | yes | helper not yet | n/a |
| PlumbingFixture | IfcSanitaryTerminal | yes | helper not yet | yes | helper not yet | n/a |
| SpecialtyEquipment | IfcBuildingElementProxy (USERDEFINED) | yes | helper not yet | yes | helper not yet | n/a |
| Mass | IfcBuildingElementProxy (USERDEFINED) | yes | helper not yet | n/a | helper not yet | n/a |
| GenericModel | IfcBuildingElementProxy | yes | helper not yet | n/a | helper not yet | n/a |
| (unknown class) | IfcBuildingElementProxy | yes | n/a | n/a | n/a | n/a |

Notes:

- "Yes (mapped; decoder pending)" rows: the class has a category-map entry and would round-trip through the bridge if a decoder existed, but no `ElementDecoder` for the class is registered in `all_decoders()` today, so the bridge never sees a decoded instance to map. See §3 MEP note.
- The units slot is populated via `BuilderOptions.units` (typically from `RvtDocExporter::build_unit_list`) and emits as `IfcUnitAssignment`. Classifications (OmniClass / Uniformat from `PartAtom`) emit as `IfcClassification` + `IfcClassificationReference`.
- The STEP writer is deterministic under `write_step_with_options(model, StepOptions { timestamp })` — a fixed `timestamp` produces byte-identical output across runs. Unicode escaping is ISO-10303-21-correct (BMP `\X2\HHHH\X0\`, supplementary `\X4\HHHHHHHH\X0\`, backslash doubled, control chars `\X\HH`).
- A committed sample IFC output lives at [`tests/fixtures/synthetic-project.ifc`](../tests/fixtures/synthetic-project.ifc) and is exercised by [`tests/ifc_synthetic_project.rs`](../tests/ifc_synthetic_project.rs). It opens cleanly in BlenderBIM / IfcOpenShell spatial browsers.

## 5. Known limitations

- **No write path for Revit files**. Stream-level patching ([`writer::write_with_patches`](../src/writer.rs)) can replace the bytes of a named stream, re-compress with truncated gzip, and re-embed into a byte-preserving sibling file (13/13 streams identical on the 2024 sample round-trip). Field-level semantic writes (edit a specific Wall's unconnected height and round-trip back to a Revit-openable `.rvt`) are not implemented. Phase 7.
- **No format versions before Revit 2016**. Earlier Revit releases used a different compression + schema framing that this library has not been probed against. No claim is made about 2015-or-earlier.
- **No IFC2X3 or IFC4.3 export**. The STEP writer targets IFC4 only. The category map is structured to make a future IFC2X3 / IFC4.3 swap a table replacement, but it has not been swapped.
- **No encrypted or password-protected files**. Revit's "Protected" models use a wrapper format this library does not attempt to strip. CFB open will succeed on the outer container but inner streams will be gibberish.
- **No linked-model resolution**. `.rvt` projects can carry references to external `.rvt` / `.rfa` / `.rcp` / `.dwg` / `.ifc` files; this library reads the host file's CFB streams but does not follow or resolve links. Any linked-model metadata is exposed only as raw bytes on whichever stream carries it (`TransmissionData` on project files).
- **MEP geometry extraction**. The 11 MEP classes listed in §3 decode their schema metadata (class, name, family-instance linkage, system classification), but the geometric centerline / 3D path of ducts, pipes, conduits, and cable-trays is not yet extracted — the IFC rows above ship an `IfcDuctSegment` / `IfcPipeSegment` with placement and spatial-containment but no `IfcShapeRepresentation`. Routing-path recovery is tracked as a Phase-5 extension once the curve-graph decoder lands.
- **Annotation geometry**. The 4 annotation classes (`Dimension`, `Tag`, `TextNote`, `Annotation`) decode the annotation-element data (anchor points, text contents, orientation, witness-line references) but do not yet render 2D annotation graphics into IFC presentation layers. Drawn sheets therefore export as `IfcAnnotation` shells without the witness-line / leader / dimension-string geometry that a plotted sheet would need.
- **Schema-directed walker is partial on 2016–2023**. See §1. The stream layout and entry-point heuristics differ pre-2024; `read_adocument` returns `Ok(None)` when it can't find a high-confidence offset instead of returning possibly-wrong fields.
- **Geometry extraction is Phase 5**. The IFC export produces valid placement and extruded rectangular solids when the caller supplies dimensions, but the reader does not yet recover per-element location curves, profile shapes, or arbitrary brep geometry from the object graph. The Extrusion helpers in [`from_decoded.rs`](../src/ifc/from_decoded.rs) take caller-supplied dimensions for exactly this reason.
- **Parcel of unparsed streams**. `Global/DocumentIncrementTable`, `Global/History`, `Global/ContentDocuments`, and `Partitions/NN` chunks beyond the first are enumerated and decompressed but their internal framing is only partially reverse-engineered (see recon report §Q6 and §Q7).
