# RE-97 — A Sketch element lists its curves

**Date:** 2026-09-28
**Issues:** #31 (slab profiles, closed); follows RE-25, RE-50, RE-95 and RE-96
**Result:**
- A Sketch element's data names the element it sketches and lists its curves:

  ```text
  +77 from the element-data header   u64 sketched element's ElementId
  later                              u32 n · n × (u64 sketch-line ElementId · u32 key)
  ```

  The keys rise through the list. They run 0 to n − 1 on a sketch never edited, and leave gaps where curves were deleted.
- RE-25 took a sketch's lines from each line's own owner reference: the last slot of its record's second reference list. On most sketches the two agree. Where they do not, the owner reference names another element: one the line was drawn for before the sketch was edited, or one that is not exported at all. The list is what Revit draws.
- A floor's or roof's sketch now takes its lines from its Sketch's list where one names it. Where the list differs from the lines naming the element, the outline is kept only where it spans the element's record box within 0.001 ft, as an arc outline is (RE-96).

| Snowdon Towers against Revit's IFC4 (`plan_profiles_vs_ifc.py`) | main | RE-97 |
|---|---:|---:|
| slabs with an outline (of 199) | 149 | 176 |
| slabs' upward-face plan area within 0.1% of Revit's | 89 | 102 |
| slabs' upward-face vertices within 0.01 ft of Revit's, both ways | 106 | 113 |
| roofs with an outline (of 20) | 13 | 20 |
| roofs' upward-face area within 0.1% of Revit's | 7 | 10 |

`plan_profiles_vs_ifc.py` now takes every element with one Tag together, since a sketch of several pieces is written as several elements with one Tag, by Revit and by rvt-rs (#331). Before, it measured whichever piece came last, so main's slab count reads 89 here against 87 in RE-96.

-----

## 1. The list

Slab 1164441 (a sidewalk) has 12 edges. Eleven of its sketch lines name it; its top edge, line 1516749, names 1506825, an element that is not exported and owns one other line. Without that edge the sketch was an open chain, and the slab kept its record box (10.3% over Revit's area). Its Sketch element, 1164440, names 1164441 at +77, and at +395 lists all 12 lines, 1516749 included: `12 · (1164444, 0) (1164445, 1) … (1516763, 11)`.

Roof 754672's Sketch, 754671, lists 20 lines with keys 4, 5, 6, 7, 9, 10, …: curves 0 to 3 and 8 were deleted. Only 5 of its lines name the roof.

`examples/probe_re97_sketch_curve_lists.rs` reads every list:

| file | sketches with a list | lists that differ from the lines naming the element |
|---|---:|---:|
| Snowdon Towers | 447 | 55 |
| Snowdon Towers structural | 112 | 7 |
| Core Interior | 128 | 6 |
| MIT house 2024 | 11 | 0 |

A list is read with at least 3 curves, rising keys, sketch-line ids only, and every copy agreeing. A line two lists name drops both. Revit 2024 only.

## 2. What changes on Snowdon Towers

34 products change: 25 slabs and roofs, some written as several pieces. All 25 are in Revit's export:
- 16 are now within 0.1% of Revit's upward-face area (13 slabs, 3 roofs). Before, they were their record box, or had no outline.
- 4 slabs (733588, 1423874, 1423980, 1501152) reach exactly half of Revit's area. Revit writes each as two layer solids with one outline, and the area counts both.
- 5 come closer but not within 0.1%:
  - slab 1394726: 0.13%;
  - slabs 1423945 and 1423886: 0.7% and 0.9%;
  - roof 754197: 1.3%;
  - the lawn 1227304: 20.6% (22.3% before).

## 3. Other files

- **Core Interior, the four RE1 models, Einhoven, the MIT house (2024, 2025), Projeto1 and `teste_export_2025`:** byte-identical to main's apart from the timestamp. Their differing lists name elements whose outline was already read or that are not floors or roofs. No witness observation changes.
- **Snowdon Towers structural:** three slabs (670055, 670327, 670421) take their list's outline. Each spans its record box exactly; no Revit export of the file is held.

## 4. End-to-end measurement

| file | sha256 |
|---|---|
| Snowdon Towers Sample Architectural.rvt (local only) | `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1` |
| Snowdon Towers Sample Architectural_IFC4.ifc (local only) | `ecfcb04e3e818090cf818873fe76fba8b447742bbf4d19dad6d590f09cbb985a` |

```bash
cargo run --profile ci --example probe_re97_sketch_curve_lists -- "Snowdon Towers Sample Architectural.rvt"
./target/ci/rvt-ifc "Snowdon Towers Sample Architectural.rvt" -o model.ifc
python3 tools/re/plan_profiles_vs_ifc.py model.ifc "Snowdon Towers Sample Architectural_IFC4.ifc" --class IfcSlab,IfcRoof
```
