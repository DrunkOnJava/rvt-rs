# RE-98 — Ceilings take their sketch's outline

**Date:** 2026-09-28
**Issues:** follows RE-25, RE-50, RE-95 to RE-97
**Result:**
- A ceiling is sketched like a floor or roof. On Snowdon Towers all 68 ceilings have sketch lines naming them and a Sketch element listing their curves (RE-97). Ceilings now take the outline their sketch closes, through the path roofs take (RE-50, RE-95 to RE-97), where before they were their record box.
- Revit writes a layered ceiling as one solid per layer, stacked, each with the ceiling's outline. `plan_profiles_vs_ifc.py` now measures an element's top surface: its upward faces at its highest level, projected on plan. So each layer counts once, and a slab, ceiling or layered roof is compared with Revit's outline, not with the sum of its layers.

| Snowdon Towers' 68 ceilings against Revit's IFC4 (top surface) | main | RE-98 |
|---|---:|---:|
| plan area within 0.1% of Revit's | 50 | 66 |
| every top-surface vertex within 0.01 ft of Revit's, both ways | 37 | 52 |

| RE1 Architecture's 6 ceilings (Revit 2025) against Revit's IFC4 | main | RE-98 |
|---|---:|---:|
| plan area within 0.1% of Revit's | 5 | 6 |
| every top-surface vertex within 0.01 ft | 5 | 6 |

-----

## 1. What changes

- **Snowdon Towers:** every ceiling now carries an outline. 18 come closer to Revit's area, 16 of them into 0.1%. 50 were rectangles their box already drew, and they are the same rectangle.
- **RE1 Architecture:** ceiling 418801 was its record box, 34% over Revit's area; it is now Revit's outline exactly.
- The two Snowdon ceilings left off, 1558379 and 1558472, are 0.43% and 0.48% over Revit's area (before, their boxes were 135% and 133% over). Revit's shapes differ there by up to 17.6 ft at a vertex; the difference is not read.

## 2. The scorer

Under the top-surface measure, slabs on Snowdon Towers read 131 within 0.1% on main before RE-97 and 148 after it. RE-97's all-upward-face count (89 to 102) summed Revit's stacked layers. A sloped roof's top surface is only its highest faces, so roofs are compared with the all-upward measure (RE-50 to RE-97), not this one.

## 3. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc (local only) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |
| RE1-Architecture.rvt / .ifc (MIT, Drshelden/IFC-ECS) | as in RE-32 |

```bash
./target/ci/rvt-ifc "Snowdon Towers Sample Architectural.rvt" -o model.ifc
python3 tools/re/plan_profiles_vs_ifc.py model.ifc "Snowdon Towers Sample Architectural_IFC4.ifc" --class IfcCovering
```

- **Snowdon Towers:** only the 68 `IfcCovering` bodies change.
- **RE1 Architecture:** only its 6 `IfcCovering` bodies change.
- **Core Interior, Snowdon Towers structural, RE1 Electrical, Mechanical and Plumbing, Einhoven, the MIT house (2024, 2025), Projeto1 and `teste_export_2025`:** byte-identical to main's apart from the timestamp. No witness observation changes.
