# Supported MVP Profile

This project is useful today for file inspection, metadata/schema extraction,
diagnostics, and support triage. It is not yet a general Revit model converter.

## Supported Today

| Area | Supported profile |
|---|---|
| File extensions | `.rvt`, `.rfa`, `.rte`, `.rft` containers that use the standard Revit OLE/CFB layout. |
| Revit versions | Metadata/schema inspection is regression-tested against the 2016-2026 family corpus. |
| Safe workflows | `rvt-inspect`, `rvt-info`, `rvt-schema`, previews, stream inventory, document metadata, class schema, and diagnostics sidecars (`Formats/Latest` multipage integrity uncertain while strip stays disabled). |
| IFC output | Spec-valid IFC4 scaffold with project/spatial framework; partition MVP Level storeys and Material names when recovered; typed wall geometry limited to the version-gated 2023 ArcWall path; on Revit 2024, `IfcWall` / `IfcDoor` / `IfcWindow` / `IfcColumn` / `IfcSlab` / `IfcShadingDevice` / `IfcSpace` instances from partition element records with a measured slab thickness (#204 / #211 / #212), slab plan profiles from their sketch lines (#31), wall runs cut back by the joins the record names (351 of 360 world-exact) and column bodies cut by the walls that cut them (256 of 256 world-exact, #239 / RE-29) — exact against Revit's own export on the one recorded edge, not a general converter. On Revit 2024 and 2025 the same records also give furniture, casework, plumbing fixtures, specialty equipment, ceilings, curtain-wall mullions and panels, railings, wall sweeps, ducts, pipes and their fittings, as bounding-box bodies, with no element Revit's own export lacks (RE-33), and structural framing, structural columns, foundations and generic models as `IfcBeam` / `IfcColumn` / `IfcFooting` / `IfcBuildingElementProxy` (RE-36), and lighting fixtures, air terminals, food-service equipment, site elements, elevators and ramps (RE-37); Revit 2025 walls, slabs and rooms follow RE-32. |
| Browser viewer | Zero-upload inspection, File Status (storey names + material samples), scene tree storey grouping when elevations allow, a typed element info panel (name, type, GUID, clickable storey, placement and extents in feet, property sets as unit-carrying rows, host and hosted rows), a schedule grouped by IFC type with per-type scene highlight, and explicit export-readiness labels before download. |

## Experimental MVP Target

The first real-model conversion profile is intentionally narrow:

| Dimension | Target |
|---|---|
| File type | `.rvt` project files before `.rfa` family geometry. |
| Versions | Revit 2024 and 2025 project files first, because the element-record evidence lives there; 2023 through the ArcWall path. |
| Discipline | Architectural core before MEP/structure-heavy projects. |
| Classes | Levels, walls, floors/slabs, doors, windows, rooms/spaces, materials, and common parameters. |
| Export quality | `rvt-ifc --mode strict` must reject files that cannot meet the requested quality. |

## Unsupported Or Partial

- Full typed element extraction from arbitrary real `.rvt` files.
- Schema-field Walls; typed Door vs Window host IFC (RE-19 negative — no
  reliable discriminator in the opening-index bytes / no schema-field envelope
  on current magnetar corpora). The Revit 2024 partition element-record path
  (#211) recovers Wall / Door / Window *instances* from a different carrier,
  and their host-wall binding is closed on that carrier (#222, RE-23: the
  `(host wall, filling element)` pair set equals Revit's export on 138 of
  138). What stays unsupported is a Door/Window host claim from the
  opening-index rows themselves.
- The plan profile of the **20 rotated shading plates**: their
  `OST_SketchLines` boxes are axis-aligned envelopes of diagonal segments, the
  closure declines them, and they keep the record box rectangle with
  `ProfileResolved: false`. All 80 exported slabs do carry the boundary
  polygon their sketch lines close (#31, RE-25, closed 2026-09-19), on top of
  the exported ElementId set, the model bounding box and a measured extrusion
  thickness (#212, RE-22).
- Floor/Room storey assignment via Level ElementIds (RE-20 negative — `Level`
  absent from Formats; bind plumbing stays fail-closed / idle).
- Slab extrusion depth on any path other than the Revit 2024 element
  records. Compound layers are read from each type's own data (RE-53)
  and written as `IfcMaterialLayerSetUsage` with their material names
  (RE-58). Elements drawn whole or sloped get no layer set, and on this
  corpus the walls' types are single layers by category, so only its 88
  floors and shading devices carry one. The paired reference export is a
  `ReferenceView_V1.2` file with no layer sets of its own, so they are
  scored against the faces its bodies style with each material.
- Recovered family profiles / wall location curves for the Revit 2024
  element-record path: bodies there are the record's own bounding box,
  except for a slab's plan profile (#31, RE-25), a wall's join-trimmed run
  and a column's join-cut prism (#238 / #239, RE-29). Nine wall ends at
  true L corners are still over-trimmed, recorded as a measured negative
  with no identified carrier.
- Reliable geometry for walls/floors/doors/windows outside the narrow
  research profile.
- Semantic Revit editing through the stream writer.
- Revit versions outside the verified corpus without a diagnostics report.
- Files that are corrupt, zero-byte LFS placeholders, encrypted, or not OLE/CFB
  Revit containers.

Use `rvt-inspect <file> --json` or the viewer diagnostics download when a file
falls outside the supported profile. Those reports are designed to be attached
to GitHub issues without exposing raw model bytes.
