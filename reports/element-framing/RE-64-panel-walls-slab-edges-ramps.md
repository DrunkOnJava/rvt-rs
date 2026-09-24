# RE-64 — Panel walls, slab edges and ramps named as Revit names them

**Date:** 2026-09-24
**Result:**
- Three more kinds of system-family element are named `Family:Type:ElementId`, with `ObjectType` `Family:Type`, as Revit's own IFC export names them:
  - **Curtain panels that are walls.** Revit lets a curtain grid cell hold a basic wall instead of a panel. On Snowdon Towers, 18 panels named `CurtainWallPanel-ElementId` are now "Basic Wall:Exterior - GWB on Metal Stud w Insulated Panel 8 7/8":ElementId" (#370).
  - **Slab edges** are named "Slab Edge:_Stair Landing Plate Stringer:629902" rather than "SlabEdge-629902".
  - **Ramps** are named "Ramp:Ramp 1:832422".
- Measured against Revit's own IFC export of Snowdon Towers, matched by `Tag`:

| | before | after |
|---|---:|---:|
| panel walls with Revit's `Name` and `ObjectType` | 0 of 18 | 18 of 18 |
| slab edges with Revit's `Name` and `ObjectType` | 0 of 59 | 59 of 59 |
| ramps with Revit's `ObjectType` | 0 of 2 | 2 of 2 |

- Revit names each ramp with a `:1` part suffix ("Ramp:Ramp 1:832422:1"), as it names the second part of a two-part floor `:2` (RE-63 §2). rvt-rs writes the ramp once, so its `Name` has no suffix.
- No other element changes. Core Interior, RE1 Architecture and Einhoven export byte-identical IFC apart from the timestamp. On Snowdon, a per-element comparison of class, `Name`, `ObjectType`, storey and every property set finds only the 79 elements above.

-----

## 1. Panels that are walls

A curtain panel's record keeps the panel category (`OST_CurtainWallPanels`, -2000170) when its cell holds a wall. RE-44's type join looks for the one type-definition record of the element's own category that its reference list names. A wall panel names no panel type, so it found none.

`examples/probe_re64_panel_wall_types.rs` lists every curtain-panel record with the panel types and wall types its reference list names. On Snowdon:

| panel records | panel types named | wall types named | Revit's export |
|---:|---:|---|---|
| 486 | 0 | none | family panels, named through their name entries (RE-38) |
| 18 | 0 | 1506825, 4 compound layers, "Exterior - GWB on Metal Stud w Insulated Panel 8 7/8"" | `Basic Wall:` that type, all 18 |
| 23 | 0 | 1005464, no compound layers, "Solar Wall Window Frame" | none of the 23, and no element of that type |

So `attach_panel_wall_types` gives an unnamed panel the one wall type its list names, and only when that type has compound layers (`partition_compound_structure::scan_type_layers`). Only a Basic Wall type has them, so the family is Basic Wall (`system_family("CurtainWallPanel", true)`). The 23 panels naming a type without layers stay `CurtainWallPanel-ElementId`, with no `TypeName`.

The 18 panels stay `IfcPlate`, which is also Revit's class for them.

## 2. Slab edges and ramps

Both already carried their type's name as `TypeName` (RE-44). Slab Edge and Ramp are each the only system family of their category, so `system_family` now maps them, as it maps a floor to Floor (RE-63).

## 3. What stays unnamed on Snowdon

| elements | rvt-rs | Revit | why |
|---:|---|---|---|
| 258 | `WallSweep-ElementId` | the ElementId alone, no `ObjectType` | Revit writes no family or type |
| 170 stringers, 43 runs, 17 landings, 26 stairs | `StairsStringer-ElementId` and so on | "Assembled Stair:Stair:620883 Run 1", `ObjectType` "Non-Monolithic Run:…" | the stair's instance name and part numbering, and the Monolithic or Non-Monolithic kind, are not read |
| 30 railings | `Railing-ElementId` | "Railing:Gate:1991314" | their types' names are not read (#322) |
| 7 | `GenericModel-ElementId` | "Model Text:10" Trebuchet MS:1448731" | model text has no type join yet |

## 4. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc (local only) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |
| 2024_Core_Interior.rvt | `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014` |
| RE1-Architecture.rvt | `4a78bdd68e7f4dd7806060db07c222da9a810e20b3f303c17a443294b91c8ce4` |

```bash
cargo run --profile ci --example probe_re64_panel_wall_types -- MODEL.rvt > rows.tsv
./target/ci/rvt-ifc MODEL.rvt -o model.ifc
python3 tools/re/names_vs_ifc.py model.ifc REVIT_EXPORT.ifc
```

On Snowdon the scorer gives 18 `IfcPlate`, 59 `IfcBuildingElementProxy` and 2 `IfcRamp` system-family elements whose `ObjectType` equals Revit's, the 18 and 59 with `Name` equal too. The witness observations do not change.
