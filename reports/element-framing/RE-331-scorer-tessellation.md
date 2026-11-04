# RE-331: The body scorers measure two tessellations, not two shapes

**Date:** 2026-11-04
**Issues:** #1355, #1365 (section 8); follows #1346 and RE-327 section 11 (a circle in two forms, 27 chords against 28), #1280 (the "ref on ours" bar), RE-311 and RE-329 (the saved-mesh body scorer)
**Artefacts:** the RE-32 oracle pair of RE1 Architecture, `RE1-Architecture.ifc` (Revit's export, sha256 a9b5d366…6200) and `RE1-Architecture.rvt` (sha256 4a78bdd6…8ce4); Core Interior's `2024_Core_Interior_slim.ifc` (sha256 bfdf36ff…ad86d); IfcOpenShell 0.9.0.post1, the version `tools/ci/requirements-ifc.txt` pins
**Tools:** `tools/re/fine_mesh.py` (the settings), `tools/re/mesh_bodies_vs_ifc.py`, `tools/re/saved_meshes_vs_ifc.py` and `tools/re/family_extrusion_bodies_vs_ifc.py` (the scorers), `tools/re/arc_tessellation_study.py` (the study of sections 2 and 3), Measure's `fine_mesh` input; for section 8 `tools/re/true_curves.py` (which bodies both files write as true curves)
**Runs:** (section 8: 41871104420, 41871109552 and 41871114299) Measure 41850758987 (the unfiltered run, default exports), 41850764234 (the flag run) and 41850769925 (`--saved-meshes` alone), all at `e024f03c`; 41854514903 (the arcs the current rule keeps as meshes, drawn by the first arc run's head `aaa38de8`, section 4.3). Section 4 reads them.
**Result:** positive: the fine mode runs on every model (its longest step took 676 s of the 2,400 s limit) and changes no default row (99 of 99 files equal in each of two comparisons with `main`'s scorers). It finds that a tessellation decides some verdicts, and only for curved saved-mesh bodies: of the 3,816 saved-mesh bodies of the `--saved-meshes` run, 295 are meshed differently; 38 hold "ref on ours" in the default tessellation and miss it in the fine one (the largest is 0.001001 ft, printed to six decimals, against the bar's 0.001); 158 miss the 0.001 ft both-directions column in the default one and hold it in the fine one. None of the 1,368 doors and windows the default export draws from saved meshes, and none of the 66 family-extrusion bodies, changes verdict (2 of the 66, circles of 0.12 ft or less, are meshed differently and hold in both). Drawn as arcs by the first arc run's head, 185 of the 269 bodies the current rule keeps as meshes are scored: all 185 hold in the fine tessellation and 184 in the default one; the 84 other bodies, office spaces, are not scored by this scorer. Section 5 lists the options for the bar and records Griffin's choice, the fine tessellation where both files write true curves (option 3).

## 1. The question

`mesh_bodies_vs_ifc`, `saved_meshes_vs_ifc` and `family_extrusion_bodies_vs_ifc` mesh rvt-rs's IFC and Revit's with IfcOpenShell's default settings, and check Revit's vertices against rvt-rs's surface ("ref on ours", 0.001 ft, Griffin's #1280 bar), plus the volume ratio and both directions. That measures the agreement of two tessellations. RE-327 section 11 found the case where the two tessellations disagree about one shape: the same exact circle is written as two half arcs by Revit 24.0.5's IFC4 export and as an `IfcCircleProfileDef` by Revit 26.2's IFC2X3 export, and IfcOpenShell's default tessellation cuts them into 28 and 27 chords at 69.85 mm (30 and 29 at 80 mm), so their vertices sit 0.0014 ft apart. #1346 therefore keeps the saved mesh for any arc over 0.12 ft in radius, which gave up the true arcs of the office's 78 floor drains that matched Revit's export exactly. This report adds the measurement that does not depend on the chords, beside today's, and says what the bar would do under it. Changing the bar changes published figures and is Griffin's decision (section 5).

## 2. The fine mode

**What it does.** With `RVT_SCORER_FINE_MESH=1` (Measure's `fine_mesh` input sets it; `1` or `true`, in any case, turns it on and nothing else does) each of the three scorers meshes the bodies it scores a second time in both files at one equal, fine deflection, scores them with the same arithmetic, and prints `fine` rows after everything it printed before, which is byte for byte as it was (section 3.4). Every vertex of a body then lies within the deflection of its true surface, and two tessellations of one surface lie within about twice that of each other, whichever chords they use. A body whose two meshes are the default meshes (a faceted body: the deflection does not touch it) keeps its default scores, since the same triangles give the same scores.

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

`python3 tools/re/arc_tessellation_study.py` writes hand-made STEP files in the shape rvt-rs writes (metres, one `IfcExtrudedAreaSolid` over an `IfcIndexedPolyCurve` of `IfcArcIndex` segments, or over an `IfcCircleProfileDef`), meshes them with the scorers' own settings and arithmetic, and prints, for the default tessellation and the fine one, the chords a whole circle is cut into. The study is a tool, not a scorer: it has no switch and always prints its fine tables after the default ones (`--deflections` adds the table of section 2).

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

**Runs.** The scorers are this branch's (`--ref feat/1355-scorer-fine-mode`, `-f fine_mesh=true`); the code is `main` at `ac7884e0`, which the branch merges; `-f ref=e024f03c9cd14837c8ed49d0ab38d1fd414ff02b`; the branch's later commit changes this report only.

- **41850758987**, the unfiltered run: `-f base=none -f nordic=true -f r2025=true -f holdout=true`, 53 models, every scorer, default exports. The door and window saved meshes of every model (section 4.1).
- **41850764234**, the flag run: the models of 41843267888 (`core-interior re1-* einhoven nordic-office-* r2025-*`) with `-f ifc_flags="--saved-meshes --family-extrusions"`, the scorers `family_extrusion_bodies_vs_ifc mesh_bodies_vs_ifc saved_meshes_vs_ifc`. The family-extrusion bodies (section 4.2).
- **41850769925**, the same models with `-f ifc_flags="--saved-meshes"`: every saved-mesh body, the curved ones among them (section 4.4).
- **41860674204**, `saved_meshes_vs_ifc` alone on those models with `-f ref=e024f03c9cd14837c8ed49d0ab38d1fd414ff02b` and the scorer of `9dee1107`, which removes the default table's offset from the fine boxes too (section 4.4); `exit 0`, the office files' step 573 s and 569 s.
- **41854514903**, the arcs (section 4.3), dispatched after the three above: `-f ref=aaa38de808f1731acf6f6a6dace59904f873335b`, the head of the first arc run (41805333248), which drew every arc as an arc, with `-f ifc_flags="--saved-meshes --family-extrusions"`, `-f scorers=family_extrusion_bodies_vs_ifc` and `-f fine_mesh=true`, on `core-interior re1-* nordic-office-*`.

All four finished green and every scorer step in them ended `exit 0`.

**The default rows did not move.** Take the `fine` lines out of the flag run's scorer files and what is left, for each of the 99 files (33 models with a Revit export, three scorers), is the text of the earlier run of `main`'s scorers with the same flags, byte for byte: 41843267888 (`--saved-meshes --family-extrusions`) against 41850764234, and 41843276506 (`--saved-meshes`) against 41850769925. 99 of 99 equal, none different, in each pair (the earlier runs were at `7ebb1bab`; these are at `e024f03c`, which adds #1351's area change and `main`'s later commits, and no body moved). The unfiltered run has no earlier run of `main`'s scorers at the default flags to read against; the default rows of its Core Interior (138 bodies) and RE1 Architecture (5) files equal what `main`'s scorers print on exports of an earlier head (section 3.4), and its code path is the one the two pairs above proved.

**Run time.** No scorer step brushed its limit (2,400 s). The longest, `saved_meshes_vs_ifc` on the office at permit in the flag run, took 676 s; the arcs run's scorer took 45 s at most. The fine pass roughly doubles that scorer on the office files (as built 341 s without it in 41843267888, 673 s with it; at permit 345 and 676 s) and adds about 14 s to `mesh_bodies_vs_ifc` on them (27 and 26 s to 41 and 40 s); on the small models it adds seconds (RE1 Architecture's `saved_meshes_vs_ifc` 12 s to 41 s). A step runs beside the others on its runner, so those times compare with each other and are not benchmarks.

In the tables below, "ref on ours" is every tested vertex of Revit's body within 0.001 ft of rvt-rs's surface, and "both directions" is every tested vertex of each body within 0.001 ft of the other's surface; the four counts under each say how many bodies hold it in the default tessellation and in the fine one, in both, in only the default one ("default only": the bodies a change of bar would cost), in only the fine one, or in neither. "Meshed differently" counts the bodies whose triangles are not the default ones in either file, which are the bodies the deflection touches: a faceted body keeps its default scores.

### 4.1 The default door and window saved meshes (run 41850758987)

| model | class | bodies | meshed differently | ref on ours: both | default only | fine only | neither | both directions 0.001 ft: both | default only | fine only | neither |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Core Interior | IfcDoor | 132 | 0 | 132 | 0 | 0 | 0 | 132 | 0 | 0 | 0 |
| Core Interior | IfcWindow | 6 | 0 | 6 | 0 | 0 | 0 | 6 | 0 | 0 | 0 |
| office, as built | IfcDoor | 269 | 0 | 269 | 0 | 0 | 0 | 265 | 0 | 0 | 4 |
| office, as built | IfcWindow | 269 | 0 | 269 | 0 | 0 | 0 | 269 | 0 | 0 | 0 |
| office, permit | IfcDoor | 269 | 0 | 269 | 0 | 0 | 0 | 265 | 0 | 0 | 4 |
| office, permit | IfcWindow | 265 | 0 | 265 | 0 | 0 | 0 | 265 | 0 | 0 | 0 |
| R2025 12 | IfcWindow | 3 | 0 | 3 | 0 | 0 | 0 | 3 | 0 | 0 | 0 |
| R2025 12 | IfcDoor | 1 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 1 |
| R2025 13 | IfcWindow | 3 | 0 | 3 | 0 | 0 | 0 | 3 | 0 | 0 | 0 |
| R2025 13 | IfcDoor | 1 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 1 |
| R2025 14 | IfcWindow | 3 | 0 | 3 | 0 | 0 | 0 | 3 | 0 | 0 | 0 |
| R2025 14 | IfcDoor | 1 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 1 |
| R2025 15 | IfcWindow | 3 | 0 | 3 | 0 | 0 | 0 | 3 | 0 | 0 | 0 |
| R2025 15 | IfcDoor | 1 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 1 |
| R2025 16 | IfcWindow | 3 | 0 | 3 | 0 | 0 | 0 | 3 | 0 | 0 | 0 |
| R2025 16 | IfcDoor | 1 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 1 |
| R2025 17 | IfcWindow | 3 | 0 | 3 | 0 | 0 | 0 | 3 | 0 | 0 | 0 |
| R2025 17 | IfcDoor | 1 | 0 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 1 |
| RE1 Architecture | IfcDoor | 5 | 0 | 5 | 0 | 0 | 0 | 5 | 0 | 0 | 0 |
| held-out models (9 of 18 hold some) | IfcDoor | 29 | 0 | 1 | 0 | 0 | 28 | 1 | 0 | 0 | 28 |
| held-out models (10 of 18 hold some) | IfcWindow | 100 | 0 | 100 | 0 | 0 | 0 | 100 | 0 | 0 | 0 |
| every model | all | 1368 | 0 | 1334 | 0 | 0 | 34 | 1326 | 0 | 0 | 42 |

(The held-out models are counts only, summed over the models that hold some.) The default export draws only doors and windows from saved meshes, and 1,368 of them are scored across the models.

- **No door or window is meshed differently** in any model, held-out included: Revit writes them faceted, so the two tessellations are the same triangles and every default figure is a fine figure. Default only 0, fine only 0.
- 1,334 hold "ref on ours" in both. The 34 that miss it miss it in both: 28 of the held-out models' 29 doors and the one door of each of R2025 12 to 17. They are not a matter of tessellation (their meshes are identical) and the default rows already count them. Both directions within 0.001 ft: 1,326 in both, 42 in neither (the 34, and 4 doors in each office file).
- So switching the bar costs no door or window. The 0.14.0 changelog's counts for them (the windows drawn from their mesh, "every one exact against Revit's export": the 88 held-out windows, the 18 R2025 windows) are the same in the fine tessellation: the 18 R2025 windows and all 100 held-out windows hold in both.

### 4.2 The family-extrusion bodies (run 41850764234)

| model | class | bodies | meshed differently | ref on ours: both | default only | fine only | neither | both directions 0.001 ft: both | default only | fine only | neither | fine: ref on ours within 0.0015 ft | largest ref on ours, ft |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Core Interior | IfcDoor | 37 | 0 | 37 | 0 | 0 | 0 | 37 | 0 | 0 | 0 | 37 | 0.000002 |
| RE1 Architecture | IfcFurnishingElement | 5 | 0 | 5 | 0 | 0 | 0 | 5 | 0 | 0 | 0 | 5 | 0.000003 |
| RE1 Architecture | IfcBuildingElementProxy | 2 | 1 | 2 | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 2 | 0.000003 |
| RE1 Electrical | IfcFlowTerminal | 2 | 0 | 2 | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 2 | 0.000002 |
| RE1 Mechanical | IfcBuildingElementProxy | 1 | 0 | 1 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 1 | 0.000002 |
| RE1 Plumbing | IfcFlowTerminal | 1 | 1 | 1 | 0 | 0 | 0 | 1 | 0 | 0 | 0 | 1 | 0.000002 |
| office, as built | IfcDoor | 4 | 0 | 4 | 0 | 0 | 0 | 4 | 0 | 0 | 0 | 4 | 0.000034 |
| office, as built | IfcFurniture | 3 | 0 | 3 | 0 | 0 | 0 | 3 | 0 | 0 | 0 | 3 | 0.000001 |
| office, as built | IfcBuildingElementProxy | 2 | 0 | 2 | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 2 | 0.000002 |
| office, permit | IfcDoor | 4 | 0 | 4 | 0 | 0 | 0 | 4 | 0 | 0 | 0 | 4 | 0.000034 |
| office, permit | IfcFurniture | 3 | 0 | 3 | 0 | 0 | 0 | 3 | 0 | 0 | 0 | 3 | 0.000001 |
| office, permit | IfcBuildingElementProxy | 2 | 0 | 2 | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 2 | 0.000002 |

- 66 bodies are replaced, as in the runs of section 9 of RE-327 (Core Interior 37, each office file 9, RE1 Architecture 7, Electrical 2, Mechanical 1, Plumbing 1), and **all 66 hold "ref on ours" and both directions within 0.001 ft in both tessellations**: default only 0, fine only 0, neither 0.
- Only 2 of the 66 are meshed differently: the circles of 0.12 ft or less drawn as arcs, RE1 Architecture's proxy and Plumbing's flow terminal (Revit's IFC2X3 export writes them as an `IfcCircleProfileDef`). Both are within 3e-6 ft of Revit's in the fine tessellation and within 1e-5 ft in the default one. The other 64 are straight and unaffected.
- The 269 instances the rule keeps as meshes for an arc over 0.12 ft are not in this run; section 4.3.

### 4.3 The arcs the current rule keeps as meshes (run 41854514903)

The flag runs cannot show them, because the current rule keeps them as meshes. Run 41854514903 draws them: the code is `aaa38de8`, the head of the first arc run (41805333248), which draws every arc as an arc, with this branch's scorers, `--saved-meshes --family-extrusions`, `family_extrusion_bodies_vs_ifc` only and `fine_mesh=true`, on Core Interior, RE1 and the BIM4LCA files.

**The counts, reconciled.** That head replaces 335 bodies (RE-327 section 9), and the current rule replaces 66 of them and keeps 269 as meshes for an arc over 0.12 ft in radius (the office files 120 each, the terrain files 11 each, RE1 Architecture 1, Electrical 6). The body scorer scored **251** of the 335 (Core Interior 37, each office file 87, each terrain file 11, RE1 Architecture 8, Electrical 8, Mechanical 1, Plumbing 1). The other **84** are the 42 family instances per office file that the first run wrote as `IfcSpace` (RE-327 section 9: "87 (+42 spaces)"); `space_profiles_vs_ifc` scores them, and this run did not run it. The run's own census counts the scored bodies with an arc over 0.12 ft in radius: 78 in each office file, 11 in each terrain file, 6 in Electrical and 1 in RE1 Architecture, **185** in all. By count the other 66 of the 251 are the bodies the current rule draws (Core Interior 37, 9 in each office file, 7 in Architecture, 2 in Electrical, 1 in Mechanical, 1 in Plumbing), so 251 = 66 + 185, and the 269 = 185 scored + 84 office spaces not scored. The artefacts print classes and counts, not the element ids, so this is arithmetic on the counts and not a match of ids.

**What the 185 score** (arcs over 0.12 ft, drawn as arcs, against Revit's export; "within 1e-5 ft" is both directions, the two tessellations coinciding):

| model | bodies | Revit's form of the body | default: "ref on ours", within 1e-5 ft | fine: "ref on ours", within 1e-5 ft |
|---|---:|---|---|---|
| office, as built | 78 | extrusion of indexed curve with arcs | 78, 78 | 78, 78 |
| office, permit | 78 | extrusion of indexed curve with arcs | 78, 78 | 78, 78 |
| office terrain, concrete | 11 | extrusion of indexed curve with arcs | 11, 11 | 11, 11 |
| office terrain, timber | 11 | extrusion of indexed curve with arcs | 11, 11 | 11, 11 |
| RE1 Electrical | 6 | extrusion of circle | 6, 6 | 6, 6 |
| RE1 Architecture | 1 | extrusion of circle + extrusion of rectangle | 0, 0 | 1, 1 |
| total | 185 | | 184, 184 | 185, 185 |

- **All 185 hold "ref on ours" in the fine tessellation**, and every model prints `fine bodies that miss 'ref on ours': 0`. The largest "ref on ours" of the classes that hold them is 0.000006 ft (RE1 Electrical's sensors and Architecture's proxies); the terminals' is 0.000002 ft. The terminals' saved meshes, the same elements, are at 0.001001 ft at most in the fine tessellation (section 4.4).
- **In the default tessellation 184 of the 185 hold, and the one that does not is the dispenser** (RE1 Architecture's element 360331, a circle of 0.2292 ft in radius that Revit 26.2 writes as an `IfcCircleProfileDef`): "ref on ours" 0.001439 ft, rvt-rs's to Revit's 0.001545 ft, volume ratio 1.0006, which the fine tessellation puts at 1e-5 ft or less. It is the only body of the 251 whose verdict differs between the tessellations, and it fails both directions within 0.001 ft in the default one and holds in the fine one.
- The other 66 bodies hold both. **187 of the 251 are meshed differently**: the 185 and the two circles of 0.12 ft or less (Architecture and Plumbing), which hold in both.
- **Not measured**: the 84 office spaces; the arcs on any model or exporter besides these files; and whether another exporter's circle of a radius where IfcOpenShell's chord counts differ (0.2292, 0.4921, 0.9843 ft, RE-327 section 11.2) would hold the default bar, which is what the any-arc rule guards against.

### 4.4 Every other curved saved-mesh body (run 41850769925)

Under `--saved-meshes`, 3,816 bodies are scored (Core Interior 138, the office 1,650 and 1,738, the terrain files 16 each, RE1 53, 47, 68 and 60, the R2025 models 30). **295 are meshed differently**, which makes them the curved bodies of Revit's export; the other 3,521 are faceted and have the same scores in both tessellations. The classes with a curved body:

| model | class | bodies | meshed differently | ref on ours: both | default only | fine only | neither | both directions 0.001 ft: both | default only | fine only | neither | fine: ref on ours within 0.0015 ft | largest ref on ours, ft |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| RE1 Architecture | IfcFurnishingElement | 21 | 1 | 21 | 0 | 0 | 0 | 18 | 0 | 0 | 3 | 21 | 0.000980 |
| RE1 Architecture | IfcBuildingElementProxy | 9 | 2 | 9 | 0 | 0 | 0 | 8 | 0 | 1 | 0 | 9 | 0.000980 |
| RE1 Architecture | IfcFlowTerminal | 6 | 1 | 3 | 0 | 0 | 3 | 2 | 0 | 1 | 3 | 3 | 0.004030 |
| RE1 Electrical | IfcBuildingElementProxy | 28 | 6 | 28 | 0 | 0 | 0 | 22 | 0 | 6 | 0 | 28 | 0.000953 |
| RE1 Electrical | IfcFlowTerminal | 14 | 10 | 14 | 0 | 0 | 0 | 4 | 0 | 10 | 0 | 14 | 0.000953 |
| RE1 Plumbing | IfcFlowFitting | 54 | 1 | 39 | 0 | 0 | 15 | 1 | 0 | 0 | 53 | 39 | 0.017311 |
| RE1 Plumbing | IfcFlowTerminal | 6 | 6 | 6 | 0 | 0 | 0 | 6 | 0 | 0 | 0 | 6 | 0.000702 |
| office, as built | IfcWasteTerminal | 78 | 78 | 59 | 19 | 0 | 0 | 0 | 0 | 59 | 19 | 78 | 0.001001 |
| office, as built | IfcLightFixture | 43 | 43 | 43 | 0 | 0 | 0 | 43 | 0 | 0 | 0 | 43 | 0.000016 |
| office, as built | IfcStair | 2 | 2 | 2 | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 2 | 0.000282 |
| office, permit | IfcWasteTerminal | 78 | 78 | 59 | 19 | 0 | 0 | 0 | 0 | 59 | 19 | 78 | 0.001001 |
| office, permit | IfcLightFixture | 43 | 43 | 43 | 0 | 0 | 0 | 43 | 0 | 0 | 0 | 43 | 0.000003 |
| office, permit | IfcStair | 2 | 2 | 2 | 0 | 0 | 0 | 2 | 0 | 0 | 0 | 2 | 0.000282 |
| office terrain, concrete | IfcWasteTerminal | 11 | 11 | 11 | 0 | 0 | 0 | 0 | 0 | 11 | 0 | 11 | 0.000965 |
| office terrain, timber | IfcWasteTerminal | 11 | 11 | 11 | 0 | 0 | 0 | 0 | 0 | 11 | 0 | 11 | 0.000965 |

Core Interior, RE1 Mechanical and the R2025 models have none.

- **"Ref on ours" at 0.001 ft.** Today 3,759 of 3,816 hold it. In the fine tessellation 3,721 do: both 3,721, **default only 38**, fine only 0, neither 57. The 38 are `IfcWasteTerminal` saved meshes, 19 in each office file; the largest "ref on ours" of the class is 0.001001 ft, printed to six decimals, against the bar's 0.001, and every one of the 78 per file is within 0.0015 ft. The 57 that miss in both are the same bodies as today (the office's 14 as built and 16 at permit, sinks and a flow terminal; RE1 Architecture's 3, Mechanical's 3 and Plumbing's 15; the 6 doors of R2025 12 to 17), and no fine-only body exists: a mesh that is Revit's chords can only be as far from the true surface in the fine tessellation as it is.
- **Both directions within 0.001 ft.** Today 3,512 of 3,816 hold it; in the fine tessellation 3,670: **fine only 158**, default only 0, neither 146. The 158 are the terminals (59 in each office file, 11 in each terrain file) and RE1 Electrical's 16 and Architecture's 2 curved proxies and terminals, which Revit's own coarse polygon of a circle made miss the default column, and a finer polygon of both makes hold.
- **Volume.** The terminals' median volume ratio is 0.9962 in the fine tessellation against 1.0035 in the default one: the saved mesh is 0.4% under the true circle, and in the fine tessellation `vol .01%` holds for none of them.
- **Boxes** (`saved_meshes_vs_ifc`, an even stride of at most 300 per class; the figures are from run 41850769925, where the fine boxes had a median offset of their own; the scorer now removes the default table's offset from both, and run 41860674204 (`saved_meshes_vs_ifc` alone, the same code and models, this head's scorer) re-measured them: its scorer file is the same as 41850769925's for all 33 models, line for line, so the figures are unchanged): every sampled body stays within 0.01 ft of Revit's box in both tessellations, except two walls at permit that miss it in both. Within 0.001 ft the office as built goes from 1,572 to 1,650 of 1,650 sampled, the 78 terminals being the difference (0 of 78 in the default tessellation), and the office at permit from 1,658 to 1,736 of 1,738; the other models do not change.

## 5. Options for the bar

The decision is Griffin's. Each option below is stated with what the runs of section 4 measured for it. The family-extrusion bodies and the doors and windows are not in the difference between them (4.1 and 4.2: no verdict changes); what differs is the 3,816 saved-mesh bodies of 4.4, and what rvt-rs may draw.

| | **1. Keep the default tessellation** | **2. "Ref on ours" in the fine tessellation** | **3. Report both; the fine one decides only for bodies Revit writes as true curves** |
|---|---|---|---|
| What decides | today's rule: both files meshed by IfcOpenShell's defaults, 0.001 ft | the same bar, both files meshed at the fine deflection | for a body rvt-rs draws as true arcs against Revit's true circle (a family extrusion), the fine tessellation; for a saved mesh, which is itself a tessellation, today's rule |
| Bodies that change verdict (3,816 saved-mesh bodies; 66 family bodies; 1,368 doors and windows) | none | **38 saved meshes would fail "ref on ours"** (19 `IfcWasteTerminal` in each office file; the largest "ref on ours" of the class is 0.001001 ft); 158 would newly hold both directions within 0.001 ft; no door, window or family body changes. With the bar at 0.0015 ft none fails | none of the saved meshes, doors, windows or the 66 family bodies; the arcs it would let in are judged by the fine tessellation |
| 0.14.0 changelog's door and window counts | unchanged | unchanged (no door or window is meshed differently, 4.1) | unchanged |
| Docs (`docs/supported-profile.md`, `docs/evidence.md`) | unchanged: the family-extrusion rows keep "an arc over 0.12 ft keeps its mesh" and the dispenser as "a count mismatch" | the "every vertex within 0.001 ft" figures rise (the office as built from 1,554 to 1,613 of 1,650 scored, at permit from 1,638 to 1,697 of 1,738, in 41850769925's counts; the terrain files and RE1 Electrical too), the family-extrusion rows change with the rule, and the count-mismatch explanation of the dispenser is restated as a tessellation artefact | the saved-mesh figures are unchanged; the family-extrusion rows change with the rule (66 replaced bodies growing by the arcs drawn), and the dispenser's explanation is restated as a tessellation artefact |
| 1.0 criterion W6 (its sentence on the saved meshes: "every vertex within 0.001 ft" on Core Interior's 138 and the office's 1,294 of 1,709) | unchanged | the office count rises, Core Interior's 138 of 138 does not move | unchanged |
| What rvt-rs may draw | nothing new: any arc over 0.12 ft in radius keeps its saved mesh (269 bodies on the measured models). Measured, drawn as arcs: 184 of the 185 scored hold today's bar, and the dispenser does not | true arcs for big circles again in every release (the 269 on the measured models). Measured, drawn as arcs: all 185 scored hold the bar in the fine tessellation; the 84 office spaces are not scored | the same arcs |
| What it costs | nothing | 38 saved meshes that are Revit's own tessellation fall under a bar they were built to meet; the fine tessellation is a second definition of the 0.001 ft figures | a body is judged by one tessellation or the other by how it is written, which needs a rule for "Revit writes it as a true curve" (the form the census prints for Revit's body is the evidence) |

In plain terms. Option 1 changes nothing and keeps big circles as meshes. Option 2 moves the measure itself to the true shape: 38 of Revit's own saved meshes would stop counting as exact (the largest is 0.001001 ft against the bar's 0.001), 158 curved bodies would start counting as exact in both directions, and big circles could be drawn again. Option 3 moves no existing figure and lets big circles be drawn again, judged by the fine tessellation, at the price of a rule that says which bodies Revit writes as true curves.

Neither 2 nor 3 changes what rvt-rs writes today; the follow-up in either case is the one #1355 names: restoring true arcs for big circles, which `src/ifc/family_extrusions.rs` keeps as meshes under the any-arc rule of #1346. Run 41854514903 measures it: drawn as arcs, the 185 scored bodies hold the bar in the fine tessellation, and 184 hold it in the default one.

### Decision (Griffin, 2026-11-04)

Griffin chose, by the question tool, **"Fine where both draw curves"**. That is option 3 above, with the rule stated as **where both files write true curves**: the fine tessellation decides pass or fail only for a body that rvt-rs writes as true curves (a family extrusion with arcs or a circle) against a body Revit's export also writes as a true curve (an `IfcCircleProfileDef`, an `IfcIndexedPolyCurve` with arcs, and the like); a saved mesh keeps today's bar in the default tessellation. **No saved-mesh verdict moves**: on the measured models the 38 saved meshes that hold "ref on ours" only in the default tessellation keep passing, and no figure of the 0.14.0 changelog, the docs or the 1.0 criterion W6 changes by this decision. On the measured models the fine verdict covers the 185 arcs over 0.12 ft in radius that the current rule keeps as meshes (all hold in the fine tessellation, section 4.3) and the circles of 0.12 ft or less already drawn (both hold).

The follow-ups:

- **#1365** implements that bar in the scorers.
- **#1366** draws the 185 circles over 0.12 ft as true arcs again, once #1365 lands. The 84 office spaces among the 269 bodies the current rule keeps as meshes are not scored by these scorers (section 4.3).

## 6. Not done, and not measured

- **Analytic distances** for planes, cylinders and circles (the issue's "where cheaper and exact") are not used. The fine tessellation alone reaches the 1e-5 ft the two forms of a circle need (section 3.2) at 0.2 to 0.3 s per circle body, bounded by the scorers' 300 bodies per class, so an exact surface model of each body would have added code without changing a figure. Circles over 2 ft in radius were not measured.
- **The 84 office spaces** (42 `IfcSpace` family instances per office file) among the 269 bodies the current rule keeps as meshes: `space_profiles_vs_ifc` scores them and run 41854514903 did not run it, so 185 of the 269 are measured (section 4.3).
- **IfcOpenShell other than 0.9.0.post1**, which is what CI pins and what the values of section 2 were found on; another consumer's tessellation of an arc.
- **Bodies the scorers do not reach**: more than 300 of a class (every Nth is scored, as in the default mode), and a body IfcOpenShell cannot mesh at either deflection.
- **A tolerance for the fine tessellation** other than the three columns of "ref on ours" (0.001, 0.0015 and 0.002 ft): the bar is Griffin's.

## 7. Reproduce

```bash
python3 tools/re/arc_tessellation_study.py RE1-Architecture.ifc --write-ours ours.ifc --deflections
RVT_SCORER_FINE_MESH=1 python3 tools/re/family_extrusion_bodies_vs_ifc.py ours.ifc RE1-Architecture.ifc
```

```bash
gh workflow run measure.yml --ref feat/1355-scorer-fine-mode -f ref=e024f03c9cd14837c8ed49d0ab38d1fd414ff02b \
  -f base=none -f nordic=true -f r2025=true -f holdout=true -f fine_mesh=true
gh workflow run measure.yml --ref feat/1355-scorer-fine-mode -f ref=e024f03c9cd14837c8ed49d0ab38d1fd414ff02b \
  -f base=none -f nordic=true -f r2025=true -f fine_mesh=true \
  -f models="core-interior re1-* einhoven nordic-office-* r2025-*" -f ifc_flags="--saved-meshes --family-extrusions" \
  -f scorers="family_extrusion_bodies_vs_ifc mesh_bodies_vs_ifc saved_meshes_vs_ifc"
gh workflow run measure.yml --ref feat/1355-scorer-fine-mode -f ref=e024f03c9cd14837c8ed49d0ab38d1fd414ff02b \
  -f base=none -f nordic=true -f r2025=true -f fine_mesh=true \
  -f models="core-interior re1-* einhoven nordic-office-* r2025-*" -f ifc_flags="--saved-meshes" \
  -f scorers="family_extrusion_bodies_vs_ifc mesh_bodies_vs_ifc saved_meshes_vs_ifc"
gh workflow run measure.yml --ref feat/1355-scorer-fine-mode -f ref=aaa38de808f1731acf6f6a6dace59904f873335b \
  -f base=none -f nordic=true -f fine_mesh=true -f models="core-interior re1-* nordic-office-*" \
  -f ifc_flags="--saved-meshes --family-extrusions" -f scorers="family_extrusion_bodies_vs_ifc"
```

Once this lands, `--ref main` replaces the branch. The `fine` rows are in each model's `mesh_bodies_vs_ifc.txt`, `saved_meshes_vs_ifc.txt` and `family_extrusion_bodies_vs_ifc.txt`; the held-out models keep their counts only (`tools/ci/scrub-holdout-results.sh`), which section 4.1 sums over the models that hold some.

## 8. The bar in the scorers (#1365)

**The decision** (section 5, Griffin on #1355, 2026-11-04): the fine tessellation decides pass or fail where both files write true curves, and nothing else changes. #1365 puts it in the scorers. Without a switch, `mesh_bodies_vs_ifc`, `family_extrusion_bodies_vs_ifc` and `saved_meshes_vs_ifc` now judge a body in the fine tessellation when both files write it as true curves, and in IfcOpenShell's default one otherwise. A saved mesh is triangles on rvt-rs's side, so it is never in the set and keeps the default bar: no saved-mesh, door or window verdict can move.

### 8.1 Which bodies are true curves

`tools/re/true_curves.py` reads each file's own representation items, the ones the arc census prints for Revit's body: the items of an element's `Body` representations, mapped items followed, of the openings that cut it (`HasOpenings`, which IfcOpenShell subtracts when it meshes the body) and of its aggregated parts. It gives each item one of four kinds:

| kind | items |
|---|---|
| curve | an `IfcExtrudedAreaSolid` (an `IfcExtrudedAreaSolidTapered` through its end profile too) over an `IfcCircleProfileDef` (or its hollow form), `IfcEllipseProfileDef` or `IfcRoundedRectangleProfileDef`; over a parameterised profile with a fillet or edge radius above zero (an L, U, T, I, C or Z shape, a hollow rectangle and the like: any attribute named for a radius); or over an arbitrary profile (with voids too) whose outline has an arc: an `IfcIndexedPolyCurve` with an `IfcArcIndex`, an `IfcCompositeCurve` or `IfcTrimmedCurve` on a circle or another conic, an `IfcCircle`, `IfcEllipse` or `IfcBSplineCurve`; a composite or derived profile of one of these; an `IfcRevolvedAreaSolid`, an `IfcSweptDiskSolid`, a swept area solid with such a profile or along such a directrix, an `IfcAdvancedBrep` with a face that is not on a plane, an `IfcSphere`, `IfcRightCircularCylinder` or `IfcRightCircularCone`; a Boolean result with such an operand |
| straight | an extrusion of a polygon or of a parameterised profile with no radius, an `IfcBlock`, an `IfcRectangularPyramid`, an `IfcBoundingBox`, a half space on a plane, a Boolean result of straight operands |
| mesh | an `IfcTessellatedItem` (a triangulated or polygonal face set), an `IfcFacetedBrep` (with voids too) or other manifold B-rep that is not advanced, a face-based or shell-based surface model, a Boolean result with such an operand |
| other | any item class this list does not name, and an outline or a profile of a class it does not name (an `IfcOffsetCurve2D`, an `IfcPcurve`, a profile type it does not know): it fails closed, the body is not one of true curves |

A file writes a body as true curves when it has at least one curve item and no mesh or other item (a straight item neither makes a body a curve nor stops it from being one: the dispenser's two rectangles do not take it out). A body is in the set only if **both** files write it so. A body that cannot be read, or that has an item of a class the list does not know, is not in the set: the default tessellation, the bar before #1365, decides it, so a gap in the list can only leave a body where it was. Reading the openings, the filleted profiles and the tapered end profile, and failing closed on an outline or a profile of a class it does not know, changed no body's membership on the exports at hand: Revit's RE1 files (4, 16, 6 and 70 bodies), Core Interior's slim export (0), the two terrain files (19 each) and the office as built and at permit (228 and 204 bodies), and rvt-rs's exports of the office as built (93) and of RE1 Plumbing (63) and Electrical (0), are the same sets before and after. The default export writes no family extrusion, so no body of a default-flag run has a curve on rvt-rs's side and the set of such a run is empty whatever the rule.

On Revit's own exports this finds, among the elements with a body: RE1 Architecture 4 (elements 360331, 370475, 377331 and 379016), Electrical 16, Mechanical 6 and Plumbing 70, the counts RE-327 section 11.1 gives for the circle profiles of those four files (its 5 for Architecture includes the opening element, which the scorers skip); in the office's concrete terrain file 19 (11 `IfcWasteTerminal` among them). None of those files' faceted bodies (16, 2, 6 and 53 in the first four files; 100 in the terrain file) is in it.

### 8.2 What the scorers print

When no body is decided by the fine tessellation the output is what it was before #1365. When some are:

- a line, `decided by the fine tessellation (both files write true curves): N of M bodies` (elements, for the box scorer);
- the class table of `mesh_bodies_vs_ifc` and of `saved_meshes_vs_ifc` gains a last column, `decided by fine` and `fine`, how many of each class (and under `mesh_bodies_vs_ifc`, of each Revit type) were decided so; the verdict columns count each body in its deciding tessellation;
- `mesh_bodies_vs_ifc` prints its `fine` rows (section 2) for those bodies, which show each of them in both tessellations: how many hold "ref on ours" and "0.001 ft" in the default tessellation, in the fine one, in both or in neither;
- its lists of the bodies furthest off end a body's line with `(fine tessellation)` when the fine tessellation decided it;
- the census of `family_extrusion_bodies_vs_ifc` ends a row `decided by the fine tessellation` when its bodies were, and an `all` row, which mixes the two, ends `N of them decided by the fine tessellation` when any was; its list of the bodies that miss ends a body's line with `(fine tessellation)` when the fine tessellation decided it; the rows of the other bodies are in the default tessellation and are as they were;
- `RVT_SCORER_FINE_MESH=1` still prints the `fine` rows for every body (section 2); the class table follows the bar either way.

The held-out scrub keeps the new line and the last column as counts beside an IFC class name.

**Cost.** Only the set is meshed finely: rvt-rs's side for the bodies it writes with a curve (a handful: the family extrusions with an arc), Revit's for those among them that Revit writes with a curve too. The default meshes and scores are made for every body as before. So the office's saved-mesh step takes what it took, and the fine pass costs the bodies of the set, not the 300 per class of `RVT_SCORER_FINE_MESH`.

### 8.3 Proved on files at hand

- The saved-mesh pairs: `mesh_bodies_vs_ifc` and `saved_meshes_vs_ifc` print, byte for byte, what the scorers of `main` print on Core Interior's 138 doors and windows and on the saved-mesh doors of RE1 Architecture, Electrical, Mechanical and Plumbing (10 of 10 files).
- A body both files write as true curves: RE1 Architecture's dispenser drawn as half arcs (section 3.3) against Revit's circle is decided by the fine tessellation, holds "ref on ours" (2.3e-6 ft) and prints `decided by the fine tessellation` on the class table and in the census; the box scorer decides the same element's box in the fine tessellation too.
- The same arcs against a Revit body that is a mesh (a faceted brep: the pair borrows another element's Tag only to put a mesh on Revit's side, so its distances mean nothing): no body is decided by the fine tessellation, and the output has no fine column, line or rows.

- The classification itself, on hand-made IFC4 entities in memory (a scratch check, not committed): 13 of 13 as intended. A circle extrusion is in the set and a rectangle extrusion is not; a circle with a straight opening is in and with a triangulated opening is not; a rectangle with a circular opening is in; an L shape with a fillet radius is in, with a radius of 0 or none is not; a tapered extrusion from a rectangle to a circle is in and from a rectangle to a rectangle is not; an outline that is an `IfcOffsetCurve2D` is not (it fails closed); a circle with a mesh item beside it is not.

### 8.4 What the filtered runs measured

Three runs, all green, every scorer step `exit 0`, the scorers of the branch (`--ref feat/1365-fine-bar`) unless said:

- **41871104420**: the new scorers on `f1f80d9e` (main when the branch merged it), the models and flags of the #1362 flag run (`core-interior re1-* einhoven nordic-office-* r2025-*`, `--saved-meshes --family-extrusions`, `-f nordic=true -f r2025=true`), the three scorers.
- **41871109552**: the same with `--ref main`, so that the scorers of `main` score the same exports. (`-f base=main` would not do: both sides are scored by the scorers of the workflow's ref, so it compares exports.)
- **41871114299**: the new scorers on `aaa38de8`, the first arc run's head, which draws every arc as an arc (`core-interior re1-* nordic-office-*`, `--saved-meshes --family-extrusions`, the family scorer only).

**The new scorers against main's, on the same exports** (41871104420 against 41871109552; 33 models with a Revit export, three scorers, 99 files):

- 97 files are the same byte for byte: all 66 files of `mesh_bodies_vs_ifc` and `saved_meshes_vs_ifc`, and 31 of the 33 of `family_extrusion_bodies_vs_ifc`. **No saved-mesh verdict moved**, the doors and windows among them.
- The two others are RE1 Architecture's and Plumbing's family files, which are where a body is decided by the fine tessellation: 1 of 7 (`Hook, Garment and Towel`, a circle of 0.12 ft or less) and 1 of 1 (`223313-WTRHTR-Tankless Electric:28kW`). Both hold "ref on ours" in both tessellations. Take out the additions (the line that counts them, the last column, the `(fine tessellation)` marks and the `fine` rows) and one line of each file differs from main's: the decided body's distances in the list of bodies furthest off (the hook's 3.279e-06 and 3.033e-06 ft in the default tessellation, 2.826e-06 and 3.477e-06 in the fine one; the water heater's 1.607e-06 and 1.607e-06 against 2.426e-06 and 2.176e-06). Every count and every verdict column is the same.
- So the family-extrusion rows moved only for curve-to-curve bodies, 2 of the 66 replaced, and only in the distances.

**The arcs** (41871114299, 251 bodies scored; against 41854514903, the #1362 scorers on the same code, whose default rows are the bar before #1365):

- **187 of the 251 are decided by the fine tessellation**: the 185 arcs over 0.12 ft that the current rule keeps as meshes (78 in each office file, 11 in each terrain file, 6 in RE1 Electrical, 1 in RE1 Architecture) and the 2 circles of 0.12 ft or less. The other 64 are decided by the default one; Core Interior's and Mechanical's files are the same byte for byte as the older run's.
- **All 251 hold "ref on ours" and both directions within 0.001 ft; 0 miss.** The dispenser is the one body whose verdict is not the older run's: it missed both in the default tessellation (0.001439 ft, volume ratio 1.0006) and holds in the fine one (5.6e-06 ft and 5.9e-06 ft, volume ratio 1.0000). RE1 Architecture's `IfcBuildingElementProxy` row goes from 2 to 3 of 3 on "ref on ours", "0.001 ft" and "vol .01%", and its census row `extrusion of circle + extrusion of rectangle; arcs over 0.12 ft` from 0 of 1 to 1 of 1.
- Elsewhere the files differ from the older run's only in the distances of the decided bodies in the lists of bodies furthest off: the terminals' stay under 2.5e-06 ft, and RE1 Electrical's occupancy sensors' are 3.8e-06 to 7.1e-06 ft where the default tessellation had 0.6e-06 to 2.3e-06.

**Time.** In the flag run the longest step is the office's `saved_meshes_vs_ifc`, 298 s on both office files (main's scorers on the same exports took 249 s; the two runs ran on different runners, and the detection alone takes 0.26 s over the 12,432 elements of an rvt-rs export of the office, timed on its own); `mesh_bodies_vs_ifc` took 26 s against 22 s, and `family_extrusion_bodies_vs_ifc` 13 and 16 s against 13 and 12 s. In the arcs run, with 187 bodies meshed finely, the family step took 45 s on each office file and 44 s on each terrain file, against 45 to 46 s for the #1362 scorers, which meshed every body finely. No step came near its limit (2,400 s).

### 8.5 The unfiltered run

Run 41875852229 (the new scorers, `-f ref=3a6ed928 -f base=none -f nordic=true -f r2025=true -f samples=true -f holdout=true`, default export flags) against the `measure-base` side of run 41869173374 (main's scorers and code at `3a6ed928`), per model, with the held-out models as counts only. Its figures go here when it finishes: whether any saved-mesh, door or window verdict moves, which rows change and where a body is curve-to-curve, and the steps' times on the office files.
