# RE-100 — A full circle in a sketch

**Date:** 2026-09-28
**Issues:** follows RE-75, RE-96 and RE-99
**Result:**
- A sketch line that is a full circle is stored in RE-75's arc layout under the tag `04 00 08 00`, where a bounded line's or arc's ends in `01`. Its two angles are 0. The rest is as an arc's: unit X and Y axes, radius, centre.

  ```text
  04 00 08 00 · f64 0 · f64 0 · f64×3 X · f64×3 Y · f64 radius · f64×3 centre
  ```

- Shaft 903725 on Snowdon Towers is round. Its sketch is circle 903763 (radius 1.0417 ft, centre (5.917, 21.792)) and the X of two diagonals Revit draws across it, which end on the circle. The circle is now read, drawn as chords within 0.0005 ft (RE-96). Only straight lines can be the crossing diagonals left out (RE-99), so the circle alone closes the shaft's outline.

| Snowdon Towers against Revit's IFC4 (top surface, `plan_profiles_vs_ifc.py`) | main | RE-100 |
|---|---:|---:|
| slabs within 0.1% of Revit's area | 150 | 154 |

| element | main | RE-100 |
|---|---:|---:|
| slab 1423886 | 0.353% | 0.001% |
| slab 1423899 | 0.334% | 0.000% |
| slab 1423922 | 0.340% | 0.000% |
| slab 1423945 | 0.309% | 0.013% |
| roof 754197 | 0.051% | 0.000% |
| roof 1916596 (a face set in Revit's export) | 298% | 278% |

-----

## 1. Scope

The circle is read only for shafts (RE-99). No floor, roof or ceiling sketch in the local files was seen to hold one. Roof 1916596 is sloped, and Revit writes it as a tessellated face set, so its top surface is not its outline; it takes the shaft's round void as the other elements do.

## 2. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc (local only) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |

```bash
./target/ci/rvt-ifc "Snowdon Towers Sample Architectural.rvt" -o model.ifc
python3 tools/re/plan_profiles_vs_ifc.py model.ifc "Snowdon Towers Sample Architectural_IFC4.ifc" --class IfcSlab,IfcRoof
```

- **Snowdon Towers:** only the 6 elements above change.
- **Core Interior, Snowdon Towers structural, the four RE1 models, Einhoven, the MIT house (2024, 2025), Projeto1 and `teste_export_2025`:** byte-identical to main's apart from the timestamp.
