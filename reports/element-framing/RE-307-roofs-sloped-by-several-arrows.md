# RE-307: Roofs sloped by several slope arrows: the T drawn as its planes, the U measured and left as its box

**Date:** 2026-11-03
**Issues:** #885 (the roof cases with no oracle); follows RE-199 §5 (one slope arrow) and §8 (several), and RE-97 and RE-50 (a Sketch's curve list and a sketch line's bounded line).
**Artefacts:** `20_T_U_Roof` in puzzbobb's `R2025_TU_Roof.zip` (#885, Apache-2.0, sha256 `38dad62722d5a6e1cf03405f99f4ece060ec7473e34dbb45de9e72247ca47e6e`, pinned in `research/r2025-models.tsv`): a Revit 2025 project with two footprint roofs, a T (roof 4149) and a U (roof 5941), and Revit 25.0.2's IFC4 Reference View export of it. Measure fetches both with `-f r2025=true` and never stores them.
**Probe:** `probes/examples/probe_re307_arrow_roofs.rs`, in Measure run [41642079234](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41642079234) and [41642988246](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41642988246) (`re/885-multi-arrow-roofs`).
**Scorer:** `tools/re/roof_bodies_vs_ifc.py`.
**Status:** positive for the T, negative for the U. The T's body is Revit's; the U keeps its record box, because the rule that gives Revit's U is measured but not established beyond it.

## 1. What the arrows hold

Both roofs are footprint roofs of one loop of eight boundary lines, with a rafter cut of Plumb (RE-199 §4), a fascia depth of 0, a roof type 0.5 ft thick, and no edge that defines a slope (every boundary line reads `defines_slope` false). Each also holds six slope arrows: sketch lines with RE-49's bounded line from the tail to the head and RE-199 §5's `SlopeCurveData`. On all twelve arrows the tail height is 0, the head height is the one set by "Height at Tail" (`m_useAngle` 0), and the angle field holds the unused 0.3217506 rad.

The probe finds, for every arrow, one boundary line that the tail lies on (within 1e-4 ft) and the arrow is square to (the cosine between them at most 3.3e-15). The slope is the head's height over the arrow's run:

| roof | arrow | boundary line it starts on | run (ft) | head height (ft) | rise over run |
|---|---|---|---:|---:|---:|
| T 4149 | 4451 | 4166, the bar's south eave | 5.24934 | 4.92126 | 0.937500 |
| T 4149 | 4605 | 4160, the bar's north eave east of the stem | 3.60892 | 9.84252 | 2.727273 |
| T 4149 | 4569 | 4172, the bar's north eave west of the stem | 3.60892 | 4.92126 | 1.363636 |
| T 4149 | 4214 | 4175, the stem's west wall | 9.51444 | 4.92126 | 0.517241 |
| T 4149 | 5531 | 4157, the stem's east wall | 9.51444 | 9.84252 | 1.034483 |
| T 4149 | 4984 | 4154, the stem's end | 4.92126 | 6.56168 | 1.333333 |
| U 5941 | 6733 | 5962, the long west wall | 6.88976 | 9.84252 | 1.428571 |
| U 5941 | 6762 | 5947, the west arm's east wall | 7.54593 | 9.84252 | 1.304348 |
| U 5941 | 6810 | 5953, the east arm's west wall | 6.88976 | 9.84252 | 1.428571 |
| U 5941 | 6845 | 5956, the long east wall | 7.21785 | 9.84252 | 1.363636 |
| U 5941 | 6937 | 5950, the notch's floor | 8.20210 | 9.84252 | 1.200000 |
| U 5941 | 6975 | 5959, the bar's south eave | 3.93701 | 9.84252 | 2.500000 |

Every head stops short of the ridge. The T's two bar ends (4163, 4169) and the U's two arm tops (5944, 5966) carry no arrow, and Revit's export holds them upright.

## 2. The planes

Revit's export agrees with each arrow being a plane through its boundary line, rising into the roof at the arrow's slope, as RE-199 §5 found for one arrow:

- **The underside** of an arrow's plane is level with the record box's base along its boundary line, and rises at the slope.
- **The top** lies the plane's own `T / cos θ` above it, so two arrows of different slopes stand at different eave heights. The T's tops at their eaves are 208.9 mm (slope 0.9375), 257.7 (1.3636), 442.7 (2.7273), 171.6 (0.5172), 219.3 (1.0345) and 254.0 (1.3333), each `0.5 ft · √(1 + slope²)`. RE-199's defining edges share one eave height (`T / cos θ` of the shallowest); arrows do not.
- **Where planes meet** the roof is the straight-skeleton roof RE-199 found: the lowest of the counting planes at a hip, the higher at a valley.

The T needs two refinements of RE-199's wedges, both visible only because the verticals now differ from plane to plane:

1. **Each surface has its own wedges.** A plane counts between the lines where it meets the planes of the edges that share an end with its edge, and the line where two tops meet is not the line where their undersides meet. At the T's reflex vertex (2200, 2700 mm) the undersides of the stem's west plane and the bar's north-west plane meet on the line through that vertex, and the tops meet 63.2 mm (0.2072 ft) north of it, at Revit's vertex (2200, 2763.2).
2. **A top is not cut at its edge's line.** The underside counts only on its edge's side. The top plane reaches past the line to the valley with its neighbour's top: Revit's north-west top plane covers the strip between y = 2700 and 2763.2 mm on the stem's side of the bar's eave line.

## 3. The T, measured

Roof 4149 (Revit's body: 32 vertices, 20 faces), Measure run 41642079234:

| | `main` | this change |
|---|---:|---:|
| farthest vertex of Revit's body from a vertex of rvt-rs's | 10.4 ft | 3.16e-06 ft |
| every vertex of Revit's body from rvt-rs's surface | 2.26 ft | 1.98e-06 ft |
| volume against Revit's 436.707420 ft³ | 4,426.405064 (the outline over the record box) | 436.707816 (1.0000009) |

The body is Revit's. Off line, the same rule, modelled in numpy on a 10 mm grid across the outline, agrees with Revit's top and underside faces at every one of 572,970 sample points to 0.5 mm, and its volume equals the volume sampled from Revit's faces (436.6846 ft³ both).

## 4. The U, measured

Roof 5941 (Revit's body: 34 vertices): the wedges of §2 applied to the U's six edges draw hips where the long west and east walls meet the bar's south eave, and they are not Revit's. Revit does not slope those two walls along their whole length:

- Face by face, Revit's west wall (x = 0) is the bar's gable section from the south eave (y = 0, top 410.4 mm) up to the ridge at 5,726 mm and down to y = 6,700, then a strip 265.8 mm tall along the west arm to y = 12,900. Below y = 6,700 the wall is upright; the west arm's plane (arrow 6733) meets the wall only along the arm. The east wall is the same.
- The arm planes reach down into the bar, to a valley with the notch plane (arrow 6937) that starts on the wall at the point (0, 6700), where the line of the notch's floor meets it, and runs to the ridge's end at (2625, 3575).

Fitted to Revit's export offline (numpy on the same 10 mm grid, over all 1,847,300 sample points of the outline), the underside is

`min(S, max(N, min(A1, A2), min(B1, B2)))`

where S is the south arrow's plane (6975), N the notch's (6937), A1 and A2 the west arm's west and east planes (6733, 6762) and B1 and B2 the east arm's (6810, 6845); the top is the same expression over the planes raised by their own `T / cos θ`. It agrees with Revit's top and underside at every sample to 0.5 mm, and the volumes agree (1,845.5613 ft³ against 1,845.5613 sampled from Revit's faces; Revit's exported body is 1,845.577001). The fit is the evidence that the U is a bar gable with two arms sitting on its north plane. It is not a rule for footprints: what makes the two long walls sloped along the arms alone, and the south wall along its whole length, is not established. The fit is not committed as a tool; the numbers above are from a session script, and the next oracle in §7 would settle it.

## 5. What rvt-rs does

- `partition_schema_mvp::attach_roof_slopes` reads, for a roof with no defining edge and two or more slope arrows (the sketch's curve list, RE-97), each arrow's boundary line: the one its tail lies on to 1e-4 ft and it is square to, no line holding two. It requires tail heights of 0, a rising slope, and the head inside the outline, and gives the roof the line's ends and the arrow's slope as an edge (`m_roof_slope_edge_*`, `m_roof_slope_angle`), with `m_roof_arrow_planes` set.
- `export_content::roof_arrow_planes_solid` draws the planes of §2 through `body_geometry::envelope_slab_mesh_split`, which takes the underside's wedges apart from the top's. It requires a plumb cut with no fascia, a single loop without holes, every sloped edge to be an outline edge, and the highest point of the top and the lowest of the underside to reproduce the record box's height and base to 1e-4 ft. Roofs sloped by defining edges, and by one arrow, go through the code they did.
- `arrow_roof_hip_ambiguous` keeps the U as its box. A sloped edge that the extension of a reflex edge meets inside it, with a sloped neighbour across a convex corner, is the configuration in which the U's walls are sloped along part of their length. The T's south eave is met inside by the stem's walls the same way, but its corners lie against upright ends, and the wedges are exact there. Such a roof keeps its box. The probe prints the extensions and the corners (`probe_re307_arrow_roofs`).

## 6. Measure

Run 41642988246 (`-f base=main -f nordic=true -f r2025=true`, every scorer):

Head `ab2f4804` against `main` at `66cd707c`, 32 models (Core Interior, Einhoven, the four RE1 models, both BIM4LCA office models, and puzzbobb's 20 R2025 models):

- **Exports changed:** one of 32, `r2025-20_T_U_Roof`. The other 31 are byte-identical (`ifc.sha256`), and every scorer's output for them is unchanged.
- **In `20_T_U_Roof`:** roof 4149 changes from a swept solid to a faceted brep and is scored as in §3. Roof 5941 is unchanged (37,356.98 ft³ against Revit's 1,845.58: its box). The scorers that moved with it are `roof_bodies_vs_ifc` (above), `schema` (swept solids carrying a Position distinct from their product placement, 18 to 17) and `wall_heights_vs_ifc` (the roof is a brep, rvt-rs's surfaces 56 to 996 triangles).
- **Roofs against Revit's body**, over the R2025 models' 22 (20 files with one roof, `20_T_U_Roof` with two): vertices within 0.001 ft and volume within 1e-4, 19 on `main` and 20 here; every vertex of Revit's body within 0.001 ft of rvt-rs's surface, 20 and 21. No roof's score got worse. The two that stay short are the roof by extrusion (RE-211: the vertex-to-vertex test is 9.11 ft by design, its surface test passes) and the U.
- The unfiltered run's probe output for `20_T_U_Roof` is byte-identical to the filtered run's (41642079234), whose head was the same commit.

Reproduce:

```text
gh workflow run measure.yml --ref main -f ref=<commit> -f base=main -f nordic=true -f r2025=true \
  -f probe=probe_re307_arrow_roofs -f probe_scorer=-
```

## 7. What this does not cover

- **The U, and any footprint whose long walls continue past an arm.** The guard leaves them as their record boxes. An oracle that would settle the rule: the U with the west arm's arrow moved onto the bar's part of the wall, one with the bar's south arrow off-centre, and an L with arrows, each with Revit's IFC4 export.
- **Arrows set by angle, or with a tail height other than 0.** 05 and 06 of the R2025 models carry a 22 mm tail height that is not in their body; several arrows with one are not drawn.
- **A two-cut or square rafter cut, a fascia depth, a hole in the footprint.** None is measured with several arrows.
- **Defining edges and arrows in one roof.** Not measured; such a roof keeps its box.
- **Hips over the T and U footprints with every edge defining (#884)** are a different input.
