# HN Show HN draft — rvt-rs

## Title options

1. Show HN: rvt-rs – a Rust library that reads Revit files and exports IFC4
2. Show HN: Open-source reader for Autodesk Revit files, written in Rust
3. Show HN: Revit to IFC4 without Autodesk – Rust, Apache-2, opens in BlenderBIM

## Post body

rvt-rs is an Apache-2 Rust library that reads Autodesk Revit files outside the Autodesk toolchain.

What works: opens .rvt, .rfa, .rte, .rft from Revit 2016-2026 by parsing the OLE/CFB container and decoding Revit's truncated-gzip streams. Enumerates the embedded Formats/Latest schema — 395 classes, 13,570 fields, 100% type-classified across the 11-release corpus. 54 per-class decoders (Wall, Floor, Roof, Door, Window, Column, Beam, Stair, Room, Grid, Level, Material, FamilyInstance, etc.). A schema-directed walker handles full ADocument reads on 2024-2026, partial on 2016-2023. Emits IFC4 STEP with a spatial tree, per-element entities, extruded solids on walls/slabs/roofs/ceilings/columns, IfcMaterial, IfcPropertySets, and opening/fill relationships on doors and windows. A committed synthetic fixture opens in BlenderBIM with 3D geometry.

Why it matters: Revit files have been opaque outside Autodesk's C# API, forcing BIM analytics, migration, and research onto Windows + Revit. rvt-rs is a byte-level reader you can drop into your stack — no Autodesk runtime, no Wine, no commercial SDK.

Honest gaps: no write support (stream-level round-trip only). No pre-2016 files. No password-protected models. No linked-model resolution. No MEP or annotation decoders. IFC profiles are rectangular only; extrusion dimensions are caller-supplied.

Repo: github.com/DrunkOnJava/rvt-rs
License: Apache-2.0

If you work with Revit files outside the Autodesk toolchain, what would unblock you first?
