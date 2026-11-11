# RE-424: a column drawn from a saved mesh gets the four base quantities Revit's extrusion gives it

**Date:** 2026-11-11
**Issues:** #1203 (the meshed columns' values), #1016 (W7's quantities), #1636 (second-project measurement); follows RE-423 (a beam's quantities from the prism its mesh is), RE-291 and RE-353 (a column's quantities from its record box, and the columns a slab cuts), RE-387 and RE-416 (the volume of a column's mesh)
**Artefacts:** the BIM4LCA housing's six files and the office's six files (Revit 2024, Revit's IFC4 exports), puzzbobb's Revit 2025 models, the held-out pairs of Revit 2022 to 2027 (counts only) and Autodesk's samples, as Measure fetches them. Never stored.
**Runs:** Measure [42980794133](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42980794133) (unfiltered, every flag, `timing`; head `cc0e59a2d8968115b8e520d52bd1c0d5d4549cc9`, kept under `refs/archive/2026-11-11/cited/cc0e59a2d896`, against `main` at `164112820df30eece4e6d3df243d59c9a14d83ee`) for sections 3 and 6; [42980805331](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42980805331) (both sides' exports of the 58 Autodesk samples kept) for section 4; [42986667074](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42986667074) (`tools/re/column_quantities_vs_ifc.py` on both sides, 42 models) for sections 1, 3 and 5, with [42979117463](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42979117463) and [42979831896](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42979831896), the scorer's first runs, for the rule's design; [42980801971](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42980801971), [42986137067](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42986137067), [42987678966](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42987678966) and [42987682914](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42987682914) (the head merged with `main` at `15593a3c`, which holds #1701, against that `main`) for section 6
**Status:** Positive on the BIM4LCA timber office, the concrete office at permit, the housing and one held-out model: 580 columns of ten files gain 2,022 quantities, every one equal to Revit's and none different; 17 of 126 exports move and the other 109 are byte-identical, and the 7 Autodesk samples that move gain the values on 44 columns, each the rule's value of its own mesh, which no Revit export scores. The 18 cut columns, the round and hollow columns, meshes of several pieces and the housing's record-box columns get none. The cost is not distinguishable from the runs' noise.

## 1. What Revit writes

`tools/re/column_quantities_vs_ifc.py` pairs every `IfcColumn` of rvt-rs's export with Revit's by `Tag`, says which set of `Qto_ColumnBaseQuantities` Revit wrote and the class of its body, and reads rvt-rs's own body as RE-423 reads a beam's: a prism along its longest axis with two flat ends of one area. Run on the BIM4LCA files before this change (Measure 42986667074, base side, `main` at `16411282`):

| file | columns | Revit's set | rvt-rs's body |
|---|---:|---|---|
| concrete housing, each state | 7 | the four, 7 | saved mesh 3, record box 4 |
| each of the housing's two terrain files | 5 | the four, 5 | saved mesh 1, record box 4 |
| timber housing, each state | 54 | the four, 54 | saved mesh 4 (two with hollow sections), record box 50 |
| timber office, as built | 344 | the four 339, `CrossSectionArea` alone 5 | saved mesh 343, record box 1 |
| timber office, permit | 340 | the four 335, `CrossSectionArea` alone 5 | saved mesh 339, record box 1 |
| concrete office, as built | 279 | the four 279 | saved mesh 199, hollow section 79, record box 1 |
| concrete office, permit | 345 | the four 327, `CrossSectionArea` alone 18 | saved mesh 289, hollow section 55, record box 1 |

The office's two terrain files hold no column.

- **Revit writes two sets on a column.** `Length`, `CrossSectionArea`, `OuterSurfaceArea` and `GrossVolume` (with `NetVolume`) on a column it draws as an extrusion, and `CrossSectionArea` alone (with `NetVolume`) on one it draws as a cut mesh: the 5 of each timber office file and the concrete office's 18 at permit, each cut where a slab meets its top (RE-291, RE-353), where the area is half the surface of the cut mesh. revit-ifc's `CreateBeamColumnBaseQuantities` takes the four from the extrusion it writes, and where it has none the area from the element's computed area parameter (RE-353, CLEANROOM.md source 5).
- **Each of the four is a sum of the column's own mesh** on every column drawn from a saved mesh whose Revit set is the four: `CrossSectionArea` is one end's area, `OuterSurfaceArea` the lateral area, `GrossVolume` the volume, `Length` the extent along the axis. The scorer finds no `rule different` on any of them in any file.
- **The axis is the vertical.** The rule takes the longest axis the mesh is a prism along with two flat ends; on every column whose Revit set is the four, in all ten BIM4LCA files that hold columns, that axis is the vertical. On the concrete office's 18 cut columns at permit the axis the rule takes is a horizontal one on 14, and the other 4 are no prism at all. So a vertical axis is what separates the two kinds of column, with no partner test: it refuses 18 of 18 cut columns and none of the extruded ones. Without it the rule would give the 14 a `CrossSectionArea` that is not Revit's (a different value on each) and `Length`, `OuterSurfaceArea` and `GrossVolume` where Revit writes none.

## 2. The rule

`mesh_volume::column_quantities` reads the saved mesh of a column the pass draws from it, as the beam rule does (`longest_flat_prism`, RE-423) and with one more condition: the axis it takes is the vertical (its cosine with the vertical at least 0.9999). All of these hold, or the column gets none:

1. **A whole, closed, single solid of flat faces with no hole through it.** The pass's `all_faces_flat` refuses a round column, and the Euler characteristic and the piece count refuse a hollow section.
2. **A prism along its longest flat-ended axis**, both ends perpendicular to it and of one area to 1e-4, the volume the end area times the extent to 1e-4, no tie between two axes.
3. **That axis is the vertical.** A column lying down, a stub wider than it is tall and a cube have another axis for their longest, or tie, and get none; so does a column cut across its top, whose longest flat-ended axis is a horizontal one or which is no prism.

Then `CrossSectionArea` is the end area, `OuterSurfaceArea` the lateral area, `GrossVolume` the volume and `Length` the extent. `saved_meshes::add_column_quantities` writes them to `Qto_ColumnBaseQuantities` where the column's set holds none of `CrossSectionArea`, `OuterSurfaceArea` and `GrossVolume` (all four), or all three (the `Length` it lacks). A set that holds one or two of the three is the cut-column rule's (`CrossSectionArea` and `NetVolume` for a column Revit draws as a cut mesh) and is left alone, and no value a column holds is replaced: the 2 columns of the timber office at permit whose `Length` rvt-rs already wrote, and which differs from Revit's, keep it. The set is there because `add_mesh_volumes` gave every column whose mesh is a closed solid of flat faces its `NetVolume`. The pass adds nothing to what it selects: it reads the meshes of the columns it already draws, and `NetVolume`, every body and every other quantity stay as they were.

## 3. Measured: every model

Unfiltered Measure [42980794133](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42980794133), every flag, 126 models on each side, head `cc0e59a2` against `main` at `16411282`: 109 exports are byte-identical and 17 move, the housing's six files, the office's three (the concrete office at permit and the timber office in both states), the held-out `modelo_bim` and 7 Autodesk samples (section 4). The concrete office as built and the two terrain files of the office are byte-identical. `same-project gates` passed on the head, and the `body_sources` of the 101 models that report them are identical on both sides.

`qtos_vs_ifc`, `Qto_ColumnBaseQuantities`, equal to Revit's (before to after, of the columns Revit writes the quantity on):

| file | `CrossSectionArea` | `OuterSurfaceArea` | `GrossVolume` | `Length` |
|---|---:|---:|---:|---:|
| timber office, as built | 60 to 340 of 344 | 59 to 339 of 339 | 58 to 338 of 339 | 203 to 339 of 339 |
| timber office, permit | 58 to 336 of 340 | 57 to 335 of 335 | 56 to 334 of 335 | 197 to 333 of 335 |
| concrete office, permit | 337 to 343 of 345 | 321 to 327 of 327 | 320 to 326 of 327 | 321 to 327 of 327 |
| concrete housing, each state | 0 to 3 of 7 | 0 to 3 of 7 | 0 to 3 of 7 | 5 of 7, unchanged |
| timber housing, each state | 0 to 2 of 54 | 0 to 2 of 54 | 0 to 2 of 54 | 25 of 54, unchanged |
| each terrain file of the housing | 0 to 1 of 5 | 0 to 1 of 5 | 0 to 1 of 5 | 5 of 5, unchanged |
| held-out `modelo_bim` | 0 to 4 of 8 | 0 to 4 of 4 | 0 to 4 of 4 | 0 to 4 of 4 |

That is 2,022 values on 580 columns of the 10 files with a Revit export that move, none different. Every line is as the scorer predicted: on the timber office at permit the scorer's table of what the rule gives where rvt-rs wrote none read `CrossSectionArea` 278 of the 282 columns that lacked it, `OuterSurfaceArea` 278 of 278, `GrossVolume` 278 of 279 and `Length` 136 of 136, all right, none different, and it reads 0 after (section 5 says what the rest are). `different` and "values Revit does not write" are unchanged on every model: the two `Length` differences of the timber office at permit are the same two as before, and the one model where the column scorer finds a value Revit does not write is Core Interior, whose Revit export holds no base quantity (107 `Length`, as before). `NetVolume` does not move on any file.

**Nothing is written on a cut column.** The concrete office at permit gains `CrossSectionArea`, `OuterSurfaceArea`, `GrossVolume` and `Length` on the same 6 columns, so none of the 18 cut columns gained a `CrossSectionArea`, and the 14 it would give `Length`, `OuterSurfaceArea` and `GrossVolume` Revit omits stay `writes none` (14 of 14 on each). The timber office's 5 cut columns, which no prism rule reads, keep what the cut-column rule wrote: 2 of the 5 `CrossSectionArea`.

**The held-out model.** The rule was designed on the BIM4LCA files and no held-out pair was read for it; the one held-out model that holds columns the rule reads is `modelo_bim` (a Revit 2023 project, whose Revit export is counted, not stored). It has 8 columns, 4 extruded and 4 cut. The 4 extruded read all four quantities equal to Revit's after the change and none before, and the 4 cut get none. Its `different` and "values Revit does not write" are unchanged, and no other held-out model moves. That is a check of the rule on 4 columns it was not designed on, and it holds on all 4.

## 4. The Autodesk samples

Run [42980805331](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42980805331) kept both sides' exports of the 58 Autodesk samples: 7 move, `rst_basic` of 2023 to 2027, Snowdon Towers Architectural and Snowdon Towers Structural of 2024. No Revit export of them exists, so each is read. `tools/re/ifc_element_diff.py` pairs every element by GlobalId and finds that no element is added or removed and that only `IfcColumn` changes, on each of them only by `Qto_ColumnBaseQuantities` gaining quantities and nothing removed or changed: 1 column in each `rst_basic` save, 26 in Snowdon Towers Architectural (13 gain all four, 13 gain the three of the section because they already hold a `Length`) and 13 in Snowdon Towers Structural, 44 in all.

**The arithmetic.** `tools/re/column_added_quantities.py` over the same 7 pairs reads every added value as the rule's value of that column's own mesh: of the 44 columns, `CrossSectionArea`, `OuterSurfaceArea` and `GrossVolume` were added on all 44 and `Length` on 31 (the other 13, in Snowdon Towers Architectural, already held one), each equal to the rule's value of the exported body, and the axis is the vertical on all 44. The two readings are independent (the pass reads the saved mesh in Rust, the scorer the exported faces in Python), so they agree on 44 of 44.

**What the scorer's reading would take and the pass does not.** The scorer cannot count the pieces of a mesh or see a curved face. A column drawn from a saved mesh that its rule accepts, whose set holds `NetVolume` (so the pass read a closed mesh of flat faces) and which was left without the four, would be a disagreement of the two: there is none in the 49 samples whose exports did not move and that hold columns, and 14 in the Snowdon Towers Architectural save, each a mesh of two or three closed pieces (Euler characteristic 4 or 6), which the pass refuses. Every other column the scorer accepts and the pass leaves is not a mesh the pass reads: a record box (2,727 columns of 31 unmoved samples hold none of the four, 139 of 9 hold one), a section family's extrusion, or a mesh with no `NetVolume`, which has a curved face or a hole.

## 5. What it leaves out

What Revit writes on a column and rvt-rs still does not, at the head (Measure 42986667074):

- **Cut columns.** Revit writes `CrossSectionArea` and `NetVolume` alone on the 18 of the concrete office at permit and the 5 of each timber office file. rvt-rs writes `CrossSectionArea` on 17 of the 18 and 2 of the 5 (the cut-column rule, RE-353) and `NetVolume` on all, as before this change, and writes no `Length`, `OuterSurfaceArea` or `GrossVolume`, which Revit omits.
- **One round column** in each office file (3053209, drawn as its record box): `Length` and `OuterSurfaceArea` are right, `CrossSectionArea` and `GrossVolume` are left out because its type holds a `Diameter` and no `b`, so the box rule leaves it out (#1203), and the box's would be the square's, not the circle's (0.0225 m² against Revit's 0.0176598); the circle's formulas would need checking against Revit's `GrossVolume` and `NetVolume`, which differ (0.108131 and 0.108179 m³).
- **Sections the pass finds no closed mesh of flat faces for.** Autodesk's `rst_basic` of 2023 draws 30 columns from a saved mesh; 29 hold no `NetVolume` and none of the four, so the pass found no closed mesh of flat faces for them (12 of the 29 have a hole through them). No Revit export scores them.
- **Hollow sections.** The concrete office's 79 columns as built and 55 at permit are drawn by their section family (`family_type_hollow_section`), and 2 of the timber housing's 4 saved-mesh columns have a hole through them: Euler's characteristic refuses them, and the mesh does not tell the outer perimeter from the inner.
- **Columns drawn as their record box.** The timber housing's 50 columns and 4 of the concrete housing's 7 (and 4 of 5 in each terrain file) get none of `CrossSectionArea`, `OuterSurfaceArea` and `GrossVolume` from this change. The box's values would be Revit's on 46 of the 50 and on 4 of 4 (the scorer's mirror of the rule on the box: `CrossSectionArea`, `OuterSurfaceArea`, `GrossVolume` and `NetVolume` alike) and are different on the other 4 of the timber housing; RE-291's rule writes none of them. Why is not established: [#1705](https://github.com/DrunkOnJava/rvt-rs/issues/1705).
- **The 2 `Length` differences** of the timber office at permit stay: no value a column holds is replaced, and the mesh's extent equals Revit's on both (#1705).

## 6. What it costs

The change reads no graphics the pass did not read: the quantities come from the meshes of columns the saved-mesh pass already draws, and the rule runs on each in the same walk. Measure's timing (base and head built once and timed in turn on one runner per file, three rounds; peak memory is GNU time's `%M`):

- **Run [42980794133](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42980794133), 64 files** (the reference models and the 58 samples; the office's four files were not fetched, the host was unreachable from that shard): time 0.98 to 1.04 (median 1.006), peak memory 1.00 to 1.04 (median 1.00). The 30 samples of 2015 to 2022, which the pass does not read and so cannot run the new code, read 0.98 to 1.02 in the same run (mean 1.004), against 1.007 for the 28 samples of 2023 to 2027 (0.98 to 1.04), and Core Interior's peak is 437 and 453 MiB in either order in this run and in RE-423's. The 7 samples that move read 0.98 to 1.03 in time and 1.00 in peak.
- **The BIM4LCA files, Measure [42980801971](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42980801971)** (the workflow of `scratch/timing-bim4lca-423`, which times the housing and the timber office as well): the housing's four large files 1.03, 0.99, 1.00 and 0.99 in time and 1.00 in peak memory; the office's four files 1.00, 1.01, 1.02 and 1.03, and a second time [42986137067](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42986137067) 1.00, 1.01, 1.00 and 1.01 (peak 1.00); the timber office 1.01 and 1.01 (peak 1.00).
- **Against `main` with #1701 (`15593a3c`), the head merged with it (`d93da0ed`)**, the base the change lands on: Measure [42987678966](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42987678966) times the reference models and the 58 samples, and [42987682914](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42987682914) the ten BIM4LCA files. The ten BIM4LCA files read 0.99 to 1.02 in time and 0.99 to 1.00 in peak memory on both sides (the housing at 955 to 1,003 MiB and the office at 1,301 to 1,813 MiB, where #1701 took them from 1,103 to 1,335 and 1,440 to 2,056). The 64 files read 0.94 to 1.10 in time (median 1.007) and 0.97 to 1.01 in peak memory; the 30 samples of 2015 to 2022, which cannot run the new code, read 0.94 to 1.06. The 7 samples that move read 0.99 to 1.10 in time and 0.97 to 1.00 in peak memory, pooled 1.022 in time against 1.006 for the 30 and 1.007 for the other 27 samples of 2023 to 2027, in this run and 1.021, 1.004 and 1.007 in the first: that pooled figure is Snowdon Towers Architectural's 28 s (1.03 in the first run, 1.02 in this) and `rst_basic` 2026's 1.1 s (0.99 in the first run, 1.10 in this), and whether any of it is this change's is not established: it is 0.65 to 0.8 s at most on the one large file, and what the change runs in it is a rule over the meshes of its 118 columns, whose median is 34 triangles.

The row registered for this change on #1636 (the files that read 0.99 to 1.01 before still do) is missed as worded: 17 of the 64 files read outside it in time, and the 30 that cannot run the code are among the spread. The registered row asked for less noise than a three-round median on shared runners gives.

**Opting out.** `--no-saved-meshes` runs no pass, so it leaves out these quantities and the meshes of the columns with them.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=cc0e59a2d8968115b8e520d52bd1c0d5d4549cc9 -f base=164112820df30eece4e6d3df243d59c9a14d83ee \
  -f nordic=true -f samples=true -f r2025=true -f holdout=true -f bimcollab=true -f racadv=true -f timing=true
gh workflow run measure.yml --ref main -f ref=cc0e59a2d8968115b8e520d52bd1c0d5d4549cc9 -f base=164112820df30eece4e6d3df243d59c9a14d83ee -f samples=true -f keep_ifc=true -f scorers=none
gh workflow run measure.yml --ref main -f ref=cc0e59a2d8968115b8e520d52bd1c0d5d4549cc9 -f base=164112820df30eece4e6d3df243d59c9a14d83ee \
  -f nordic=true -f r2025=true -f scorers=column_quantities_vs_ifc
```

The scorers: `python3 tools/re/column_quantities_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>` and `python3 tools/re/column_added_quantities.py <before.ifc> <after.ifc>` for the exports a run keeps (`python3 tools/re/ifc_element_diff.py <before.ifc> <after.ifc>` beside it).

## 8. Sources

RE-353 (revit-ifc's `PropertyUtil.CreateBeamColumnBaseQuantities`, CLEANROOM.md source 5), RE-291, RE-387, RE-416, RE-423. No Autodesk source was read.
