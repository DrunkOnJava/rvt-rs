# RE-54 — Wall bodies from their centreline

**Date:** 2026-09-23
**Result:**
- **Positive.** The location line a wall's data stores (RE-49's bounded line) is the wall's centreline, whatever its location-line setting. Revit's IFC4 body lies exactly half the type's thickness either side of it on 964 of Snowdon Towers' 1,054 walls whose type's layers are read (RE-53).
  - All 964 carry 1 in the word after the setting.
  - None of the 23 walls with 0 or 2 there is centred.
- rvt-rs now builds a Revit 2024 wall's body from its centreline at its type's thickness, where the record box does not already give it.
  - **Angled walls.** The 102 on Snowdon were drawn as their whole axis-aligned record box, up to 9.2 ft off across. Their faces are now exactly Revit's on 75 (none before). Their ends are exact on 13 (1 before): the walls where a rectangle of the type's thickness closes the record box.
  - **Axis-parallel walls whose box is wider than their type.** For example, CMU retaining walls. Their faces are now exact too: 927 of 976 axis-parallel walls, up from 918.
  - **Walls drawn in layers.** RE-53's per-layer drawing needs a body as thick as its layers, so it now applies to 967 walls (883 before). Every layer centre is within 0.001 ft of Revit's on 935 of 946 comparable walls.
- Per wall, taking the larger of its face and end differences from Revit's body: 71 better, 1,003 unchanged, 4 worse.
- Core Interior's walls are unchanged, and its IFC is byte-identical apart from its timestamps.

**Probe:** the measurements in §1 used a scratch reader of each wall's bounded line, orientation words, type thickness and raw record box. The wall's words are now read by `partition_compound_structure::wall_orientation`.

**End-to-end check:** `tools/re/wall_bodies_vs_ifc.py` reads the GLB `rvt-gltf` writes and Revit's IFC4 export of the same file. It measures each wall's body in plan along and across Revit's own wall direction, and reports how far rvt-rs's faces and ends lie from Revit's. Commands, input hashes and output are in §3.

**Oracles:**
- Snowdon Towers Sample Architectural, Revit 2024 (local only, no licence), with Revit's IFC4 export of it (`bldrs-ai/test-models`).
- 2024_Core_Interior with its `IFC Exports/2024_Core_Interior_slim.ifc` (MIT), where every wall is axis-parallel and must not change.

-----

## 1. The stored line is the centreline

For each wall, the scratch reader took:
- the bounded line in its data;
- the location-line setting, the word and the flip after `ff ff ff ff 01 00 00 00` (RE-53);
- its type's layers summed as its thickness T.

Revit's IFC4 body was meshed by IfcOpenShell and measured across the wall, from the line, in units of T:

| location-line setting | walls | body from −T/2 to +T/2 |
|---|---:|---:|
| 0, wall centreline | 276 | 268 |
| 1, core centreline | 130 | 87 |
| 2, finish face exterior | 387 | 362 |
| 3, finish face interior | 261 | 247 |

So the setting says where the user drew the wall, not where the stored line lies: the stored line is always the centreline.

The word after the setting separates the bodies that are not centred:

| word | centred | not centred |
|---:|---:|---:|
| 1 | 964 | 67 |
| 2 | 0 | 21 |
| 0 | 0 | 2 |

The 67 with word 1 that are not centred are mostly soffit beam wraps, which Revit cuts back where they wrap a beam, and walls with edited profiles. The word's meaning is not identified. Only walls with word 1 get a centreline body.

Along the wall, the line runs from joint to joint. On the 102 angled walls:
- 126 of 214 faces at an end no other line shares stop at the line's end. Most of the rest end on the middle of another wall (a T-junction), which the line's ends do not show.
- At a joint of two walls, 115 of 156 faces stop on one of the other wall's faces, butt-joined: running on to its far face, or stopping at its near face. 4 stop at the joint, none fits a mitre, and 37 fit none of these. Which wall runs on is RE-26's open question (#238).

## 2. The body

`partition_schema_mvp::attach_wall_layers` gives each 2024 wall with word 1 its centreline and its type's thickness (`m_wall_axis_*`, `m_wall_type_thickness`). `export_content::wall_centreline_body` builds a rectangle T across, centred on the line:

- **An axis-parallel wall** keeps the record box's extent along its run, which RE-26's joins trimmed. It takes its position and thickness across from the line and the type.
  - A box thinner than the type is a wall Revit cut, such as a soffit wrap. It keeps its box.
- **A wall at an angle** has two cases.
  - Where a rectangle T across along the line fits its record box exactly (both plan extents agree on its length within 1e-4 ft) and the box is centred on the line, that rectangle is its body.
  - Otherwise it runs from one end of its line to the other. It keeps its box instead if the box is more than 2 × max(T, 1 ft) wider than that rectangle's: a leaning or profiled wall, such as Snowdon's "Generic - 4"" walls, whose box is 23 ft wider.
  - An angled wall's body is an `IfcExtrudedAreaSolid` of an `IfcRectangleProfileDef`, placed at its centre and rotated to its line.

The centreline body replaces the box only where the two differ. It is labelled `BodySource` `partition_wall_centreline`, with `ThicknessSource` `wall_type_compound_structure`. A wall the box already drew right is unchanged, which is why Core Interior's output is.

rvt-rs's IFC of Snowdon, meshed by IfcOpenShell, equals its GLB wall for wall on all 1,078 walls to 1e-5 ft.

## 3. End-to-end measurement

Inputs:

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc (Revit 24.0.20.20) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |

Commands (IfcOpenShell 0.8.5):

```bash
cargo build --profile ci --bin rvt-gltf
./target/ci/rvt-gltf "Snowdon Towers Sample Architectural.rvt" -o snowdon.glb
python3 tools/re/wall_bodies_vs_ifc.py snowdon.glb "Snowdon Towers Sample Architectural_IFC4.ifc"
python3 tools/re/wall_layers_vs_ifc.py snowdon.glb "Snowdon Towers Sample Architectural_IFC4.ifc"
```

`rvt-gltf` writes 5,616,816 bytes (6,105 building elements). Walls within each distance of Revit's body, before (RE-53, #359) and after:

| | walls | within 0.001 ft before | after | within 0.1 ft before | after | worst before | after |
|---|---:|---:|---:|---:|---:|---:|---:|
| faces, axis-parallel | 976 | 918 | 927 | 967 | 967 | 0.604 ft | 0.604 ft |
| faces, angled | 102 | 0 | 75 | 2 | 75 | 9.199 ft | 6.132 ft |
| ends, axis-parallel | 976 | 338 | 338 | 482 | 482 | 1.394 ft | 1.394 ft |
| ends, angled | 102 | 1 | 13 | 22 | 31 | 2.310 ft | 2.498 ft |

Per wall, taking the larger of its face and end differences: 71 better, 1,003 unchanged and 4 worse.
- The 4 are angled: rainscreen walls 1340497 (2.31 to 2.50 ft), 1369253 and 1369254 (0.22 to 0.28 ft and 0.43 to 0.53 ft), and chase wall 1533756 (0.073 to 0.078 ft).
- Their faces are now exact, but their ends run to the joint where Revit's stop short or run on.

The layers scorer on the same GLB:
- 967 walls drawn in layers;
- every layer centre within 0.001 ft on 935 of 946 comparable walls, and within 0.126 ft on all;
- 1,674 compared layer colours equal to Revit's, and one layer by category drawn in the category colour.

The 11 beyond 0.001 ft are RE-53's soffit wraps. `examples/probe_re53_wall_layers.rs` counts 973 of the 1,054 walls whose layers are read as drawn in layers. The other 81 have a body that is still not as thick as their layers: cut, profiled, word 0 or 2, or refused as above.

On Core Interior, the scorer against the slim export gives 360 of 360 faces and 351 of 360 ends within 0.001 ft, before and after. The 9 are RE-26's L-corner ends (#238).

## 4. What this does not do

- **Angled walls' ends at joints.** The body runs to the joint, where Revit butts one wall against the other (§1). Which wall runs on is RE-26's open question. A wall ending on the middle of another (a T-junction) is not found by its line's ends.
- The 67 word-1 walls whose body Revit cuts or profiles, and the 23 walls with word 0 or 2, keep their record box.
- Revit 2025 walls: location lines are read on 2024 only (RE-49).
- The meaning of the word after the location-line setting.
