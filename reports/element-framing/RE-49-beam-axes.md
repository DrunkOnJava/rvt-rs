# RE-49 — Beams along their location lines

**Date:** 2026-09-23
**Result:**
- **Positive.** A structural-framing element's serialised data carries its location line as a bounded line record: the two parameters, an origin and a unit direction.
- With the element's record box, the line gives the beam's solid: a box `length × width × depth` along the line.
- rvt-rs now exports a level beam as its section's plan rectangle along the line, rotated to the line's plan angle. A sloped beam becomes its section swept along its centreline (`IfcFixedReferenceSweptAreaSolid`).
- Before, every beam was the axis-aligned box around it. For a beam rotated in plan that box is several times the beam: 8.4 times Revit's own beam envelope at the median, against 1.11 times now.
- On Snowdon Towers Structural, 923 of the 942 exported beams run along their line. The section is Revit's exactly on 613 of the 839 beams a later edition kept unchanged, and on 179 more it is Revit's less the top a floor join cuts away.
- A beam whose box no solid along the line reproduces keeps its box.

**Probe:** `examples/probe_re49_beam_axes.rs` counts the lines and solves in a file. Given a VIM export of the same model, it also scores every solved beam against the mesh the VIM holds for it.

**Oracle:** Snowdon Towers Sample Structural (local only, `.rvt` sha256 `6dc833f9…`). The geometry oracle is a VIM export of the sample's 2027 edition (`vimaec/vim-hackathon` `vims/Snowdon.r2027.vim`, sha256 `3e7de636…`, local only): its per-element meshes are Revit's own tessellated geometry. Revit's IFC export of the architectural sample holds no beam, and no licensed file in CI has structural framing.

-----

## 1. The line

In a structural-framing element's data (after its element-data header and ElementId, RE-44), the first occurrence of

```text
04 00 08 01
```

starts a bounded line:

| offset | type | beam 627866 (W18x55) |
|---:|---|---|
| +0 | tag | `04 00 08 01` |
| +4 | f64, feet | start parameter 0.405647 |
| +12 | f64, feet | end parameter 23.015746 |
| +20 | f64 × 3, model feet | origin (61.767124, 41.951037, 18.333333) |
| +44 | f64 × 3 | unit direction (0.130526, −0.991445, 0) |

- The line's ends are `origin + parameter · direction`. For 627866 they lie at the top of the steel (z 18.333, the record box's top), 22.610 ft apart.
- The data holds a second line with the same parameters, origin and direction at the section's mid-depth (z 17.579).
- The line sits between 0x4a1 and 0x1000 bytes into the data on 899 of the 942 beams. The longer data of some concrete beams puts it up to 0x64a8 bytes in, so it is found, not assumed.
- The same record is in the data of walls, model lines and sketch lines. Only structural framing is read.

On Snowdon every one of the 942 exported structural-framing elements has a line. On 940 both ends lie inside the element's record box to the last bit. On the other 2 (686634 and 793189) they lie several feet outside, and those lines are not read.

## 2. The solid

A box along a unit direction `d`, `L` long, `W` wide across it horizontally (`u`) and `D` deep across it in the line's vertical plane (`w = d × u`), has on each model axis the extent

```text
E = L·|d| + W·|u| + D·|w|
```

- `L` is the line's length.
- The vertical extent gives `D`, since `u` is level.
- One plan extent gives `W`. The other must then come out right: it is a check the solve must pass.
- The solved box's centre is the record box's.

The check passes to 0.01 ft on 923 of the 940 beams:

| | beams |
|---|---:|
| level, on a model axis | 802 |
| level, rotated in plan | 111 |
| sloped | 10 |
| no box along the line reproduces the record box | 17 |

- 15 of the 17 lie at −24.5° or 65.5° and frame obliquely into members on the model axes. Their end cuts follow those members rather than running square to the beam, so the shape is a parallelogram in plan and no box reproduces its extents.
- The other 2 (665215 and 705161) are on a model axis, and their record box runs 0.42 ft longer than the line.
- All 17 keep their record box.

Exported, a level beam is `Extrusion::rectangle(L, W, D)`, placed at the record box's plan centre and base and rotated to the line's plan angle. A sloped beam keeps its record box as its extrusion, and its solid is a `SolidShape::SweptPath` of the `D × W` section along the centreline, with `+Z` as the fixed reference. The element property set gains `BodySource = partition_beam_axis`, `AxisLengthFeet`, `SectionWidthFeet` and `SectionDepthFeet`.

IfcOpenShell 0.8.5 meshes every exported beam's solid to a box equal to its record box: 938 to 0.001 ft and 4 to 0.0016 ft. The 10 swept solids come out 0.5913 ft wide and 0.2917 ft deep across their axis, as solved.

## 3. Measured against Revit's geometry

The VIM is of a later edition of the model, which regenerated its beam systems and moved or retyped some beams. A beam is scored only when three things hold:

- its VIM `Location` is the line's midpoint to 0.01 ft;
- its VIM family and type are the ones rvt-rs names;
- its VIM mesh holds that `Location` across the line.

Of the 923 solved beams, 28 are not in the VIM, 40 moved, and 16 have a mesh that does not hold its own `Location` (beam-system joists whose mesh is one or more joist spacings away). That leaves 839.

**Section:** the mesh's width and depth across the line, and its centre, against the solid's (0.01 ft):

| | level, on a model axis | level, rotated in plan |
|---|---:|---:|
| equal | 561 | 52 |
| equal less the top: a floor join cut it | 176 | 3 |
| differs | 27 | 20 |

- "Less the top" means the width and the across-line centre are equal, and the depth is short by twice the drop in the depth centre. That is a concrete beam whose top Revit cuts where a floor joins it (2 ft beams read 1.167 ft deep in the mesh).
- All 20 rotated beams that differ have the same section 0.208 ft (2.5 in) lower in the mesh.
- Of the 27 level beams that differ, 15 are 0.341 ft wider than the mesh, whose section is 0.171 ft off to one side (5 of them also 0.208 ft deeper). In the other 12 the mesh lies 10.6 to 27 ft above or below the VIM's own `Location`. That is an inconsistency inside the VIM, which the across-line filter does not catch.
- The 10 sloped beams all moved in the later edition, so none is scored here. Section 2's IfcOpenShell check stands for them.

**Ends:** the mesh's extent along the line against the line's:

| | level, on a model axis | level, rotated in plan |
|---|---:|---:|
| equal | 485 | 0 |
| mesh trimmed inside the line | 279 | 73 |
| mesh runs past an end | 0 | 2 |

Revit trims a beam where it frames into a column or girder. The larger end trim runs from 0.016 ft to 6.174 ft, with a median of 0.625 ft (352 beams). Nothing in the beam's data found so far holds these trims. They follow from the supporting member's section and the connection's cutback, so the solid runs the line's full length.

**Volume:** for the 75 scored rotated beams, the solid's volume is 1.108 times the volume of the mesh's own box along the line at the median (1.403 at most, from the trims). The record box it replaces is 8.406 times at the median, and 18.994 at most.

## 4. What this does not do

- **Beam ends are not trimmed** at their supports: 352 of 839 scored beams run past Revit's by a median 0.625 ft.
- **No floor join cuts** the top of a concrete beam (179 beams).
- **The section is a rectangle.** A W shape's flanges and web are not drawn: the solid is the section's envelope. The section's family parameters are not read.
- **Skewed ends are not drawn:** 15 oblique beams, and 2 whose box is longer than their line, keep their record box.
- **A rolled section** (a beam rotated about its own axis) would be solved as the envelope of the rolled section across the line. None is identified on Snowdon.
- **No `Axis` representation** is written: the centreline is only the swept solid's directrix.
- It reads Revit 2024 only. No other release in the corpus has structural framing.
