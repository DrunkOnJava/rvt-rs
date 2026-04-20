# New open-source Revit → IFC4 exporter: rvt-rs

*Forum post draft for forums.buildingsmart.org (Software / Implementations).*
*Author: Griffin Long — https://github.com/DrunkOnJava/rvt-rs — Apache-2.0.*

Hello everyone.

I'm sharing an early release of **rvt-rs**, an Apache-2.0 Rust library that reads Autodesk Revit `.rvt` / `.rfa` / `.rte` / `.rft` files from Revit 2016–2026 and emits **IFC4** STEP (ISO-10303-21). The project is aimed at filling one specific gap, and I'd like this community's scrutiny on exactly what the emitted IFC looks like, because that's the part where I need IFC specialists more than I need Revit specialists.

The repository is public here:

- Source: https://github.com/DrunkOnJava/rvt-rs
- License: Apache-2.0
- Sample output: [`tests/fixtures/synthetic-project.ifc`](https://github.com/DrunkOnJava/rvt-rs/blob/main/tests/fixtures/synthetic-project.ifc)
- Compatibility page: [`docs/compatibility.md`](https://github.com/DrunkOnJava/rvt-rs/blob/main/docs/compatibility.md)

## Why another exporter

The de-facto production path from Revit to IFC is Autodesk's own `revit-ifc` (the open-source add-in behind the "Export IFC" dialog), and it is good. The catch is that it *requires Revit*. If you need to ingest `.rvt` content in a pipeline where Revit is not installed — a Linux server, a WASM browser app, a research tool that wants to enumerate thousands of historical project files, a CI check on a federated model — you are out of options that don't involve paying per-seat to keep a Windows VM running Revit on a schedule.

`rvt-rs` is a clean-room reader built from openly readable artefacts (the file itself, Autodesk's own published schema diagnostics, the `Formats/Latest` class schema that Revit emits into every file it writes). It runs anywhere Rust runs, including `wasm32-unknown-unknown`. The project is under `#![deny(unsafe_code)]` on the lib crate and does not depend on any Autodesk SDK or any component of revit-ifc.

The goal is **not** to replace revit-ifc for production coordination exports. It is to make a second path exist at all, and to make it inspectable.

## What rvt-rs currently emits

The STEP writer (`src/ifc/step_writer.rs`, ~1400 lines, fully tested, deterministic under a fixed `StepOptions::timestamp`) produces the following IFC4 entity population from a decoded Revit file. Every line below is grounded in code I can point at.

**Header + required framework:**

- `IfcProject` with name + description from PartAtom / BasicFileInfo
- `IfcPerson` + `IfcOrganization` + `IfcPersonAndOrganization` + `IfcApplication` + `IfcOwnerHistory`
- `IfcSIUnit` × 4 (LENGTHUNIT `.MILLI. .METRE.`, AREAUNIT `.SQUARE_METRE.`, VOLUMEUNIT `.CUBIC_METRE.`, PLANEANGLEUNIT `.RADIAN.`) bundled in one `IfcUnitAssignment`
- `IfcCartesianPoint` + `IfcDirection` × 2 + `IfcAxis2Placement3D` + `IfcGeometricRepresentationContext` (precision `1.E-5`, 'Model', 3D)
- FILE_SCHEMA emits `IFC4` (not IFC2x3, not IFC4.3).

**Spatial structure:**

- `IfcSite` → `IfcBuilding` → `IfcBuildingStorey` — always present; one storey per Revit `Level` element where `is_building_story = true`, using each level's actual name and elevation (feet → metres conversion applied at emit time).
- `IfcLocalPlacement` per spatial element, chained via `PlacementRelTo` so the coordinate frames compose.
- `IfcRelAggregates` for project→site, site→building, and building→storeys (last one bundles all storeys into a single relationship).

**Building elements** (spec-valid IFC4 constructors, per-class):

- `IfcWall`, `IfcSlab`, `IfcRoof`, `IfcDoor`, `IfcWindow`, `IfcColumn`, `IfcBeam`, `IfcStair`, `IfcCovering`, `IfcRailing`, `IfcFurniture`, `IfcRamp`, `IfcFooting`, `IfcReinforcingBar`, `IfcSpace`
- Unknown / not-yet-decoded classes fall back to `IfcBuildingElementProxy` rather than being silently dropped.
- `IfcDoor` and `IfcWindow` are emitted with their 10-field form (extra `OverallHeight` / `OverallWidth` slots present as `$,$` until geometry lands them).
- Each element gets its own `IfcLocalPlacement` + `IfcAxis2Placement3D`, with a per-element `IfcCartesianPoint` when a location is known (feet → metres at the emit boundary) and a per-element `IfcDirection` when a yaw rotation is present.

**Containment:**

- `IfcRelContainedInSpatialStructure`, one per non-empty storey, bundles that storey's elements. Elements without an explicit `storey_index` land in `storeys[0]` rather than being dropped.

**Geometry (rectangular extrusions only, today):**

- For any element the caller supplies an `Extrusion { width_feet, depth_feet, height_feet }` for, the writer emits the full chain: `IfcRectangleProfileDef (.AREA.)` → `IfcExtrudedAreaSolid` → `IfcShapeRepresentation (RepresentationIdentifier='Body', RepresentationType='SweptSolid')` → `IfcProductDefinitionShape`, wired to the element's `Representation` slot.
- Swept direction is the shared `+Z` `IfcDirection`. Profile placement uses a fresh 2D `IfcAxis2Placement2D` per extrusion so byte-diff tooling stays clean.

**Openings in walls (doors + windows):**

- When an element has both an extrusion and a `host_element_index` pointing at its parent wall, the writer emits an `IfcOpeningElement` (same extrusion shape, reusing the door/window placement), an `IfcRelVoidsElement(host_wall, opening)` (so the wall is subtracted), and an `IfcRelFillsElement(opening, door/window)` (so the door fills the hole).
- The opening carries `PredefinedType = .OPENING.` (IFC4's `IfcOpeningElementTypeEnum`).

**Materials:**

- `IfcMaterial` with just a name (the IFC4 minimum).
- When a material has a color, the writer also emits `IfcColourRgb` + `IfcSurfaceStyleRendering (ReflectanceMethod=.FLAT.)` + `IfcSurfaceStyle (Side=.BOTH.)` + `IfcPresentationStyleAssignment` + `IfcStyledItem`. Transparency ships on the rendering entity in the `0..1` range.
- One `IfcRelAssociatesMaterial` per material, bundling all elements that reference that material index — not N rels per material.

**Property sets:**

- `IfcPropertySet` with `IfcPropertySingleValue` entries, linked via `IfcRelDefinesByProperties`.
- Typed values properly: `IfcText`, `IfcInteger`, `IfcReal`, `IfcBoolean`, `IfcLengthMeasure` (feet → metres at emit), `IfcPlaneAngleMeasure`. The enum drives STEP-level emission from `entities::PropertyValue::to_step()`.
- Today we ship typed `Pset_WallCommon` / `Pset_DoorCommon` / `Pset_WindowCommon` / `Pset_StairCommon` helpers; see the "honest gaps" section about naming.

**Classifications:**

- OmniClass + Uniformat codes from Revit's `PartAtom` surface as `IfcClassification` + `IfcClassificationReference`, bound to the project via `IfcRelAssociatesClassification`. One reference per code; per-reference association rel.

**GUIDs:**

- 22-character strings in the IFC-GUID alphabet (`0-9A-Za-z_$`). Deterministic per entity index so the STEP text diffs cleanly across runs. Uniqueness is asserted in the test suite.

**Unicode:**

- ISO-10303-21 string escape is correct: BMP non-ASCII is `\X2\HHHH\X0\`, supplementary plane (emoji, rare scripts) is `\X4\HHHHHHHH\X0\`, ASCII control bytes are `\X\HH`, apostrophes are doubled, backslashes are doubled. Tested against accented names, CJK, and emoji.

## Verification

The synthetic fixture at [`tests/fixtures/synthetic-project.ifc`](https://github.com/DrunkOnJava/rvt-rs/blob/main/tests/fixtures/synthetic-project.ifc) is a committed 20ft × 10ft one-room building with:

- 1 `IfcSite` / 1 `IfcBuilding` / 3 `IfcBuildingStorey` (Ground 0 ft, Second 10 ft, Roof Deck 20 ft) — elevations round-trip to `3.048` and `6.096` metres
- 4 `IfcWall` (with per-element placements, two of them rotated π/2 about `+Z`)
- 1 `IfcSlab`, 1 `IfcDoor`, 2 `IfcWindow`, 1 `IfcStair`, 1 `IfcBuildingElementProxy` (unknown-class fallback)
- 2 `IfcMaterial` with full surface-style chain (concrete, tinted glass with `Transparency = 0.6`)
- 1 `IfcOpeningElement` + `IfcRelVoidsElement` + `IfcRelFillsElement` for the front door cut out of the south wall
- `Pset_WallCommon` + `Pset_WindowCommon` carrying real typed values (`IfcLengthMeasure(0.762)` from a 2.5 ft sill height)

The integration test `tests/ifc_synthetic_project.rs` pins exact entity counts for every category above, plus a byte-stability test under a fixed timestamp so identical `(model, options)` inputs produce identical STEP bytes. The fixture opens cleanly in BlenderBIM: spatial browser shows the project → site → building → 3 storeys tree, the 4 walls + slab + door + windows render as 3D volumes, and the door appears as a real opening in the south wall thanks to the void/fill chain.

Compilation of the fixture through IfcOpenShell's `ifcopenshell.open()` succeeds locally. CI-side validation via IfcOpenShell is tracked as issue **IFC-41** and is *not yet enabled* — I'd rather say that out loud than claim green CI that doesn't exist yet.

## Honest gaps worth calling out to this audience specifically

This is the part I most want forum scrutiny on, because the gaps are all specifically IFC4-schema-shaped and the people here will know which of them are going to bite which MVD.

- **Profiles are rectangular only.** Every extruded solid uses `IfcRectangleProfileDef`. No `IfcCircleProfileDef`, no `IfcArbitraryClosedProfileDef`, no composite. That means curved-door / curved-window / oval-window families round-trip as their bounding rectangle. I know this is wrong and I want it fixed per-element, not globally faked.
- **No `IfcMaterialLayerSet` yet.** Walls are emitted with `IfcRelAssociatesMaterial` pointing at a single flat `IfcMaterial`, which is valid IFC4 but loses the Revit wall-type's actual layer stack. Tracked as **IFC-28**. Structural members similarly don't emit `IfcMaterialProfileSet` (tracked as **IFC-30**).
- **`IfcOpeningElement`, not `IfcOpeningStandardCase`.** IFC4 introduced `IfcOpeningStandardCase` for the common "straight rectangular cut through a wall with both sides parallel" geometry. I emit the plain `IfcOpeningElement` even when the standard-case constraints are satisfied. Tracked as **IFC-17**. My understanding is that some importers treat this leniently but validators will flag it; I'd appreciate feedback on which consumers care.
- **Units are not yet read from the Revit file.** Revit stores units as `autodesk.unit.*` identifiers (e.g. `autodesk.unit.unit:millimeters-1.0.1`). I parse them into an `UnitAssignment { forge_identifier, ifc_mapping }` struct, but the writer today hard-codes the `IfcUnitAssignment` to SI millimetres + square metres + cubic metres + radians regardless of what the source file uses. If the source file is in feet, the emitted file is labelled as metres and the numeric values have been converted, so the file *opens* correctly, but the unit semantics are lost. Full per-file unit read-back + mapping to `IfcSIUnit` / `IfcConversionBasedUnit` is tracked as **IFC-39** and **IFC-40**.
- **Property-set naming does not yet match Autodesk's `Pset_RevitType_{ClassName}` convention.** I emit `Pset_WallCommon` / `Pset_DoorCommon` / etc. — which are buildingSMART-standard names, not Revit-exporter-matching ones. That means round-tripping `rvt-rs`-produced IFC back through Revit's IFC importer will not produce the same parameter shape that revit-ifc would have, even for elements where the field set is identical. I'd like input here — is aligning with Autodesk's convention the right call for interoperability, or should I stick to the buildingSMART canonical `Pset_<Class>Common` names?
- **`IfcTypeProduct` is not emitted.** Revit `Symbol` / `WallType` / `FloorType` / … decode fine at the Layer-5b level (they're registered decoders in `src/elements/*`), but they currently only flow through as flat properties on their instances. No `IfcWallType`, `IfcDoorType`, `IfcTypeProduct` entities are emitted and no `IfcRelDefinesByType` relationships exist. Tracked as part of **IFC-21** / **IFC-22**. This is a real hole and I want to close it next; I'd welcome opinions on whether to emit bare type products first and add `IfcRepresentationMap` later, or do both at once.
- **No `IfcBooleanResult`.** The only volumetric subtraction we perform is the `IfcRelVoidsElement` / opening chain. No arbitrary `IfcBooleanClippingResult` or `IfcBooleanResult` is emitted, which means complex hosted families that rely on boolean clips of the instance solid against a reference surface do not round-trip. Tracked as **IFC-19**.
- **Partial schema-directed walker on Revit 2016–2023.** This is not strictly an IFC4 issue but it shapes what IFC you can actually get out: `walker::read_adocument` is reliable on 2024–2026 and returns `Ok(None)` on 2016–2023 rather than guess. So on 2016–2023 the IFC output today is the document-level scaffold (project + site + building + default storey + classifications + units + thumbnail-sourced metadata) without the full decoded element list. Full walker coverage across all 11 releases is tracked as **L5B-11** / **L5B-59**.
- **No geometry beyond rectangular extrusions as extracted from the file.** The `Extrusion` values in the synthetic fixture are *caller-supplied* — they come from tests and from helper functions that take a length or a thickness as a parameter. Extracting per-element location curves, profile shapes, and arbitrary brep faces from the Revit object graph is Phase 5 work. What ships today is the bridge: once the reader recovers a profile, it flows through `IfcArbitraryClosedProfileDef` + `IfcExtrudedAreaSolid` etc. without touching the writer.

I've tried to be audit-honest about all of this in [`docs/compatibility.md`](https://github.com/DrunkOnJava/rvt-rs/blob/main/docs/compatibility.md) — if anything there disagrees with the source, the source wins and the doc is wrong. Bug reports against the compat page are as welcome as bug reports against the code.

## What I'd most value from this forum

Four things specifically:

1. **IFC4 conformance critique.** Are any of the 8 gaps above going to break specific MVDs that matter (Coordination View 2.0, Reference View, Design Transfer View)? Are any of the entity populations I *do* emit non-conformant in ways I haven't spotted? The synthetic fixture is small enough to review end-to-end.
2. **Validation tooling suggestions.** My plan for **IFC-41** is to run IfcOpenShell's validator in CI on every PR that touches `src/ifc/`. Is there a better community-sanctioned validator I should use instead, or in addition? I've seen references to the buildingSMART Validation Service — is an offline-usable version available, or is it hosted-only?
3. **Pset naming convention.** Is there consensus in this community about whether IFC4 exporters should prefer buildingSMART-canonical `Pset_<Class>Common` names, Autodesk's `Pset_RevitType_{ClassName}` convention, or emit both? I want whatever maximises the chance that downstream coordination tools (Solibri, Navisworks, BIMcollab, Revizto) find the values where they expect them.
4. **Anything I've said above that is flat wrong.** This is a 0.1.x release. I'd rather hear "the `IfcGeometricRepresentationContext` precision of `1.E-5` is too tight for building-scale" (or whatever) now than after five more minor releases.

Sample files, an integration-test walkthrough, the 54 currently-decoded Revit classes, and the per-class IFC coverage matrix are all in the repo. Issues are open. Apache-2.0 means vendors are welcome to embed it directly.

Thanks for reading. Critique gratefully received.

— Griffin Long

---

## Appendix: where to look in the source tree

For anyone who wants to read the IFC emission code directly:

- `src/ifc/step_writer.rs` — the ISO-10303-21 serialiser. Pure string emission, deterministic.
- `src/ifc/from_decoded.rs` — `ElementInput` + `build_ifc_model` (the bridge from decoded Revit elements to the IFC model) + the `*_extrusion` and `*_property_set` helpers.
- `src/ifc/entities.rs` — the in-memory `IfcEntity::BuildingElement` variant, `PropertyValue` enum with its `to_step()` mapping, and the `Extrusion` / `PropertySet` / `Property` structs.
- `src/ifc/category_map.rs` — Revit class name → IFC4 type + predefined-type mapping.
- `src/ifc/mod.rs` — `IfcModel`, `Storey`, `MaterialInfo`, and the `Exporter` trait (`PlaceholderExporter`, `RvtDocExporter`).
- `tests/ifc_synthetic_project.rs` — the end-to-end integration test that pins every entity count referenced in this post.
- `tests/fixtures/synthetic-project.ifc` — the committed sample output.

## Appendix: version + license

- Version: 0.1.x (pre-1.0; breaking changes possible on the 5b walker and the `ElementInput` surface until 0.2).
- License: Apache-2.0 (see [`LICENSE`](https://github.com/DrunkOnJava/rvt-rs/blob/main/LICENSE) and [`NOTICE`](https://github.com/DrunkOnJava/rvt-rs/blob/main/NOTICE)).
- Clean-room provenance: [`CLEANROOM.md`](https://github.com/DrunkOnJava/rvt-rs/blob/main/CLEANROOM.md).
- Security posture: [`SECURITY.md`](https://github.com/DrunkOnJava/rvt-rs/blob/main/SECURITY.md), [`THREAT_MODEL.md`](https://github.com/DrunkOnJava/rvt-rs/blob/main/THREAT_MODEL.md).
