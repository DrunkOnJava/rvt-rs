# New open-source Revit → IFC4 exporter: rvt-rs

*Forum post draft for forums.buildingsmart.org (Software / Implementations).*
*Author: Griffin Long — https://github.com/DrunkOnJava/rvt-rs — Apache-2.0.*

Hello everyone.

I'm sharing **rvt-rs**, an Apache-2.0 Rust library that reads Autodesk Revit `.rvt` / `.rfa` / `.rte` / `.rft` files from Revit 2016–2026 and emits **IFC4** STEP (ISO-10303-21) without a Revit installation. On Revit 2023, 2024 and 2025 project files it exports the building's elements. I'd like this community's scrutiny on exactly what the emitted IFC looks like, because that's the part where I need IFC specialists more than I need Revit specialists.

The repository is public here:

- Source: https://github.com/DrunkOnJava/rvt-rs
- License: Apache-2.0
- Sample output: [`tests/fixtures/synthetic-project.ifc`](https://github.com/DrunkOnJava/rvt-rs/blob/main/tests/fixtures/synthetic-project.ifc)
- Compatibility page: [`docs/compatibility.md`](https://github.com/DrunkOnJava/rvt-rs/blob/main/docs/compatibility.md)
- What is read, measured against Revit's own exports: [`README.md`](https://github.com/DrunkOnJava/rvt-rs/blob/main/README.md#what-rvt-rs-reads-from-real-projects)

## Why another exporter

The de-facto production path from Revit to IFC is Autodesk's own `revit-ifc` (the open-source add-in behind the "Export IFC" dialog), and it is good. The catch is that it *requires Revit*. If you need to ingest `.rvt` content in a pipeline where Revit is not installed — a Linux server, a WASM browser app, a research tool that wants to enumerate thousands of historical project files, a CI check on a federated model — you are out of options that don't involve paying per-seat to keep a Windows VM running Revit on a schedule.

`rvt-rs` is a clean-room reader built from openly readable artefacts (the file itself, the `Formats/Latest` class schema that Revit writes into every file, public BuiltInParameter and BuiltInCategory values, and prior public reverse-engineering work). It runs anywhere Rust runs, including `wasm32-unknown-unknown`. The library crate is `#![forbid(unsafe_code)]` unconditionally, and it does not depend on any Autodesk SDK or any component of revit-ifc.

The goal is **not** to replace revit-ifc for production coordination exports. It is to make a second path exist at all, and to make it inspectable. Every capability claim in the repository is measured against Revit's own IFC (or VIM) export of the same file, and the tests fail when a claim overstates what is measured.

## What rvt-rs currently emits

The STEP writer (`src/ifc/step_writer.rs`) is deterministic under a fixed timestamp and produces the following IFC4 population from a decoded Revit file.

**Header + required framework:**

- `IfcProject` with name + description from PartAtom / BasicFileInfo
- `IfcPerson` + `IfcOrganization` + `IfcPersonAndOrganization` + `IfcApplication` + `IfcOwnerHistory`
- `IfcSIUnit` × 6 in one `IfcUnitAssignment`: the SI units every value is written in (metre, square metre, cubic metre, radian, kilogram, second)
- `IfcGeometricRepresentationContext` (precision `1.E-5`, 'Model', 3D)
- FILE_SCHEMA emits `IFC4`.

**Spatial structure:**

- `IfcSite` → `IfcBuilding` → `IfcBuildingStorey`, one storey per Revit `Level` that is a building story (Revit's Building Story setting, read from the Level), with the Level's name and elevation.
- `IfcRelAggregates` for project→site, site→building and building→storeys.

**Building elements:**

- `IfcWall`, `IfcSlab`, `IfcRoof`, `IfcCovering`, `IfcDoor`, `IfcWindow`, `IfcColumn`, `IfcBeam`, `IfcMember`, `IfcPlate`, `IfcCurtainWall`, `IfcRailing`, `IfcStair` / `IfcStairFlight`, `IfcRamp`, `IfcFooting`, `IfcFurniture`, `IfcSpace`, and the MEP terminals, segments and fittings. Each element comes from its own record in the file, under its Revit ElementId as `Tag`.
- Stairs and curtain walls aggregate their flights, landings, stringers, panels and mullions through `IfcRelAggregates`.
- An element whose category has no IFC4 mapping is an `IfcBuildingElementProxy`. Categories Revit's export leaves out (2D-only families, empty curtain panels, non-primary design options) are left out too, and the export diagnostics count them.
- `IfcDoor` / `IfcWindow` carry `OverallHeight` / `OverallWidth` from their type's rough opening where it is read.

**Types:**

- One `IfcWallType`, `IfcDoorType`, `IfcColumnType`, … per Revit type, named `Family:Type` with the type's ElementId as `Tag`, joined to its elements by `IfcRelDefinesByType`.

**Containment:**

- `IfcRelContainedInSpatialStructure` per storey. An element joins the storey of the Level its record names (or its base constraint, host or elevation where it names none); an element whose storey is not known is contained in the `IfcBuilding`, never in a guessed storey.

**Geometry:**

- Bodies are `IfcExtrudedAreaSolid` over `IfcRectangleProfileDef`, `IfcArbitraryClosedProfileDef` (slab, roof, ceiling and room outlines from their sketches or stored solids) or a parametric section (`IfcIShapeProfileDef` and friends for structural members), placed by each element's own `IfcLocalPlacement`.
- Walls are drawn along their location line at their type's thickness, trimmed at their joins. Tapered walls are drawn with their tapered cross-section.
- Where an element's geometry is not decoded, its body is its recorded bounding box, and a `BodySource` property says which.

**Openings:**

- A door or window cuts its host through `IfcOpeningElement` + `IfcRelVoidsElement` + `IfcRelFillsElement`, the opening sized from the type's rough opening where it is read. Shafts cut the floors, roofs and ceilings they pass through.

**Materials:**

- `IfcMaterial` with its Revit name and shading colour (`IfcColourRgb` + `IfcSurfaceStyleRendering` + `IfcSurfaceStyle` + `IfcStyledItem`, transparency included).
- Layered walls, floors, roofs and ceilings: `IfcMaterialLayerSetUsage` over an `IfcMaterialLayerSet` with each layer's material and thickness.
- Structural members drawn as their section: `IfcMaterialProfileSetUsage`.
- Family instances: `IfcMaterialConstituentSet` of the materials their type's geometry uses.

**Property sets:**

- `IfcPropertySet` with typed `IfcPropertySingleValue` entries (`IfcText`, `IfcInteger`, `IfcReal`, `IfcBoolean`, `IfcLengthMeasure`, `IfcPlaneAngleMeasure`), linked via `IfcRelDefinesByProperties`: the buildingSMART `Pset_WallCommon`, `Pset_DoorCommon`, `Pset_WindowCommon`, `Pset_StairCommon`, `Pset_StairFlightCommon` and so on, plus an `RvtElementRecordGeometry` set that records where each value came from.

**Classifications:**

- OmniClass + Uniformat codes from Revit's `PartAtom` surface as `IfcClassification` + `IfcClassificationReference`, bound via `IfcRelAssociatesClassification`.

**GlobalIds:**

- Elements, rooms, storeys and types get the GlobalId Revit's own exporter gives them, rebuilt from the file (the creating editing episode's GUID XORed with the ElementId). Every other entity gets one derived from the document's GUID and the entity's own identity, never its position in the file, so re-exports keep their GlobalIds.

**Unicode:**

- ISO-10303-21 string escape: BMP non-ASCII is `\X2\HHHH\X0\`, supplementary plane is `\X4\HHHHHHHH\X0\`, ASCII control bytes are `\X\HH`, apostrophes and backslashes are doubled.

## Verification

- **A synthetic fixture.** [`tests/fixtures/synthetic-project.ifc`](https://github.com/DrunkOnJava/rvt-rs/blob/main/tests/fixtures/synthetic-project.ifc) is a committed one-room building: 3 storeys, 4 walls, a slab, a door cut into the south wall, 2 windows, a stair, and 2 styled materials. Its entity counts are pinned by `tests/ifc_synthetic_project.rs`.
- **IfcOpenShell in CI.** CI opens that fixture and a real project's export with IfcOpenShell on every PR. It also checks every instance's attribute count and `PredefinedType` against the IFC4 schema (`tools/ci/ifc_schema_arity.py`).
- **Revit's own exports.** On real projects the export is scored against Revit's own IFC export of the same file: element sets, storeys, names, types, materials, layer sets, GlobalIds and bodies, category by category. Where the two agree across IfcOpenShell and IFClite, that is recorded as a witness verdict. The README lists every measured result, the misses included.

## Honest gaps worth calling out to this audience specifically

This is the part I most want forum scrutiny on.

- **Family geometry is not drawn.** A door, window, furniture item, fixture or piece of equipment is drawn as the box of its record, turned to its placement, not as its family's geometry: no `IfcRepresentationMap` / `IfcMappedItem` from family definitions yet. Steel columns and beams are the exception, drawn as their type's section. Doors and windows still cut correctly sized openings.
- **Bounding boxes where geometry is not decoded.** Roofs beyond flat and single-slope ones, walls with edited profiles, and anything else not yet decoded keep their bounding box. The `BodySource` property says so per element.
- **`IfcOpeningElement`, not `IfcOpeningStandardCase`.** I emit the plain `IfcOpeningElement` even when the standard-case constraints are satisfied (IFC-17). Which consumers care?
- **Property-set naming.** I emit buildingSMART's `Pset_<Class>Common` names, not Autodesk's `Pset_RevitType_{ClassName}` convention, and most Revit parameters are not read yet. Is aligning with Autodesk's convention the right call for interoperability, or should I stick to the canonical names?
- **Type products carry no property sets or representation maps.** Revit's own exporter often splits one Revit type into several type objects (one per column, for instance); rvt-rs writes one per type. Do consumers need the split?
- **No Boolean clipping in real exports.** The writer can emit `IfcBooleanResult`, but no body read from a real file uses it yet, so clipped hosted families are drawn unclipped.
- **Releases.**
  - Revit 2016–2022 project files export only the spatial scaffold and diagnostics.
  - Revit 2026 elements are not decoded.
  - Revit 2023 elements carry types, materials and storeys, but their bodies are bounding boxes.

## What I'd most value from this forum

Four things specifically:

1. **IFC4 conformance critique.** Are any of the gaps above going to break specific MVDs that matter (Reference View, Design Transfer View)? Are any of the entity populations I *do* emit non-conformant in ways I haven't spotted? The synthetic fixture is small enough to review end-to-end.
2. **Validation tooling.** CI runs IfcOpenShell and a schema-arity check today. Is there a community-sanctioned validator I should add? Is an offline-usable version of the buildingSMART Validation Service available, or is it hosted-only?
3. **Pset naming convention.** Is there consensus about whether IFC4 exporters should prefer buildingSMART-canonical `Pset_<Class>Common` names, Autodesk's `Pset_RevitType_{ClassName}` convention, or both? I want whatever maximises the chance that downstream coordination tools (Solibri, Navisworks, BIMcollab, Revizto) find the values where they expect them.
4. **Anything I've said above that is flat wrong.** This is a 0.x release, so breaking changes are still possible. I'd rather hear "the `IfcGeometricRepresentationContext` precision of `1.E-5` is too tight for building-scale" (or whatever) now than after five more minor releases.

The per-category results, the Revit exports they are measured against, and the compatibility matrix are all in the repo. Issues are open. Apache-2.0 means vendors are welcome to embed it directly.

Thanks for reading. Critique gratefully received.

— Griffin Long

---

## Appendix: where to look in the source tree

For anyone who wants to read the IFC emission code directly:

- `src/ifc/step_writer.rs` — the ISO-10303-21 serialiser. Pure string emission, deterministic.
- `src/ifc/mod.rs` — `IfcModel`, `Storey`, `MaterialInfo`, and the `RvtDocExporter` that builds the model from a Revit file.
- `src/ifc/export_content.rs` — decoded Revit elements → IFC building elements, their bodies and property sets.
- `src/ifc/entities.rs` — the in-memory `IfcEntity` variants, `PropertyValue` with its `to_step()` mapping, and the profile, extrusion and material set structs.
- `src/ifc/category_map.rs` — Revit class name → IFC4 type + predefined-type mapping.
- `tests/ifc_synthetic_project.rs` — the end-to-end test that pins the synthetic fixture's entity counts.
- `tests/fixtures/synthetic-project.ifc` — the committed sample output.

## Appendix: version + license

- Version: 0.3.x (pre-1.0; breaking changes possible).
- License: Apache-2.0 (see [`LICENSE`](https://github.com/DrunkOnJava/rvt-rs/blob/main/LICENSE) and [`NOTICE`](https://github.com/DrunkOnJava/rvt-rs/blob/main/NOTICE)).
- Clean-room provenance: [`CLEANROOM.md`](https://github.com/DrunkOnJava/rvt-rs/blob/main/CLEANROOM.md).
- Security posture: [`SECURITY.md`](https://github.com/DrunkOnJava/rvt-rs/blob/main/SECURITY.md), [`THREAT_MODEL.md`](https://github.com/DrunkOnJava/rvt-rs/blob/main/THREAT_MODEL.md).
