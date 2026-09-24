# RE-70 — Which wall runs through a butt join

**Date:** 2026-09-24
**Issue:** #358
**Result:**
- A wall's own element data lists the walls it runs through at a join. Where two walls' centrelines end at one point, Revit runs one of them on to the other's far face and stops the other at its near face. The wall that runs through names the other in its join lists and is not named back.
- Checked against Revit's own IFC4 bodies on Snowdon Towers, and against the record box on five files. The lists never contradict a joint Revit draws cleanly:

| check | agree | undecided | disagree |
|---|---:|---:|---:|
| Revit's body, perpendicular L joints with clean ends | 194 | 2 | 0 |
| Revit's body, angled L joints with clean ends | 18 | 0 | 0 |
| record box, axis-parallel L joints, 5 files | 1,110 | 47 | 2 |

- A wall's body now takes its end from the lists where both walls have a single layer:
  - On Snowdon Towers, 9 walls' ends improve and none gets worse, and 8 more axis-parallel walls have both of Revit's ends: 346 of 976, against 338.
  - Core Interior, RE1 and the MIT tutorial house, in 2024 and 2025, export unchanged.
- Measured on Revit 2024 (Snowdon Towers, Core Interior, the MIT house) and Revit 2025 (RE1, the MIT house). Only Snowdon has a Revit export whose wall bodies the scorer compares.

-----

## 1. The join lists

A wall's element data (its `element_data_header`, its ElementId, and the bytes up to the next element's header) holds, usually twice, a list of this form:

```text
07 00 00 00                   word 7
u32 tag                       one value per document
02 00 00 00
00 00 00 00                   Revit 2025 only
u32 count
count × { u32 · u64 ElementId · u32 }
```

On Snowdon Towers, 412 of the 1,031 walls with a centreline name at least one wall in such lists. Across all of that file's word-7 frames with its tag, the last `u32` of an entry is 5 or 6 on 527 of 567 entries and 7 to 18 on the others. Neither `u32` of an entry is decoded, and which of a wall's two lists belongs to which end is not established. On wall 619404 the first list holds the partner at the line's end; on wall 620046, the partner at its start.

**The tag is a per-document value.** Snowdon Towers, Core Interior and RE1 use `0x000b9f7d`. The MIT tutorial house uses `0x00039f3d` in both its 2024 and its 2025 save. A second frame of the same shape carries the tag plus 2 (`0x0b9f7f`, `0x039f3f`). The tag is:
- not a schema class tag: both 2024 files carry the same 472,791-byte `Formats/Latest` without it;
- not a declared ElementId in any of the files;
- not per build: Snowdon (build 20230405) and Core Interior (20230509) share it, and the MIT house (20230911) differs.

So the reader takes it from the data. Only a frame whose entries all name recovered walls counts, and a file where frames of two tags do gives nothing. On every file measured exactly one tag passes:

| file | release | tag | join lists |
|---|---|---|---:|
| Snowdon Towers Architectural | 2024 | `0x0b9f7d` | 485 |
| 2024_Core_Interior | 2024 | `0x0b9f7d` | 226 |
| MIT tutorial house | 2024 | `0x039f3d` | 28 |
| MIT tutorial house | 2025 | `0x039f3d` | 28 |
| RE1-Architecture | 2025 | `0x0b9f7d` | 1 |

The MIT house's 45 walls give identical lists in their 2024 and 2025 saves.

## 2. Against Revit's bodies

An L joint is a wall end where exactly one other wall, overlapping it in elevation, ends its centreline at the same point. On Snowdon those points coincide to 1e-9 ft, and every perpendicular pair the rule decides is perpendicular to 1e-12. Each end's two side faces in Revit's IFC4 body are measured against the other wall's two faces:
- **through:** both faces end on the other wall's far face;
- **stopped:** both end on its near face;
- **irregular:** anything else. This is Revit's layer-by-layer join cleanup, where finishes wrap the corner.

Snowdon Towers has 908 L-joint ends:

| Revit's end | rule: through | rule: stopped | undecided |
|---|---:|---:|---:|
| through (perpendicular) | 110 | 0 | 0 |
| stopped (perpendicular) | 0 | 84 | 2 |
| through (angled) | 10 | 0 | 0 |
| stopped (angled) | 0 | 8 | 0 |
| irregular, one face on the far face | 286 | 7 | 2 |
| irregular, one face on the near face | 3 | 325 | 7 |

No pair names each other.

**Layers decide whether the end is clean.** Of the perpendicular ends the rule decides:
- where both walls have a single layer, 158 of 158 are clean;
- where either has several, 36 of 667 are clean.

A rectangular body drawn to the rule is therefore Revit's only between single-layer walls, and the exporter applies it only there.

## 3. Against the record box

On an axis-parallel wall the record box already shows the answer. At a perpendicular L joint the box of the wall that runs through reaches half the other wall's thickness past the joint, and the box of the wall that stops ends at it. This carrier is independent of the lists:

| file | box through, rule through | box stopped, rule stopped | rule undecided | disagree |
|---|---:|---:|---:|---:|
| Snowdon Towers | 386 | 394 | 13 | 2 |
| 2024_Core_Interior | 118 | 118 | 18 | 0 |
| MIT house 2024 | 23 | 23 | 8 | 0 |
| MIT house 2025 | 23 | 23 | 8 | 0 |
| RE1-Architecture | 1 | 1 | 0 | 0 |

This is also why RE-26's trims already get most axis-parallel butt joins right, and why the gain below is small. The two disagreements are wall 2040599's two ends. They are irregular in Revit's body: the rule says stopped, and one face reaches the other wall's far face.

## 4. The rule

`element_record_wall_joins::butt_join_reaches`, with the lists read by `partition_compound_structure::scan_wall_join_partners`, on Revit 2024 and 2025. An end is decided when all of these hold:
- exactly one other wall with a centreline ends there;
- that wall overlaps it in elevation and is perpendicular to it (cosine within 1e-6);
- both walls have a single layer;
- exactly one of the two names the other.

The end then reaches half the other wall's thickness past the line where this wall names the other, and stops that far short where it is named.

`ifc::export_content::wall_centreline_body` draws a decided end there, for axis-parallel and angled walls alike, and keeps today's end elsewhere. The IFC reports it as `JoinReachStartFeet` / `JoinReachEndFeet`.

Applying the rule to multi-layer joints as well was measured first. It fixed 9 walls and made 14 worse, the 12 angled ones among them, all at irregular ends.

## 5. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc (local only) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |
| 2024_Core_Interior.rvt | `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014` |
| demo-01-Source_House_2024_tn2.rvt (local only) | `763f239e70db1ac90e5aa806ec4e6a2cebdc3e8a2e902bcfda6b8de43d2fdcfb` |
| demo-01-Source_House_2025_tn2.rvt (local only) | `28440eaf8b31aa9e91780c75661c3354fa1fad13976bcaf52675c4d11c1a1b6f` |
| RE1-Architecture.rvt | `4a78bdd68e7f4dd7806060db07c222da9a810e20b3f303c17a443294b91c8ce4` |

```bash
cargo run --profile ci --example probe_re70_wall_join_lists -- MODEL.rvt > walls.jsonl
python3 tools/re/wall_join_lists_vs_ifc.py walls.jsonl REVIT_EXPORT.ifc
./target/ci/rvt-gltf MODEL.rvt -o model.glb
python3 tools/re/wall_bodies_vs_ifc.py model.glb REVIT_EXPORT.ifc --json walls.json
```

Snowdon Towers, `rvt-gltf` against Revit's IFC4 bodies (IfcOpenShell 0.8.5), walls with both ends within the tolerance:

| | main | RE-70 |
|---|---:|---:|
| axis-parallel, 0.001 ft | 338 of 976 | 346 |
| axis-parallel, 0.1 ft | 482 | 488 |
| angled, 0.001 ft | 13 of 102 | 13 |
| angled, 0.01 ft | 15 | 16 |

Nine walls improve and none gets worse. No wall's faces move. The eight axis-parallel ones are split-face CMU retaining walls, whose boxes run 0.401 ft past both joints where Revit's bodies reach ±0.318 ft, half the other wall's thickness. Twenty ends report a `JoinReach` property.

Core Interior and RE1 give byte-identical GLBs. The MIT house gives byte-identical IFC in both releases, apart from the timestamp. Their single-layer walls' boxes already give these ends. `rvt-ifc` on Snowdon takes 9.7 to 10.1 s on both builds.

## 6. What stays

Measured per wall end on Snowdon Towers after RE-70:
- **Free ends and T junctions** (no other centreline ends there): 482 of 1,061 ends are off Revit's.
- **Layered L joints** (irregular in Revit's body): 375 ends are off, 348 of them on axis-parallel walls. Revit cleans these layer by layer and a single rectangle cannot draw them. Drawing each layer to its own end is the next step, and the lists say which wall's core runs through.
- **Angled walls meeting at other than a right angle**: the rule holds on all 18 clean ends, but the through wall's end is then slanted, which a rectangle cannot draw.
- **36 clean multi-layer ends**: nothing found yet separates them from the 631 irregular ones.

The witness observations do not change.
