# RE-89 — Window openings in tapered walls

**Date:** 2026-09-28
**Issues:** #227
**Result:**
- Revit's export cuts a window's opening in a tapered wall (RE-86) as a vertical box. It is the host type's thickness deep across the wall and centred on the window's own origin (RE-87), not on the wall's line or its box. On Snowdon Towers every window in a "Solar Wall" host is cut this way.
- rvt-rs cut no opening in these hosts, because the base is wider than RE-83's 3 ft limit. Each opening kept the turned window box. It now takes Revit's cut.

| Snowdon Towers' 27 openings of turned doors and windows, measured in the filler's frame | RE-87 | RE-89 |
|---|---:|---:|
| across the wall, within 0.01 ft of Revit's opening | 4 | 24 |
| 12 windows in Solar Wall (Multi-Level) | 0.773 ft | 0.000 ft |
| 8 windows in Solar Wall (L5) | 0.125 ft | 0.000 ft |
| 3 doors | 4.628 ft | 4.628 ft |

- Along the window and in height, nothing changes.
- A door's opening in a tapered wall is flush with the exterior face at its base instead: 1055444, 1055510 and 1055798 end at the exterior face at the door's sill. Three doors of one wall type are too few to rule on, so they keep their box.

-----

## 1. The opening

Revit's opening bodies were cut at five heights in the host's frame, moved into internal feet through the export's site placement:

- Each opening spans the same range across the wall at every height. It is one type thickness deep: 2.039 ft in the Multi-Level walls, 1.794 ft in the L5 walls.
- Its centre across the wall equals the window's transform origin projected across the host's line: 0.560 ft for 1055807, 1.415 ft for the six Multi-Level windows on one level, and 1.437 ft for the L5 windows. The origin sits 3 ft below the opening, at the window's base point.

## 2. The cut

`ifc::tapered_host_window_cut` runs before RE-83's cut. It applies where:
- the filler is an `IfcWindow` turned along its host's line (RE-87), either way;
- the host's `BodySource` is `partition_wall_tapered_section` (RE-86);
- both the host type's thickness and the window's origin are read.

The opening is the window's width along it, the thickness across, centred on the origin. A turned family instance now reports its origin (`InstanceOriginX`, `InstanceOriginY`), which carries it to the cut.

## 3. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc (local only) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |

- **Snowdon Towers:** only the 20 openings' shapes change. The 181 turned family instances gain the two origin properties.
- **Snowdon Towers structural:** its 5 turned footings gain the origin properties.
- **Core Interior, RE1 Architecture and Plumbing, Einhoven and the MIT house (2024):** unchanged; none has a turned family instance.
- `openings_vs_ifc.py` compares world axis-aligned boxes, which a turned rectangle's does not change, so its score stays 124 of 194 within 0.26 ft.
