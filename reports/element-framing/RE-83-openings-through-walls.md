# RE-83 — Openings as deep as the wall they cut

**Date:** 2026-09-27
**Issue:** #227 (opening geometry), #439 (three host mismatches found here)
**Scoring:** `tools/re/openings_vs_ifc.py`

**Result:** positive for openings in straight walls: each door and window now cuts an opening exactly as deep as its host wall. Width and height are still the door's or window's box, and Revit's are its opening profile, so the result is within 0.25 ft, not exact.

## What changed

Until now the `IfcOpeningElement` of a door or window was the element's own box: its plan footprint and height. A door's box includes its swing and trim, so the opening stood well out of both faces of the wall (up to 4.9 ft on Core Interior).

The opening now keeps the element's box on two axes and takes its depth from the host wall. The host's plan outline (the same outline its body is built from) is measured along the element's two local plan axes. The axis on which the wall is a thin band (at most 3 ft, and under half the other axis) is the through-axis: the opening spans the wall's band there and keeps the element's size on the other axis and in height. When no axis is a thin band, as for a wall that is not a straight vertical extrusion, the opening stays the element's box.

The element's rotation is not used to choose the axis: on Core Interior some walls run along Y with a door rotation of 0, and only the host's own outline gave the right axis.

## Measured

`tools/re/openings_vs_ifc.py`, rvt-rs's export against Revit's own IFC4 export, openings keyed by the Tag of the element that fills them, largest difference between the two openings' world boxes:

| file | openings in both | within 0.26 ft, before | within 0.26 ft, now | within 1 ft, now | largest, before | largest, now |
|---|---:|---:|---:|---:|---:|---:|
| Core Interior (slim export) | 138 | 0 | 138 | 138 | 4.92 ft | 0.25 ft |
| Snowdon Towers Architectural (local only, `--shared 26.0156,1370151.7953,258247.9762,780.5`) | 110 | 0 | 72 | 83 | 5.83 ft | 5.83 ft |

None is within 0.01 ft. On Core Interior what remains is the frame margin: Revit's opening is the door's rough opening, rvt-rs's is the door's box, a few inches wider and taller. Width and Height are type parameters (3.0 and 8.0 appear in type 17333's value block) but their keys are not decoded, so they are not used.

The 38 Snowdon openings over 0.26 ft:

- **23 in "Solar Wall" hosts** (20 windows, 3 doors). Revit meshes these walls as faceted face sets, not straight extrusions. rvt-rs's openings there are 5 to 7 ft across on both plan axes of the world box, where Revit's are about 2 ft on the thin one: the element's box was kept, not cut to the wall.
- **7 windows in exterior and chase walls**, just over 0.26 ft: the frame margin, a little larger on these window families.
- **5 windows in "Chase - GWB & Metal Stud" walls**, 0.83 to 3.62 ft: four are off centre by about 0.2 ft and wider, and one of them (1528884) is attributed to a different host wall than Revit's (#439).
- **2 doors in curtain walls**: their host differs from Revit's as well (#439).
- **1 overhead rolling door** (1337960): its box is 1.5 ft taller than Revit's opening.

Host pairs: Core Interior 138 of 138 equal Revit's. Snowdon 107 of the 110 fillers present in both; the three that differ are #439. Revit's Snowdon export has 194 openings and rvt-rs's 147; 110 are in both. The rest (slab and shading-device penetrations, openings with no filler) are the remaining part of #227.

## Not claimed

- Revit's opening profile: width and height are the element's box.
- Openings in walls that are not straight vertical extrusions, curtain walls, and curved walls.
- Openings without a door or window (slab penetrations, shaft openings, wall openings).
