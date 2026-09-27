# Supported profile

What rvt-rs can be trusted with today, in the terms a BIM user would check.
The evidence behind every line is in [`support-matrix.json`](support-matrix.json)
(`rvt-capabilities --matrix -f text` prints it) and in the measured tables of the
[README](../README.md#what-rvt-rs-reads-from-real-projects). This page
describes `main`; the latest release, v0.2.0, predates everything under
`[Unreleased]` in [`CHANGELOG.md`](../CHANGELOG.md).

## Four levels of "it works"

A Revit file can succeed at one level and not the next, so rvt-rs keeps them
apart:

1. **Opened.** The OLE container, its streams, metadata, previews and schema
   read. Every Revit release from 2016 to 2026.
2. **Elements identified.** Each element is found with its Revit ElementId and
   category, and the set equals the one Revit's own IFC export holds. Revit
   2024 and 2025 project files.
3. **Geometry.** An element's body. Exact where the element's data is decoded
   (the table below), otherwise its bounding box, and the export diagnostics
   say which.
4. **Properties.** Names, types, storey, materials, layers and GlobalIds are
   read; most other parameters are not (#35, #155).

## By input

| Input | Opened | Elements identified | Geometry | Properties |
|---|---|---|---|---|
| Revit 2024 and 2025 project (`.rvt`) | yes | yes, measured on Core Interior, the four RE1 models, Snowdon Towers, Projeto1 and `teste_export_2025` | see below | see below |
| Revit 2026 project | yes | no: the element-record marker is predicted (RE-32) but unmeasured | none | metadata |
| Revit 2023 and earlier project | yes | no (2023 arc walls only) | none | metadata |
| Family (`.rfa`) or template (`.rte`, `.rft`) | yes | family metadata and OmniClass only | none | none |

A file that is corrupt, encrypted, a zero-byte Git LFS placeholder, or not an
OLE/CFB Revit container is refused with an error, never a partial model.

## Geometry on Revit 2024 and 2025 projects

| Element | What you get | How close |
|---|---|---|
| Walls | a body from the wall's centreline and its type's thickness, drawn as its layers in the viewer and glTF, ending as its joins say, along its arc when curved | faces exactly Revit's on 1,014 of Snowdon's 1,078 walls; both ends on 716 |
| Floors, building pads | the sketched plan outline, holes included, at the measured thickness | Core Interior 80 of 80 outlines |
| Roofs | the sketched outline; a shed roof along its slope | 11 of 13 outlined Snowdon roofs with Revit's area; hip and gable roofs are boxes (#356) |
| Columns | the column prism minus what walls cut from it | Core Interior 256 of 256, exact |
| Beams | along their location line with their section | 923 of 942 Snowdon structural beams on their line |
| Stairs | aggregates of flights, landings and supports; straight flights as treads and risers | 30 of 34 drawn flights equal to Revit's; other flight kinds are boxes (#357) |
| Curtain walls | aggregates of panels and mullions placed by their grids | every Snowdon panel and mullion relation Revit's |
| Doors, windows, openings | the element's box, cutting its host wall | host pairs Revit's; the opening is not Revit's opening profile (#227) |
| Rooms | `IfcSpace` with number and name | boundaries not decoded (#90) |
| Furniture, fixtures, equipment, MEP, site | the element's bounding box | placement and extent only; no family geometry |

## Properties on Revit 2024 and 2025 projects

Read: ElementId, category, `Family:Type:ElementId` name, type name, storey,
material names and layer thicknesses, Revit's GlobalId (other entities get one
derived from the document and their own identity, stable between exports,
#400), stair riser and tread dimensions, and Revit's "Export to IFC As"
override. Not read: most instance and type parameters (#35, #155) and phases
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
- Element records of releases other than 2024 and 2025.
- Semantic editing of a Revit file. `rvt-write` patches whole streams and
  preserves the rest byte for byte; it does not change model data.
- Door/Window typing from the opening-index rows (RE-19) and Level ElementIds
  from the Formats schema (RE-20): both closed negative. Element records supply
  both instead.

Use `rvt-inspect <file> --json` or the viewer's diagnostics download when a
file falls outside this profile. Those reports can be attached to a GitHub
issue without sharing the model; [`CONTRIBUTING.md`](../CONTRIBUTING.md)
describes how to send a diagnostic instead of a building file.
