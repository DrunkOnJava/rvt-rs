# RE-425: a column drawn as its type's rectangle gets the base quantities its saved mesh gives, and keeps the rectangle

**Date:** 2026-11-11
**Issues:** #1705 (the housing's record-box columns), #1203 (columns), #1016 (W7's quantities), #1636 (second-project measurement); follows RE-424 (a column drawn from a saved mesh gets the four from its prism), RE-423 and RE-388 (a beam's and a wall's quantities from the mesh whether or not the body is drawn from it) and RE-291 (the box rule)
**Artefacts:** the BIM4LCA housing's six files and the office's six files (Revit 2024, Revit's IFC4 exports, CC BY-SA 4.0), the held-out pairs of Revit 2022 to 2027 (counts only), Autodesk's samples and Core Interior, as Measure fetches them. Never stored.
**Runs:** Measure [42997969828](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42997969828) (the two RE-291 probes on the housing as built) and [42999069460](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42999069460) (rvt-rs's exports of the two kept) for section 1; [42999974783](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42999974783) (`tools/re/column_quantities_vs_ifc.py` on both sides, 42 models) and [43000953082](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43000953082) (unfiltered, every flag, `timing`; head `c596c666075ffa51977b3452f4c7c639a70b69ee`, kept under `refs/archive/2026-11-11/cited/c596c666075f`, against `main` at `acee45d8e7f2154be5c4c4025e0826e172c341e7`, 126 models each side) for sections 3 and 6; [43000956414](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43000956414) (both sides' exports of the 58 samples and the reference models kept) for section 4; [43005731573](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43005731573) (the reference models timed again) for section 6
**Status:** Positive on the BIM4LCA housing, where 526 values on 6 files become Revit's and none differs, the four chamfered columns among them; the offices are byte-identical. 15 of 126 exports move (the housing's six, Core Interior and the eight MEP samples of 2024 to 2027) and no body changes. The cost is a pass that reads more columns' graphics: 37 MiB on the timber housing, 83 to 99 MiB (1.18 to 1.23 times) and 3% of time on Core Interior, none the runs can see elsewhere.

## 1. What kept these columns out

The BIM4LCA timber housing draws 50 of its 54 columns, and the concrete housing 4 of its 7 (and 4 of the 5 in each terrain file), as a box (`BodySource` `partition_element_record_bbox`), and Revit's export writes all four of `Length`, `CrossSectionArea`, `OuterSurfaceArea` and `GrossVolume` on every one of them. The shipped census had rvt-rs equal to Revit's `CrossSectionArea`, `OuterSurfaceArea` and `GrossVolume` on 2 of the timber file's 54 columns (the two drawn from meshes) and `Length` on 25. The scorer (RE-424) reads rvt-rs's own box as a prism and finds its values are Revit's on 46 of the 50 and not on 4. The reason none was written is that neither pass that writes a column's quantities took them:

- **RE-291's box rule** (`export_content/column_quantities.rs`) takes a column whose body has no profile override. These carry one: `ProfileResolved` is true and `ProfileSource` is `family_type_symbol_envelope` with the type's `TypeSectionWidth` and `TypeSectionDepth`, so their body is already the type's rectangle over the record's height. The probes (Measure 42997969828) show the types name their section `Width` and `Depth` (Puupilari PI-3 to PI-6, Concrete column PI-1 and PI-2, Teräspilari PI-7), not the `b` and `h` of the office's families, so `rectangular_section` reads none of them either.
- **The saved-mesh pass** offers a box's mesh only where no profile is resolved (`is_replaceable_box`: a resolved outline is the body of any other class), so it never read these columns' graphics. None holds a `SavedMeshRefusal`.

The 4 columns whose rectangle is not Revit's solid are the `Concrete column` family (PI-1: 1 column; PI-2: 3 of its 4 box columns): Revit's `CrossSectionArea` is 0.03955 and 0.13955 m² where the type's rectangle gives 0.04 and 0.14 (1.1% and 0.3% over), a chamfered section whose type holds `Width` and `Depth` and nothing else. The one PI-2 column that is drawn from its saved mesh reads Revit's 0.03955 (RE-424).

## 2. The rule

A column whose body is the record box with a resolved profile (`column_rectangles` in `saved_meshes::attach_scoped_with_control`) has its saved mesh read for its quantities only, as a beam's (RE-423) and a wall's (RE-388) are: `column_mesh_quantities` takes the mesh where every face is a plane and `mesh_volume::column_quantities` (RE-424) accepts it, one closed solid with no hole through it that is a prism along the vertical with two flat, square ends of one area. The body is not touched: it stays the type's rectangle, which is Revit's own form where the section is one, so no `BodySource` and no body scorer changes.

`add_column_quantities` writes `CrossSectionArea`, `OuterSurfaceArea`, `GrossVolume` and `NetVolume` (Revit's `NetVolume` of an extruded column is its `GrossVolume`) and `Length` to `Qto_ColumnBaseQuantities`, creating the set where the column holds none, as for a beam: where the set holds none of the first three it gets all of them, where it holds all three the `Length` and `NetVolume` it lacks, and a set that holds one or two (the cut-column rule's) is left alone. No value a column holds is replaced. The pass reads the graphics of these columns beside the rest, so it adds their ids to what it selects; RE-424's rule added none.

## 3. Measured: every model

Unfiltered Measure [43000953082](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43000953082), every flag, 126 models on each side, head `c596c666` against `main` at `acee45d8`: 111 exports are byte-identical and 15 move, the housing's six files, Core Interior and eight Autodesk samples (section 4). The office's six files are byte-identical: their columns are drawn from meshes (RE-424), are hollow-section families, or are the one round box. `same-project gates` passed on the head and the `body_sources` of the 101 models that report them are identical on both sides.

`qtos_vs_ifc`, `Qto_ColumnBaseQuantities`, equal to Revit's (before to after, of the columns Revit writes the quantity on):

| file | `CrossSectionArea`, `OuterSurfaceArea`, `GrossVolume` | `NetVolume` | `Length` |
|---|---:|---:|---:|
| timber housing, each state | 2 to 52 of 54 | 4 to 54 | 25 to 54 |
| concrete housing, each state | 3 to 7 of 7 | 3 to 7 | 5 to 7 |
| each terrain file of the housing | 1 to 5 of 5 | 1 to 5 | 5 of 5, unchanged |

That is 526 values on the housing's six files (229 in each timber state, 18 in each concrete one, 16 in each terrain file), none different. Every line is as the scorer predicted: the 50 box columns of the timber file, the 4 of the concrete file and the 4 of each terrain file read Revit's, and the 4 whose rectangle was not Revit's solid read Revit's chamfered section too (the scorer's `rule different` for them became `equal`). The 2 columns of the timber file still without are the two hollow sections drawn from their meshes (RE-424), which Euler's characteristic refuses. `different` and "values Revit does not write" are unchanged on every model with a Revit export; the office's exports do not move.

## 4. Core Interior and the Autodesk samples

**Core Interior** (Revit 2024; its Revit export was made with base quantities off, RE-242, so the census counts what rvt-rs writes there apart): 107 of its 256 columns gain `CrossSectionArea`, `OuterSurfaceArea`, `GrossVolume` and `NetVolume` (their `Length` was there), and the quantities rvt-rs writes that Revit's export does not hold go from 3,147 to 3,575. Its two witness observations (`research/witness/magnetar-2024-core-interior*/observations/rvt-rs.json`) change in `IFCQUANTITYAREA` (885 to 1,099) and `IFCQUANTITYVOLUME` (893 to 1,107), refreshed from Measure's files and replayed to `match` on all three observations of both artifacts. No project-count fixture pins a quantity entity.

**The eight MEP samples** (`rme_advanced` and `rme_basic` of 2024 to 2027): no Revit export of them exists, so each is read. Run [43000956414](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43000956414) kept both sides' exports of the 58 samples and the reference models: 9 of 64 move, Core Interior and these eight. `tools/re/ifc_element_diff.py` pairs every element by GlobalId and finds that none is added or removed and that only `IfcColumn` changes, in each sample on 4 columns (32 in all) by `Qto_ColumnBaseQuantities` gaining the three areas and the volume, and in Core Interior on the 107, nothing removed or changed. The 4 columns are `Strut:1.5" x 1.5"`: the type's rectangle is 1.5 inches square (CrossSectionArea 0.00145 m² read from the body), and the mesh is a thin-walled section of 0.000176 m² and 0.2254 m of perimeter, so the quantities the pass writes are the mesh's and not the body's. `tools/re/column_added_quantities.py` reads Core Interior's 107 as the rule's value of the exported body (107 of 107) and the 32 struts as not the body's, which is what keeping the body means. No Revit export scores either.

## 5. What it leaves out

- **The bodies.** A column keeps the type's rectangle even where the section is not one: the housing's 4 chamfered columns (their volume 0.3% to 1.1% over Revit's) and the samples' 32 struts (a thin section drawn as a solid 1.5 inch bar). Their meshes are read and are the better body; replacing them changes `body_sources` that the same-project gates count, so this change reads them for quantities only, as RE-423 did for beams. The residual is filed.
- **Columns whose mesh is not a flat closed prism along the vertical** get none: the timber housing's 2 hollow sections, the offices' hollow-section families and round column, Core Interior's other 149 columns.
- **Columns of another body** (a record box with no resolved profile, a trimmed or cut box, a section family) are not read; RE-424 reads the ones drawn from a mesh.
- **The 2 timber-office `Length` differences** stay: no value a column holds is replaced.

## 6. What it costs

The quantities come from graphics the saved-mesh pass did not read before: the pass selects the ids of these columns beside the others. Measure's timing (base and head built once and timed in turn on one runner per file, three rounds; peak memory is GNU time's `%M`), run 43000953082, 74 files with `main`'s workflow (which times the BIM4LCA housing and timber office since #1703):

- **Time** 0.98 to 1.04 (median 0.996). The eight MEP samples that move read 0.98 to 1.00 (pooled 0.991) and the 30 samples of 2015 to 2022, which the pass cannot read, 0.98 to 1.01. The ten BIM4LCA files read 0.98 to 1.00 but for the office's 0.8 second terrain concrete file (1.04, 29 ms); the housing's four read 1.00.
- **Peak memory** median 1.00; the files that read more: the two timber housing files 1,002 to 1,039 MiB and 993 to 1,030 MiB (1.04, 37 MiB, 50 more columns' graphics each), Core Interior 437 to 536 MiB (1.23) and, in run 43005731573 that timed the reference models again, 453 to 536 MiB (1.18) with the time at 1.03 both times (4,999 to 5,125 ms and 5,322 to 5,486 ms), and `rac_advanced` 2023 571 to 586 MiB (1.03, a file this change does not move). No other file's peak reads outside 0.98 to 1.02 but `rac_basic` 2027's, lower, 392 to 377 MiB (0.96).
- Core Interior's baseline wanders between 437 and 453 MiB from run to run, which does not account for the 83 to 99 MiB; why the extra columns cost that is not established (no profile by stage has been taken). The pass's peak is the decode of the symbols it reads (#1694), and these columns' symbols are among them.

**Opting out.** `--no-saved-meshes` runs no pass, so it leaves out these quantities and this cost with the rest.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=c596c666075ffa51977b3452f4c7c639a70b69ee -f base=acee45d8e7f2154be5c4c4025e0826e172c341e7 \
  -f nordic=true -f samples=true -f r2025=true -f holdout=true -f bimcollab=true -f racadv=true -f timing=true
gh workflow run measure.yml --ref main -f ref=c596c666075ffa51977b3452f4c7c639a70b69ee -f base=acee45d8e7f2154be5c4c4025e0826e172c341e7 -f samples=true -f keep_ifc=true -f scorers=none
gh workflow run measure.yml --ref main -f ref=c596c666075ffa51977b3452f4c7c639a70b69ee -f base=acee45d8e7f2154be5c4c4025e0826e172c341e7 -f nordic=true -f r2025=true -f scorers=column_quantities_vs_ifc
gh workflow run measure.yml --ref main -f ref=<sha> -f base=none -f nordic=true -f models="nordic-housing-timber-as-built nordic-housing-concrete-as-built" -f scorers=none \
  -f probe="probe_re291_column_type_parameters probe_re291_column_pairs" -f probe_scorer="- -"
```

The scorers: `python3 tools/re/column_quantities_vs_ifc.py <rvt-rs.ifc> <revit-export.ifc>`, `python3 tools/re/column_added_quantities.py <before.ifc> <after.ifc>` and `python3 tools/re/ifc_element_diff.py <before.ifc> <after.ifc>` for the exports a run keeps.

## 8. Sources

RE-353 (revit-ifc's `PropertyUtil.CreateBeamColumnBaseQuantities`, CLEANROOM.md source 5), RE-291, RE-388, RE-423, RE-424. No Autodesk source was read.
