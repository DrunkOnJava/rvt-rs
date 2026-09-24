# RE-55 — Revit 2025 walls: their location line, sides and layers

**Date:** 2026-09-23
**Result:**
- **Positive.** A Revit 2025 wall stores its location line, its orientation words and its type's layers exactly as 2024 does (RE-49, RE-53, RE-54). One house saved in Revit 2024 and in Revit 2025 reads the same values from both files on all 50 of its walls: the same line ends, location-line setting, word, flip and layer widths.
- rvt-rs now reads a wall's line on 2025 as well as 2024. So 2025 walls get RE-54's centreline bodies and RE-53's layers.
  - That house's 45 exported walls draw identically from its 2024 and its 2025 file: the same 127 layer nodes, the same plan to 1e-4 ft.
  - Against Revit's own 2025 exports:
    - RE1 Architecture's 7 walls and Projeto1's 1 are axis-parallel and stay exact.
    - 2 of teste_export_2025's 3 angled walls were boxes up to 38.3 ft off across. They now have Revit's faces and ends to 0.001 ft. The third is the wall that export shows in a later state of the model (RE-44): the file's type is 200 mm, the export's 300 mm.
- Beams (RE-49) and sketch lines (RE-50) keep reading their line on 2024 only. Their 2025 geometry is not measured.

**Oracles:**
- The MIT 4.567 course's copy of the Autodesk tutorial house, saved in Revit 2024 and Revit 2025 (`demo-01-Source_House_2024_tn2.rvt`, `..._2025_tn2.rvt`, cat2.mit.edu, "copyright reserved", local only). The same model in two releases, so the 2025 bytes must say what the 2024 bytes say.
- RE1 Architecture (MIT), Projeto1 and teste_export_2025 (local only), each with Revit's own IFC4 export, for geometry.

-----

## 1. The same house in two releases

| file | sha256 |
|---|---|
| demo-01-Source_House_2024_tn2.rvt (Revit 2024, build 20230911_1230) | `763f239e70db1ac90e5aa806ec4e6a2cebdc3e8a2e902bcfda6b8de43d2fdcfb` |
| demo-01-Source_House_2025_tn2.rvt (Revit 2025, build 20241202_1040) | `28440eaf8b31aa9e91780c75661c3354fa1fad13976bcaf52675c4d11c1a1b6f` |

Both files hold the same 50 wall ElementIds. For each wall, a scratch reader took the fields from its element data (the header of each release, RE-44):
- the first bounded line (`04 00 08 01`, RE-49);
- the three words after `ff ff ff ff 01 00 00 00` (RE-53);
- its type's layers (RE-53).

| field | equal in both files |
|---|---:|
| line start and end | 50 of 50 |
| location-line setting | 50 of 50 |
| word | 50 of 50 |
| flip | 50 of 50 |
| type layer widths | 50 of 50 |

RE-53 and RE-54 measured what these fields mean against Revit's geometry on 2024. The 2025 file carries the same values in the same places, so it means the same.

## 2. What changes

`partition_compound_structure::scan_wall_lines` reads a wall's line on the releases in `WALL_LINE_SUPPORTED_REVIT_VERSIONS` (2024 and 2025). It shares the bounded-line scan with beams through `partition_beam_axes::scan_first_bounded_lines`. `scan_bounded_lines` keeps its 2024-only gate for beams and sketch lines, whose 2025 geometry has no oracle yet. `attach_wall_layers` uses the wall gate.

## 3. End-to-end measurement

Commands (IfcOpenShell 0.8.5):

```bash
./target/ci/rvt-gltf MODEL.rvt -o model.glb
python3 tools/re/wall_bodies_vs_ifc.py model.glb REVIT_EXPORT.ifc
python3 tools/re/wall_layers_vs_ifc.py model.glb REVIT_EXPORT.ifc
```

| file | walls | faces within 0.001 ft before | after | ends within 0.001 ft before | after |
|---|---:|---:|---:|---:|---:|
| RE1 Architecture | 7 | 7 | 7 | 7 | 7 |
| Projeto1 | 1 | 1 | 1 | 1 | 1 |
| teste_export_2025 | 3 | 0 | 2 | 0 | 3 |

- teste_export_2025's worst face difference falls from 38.334 ft to 0.164 ft, which is wall 327589's later state in the export.
- RE1's 7 walls are now drawn in layers. Every layer's material appears more than once in its wall, so Revit's styles cannot place them.
- teste_export_2025's 3 walls are one layer each. Two have Revit's colour; the third is 327589, whose material in the export is the later type's.

The tutorial house has no export of Revit's own. Its 2024 and 2025 GLBs from `rvt-gltf` hold the same 45 walls in 127 nodes, equal in plan to 1e-4 ft.

## 4. What this does not do

- Beams and sketch-line outlines on 2025.
- Releases before 2024.
- The limits of RE-53 and RE-54 (wall joins, cut and profiled walls, material names).
