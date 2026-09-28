# RE-94 — Door openings at their type's rough opening

**Date:** 2026-09-28
**Issues:** #227
**Result:**
- A door type's value block holds its Rough Width (−1001305) and Rough Height (−1001304) as `f64 feet · ff × 8 · i64 parameter` (RE-93).
- Revit's IFC4 export cuts each Snowdon Towers door's opening centred on its origin (RE-87) along its X axis, from the origin up, at one of two sizes:
  - 57 of 126: the type's Rough Width × Rough Height;
  - 67: Width + 0.208 ft by Height + 0.104 ft;
  - 2 others: the rolling door and a door whose type stores no rough size.
- What decides it is the door's own body. Every door cut at its rough size has a body exactly Rough Height tall (7.333 ft for a 7 ft door), and every other door's body is its frame's height (7.167 ft). rvt-rs's record box is that body, so a door whose record box is its type's Rough Height tall now takes the rough opening. Revit writes the rough size as the door's `OverallWidth` / `OverallHeight`, and rvt-rs now does too.

| Snowdon Towers' 126 doors against Revit's openings (`opening_boxes_vs_ifc.py`) | main | RE-94 |
|---|---:|---:|
| within 0.001 ft | 0 | 54 |
| within 0.1 ft | 121 | 68 |
| within 0.26 ft | 1 | 0 |
| over 1 ft | 4 | 4 |
| `OverallWidth` and `OverallHeight` equal to Revit's | 0 | 57 |

-----

## 1. The two sizes

The doors of one type fall on both sides. For example, 19 doors of "36" x 84" (60 MIN)" are cut at 3.208 × 7.104 ft and 5 at the type's 3.333 × 7.333. The host wall's thickness mostly separates them (frame-sized in partitions up to 7.25 in, rough-sized from 7.625 in), but not on every door. The door's body height separates them on all 126.

The frame-sized opening is the body with 0.0625 ft taken off each jamb and the head. Neither that nor the frame-sized opening's width and height is among the door type's stored lengths, so those doors keep their box opening.

## 2. Doors left alone

- **Opening-only doors (RE-84).** A door whose type draws no geometry is written as its opening alone. RE1 Architecture's cased opening has a record box its Rough Height tall, but Revit cuts it 1.0 m wide against a Rough Width of 1.1 m, so these doors are left out (9 on Snowdon, 1 on RE1).
- **The three doors in Snowdon's tapered Solar Walls** (1055444, 1055510, 1055798) take the rough size along the wall and in height. Across the wall Revit's opening is flush with the exterior face (RE-89), which is not read, so they stay 4.2 ft off there, as before.
- A door is left alone where its record box does not hold the opening with at most 0.5 ft to spare (RE-93's guard). An opening edge on the box's own edge counts as held.

## 3. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc (local only) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |

```bash
cargo run --profile ci --example probe_re94_door_openings -- "Snowdon Towers Sample Architectural.rvt"
./target/ci/rvt-ifc "Snowdon Towers Sample Architectural.rvt" -o model.ifc
python3 tools/re/opening_boxes_vs_ifc.py model.ifc "Snowdon Towers Sample Architectural_IFC4.ifc"
```

- **Snowdon Towers:** only 57 door `IfcOpeningElement` bodies change: 54 to within 0.001 ft of Revit's, and the three tapered-wall doors as in §2. The same 57 doors gain their opening's properties and `OverallWidth` / `OverallHeight`, all equal to Revit's.
- **Snowdon Towers structural, Core Interior, the four RE1 models, Einhoven, the MIT house (2024, 2025), Projeto1 and `teste_export_2025`:** byte-identical to main's apart from the timestamp.
