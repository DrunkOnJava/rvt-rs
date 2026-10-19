# Supported profile

What rvt-rs can be trusted with today, in the terms a BIM user would check.
The evidence behind every line is in [`support-matrix.json`](support-matrix.json)
(`rvt-capabilities --matrix -f text` prints it) and in the measured tables of the
[README](../README.md#what-rvt-rs-reads-from-real-projects). This page
describes `main`; the latest release is v0.6.0, and what has landed since
is under `[Unreleased]` in [`CHANGELOG.md`](../CHANGELOG.md).

## Four levels of "it works"

A Revit file can succeed at one level and not the next, so rvt-rs keeps them
apart:

1. **Opened.** The OLE container, its streams, metadata, previews and schema
   read: every Revit release from 2016 to 2026, across the family corpus.
   Revit 2015 and 2027 files open too, on Autodesk's sample projects, where
   previews and PartAtom are not measured.
2. **Elements identified.** Each element is found with its Revit ElementId and
   category, and the set equals the one Revit's own IFC export holds. Revit
   2024 and 2025 project files, and Revit 2023 ones with their bounding boxes
   only (RE-81). Experimentally, Revit 2026 ones, measured so on one project
   (RE-124), and Revit 2015 to 2022 and 2027 ones, which have no Revit export
   to compare and are measured against the same sample projects saved by
   Revit 2019 (for 2015 to 2018), 2023 (for 2019 to 2022) and 2026 (for 2027)
   instead: a 2027 project's ElementIds and classes are its 2026 copy's, and
   on 2015 to 2022 every ElementId shared with the copy keeps its class, with
   the copy's building elements class for class where Autodesk did not edit
   the model between releases (the table below).
3. **Geometry.** An element's body. Exact where the element's data is decoded
   (the table below), otherwise its bounding box, and the export diagnostics
   say which.
4. **Properties.** Names, types, storey, materials, layers and GlobalIds are
   read; most other parameters are not (#35, #155).

## By input

| Input | Opened | Elements identified | Geometry | Properties |
|---|---|---|---|---|
| Revit 2024 and 2025 project (`.rvt`) | yes | yes, measured on Core Interior, the four RE1 models, Snowdon Towers, Projeto1 and `teste_export_2025` | see below | see below |
| Revit 2026 project | yes | experimental (since 0.5.0): measured on one 2026 project against Revit's own IFC4X3 export, 113 of its 115 elements and none outside it (RE-124) | the element's bounding box, walls cut back at their joins | Revit's GlobalIds, storeys, rooms with Revit's numbers and names, doors and windows in their hosts, family and type names, layer sets |
| Revit 2023 project | yes | yes, measured on two 2023 projects against Revit's IFC4 exports (RE-81); components nested in doors and windows are left out | the element's bounding box; rooms their outline (RE-102) | ElementId, category, door and window host walls (RE-85), Levels as storeys with each element on the one its record names (RE-107), and family instances' family and type names (RE-109) |
| Revit 2027 project | yes | experimental (since 0.6.0): the same ElementIds and classes as the 2026 copies of Autodesk's five sample projects (RE-179); no Revit 2027 export is measured | as on 2026 | as on 2026 |
| Revit 2019 to 2022 project | yes | experimental (since 0.6.0): 2023's records and readers with each release's schema tags; every ElementId shared with the 2023 copies of Autodesk's sample projects keeps its class, and the `rac_basic`, `rst_basic` and 2021 and 2022 `rac_advanced` exports have the 2023 copies' building elements, class for class (RE-180); no Revit export of a 2019 to 2022 file is measured | as on 2023 | as on 2023 |
| Revit 2015 to 2018 project | yes | experimental (since 0.6.0): 2019's records and readers with each release's schema tags; every ElementId shared with the 2019 copies of Autodesk's 2016 to 2018 sample projects keeps its class, and their exports have the 2019 copies' building elements, class for class; on five of the six 2015 samples every shared ElementId keeps its class, and the sixth, `rst_basic`, is another model (RE-181); no Revit export of a 2015 to 2018 file is measured | as on 2019 | as on 2019 |
| Revit 2014 and earlier project | yes | no | none | metadata |
| Family (`.rfa`) or template (`.rte`, `.rft`) | yes | family metadata and OmniClass only | none | none |

A file that is corrupt, encrypted, a zero-byte Git LFS placeholder, or not an
OLE/CFB Revit container is refused with an error, never a partial model.

## Geometry on Revit 2024 and 2025 projects

| Element | What you get | How close |
|---|---|---|
| Walls | a body from the wall's centreline and its type's thickness, drawn as its layers in the viewer and glTF, ending as its joins say, along its arc when curved, leaning on its exterior face when tapered | faces exactly Revit's on 1,030 of Snowdon's 1,078 walls; both ends on 724 |
| Floors, building pads | the sketched plan outline, holes included, at the measured thickness | Core Interior 80 of 80 outlines |
| Roofs | the sketched outline; a shed roof along its slope | 11 of 13 outlined Snowdon roofs with Revit's area; hip and gable roofs are boxes (#356) |
| Columns | the column prism minus what walls cut from it | Core Interior 256 of 256, exact |
| Beams | along their location line with their section; a steel beam whose type stores an I section as that I (RE-103) | 923 of 942 Snowdon structural beams on their line; 377 drawn as their I, Revit's own shape and place on all 271 its VIM export scores; channels and angles are not drawn yet (#94) |
| Steel columns | a structural column whose type stores an I section as that I, turned to the column's own X axis (RE-104) | all 40 of Snowdon's steel columns; Revit's own shape and turn on all 24 its VIM export scores; tubes and other sections keep their box. Every I-section member carries its I as an `IfcMaterialProfileSet` of its type's material (RE-105) |
| Pipes | a cylinder along its two ends with its outside radius, where the ends and the record box prove one (RE-131) | all 69 pipes on RE1 Mechanical and Plumbing (Revit 2025), Revit's own radius, ends and length to 0.00001 ft; Revit 2025 only; sloped pipes are not measured, ducts and fittings keep their box |
| Stairs | aggregates of flights, landings and supports; straight flights as their steps, monolithic ones included | 33 of 38 drawn flights equal to Revit's; spiral and riserless flights are boxes (#357) |
| Curtain walls | aggregates of panels and mullions placed by their grids; a mullion or panel turned or tilted off the model's axes as the box along its own axes (RE-106) | every Snowdon panel and mullion relation Revit's; 1,019 turned or tilted mullions and panels with Revit's extents along their axes |
| Doors, windows, openings | the element's box, turned with the instance where it is turned (RE-87); its opening is as deep as the host wall | host is Revit's on 138 of 138 (Core Interior) and 192 of 194 (Snowdon; the other 2 in another wall Revit cuts, RE-85); the opening is within 0.25 ft of Revit's on Core Interior; a window whose type stores Width, Height and Default Sill Height takes Revit's opening exactly (56 of 68 on Snowdon, RE-93), and a door whose body is its type's Rough Height tall its rough opening (57 of 126, RE-94); any other door whose type stores Width and Height carries them as its overall size (136 of 137 on Core Interior and RE1, RE-187); otherwise not Revit's opening profile (#227, RE-83) |
| Rooms | `IfcSpace` with number, name and the outline of the room's stored solid (RE-101) | Revit's own area and outline on all 116 Core Interior and 11 RE1 Architecture rooms; 45 of 54 Snowdon Towers rooms at the area its VIM export gives, the other 7 whose solids do not close keeping their box (#90); on Revit 2023, all 9 of Exemplo_data's (RE-102) |
| Areas | `IfcSpace` with number, name and storey; no body | all 30 of Revit's area spaces on the BIM4LCA office, as built and at building permit (RE-191); Revit 2024 only |
| Furniture, fixtures, equipment, MEP devices, site | the element's bounding box, turned with the instance where it is turned (RE-87) | placement and extent only; no family geometry. MEP devices and equipment are measured on RE1 Electrical and Mechanical only (RE-79) |
| Family instances with `--saved-meshes` | the mesh Revit saved for the element's display | within 0.01 ft of Revit's export: Core Interior 158 of 158, RE1 (2025) 207 of 214, Snowdon Towers 3,332 of 3,736 (RE-78) |

## Properties on Revit 2024 and 2025 projects

Read: ElementId, category, `Family:Type:ElementId` name, type name, storey,
material names and layer thicknesses, family instances' materials (RE-82), the IFC type
object of each element's type (RE-110), Revit's GlobalId (other entities get one
derived from the document and their own identity, stable between exports,
#400), stair riser and tread dimensions, each element's `IsExternal` as Revit's
exporter sets it (a parameter named for its property set, its type's
Function, else its class's rule; none on curtain mullions and panels, RE-193),
the project and shared parameters named for a property of one of IFC4's
property sets (`Pset_WallCommon.FireRating`), from buildingSMART's
definitions of the 278 sets that apply to an entity, written in that set
where IFC4 applies it to the entity written or one of its supertypes, with
the predefined type a scoped set needs (booleans, text, counts and integers,
lengths and thermal transmittance, RE-194, #817), and Revit's "Export to IFC As"
override, applied where it names `IfcShadingDevice`, `IfcSlab`, `IfcCovering`,
`IfcWasteTerminal`, `IfcFireSuppressionTerminal`, `IfcSanitaryTerminal`,
`IfcAirTerminal`, `IfcElectricAppliance` (RE-190), `IfcStair` or `IfcDoor` (#792)
and Revit's exporter acts on it (not on a 2024 file's curtain walls, RE-188, or its wall sweeps, RE-192),
with the values not applied listed in the diagnostics. Not read: most instance and type parameters (#35, #155) and phases
(#328), so an element Revit's export leaves out because of its phase can still
appear. Elements in a design-option set's non-primary options are left out, as
Revit leaves them out.

## What approximations look like in the output

- A body that is the element's bounding box is a correctly placed, correctly
  sized box. It is never a guess at a shape.
- Where rvt-rs cannot tell something, it leaves it out rather than guess: an
  element whose storey is unknown sits in the building, not on a storey; a
  missing value is absent, not zero.
- `rvt-ifc --diagnostics out.json` and the viewer's diagnostics download list
  what was exported, approximated and skipped, and why. `--mode
  typed-no-geometry`, `geometry` and `strict` refuse a file that does not meet
  that bar instead of writing a thinner IFC.

## Not supported

- Converting an arbitrary Revit model to IFC with Revit-grade fidelity.
- Element records of releases other than 2015 to 2027. Of those, 2015 to 2022, 2026 and 2027 are experimental: 2026 is measured on one project against Revit's export (RE-124), and since 0.6.0 2027 against the 2026 copies of Autodesk's sample projects (RE-179), 2019 to 2022 against their 2023 copies (RE-180), and 2015 to 2018 against their 2019 copies (RE-181), none with a Revit export of its own release. On 2023 (since 0.4.0): geometry beyond each element's box, wall joins and beam cuts, layered or angled joins, design options and IFC export overrides.
- Semantic editing of a Revit file. `rvt-write` patches whole streams and
  preserves the rest byte for byte; it does not change model data.
- Door/Window typing from the opening-index rows (RE-19) and Level ElementIds
  from the Formats schema (RE-20): both closed negative. Element records supply
  both instead.

Use `rvt-inspect <file> --json` or the viewer's diagnostics download when a
file falls outside this profile. Those reports can be attached to a GitHub
issue without sharing the model; [`CONTRIBUTING.md`](../CONTRIBUTING.md)
describes how to send a diagnostic instead of a building file.
