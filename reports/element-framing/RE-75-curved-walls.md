# RE-75 — Curved walls

**Date:** 2026-09-25
**Issue:** #358
**Result:**
- A curved wall's data stores its location arc in the record RE-49 reads as a bounded line (`04 00 08 01`). The `u64` in the 8 bytes before the tag says which curve it is: 0 for a line, 1 for an arc. An arc's record is laid out as below.

  ```text
  -8   u64 1         the record is an arc
  +0   04 00 08 01   the tag
  +4   f64           start angle, radians
  +12  f64           end angle, radians
  +20  f64 × 3       unit X axis
  +44  f64 × 3       unit Y axis, square to X
  +68  f64           radius, feet
  +76  f64 × 3       centre, model feet
  ```

  The arc runs from `centre + radius · (cos a · X + sin a · Y)` at the start angle to the end angle.
- On Snowdon Towers, 32 walls store an arc. On the 24 that Revit's IFC4 export draws with an arc axis, the arc's centre and radius are the axis's within 0.001 ft. The other 8 are in a non-primary design option, which neither export contains.
- Read as a line, the arc's two axes had given a line 0.15 to 17 ft long near the model's origin. The exporter had rejected it and drawn the record box, which overlapped Revit's footprint by a median 0.03 (intersection over union).
- Like a line, the arc is the wall's centreline. Revit's body lies half the type's thickness either side of it on 21 of the 24. Each curved wall is now drawn as that ring sector, from its start angle to its end angle.

| Snowdon Towers' 24 curved walls, `wall_bodies_vs_ifc.py` | main | RE-75 |
|---|---:|---:|
| faces within 0.001 ft of Revit's | 0 | 23 |
| ends within 0.1 ft | 12 | 14 |
| ends within 1 ft | 23 | 24 |
| largest face difference | 15.119 ft | 0.005 ft |
| largest end difference | 5.432 ft | 0.625 ft |
| footprint overlap with Revit's, median (intersection over union) | 0.03 | 0.95 |

- The ends are the arc's own, where Revit trims a wall at its joins. That is the residual: 16 of the 48 ends lie within 0.001 ft of Revit's axis ends, and all within 0.625 ft.

-----

## 1. The record

`examples/probe_re75_wall_arcs.rs` prints each wall whose first curve record is an arc. `tools/re/wall_arcs_vs_ifc.py` finds the centre and radius of each arc in Revit's `Axis` representations from its three points, turned into model feet, and compares them.

```text
24 walls with an arc axis in Revit's export, 32 arcs read
  24 with Revit's centre and radius within 0.001 ft
  8 arcs read on walls Revit's axis draws otherwise
  arc ends against Revit's axis ends: 48; within 0.001 ft: 16 0.01 ft: 17 0.1 ft: 29 1.0 ft: 48; max 0.625 ft
```

A census of the record across every wall's data, on Snowdon Towers (architectural and structural), Core Interior, RE1 and the MIT house:

- A wall's first curve record has `u64` 0 before it and reads as a line on 1,073 Snowdon architectural walls, 56 structural, 1,209 Core Interior, 8 RE1 and 50 MIT walls.
- It has `u64` 1 before it and reads as an arc on 32 Snowdon architectural walls and 2 structural ones. No other file has one.
- Later records in a wall's data carry other values there, up to 14, so the word is read only on the first record.
- Beams (RE-49) have a different word before the tag and are unchanged.

## 2. The ring sector

With the arc's centre, radius and angles, the wall's body is the ring sector between the radius less and plus half the type's thickness. It is drawn as an `IfcArbitraryClosedProfileDef` whose chords stray at most 0.0005 ft inside each arc (`WALL_ARC_TOLERANCE_FEET`). At 0.005 ft the chords of both arcs pulled the drawn band about 0.003 ft towards the centre, which cost the thinnest walls several points of overlap.

A curved wall keeps its "Basic Wall" family name. It gets no layer set, as on main; Revit's export gives each of the 24 a single `IfcMaterial`. The GLB draws each curved wall as one mesh, not in layer bands: its layers cannot be cut from the sector by straight bands, and `layered_extrusion_meshes` finds the sector wider along any one direction than its layers (one GLB node on each of the 24).

## 3. The scorer

`wall_bodies_vs_ifc.py` measured every wall along and across Revit's placement direction. A curved wall has no one direction, so the record box had scored as "right" on some curved walls' extents while overlapping their footprint by 0.03. The scorer now sorts a wall whose Revit axis is an arc into its own `curved` kind. It measures the faces by the least and greatest distance from the arc's centre, and the ends by the first and last angle round it, as feet along the arc at its middle radius.

On Snowdon Towers that moves 16 walls out of the axis-parallel kind and 8 out of the angled kind, so those kinds now count 960 and 94 walls. Their scores are the same on main and RE-75.

## 4. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc (local only) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |

```bash
cargo run --profile ci --example probe_re75_wall_arcs -- MODEL.rvt > arcs.jsonl
python3 tools/re/wall_arcs_vs_ifc.py arcs.jsonl REVIT_EXPORT.ifc
./target/ci/rvt-gltf MODEL.rvt -o model.glb
python3 tools/re/wall_bodies_vs_ifc.py model.glb REVIT_EXPORT.ifc
```

- **Snowdon Towers:**
  - Only the 24 curved walls' IFC changes.
  - The 147 openings and 6 slabs with no Revit GlobalId get new synthetic GlobalIds, because those are numbered by position in the file (#400).
  - `rvt-ifc`'s own IFC and `rvt-gltf`'s GLB agree on all 1,078 walls, and IfcOpenShell meshes every one.
  - `rvt-ifc` takes 10.1 s.
- **Snowdon Towers structural, Core Interior, the four RE1 models, Einhoven and the MIT house (2024 and 2025):** IFC byte-identical to main's apart from the timestamp. None of the structural file's 58 walls gets its type's layers, so none of them, the 2 curved ones included, is drawn from its line or arc, on main or here.
