# OSArch / IfcOpenShell forum post — rvt-rs

Target venues: community.osarch.org and the IfcOpenShell discussions board. One post text serves both, with small header adjustments per venue.

Category on OSArch: "Software / open-bim-tools" (discussions about open BIM tooling, not support requests).

## Title

rvt-rs: reading Revit files into IFC without the Autodesk dependency — feedback welcome

## Post body

Hi everyone,

I've been building `rvt-rs` — an Apache-2.0 Rust library that reads Autodesk Revit files (`.rvt`, `.rfa`, `.rte`, `.rft`) and emits IFC4 STEP, without needing a Revit installation or the Autodesk .NET SDK. This is a complement to IfcOpenShell and BlenderBIM, not an alternative. The piece I'm trying to fill is the missing *input* stage: the one that today forces anyone doing openBIM work with Revit authored content onto Windows + Revit + the Autodesk IFC Exporter, because that exporter runs inside Revit using the Revit API and only emits what the API surfaces.

I'd like feedback from the people who actually live in IFC every day, because the places I need the most help aren't "does it parse bytes" — it's "does the IFC4 I'm producing compose cleanly with your workflows."

### How this project came to exist

A gap. The openBIM community has done the hard work on IFC, on BCF, on BIMserver, on BlenderBIM. But the last mile into Revit-authored content has always gone through Autodesk's `revit-ifc` exporter, which requires a Windows machine with a Revit license and produces output the OSArch wiki itself describes — in the "Revit setup for OpenBIM" page — as *"out of the box, just crap."*

That is a solvable problem on the file-format side. Every Revit file is a Microsoft Compound File Binary (OLE2) container with a known stream layout, truncated-gzip compressed streams, and a self-describing class schema in `Formats/Latest` (395 classes, 13,570 fields, one-to-one match against the symbols exported by the public `RevitAPI.dll` NuGet package). Read it byte-level, decode each class per the schema, and you no longer need Autodesk in the loop to produce IFC. That's the thesis of `rvt-rs`.

All file-format observations come from publicly-shipped Autodesk sample content and the public `RevitAPI.dll` symbol list. No Autodesk proprietary code is used, referenced, or redistributed. See the `NOTICE` file for the clean-room statement.

### What works end-to-end today

`.rvt` / `.rfa` → decoded element stream → IFC4 STEP → opens in BlenderBIM. The pipeline is:

1. Open the CFB container (no Revit required).
2. Decompress the truncated-gzip streams (`Formats/Latest`, `Global/Latest`, `Partitions/NN`).
3. Parse the full class schema. 100% of fields across the 11-release reference corpus (Revit 2016-2026) classify into typed encodings — no `Unknown`s.
4. Run 54 per-class decoders (Wall, Floor, Roof, Ceiling, Door, Window, Column, Beam, Stair, Railing, Room, Area, Space, Furniture, Rebar, Level, Grid, Material, Category, Phase, DesignOption, Workset, and more).
5. Map each decoded element into `ifc::build_ifc_model`, emit IFC4 STEP via `ifc::write_step`.

A committed synthetic-project fixture lives at [`tests/fixtures/synthetic-project.ifc`](../../tests/fixtures/synthetic-project.ifc) — 157 lines of IFC4 with:

- `IfcProject` + `IfcSite` + `IfcBuilding` + 3 `IfcBuildingStorey`s (Ground, Second at 3.048 m, Roof Deck at 6.096 m).
- 4 `IfcWall`s (N/S/E/W) with `IfcExtrudedAreaSolid` geometry, 1 `IfcSlab` (ground floor), 1 `IfcDoor`, 2 `IfcWindow`s, 1 `IfcStair`, 1 `IfcBuildingElementProxy` (the unknown-class fallback).
- `IfcOpeningElement` + `IfcRelVoidsElement` + `IfcRelFillsElement` wiring the front door to the south wall (i.e. there's a real hole where the door goes).
- `IfcMaterial` + `IfcSurfaceStyle` + `IfcRelAssociatesMaterial` for concrete and tinted glass.
- `Pset_WallCommon` (5 properties) + `Pset_WindowCommon` (2 properties) via `IfcPropertySet` + `IfcPropertySingleValue` + `IfcRelDefinesByProperties`.
- `IfcSIUnit` assignments in millimetres + SI area/volume + radians.
- `IfcRelContainedInSpatialStructure` binding elements to their storey.
- Deterministic `GlobalId`s (seeded counter, not random) so the fixture is byte-stable under a fixed timestamp — the test asserts this.

Load that fixture in BlenderBIM and the spatial tree, the geometry, the opening, the materials and their surface styles, and the property sets all render. That's what "works end-to-end" means here.

The live `.rvt` → `.ifc` CLI (`rvt-ifc`) produces a valid spatial tree + classifications from any file in the corpus today, but per-element geometry from real files is gated on the Phase 5 geometry-extraction work (`GEO-27..34` in the tracker). The synthetic fixture exercises the *writer* end; the live end-to-end bridge from decoded Revit geometry into the writer is the next frontier.

### How this composes with the OSArch stack

I've tried to build this so it slots in cleanly wherever you already have open BIM tooling:

- **BlenderBIM input.** `rvt-rs` writes IFC4 STEP that BlenderBIM imports directly. No extra conversion step, no intermediate format, no "post-processing macro." The synthetic fixture is a working example — it opens the same way any hand-authored IFC does.
- **IfcOpenShell pipeline.** STEP out of `rvt-rs` → `ifcopenshell.open()` in Python → everything you already know: `ifcopenshell.util.selector`, `ifcopenshell.api`, BCF export, BonsaiBIM, `ifcpatch`, `ifctester`. I haven't tried to reinvent any of that — the goal is that `rvt-rs` produces the IFC, and your existing tools take it from there.
- **Validation.** `IFC-41` in the tracker is a CI job that runs every corpus IFC output through IfcOpenShell for schema validation. That job isn't written yet; it's the kind of thing I'd love to co-design with someone who knows where IfcOpenShell's validator is strict vs. permissive relative to buildingSMART's own validator.
- **Test corpus.** The synthetic fixture and the 11-release reference corpus (Autodesk's public `rac_basic_sample_family` RFA, one per Revit release from 2016 through 2026, distributed via git-lfs through the `phi-ag/rvt` repo with explicit permission from Autodesk for reverse-engineering sample content) could feed into IfcOpenShell's own test corpus for the Revit-interop path — if that's useful.
- **Python interop.** `pip install rvt` gives you a PyO3 binding. A Revit file in, typed element views + IFC4 STEP out, in Python, without a Revit or Windows dependency. It should sit alongside `ifcopenshell` in a notebook without friction.

### Where IfcOpenShell / OSArch expertise would unblock real progress

These are the IFC-emission tasks I've scoped but am least confident about doing in isolation, because the "right answer" depends on buildingSMART convention and what the ecosystem actually consumes, not on what parses:

- **IFC-40 — Map Revit units (`autodesk.unit.*` identifiers) to `IfcSIUnit` / `IfcConversionBasedUnit`.** The Forge Design Data Schema namespace is on-disk and decodable; the mapping table to IFC units is what I don't want to get wrong. `autodesk.unit.unit:millimeters-1.0.1` → `IfcSIUnit(*, .LENGTHUNIT., .MILLI., .METRE.)` is easy. `autodesk.unit.unit:fahrenheit-1.0.1`, `autodesk.unit.unit:poundsForceFeet-1.0.1`, and the ~200 others — a review against what IfcOpenShell and BlenderBIM import cleanly would save a lot of trial-and-error.
- **IFC-28 — `IfcMaterialLayerSet` for composite walls.** Revit stores per-wall-type layer breakdowns (layer thickness, function, material). I know how to decode it; I don't know the OSArch-community-preferred emission pattern (e.g. should unsupported layer-functions map to `IfcMaterialLayer.Category = "USERDEFINED"` with the original Revit function name, or are there better conventions?).
- **IFC-17 — `IfcOpeningStandardCase` vs `IfcOpeningElement`.** Right now I emit `IfcOpeningElement` unconditionally. The StandardCase variant has strict geometric constraints; I'd like a reviewer to tell me which families in Revit actually satisfy the constraints vs. which should stay on the general form.
- **IFC-21 — `IfcTypeProduct` for family-type instancing.** Revit's FamilyInstance/Symbol split maps pretty naturally to `IfcBuildingElementType` + `IfcRelDefinesByType` + per-instance `IfcBuildingElement`, but I want to get the `RepresentationMaps` part right — using one `IfcShapeRepresentation` per type and referencing from instances, not duplicating the geometry per instance. Any known-good reference implementations to study would help.
- **Pset naming conventions.** I'm currently emitting `Pset_WallCommon`, `Pset_DoorCommon`, etc. where a matching buildingSMART standard Pset exists, and `Pset_RevitType_{ClassName}` for Revit-specific parameter groups to stay aligned with what the Autodesk exporter does. Is that the right default? Would it be better to normalize everything to buildingSMART standard Psets and emit the Revit-specific data as `IfcExtendedProperties` or a separate `Pset_Revit_*` namespace?

If any of those are questions with an obvious answer to someone fluent in IfcOpenShell, I'd rather ask now than ship a wrong convention and have to migrate later.

### Honest gaps — what doesn't work today

I'd rather tell you the gaps up front than have you hit them:

- **No Revit write path.** Stream-level byte-preserving round-trip works (13/13 streams identical on the 2024 sample); field-level semantic writes (edit a specific Wall's `unconnected_height` and save back to a Revit-openable `.rvt`) are Phase 7, not shipped.
- **No pre-2016 Revit files.** The schema parser and compression path almost certainly still work, but nothing is claimed outside [2016, 2026] because the corpus doesn't cover it.
- **ADocument walker is partial on Revit 2016-2023.** Reliable on 2024-2026; the pre-2024 entry-point heuristics differ and I return `Ok(None)` rather than guess.
- **No encrypted / password-protected files.** The outer CFB opens, but inner streams will be gibberish.
- **No linked-model resolution.** Single-file reader only.
- **No MEP decoders.** `LightingFixture`, `MechanicalEquipment`, `PlumbingFixture`, etc. are mapped in `ifc::category_map` but have no `ElementDecoder` registered yet.
- **No annotations / dimensions / tags / legends.**
- **Geometry extraction is gated on Phase 5.** The IFC writer emits valid extruded solids when the caller supplies dimensions; pulling those dimensions from real `.rvt` location curves + profile shapes is `GEO-27..34`.
- **IFC profiles are rectangular only.** The `IfcRectangleProfileDef` path works; `IfcArbitraryClosedProfileDef`, `IfcIShapeProfileDef`, etc. are not emitted yet.
- **IFC4 only.** The category map is structured to make an IFC2X3 / IFC4.3 swap a table replacement, but it has not been swapped.

The [`docs/compatibility.md`](../compatibility.md) page has the full per-class status matrix, row by row, with source links so you can check any claim against current code.

### Ways to contribute

If you want to help:

1. **Review the synthetic IFC output.** Open [`tests/fixtures/synthetic-project.ifc`](../../tests/fixtures/synthetic-project.ifc) in BlenderBIM or run it through `ifcopenshell-python` and tell me where the output disagrees with what your tooling expects. Tripping on the unit assignment, the placement hierarchy, the property-set binding, the classification references — any of it. File issues against specific STEP entity IDs; I want to fix convention bugs, not parse bugs, in this layer.
2. **File IFC-schema gap issues.** If you see something that should emit but doesn't (an `IfcMaterialLayerSet`, an `IfcPropertySetTemplate`, a specific `Pset_*` variant), open an issue with the Revit input + expected IFC output. The task tracker has `IFC-*` task IDs; I'm happy to add new ones.
3. **Share a real `.rvt` corpus.** The `rac_basic_sample_family` RFA fixtures are small single-family content. To stress test this on project scale I need 10+ real project `.rvt`s the authors are willing to share (`Q-01` in the tracker). An opt-in corpus-submission issue template is in `.github/ISSUE_TEMPLATE/corpus-submission.md` — it covers licensing, scrubbing (we can redact names / paths / custom families before commit), and the git-lfs storage mechanics.
4. **IfcOpenShell CI integration.** `IFC-41` — I need a CI job that validates every corpus IFC output through IfcOpenShell. If someone who knows the IfcOpenShell validator internals wants to help wire that up, the PR would be welcome.

### Links

- Repo: https://github.com/DrunkOnJava/rvt-rs
- Compatibility matrix: [`docs/compatibility.md`](../compatibility.md)
- Demo gallery with committed CLI output + the synthetic IFC fixture: [`docs/demos.md`](../demos.md)
- Python bindings guide: [`docs/python.md`](../python.md) — `pip install rvt`
- Recon report (the full Revit-format reverse-engineering narrative with 12 dated addenda): [`docs/rvt-moat-break-reconnaissance.md`](../rvt-moat-break-reconnaissance.md)
- License: Apache-2.0. Autodesk and Revit are registered trademarks of Autodesk, Inc.; this project is not affiliated with, endorsed by, or sponsored by Autodesk.

Thanks for reading — and for all the openBIM work this project stands on top of. Every decoded element that lands in BlenderBIM owes something to the IfcOpenShell STEP parser, the BlenderBIM viewer code, the OSArch wiki pages on Revit interop, and years of buildingSMART standards work. I'm trying to feed the Revit end of that pipeline with something that doesn't need Autodesk in the room.

Feedback welcome — especially the kind that tells me a convention I'm using is wrong.

— Griffin

## Venue notes

**community.osarch.org:** post in the "Software" top-level category, subcategory "open-bim-tools" if it exists, otherwise "general". OSArch convention is a discussion post (not a support request), first-person, with a clear "feedback welcome" framing. Tag with `revit`, `ifc`, `rust`, `apache-2`. Link back to this file or to the README for the canonical reference. Expect mentions of `@duncan` (Dion Moult), `@theoryshaw` (Ryan Schultz), and BlenderBIM core contributors — respond in thread, not in DMs, so the full conversation stays on the forum.

**IfcOpenShell discussions:** use the GitHub Discussions board at github.com/IfcOpenShell/IfcOpenShell/discussions. Category "Show and tell" for the announcement; "Ideas" if there's a follow-up specifically about the `IFC-41` validator CI integration. Cross-link to the OSArch thread so both venues see each other's feedback.

**Don't cross-post to r/BIM or LinkedIn from here** — those have their own `LAUNCH-*` tasks with audience-appropriate framing. This post is written for people who already believe in open IFC workflows; the broader-audience posts pitch the problem differently.
