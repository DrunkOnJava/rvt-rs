# RE-313 — Every IfcSpace's area and outline measured against Revit's export

**Date:** 2026-11-03
**Issues:** #1260 (rooms for an importer); follows RE-101 and RE-102 (room outlines from their stored solids), RE-255 (space heights), RE-296 (space volumes) and RE-191 (areas)
**Artefacts:** Core Interior (Revit 2024) and RE1 Architecture (Revit 2025) with Revit's IFC4 and IFC2X3 exports of them, the BIM4LCA office as built and at building permit (Revit 2024), puzzbobb's R2025 models, and the held-out models (counts only), as Measure fetches them
**Runs:** Measure on `main` before this change, [41655922826](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41655922826); the change against `main`, unfiltered on models, with the psets, quantity, plan profile and class set scorers beside the space scorers, [41658742590](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41658742590); the final scorer, [41660946955](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41660946955); the RE-101 probe on the office, [41657521650](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41657521650)
**Result:** positive, with the misses explained. rvt-rs's room outlines had been measured once, by hand, on three files (RE-101). Measure now scores them on every reference model and CI gates two. Of 605 room scores whose outline was read from the room's stored solid (the office counted at both stages), 604 have Revit's area to 0.1% and Revit's outline to 0.01 ft. A room that keeps its record's bounding box has Revit's area on none of the 24 the office holds, and on the 16 held-out rooms of 2026 and 2027, which are rectangles. Every `IfcSpace` now says which of the two it is.

## 1. The scorer

`tools/re/space_profiles_vs_ifc.py` takes `plan_profiles_vs_ifc.py`'s measurements to every `IfcSpace`. Both exports are meshed by IfcOpenShell and moved into Revit's internal feet through each export's own site placement. A space's area is the plan union of its upward faces.

- Spaces pair by GlobalId, which Revit's export and rvt-rs derive alike (RE-196). A file whose rooms carry none of Revit's, as Revit 2023's, pairs by nearest centroid, one to one; the scorer picks the mode from the files.
- The outline is measured two ways. **Outline** is `plan_profiles_vs_ifc.py`'s: the largest distance from a vertex of either mesh's upward faces to the other's top-surface boundary. **Boundary** is the largest distance from a point of either top-surface boundary to the other's. They differ where Revit's mesh has an upward face inside the room, a ceiling step (RE-101 §1): its vertices are far from the boundary, and the outline distance reads them as error.
- It prints counts only on lines that start `space profiles:`: spaces in each file, in both, only in rvt-rs, only in Revit (and of those, the ones rvt-rs writes without a body), area within 0.1% and 1%, outline, boundary and vertices within 0.01 ft, and the same counts by what the space is (a room, a family instance written as a space) and `SpaceOutlineSource`. Measure keeps those lines for held-out models and drops every line with a GlobalId (`tools/ci/scrub-holdout-results.sh`). For each space off in area or outline it lists its GlobalId, both areas, the distances, its source, its pieces and holes on each side and the distance between the two centroids.
- `--min-in-both`, `--min-area`, `--min-outline` and `--min-boundary` make the run a gate.

## 2. Measured

Measure on `main` at a98bd316 (the first run), and the final scorer on this branch (the last two columns):

| Model | Spaces (rvt-rs / Revit) | In both | Area 0.1% | Outline 0.01 ft | Boundary 0.01 ft | By what the space is and its source: count, area, boundary |
|---|---:|---:|---:|---:|---:|---|
| Core Interior (2024) | 116 / 116 | 116 | 116 | 116 | 116 | 116 rooms, stored solid: 116, 116 |
| RE1 Architecture (2025) | 11 / 11 | 11 | 11 | 11 | 11 | 11 rooms, stored solid: 11, 11 |
| BIM4LCA office as built (2024) | 308 / 308 | 276 | 221 | 211 | 221 | 220 rooms, stored solid: 220, 220; 12 rooms, record box: 0, 0; 44 family instances, record box: 1, 1 |
| BIM4LCA office at building permit (2024) | 308 / 308 | 278 | 222 | 216 | 222 | 222 rooms, stored solid: 221, 221; 12 rooms, record box: 0, 0; 44 family instances, record box: 1, 1 |
| R2025 12 to 17 (rooms and stairs), 6 models | 12 / 12 | 12 | 12 | 12 | 12 | 12 rooms, stored solid: 12, 12 |
| Held-out, 2022, 2024 and 2025, 6 models | 24 / 24 | 24 | 24 | 24 | 24 | 24 rooms, stored solid: 24, 24 |
| Held-out, 2026 and 2027, 4 models | 16 / 16 | 16 | 16 | 16 | 16 | 16 rooms, record box: 16, 16 |
| One held-out house | 13 / 13 | 12 | 7 | 0 | 0 | 12 rooms, record box: 7, 0 |

The other R2025 models, RE1 Electrical, Mechanical and Plumbing and Einhoven (which has no Revit export) have no spaces. The office pairs 276 and 278 of the 278 spaces rvt-rs writes a body for (two as built have no area in Revit's top surface), and the 30 of Revit's it does not pair are its Areas: Revit's export gives them a body, and rvt-rs writes them without one (RE-191). The 16 held-out rooms of 2026 and 2027 are rectangles, since their outline equals the box's to 0.01 ft.

## 3. What misses, and why

A miss is a space off by more than 0.1% in area or 0.01 ft in outline.

- **Rooms keeping their record box (12 on each office file, 12 on the held-out house).** The RE-101 probe (41657521650) says why on the office: 221 of its 234 rooms as built and 222 at permit keep an outline, and the other 13 and 12 are "solid does not close on the box". The probe counts 13 as built where the scorer counts 12: two as-built pairs have no area in Revit's top surface and are not scored. The box is Revit's room only where the room is a rectangle: 0 of 24 on the office, 7 of 12 by area and 0 by outline on the house, 16 of 16 on the 2026 and 2027 rooms. What those rooms' solids have in common is not found.
- **Family instances written as spaces (44 on the office).** The Tagged-spaces scorer (`tagged_spaces_vs_ifc`) says what they are: 42 "Turning Free Space" families and two "VOLUME" families, each with a mapped representation in Revit's export. rvt-rs draws their record box, a square where the family is a disc (24.22 ft² against 19.01 ft² on the 1500 mm turning spaces listed, 0.785 apart) and a mismatch on 43 of 44. These are #1262's.
- **Stored solid, area right, outline distance off (9 as built, 5 at permit).** Their boundary distance is 0.000 ft, their areas equal to 0.00%, their pieces and holes equal and their centroids 0.000 ft apart. Revit's mesh has upward faces inside those rooms, ceiling steps, whose vertices the outline distance counts: it is the metric, not the outline. The boundary distance is 0.000 ft on every one of them, and the table's boundary column is the room's outline.
- **Stored solid, area wrong (1 at building permit, 66%).** 172.77 against 103.90 ft², the centroids 3.8 ft apart, one piece and no hole on each side. It is not in the as-built file. Its cause is not found: the probe's per-room lines name ElementIds, the scorer's GlobalIds, and nothing pairs them.

## 4. What an importer reads

`RvtElementRecordGeometry.SpaceOutlineSource`:

| Value | Written on | Meaning |
|---|---|---|
| `stored_solid` | a room | the outline was read from the room's stored solid |
| `record_box` | a room, or a family instance written as a space | the body is the record's bounding box (turned with the instance where it is turned) |
| `saved_mesh` | either | `--saved-meshes` drew it from the mesh Revit saved |
| `family_body` | a family instance written as a space | a carrier drew more than the box; its `BodySource` says what |
| `none` | an Area | no body |

A room's `BodySource` is the record box either way, because the placement and the height come from it, so it cannot tell the two apart. The property goes in rvt-rs's own set and changes no value Revit's export writes. The diagnostics count the boxes as `exported.bounding_box_spaces` (the 56 on each office file: the 12 rooms and the 44 family instances), and `rvt-ifc` prints "N of M spaces are their room's bounding box" when there are any. [docs/rooms.md](../../docs/rooms.md) is the importer's page.

W7: the change moves `IFCPROPERTYSINGLEVALUE` by one per space (Core Interior 29098 to 29214, both witness observations refreshed from CI's artifact, the verdicts replaying PASS and unchanged) and no Revit value. On the change against `main`, nothing else differs: the psets, quantity, plan profile, class set, tagged space and room name scorers print the same on all 45 models, and the exports' hashes move on exactly the 21 models that have spaces (run 41658742590).

## 5. The gate

The `IfcOpenShell validates exports` job exports Core Interior and RE1 Architecture, scores their spaces against Revit's exports of them and fails below 116 of 116 and 11 of 11, in both, in area, in outline and in boundary. A regression in the outline read, in the pairing or in the writer now fails CI instead of waiting for a Measure run. RE1's two files are fetched into the job by hash, as `Corpus health` fetches them. `space_profiles_vs_ifc.py --min-area 12` against RE1's own export exits 1 with "area 11 below 12", which is how the gate was shown to fail.

## 6. Not measured

Revit 2008 to 2021 and 2023 (no reference model with Revit's room export on a runner; RE-102's 9 rooms are local), the rooms of 2026 and 2027 beyond rectangles (#1260), the Areas' bodies (RE-191), and `Height` and `NetVolume` by this scorer (`qtos_vs_ifc` scores them). The office's 12 closing failures and the one 66% room are left as measured negatives.

## Reproduce

```bash
# once landed, from main (the scorer is on main then); before, --ref <the branch>
gh workflow run measure.yml --ref main -f ref=a98bd316b39335a4c74ad3f16004b1ff2f721e1d -f base=none \
  -f nordic=true -f r2025=true -f holdout=true -f scorers="space_profiles_vs_ifc tagged_spaces_vs_ifc room_names_vs_ifc"
gh workflow run measure.yml --ref main -f ref=2f155a031eb8d7757e6421efd6988022a5aa0d33 -f base=none \
  -f nordic=true -f r2025=true -f holdout=true -f scorers="space_profiles_vs_ifc tagged_spaces_vs_ifc room_names_vs_ifc"
gh workflow run measure.yml --ref main -f ref=2746264d28a1dc9064745ea07fb177873f0a90c8 -f base=none \
  -f nordic=true -f models="nordic-office-* core-interior re1-architecture" \
  -f probe=probe_re101_room_solids -f probe_scorer=- -f scorers=space_profiles_vs_ifc
python3 tools/re/space_profiles_vs_ifc.py model.ifc revit-export.ifc --list 10
```
