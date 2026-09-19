# Supported MVP Profile

This project is useful today for file inspection, metadata/schema extraction,
diagnostics, and support triage. It is not yet a general Revit model converter.

## Supported Today

| Area | Supported profile |
|---|---|
| File extensions | `.rvt`, `.rfa`, `.rte`, `.rft` containers that use the standard Revit OLE/CFB layout. |
| Revit versions | Metadata/schema inspection is regression-tested against the 2016-2026 family corpus. |
| Safe workflows | `rvt-inspect`, `rvt-info`, `rvt-schema`, previews, stream inventory, document metadata, class schema, and diagnostics sidecars (`Formats/Latest` multipage integrity uncertain while strip stays disabled). |
| IFC output | Spec-valid IFC4 scaffold with project/spatial framework; partition MVP Level storeys / Room spaces / Material names when recovered (Floor boundary-loop slabs only where no element records decode); typed wall geometry limited to the version-gated 2023 ArcWall path; on Revit 2024, `IfcWall` / `IfcDoor` / `IfcWindow` / `IfcColumn` / `IfcSlab` / `IfcShadingDevice` instances from partition element records with bounding-box envelope bodies and a measured slab thickness (#204 / #211 / #212) — exact against Revit's own export on the one recorded edge, not a general converter. |
| Browser viewer | Zero-upload inspection, File Status (storey names + material samples), scene tree storey grouping when elevations allow, and explicit export-readiness labels before download. |

## Experimental MVP Target

The first real-model conversion profile is intentionally narrow:

| Dimension | Target |
|---|---|
| File type | `.rvt` project files before `.rfa` family geometry. |
| Versions | Revit 2023 and 2024 project files first, because the current project corpus and ArcWall evidence live there. |
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
- Compound wall-layer thicknesses, and slab extrusion depth on any path
  other than the Revit 2024 element records.
- Recovered family profiles / wall location curves for the Revit 2024
  element-record path: bodies there are the record's own bounding box.
- Reliable geometry for walls/floors/doors/windows outside the narrow
  research profile.
- Semantic Revit editing through the stream writer.
- Revit versions outside the verified corpus without a diagnostics report.
- Files that are corrupt, zero-byte LFS placeholders, encrypted, or not OLE/CFB
  Revit containers.

Use `rvt-inspect <file> --json` or the viewer diagnostics download when a file
falls outside the supported profile. Those reports are designed to be attached
to GitHub issues without exposing raw model bytes.
