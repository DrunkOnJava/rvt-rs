# RE-50 — Roof outlines, and sketch lines' exact ends

**Date:** 2026-09-23
**Result:**
- **Positive.** A roof's sketch lines name the roof, as a floor's name the floor, and they close into the roof's plan outline.
- **Positive.** Each sketch line's own data carries its exact ends in the bounded line record RE-49 reads for beams.
- rvt-rs now gives roofs their outline, and uses a sketch line's recorded ends wherever the box-based solve of RE-25 does not close, for floors as well as roofs.
- On Snowdon Towers, 13 of the 20 roofs now carry their outline (none did), and 11 of those have the area of Revit's own roof.
- On the same file, 15 more slabs carry their sketched outline: 132 of its 199 slab Tags, from 117.
- No outline that closed before changes.

**Probe:** `examples/probe_re50_roof_profiles.rs` lists each roof, whether its sketch closes, and, where the boxes do not close it, whether its lines' recorded ends do.

**Oracle:** Snowdon Towers Sample Architectural (local only) with Revit's IFC4 export of it. The areas compared are each outline's against the plan area of the upward faces of Revit's roof or slab geometry, meshed by IfcOpenShell 0.8.5. No licensed file in CI has a roof, and Core Interior's 80 floors already all closed.

-----

## 1. Roofs

A placed roof's `OST_SketchLines` records carry the roof's ElementId as their owner reference, exactly as a floor's do (RE-25). The same solve closes 9 of Snowdon's 20 roofs.

Why the other 11 do not close under the box solve:

| roof | box, ft | sketch lines it owns | lines not on a model axis |
|---:|---|---:|---:|
| 754197 | 178.658 × 49.471 × 0.500 | 4 | 0 |
| 754672 | 67.197 × 31.165 × 0.500 | 5 | 4 |
| 754785 | 28.937 × 30.159 × 0.500 | 3 | 2 |
| 759489 | 9.875 × 23.042 × 1.451 | 0 | 0 |
| 882818 | 8.375 × 23.042 × 1.451 | 0 | 0 |
| 964559 | 22.854 × 32.175 × 0.667 | 1 | 0 |
| 1916467 | 7.146 × 16.208 × 1.212 | 4 | 1 |
| 1916596 | 4.844 × 13.656 × 1.123 | 4 | 1 |
| 1926040 | 178.658 × 49.471 × 0.875 | 45 | 16 |
| 1975810 | 28.937 × 30.159 × 0.917 | 9 | 5 |
| 1985275 | 67.197 × 31.165 × 0.917 | 19 | 11 |

A segment's record carries only its box. For a diagonal, the box leaves two candidate ends, which the box solve accepts only when the loop forces the choice.

## 2. A sketch line's recorded ends

A sketch line's element data (after its element-data header and ElementId, RE-44) carries the same bounded line record as a beam's (RE-49):

```text
04 00 08 01 · f64 start · f64 end · f64×3 origin · f64×3 unit direction
```

Its ends, `origin + parameter · direction`, are the segment's. Chaining them closes a loop with no choice left, provided every end is shared by exactly two segments. A line is used only under three conditions:

- it is level;
- both its ends lie in its own record's box;
- every line of the element qualifies, or the element keeps no profile.

The recorded ends are read only where the box solve does not close. Every profile that closed before therefore stays exactly as it was.

On Snowdon's 11 unclosed roofs:

- 4 close from their lines' recorded ends: 1916467, 1916596, 1975810 and 1985275.
- 4 have lines whose ends do not chain: 754197, 754672, 754785 and 1926040. Their sketches hold lines that are not all boundary: 754197 has four lines on a model axis that still do not meet end to end.
- 3 own too few lines: 759489 and 882818 own none, and 964559 owns one.

## 3. Measured against Revit's roofs and slabs

Areas are per Tag, summing every rvt-rs piece of a multi-loop sketch (#331) and scoring a roof against Revit's IfcRoof and its parts. "Per layer" means the outline is Revit's area divided by a whole number: Revit writes a layered roof or slab as stacked layer solids of the same outline, and each is counted.

| | outline | equal | equal per layer | larger |
|---|---:|---:|---:|---:|
| roofs, before | 0 of 20 | | | |
| roofs, after | 13 of 20 | 6 | 5 | 2 |
| slabs, before | 117 of 199 | 70 | 34 | 7 |
| slabs, after | 132 of 199 | 74 | 41 | 11 |

- Every roof outline's plan box is exactly the roof's record box.
- The 2 larger roofs are 1916596, 5.4% larger than Revit's roof, and 2112673, 34.7% larger. Revit's geometry of both is cut by something the sketch does not hold.
- The 15 slabs the recorded ends add:
  - 11 equal Revit's (4 outright, 7 per layer);
  - 4 are larger: 2077111, 2113168 and 2113320 by 3.0% each, and 2134315 by 0.2%.
- The 4 larger slabs have the same residual as 6 of the 7 that differed before (larger by 0.3% to 19%). Revit subtracts shaft openings and other cuts, which are separate elements, from the slab's own sketch.
- The seventh, 2114707, is half of Revit's area, and was so before this change.
- Both slab rows include the 6 Legends-phase slabs of #328, which have outlines but are not in Revit's export, so they are not scored.

## 4. What this does not do

- **A roof's slope is not read.** A roof keeps its record box's height as its extrusion depth, so a sloped roof is a level plate as thick as its box: 1.397 ft for a 2° metal-deck roof whose deck is about 0.64 ft. Its outline is right. Revit writes 10 of Snowdon's roofs as polygon meshes (tapered insulation) and 7 as extrusions along a 2° tilt.
- **Cuts are not applied.** Shaft openings and other elements that cut a roof or slab are not subtracted.
- **7 roofs keep their record box.**
- The recorded ends are read on Revit 2024 only, where RE-49 measured the record. The box solve still runs on 2025.
