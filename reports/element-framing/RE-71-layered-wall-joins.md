# RE-71 — Where each layer ends at a layered butt join

**Date:** 2026-09-24
**Issue:** #358
**Result:**
- At a butt join where both walls have several layers, Revit cleans the join layer by layer, and the layers nest like concentric Ls. Count both walls' layers from the outside of the corner:
  - layer `i` of the wall that runs through (RE-70) reaches the outer edge of the partner's layer `i`;
  - layer `i` of the wall that stops reaches its inner edge.
- It holds where both walls show the same layer functions in the same order from the corner and share their base and top. On Snowdon Towers that is 528 ends: 521 are right in every layer, and 1,382 of their 1,402 layer ends match Revit's body.
- Those walls are now drawn as the staircase the joins make, as an `IfcArbitraryClosedProfileDef` and one GLB band per layer. On Snowdon Towers:

| walls with both of Revit's ends | main | RE-71 |
|---|---:|---:|
| axis-parallel | 346 of 976 | 516 |
| angled | 13 of 102 | 18 |

| layer ends in the GLB, against Revit's body | main | RE-71 |
|---|---:|---:|
| right | 3,078 of 5,634 | 3,925 |

- 16 layer ends that were right go wrong, all on walls Revit draws as a clean butt (§3). RE1's one layered joint is drawn as a clean butt, but RE1's export writes every wall as one rectangle, so no layered end shows in it (RE-73 §3). Against it, 6 of 7 walls keep both ends (7 on main), and 38 of 42 layer ends match (42 on main).
- Measured on Snowdon Towers (Revit 2024) and RE1 (Revit 2025). Core Interior is unchanged; its walls have one layer.

-----

## 1. The staircase

`examples/probe_re71_layered_joins.rs` prints each wall's centreline, exterior side, layers (exterior first, with their functions) and its RE-70 join at each end. `tools/re/wall_layer_joins_vs_ifc.py` slices Revit's IFC4 body in plan along the middle of each layer, measures how far past the joint point the layer reaches, and matches it to the partner's layer boundaries (its faces and the lines between its layers).

On Snowdon Towers, 1,919 of the 1,930 layer ends at layered L joints land exactly on one of the partner's boundaries, within 0.002 ft. The most common shapes, both walls' layers counted from the outside of the corner:

| ends | wall | layers, outside first | ends on partner boundary |
|---:|---|---|---|
| 135 | stops | finish 2, structure | 1, 0 |
| 134 | runs through | finish 2, structure | 2, 1 |
| 61 | runs through | finish 2, structure, finish 2 | 3, 2, 1 |
| 49 + 49 | stops | finish 2, structure, finish 2 | the inner edge of each partner layer |
| 11 | stops | finish 1, thermal, thermal, substrate, structure, finish 2 | the inner edge of each partner layer |

Each is the same rule: the through wall's layers run to the outer edges of the partner's layers, rank for rank, and the stopping wall's layers stop at their inner edges. The outermost layer of the stopping wall still reaches past the joint, over the end of the partner's core: finishes wrap the corner.

## 2. Where it holds

| layered join ends on Snowdon Towers | ends | the rule right in every layer |
|---|---:|---:|
| same layer sequence from the corner, same base and top (drawn) | 528 | 521 |
| any other (not drawn) | 139 | 54 |

- **Heights.** Where the two walls differ in base or top, Revit's one body per wall shows the join over part of the wall only. 17 of the 75 such ends with the same layer sequence come out as clean butts.
- **Sequences.** Where the layer sequences differ, layers pair by function around the core rather than rank for rank. An 8-layer demising wall stopping against a 6-layer exterior wall runs its finishes past the exterior wall's core. That pairing is not modelled.

So the exporter draws the staircase only on the 528 ends of the first kind, and keeps the rest as before.

## 3. What does not separate the clean butts

Of those 528 ends, 7 have a layer the rule gets wrong:
- **Six are clean butts.** Revit runs every layer of the through wall to the far face, or stops every layer of the stopping wall at the near face: walls 837426, 1323736, 1477441 and 1929800, and 1479342 at both ends. The one layered joint on RE1, 416297 against 418450, is drawn the same way, but RE1's export writes every wall as one rectangle and cannot show layers ending apart (RE-73 §3).
- **The seventh**, 752956's end against 752985, follows the rule for its three outer layers and stops its inner three on the third's line.

Nothing measured tells the clean butts from the 522 staircases:
- the entries' last `u32` (k) is 5, 6, 7 or 8 in both groups: 8 on 2 of the 6 clean ones, against 6 of the 522 layered;
- the entries' first `u32` runs 29 to 238 on the clean ones, and 194 of the 522 layered ones are under 250 too;
- the walls' heights, types and layer counts match in both groups.

## 4. The rule

`element_record_wall_joins::layer_reaches`, called by `butt_joins` for every RE-70 join, on Revit 2024 and 2025. It gives a per-layer reach, exterior first, where:
- both walls have the same number of layers, at least two;
- their functions in order from the outside of the corner are the same;
- their bases and tops agree within 0.001 ft.

`partition_schema_mvp` keeps it as `m_wall_join_start_layer_reaches` / `m_wall_join_end_layer_reaches`. `ifc::export_content::layered_wall_profile` turns the body into one band per layer across the wall, each band running from its start reach to its end reach, as an `IfcArbitraryClosedProfileDef` with `BodySource` `partition_wall_centreline_layer_joins`. An end with no layered join keeps the body's end for every layer.

The GLB's layered meshes cut that outline into bands. The band clipper kept vertices on a band's edge where the neighbouring layer runs further. That left zero-width spikes out to the other layer's end, which drew nothing but carried vertices at the wrong length (`body_geometry::clip_ring_to_band` now drops them). Core Interior's GLB is byte-identical with and without that change.

## 5. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc (local only) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |
| RE1-Architecture.rvt | `4a78bdd68e7f4dd7806060db07c222da9a810e20b3f303c17a443294b91c8ce4` |
| 2024_Core_Interior.rvt | `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014` |

```bash
cargo run --profile ci --example probe_re71_layered_joins -- MODEL.rvt > walls.jsonl
./target/ci/rvt-gltf MODEL.rvt -o model.glb
python3 tools/re/wall_layer_joins_vs_ifc.py walls.jsonl REVIT_EXPORT.ifc --glb model.glb
python3 tools/re/wall_bodies_vs_ifc.py model.glb REVIT_EXPORT.ifc
```

- **Snowdon Towers:** the tables above.
  - `rvt-ifc`'s own IFC and `rvt-gltf`'s GLB agree on all 1,078 walls to 0.000 ft, and IfcOpenShell meshes every one.
  - 389 walls carry the staircase body.
  - `rvt-ifc` takes 9.5 s.
- **RE1:** 2 walls carry it; the scorer's walls with both of Revit's ends go from 7 to 6.
- **Core Interior:** byte-identical GLB.
- **MIT tutorial house (local only, no Revit export):** 17 walls carry it in both the 2024 and the 2025 save. Its IFC and GLB agree on all 45 walls.

The witness observations do not change.
