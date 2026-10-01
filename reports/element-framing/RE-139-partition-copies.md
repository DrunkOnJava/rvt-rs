# RE-139: what differs between the copies of a record

**Date:** 2026-10-01
**Issues:** #548, #152; follows RE-136
**Artefacts:** the four RE1 models (Revit 2025), `Revit_IFC5_Einhoven.rvt` (Revit 2023) and `2024_Core_Interior.rvt` (Revit 2024), all MIT. Measured on GitHub's hosted runners with the Measure workflow, runs 36881222392 (differing byte positions), 36881931583 (bounding boxes) and 36882594734 (moved walls and the partitions they are in).
**Probe:** `examples/probe_re135_undeclared_records.rs`, extended.
**Status:** partial. What differs is measured; which copy is current is not.

## 1. The question

RE-136 found that an id which opens several records in a project's leading chain has them in different partitions, never in one, with the same class: 1,444 ids on Core Interior, of which 873 are byte-identical and 571 are not. #548 asked what differs, since a reader that takes one copy could take the wrong one.

## 2. What differs

For each id whose copies differ, the largest difference between the bounding boxes of its copies (the record's box, `+0x58`):

| class (Core Interior, 2024) | ids that differ | boxes equal | by less than 1e-9 ft | by up to 1 ft | by more |
|---|---:|---:|---:|---:|---:|
| `SWall` | 236 | 58 | 7 | **171** | 0 |
| `FamilyInstance` | 184 | 88 | 0 | **96** | 0 |
| `RoomElem` | 111 | 0 | 0 | **111** | 0 |
| `Floor` | 12 | 0 | 0 | **12** | 0 |
| 10 other classes | 27 | 24 | 2 | 0 | 1 |

(570 of RE-136's 571 ids: the box of one pair could not be read.)

- **For walls, family instances, rooms and floors the copies are different revisions of the element.** Box differences are real and small: the three wall examples printed (20796 to 20798) differ by exactly 1 inch (0.0833 ft) in one coordinate of the box. Where a copy's box does not differ, other bytes do (58 walls, 88 family instances).
- **The differing positions are in the box** (`+88` to `+135` of the record, the six `f64`) and at a few fixed positions after it: for walls `+148`, for family instances `+156` to `+258` in runs of one to three bytes, for rooms `+96` to `+125`.
- **The copies are in the main partition and one other.** The partitions of the differing copies (every class, a box difference of 1e-6 ft or more) are `Partitions/46` and `Partitions/59` for 377 ids, `Partitions/46` and `Partitions/55` for 12, and a three-partition combination for each of 2 more.
- **The other models have almost no box difference.** RE1 Architecture, Electrical, Mechanical and Plumbing: every differing id's box is equal or differs by less than 1e-9 ft. Einhoven (2023): of its 5 differing ids, the `Partitions/2` copies of wall 2808 and instance 5317 hold a box translated by (48.246, 31.17) ft from their `Partitions/0` copy (the same offset for both), which is not examined here; and the fields a curtain wall type's second record holds where a box would be are not a box (values of 1e130 and 1e149).

## 3. Does the choice reach the export

All 171 Core Interior walls whose copies differ in their boxes are exported (of 360 walls exported), and the Measure scorers hold rvt-rs's walls to Revit's: all 360 walls have their faces and their ends within 0.001 ft of Revit's (`wall_bodies_vs_ifc`), and the export carries Revit's GlobalId on all 854 elements. So for every wall whose copies moved, what rvt-rs exports is Revit's.

That does not say the other copy would give the same. The exported body of a wall is built from its type's layers and its axis, not from the box's thickness: wall 20796's exported profile is 0.6667 ft thick where its copies' boxes are 0.5 and 0.5833 ft deep, and wall 20798's is 0.6667 where its box is 0.5, so a one-inch change of the box may not reach the body (wall 20797's profile, 0.5 ft, is its box's).

## 4. Not established

- **Which copy is current.** The ElemTable's revision counters (RE-35 section 6) and the partitions' own metadata were not compared with the copies.
- **Whether the other copy would give the same export.** Taking each copy in turn and scoring the walls, rooms and family instances against Revit's would say; it was not done.
- What the 48.246 ft translation on Einhoven is, and what the bytes after the box (`+148` on walls, `+156` and later on family instances) are.
