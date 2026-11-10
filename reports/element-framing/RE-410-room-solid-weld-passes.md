# RE-410: the three weld passes of RvtIO's room-boundary reader, tried on the edges of 3,759 rooms' solids

**Date:** 2026-11-10
**Issues:** #90 (rooms on their box); discussion #1241 (RvtIO, the offer of its weld passes); follows RE-101 (a room's outline from its stored solid's faces), RE-316 (the faces up to the solid's end) and RE-374 (the solid's edges and the floor outline they give)
**Artefacts:** Autodesk's sample projects of 2015 to 2027 (pinned in `research/autodesk-sample-projects.tsv`), the BIM4LCA housing and office files (`research/bim4lca-measure.tsv`, CC BY-SA 4.0), puzzbobb's Revit 2025 models, the four RE1 models, Core Interior and Einhoven: 100 models, 3,759 rooms, each with one solid. Never stored.
**Probe:** `probes/examples/probe_90_room_weld_passes.rs`
**Runs:** Measure [42923425380](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42923425380) (the probe as first written, which placed the ends of an edge round a cylinder as if they were its ends: its tallies are not used) and [42925686548](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42925686548) (the probe at `2c979d74`, every number below; `-f samples=true -f nordic=true -f r2025=true -f base=none -f scorers=none`)
**Status:** Positive, as a measurement, and not shipped: welding the ends of a solid's edges at 0.000523 ft closes the faces that can be placed of 6 of the 7 Snowdon Towers rooms #90 names, and the third pass, 1/32 in, adds nothing there. The reader of RE-374 snaps at 0.0001 ft and refuses those rooms for other reasons, so the passes change no room today.

## 1. What was offered

RvtIO (discussion #1241, 2026-11-10; its `GRepParserV2.cs` at the commit its maintainer named there) reads a stored room shell's boundary points with no welding, then welds points whose x, y and z each differ by no more than 0.0005233832795 ft (0.159527 mm), then by no more than 1/32 in (0.79375 mm), merging points transitively; a segment whose two ends weld to one point is dropped, a loop with fewer than three distinct points is a sliver and dropped, and the result has to pass its trim-coverage and closed-shell checks. We said we would try the three passes on the seven Snowdon Towers rooms that stay open and report which of them close.

## 2. What the probe does, and how it differs

RvtIO welds the points of serialized face loops. rvt-rs holds a solid's edges (RE-374): each names two faces and each face's (u, v) at the edge's two ends, so a face's boundary is the points its edges give on it, a plane's `origin + u U + v V` and a cylinder's along its axis. The probe welds those points at each tolerance (x, y and z each within it, transitively; exactly equal for "none"), drops a segment whose two ends weld to one point, counts a face with fewer than three distinct points left as a sliver, and calls a face closed when every point of its boundary has an even degree. It reads two things RvtIO's passes would: the edges whose two faces place their ends differently (`split`), and the faces that close.

Not placed: an edge round a cylinder stores two points that are not its ends (RE-374), on the cylinder and on the plane across it alike, so both its faces are left out of the count; a solid with such a face is `closed where placed` at best. The reading is therefore RvtIO's rule applied to the points the edges give, and not RvtIO's reader.

## 3. The seven rooms

Snowdon Towers Architectural 2024, room by room. The first pass is no welding. Each cell is `split edges / open faces`:

| room | faces (read) | cylinders | edges | faces not placed | none | 0.000523 ft | 1/32 in |
|---|---:|---:|---:|---:|---|---|---|
| 828537 | 66 (59) | 0 | 200 | 2 | 44 / 23 | 0 / 0 | 0 / 0 |
| 828548 | 97 (88) | 0 | 337 | 6 | 21 / 21 | 0 / 0 | 0 / 0 |
| 830902 | 24 (22) | 5 | 72 | 7 | 31 / 9 | 0 / 0 | 0 / 0 |
| 830912 | 21 (21) | 5 | 66 | 7 | 39 / 3 | 0 / 0 | 0 / 0 |
| 2181757 | 136 (131) | 11 | 419 | 14 | 71 / 22 | 0 / 0 | 0 / 0 |
| 2181762 | 95 (75) | 27 | 288 | 48 | 56 / 16 | 0 / 2 | 0 / 2 |
| 2285535 | 14 (14) | 2 | 40 | 4 | 24 / 2 | 0 / 0 | 0 / 0 |

- **No welding:** every one of the seven has edges whose two faces place their ends differently (21 to 71) and faces that do not close.
- **0.000523 ft:** every such edge welds, and the faces that can be placed close on 6 of the 7. Room 2181762 keeps faces 79 and 80 open at every tolerance, with 48 of its 95 faces not placed, so no pass closes it.
- **1/32 in:** the same as 0.000523 ft on all seven: no segment collapses and no face is a sliver. The third pass closes nothing there that the second did not.
- "Closes" is for the faces that can be placed; the faces not placed are 2, 6, 7, 7, 14, 48 and 4, so no pass shows a whole shell closed on any of the seven.

## 4. Every model

3,759 solids on 100 models (of their rooms the walk of RE-101 reads 3,122, the span of RE-316 reads 10 exactly and 27 approximately, and 600 stay their record box):

| | none | 0.000523 ft | 1/32 in |
|---|---:|---:|---:|
| closed | 1,278 | 2,680 | 2,687 |
| closed where placed | 551 | 1,068 | 1,069 |
| open | 1,930 | 11 | 3 |
| solids with an edge whose two faces' ends differ by more than the tolerance | 2,132 | 7 | 0 |

- **The second pass settles 3,748 of 3,759.** 1,919 solids close first at 0.000523 ft and 1,829 close without welding (1,278 whole, 551 where placed).
- **The third pass settles 8 more**, all on the BIM4LCA office (rooms 1383998, 1493531, 1681182, 1700966, 1736690, 1743279 and 2297732 as built, 2611622 at timber permit): in 7 of them an edge whose two faces place its ends between 0.000523 and 0.0026 ft apart, in room 1743279 two points of one face that far apart. Room 1681182 is RE-374's solid with 4 edges that its planes place more than 0.001 ft apart (`edge_faces_disagree`); they weld at 1/32 in.
- **1/32 in is the only pass with a collapsed segment or a sliver face:** on 8 solids.
- **Three solids close at no tolerance:** Snowdon's 2181762, and rooms 1394237 and 1396610 of the office's terrain files (open faces 8 and 2, with 4 and 1 segments collapsed at 1/32 in).

## 5. What it means for rvt-rs

- The reader of RE-374 snaps the ends of a floor's lines at 0.0001 ft and accepts a plane pair that agrees to 0.001 ft. Where the solids close, 0.000523 ft closes them. The rooms it refuses are refused for the prism height, a face whose record is not read and floor lines that do not close (RE-374 section 5: 564, 179 and 27 refusals), not for a weld. Four of the seven (828537, 828548, 2181757 and 2181762) keep their box for the height their area is taken at (#1589), and 2181762 also stays open at faces 79 and 80 under every pass.
- Whether a snap of 0.000523 ft in place of 0.0001 ft turns any of the 27 rooms whose floor lines do not close into a read outline is not measured here; it is a change to `edge_outline_at`, with its own measurement.
- RvtIO's third pass is a reading for the 8 office solids only; nothing in the corpus decides whether a weld at 1/32 in is right for them.

## 6. Not shown

- **RvtIO's reader on our rooms.** This is its rule on our edge points; its loops, trim-coverage check and closed-shell check were not run, and the answer for a face with a cylinder's round edge is "not placed".
- **A closed shell.** No solid is shown closed over every face where a face is not placed (1,069 solids and the seven rooms).
- **Revit's area for the four stepped rooms.** The weld does not decide the section height their area is taken at (RE-374 section 4, #1589).

## 7. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=2c979d7478dd3dabb92df9e655d4ed951bc54502 -f base=none \
  -f samples=true -f nordic=true -f r2025=true -f probe=probe_90_room_weld_passes -f probe_scorer=- -f scorers=none
```

The probe's `weld room` lines give each solid's faces, edges, unplaced faces and, for each pass, the `split` edges, `collapsed` segments, `slivers` and the open faces; its `weld tally` lines count the solids by what `main` reads and the pass at which they close.

## 8. Sources

RE-101, RE-316, RE-374; RvtIO's rules as its maintainer described them on discussion #1241 (its source was not read here). No Autodesk source was read.
