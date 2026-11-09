# RE-384: a sloped floor's PitchAngle is the angle of its top plane

**Date:** 2026-11-09
**Issues:** #1193 (W7: sloped slab `PitchAngle` is one of the values it lists); follows RE-238 (a level slab's `PitchAngle`, which left the sloped ones out because rvt-rs drew them level), #1384 (a floor whose shape-edited corners lie in one plane is drawn along it) and RE-295 and RE-298 (the sloped floors' quantities)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, as Measure fetches them)
**Runs:** Measure [42656294752](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42656294752) and [42658128847](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42658128847) (the property census of the two office files with Revit's set placement, `main` at `060e8f0a`, the second listing the objects missing `Pset_SlabCommon.PitchAngle`), the office's rvt-rs export from Measure [42655806838](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42655806838) (`keep_ifc`)
**Status:** @@STATUS@@

## 1. What Revit writes

RE-238: revit-ifc's `PitchAngleCalculator` is its `SlopeCalculator` (CLEANROOM.md source 5), which gives a slab with no beam ends, no ramp slope and no location curve the slope of the largest upward face of its geometry. The office's export writes `Pset_SlabCommon.PitchAngle` on 62 slabs: 0 on 54 and an angle on the 8 sloped floors (`AP-1` base slabs and `AP-3` gutter floors, 1.9 to 24 degrees), which the property census lists as missing on both files:

| Slab (ElementId, type) | Revit's `PitchAngle`, rad |
|---|---:|
| 1994321, 1994605 (`AP-3`) | 0.0688565 (both) |
| 1994706 (`AP-3`) | 0.0329099 |
| 1994714 (`AP-3`) | 0.0761881 |
| 2057322, 2057673 (`AP-1`) | 0.3776518, 0.3776533 |
| 2058504 (`AP-1`) | 0.4196032 |
| 2070694 (`AP-1`) | 0.3948475 |

## 2. What rvt-rs holds

#1384 reads a floor whose corner vertices are shape-edited and lie in one plane, `z = a·x + b·y + c` in model feet (`slab_top_plane_from_fields`), and draws the floor along it as a faceted solid when the plane reproduces the record box (`floor_slope_solid`, `BodySource` `FLOOR_SHAPE_BODY_SOURCE`). RE-238's statement that rvt-rs draws these floors level predates it: all 8 are drawn along their plane, and `PitchAngle` was the value left out. The angle of the largest upward face of rvt-rs's own body of each, measured on its export of the office as built (Measure 42655806838, `main` at `060e8f0a`), is Revit's value to 2e-7 rad on all 8 (to 4e-9 on six of them; the other two, `AP-1` floors 2057322 and 2070694, differ by 1.7e-7 and 1.6e-7).

`add_shaped_floor_pitch` (`src/ifc/export_content.rs`) writes `PitchAngle` as the arctangent of the length of the plane's gradient, `atan(hypot(a, b))`, in `Pset_SlabCommon` on a floor drawn that way (`shaped_floor`); a floor whose plane is read but whose body is not drawn from it gets none, and a level one keeps RE-238's 0.

## 3. Measured

@@MEASURE@@

## 4. Not shown

- **The sloped floors' `Depth` and `GrossVolume`** (RE-298: the layers' thickness times the cosine of the slope on a floor of one layer, which the four `AP-3` floors are) are not written here; the angle is the input they need, and Revit's values for them are not listed by the census.
- **A floor sloped by a slope arrow, or shape-edited with corners not in one plane**, is not drawn (#1384) and gets no `PitchAngle`.
- **Revit 2025 to 2027:** the shape-edited plane is read on the releases #1384 measured; no export of another release holds a sloped floor to check the angle against.

## 5. Reproduce

```bash
# The census that lists the sloped floors missing the value (office, Revit's set placement on):
gh workflow run measure.yml --ref main -f ref=main -f base=none -f nordic=true \
    -f models="nordic-office-as-built nordic-office-permit" -f scorers=psets_vs_ifc \
    -f ifc_flags=--revit-set-placement -f psets_missing=Pset_SlabCommon.PitchAngle
# rvt-rs's export, whose bodies' angles RE-384 compares (the angle of the largest upward face of each listed slab is the check of section 2):
gh workflow run measure.yml --ref main -f ref=main -f base=none -f nordic=true \
    -f models="nordic-office-as-built nordic-office-permit" -f scorers=none -f keep_ifc=true
```
