# RE-313 — Every IfcSpace's area and outline measured against Revit's export

**Date:** 2026-11-03
**Issues:** #1260 (rooms for an importer); follows RE-101 and RE-102 (room outlines from their stored solids), RE-255 (space heights), RE-296 (space volumes) and RE-191 (areas)
**Artefacts:** Core Interior (Revit 2024) and RE1 Architecture (Revit 2025) with Revit's IFC4 and IFC2X3 exports of them, the BIM4LCA office as built and at building permit (Revit 2024), puzzbobb's R2025 models, and the held-out models (counts only), as Measure fetches them
**Runs:** @RUNS@
**Result:** positive, with the misses explained: rvt-rs's room outlines were measured only once, by hand, on three files (RE-101). They are now scored on every reference model by a Measure scorer, and gated in CI on two. Of 605 room scores with the outline read from the room's stored solid (the office counted at both stages), 604 have Revit's area to 0.1% and 590 Revit's outline within 0.01 ft. A room whose outline was not read is its record's bounding box, and its area is Revit's on 2 of the office's 112 and on 7 of 12 on a held-out house, and on every rectangle. Each `IfcSpace` now says which of the two it is.

## 1. The scorer

`tools/re/space_profiles_vs_ifc.py` runs `plan_profiles_vs_ifc.py`'s measurements on every `IfcSpace` of the two files. Both are meshed by IfcOpenShell and moved into Revit's internal feet through each export's own site placement. A space's area is the plan union of its upward faces, and its outline's distance is the largest distance from a vertex of either top surface to the other's boundary (so Revit's extra vertices on a shared edge cost nothing).

- Spaces pair by GlobalId, which Revit's export and rvt-rs derive alike (RE-196). A file whose rooms carry none of Revit's, as Revit 2023's, pairs by nearest centroid, one to one; the scorer picks the mode from the files.
- It prints counts only on lines that start `space profiles:`: spaces in each file, in both, only in rvt-rs, only in Revit and written without a body, area within 0.1% and 1%, outline and vertices within 0.01 ft, and the same counts by `SpaceOutlineSource`. Measure keeps those lines for held-out models and drops every line with a GlobalId (`tools/ci/scrub-holdout-results.sh`).
- `--min-in-both`, `--min-area` and `--min-outline` make the run a gate.

## 2. Measured

Measure, unfiltered on models and with `-f scorers="space_profiles_vs_ifc tagged_spaces_vs_ifc room_names_vs_ifc"`, on `main` at a98bd316 (before this change, the first rows):

| Model | Spaces (rvt-rs / Revit) | In both | Area 0.1% | Area 1% | Outline 0.01 ft | By outline source |
|---|---:|---:|---:|---:|---:|---|
| Core Interior (2024) | 116 / 116 | 116 | 116 | 116 | 116 | 116 stored solid, 116 / 116 / 116 |
| RE1 Architecture (2025) | 11 / 11 | 11 | 11 | 11 | 11 | 11 stored solid, 11 / 11 / 11 |
| BIM4LCA office as built (2024) | 308 / 308 | 276 | 221 | 221 | 211 | 220 stored solid, 220 / 211; 56 record box, 1 / 0 |
| BIM4LCA office at building permit (2024) | 308 / 308 | 278 | 222 | 222 | 216 | 222 stored solid, 221 / 216; 56 record box, 1 / 0 |
| R2025 12 to 17 (rooms and stairs), 6 models | 12 / 12 | 12 | 12 | 12 | 12 | 12 stored solid |
| Held-out, 2022 to 2025, 6 models | 24 / 24 | 24 | 24 | 24 | 24 | 24 stored solid |
| Held-out, 2026 and 2027, 4 models | 16 / 16 | 16 | 16 | 16 | 16 | 16 record box |
| One held-out house | 13 / 13 | 12 | 7 | 8 | 0 | 12 record box, 7 area |

The other R2025 models, RE1 Electrical, Mechanical and Plumbing and Einhoven (which has no Revit export) have no spaces. Neither office file pairs 30 of Revit's spaces, which are its Areas: Revit's export gives them a body, and rvt-rs writes them without one (RE-191). The office pairs 276 and 278 of the 278 spaces rvt-rs writes a body for; two as built have no area in Revit's top surface.

## 3. What misses, and why

A miss is a room off by more than 0.1% in area or 0.01 ft in outline.

- **Record box (56 rooms on each office file, 12 on the held-out house).** The room's outline was not read from its stored solid, so its body is the record's bounding box. The box is Revit's room only where the room is a rectangle (1 of 56 on the office, all 16 held-out rooms of 2026 and 2027, 7 of 12 on the house by area); the outline is never within 0.01 ft but on those rectangles. @BOX@
- **Stored solid, area right, outline off (9 as built, 5 at permit).** The area equals Revit's to 0.00% and the outline is 0.05 to 2.8 ft away. @STORED@
- **Stored solid, area wrong (1 at building permit, 66%).** @ONE@

## 4. What an importer reads

`RvtElementRecordGeometry.SpaceOutlineSource` is `stored_solid` where the outline was read from the room's stored solid and `record_box` where the body is the record's box; `saved_mesh` where `--saved-meshes` drew it from the mesh Revit saved; `none` on an Area. A room's `BodySource` is the record box either way, because the placement and the height come from it, so it cannot tell the two apart. The property goes in rvt-rs's own set and changes no value Revit's export writes. The diagnostics count the boxes as `exported.bounding_box_spaces`, and `rvt-ifc` prints "N of M spaces are their room's bounding box" when there are any. [docs/rooms.md](../../docs/rooms.md) is the importer's page.

The change against `main` moves `IFCPROPERTYSINGLEVALUE` by one per space (Core Interior 29098 to 29214) and nothing else: @W7@

## 5. The gate

The `IfcOpenShell validates exports` job exports Core Interior and RE1 Architecture, scores their spaces against Revit's exports of them and fails below 116 of 116 and 11 of 11 in both, in area and in outline. A regression in the outline read, in the pairing or in the writer now fails CI instead of waiting for a Measure run. Revit 2025's RE1 files are fetched by hash into the job (two files, as `Corpus health` fetches them).

## 6. Not measured

Revit 2008 to 2021 and 2023 (no reference model with Revit's room export on a runner; RE-102's 9 rooms are local), the rooms of 2026 and 2027 beyond rectangles (#1260), the office's Areas' bodies (RE-191), and `Height` and `NetVolume` by this scorer (`qtos_vs_ifc` scores them).

## Reproduce

```bash
gh workflow run measure.yml --ref feat/rooms-space-outlines-measured -f ref=a98bd316b39335a4c74ad3f16004b1ff2f721e1d -f base=none \
  -f nordic=true -f r2025=true -f holdout=true -f scorers="space_profiles_vs_ifc tagged_spaces_vs_ifc room_names_vs_ifc"
python3 tools/re/space_profiles_vs_ifc.py model.ifc revit-export.ifc --list 10
```
