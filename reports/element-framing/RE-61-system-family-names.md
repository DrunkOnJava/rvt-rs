# RE-61 — Layer sets named `Family:Type`

**Date:** 2026-09-24
**Result:**
- The IFC export names each material layer set `Family:Type`, as Revit does: "Basic Wall:Exterior - Brick on Mtl. Stud", not "Exterior - Brick on Mtl. Stud" (RE-58).
- Revit does not store a system family's name. "Basic Wall" and "Compound Ceiling" appear nowhere in RE1 Architecture's bytes. Revit derives the name from the type's kind, and only one system family per category has compound layers, so a type with layers gives it:
  - a wall is a Basic Wall, not a Curtain or Stacked Wall;
  - a floor is a Floor;
  - a ceiling is a Compound Ceiling, not a Basic Ceiling;
  - a roof is a Basic Roof, not Sloped Glazing;
  - a building pad is a Pad.
- Measured against Revit's own IFC exports:

| file | layer set names equal to the `Family:Type` of Revit's element names | sets |
|---|---:|---:|
| Snowdon Towers (local only) | 1,210 of 1,210 elements | 62 of 62 |
| 2024_Core_Interior | 88 of 88 | 4 of 4 |
| RE1 Architecture | 15 of 15 | 4 of 4 |

- RE1's 2025 export writes its own layer sets, and their four names are rvt-rs's exactly.
- Snowdon's export names 43 of its constituent sets `Family:Type`. The 38 that rvt-rs writes a layer set for have the same names.
- Elements are matched by `Tag`, and Revit's export gives an opening the `Tag` of the element that cuts it. Walls 698311 and 698364 cut openings in wall 619550, and those openings carry the walls' Tags and 619550's name. `tools/re/layer_sets_vs_ifc.py` leaves openings out when it matches.

-----

## 1. The mapping

`partition_schema_mvp::layered_system_family` maps a record-backed element's class to its system family. `ElementLayers::system_family` carries it, set only where the element's type layers are read (RE-53, RE-57). A wall, ceiling or roof without read layers gets no family, so a Curtain Wall, Basic Ceiling or Sloped Glazing type is never misnamed.

Against the `Family:Type:ElementId` names Revit's exports give every record-backed element of these five classes:

| class | family | Snowdon | Core Interior | RE1 |
|---|---|---:|---:|---:|
| Wall | Basic Wall | 1,077 | 360 | 7 |
| Floor | Floor | 182 | 99 | 2 |
| Ceiling | Compound Ceiling | 68 | — | 6 |
| Roof | Basic Roof | 20 | — | — |
| BuildingPad | Pad | — | 1 | — |

Core Interior's 20 floors that Revit exports as shading devices are Floors too. The mapping follows the element's Revit category, not its IFC entity.

## 2. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |
| 2024_Core_Interior.rvt | `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014` |
| IFC Exports/2024_Core_Interior_slim.ifc | `bfdf36ffb0bb768f3409d818403990e64d4c262c6780603be87f8077387ad86d` |
| RE1-Architecture.rvt | `4a78bdd68e7f4dd7806060db07c222da9a810e20b3f303c17a443294b91c8ce4` |
| RE1-Architecture.ifc | `a9b5d36677aa6a8bb91b77d8bc354ed9028a7ca3e9e2491a47ba14cfd8e26200` |

```bash
./target/ci/rvt-ifc MODEL.rvt -o model.ifc
python3 tools/re/layer_sets_vs_ifc.py model.ifc REVIT_EXPORT.ifc
```

`tools/re/layer_sets_vs_ifc.py` now also compares each layer set's name with the `Family:Type` part of Revit's name for the element.

The Core witness observations do not change: they count entities, and no entity is added or removed.

## 3. What this does not do

- Elements are still named `Class-ElementId` ("Wall-20796"), where Revit writes `Basic Wall:8" Interior Partition 3 Hour:20796`.
- A type without read layers keeps its plain type name.
