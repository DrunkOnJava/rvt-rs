# RE-120 — Revit 2023 walls cut back at their joins

**Date:** 2026-09-28
**Issues:** #421 (Revit 2023); applies RE-26 and RE-29 (2024 wall joins) to RE-81's 2023 records
**Artefacts:** `255ribeiro/intro_ifc` `Exemplo_data.rvt` and `modelo_bim.rvt` (Revit 2023, local only, no licence), each with Revit's own IFC4 export; `Revit_IFC5_Einhoven.rvt` (Revit 2023, MIT, magnetar-io), with no export.
**Result:** positive. A 2023 wall's record box is the untrimmed wall, as RE-26 found on 2024, and a 2023 record's reference list names the walls it joins, as RE-29 found on 2024. 2024's join solver, run on the 2023 records unchanged, gives every wall of both projects Revit's exported box.

## 1. The observation

On Exemplo_data, 15 of 17 walls' boxes run 0.1 m, half the 0.2 m wall thickness, past Revit's at one or both ends. At an L or T joint the record box of the wall that stops runs to the other wall's centreline, where Revit's export stops it at that wall's face. Wall 335805 ends at y = -3.147 m, the centreline of wall 335756 (-3.247 to -3.047 m), and Revit's export ends it at -3.047 m.

This is the 2024 picture RE-26 measured on Core Interior: the box is the location line to its raw endpoints, inflated by half the thickness, and the trim at an end is half the thickness of the wall whose centreline lands there. RE-29's condition, that the trimming wall be named in the record's reference list, needs the list, and a 2023 record has one: RE-81's two counted lists of `u32` ElementIds, which already carry door and window hosts (RE-85) and Levels (RE-107).

## 2. The change

`partition_schema_mvp::recover_2023_records` runs `element_record_wall_joins::join_trims` over the selected 2023 wall records and applies each trim as on 2024 (`apply_wall_join_trim`, body source `partition_element_record_join_trimmed`). No threshold, offset or rule is new; the solver declines the same cases it declines on 2024 (disagreeing thicknesses, square plan boxes, a trim that would collapse the run).

## 3. End-to-end measurement

Wall boxes against Revit's export, world axis-aligned, exact to 1e-3 m:

| | Exemplo_data | modelo_bim |
|---|---:|---:|
| walls | 17 | 4 |
| Revit's box, before | 2 | 4 |
| Revit's box, RE-120 | 17 | 4 |

The GLB, scored by `tools/re/wall_bodies_vs_ifc.py` along Revit's wall axes: Exemplo_data's ends are within 0.001 ft on 17 of 17 walls (2 before, worst 0.328 ft), faces 17 of 17 before and after; modelo_bim's 4 of 4 before and after. modelo_bim's walls meet columns, not each other; the solver trims them to the same box.

Einhoven: 2 of its 26 walls are record-box walls, and both are now trimmed; the other 24 are its ArcWall-carrier walls and unchanged. It has no export to score them against.

Every 2024 and 2025 file is unchanged: the new call is in the 2023 branch only.

| file | sha256 |
|---|---|
| Exemplo_data.rvt | `c647c521262860bfa765fefda0d4c7542c9164a389d6af1c08b3d31d142588e2` |
| Exemplo_data.ifc | `42fdb8b0b540ebb993064e347ddf4a3e50d329f5cd8f45d0b6c73ee7a7da8c4a` |
| modelo_bim.rvt | `e06107f54481f408995ddf8f66093f1e4c4bc576ac0a3ff186edc4d32e21ff62` |
| modelo_bim.ifc | `58a6aa2872a818a43ed383e833a3a88ec768ef20ef75658bcbd9f7e846fab47f` |

```bash
./target/ci/rvt-gltf Exemplo_data.rvt -o exemplo.glb
python3 tools/re/wall_bodies_vs_ifc.py exemplo.glb Exemplo_data.ifc
```

## 4. Open

- Two small projects of axis-parallel walls, with no true L corner of the kind RE-29 §4 leaves one end wrong on. Angled and layered joins (RE-70, RE-71, RE-74) are not applied on 2023: 2023 join lists (RE-70's element-data frames) are unread.
- Also measured, not changed: every door's record box runs past Revit's body on one side by about the leaf width, on 2023 and on RE1 Architecture (2025) alike; 2023 windows reach 0.36 to 0.39 m outside their wall; 2023 beams run to the column centres, where Revit cuts them at the column faces (0.3 m each end on modelo_bim's eight).
