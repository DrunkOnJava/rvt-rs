# RE-304 — A cylinder face trimmed by any loop is meshed in strips across its angle

**Date:** 2026-11-03
**Issues:** #1247, #255, #1144; follows RE-224, RE-300 and RE-301
**Run:** Measure [41563982575](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41563982575) (this change at 1b983f8a against `main` at 3e0ad51d, `--saved-meshes`, `mesh_bodies_vs_ifc`; the census from Measure [41555808867](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41555808867))
**Result:** positive. `native_saved_mesh::cylinder_face` meshed a cylinder face only where its trim loop runs round its UV box, side by side, and rejected any other loop, which keeps the whole element's box (RE-224). Meshing every other single loop in strips across U draws 155 more BIM4LCA office doors from their saved meshes, 151 exact to 0.001 ft against Revit's export and all within 0.01 ft and 1% of its volume.

## 1. The gap

After RE-300 and RE-301, the census of what keeps a saved mesh as a box on the office, RE1 and Autodesk's 2024 samples (1,213 elements) is led by `nonrectangular cylinder trim`: 718 elements, 451 for that alone. A door's or a fitting's curved faces are cylinders whose loops follow cuts and joins, not their UV box.

## 2. The reading

A cylinder is straight along its axis: `P(u, v) = centre + r(cos u·x + sin u·y) + v·z` is linear in `v`. So a triangle whose corners lie on the surface lies on it across V, and only its extent in U needs a chord bound. The face's single trim loop (in UV, from its edges) is clipped against `u = u_k` for the same U values the rectangular grid uses: steps of the angular chord bound, plus each trim vertex's `u`. Each strip's piece is earcut in arc-length coordinates `(r·u, v)`, and its triangles are wound by the face's orientation, as the grid's are. The strips must cover the loop's own area to within 1e-6 of it, or the loop is taken as self-intersecting and the face rejected.

A loop that runs round its UV box keeps the grid it had, so every face meshed before is meshed the same. Faces with several loops (holes) are still rejected (`multiple cylinder trim loops unsupported`, 82 elements).

## 3. Measured

Measure 41563982575, this change against `main`, both with `--saved-meshes`:

| file | bodies drawn from saved meshes, before → after | kept as boxes, before → after |
|---|---|---|
| BIM4LCA office as built (2024) | 10,924 → 11,079 | 279 → 124 |
| BIM4LCA office at permit (2024) | 11,041 → 11,196 | 279 → 124 |
| RE1 Architecture (2025) | 52 → 53 | 4 → 4 |
| RE1 Plumbing (2025) | 41 → 52 | 0 → 0 |
| rme basic and advanced (2024 samples) | 5,506 → 5,669 and 5,509 → 5,633 | 141 → 34 |
| Snowdon Towers plumbing (2024 sample) | 2,516 → 2,781 | 121 → 121 |
| Snowdon Towers architectural, HVAC, rac basic (2024 samples) | +34, +24, +15 | |
| Core Interior, RE1 Electrical and Mechanical, the other samples | unchanged | unchanged |

The samples have no Revit export, so their counts are not scored; RE-303 shows a decoded mesh can still be incomplete, so they stay unverified until a file with Revit's export covers them.

Against Revit's own export (`mesh_bodies_vs_ifc`):

| file | scored, before → after | exact (0.001 ft) | within 0.01 ft | volume within 1% |
|---|---|---|---|---|
| office as built | 1,495 → 1,650 | 1,403 → 1,554 | 1,481 → 1,636 | 1,477 → 1,632 |
| office at permit | 1,583 → 1,738 | 1,487 → 1,638 | 1,565 → 1,720 | 1,564 → 1,719 |
| RE1 Architecture | 52 → 53 | 45 → 45 | 50 → 50 | 52 → 53 |
| RE1 Plumbing | 41 → 52 | 7 → 7 | 27 → 38 | 7 → 7 |

- **Office:** the 155 new bodies are all doors (`IfcDoor` 114 → 269 scored, 265 exact), among them 34 "O-3:O-3.10x21" and 22 each of "O-4" 10x21 and 8x21. Four are short of exact; the scorer lists one, "O-9", within 0.0048 ft at a volume ratio of 0.9998. The default export draws doors from their saved meshes, so these reach it.
- **RE1 Architecture:** a toilet, within 0.0126 ft at 1.0035 of Revit's volume.
- **RE1 Plumbing:** 11 more pipe fittings, like the 35 drawn before (RE-78's fittings, whose class median is 1.22 of Revit's volume): Revit's surface and rvt-rs's within 0.01 ft of each other (the listed tees' farthest vertices 0.006 to 0.007 ft), at 1.44 to 1.47 of Revit's volume; no part is missing, the volume difference is RE-78's open question.

The cost, with `--saved-meshes`: the office as built goes from 473,681 to 568,965 triangles (about 615 for each new door, against about 43 for the average body), its IFC from 76.6 to 82.1 MB, and its export from 33.8 s to 37.7 s on the same runner CPU type. The strips are cut at every trim vertex's `u` as well as at the chord step, as the grid's U values are, which keeps the strip edges on the plane faces' refined edges at the price of thin strips.

Some samples count more boxes after than before (Snowdon Towers architectural 314 → 326): an element whose saved graphics gave no surface at all is not counted as kept, and one that now gives part of one is.

## 4. Not claimed

- Faces with several trim loops.
- Whether #1247's 341 beams are among what this draws: no beam in the corpora measured here lost its saved mesh. If their loss came from a cut or mitred steel section's fillet faces, it would.

## 5. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=1b983f8a -f base=3e0ad51d -f ifc_flags=--saved-meshes \
  -f nordic=true -f samples=true -f sample_years="2024 2024" \
  -f models="core-interior re1-* nordic-office-* sample-2024-*" -f scorers="mesh_bodies_vs_ifc schema"
```
