# RE-73 — Where a wall stops at a T joint

**Date:** 2026-09-24
**Issue:** #358
**Result:**
- A wall whose centreline ends part way along another wall's centreline makes a T joint. The other wall runs through, and this one stops against it layer by layer, by Revit's layer priorities (1 structure, 2 substrate, 3 thermal or air, 4 finish 1, 5 finish 2):
  - counted from the face it meets, each layer passes the other wall's layers of a larger function number and stops at the first of an equal or smaller one;
  - so a core passes the other wall's finishes and stops at its core, and a finish stops at its face;
  - a single-layer wall stops at the face.
- The join lists (RE-70) name the other wall at only 8 of Snowdon's 334 T ends, so the exporter finds them by geometry: exactly one other wall overlapping in elevation, perpendicular, with the end on its centreline far enough from its ends that the whole width of the stopping wall meets its side.
- On Snowdon Towers that is 334 ends. 280 of the 313 that can be measured are right in every layer against Revit's IFC4 bodies, and 960 of 1,025 layer ends match.

| Snowdon Towers | main | RE-73 |
|---|---:|---:|
| axis-parallel walls with both of Revit's ends | 516 of 976 | 675 |
| angled walls with both of Revit's ends | 18 of 102 | 18 |
| layer ends in the GLB matching Revit's body | 3,925 of 5,634 | 4,290 |

- Per wall on Snowdon: 186 better, 14 worse, faces unchanged. 172 walls gain both of Revit's ends and 13 lose them.
- Revit draws a few T joints as a clean stop at the face, which nothing read predicts: 16 Snowdon ends, and all three on RE1 (Revit 2025), where 3 of the 7 walls keep both of Revit's ends (6 on main). RE-71 found RE1's one layered corner drawn the same way.
- Core Interior's 202 T ends, all between single-layer walls, are right. Its bodies already stopped there (RE-26's trims), so they are unchanged (351 of 360 walls world-exact), but each end now names the wall it stops against.

-----

## 1. The rule

`examples/probe_re73_tee_joins.rs` prints each wall's centreline, exterior side, layers (exterior first, with their functions) and, at each end, the wall it joins and how far past the end the exporter draws each layer. `tools/re/wall_tee_joins_vs_ifc.py` takes the ends that lie part way along the other wall's centreline, slices Revit's IFC4 body in plan along the middle of each layer, and compares how far past the end the layer reaches.

| T joint ends on Snowdon Towers | ends | every layer right |
|---|---:|---:|
| layered | 290 measured (311) | 260 |
| single layer | 23 | 20 |

Every measured layer end, by the layer and the other wall's layer it would pass (a larger function number than its own):

| layer | other wall's layer | Revit passes it | Revit stops |
|---|---|---:|---:|
| structure | substrate, thermal | 20 | 0 |
| structure | finish 1 | 104 | 0 |
| structure | finish 2 | 257 | 19 |
| substrate, thermal | a larger number | 35 | 0 |
| finish 1 | finish 2 | 126 | 28 |

A layer stops at the first of the other wall's layers with an equal or smaller number on 1,007 of 1,021 measured layer ends, and passes it on 14.

## 2. Which ends are T joints

An end is read when:
- no other wall's centreline ends at the same point (that is an RE-70 butt join, or a cross);
- exactly one other wall overlapping it in elevation has the end on its centreline;
- the two are perpendicular;
- the end is at least half the stopping wall's width from both ends of the other wall.

The stopping wall may be the thicker of the two. Of the 81 measured ends where it is, the rule is right on 63; a clean stop at the face would be right on 15.

The last condition is Core Interior's. Eight of its walls end 0.083 ft from the end of the wall whose centreline they reach, inside their own 0.667 ft width. Revit does not join them: its bodies run to that wall's centreline. On Snowdon the 59 such ends stop at the face (45), at the centreline (4) or otherwise (9; one is not measured). With no one outcome, they are left alone.

Free ends on one other wall's centreline that the exporter leaves alone, on Snowdon Towers:

| why | ends |
|---|---:|
| another wall ends at the same point | 15 |
| not perpendicular | 41 |
| the other wall ends within its width | 59 |

## 3. What does not separate the clean stops

Of the 313 measured ends, 33 have a layer the rule gets wrong:
- **16 are clean stops.** Every layer of the stopping wall ends at the other wall's face, although its core could pass the other wall's finish. RE1's three T ends, all walls of one type (finish 1, structure, finish 1), are the same.
- **17 are other shapes.** A layer runs short of or past its line, or, once, a single-layer wall runs on to the other wall's centreline.

Nothing measured tells the clean stops from the staircases:
- **Wall types.** Soffit beam wraps stopping against 3 1/8" chase walls are right on 8 ends and wrong on 7, on the same face of the same types.
- **The record box.** At a T end it runs to the other wall's centreline, the location line's end, on 330 of 332 axis-parallel ends. RE-26's trims then stop it at the face on 308 of the 313 measured ends, right or wrong alike.
- **The join lists.** They name the other wall at 8 of the 334 ends. None of the 16 clean stops is among them, and the rule is right on 2 of the 7 of those 8 that can be measured.
- **Heights.** Where the other wall covers only part of the stopping wall's height (12% to 90% of it), the rule is right on 25 of 30 measured ends, as the whole wall's join. Revit cleans the join over the full height either way.

## 4. In the exporter

`element_record_wall_joins::tee_join`, called by `butt_joins` for every end no other wall's centreline ends at, on Revit 2024 and 2025:
- a single-layer wall gets a `reach_feet` of minus half the other wall's thickness;
- a layered wall gets a per-layer `layer_reach_feet`, drawn as RE-71's staircase outline (`BodySource` `partition_wall_centreline_layer_joins`).

The end names the other wall as its partner, with `runs_through` false. That is `JoinStartWallElementId` / `JoinEndWallElementId` and `JoinStartRunsThrough` / `JoinEndRunsThrough` in the IFC, and "Start: stops at" / "End: stops at" in the viewer's element panel.

## 5. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc (local only) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |
| 2024_Core_Interior.rvt | `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014` |
| 2024_Core_Interior_slim.ifc | `bfdf36ffb0bb768f3409d818403990e64d4c262c6780603be87f8077387ad86d` |
| RE1-Architecture.rvt | `4a78bdd68e7f4dd7806060db07c222da9a810e20b3f303c17a443294b91c8ce4` |
| RE1-Architecture.ifc | `a9b5d36677aa6a8bb91b77d8bc354ed9028a7ca3e9e2491a47ba14cfd8e26200` |

```bash
cargo run --profile ci --example probe_re73_tee_joins -- MODEL.rvt > walls.jsonl
python3 tools/re/wall_tee_joins_vs_ifc.py walls.jsonl REVIT_EXPORT.ifc
./target/ci/rvt-gltf MODEL.rvt -o model.glb
python3 tools/re/wall_bodies_vs_ifc.py model.glb REVIT_EXPORT.ifc
cargo run --profile ci --example probe_re71_layered_joins -- MODEL.rvt > layered.jsonl
python3 tools/re/wall_layer_joins_vs_ifc.py layered.jsonl REVIT_EXPORT.ifc --glb model.glb
python3 tools/re/ifc_world_aabb.py OURS.ifc --type IFCWALL --scale 3.280839895013123
```

- **Snowdon Towers:** the tables above.
  - `rvt-ifc`'s own IFC and `rvt-gltf`'s GLB agree on all 1,078 walls to 0.000 ft, and IfcOpenShell meshes every one.
  - `rvt-ifc` takes 10.0 s.
- **Core Interior:** 202 T ends, all right in Revit's body; 351 of 360 walls world-exact, as on main, and no wall's box changes. The IFC gains the partner properties at those ends, so both witness observations of it are regenerated; they replay to PASS.
- **RE1 (Revit 2025):** 3 T ends, all clean stops in Revit's body; 3 of 7 walls keep both of Revit's ends (6 on main).
- **MIT tutorial house (local only, no Revit export):** 20 T ends on 16 walls, the same in the 2024 and the 2025 save. Its IFC and GLB agree on all 45 walls in both.
