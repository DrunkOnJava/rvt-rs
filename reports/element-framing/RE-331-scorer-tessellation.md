# RE-331: The body scorers measure two tessellations, not two shapes

**Date:** 2026-11-04
**Issues:** #1355; follows #1346 and RE-327 section 11 (a circle in two forms, 27 chords against 28), #1280 (the "ref on ours" bar), RE-311 and RE-329 (the saved-mesh body scorer)
**Artefacts:** the RE-32 oracle pair of RE1 Architecture, `RE1-Architecture.ifc` (Revit's export, sha256 a9b5d366…6200) and `RE1-Architecture.rvt` (sha256 4a78bdd6…8ce4); Core Interior's `2024_Core_Interior_slim.ifc` (sha256 bfdf36ff…ad86d); IfcOpenShell 0.9.0.post1, the version `tools/ci/requirements-ifc.txt` pins
**Tools:** `tools/re/fine_mesh.py` (the settings), `tools/re/mesh_bodies_vs_ifc.py`, `tools/re/saved_meshes_vs_ifc.py` and `tools/re/family_extrusion_bodies_vs_ifc.py` (the scorers), `tools/re/arc_tessellation_study.py` (the study of sections 2 and 3), Measure's `fine_mesh` input
**Runs:** to be added when they finish (section 4)
**Result:** the fine mode is built and proved on the known cases (sections 2 and 3). The measured effect of the fine mode across the models, and the options for the bar, are in sections 4 and 5, which are filled in when the Measure runs have finished.

## 1. The question

`mesh_bodies_vs_ifc`, `saved_meshes_vs_ifc` and `family_extrusion_bodies_vs_ifc` mesh rvt-rs's IFC and Revit's with IfcOpenShell's default settings, and check Revit's vertices against rvt-rs's surface ("ref on ours", 0.001 ft, Griffin's #1280 bar), plus the volume ratio and both directions. That measures the agreement of two tessellations. RE-327 section 11 found the case where the two tessellations disagree about one shape: the same exact circle is written as two half arcs by Revit 24.0.5's IFC4 export and as an `IfcCircleProfileDef` by Revit 26.2's IFC2X3 export, and IfcOpenShell's default tessellation cuts them into 28 and 27 chords at 69.85 mm (30 and 29 at 80 mm), so their vertices sit 0.0014 ft apart. #1346 therefore keeps the saved mesh for any arc over 0.12 ft in radius, which gave up the true arcs of the office's 78 floor drains that matched Revit's export exactly. This report adds the measurement that does not depend on the chords, beside today's, and says what the bar would do under it. Changing the bar changes published figures and is Griffin's decision (section 5).

## 2. The fine mode

**What it does.** With `RVT_SCORER_FINE_MESH=1` (Measure's `fine_mesh` input sets it; any value but empty and `0` turns it on) each of the three scorers meshes the bodies it scores a second time in both files at one equal, fine deflection, scores them with the same arithmetic, and prints `fine` rows after everything it printed before, which is byte for byte as it was (section 3.4). Every vertex of a body then lies within the deflection of its true surface, and two tessellations of one surface lie within about twice that of each other, whichever chords they use. A body whose two meshes are the default meshes (a faceted body: the deflection does not touch it) keeps its default scores, since the same triangles give the same scores.

- `mesh_bodies_vs_ifc` prints, per class and in total: the columns it always prints (volume within 1% and 0.01%, both directions within 0.01 ft and 0.001 ft, "ref on ours"), "ref on ours" within 0.0015 ft and 0.002 ft, both directions within 1e-5 ft (the tessellations coincide), the median volume ratio and the largest "ref on ours"; then per class how many bodies are meshed differently and how many hold "ref on ours" and "0.001 ft" (both directions) in the default tessellation, the fine one, both or neither; then the bodies that miss "ref on ours" in the fine one, with their default distances beside.
- `family_extrusion_bodies_vs_ifc` prints its census of the bodies with an arc (by the form Revit's export gives them and the arcs rvt-rs wrote) a second time for the fine scores, led by `fine`.
- `saved_meshes_vs_ifc` (a box scorer) meshes an even stride of at most 300 paired elements of each class again and prints, per class, the boxes within 0.001 ft in the default tessellation and in the fine one, and the fine buckets.
- The held-out scrub (`tools/ci/scrub-holdout-results.sh`) keeps the `fine` rows of the first and last: numbers beside an IFC class name, as it keeps their other rows.

**The values.** IfcOpenShell 0.9.0.post1 takes `mesher-linear-deflection` (metres, default 0.001) and `mesher-angular-deflection` (radians, default 0.5) in `ifcopenshell.geom.settings()`; `circle-segments` (720 and 1,024 tried) left the chord counts of a circle profile at 27 and 28, so it is not used. A linear deflection of 1e-5 ft (3.048e-6 m) and an angular one of 0.03 rad cut a whole circle into 419 to 1,396 chords, depending on the radius, where the default cuts it into 26 to 80 (section 3.1). The two deflections bound the miss together: the angular one sets the step of a small circle and the linear one the sagitta of a large one, and with the linear deflection at 1e-5 m the forms of a circle of 1 to 2 ft in radius were up to 1.7e-5 ft apart where at 3.048e-6 m (1e-5 ft) they are up to 6e-6 ft. The cost is the mesher's, and it grows steeply as the angular deflection falls: 0.71 s to mesh one circle body at 0.01 rad and 0.07 s at 0.03 rad (linear 1e-5 m), 0.20 to 0.23 s at 0.02 to 0.04 rad with the linear deflection of 3.048e-6 m. `python3 tools/re/arc_tessellation_study.py --deflections` measures them: the mean time to mesh one circle body, and over radii of 0.229, 0.2625, 0.5, 1 and 2 ft the worst miss between rvt-rs's two half arcs and Revit's circle written as the three forms at four starts (feet, either direction; one machine under load, so the times compare with each other):

| linear deflection, m | angular deflection, rad | mean seconds per circle body | worst miss between the forms, ft |
|---:|---:|---:|---:|
| 1e-5 | 0.01 | 0.708 | 0.000007 |
| 1e-5 | 0.02 | 0.127 | 0.000013 |
| 1e-5 | 0.03 | 0.073 | 0.000017 |
| 1e-5 | 0.05 | 0.051 | 0.000017 |
| 3.048e-6 (1e-5 ft) | 0.02 | 0.233 | 0.000006 |
| **3.048e-6 (1e-5 ft)** | **0.03** | **0.205** | **0.000006** |
| 3.048e-6 (1e-5 ft) | 0.04 | 0.204 | 0.000006 |

With the linear deflection at 1e-5 ft the worst miss is 6e-6 ft from 0.02 to 0.04 rad and the cost does not move, so 0.03 rad is the middle of a flat stretch. With it at 1e-5 m the miss grows as the angular deflection loosens (7e-6 ft at 0.01 rad, 1.7e-5 ft at 0.03 and 0.05), and a tighter angular deflection only costs more.

**Cost in the files.** Revit's export of RE1 Architecture (72 elements): 1.2 s to mesh in the default tessellation and 6.6 s in the fine one, 17,106 triangles against 50,890; four elements (the dispenser among them) grow 14 to 16 times and the other 68 do not change. Revit's export of the office's concrete terrain variant (132 elements): 11.8 s against 18.5 s, 167,827 triangles against 239,427; 19 elements grow. The scorers mesh at most 300 bodies of each class (`CLASS_CAP`), so the extra work is bounded the way the default work is.

## 3. Proof on the known cases

### 3.1 One circle, two forms, two tessellations

`python3 tools/re/arc_tessellation_study.py` writes hand-made STEP files in the shape rvt-rs writes (metres, one `IfcExtrudedAreaSolid` over an `IfcIndexedPolyCurve` of `IfcArcIndex` segments, or over an `IfcCircleProfileDef`), meshes them with the scorers' own settings and arithmetic, and prints, for the default tessellation and the fine one, the chords a whole circle is cut into:

| radius mm | radius ft | `IfcCircleProfileDef` | 2 half arcs | 4 quarter arcs |
|---:|---:|---:|---:|---:|
| 6.35 | 0.0208 | 26 / 419 | 26 / 420 | 28 / 420 |
| 25.40 | 0.0833 | 26 / 419 | 26 / 420 | 28 / 420 |
| 41.80 | 0.1371 | 26 / 419 | 26 / 420 | 28 / 420 |
| 50.00 | 0.1640 | 26 / 419 | 26 / 420 | 28 / 420 |
| **69.85** | **0.2292** | **27 / 476** | **28 / 476** | 28 / 476 |
| **80.00** | **0.2625** | **29 / 509** | **30 / 510** | 32 / 512 |
| 100.00 | 0.3281 | 32 / 570 | 32 / 570 | 32 / 572 |
| 150.00 | 0.4921 | 39 / 697 | 40 / 698 | 40 / 700 |
| 300.00 | 0.9843 | 55 / 986 | 56 / 986 | 56 / 988 |
| 600.00 | 1.9685 | 77 / 1,394 | 78 / 1,394 | 80 / 1,396 |

(default / fine). The default counts are RE-327 section 11.2's. In the fine tessellation the counts still differ by one or two at some radii (476 and 476 at 69.85 mm, 509 and 510 at 80 mm), but every vertex lies within the sagitta of 419 chords or more, so the surfaces coincide.

### 3.2 The two forms against each other

Rvt-rs's two half arcs against Revit's circle, as an `IfcCircleProfileDef` at the same start, and as the three forms (profile, half arcs, quarter arcs) at four starts (0, 90, 7.3 and 180 degrees), the worst of the twelve; "Revit's vertices to ours" and "ours to Revit's" in feet:

| radius mm | radius ft | same start, default: Revit's to ours, ours to Revit's, volume ratio | same start, fine | worst, default | worst, fine |
|---:|---:|---|---|---|---|
| 6.35 | 0.0208 | 0.000000, 0.000000, 1.00000 | 0.000001, 0.000001, 1.00000 | 0.000153, 0.000152 | 0.000002, 0.000002 |
| 25.40 | 0.0833 | 0.000000, 0.000000, 1.00000 | 0.000002, 0.000002, 1.00000 | 0.000608, 0.000608 | 0.000003, 0.000003 |
| 41.80 | 0.1371 | 0.000000, 0.000000, 1.00000 | 0.000004, 0.000004, 1.00000 | 0.001000, 0.001000 | 0.000004, 0.000005 |
| 50.00 | 0.1640 | 0.000000, 0.000000, 1.00000 | 0.000005, 0.000005, 1.00000 | 0.001196, 0.001196 | 0.000005, 0.000006 |
| **69.85** | **0.2292** | **0.001439, 0.001550, 1.00064** | 0.000000, 0.000000, 1.00000 | 0.001441, 0.001550 | 0.000005, 0.000005 |
| **80.00** | **0.2625** | **0.001436, 0.001539, 1.00051** | 0.000005, 0.000005, 1.00000 | 0.001438, 0.001539 | 0.000006, 0.000005 |
| 100.00 | 0.3281 | 0.000000, 0.000000, 1.00000 | 0.000000, 0.000000, 1.00000 | 0.001440, 0.001441 | 0.000006, 0.000007 |
| 150.00 | 0.4921 | 0.001516, 0.001596, 1.00021 | 0.000005, 0.000005, 1.00000 | 0.001517, 0.001596 | 0.000006, 0.000005 |
| 300.00 | 0.9843 | 0.001548, 0.001605, 1.00008 | 0.000000, 0.000000, 1.00000 | 0.001548, 0.001605 | 0.000005, 0.000006 |
| 600.00 | 1.9685 | 0.001596, 0.001638, 1.00003 | 0.000000, 0.000000, 1.00000 | 0.001596, 0.001638 | 0.000005, 0.000006 |

The default columns at 69.85 and 80 mm are the 0.0014 ft miss of RE-327 section 11.2 (0.001439 and 0.001436 ft), reproduced. In the fine tessellation the two forms coincide within 1e-5 ft (at most 0.000007 ft) at every radius and every start, including the quarter-arc form and a circle turned half a step.

### 3.3 The dispenser, and the scorer itself

RE1 Architecture's element 360331 (RE-327 section 11.1: seven extrusions, five of them over an `IfcCircleProfileDef`), redrawn the ways rvt-rs could draw it and scored against Revit's own body (`python3 tools/re/arc_tessellation_study.py RE1-Architecture.ifc`):

| drawing | default: volume ratio, Revit's to ours, ours to Revit's | fine |
|---|---|---|
| two half arcs per circle (the first run) | 1.00055, 0.001439, 0.001542 | 1.00000, 0.000002, 0.000002 |
| `IfcCircleProfileDef`, reference direction as Revit's | 1.00000, 0.000003, 0.000000 | 1.00000, 0.000000, 0.000000 |
| `IfcCircleProfileDef`, turned 90 degrees | 1.00000, 0.001162, 0.001162 | 1.00000, 0.000002, 0.000002 |
| `IfcCircleProfileDef`, turned 180 degrees | 1.00000, 0.001550, 0.001550 | 1.00000, 0.000002, 0.000002 |

The default column is run 41805333248's dispenser (0.001439 ft) to the digit. The scorer itself, end to end: `python3 tools/re/arc_tessellation_study.py RE1-Architecture.ifc --write-ours ours.ifc` writes the half-arc drawing as an IFC of rvt-rs's shape (`BodySource` `family_extrusions`, Tag 360331, placed where Revit's element is), and `RVT_SCORER_FINE_MESH=1 python3 tools/re/family_extrusion_bodies_vs_ifc.py ours.ifc RE1-Architecture.ifc` prints the default rows (volume ratio 1.0006, Revit's vertices 0.001439 ft from the surface, rvt-rs's 0.001542 ft from Revit's, 0 of 1 holding "ref on ours") and then the `fine` rows (volume ratio 1.0000, 0.000002 ft, 1 of 1 holding "ref on ours" and both directions within 1e-5 ft; the `fine change` row has it in "fine only"). This body is hand-drawn as rvt-rs writes it; Measure's flag run measures rvt-rs's own export (section 4).

### 3.4 The default output is as it was

The three scorers' output without the variable was compared, byte for byte, with the scorers of `main` (4be3a8ab) on the pairs of files that were at hand: RE1 Architecture's five saved-mesh doors against Revit's export, Core Interior's 138 saved-mesh doors and windows against its slim export (both scorers, `mesh_bodies_vs_ifc` and `saved_meshes_vs_ifc`), and the saved-mesh bodies of RE1 Electrical, Mechanical and Plumbing (none: both scorers print their empty tables). All ten comparisons are identical, and the dispenser's default rows above are the same as the scorer of `main` prints. On those files the fine mode finds no body meshed differently from the default (Core Interior's doors and windows and RE1 Architecture's doors are faceted in Revit's export): 138 of 138 and 5 of 5 with every default and fine score equal. The effect on curved bodies is measured by Measure (section 4), which also compares the default rows of its runs with the earlier runs of `main`'s scorers.

## 4. What the runs measured

RUNS

## 5. Options for the bar

OPTIONS

## 6. Not done, and not measured

- **Analytic distances** for planes, cylinders and circles (the issue's "where cheaper and exact") are not used. The fine tessellation alone reaches the 1e-5 ft the two forms of a circle need (section 3.2) at 0.2 to 0.3 s per circle body, bounded by the scorers' 300 bodies per class, so an exact surface model of each body would have added code without changing a figure. Circles over 2 ft in radius were not measured.
- **IfcOpenShell other than 0.9.0.post1**, which is what CI pins and what the values of section 2 were found on; another consumer's tessellation of an arc.
- **Bodies the scorers do not reach**: more than 300 of a class (every Nth is scored, as in the default mode), and a body IfcOpenShell cannot mesh at either deflection.
- **A tolerance for the fine tessellation** other than the three columns of "ref on ours" (0.001, 0.0015 and 0.002 ft): the bar is Griffin's.

## 7. Reproduce

```bash
python3 tools/re/arc_tessellation_study.py RE1-Architecture.ifc --write-ours ours.ifc --deflections
RVT_SCORER_FINE_MESH=1 python3 tools/re/family_extrusion_bodies_vs_ifc.py ours.ifc RE1-Architecture.ifc
```

RUN_COMMANDS
