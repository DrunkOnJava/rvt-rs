# RE-411: RvtIO's acceptance rule for a round column, run on the saved mesh of 6,672 columns beside RE-353's rule

**Date:** 2026-11-10
**Issues:** #1203 (the round column), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); discussion #1241 (RvtIO, the offer of its rule); follows RE-291 (a box-drawn column's section from its type) and RE-353 (the round column)
**Artefacts:** Autodesk's sample projects of 2015 to 2027 (pinned in `research/autodesk-sample-projects.tsv`), the BIM4LCA housing and office files (`research/bim4lca-measure.tsv`, CC BY-SA 4.0), puzzbobb's Revit 2025 models, the four RE1 models, Core Interior and Einhoven: 100 models, 6,672 columns of the classes `Column` and `StructuralColumn`, 3,993 of them with saved graphics (Revit 2023 to 2027). Never stored.
**Probe:** `probes/examples/probe_1203_round_column_rule.rs`
**Runs:** Measure [42927548215](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42927548215) (the probe at `d411801d`, every number below; `-f samples=true -f nordic=true -f r2025=true -f base=none -f scorers=none`), kept under `refs/archive/2026-11-10/cited/d411801d2fb3`
**Status:** Positive, and it finds a defect: RvtIO's rule on the saved meshes takes 1,902 columns as round where RE-353's caption rule takes 6, and 24 of the 1,902 carry the areas and volumes of a square. RE-291 reads a column type whose only length is `b` as a square section; 24 round columns hold such a type. The repair is RE-416, in its own change.

## 1. What was offered

RvtIO (discussion #1241, 2026-11-10) takes a column for a vertical cylinder in `RoundCylinderCurrent` when exactly two of its decoded faces are planes, each with one loop, no hole and at least eight boundary points; both horizontal, on circles of one centre and radius, at elevations at least 1e-9 ft apart; each cap's calculated area (`QuadratureArea`, computed from the stored loop's points) within 0.5% of pi r squared; and every point of every face on that cylinder between the caps, to `max(1e-6 r, 8 ULP32(largest coordinate))`. No `CylSurf` record is required of the side: a decoded tessellated side can pass. We said we would run it on the office's round columns beside RE-353 and post the counts and the columns where the two disagree.

## 2. What was run

RvtIO decodes the faces of a column's geometry; rvt-rs reads the mesh Revit saved for it, one primitive for each face. The probe applies the same criteria to those faces: a face is planar where its points lie within `8 ULP32` of one plane, a cap's boundary is the edges used by one triangle (one simple loop, a disk), the circle is a least-squares fit of the boundary in plan, and a cap's area is that of its triangles. A cap's area is therefore a polygon's, not the quadrature of the stored arc, so it is under the circle's by what its chords leave: 0.5% is met by a cap of about 40 points or more. The probe also reads the type's named lengths (RE-291, RE-353) and runs the export, to say what `Qto_ColumnBaseQuantities` rvt-rs writes for each column.

## 3. The counts

| | columns |
|---|---:|
| columns, 100 models | 6,672 |
| with no saved graphics read (2,658 of Revit 2015 to 2022, 21 of Revit 2024) | 2,679 |
| round by both rules | 4 |
| round by RE-353's rule only | 2 |
| round by RvtIO's rule only | 1,898 |
| round by neither | 2,089 |

- **Both:** the office's one round column (3053209, `Round Column:PI-6`, radius 0.24606 ft, a cap of 48 points at 0.99599 of pi r squared) on each of the four office files.
- **RE-353 only:** the 2 columns (1206189 and 1211583) in Snowdon Towers Architectural whose type's `Diameter` is 0.3333 ft (4 in). Their saved mesh's caps reach 0.99398 of pi r squared, short of the 0.995 the rule needs. The shortfall is the mesh's chords, which RvtIO's area from the stored loop does not have, so this probe cannot say whether RvtIO's reader takes them.
- **Neither:** all 2,089 are refused for the number of planar faces ("n planar faces, not 2"): boxes, steel sections and other solids.

## 4. The 1,898 columns only RvtIO's rule takes, by type

Grouped by the named length parameters of the column's type (`type_named_lengths`, RE-291):

| the type's named lengths | columns round by RvtIO's rule | files |
|---|---:|---|
| `Durchmesser` 0.9843 ft and `Radius` 0.4921 ft (German captions) | 1,048 | the 8 rme advanced and basic samples of 2024 to 2027 |
| `b` 0.9843 ft alone | 656 | rac_advanced 2024 to 2027 |
| `b` 1.4764 ft alone | 24 | rac_advanced 2024 to 2027 (6 in each, of one type; a second type with the same `b` has one column in each rst_basic file, which the cylinder test does not take) |
| none read | 170 | rac_advanced 2023 (164 of radius 0.4921 ft and 6 of 0.7382 ft; a type's named lengths are read for Revit 2024 to 2027 only) |

RE-353 takes a column as round only where the caption is `Diameter` and the offsets are zero, so none of the 1,898 is reached: the German captions are not `Diameter`, and a type that holds `b` alone is read by RE-291 as a square (`rectangular_section`).

## 5. What rvt-rs writes for them

`Qto_ColumnBaseQuantities` of the 1,902 columns RvtIO's rule takes:

| quantities | columns |
|---|---:|
| `Length` alone | 1,704 (the 1,048 and the 656) |
| none | 170 (rac_advanced 2023) |
| `Length` and `OuterSurfaceArea`, the circle's (RE-353) | 4 (the office's) |
| `Length`, `CrossSectionArea`, `OuterSurfaceArea`, `GrossVolume`, `NetVolume`, **the square's** | **24** |

The 24 are the type with `b` 1.4764 ft (450 mm) in rac_advanced 2024, 2025, 2026 and 2027, ElementIds 201277 to 201282 in each: a cylinder of radius 0.7382 ft and height 6 ft, with caps of 88 points at 0.9987 of pi r squared. `b` is the circle's diameter, so the section RE-291 reads for them is the circle's bounding square, and the pass writes that square's quantities: `CrossSectionArea` 2.1797 ft² where the circle is 1.7119 ft² (ratio 1.273, 4/pi), `OuterSurfaceArea` 35.433 ft² where the cylinder's side is 27.829 ft² (1.273), and `GrossVolume` and `NetVolume` 13.078 ft³, 1.273 times the cylinder's. The pass tells a round column from a square one by its type holding a `Diameter` (`column_quantities.rs`: "a round column is drawn as a box too, and its type holds a `Diameter`"), which is how the office's is; these 24 are round and their type holds `b` alone. Measure holds no Revit export of the 2024 to 2027 saves (its one export of the sample is the three.js IFC2X3 export of the 2015 to 2020 state, `research/rac-advanced-export.tsv`), so the 24 values are not scored: they are wrong by the geometry alone.

## 6. What it means

- **The defect is RE-416.** A column drawn from its saved mesh keeps the box's `CrossSectionArea`, `OuterSurfaceArea`, `GrossVolume` and `NetVolume` only where the box's volume is within 2% of the mesh's, and keeps its `Length`.
- **RE-353's caption rule is narrow.** Geometry shows 1,898 round columns in the samples that it does not take, in three kinds of type. Whether `OuterSurfaceArea` (the circumference times the height, exact on the office's column, RE-353 section 4) is written for them is a separate decision: Measure holds no Revit export of those files to score it against, so it is left as it is.
- **The two rules agree wherever both read a column**, but for the 2 Snowdon columns of section 3, whose mesh caps are 0.6% under the circle, 0.1% past what the rule allows.

## 7. Not shown

- **RvtIO's reader.** This is its rule applied to the faces of the saved mesh, with a cap's area from the mesh's triangles; where its reader takes the area from a stored loop's arcs, more caps meet 0.5%.
- **Revit's values for the 1,898.** Measure holds no Revit export of those saves.
- **A column with no saved graphics read.** 2,658 columns of Revit 2015 to 2022 and 21 of Revit 2024 (20 of them in Snowdon Towers Structural) have none, so RvtIO's rule cannot be run on them; RE-353 takes none of them as round.

## 8. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=d411801d2fb3aaa29893c52e0b8bb8cf736cc5dd -f base=none \
  -f samples=true -f nordic=true -f r2025=true -f probe=probe_1203_round_column_rule -f probe_scorer=- -f scorers=none
```

The probe's `column` lines give each column's two verdicts, its circle and the quantities written; its `type` lines group the columns by the named lengths of their type; its `tally` lines count them.

## 9. Sources

RE-291, RE-353; RvtIO's rule as its maintainer described it on discussion #1241 (its source was not read here). No Autodesk source was read.
