# RE-199 — Gable, hip and L-shaped hip roofs, rafter cuts, slope arrows, and roof slopes on Revit 2025

**Date:** 2026-10-05
**Result:**
- **Positive.** Revit 2025 writes RE-56's edge slope block on roof sketch lines, in the same place and shape. The f64 at +0x3b, −rise/run on 2024's one defining edge, is on 2025 how far the edge's plane is raised at the edge (most likely Revit's "Plate Offset from Base").
- **Positive.** On 2025, a sketch line's data holds its segment as RE-49's bounded line, and a Sketch lists its curves as RE-97 found on 2024. Both readers refused 2025, so outlines of edited sketches, and every 2025 roof slope, were lost.
- **Positive.** A roof with two or more defining edges is its edges' straight-skeleton roof:
  - each plane rises from its edge at its angle;
  - each plane counts only within its edge's wedge, between the lines where it meets its neighbouring defining edges' planes. Those lines converge at a hip and part at a valley;
  - the roof's top is the lowest counting plane at each point;
  - the planes' tops meet the outline at one eave height, `T / cos θ` of the **shallowest** defining edge;
  - each underside lies its own `T / cos θ` below its top, cut off at the base.

  Over a convex footprint this is the lowest envelope of the planes. With equal pitches it is a plain gable, and with one edge RE-56's shed. Over an L it gives the valley.
- **Positive.** The rafter cut is parameter −1001710 of the roof's `ProfileRoof` record: 33615 plumb cut (the default), 33619 two-cut plumb, 33618 two-cut square. A two-cut is the planes with their tops a fascia depth `F` above a level cut at the eave, each plane cut at the base raised by its edge's plate offset.
  - `F` is the f64 just before the `i64` −1001711 in the roof's element data: 0.0 by default, 50 mm where set.
  - With `F = 0` the body is the plain one set down by `T / cos θ`, and the plumb and square cuts coincide.
- **Positive.** A slope arrow is a sketch line carrying `SlopeCurveData`: the tail's and head's height offsets, their levels' elevations, the slope angle, the two levels, and `m_useAngle`.
  - Set by "Height at Tail" (`m_useAngle` 0), the roof is a shed rising from the tail's height to the head's along the arrow.
  - Set by "Slope" (1), it rises at the angle from the tail's height.
- rvt-rs now draws all of these, each only where the result reproduces the roof's record box height (as RE-56 does for sheds).

**Oracle:** `R2025_Models.zip`, attached to #356 under Apache-2.0: 23 purpose-made Revit 2025 projects, all but `07_…` with Revit 25.0.2's IFC4 Reference View export. They share four walls; the roof type is 152.4 mm thick (T = 0.5 ft). The probe is `examples/probe_re199_roof_edge_slopes.rs`.

| file | roof | defining edges, or arrow | rafter cut |
|---|---|---|---|
| `02_1Slope` | shed, 20°, sketch edited | 1 | plumb |
| `02_1Slope_Rafter2PlumbCut` | `02_1Slope`, two-cut plumb | 1 | two-cut plumb |
| `02_1Slope_Rafter2SquareCut` | `02_1Slope`, two-cut square | 1 | two-cut square |
| `02_1Slope_Rafter2SquareCut_Overhang` | the square two-cut, one edge 33 mm off its wall | 1 | two-cut square |
| `03_2Slope_Rafter2SquareCut` | gable, square two-cut, one edge raised 54 mm | 2 | two-cut square |
| `04_Gable` | hip (despite its name) | 4 at 30° | plumb |
| `04_Gable_Swapped_Slopes` | hip | 4: 15° and 35° | plumb |
| `05_1Slope_ArrowDefined` | slope arrow, Height at Tail 22 / 3000 mm | arrow | plumb |
| `06_1Slope_ArrowDefined_SetToSlope` | `05`'s arrow set by Slope, 1:3 | arrow | plumb |
| `07_1Slope_ArrowDefined_2Cut50mmFascia` | `06` with a two-cut and a 50 mm fascia (no export) | arrow | two-cut plumb |
| `08_LShapedHipRoof` | L-shaped hip | 6 at 30° | plumb |
| `09_LShapedHipRoof_1Edge300Offset` | `08` with one edge raised 300 mm | 6 at 30° | plumb |
| `10_LShapedHipRoof_1Edge300Offset_Hole` | `09` with a hole in the sketch | 6 at 30°, 6 hole lines not defining | plumb |
| `11_RoofByExtrusion` | roof by extrusion (not read) | — | — |

The zip's other files hold no roof: `00_Blank`, `01_Walls`, `12_Rooms_Door_Windows`, the stairs `13` to `17`, and `18_WallJoins`.

-----

## 1. The 2025 edge block

```text
-0x18  f64      slope angle, radians
-0x10  00 × 16
 0x00  ff × 32
+0x20  i64      −6
+0x28  u32      0
+0x2c  u32      7
+0x30  u8 × 4   00 00 01 00, or 01 01 01 00 on a defining edge
+0x3b  f64      how far the edge's plane is raised (2024: −rise/run on a defining edge)
+0x47  f64      the edge's offset from its wall, where set
```

The block sits at +0x18b to +0x193 past each sketch line's ElementId.

`partition_roof_slopes::edge_slope_in(data, 2025)` reads the raise as `plate_offset_feet`. `edge_slope` keeps 2024's rule.

The raise is 0.0 on every edge but two:
- `03_2Slope_Rafter2SquareCut`'s south edge, 2906: 0.17713 ft (54 mm), with 0.10827 ft (33 mm) at +0x47. Its ridge sits off-centre in Revit's export (top faces of 31.76 and 30.04 m²), and the south plane raised 54 mm reproduces both the ridge and the record box.
- `09_LShapedHipRoof_1Edge300Offset`'s edge 3842, given a 300 mm offset: 0.98425 ft (300 mm).

## 2. Edited sketches

In `02_1Slope`, `03_2Slope_Rafter2SquareCut`, `05_1Slope_ArrowDefined` and the files made from them, some or all of the roof's sketch lines carry an owner reference to another element, as RE-97 found on 2024. The roof's Sketch lists them, in RE-97's layout:
- `02_1Slope` lists 2903, 2906, 2909, 2912;
- `05_1Slope_ArrowDefined` lists those four and its arrow, 3505.

`scan_sketch_curve_lists` now reads 2025. `attach_roof_slopes` takes a roof's edges from the list where there is one, and `sketch_plan_profiles` leaves a roof's arrow out of its outline.

## 3. The planes

Every defining edge's angle matches a face normal of Revit's export (IfcOpenShell 0.9.0).

The eave is set by the shallowest defining edge:
- `04_Gable_Swapped_Slopes` matches Revit's export with its eave at the 15° edges' `T / cos θ`, not the 35° edges';
- `04_Gable`, all 30°, is the equal-pitch case.

`08`, `09` and `10` are L-shaped. The plane of each edge at the L's inside corner crosses the roof, and is held to its wedge. Without the wedges, the lowest envelope cuts the main block down to those planes; with them, the valley is the higher of the two planes, and every vertex matches Revit's export.

## 4. Rafter cuts

A footprint roof is a `ProfileRoof`. Its `m_paramSet` (decoded by member name, `native_document::extract`) holds two pairs:
- −1001709, 0 on every roof;
- −1001710, the rafter cut.

| −1001710 | rafter cut | files |
|---|---|---|
| 33615 | Plumb Cut (default) | `02_1Slope`, `04_Gable`, `04_Gable_Swapped_Slopes`, `05`, `06`, `08`, `09`, `10` |
| 33619 | Two Cut – Plumb | `02_1Slope_Rafter2PlumbCut`, `07_1Slope_ArrowDefined_2Cut50mmFascia` |
| 33618 | Two Cut – Square | `02_1Slope_Rafter2SquareCut`, `02_1Slope_Rafter2SquareCut_Overhang`, `03_2Slope_Rafter2SquareCut` |

`02_1Slope`, `02_1Slope_Rafter2PlumbCut` and `02_1Slope_Rafter2SquareCut` are one roof with each cut. Their `ProfileRoof` records differ only in this value and in the geometry history Revit caches: one face generator more for the plumb two-cut, and 28 more for the square two-cut, with the face and edge histories of the square cut's faces. `partition_roof_slopes::scan_rafter_cuts` reads the cut on 2025.

A two-cut moves the record box. On `02_1Slope` → `02_1Slope_Rafter2PlumbCut`, its base rises by `T / cos θ` and its height falls by as much (6.92664 → 6.39456 ft). Revit's export is the plain body set down by `T / cos θ` and cut level at the eave's top:
- the underside shows a horizontal strip as wide as the depth over which the sloped underside lay below the cut (4.86 m² on `02_1Slope_Rafter2PlumbCut`);
- each plane is cut at its own eave's top. `03_2Slope_Rafter2SquareCut`'s raised south plane is cut 54 mm above the north plane.

The fascia depth `F` sets how far the planes' tops stand above the cut at the eave:
- `07_1Slope_ArrowDefined_2Cut50mmFascia`'s 50 mm fascia raises its record box by exactly 50 mm over the same roof with `F = 0`: 12.096 against 11.932 + 0.164 ft.
- `F` is read from the roof's element data, the f64 just before the first `i64` −1001711: 0.164042 ft on `07`, and 0.0 on every other roof, two-cuts included.

rvt-rs draws by the read cut:
- **plumb cut:** the plain body;
- **two-cut plumb:** the two-cut body with the read `F`;
- **two-cut square:** the two-cut body only where `F = 0`, the case measured.

Where the cut is not read, it draws the plain body, else the two-cut one that reproduces the record box.

## 5. Slope arrows

An arrow is a sketch line (record tag `1f 0f`, where boundary lines have `cf 0c`) with RE-49's bounded line from its tail to its head, and no RE-56 block. Its data holds `SlopeCurveData` in its members' order:

```text
-0x28  f64      height offset at the tail (m_startHeight), feet
-0x20  f64      height offset at the head (m_endHeight), feet
-0x18  f64 × 2  the levels' elevations, 0.0 on these arrows
-0x08  f64      slope angle (m_angle)
 0x00  ff × 16  the tail's and head's levels, none
+0x10  u32      m_useAngle: 0 set by "Height at Tail", 1 by "Slope"
```

| file | tail | head | length | angle | record box |
|---|---|---|---|---|---|
| `05_1Slope_ArrowDefined` | 22 mm | 3000 mm | 10911 mm | 0.3217506 (1:3), unused | matches Revit's export (§6) |
| `06_1Slope_ArrowDefined_SetToSlope` | 22 mm | (3000 mm, unused) | 10911 mm | 0.3217506, used | 35.797 / 3 + 0.5 / cos θ = 12.4595 ft against 12.459; matches Revit's export |

`06` is `05` with Specify set to Slope. The only byte that changes in the arrow is the word at +0x10, from 0 to 1. `arrow_heights` reads both; any other value there is not read.

## 6. End to end

`rvt-ifc` against Revit's export, meshed by IfcOpenShell:

| file | Revit vertices | farthest from rvt-rs's | volume, rvt-rs | volume, Revit |
|---|---|---|---|---|
| `02_1Slope` | 8 | 2.6e-6 ft | 334.639943 ft³ | 334.640341 ft³ |
| `02_1Slope_Rafter2PlumbCut` | 8 | 3.3e-6 | 320.718286 | 320.717684 |
| `02_1Slope_Rafter2SquareCut` | 8 | 3.3e-6 | 320.718286 | 320.717684 |
| `02_1Slope_Rafter2SquareCut_Overhang` | 8 | 3.1e-6 | 318.656072 | 318.655475 |
| `03_2Slope_Rafter2SquareCut` | 12 | 3.0e-6 | 304.732578 | 304.732818 |
| `04_Gable` | 12 | 2.4e-6 | 363.106249 | 363.106044 |
| `04_Gable_Swapped_Slopes` | 16 | 2.6e-6 | 331.029126 | 331.029631 |
| `05_1Slope_ArrowDefined` | 8 | 2.5e-6 | 325.960337 | 325.961323 |
| `06_1Slope_ArrowDefined_SetToSlope` | 8 | 2.3e-6 | 331.469560 | 331.468952 |
| `08_LShapedHipRoof` | 20 | 2.4e-6 | 356.504015 | 356.504300 |
| `09_LShapedHipRoof_1Edge300Offset` | 24 | 2.9e-6 | 356.503864 | 356.504300 |
| `10_LShapedHipRoof_1Edge300Offset_Hole` | 40 | 2.9e-6 | 336.967483 | 336.967803 |

`07_1Slope_ArrowDefined_2Cut50mmFascia` has no export. Its body reproduces its record box, 12.096 ft.

Before the change, each of these roofs was a level block as tall as its record box.

`export_content::roof_planes_solid` builds the bodies. `body_geometry::envelope_slab_mesh` cuts the plan triangulation along every line where two tops, two undersides, or an underside and a cut level meet, or a wedge ends, then lifts each piece. The result is an `IfcFacetedBrep`.

The same results hold on seven earlier imperial models of mine, not attached: a shed, a two-cut plumb shed, a 10:12 gable, and 10:12/9:12 hips both ways round, against their own exports or record boxes, with a 0 → 10 ft height arrow.

## 7. Elsewhere in the 2024 and 2025 corpus

`rvt-ifc` before and after, on every `.rvt` in the local 2024 and 2025 corpus, with the header timestamp and IfcOwnerHistory date left out:
- **Unchanged:** 2024 Core Interior, rac_basic and rst_basic 2024, the Snowdon Towers 2025 saves but Facades and Site, the three Japanese 2025 samples, and both Golden Nugget models.
- **rac_basic 2025:** floors 490319, 523064 and 647908 take their sketch outlines (§1's bounded lines and §2's lists). Their volumes, 1583.0391, 3044.1676 and 556.8381 ft³, are now the 2024 save's.
- **Snowdon Site 2025:** floor 1139554 takes its sketch outline, inside the same record box. There is no export to check it against.
- **Snowdon Facades 2025:** roof 1247352 is drawn as a 7.1° shed and roof 1248098 as a 4.8° gable. Both reproduce their record box height, but neither has a Revit export to check against. Roof 1247078 takes its sketch outline.

Reading the rafter cut walks the native records of roofs that get slope edges. Snowdon Facades exports in 11 s instead of 7; the other models' times are unchanged.

## 8. What this does not do

- **An arrow whose head stops short of the far edge.** Revit puts an arrow's tail on a boundary line, but its head may stop early, and the plane then runs on past the head. rvt-rs draws it that way, but no measured model has such an arrow.
- **A square two-cut with a fascia depth.** No measured model has one, so it stays a level block.
- **Footprints whose skeleton has split events** (an edge's wedge cut by a non-neighbouring edge's), such as a T or U with long arms. The wedges are each edge's first ones, which an L does not outgrow. The record box guards the rest.
- **Roofs by extrusion** (`11_RoofByExtrusion`: a curved profile, 1,187 vertices in Revit's export).
- **The 2024 tutorial house** (RE-56's oracle) is not on this machine. Its shed takes the unchanged single-edge path.
