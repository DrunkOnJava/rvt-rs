# RE-416: a column's box quantities hold only where its saved mesh is the box, so a round column of a type whose one length is `b` keeps its `Length` and not a square's areas and volumes

**Date:** 2026-11-10
**Issues:** #1203 (W7: the round column and the meshed columns' values, and the section check on box-drawn columns), #1016 (quantities); follows RE-291, RE-353 and RE-411 (the finding)
**Artefacts:** Autodesk's `rac_advanced` sample of Revit 2024, 2025, 2026 and 2027 (24 round columns, 6 in each save; no Revit export of those saves), and the 126 models of Measure, Revit's exports of the BIM4LCA office and housing, Core Interior, RE1, the Revit 2025 models, the held-out models and the BIMcollab and `racadv` IFC2X3 pairs among them. Never stored.
**Runs:** Measure [42930883818](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42930883818) (unfiltered, head `e8ef83ee1e9d435057c147b20d459d59ae3be396` against `main` at `fa2864d4f6a2730eea21c37f54a88340a5c74853`, kept under `refs/archive/2026-11-10/cited/e8ef83ee1e9d`) for section 3, [42935160557](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42935160557) (the four saves, both sides' exports kept, head `547af92eacf7be3385e834635cd319c1a334d0e0`) for section 4, and [42935008002](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42935008002) (the probe of RE-411 on the repaired code, `75ca5236ee2fcccf97283e7c21b658c0adfcabc0`, kept under `refs/archive/2026-11-10/cited/75ca5236ee2f`) for section 5
**Status:** Positive. The 24 round columns of `rac_advanced` lose exactly the four quantities that were a square's and keep their `Length`, in each of the four saves, and nothing else in the 126 models moves: 122 exports are byte-identical, every model with a Revit export among them, and the other 4 are those saves. The 24 are repaired by geometry, not scored, since Measure holds no Revit export of them.

## 1. What RE-411 found

24 round columns of the `rac_advanced` sample (450 mm across, ElementIds 201277 to 201282 in each save of Revit 2024 to 2027) are written with the square's quantities: `CrossSectionArea` 2.1797 ft², `OuterSurfaceArea` 35.433 ft², `GrossVolume` and `NetVolume` 13.078 ft³, against the cylinder's 1.7119, 27.829 and 10.272, 4/pi of them each. The pass that writes the box's four quantities on a column (`column_quantities.rs`, RE-291) takes a type whose length parameters are exactly `b`, or `b` and `h`, for a plain rectangle and tells a round column from it by its type holding a `Diameter`, which is how the BIM4LCA office's one round column is; these 24 are round and their type holds `b` alone (RE-411 section 5).

## 2. The rule

`saved_meshes::drop_column_box_quantities`, in the pass that draws a body from the mesh Revit saved (RE-416): for each column drawn from its mesh that holds a `Qto_ColumnBaseQuantities` with a `NetVolume`, or a `GrossVolume` where it holds no other, the held volume is compared with the mesh's. Where they differ by more than 2% of the mesh's, the box is not the column's solid, and its `CrossSectionArea`, `OuterSurfaceArea`, `GrossVolume` and `NetVolume` are taken out. Its `Length` stays: it is the box's height, which a cylinder has too (6.00000 ft on the 24, as the box and as the mesh).

- **The mesh's volume** is `oriented_mesh_volume`: each triangle turned outward by the normals of its vertices, measured from the instance's origin. It needs neither a closed mesh nor flat faces, so it reads the chord-drawn side of a cylinder to the accuracy of its chords; `closed_mesh_volume` (RE-387) needs both and cannot.
- **The 2%** sits between two kinds of column. A cylinder drawn with chords is under its volume by what its chords leave: 0.13% for the 88-point caps of these 24 (cap area 0.99867 of pi r squared), and the square that bounds it is 27% over it. A column whose box is its solid agrees with its mesh to single precision, which RE-387 measured at 1e-4 on mullions.
- **What it leaves alone.** A column that holds neither volume (the office's round column, whose `Length` and `OuterSurfaceArea` are RE-353's), a column with no saved mesh read (Revit 2015 to 2022) and a column drawn from a mesh the pass does not take.

## 3. Measured: every model

Unfiltered Measure 42930883818, every flag, 126 models on each side: 122 exports are byte-identical and 4 move, `rac_advanced` 2024, 2025, 2026 and 2027, the only files that hold the 24. Every model with a Revit export is byte-identical: the BIM4LCA office in its six states, whose census reads `Qto_ColumnBaseQuantities` equal on 278 of 279 `CrossSectionArea` and `GrossVolume` and 199 `NetVolume` values as built, 337 of 345, 320 of 327 and 289 of 345 at permit, the 18 columns Revit draws as cut meshes and its round column among them; the housing, whose 7 columns hold `Length` 5 equal and `NetVolume` 3; Core Interior's 107 `Length` values; the held-out and IFC2X3 models; and the 24 Revit 2025 models. The rule therefore takes nothing from a column whose box is Revit's solid.

## 4. Measured: the four saves, element by element

Measure 42935160557 ran the head (`547af92e`, the code of the run above with the repair's id renamed in its comments) and its parent on `main` (`fa2864d4`) with both sides' exports kept, on the 58 Autodesk samples (`samples=true`; the `models` filter does not narrow it): 54 exports are byte-identical and 4 move, the `rac_advanced` saves of 2024 to 2027. `tools/re/ifc_element_diff.py` pairs every `IfcElement` of the two exports of a save by GlobalId and compares its class, `Tag`, `Name` and every property and quantity set:

| save | elements | only in one file | elements that differ | what differs | entity count, head minus base |
|---|---:|---:|---:|---|---|
| `rac_advanced` 2024 | 5,674 | 0 | 6 (`Tag` 201277 to 201282) | `Qto_ColumnBaseQuantities` of each loses `CrossSectionArea`, `GrossVolume`, `NetVolume` and `OuterSurfaceArea`; nothing is added or changed | `IfcQuantityArea` -12, `IfcQuantityVolume` -12 |
| `rac_advanced` 2025 | 5,674 | 0 | 6 (the same `Tag`s) | the same | the same |
| `rac_advanced` 2026 | 5,674 | 0 | 6 (the same `Tag`s) | the same | the same |
| `rac_advanced` 2027 | 5,674 | 0 | 6 (the same `Tag`s) | the same | the same |

So the rule removes 24 columns' four quantities and touches no other element, class or value in any of the four files.

## 5. Measured: the probe of RE-411 on the repaired code

Measure 42935008002 ran `probe_1203_round_column_rule` (RE-411) at `75ca5236ee2fcccf97283e7c21b658c0adfcabc0`, the repair's code with the probe added, on the 58 Autodesk samples (the `models` filter does not narrow `samples`). The 24 columns, 6 in each of `rac_advanced` 2024, 2025, 2026 and 2027, radius 0.73819 ft and height 6.00000 ft, write `Qto_ColumnBaseQuantities` with `Length` alone; RE-411 section 5 read `Length`, `CrossSectionArea`, `OuterSurfaceArea`, `GrossVolume` and `NetVolume` on the same columns. The other round columns of the same files (radius 0.49213 ft, 164 in each save, and the rme samples' 1,048) write `Length` alone, as RE-411 read them, and the Revit 2023 save writes none, as before.

## 6. Not shown

- **The circle's quantities.** The 24 keep a `Length` and write nothing else. The cylinder's `OuterSurfaceArea` is its circumference times its height, exact on the office's round column (RE-353 section 4); it is not written here, because no Revit export of these saves scores it and RE-353 measured it on one column of one project. Its `CrossSectionArea`, `GrossVolume` and `NetVolume` are not the circle's on that column either, which RE-353 left out for the same reason.
- **A Revit value for the 24.** Measure holds no Revit export of the 2024 to 2027 saves (its one export of the sample is the three.js IFC2X3 export of the 2015 to 2020 state), so the 24 are repaired by geometry, not scored.
- **How often the comparison ran.** The number of columns the 2% test reached, on each model, is not counted; section 3 shows that none it reached on a model with a Revit export changed.
- **Another project's round column of a type with `b` alone.** The 24 are one family type in one sample, in four saves.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref fix/1203-round-column-box-quantities -f ref=e8ef83ee1e9d435057c147b20d459d59ae3be396 -f base=fa2864d4f6a2730eea21c37f54a88340a5c74853 \
  -f nordic=true -f samples=true -f r2025=true -f holdout=true -f bimcollab=true -f racadv=true
```

The element diff of section 4:

```bash
gh workflow run measure.yml --ref fix/1203-round-column-box-quantities -f ref=547af92eacf7be3385e834635cd319c1a334d0e0 -f base=fa2864d4f6a2730eea21c37f54a88340a5c74853 \
  -f samples=true -f keep_ifc=true -f scorers=none
python3 tools/re/ifc_element_diff.py <base>/sample-2024-racadvancedsampleproject/model.ifc <head>/sample-2024-racadvancedsampleproject/model.ifc
```

The probe of section 5 is `probe_1203_round_column_rule` (RE-411) at `75ca5236ee2fcccf97283e7c21b658c0adfcabc0`, kept under `refs/archive/2026-11-10/cited/75ca5236ee2f`; the head of section 4 under `refs/archive/2026-11-10/cited/547af92eacf7`.

## 8. Sources

RE-291, RE-353, RE-387, RE-411. revit-ifc's `PropertyUtil.CreateBeamColumnBaseQuantities` as RE-353 read it (CLEANROOM.md source 5). No Autodesk source was read.
