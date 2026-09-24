# RE-72 — Curtain grids, and panels that hold a wall

**Date:** 2026-09-24
**Issue:** #370
**Result:**
- **The grid names the wall.** A curtain panel names the curtain grid it lies on. The grid's own element data names its curtain wall, at +85 and again at +478. That wall is the one Revit's export aggregates the panel under, on all 18 panels that name such a grid and that Revit aggregates. It settles panels that name several walls, and the nested curtain wall RE-46 gave a wrong parent (1593696).
- **Panels that hold a basic wall.** Revit exports these (RE-64's panel walls) as `IfcCurtainWall`, each with its own body, nested under the curtain wall it is a panel of. rvt-rs now does the same.
- Against Revit's IFC4 export of Snowdon Towers, `tools/re/aggregates_vs_ifc.py`:

| curtain-wall parts | main | RE-72 |
|---|---:|---:|
| mullions with Revit's whole | 1,595 of 1,595 | 1,595 |
| panels with Revit's whole | 463 of 480 | 462 of 462 |
| panels that hold a wall, with Revit's whole and class | 0 of 18 | 18 of 18 |
| parts Revit aggregates and rvt-rs does not | 16 | 0 |
| parts under a different whole | 1 | 0 |
| `IfcCurtainWall`s, bodiless and with a body as Revit's | 42 of 60 | 60 of 60 |

- Panel 1593696 now takes its nested curtain wall's storey. Snowdon has one element on a storey other than Revit's (railing 1646280), against two.
- All 60 curtain walls carry Revit's `Name` and `ObjectType`.
- Core Interior, RE1, the MIT house (2024 and 2025) and Einhoven export IFC byte-identical to main's, apart from the timestamp.
- Measured on Snowdon Towers (Revit 2024) only: no other local file's output changes.

-----

## 1. The grid

RE-46 gives a panel the curtain wall its reference list names, and RE-62 the one whose record box contains the panel's when the list names none. On Snowdon, 16 panels Revit aggregates got neither. Each names several walls: its own curtain wall, the curtain wall that wall is a panel of, and walls it abuts.

`examples/probe_re72_curtain_panels.rs` lists every id a panel or mullion names. Each of the 16 names one id with no element record whose element data names that panel's curtain wall as a `u64` at +85 and again at +478 past the data's ElementId (`partition_schema_mvp::CURTAIN_GRID_WALL_OFFSETS`). On the seven grids looked at byte by byte, that data is 583 bytes and opens with the same `u64`, an id every panel names too.

Across the file, 22 such grids name a wall, and 22 panels name one:

| panels naming a grid | Revit aggregates it | the grid's wall is Revit's whole |
|---:|---|---:|
| 16 | yes; rvt-rs gave none | 16 |
| 1 | yes; RE-46 gave another wall (1593696) | 1 |
| 1 | yes; rvt-rs already right | 1 |
| 4 | no: panels Revit's export leaves out (#309) | |

Revit numbers the grid right after its wall on 6 of the 7 walls first looked at. It doesn't on 1593650, whose grid is 1593687, so the rule reads the grid's data and not its id.

`partition_schema_mvp::attach_curtain_walls` now tries the grid first. A part whose named grids name exactly one wall takes it. RE-46's named wall and RE-62's box come after. A wall a grid names becomes a curtain wall even if no mullion names it.

## 2. Panels that hold a wall

RE-64 names a panel that holds a basic wall with that wall type. Revit's export writes all 18 of them on Snowdon as `IfcCurtainWall`: named "Basic Wall:<type>:<ElementId>", each with its own body, and aggregated under the curtain wall they are panels of. Revit's other 42 curtain walls are bodiless aggregates. rvt-rs wrote the 18 as `IfcPlate`.

`attach_panel_wall_types` now marks them (`m_curtain_panel_is_wall`). The last step of recovery gives them the class `CurtainPanelWall`, which exports as `IFCCURTAINWALL` with the panel's body. It runs last so the naming and layer steps before it still treat them as panels.

## 3. What stays

- **47 panels Revit's export leaves out** are still exported (#309):
  - 24 have the type "Empty" of the "Empty System Panel" family. Nothing found so far marks that type in a locale-independent way.
  - 23 name a type object, "Solar Wall Window Frame", and no name entry. None of the ids they name is in Revit's export.
- **Railing 1646280** stays on a storey other than Revit's.

## 4. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc (local only) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |

```bash
cargo run --profile ci --example probe_re72_curtain_panels -- MODEL.rvt > parts.tsv
./target/ci/rvt-ifc MODEL.rvt -o model.ifc
python3 tools/re/aggregates_vs_ifc.py model.ifc REVIT_EXPORT.ifc -v
python3 tools/re/storeys_vs_ifc.py --any-copy model.ifc REVIT_EXPORT.ifc
python3 tools/re/names_vs_ifc.py model.ifc REVIT_EXPORT.ifc
```

The witness observations do not change: Core Interior's IFC is byte-identical to main's.
