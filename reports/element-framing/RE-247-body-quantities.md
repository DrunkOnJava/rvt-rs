# RE-247 — Base quantities a body gives

**Date:** 2026-10-26
**Issues:** #1016 (quantities), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-131 (pipe cylinders), RE-52 and RE-92 (stair run side views), RE-202 (column joins), RE-221 and RE-103 (column sections)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit's IFC4 exports), puzzbobb's R2025 models (Revit 2025), Core Interior and RE1 (whose exports have no base quantities, RE-242), as Measure fetches them
**Runs:** Measure [40327328254](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40327328254), [40329358476](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40329358476) and [40330336007](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40330336007) (`tools/re/qto_candidates_list.py`, by `BodySource` and with the objects a candidate misses), [40332337452](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40332337452) (`tools/re/qto_presence_list.py`), [40333304451](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40333304451), [40334578963](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40334578963) and [40337954619](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40337954619) (`examples/probe_re247_qto_records.rs` with `tools/re/record_scalars_vs_ifc.py`), [40331272217](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40331272217) and [40333470386](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40333470386) (the stair flights), and [40339772274](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40339772274) (this change against `main`, unfiltered, with `timing`)
**Result:** positive, narrowed: a stair flight's `Length` and `NetVolume`, a pipe segment's `GrossCrossSectionArea` and `OuterSurfaceArea` and, on a column joined to nothing, its `Length` are values of the body rvt-rs draws, equal on every one Revit's export gives; a column's `Length` elsewhere and a curtain panel's `Width` are left out, as Revit's export leaves them off objects no field tells apart.

## 1. Which value each quantity is

`qto_candidates_list.py` takes, for every object both files write, the depth, profile area, perimeter and extents, volume and surfaces of rvt-rs's body, and counts for each of Revit's base quantities the candidates equal to it (a relative 1e-5, or 1e-6 absolute in SI units); `--by-source` splits each count by the `BodySource` rvt-rs gives the object, and `--misses` lists what a nearly exact candidate misses. On the office:

| quantity | candidate | as built | at permit |
|---|---|---:|---:|
| `Qto_ColumnBaseQuantities.Length` | the extrusion's depth | 279 of 279 | 327 of 327 |
| `Qto_PlateBaseQuantities.Width` | the larger side of the profile | 257 of 257 | 266 of 266 |
| `Qto_PipeSegmentBaseQuantities.GrossCrossSectionArea` | the profile's area | 12 of 12 | 12 of 12 |
| `Qto_PipeSegmentBaseQuantities.OuterSurfaceArea` | the profile's perimeter times the depth | 12 of 12 | 12 of 12 |
| `Qto_StairFlightBaseQuantities.NetVolume` | the side view's area times the run's width | 34 of 35 | 34 of 35 |

The 35th flight is the spiral one (#357), drawn as its treads. A column's other quantities (`CrossSectionArea`, `OuterSurfaceArea`, `GrossVolume`, `NetVolume`) are its box's on 199 of the 200 drawn as their record box as built (the 200th, 3053209, is a round column rvt-rs draws as a box) and 269 of 290 at permit, a panel's `NetVolume` on 257 of 260 and a pipe's `NetCrossSectionArea` is its whole circle's on all 12, the same as its gross area, so it cannot exclude the bore: none of these is written.

## 2. A stair flight's Length

A flight's `Length` is not its side view's extent along the run. On R2025 13 (Rooms, Stairs), whose run ends with a riser, the extent is 2.25 m and Revit's `Length` 2.24 m; on R2025 14, the same stair ending with a tread, both are 2.24 m. It is the distance along the climb from the run's first riser line to its last, the sketch's `risers` that RE-92 reads, with or without a riser at the top: equal on the office's 34 flights drawn from their sketch and on R2025 13, 14, 15 (riserless, drawn as its treads) and 16 (monolithic). The `NetVolume` of a riserless run is the sum of its treads' side views times the width.

## 3. Where Revit's export writes a column's Length and a panel's Width

`qto_presence_list.py` tallies whether Revit writes the quantity against the kind of item its body is drawn with:

| model | columns with `Length` | without | curtain panels with `Width` | without |
|---|---:|---:|---:|---:|
| office as built | 279 of 279 | 0 | 257 of 260 | 3 |
| office at permit | 327 of 345 | 18, all drawn as an `IfcPolygonalFaceSet` | 266 of 269 | 3 |

Every column Revit draws as an extrusion has its `Length`; the 18 it draws as a cut mesh have `CrossSectionArea` and `NetVolume` but no `Length`, and their `NetVolume` is below their box's (1929237: 0.6236 m³ against 0.628 m³; 1343915: 0.5185 m³ against 0.536 m³). The three panels (2495452, 2500446 and 3002843, all `System Panel:Lasitettu`, in both files) are extrusions with a `NetVolume` 0.5% below their box's and no `Width`.

## 4. What tells them apart

`probe_re247_qto_records.rs` prints every scalar of each column's and panel's native record, the object classes of the record its `m_hostId` names, and for each pair step (`CutPairGStep`, `CleanPairGStep`) the kind of element its `m_otherId` names; `record_scalars_vs_ifc.py` now ranks the fields against whether Revit writes each quantity, among the elements of its entity (`present:`):

- No field predicts it: the best is right on 329 of the 345 columns at permit and 265 of the 269 panels.
- The meshed columns' pair steps name the same kinds of element, in the same numbers, as many extruded ones': slabs, walls and coverings.
- Every slab joined to any column is one rvt-rs finds level (its `PitchAngle` 0, RE-238), so a sloped slab does not tell the meshes either.
- What does hold: a column whose record holds no `CleanPairGStep` is drawn as an extrusion on every one, 92 as built and 70 at permit; each of the 18 meshes holds one or more. Every column holds at least one `CutPairGStep`, so that alone says nothing.

## 5. What rvt-rs does

- `native_column_joins::joins` reads, in the one pass RE-202 makes, every column's join order and whether any copy of its record holds a `CleanPairGStep`; `partition_schema_mvp` marks the columns with none (`COLUMN_UNJOINED_FIELD`), and only those carry `Qto_ColumnBaseQuantities.Length`, the depth of the extrusion they are drawn as.
- A stair run drawn from its sketch (RE-52) carries `Qto_StairFlightBaseQuantities.Length`, its riser lines' span (`STAIR_RUN_LENGTH_FIELD`), and `NetVolume`, its side views' area times its width.
- A pipe whose cylinder is read (RE-131) carries `GrossCrossSectionArea`, its circle's area, and `OuterSurfaceArea`, its circumference times its length, beside its `Length`.
- A curtain panel's `Width` is not written.

The maintainer chose this rule over writing every column's `Length` and every panel's `Width` (about 1,130 values equal, 24 that Revit's export does not write and which are probably a little off, since those bodies are smaller than rvt-rs's boxes): nothing is written that Revit's export leaves out (2026-10-26).

## 6. Measured

This change against `main` (run 40339772274: `-f nordic=true -f r2025=true -f timing=true`, every scorer):

| model | quantities equal | different | missing | Revit does not write |
|---|---:|---:|---:|---:|
| office as built | 19,972 → 20,157 | 1 → 1 | 20,222 → 20,037 | 0 → 0 |
| office at permit | 19,902 → 20,065 | 1 → 1 | 20,511 → 20,348 | 0 → 0 |

Per quantity on the office as built (at permit): column `Length` 93 (71) equal, 186 (256) missing; stair flight `Length` and `NetVolume` 34 (34) each, the spiral flight missing; pipe `GrossCrossSectionArea` and `OuterSurfaceArea` 12 (12) each; none different. R2025 13, 14, 15 and 16 each gain their flight's `Length` and `NetVolume`, equal. Core Interior's, RE1 Mechanical's and RE1 Plumbing's exports have no base quantities (RE-242): rvt-rs's 107 column lengths and 6 and 63 pipes' two areas are counted apart (952, 12 and 126 in all). The property census, IfcOpenShell's validation and every other scorer are as on `main`; export times are unchanged (0.99 to 1.01). Core Interior's witness observations move by 107 `IFCELEMENTQUANTITY`, `IFCQUANTITYLENGTH` and `IFCRELDEFINESBYPROPERTIES` each, and both verdicts replay PASS unchanged.

## 7. What stays missing

- Columns joined by a `CleanPairGStep`: 186 lengths as built and 256 at permit, of which Revit draws 18 as a mesh and the rest as an extrusion, with nothing yet telling them apart.
- Curtain panels' `Width`: 257 and 266 equal values, left out for the 3 panels Revit's export gives none.
- Columns' `CrossSectionArea`, `OuterSurfaceArea` and volumes, panels' and pipes' remaining quantities (section 1), and the spiral flight's quantities. Section 8 writes three of them on the hollow-section columns.

## 8. Hollow-section columns (added 2026-10-27)

revit-ifc's `PropertyUtil.CreateBeamColumnBaseQuantities` (CLEANROOM.md source 5) takes a column's `CrossSectionArea` as its extrusion's profile area, its `OuterSurfaceArea` as its length times the profile's outer and inner perimeters, its `GrossVolume` as the area times the length, and its `NetVolume` as the volume Revit computes for the element. Every one of the office's columns joined to nothing (93 as built) is drawn either as its record's box (14) or as its type's hollow section (79), the section rvt-rs reads from the family's parameters, rounded corners and all (RE-221). On the box, the profile's area is Revit's on every column but 3053209, a round column of 150 mm that rvt-rs draws as a 150 mm box: nothing read yet says a box-drawn column is round. On the hollow sections, the area and the lengths of the section's outline and hole, its arcs counted as arcs, give Revit's `CrossSectionArea`, `OuterSurfaceArea` and `GrossVolume` on all 79 as built and all 55 at permit, while its `NetVolume` is less than area times length on every one of them (by 1.7 to 3.4% on the 40 the census lists).

So a column joined to nothing and drawn as its type's hollow section carries `CrossSectionArea`, `OuterSurfaceArea` and `GrossVolume`; its `NetVolume`, and all four on a column drawn as its box, are left out.

| model | quantities equal | different |
|---|---:|---:|
| office as built | 20,556 → 20,793 (79 × 3) | 1 → 1 |
| office at permit | 20,543 → 20,708 (55 × 3) | 1 → 1 |

(Measure [40450791034](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40450791034), this change against `main` before #1052, unfiltered.) Only the two office exports move: Core Interior's, RE1's and every R2025 export are as on `main`, so no witness observation or fixture changes.

## 9. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=08df58d42a4a00db195f46fdc9f535074bf2b546 -f base=main \
  -f nordic=true -f r2025=true
gh workflow run measure.yml --ref main -f ref=08df58d42a4a00db195f46fdc9f535074bf2b546 -f base=none \
  -f nordic=true -f models="nordic-office-*" -f scorers=none \
  -f probe=probe_re247_qto_records -f probe_scorer=record_scalars_vs_ifc
gh workflow run measure.yml --ref main -f ref=3867b8948d43c161ee496fb2211d0572bdce3cc3 -f base=main -f nordic=true -f r2025=true
```
