# RE-211: A roof by extrusion, from its sketch's B-spline

**Date:** 2026-10-23
**Issues:** #883 (roofs by extrusion are level blocks); follows RE-199 (#356, footprint roofs as their planes), RE-25 and RE-97 (a sketch line's owner and a Sketch's curve list), RE-50 and RE-96 (a sketch line's segment and arc).
**Artefacts:** `11_RoofByExtrusion` in puzzbobb's `R2025_Models.zip` (#356, Apache-2.0, pinned in `research/r2025-models.tsv`), Revit 2025, with Revit 25.0.2's IFC4 Reference View export of it. Measure fetches both with `-f r2025=true` and never stores them.
**Probe:** `examples/probe_re211_extrusion_roofs.rs`, in Measure runs [40001606403](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40001606403), [40002615569](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40002615569) and [40003971692](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40003971692) (`re/883-extrusion-roofs`).
**Scorer:** `tools/re/extrusion_roofs_vs_ifc.py`.
**Status:** positive on the one roof by extrusion measured.

## 1. The roof

Roof 4262 is the model's one roof. Its record box is 1.2258 to 41.4901 ft in x, 1.8189 to 20.0440 ft in y and 12.6157 to 23.0616 ft in z. Revit's body has that box, and 1,187 vertices: a curved top and underside, meshed across both curves. rvt-rs wrote it as a level block as tall as the box.

## 2. The sketch is a B-spline in a vertical plane

The roof's one sketch line, 4300, names the roof as its owner (RE-25). Its record box is flat in y, at 1.81892 ft, the box's own low end in y: the sketch's plane. Neither a line (RE-50) nor an arc (RE-96) reads from its data. After the second bounded-curve tag (`04 00 08 01`, at +500 of its data, the `u64` before it 1, as an arc's) is a B-spline:

```text
+4   f64           start parameter          0
+12  f64           end parameter            34.035295
+20  u32           degree                   3
+24  u8 × 2        flags                    0, 0
+26  u32           control points           6
+30  6 × f64 × 4   weight, x, y, z           1, (1.2258, 1.8189, 13.1234) ... 1, (41.4901, 1.8189, 23.0616)
     u32           knots                    10
     10 × f64      knots                    0, 0, 0, 0, 12.336, 22.4611, 34.0353 × 4
```

That is a clamped cubic, every weight 1. The knot count is the control points plus the degree plus one, the knots rise, and the parameters span them. The curve runs through its first and last control points. It rises along x throughout, from z 13.1234 at x 1.2258 to z 23.0616 at x 41.4901. The sketch line's record box is the control points' box: its top, 23.1884 ft, is the third control point's z, above the curve.

## 3. The section

The curve is the roof's top. The underside is the top offset down, square to it, by the roof type's thickness. Both ends are cut plumb, at the curve's ends, each with the underside carried along its tangent to the cut. The thickness the record box's bottom implies under that rule is 0.49993 ft, against the type's 0.5 ft. The roof runs along y over its box, from the sketch's plane, 18.225066 ft. That length is also in the roof's data, at +122.

Measured against Revit's body (run 40003971692):

| | |
|---|---:|
| the section's box against the record box | equal, x and z |
| Revit's vertices within 0.0001 ft of the section's outline | 1,177 of 1,187 |
| within 0.001 ft | 1,187 of 1,187 (worst 0.000102 ft) |
| volume, the section over the box's length against Revit's body | 411.9956 against 411.9355 ft³ (1.000146) |

The vertices off by more than 0.0001 ft are on the curved faces, where Revit's mesh cuts the curve with chords.

## 4. What rvt-rs does

- `partition_beam_axes::bounded_spline_at` reads the record above, on Revit 2025, the one release it is measured on. It refuses unless the values are finite, the degree is 1 to 9 with more control points than that and at most 4,096, the weights are positive, the knots are as many as needed and rising, and the parameters lie within them.
- `partition_schema_mvp::attach_extrusion_roofs` reads each roof whose sketch lines are all flat along one horizontal axis, at one place. It joins the lines end to end into a profile rising along the plane's other horizontal axis, each a B-spline or a straight line within its record box. It offsets the profile by the type's layers and cuts its ends plumb. It keeps the section only where it reproduces the roof's record box across the plane, to 0.001 ft.
- `export_content::roof_extrusion_solid` writes the section over the record box along the axis as a closed faceted brep, with `BodySource` `partition_roof_extrusion`.

The export, measured against `main` in Measure run [40008435453](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40008435453) (`-f r2025=true`):

| | `main` | this change |
|---|---:|---:|
| roof 4262's volume against Revit's 411.9355 ft³ | 7,665.40 ft³ (the record box) | 412.0527 ft³ (1.00028) |
| Revit's vertices within 0.001 ft of rvt-rs's surface | | 1,187 of 1,187 (worst 0.000541 ft) |

`roof_bodies_vs_ifc` now also measures each of Revit's vertices against rvt-rs's surface. Its vertex test pairs each of Revit's vertices with rvt-rs's nearest vertex, and a body swept along a straight run needs no vertices between its ends, where Revit's mesh has them, so that test reports 9.11 ft here: half the run. On the other 18 roofs of the R2025 models both tests pass as before. No other output of any model Measure scores changed.

## 5. What it does not cover

- One roof, on one release. A roof by extrusion on 2024, 2026 or 2027 keeps its box, as one whose sketch holds an arc does: a sketch arc is read on 2024 only (RE-96).
- The roof is extruded from its sketch's plane over the record box. Revit's Extrusion Start and End, which may start it off the plane, are read only as far as the box gives them.
- A profile that turns back along its plane, or whose underside crosses itself where the top curves tighter than the thickness, keeps its box.
