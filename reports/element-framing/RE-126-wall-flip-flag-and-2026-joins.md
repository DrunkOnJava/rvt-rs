# RE-126: a wall's flip byte and its second flag; Revit 2026 wall join lists

**Date:** 2026-09-28
**Issues:** #421 (Revit 2026); follows RE-54 (wall orientation), RE-70 (join lists) and RE-71 (layered joins)
**Artefacts:**
- the flowbim.ee house (Revit 2026, `.rvt` sha256 `eeff78f3d07dc233…`), with Revit 26.4's IFC4X3 CoordinationView export (`c9e2986c0f349b3a…`);
- `2024_Core_Interior.rvt` (MIT), with its slim IFC4 ReferenceView export;
- RE1 Architecture and Electrical (Revit 2025, MIT), Einhoven (2023, MIT) and Snowdon Towers Architectural (2024, local only).

**Probe:** `examples/probe_re126_wall_flip_flag.rs`
**Result:** positive.

## 1. The flip is one byte

After `WALL_FLIP_ANCHOR`, a wall's data holds the location line (`u32`) and the orientation word (`u32`). RE-54 then read the flip as a `u32` of 0 or 1. It is one byte, and the byte after it is a second flag:
- 0, followed by `00 00`, or
- 1, followed by `ff ff` and then `ff ff 20 10 …`.

What the flag sets is not measured. The probe's census (the six bytes after the word):

| file | walls | `00 00 00 00` | `01 00 00 00` | `00 01 ff ff` | `01 01 ff ff` | read before | read now |
|---|---:|---:|---:|---:|---:|---:|---:|
| Revit 2026 house | 59 | 17 | 7 | 33 | 2 | 24 | **59** |
| Core Interior (2024) | 360 | 349 | 7 | 4 | 0 | 356 | **360** |
| RE1 Architecture (2025) | 8 | 7 | 1 | 0 | 0 | 8 | 8 |
| Snowdon Towers (2024) | 1,120 | 665 | 439 | 0 | 0 | 1,103 | 1,103 |

Snowdon's 16 other walls have `ff` bytes after the anchor. They are something else and stay unread. A wall that sets the flag had no exterior side, no layer set, and no centreline body.

## 2. Revit 2026 join lists

A 2026 wall's join list is laid out as on 2025: `07 · tag · 02 · 0 · count · count × (u32 · u64 ElementId · u32)`. It carries the same tag (`0x0b9f7d`) as Core Interior and Snowdon, so `wall_join_count_offset(2026)` is 16. The lists are sparse on this house: 2 of 59 walls name another.

## 3. Measured

**The 2026 house**, against Revit's export:
- **Layer sets** (`tools/re/layer_sets_vs_ifc.py`): 61 written (26 before). All 61 are named as Revit names them and are as thick as Revit's. Layer names are Revit's, in Revit's order, on 320 layers (183 before).
- **Wall faces and heights** (`wall_bodies_vs_ifc.py`): faces are Revit's on 59 of 59 walls and heights on 37, both unchanged.
- **Wall ends.** This export is CoordinationView. It writes each layered wall as one box, a rectangle profile or a clipped extrusion with an `IfcMaterialLayerSet`: at a corner the wall that runs through reaches the partner's far face, and the one that stops ends at its near face. rvt-rs draws a layered wall layer by layer, and at a corner its layers nest (RE-71, measured on Snowdon's ReferenceView bodies). So the union of our layers is compared with Revit's box on each end:
  - Union of layers against Revit's body: 25 of 59 walls have both ends within 0.001 ft (33 with the join lists alone, 35 on `main`). The wider layers of a wall that stops reach past Revit's box.
  - The box a CoordinationView body implies from our joins (the furthest layer where the wall runs through, the nearest where it stops, the line's end where it has no join): 85 of 118 ends where Revit puts them. Before, 27 of 48 were, on the 24 walls that had a centreline.
  - Where Revit's true layered geometry lies at these corners is not measured: no ReferenceView export of this house exists.
- **What stays off:** 21 ends, each a partition ending where two stone walls meet at an L (three walls at one point, left alone by RE-70) or at a corner the lists do not decide. The second list next to the join lists, tag `0x0b9f7f`, names each joined wall with a list of `u32`s. It is not decoded.

**Core Interior:** 407 layer sets written (403 before), all as thick as Revit's. Wall bodies are unchanged (360 faces and 351 ends within 0.001 ft), and both witness observations are regenerated (PASS).

**Every other file** (RE1 Architecture, RE1 Electrical, Einhoven, Snowdon Towers): the IFC is byte-identical apart from its timestamp.

## 4. Tooling

`tools/re/wall_layers_vs_ifc.py::site_to_internal` read the site placement in the file's length unit but the meshes in metres. On a millimetre export such as the 2026 house every point was off by the site's offset × 1000, so it now scales the placement by `calculate_unit_scale`. Exports in metres are unchanged.
