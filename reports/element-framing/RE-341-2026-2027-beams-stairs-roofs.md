# RE-341: Revit 2026 and 2027 beams, stair runs, landings and sketch lines; roof slopes left out

**Date:** 2026-11-04 (the contributor's measurements); maintainer changes and the Measure run, 2026-11-05.
**Issues:** #1190 (geometry beyond the box on 2015 to 2023, 2026 and 2027); follows RE-124 and RE-179 (2026 and 2027 element records are 2025's, with 2026's own layer records).
**Artefacts:** Autodesk's own sample projects, pinned in `research/autodesk-sample-projects.tsv` and fetched by `tools/fetch-autodesk-samples.sh`, never stored: `rac_advanced` 2024 (`ff791052…e5afc`), 2025 (`512e9188…e25d1`), 2026 (`f5fb3712…83ace`) and 2027 (`a88eb9f8…024ed`); `rac_basic` 2025 (`41f1893a…d4293`), 2026 (`bc547fe9…d99dc`) and 2027 (`c9ce4d01…91a01`); `rst_basic` 2024 (`aab838f8…c616e`), 2025 (`5089b2c6…31acb`), 2026 (`e7cf23a9…75418`) and 2027 (`01685786…d1453`). Revit's own held-out 2026 and 2027 exports (`research/holdout-models.tsv`), counts only.
**Scorer:** `tools/re/save_pair_bodies.py` (a box and volume check, not shape equality).
**Status:** positive for beams, stair runs, landings and the sketch-line readers against the same samples saved by Revit 2024 or 2025; **sloped roofs are left out** (section 5). Against Revit's own export only one stair flight on each of two held-out models (a 2026 one and a 2027 one) is scored, as counts (section 4): its property values and quantities become equal to Revit's. **No beam, landing, sketch outline, shaft void or roof of a 2026 or 2027 file is scored against Revit's own export**; for those the oracle is the same model saved by an earlier release, whose export is measured against Revit's (RE-50, RE-56, #357, #824). The head that leaves roofs out and reads Sketch curve lists on 2026 and 2027 (section 6) is **not yet measured**.

## 1. Before

On `main`, 2026 and 2027 files get the 2025 records (RE-124, RE-179; a 2026 layer record is 4 bytes longer): walls with their layers and joins, family instances' boxes and saved meshes are the same as on 2025. Six readers are still limited to 2024 or 2025 by their release lists, so on 2026 and 2027 these are drawn as their record boxes:

| reader | release list on `main` |
|---|---|
| a beam's axis (`partition_beam_axes`, RE-49) | 2024 |
| a sketch line's segment (`scan_sketch_bounded_lines`, RE-50, RE-199) | 2024, 2025 |
| a stair run from its native records (`native_stairs`, #357) | 2025 |
| a stair run's shape (#323) | 2024, 2025 |
| a landing's top and thickness (#824) | 2024 |
| a roof's slope (`partition_roof_slopes`, RE-56) | 2024, 2025 |

Every body source the samples' exports report (`exported.body_sources`) on `main` (the contributor's local counts; section 4 has Measure's, on `main` 182ed219):

| body source | `rac_advanced` 2024 / 2025 / 2026 / 2027 | `rac_basic` 2025 / 2026 / 2027 | `rst_basic` 2024 / 2026 / 2027 |
|---|---|---|---|
| `partition_beam_axis` | 29 / 0 / 0 / 0 | | 345 / 0 / 0 |
| `partition_beam_box_along_axis` | 8 / 0 / 0 / 0 | | 9 / 0 / 0 |
| `partition_stair_run_sketch` | 5 / 10 / 0 / 0 | 2 / 0 / 0 | 1 / 0 / 0 |
| `native_landing_tread` | 5 / 0 / 0 / 0 | | |
| `partition_roof_slope` | 1 / 1 / 0 / 0 | | |

## 2. The change

Five lists gain 2026 and 2027: the beam axis list (`BEAM_AXIS_SUPPORTED_REVIT_VERSIONS`, 2024, 2026, 2027; 2025 is not measured), the sketch line list (`SKETCH_LINE_SUPPORTED_REVIT_VERSIONS`), the native stair run list, the run shape list and the landing list (`NATIVE_LANDING_REVIT_VERSIONS`, 2024, 2026, 2027; 2025 is not measured). The roof slope list (`ROOF_SLOPE_SUPPORTED_REVIT_VERSIONS`) stays 2024 and 2025 (section 5). A Sketch's curve list (RE-97) is read on 2026 and 2027 too (section 6).

A wall's profile openings (RE-151) checked the beam reader's list, not one of their own. They are measured on 2024 only (Core Interior's wall 55840), so they have their own list, 2024, and take no openings on a release that only the beam reader gains.

**What else the widened lists reach.** The beam list also gates `scan_bounded_lines`, `scan_sketch_curves` and `scan_sketch_circles`, and the sketch line list `scan_sketch_bounded_lines`; those are what a sketch-driven element is read with. On 2026 and 2027, floors, ceilings and slabs, and footprint roofs (a roof's outline, not its slope), whose lines the record-box solve does not close take the lines' exact ends, curves and circles as on 2024 (RE-50, RE-95, RE-96, RE-100); and shaft openings are read (RE-99, RE-100) and cut from the floors, roofs and ceilings their height passes through. The body sources do not name these bodies, so section 4's counts do not include them.

2025 is not widened. A 2025 file's beams and landings would change as well, and the only Revit 2025 exports here that hold them are local, unpublished exports, so they cannot measure it.

## 3. The 2026 and 2027 saves against the earlier ones

The contributor's local measurement, on `main` 77d71cbb and the branch head before the maintainer changes; not re-run on the current head. `rvt-ifc FILE -o out.ifc` on each save, scored with `tools/re/save_pair_bodies.py` against the save named. Elements are paired by ElementId (`Tag`), and a pair is the same where the world boxes agree to 3 mm and the volumes to 0.1%. That is a box and volume check, not shape equality: a mirrored body, or one whose volume sits differently inside the same box, agrees. The script then dropped a shape that did not mesh and kept the last of a repeated `Tag` without saying so, so the number of bodies these runs never paired is not known; it now counts and prints both and exits 1 on either (not yet re-run).

**The bodies these readers now draw**, against the save that draws them on `main`:

| candidate | reference | bodies | same |
|---|---|---:|---:|
| `rst_basic` 2026 and 2027 | `rst_basic` 2024 | 355 each (345 beams by their axis, 9 along it, 1 stair run) | 355 each |
| `rac_advanced` 2026 and 2027 | `rac_advanced` 2024 | 43 each (37 beams, 5 landings, 1 roof) | 43 each |
| `rac_advanced` 2026 and 2027 | `rac_advanced` 2025 | 10 stair runs each | 10 each |
| `rac_basic` 2026 and 2027 | `rac_basic` 2025 | 2 stair runs each | 2 each |

The roof in the second row is not drawn on 2026 and 2027 by the current head (section 5); the other 42 are.

`rac_advanced`'s 5 other stair runs are drawn from their sketch on 2025 and as their boxes on 2024, so they are scored against 2025. On `main` the 2025 save's sloped roof (201262) is already not its 2024 save's: 12 triangles and 6.88 m³, against 532 triangles and 5.33 m³. The 2026 and 2027 roofs, drawn on the first head, were the 2024 body.

**Every body in the file**, against the reference save, `main` then the first head (with the roof):

| candidate | reference | different on `main` | different here |
|---|---|---:|---:|
| `rst_basic` 2026 | `rst_basic` 2024 | 35 of 453 | 1 of 453 |
| `rst_basic` 2027 | `rst_basic` 2024 | 35 of 453 | 1 of 453 |
| `rac_advanced` 2026 | `rac_advanced` 2025 | 64 of 5,527 | 63 of 5,527 |
| `rac_advanced` 2027 | `rac_advanced` 2025 | 64 of 5,527 | 63 of 5,527 |
| `rac_basic` 2026 | `rac_basic` 2025 | 27 of 438 | 23 of 438 |
| `rac_basic` 2027 | `rac_basic` 2025 | 27 of 438 | 23 of 438 |

Which bodies change, element for element:

- **`rst_basic` 2026 and 2027:** 34 become 2024's (32 beams, a slab and a stair run) and none stops being it. The one left is a column, as on `main`.
- **`rac_basic` 2026:** 4 become 2025's (2 stair runs and 2 slabs drawn from their sketch) and none stops being it.
- **`rac_advanced` 2026:** 10 stair runs become 2025's. 9 bodies stop being 2025's and are 2024's instead: 2 beams and 5 landings, which 2025 still draws as boxes, and a ceiling (147534) and a floor (139803) that the sketch lines now draw as on 2024 (2025's are 0.99 m³ and 0.60 m³ smaller). The rest left on `rac_advanced` and `rac_basic` are walls and coverings, as on `main`.

2027's counts are 2026's on every sample.

## 4. Measure run 41988011289

Maintainer-measured. Dispatched at ref 1d5bbf03d9d189cff773484ceaddb340ba4e828e (the first head), base `main` 182ed219, with the held-out, Autodesk sample, BIM4LCA office and R2025 sets. Held-out models give counts only.

- **Which exports move.** Of 111 exports with an IFC hash, 102 are byte for byte the same as `main`'s and 9 differ, all 2026 or 2027: the six Autodesk samples (`rac_advanced`, `rac_basic` and `rst_basic`, each 2026 and 2027) and three held-out models (a 2026 one, a 2027 one and a third, 2026 or 2027). The held-out 2026 and 2027 office models are identical.
- **Body sources** (`exported.body_sources`, public samples). `rac_advanced` 2026 and 2027: `partition_element_record_bbox` 5,205 to 5,152; 53 bodies leave the box, which are 29 beams by their axis, 8 along it, 5 landings, 10 stair runs and 1 sloped roof. `rac_basic`: 330 to 328, 2 stair runs. `rst_basic`: 428 to 73, 355 bodies (345, 9 and 1). 2027's equal 2026's.
- **Revit's own held-out 2026 and 2027 exports.** On each, one stair flight leaves the record-box set (`box bodies` 60 to 59, the `IfcStairFlight` row gone). Property values equal Revit's: 2026 209 to 224 of 352, 2027 209 to 224 of 354 (14 in `Pset_StairCommon` and `Pset_StairFlightCommon`, one more `Reference`); different 0 and values Revit does not write 0, both unchanged. Quantities equal: 2026 21 to 23 of 322, 2027 17 to 19 of 367 (`Qto_StairFlightBaseQuantities.Length` and `NetVolume`); different 1 and values Revit does not write 0, unchanged.
- **The third held-out model.** Its only change in the digest is that one `IfcRoof` leaves the record-box bodies (`box bodies` 45 to 44). That is the first head's sloped roof; section 5.
- **MCP consistency.** On the three held-out models that moved, 17 consistency lines each, 0 differ on 1,202, 887 and 887 values compared.
- **IFC rule check** (`schema.txt`, IfcOpenShell's EXPRESS WHERE rules). `rac_advanced` 2026 and 2027 go from 0 to 2 findings and `rst_basic` 2026 and 2027 from 0 to 14, all `IfcShapeRepresentation.CorrectItemsForType` on an `IfcFixedReferenceSweptAreaSolid`. They are the sloped-beam findings the 2024 saves of the same samples carry (the main session's reading; #1432 fixes them).

## 5. Sloped roofs are left out

The first head widened the roof slope list to 2026 and 2027. The current head does not, for four reasons.

1. **The comment was wrong and the report was right.** `rac_advanced`'s sloped roof on 2026 and 2027 is its 2024 save's body (532 triangles, 5.33 m³) and not its 2025 save's (12 triangles, 6.88 m³). The two saves disagree and no Revit export scores either: the only Revit export of the sample is an IFC2X3 export of its 2015 to 2020 state (`research/rac-advanced-export.tsv`), which predates both saves.
2. **The edge reader forks on release and the 2026 and 2027 edges take the 2024 arm.** On 2025 the f64 at +0x3b of an edge that defines the slope is how far the edge's plane is raised; on 2024 it is −rise/run, and the edge is read only where that equals −tan of its angle within 1e-9. The fascia depth (`scan_fascia_depths`), the rafter cut (`scan_rafter_cuts`) and the several-arrow roofs (RE-307) are read on 2025 only. So on 2026 and 2027 the plate offset is 0 and none of a fascia, a two-cut rafter or several arrows can be told. The R2025 models show each exists (RE-199, RE-307), and a single-arrow shed with a fascia or a two-cut rafter would be drawn as a plain sloped slab with its eave cut missing, a body that looks right and is not. The record-box height check (`ROOF_SLOPE_CLOSURE_FEET`) rejects most of those but is the only guard.
3. **Nothing scores a 2026 or 2027 roof against Revit's export.** The third held-out model's roof left the box on the first head; the held-out digest (`tools/ci/scrub-holdout-results.sh`) keeps no `roof_bodies_vs_ifc` line, so nothing says what replaced it.
4. **The rule.** A rule right on most elements but wrong on some is narrowed until it is never wrong. With the fascia and rafter cut unreadable on these releases, every roof is a roof whose cut cannot be ruled out, so the narrowing is the whole list.

So `ROOF_SLOPE_SUPPORTED_REVIT_VERSIONS` is 2024 and 2025 again, and the 2025 forks (`== 2025`) are untouched. The roof measurement above is kept as a finding: `rac_advanced`'s roof 2026 and 2027 equals its 2024 save's. Open: why the 2025 save's roof differs from the 2024 save's (not investigated). A roof's state on the current head is not yet measured.

## 6. A Sketch's curve list on 2026 and 2027

Widening the beam and sketch line lists made the exact line ends readable on 2026 and 2027, but the reader of a Sketch's curve list (RE-97, `sketch_list_copies_at`) stayed `2024 | 2025`. On 2026 and 2027 its lists were empty, so the guard in `solve_sketch_plan_profiles` for an element whose Sketch lists other curves than those naming it (it keeps an outline only where it spans its record box) could not fire, and a shaft took its membership from the lines' owner references alone. The current head reads the lists on 2026 and 2027. A list is taken only where it validates (RE-97: the sketched element at +77 is not a sketch line, at least 3 listed ids that are sketch lines, keys rising), so a release whose Sketch layout differs gets none, as before. Not yet measured: whether it keeps the bodies counted in section 3 (the ceiling and floor of `rac_advanced` that the sketch lines draw as 2024's), and whether any 2026 or 2027 Sketch holds the layout.

## 7. Not measured

- A beam, landing, sketch outline, shaft void or sloped roof of a 2026 or 2027 file against Revit's own export: none is scored. The held-out office models are unchanged; the only held-out elements that move are one stair flight each and the roof of section 5.
- The current head: roofs left out, Sketch curve lists read on 2026 and 2027, `tools/re/save_pair_bodies.py`'s counts of bodies never paired.
- How many floors, ceilings, slabs, footprint roofs and shaft voids change on 2026 and 2027: their body sources do not differ from the box's, so the counts in section 4 do not name them.
- 2025's beams (RE-49) and landings (#824): left for a Revit 2025 export that can be published.
- 2015 to 2023: these readers stay at 2024 and later.
- A wall's profile openings on 2025 to 2027 (RE-151): left at 2024.

## 8. Reproduce

```bash
tools/fetch-autodesk-samples.sh samples 2024 2025 2026 2027
for f in samples/*.rvt; do rvt-ifc "$f" -o "${f%.rvt}.ifc"; done
python3 tools/re/save_pair_bodies.py samples/2027-rstbasicsampleproject.ifc samples/2024-rstbasicsampleproject.ifc
python3 tools/re/save_pair_bodies.py samples/2026-racadvancedsampleproject.ifc samples/2024-racadvancedsampleproject.ifc \
    --sources partition_beam_axis,partition_beam_box_along_axis,native_landing_tread
gh workflow run measure.yml --ref main -f ref=1d5bbf03d9d189cff773484ceaddb340ba4e828e -f base=main \
    -f nordic=true -f r2025=true -f holdout=true -f samples=true
```

The Measure command is the first head's (run 41988011289, section 4); a run of the current head names its own commit.
