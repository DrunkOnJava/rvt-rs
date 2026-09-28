# RE-106 — Curtain mullions and panels are boxes along their own axes

**Date:** 2026-09-28
**Issues:** follows RE-46, RE-72, RE-87 and RE-90
**Result:**
- A curtain mullion's or panel's record box is the axis-aligned box of a box along the instance's own axes: the X, Y and Z axes of its transform (RE-87), centred on the record box.
- The half-extents `e` along those axes follow from the record box's half-extents `h` alone, by solving three linear equations: `sum_i |axis_i| e_i = h`. An axis on the model's axes contributes to one equation; a turned or tilted one spreads over two or three.
- A mullion or panel turned or tilted off the model's axes is now drawn as that box. One on the axes keeps its record box, which is already that box.
- Before, all of them were drawn as their record box. For a diagonal mullion or a sloped glazing bay, that box is far larger than the element.

| Snowdon Towers against Revit's IFC4 (`tools/re/oriented_boxes_vs_ifc.py`) | count |
|---|---:|
| mullions turned or tilted, drawn along their axes | 786 |
| panels turned or tilted, drawn along their axes | 233 |
| of those 1,019, Revit's mesh has the box's extent and centre along each of its axes within 0.01 ft | 1,019 (largest gap 0.0000 ft) |

-----

## 1. The solve

Mullion 946566 on Snowdon Towers:
- **Axes:** its transform's X axis is the model's Z, its Y axis the model's −Y and its Z axis the model's X.
- **Record box:** half-extents 0.594, 0.208 and 0.104 ft along the model's X, Y and Z.
- **Solution:** half-extents 0.104, 0.208 and 0.594 ft along its own X, Y and Z, a 2.5 × 5 in mullion 1.19 ft long.

On a turned or tilted instance each record box half-extent mixes two or three of the unknowns. The solve is refused where the absolute axis matrix's determinant is below 0.1, as near 45 degrees, or where a half-extent would not be positive. Neither happens on Snowdon Towers.

`examples/probe_re106_oriented_boxes.rs` counts per class:

| Snowdon Towers | mullions | panels |
|---|---:|---:|
| on the model's axes | 639 | 229 |
| turned or tilted, solved | 786 | 233 |
| no transform read (panels that are walls, #309) | 0 | 23 |

The turned or tilted mullions include tilted ones in sloped glazing, level ones turned in plan, and upright ones whose section is turned.

## 2. The solid and the IFC

The box is an `IfcExtrudedAreaSolid` of an `IfcRectangleProfileDef`, `2e_x × 2e_y`, extruded `2e_z` along the instance's Z axis from the box's lower Z face. Its placement's RefDirection is the instance's X axis. The body source is `partition_family_instance_oriented_box`.

`tools/re/oriented_boxes_vs_ifc.py` reads each box's axes back from that placement, then measures Revit's mesh for the same Tag along them. All 1,019 have Revit's extent and centre along every axis, to 0.0000 ft. Revit's mesh can still be more than a box, a shaped mullion profile for example, but its extents along the instance's axes are the box's.

## 3. Other family instances: a measured negative

The solve holds because a mullion and a panel are boxes along their own axes. The same solve on Snowdon Towers' other 294 family instances off the model's axes does not generally hold. They are planting, furniture, generic models, windows, light fixtures, doors, hardscape and stairs.

Of the 292 in Revit's export:
- 75 match Revit's extents;
- 178 differ;
- 39 leave the solve undetermined.

A shape that is not a box along its axes has a smaller world box than its box along them, so the record box overstates the solved extents. Those instances keep what they had: RE-87's plan turn for upright ones, and the record box otherwise.

## 4. Other files

- **Core Interior:** has no curtain mullions or panels.
- **RE1 Architecture, the MIT house (2024, 2025):** every mullion and panel is on the model's axes.
- **The rest:** Core Interior, RE1, the MIT house, Snowdon Towers structural, Einhoven, Projeto1, `teste_export_2025`, Exemplo_data and modelo_bim are byte-identical to main's apart from the timestamp. Snowdon Towers is not a witness artifact, so no witness observation changes.

## 5. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc (local only) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |

```bash
cargo run --profile ci --example probe_re106_oriented_boxes -- "Snowdon Towers Sample Architectural.rvt"
./target/ci/rvt-ifc "Snowdon Towers Sample Architectural.rvt" -o model.ifc
python3 tools/re/oriented_boxes_vs_ifc.py model.ifc "Snowdon Towers Sample Architectural_IFC4.ifc"
```
