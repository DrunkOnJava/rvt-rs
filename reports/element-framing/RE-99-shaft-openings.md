# RE-99 — Shaft openings cut the floors, roofs and ceilings within their height

**Date:** 2026-09-28
**Issues:** follows RE-25, RE-50 and RE-95 to RE-98
**Result:**
- A shaft opening's element record carries `BuiltInCategory` −2000996 (`OST_ShaftOpening`), and its record box spans its height. Snowdon Towers has 9.
- A shaft is sketched as a floor is. Its sketch also holds the two diagonals of the X that Revit draws across a shaft in plan. They cross each other, which a boundary's lines never do. Left out, the boundary's recorded lines (RE-50) close into the shaft's outline on 8 of the 9 shafts. The ninth's boundary curve is not read.
- Revit's IFC4 export leaves each shaft's outline out of the floors, roofs and ceilings within its height. An element whose outline is one piece, lies within the shaft's height, and holds the shaft's outline strictly inside its outer loop and clear of its voids now takes the outline as a void.

| Snowdon Towers against Revit's IFC4 (top surface, `plan_profiles_vs_ifc.py`) | main | RE-99 |
|---|---:|---:|
| elements carrying a shaft's outline as a void | 0 | 7 |
| slabs within 0.1% of Revit's area | 148 | 150 |

The 7 elements, all closer to Revit's area than before:

| element | main | RE-99 |
|---|---:|---:|
| slab 2113951 | 18.94% | 0.000% |
| slab 2114707 | 0.30% | 0.000% |
| roof 754197 | 1.28% | 0.05% |
| slabs 1423886, 1423899, 1423922, 1423945 | 0.73–0.85% | 0.31–0.35% |

The four 142388x slabs are also cut by shaft 903725, whose boundary is the curve not read, so they keep that residual.

-----

## 1. What is not cut

- **A shaft on an element's edge.** Shaft 759388 lies on the edge of four slabs (931357, 2077111, 2113168, 2113320). Revit's export cuts it from their outer loop as a notch, which is a polygon difference. That is not done, and they stay 3% to 9% over Revit's area.
- **An element of several pieces,** or one the shaft only partly overlaps: left as it is.

## 2. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc (local only) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |

```bash
cargo run --profile ci --example probe_re99_shaft_openings -- "Snowdon Towers Sample Architectural.rvt"
./target/ci/rvt-ifc "Snowdon Towers Sample Architectural.rvt" -o model.ifc
python3 tools/re/plan_profiles_vs_ifc.py model.ifc "Snowdon Towers Sample Architectural_IFC4.ifc" --class IfcSlab,IfcRoof
```

- **Snowdon Towers:** only the 7 elements above change.
- **Core Interior, Snowdon Towers structural, the four RE1 models, Einhoven, the MIT house (2024, 2025), Projeto1 and `teste_export_2025`:** byte-identical to main's apart from the timestamp. No witness observation changes.
