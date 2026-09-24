# RE-74 — Where walls end when they meet at an angle

**Date:** 2026-09-24
**Issue:** #358
**Result:**
- Where two walls meet at an angle other than a right angle, Revit ends each layer on a slanted line: the line of the other wall's face or layer boundary that RE-70, RE-71 and RE-73 give at a right angle. The rules are unchanged, and each layer's end is where its two edges meet that line.
  - **L joints** (both centrelines end at one point, RE-70's join lists deciding which wall runs through): between single-layer walls, the wall that runs through reaches the other's far face and the other stops at its near face. Between layered walls that match from the outside of the corner and share their base and top, layer `i` of the through wall reaches the outer edge of the other's layer `i`, and layer `i` of the wall that stops, its inner edge.
  - **T joints** (RE-73): each layer of the stopping wall passes the other wall's layers of a larger function number, counted from the face it meets, and stops at the first of an equal or smaller one. A single-layer wall stops at the face.
- On Snowdon Towers Revit draws them this way where the corner between the two walls is 82.5 to 114.5 degrees. None of the 14 L ends at sharper corners (7.5 and 24.5 degrees) or shallower bends (148 to 172.5 degrees, walls that nearly continue in line) follows them, so the exporter reads an angled join only between 45 and 135 degrees.

| Snowdon Towers, `wall_bodies_vs_ifc.py` / `wall_layer_joins_vs_ifc.py --glb` | main | RE-74 |
|---|---:|---:|
| angled walls with both of Revit's ends | 18 of 102 | 34 |
| axis-parallel walls with both of Revit's ends | 675 of 976 | 689 |
| GLB layer ends matching Revit's body | 4,290 of 5,634 | 4,394 |

- Per wall on Snowdon: 33 better, 1 worse, faces unchanged. 30 walls gain both of Revit's ends, and none loses them.
- Every angled L join the exporter draws is right along every layer edge (14 ends, 48 edges). At angled T joints, 18 of 22 ends are, and 187 of 195 edges.
- Every other local file exports byte-identical IFC, so the witness observations do not change.

-----

## 1. What the exporter draws

`examples/probe_re73_tee_joins.rs` prints each wall end's join with its layers' reaches, now a pair per layer: along the layer's exterior-side edge and its interior-side edge. `tools/re/wall_angled_joins_vs_ifc.py` takes every end whose joined wall meets it at an angle, slices Revit's IFC4 body in plan just inside each edge of each layer, and compares the reaches.

| angled joins on Snowdon Towers | ends | every layer edge right |
|---|---:|---:|
| L, single layer | 12 | 12 |
| L, layered | 2 | 2 |
| T, single layer | 2 | 2 |
| T, layered | 20 measured (21) | 16 |

| layer edges | right | wrong |
|---|---:|---:|
| L | 48 | 0 |
| T | 187 | 8 |

The four T ends with a wrong layer edge are two configurations: walls 2010610, 2010935 and 2010943, one six-layer wall repeated on three storeys, each stopping against wall 1699930; and wall 2079746 (§3).

Ten more layered L ends name the wall they join but draw no layer ends, because the two walls' layers differ from the corner or their heights differ, as at right angles (RE-71 §2).

## 2. The angle

On Snowdon Towers, 40 L ends join two walls at an angle, one other wall's centreline ending at the same point. By the corner between the two walls (the angle between their directions from the joint), and what Revit's body does:

| corner | the rule's slanted ends | something else |
|---|---:|---:|
| 7.5 and 24.5 degrees (sharp) | 0 | 6 |
| 65.5 degrees | 0 | 2 |
| 82.5 degrees | 2 | 0 |
| 97.5 degrees | 5 | 7 |
| 114.5 degrees | 7 | 3 |
| 148 to 172.5 degrees (shallow bends) | 0 | 8, one mitred |

The 12 "something else" ends at 65.5 to 114.5 degrees are all layered joints the rule does not draw (§1). Outside 45 to 135 degrees nothing follows the rule, so `ANGLED_JOIN_MAX_COSINE` stops there.

## 3. The one worse wall

Wall 2079746 stops at an angled T joint against wall 698364. Its structure layer now ends where Revit's does at every offset measured, 0.455 to 0.494 ft short of its line's end, where main drew it to the line's end. Its 5/8-inch interior finish is the exception: Revit runs it 0.508 ft past the end, and the rule stops it 0.554 ft short. The whole-body score counts the wall's farthest point, so the wall goes from 0.514 to 0.969 ft off, although its structure layer is now right along both edges.

## 4. What is left on angled walls

Of the 170 ends of the 85 angled walls that both Revit's export and rvt-rs's give a body:

| end | right | wrong |
|---|---:|---:|
| no other wall meets it | 48 | 33 |
| angled L | 7 | 15 |
| angled T | 12 | 0 |
| right-angle L | 21 | 9 |
| right-angle T | 1 | 4 |
| three or more walls, collinear | 6 | 4 |

(The table leaves out 10 ends where a slice misses Revit's body.)

- **Free ends.** 16 of the 33 wrong ones have nothing near. On 8 of them Revit's body ends 0.897 or 1.02 ft from the line's end, on one face or both; on the rest it ends up to 12 ft from it. That fits walls whose profile was edited, or whose ends are attached to something rvt-rs does not read. 15 more end inside another wall's body and 2 near one, with no join.
- **Angled L ends** left wrong are the sharp corners and shallow bends (§2), and layered ones not drawn.

## 5. In the exporter

- `element_record_wall_joins::butt_joins` reads an L joint at 45 to 135 degrees when RE-70's lists decide it, and `angled_layer_edges` gives its slanted ends.
- `tee_join` reads a T joint at 45 to 135 degrees, and `angled_tee_edges` gives its slanted ends. Its clearance from the other wall's ends is the stopping wall's half-width divided by the sine of the angle, which at a right angle is RE-73's.
- Both return `ButtJoin::layer_edge_reach_feet`: per layer, exterior first, the reach along its exterior-side and interior-side edge.
- `partition_schema_mvp` keeps it as `m_wall_join_start_layer_edge_reaches` / `m_wall_join_end_layer_edge_reaches`, two values per layer.
- `WallLayerEnds::reach_feet` now holds a pair per layer, equal at square ends. `ifc::export_content::layered_wall_profile` draws each layer's end from one edge's reach to the other's, so a square end is unchanged and a slanted one becomes a slanted edge of the `IfcArbitraryClosedProfileDef`. A single-layer wall is one band.

Right-angle joins keep RE-70's, RE-71's and RE-73's code, so their output is unchanged.

## 6. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc (local only) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |

```bash
cargo run --profile ci --example probe_re73_tee_joins -- MODEL.rvt > walls.jsonl
python3 tools/re/wall_angled_joins_vs_ifc.py walls.jsonl REVIT_EXPORT.ifc
./target/ci/rvt-gltf MODEL.rvt -o model.glb
python3 tools/re/wall_bodies_vs_ifc.py model.glb REVIT_EXPORT.ifc
```

- **Snowdon Towers:** the tables above.
  - `rvt-ifc`'s own IFC and `rvt-gltf`'s GLB agree on all 1,078 walls to 0.000 ft, and IfcOpenShell meshes every one.
  - `rvt-ifc` takes 9.6 s.
  - `wall_tee_joins_vs_ifc.py` now counts 277 layered and 22 single-layer T ends right in every layer, with angled T joints included (260 and 20 before).
- **Core Interior, the four RE1 models, Einhoven and the MIT house (2024 and 2025):** IFC byte-identical to main's apart from the timestamp: none has an angled join the exporter reads.
