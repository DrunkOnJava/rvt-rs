# RE-341: Revit 2026 and 2027 beams, stair runs, landings and sketch lines; roof slopes left out

**Date:** 2026-11-04 (the contributor's measurements); maintainer changes and the Measure run, 2026-11-05.
**Issues:** #1190 (geometry beyond the box on 2015 to 2023, 2026 and 2027); follows RE-124 and RE-179 (2026 and 2027 element records are 2025's, with 2026's own layer records).
**Artefacts:** Autodesk's own sample projects, pinned in `research/autodesk-sample-projects.tsv` and fetched by `tools/fetch-autodesk-samples.sh`, never stored: `rac_advanced` 2024 (`ff791052…e5afc`), 2025 (`512e9188…e25d1`), 2026 (`f5fb3712…83ace`) and 2027 (`a88eb9f8…024ed`); `rac_basic` 2025 (`41f1893a…d4293`), 2026 (`bc547fe9…d99dc`) and 2027 (`c9ce4d01…91a01`); `rst_basic` 2024 (`aab838f8…c616e`), 2025 (`5089b2c6…31acb`), 2026 (`e7cf23a9…75418`) and 2027 (`01685786…d1453`). Revit's own held-out 2026 and 2027 exports (`research/holdout-models.tsv`), counts only.
**Scorers:** `tools/re/bodies_across_saves.py` (the gate Measure runs, section 4) and `tools/re/save_pair_bodies.py` (the contributor's box and volume check, not shape equality).
**Status:** positive for beams, stair runs, landings and the sketch-line readers against the same samples saved by Revit 2024 or 2025; **sloped roofs are left out, and roofs take neither sketch reader added for 2026 and 2027** (section 5). Against Revit's own export only one stair flight on each of two held-out models (a 2026 one and a 2027 one) is scored, as counts (section 4): its property values and quantities become equal to Revit's. **No beam, landing, stair run, sketch outline or shaft void of a 2026 or 2027 file is scored against Revit's own export.** Each is held to the same Autodesk project's earlier save by `tools/re/bodies_across_saves.py`, which Measure runs and fails on; the earlier save's export is measured against Revit's (RE-50, RE-56, #357, #824), and **the check cannot catch an error the earlier save shares with the 2026 or 2027 read.** Run 42010304828 (head e56d31b5) measured the check (section 4); the head after it, with `main` merged and the ceiling compared under its source, is **not yet read**.

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

Five lists gain 2026 and 2027: the beam axis list (`BEAM_AXIS_SUPPORTED_REVIT_VERSIONS`, 2024, 2026, 2027; 2025 is not measured), the sketch line list (`SKETCH_LINE_SUPPORTED_REVIT_VERSIONS`), the native stair run list, the run shape list and the landing list (`NATIVE_LANDING_REVIT_VERSIONS`, 2024, 2026, 2027; 2025 is not measured). The roof slope list (`ROOF_SLOPE_SUPPORTED_REVIT_VERSIONS`) stays 2024 and 2025 (section 5). A Sketch's curve list (RE-97) is read on 2026 and 2027 too (section 6), for floors, ceilings and slabs; a roof takes neither it nor the lines' exact ends (`roof_reads_exact_sketches`, section 5).

A wall's profile openings (RE-151) checked the beam reader's list, not one of their own. They are measured on 2024 only (Core Interior's wall 55840), so they have their own list, 2024, and take no openings on a release that only the beam reader gains.

**What else the widened lists reach.** The beam list also gates `scan_bounded_lines`, `scan_sketch_curves` and `scan_sketch_circles`, and the sketch line list `scan_sketch_bounded_lines`; those are what a sketch-driven element is read with. On 2026 and 2027, floors, ceilings and slabs whose lines the record-box solve does not close take the lines' exact ends, curves and circles as on 2024 (RE-50, RE-95, RE-96, RE-100); and shaft openings are read (RE-99, RE-100) and cut from the floors and ceilings their height passes through. A roof is held back from all three, as are the roofs' shaft voids. The body sources do not name these bodies (a floor drawn from its outline keeps `partition_element_record_bbox`), so the body source counts of section 4 do not include them; `tools/re/bodies_across_saves.py` does.

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
- **`rac_advanced` 2026:** 10 stair runs become 2025's. 9 bodies stop being 2025's and are 2024's instead: 2 beams and 5 landings, which 2025 still draws as boxes, and a floor (139803) that the sketch lines now draw as on 2024 (2025's is 0.60 m³ smaller) and a ceiling (147534) the contributor counted as drawn as on 2024 too; the 2024 save draws that ceiling as its record box (section 4). The rest left on `rac_advanced` and `rac_basic` are walls and coverings, as on `main`.

2027's counts are 2026's on every sample.

## 4. Measure, and the check against the same project's earlier save

**Run 41988011289** (the first head, 1d5bbf03d9d189cff773484ceaddb340ba4e828e; base `main` 182ed219; the held-out, Autodesk sample, BIM4LCA office and R2025 sets; held-out models give counts only). Maintainer-measured.

- **Which exports move.** Of 111 exports with an IFC hash, 102 are byte for byte `main`'s and 9 differ, all 2026 or 2027: the six Autodesk samples (`rac_advanced`, `rac_basic` and `rst_basic`, each 2026 and 2027) and three held-out models (a 2026 one, a 2027 one and a third). The held-out 2026 and 2027 office models are identical.
- **Revit's own held-out 2026 and 2027 exports.** On each, one stair flight leaves the record-box set (`box bodies` 60 to 59). Property values equal Revit's: 2026 209 to 224 of 352, 2027 209 to 224 of 354 (14 in `Pset_StairCommon` and `Pset_StairFlightCommon`, one more `Reference`); different 0 and values Revit does not write 0, unchanged. Quantities equal: 2026 21 to 23 of 322, 2027 17 to 19 of 367 (`Qto_StairFlightBaseQuantities.Length` and `NetVolume`); different 1 and values Revit does not write 0, unchanged.
- **MCP consistency.** On the three held-out models that moved, 17 consistency lines each, 0 differ on 1,202, 887 and 887 values compared.

**Run 42004827803** (head f9035152, which leaves roof slopes out; base `main` 182ed219). The main session's reading.

- **Which exports move.** 102 of 111 identical; 9 move: the six Autodesk samples, the third held-out model and the held-out 2026 and 2027 models. No release before 2026 moves.
- **MCP consistency.** 0 differ on 629 lines.
- **IFC rule findings** on the 2026 and 2027 samples: `rac_advanced` 2 each and `rst_basic` 14 each, from the fixed-reference sweep (`IfcShapeRepresentation.CorrectItemsForType` on an `IfcFixedReferenceSweptAreaSolid`). #1432 fixes that writer; they should be gone once it lands, and have not been measured so.
- **Body sources**, base, then head, then the same project's 2024 save's head (`exported.body_sources`):

| body source | `rac_advanced` 2026 and 2027 | `rst_basic` 2026 and 2027 | `rac_basic` 2026 and 2027 |
|---|---|---|---|
| `partition_beam_axis` | 0, 29, 29 | 0, 345, 345 | |
| `partition_beam_box_along_axis` | 0, 8, 8 | 0, 9, 9 | |
| `native_landing_tread` | 0, 5, 5 | | |
| `partition_stair_run_sketch` | 0, 10, 5 | 0, 1, 1 | 0, 2, 1 |
| `partition_landing_slab` | | | 0, 0, 1 |
| `partition_roof_slope` | 0, 0, 1 | | |
| `partition_element_record_bbox` | 5,205, 5,153, 5,157 | 428, 73, 73 | 330, 328, 328 |

  The stair run counts differ from the 2024 save's because its byte-anchor reader finds 5 of `rac_advanced`'s 10 runs and 1 of `rac_basic`'s 2; the 2025 save draws all of them (10 and 2). The slab and ceiling body source counts do not move between base and head on `rac_advanced`, and equal the 2024 save's: a count match, not an element by element one.
- **Held-out.** The two stair models each lose one `IfcStairFlight` from the record-box list (60 to 59), and the third loses its `IfcRoof` (45 to 44), with the roof slope list already reverted: section 5.

**The check.** `tools/re/bodies_across_saves.py` runs in Measure's sample shard, which keeps the exports of the 2024 to 2027 saves of `rac_advanced`, `rac_basic` and `rst_basic` until it has run (`tools/ci/measure-reference-models.sh`), and `tools/ci/check-bodies-across-saves.sh` turns a failure on the head side into a red run after the results are uploaded. Each 2026 and 2027 save is held to the same project's earlier saves by `Tag`:

- **Beams and landings** (`partition_beam_axis`, `partition_beam_box_along_axis`, `native_landing_tread`, `partition_landing_slab`) and **sketch outlines** (an `IfcSlab`, `IfcCovering` or `IfcRoof` whose plan outline was recovered, `ProfileResolved`, including the shaft voids cut from them) to the **2024 save**. The 2025 save cannot be their reference: its body sources hold no beam or landing source, so it draws them as record boxes (the 2025 diagnostics of run 41988011289).
- **Stair runs** (`partition_stair_run_sketch`) to the **2025 save**, which reads runs from native records as 2026 and 2027 do and draws all 10 of `rac_advanced`'s and 2 of `rac_basic`'s; the 2024 save draws 5 and 1.
- **Fields**, any difference failing: IFC class, GlobalId, storey, `BodySource`, representation types and solid kinds, placement, body volume (1e-6 relative) and vertices (each within 0.001 ft of one of the other's, both ways). `RepresentationType` is compared too: both saves come from the same build, so #1432's change to it does not show.
- **Also failing:** an element with no counterpart, one the earlier save draws otherwise, one of the earlier save's that is missing here altogether, a missing export, fewer bodies than the sample is known to hold (`MINIMUMS`: `rac_advanced` 29 axes, 8 along them, 5 landings, 10 runs; `rst_basic` 345, 9 and 1; `rac_basic` 2 runs), `rac_advanced`'s floor 139803 not compared equal as an outline and its ceiling 147534 not compared equal under whatever source each save gives it (the check prints that source and the volume in every save), fewer sketch outlines compared than run 42010304828 saw (`OUTLINE_LEAST`: 82, 12 and 5), and a check that compared nothing. An element the earlier save draws and this one leaves a record box is counted per class, not scored.
- **What it cannot catch:** an error the earlier save shares with the 2026 or 2027 read, and a project edited between saves the same way in both. No Revit export is behind it.

Its output is `bodies_across_saves.txt` beside each sample. It was first exercised on synthetic IFC pairs (a beam 1 mm deeper, one shifted by a metre, a body source that differs, a body left out, a missing 2025 save, a named element drawn as an outline here and a box there): each fails and the identical pair passes.

**Run 42010304828** (head e56d31b5; filtered: samples on, held-out models only; the head side's check turned the run red). Per 2026 and 2027 sample, the two years alike:

| sample | beams and landings against 2024 | stair runs against 2025 | sketch outlines against 2024 | left as boxes, drawn by the earlier save | verdict |
|---|---|---|---|---|---|
| `rst_basic` | 354 `IfcBeam` | 1 `IfcStairFlight` | 5 `IfcSlab` | 0 | PASS |
| `rac_basic` | 0 | 2 `IfcStairFlight` | 12 (2 `IfcCovering`, 10 `IfcSlab`) | 1 `IfcSlab` (the landing slab the 2024 save draws), 1 `IfcRoof` | PASS |
| `rac_advanced` | 42 (37 `IfcBeam`, 5 `IfcSlab`) | 10 `IfcStairFlight` | 82 (64 `IfcCovering`, 1 `IfcRoof`, 17 `IfcSlab`), floor 139803 among them | 4 `IfcRoof` | FAIL: ceiling 147534 required and not compared |

Every body compared was equal in every field, and nothing else failed. On the base side (`main`) the same check fails as it should: `rac_advanced` 2026 compares no beam, landing or stair run (42 and 10 left as boxes, fewer than the 29, 8, 5 and 10 drawn that it holds), floor 139803 differs in volume (440.512245 m³ here, 441.111228 m³ in the 2024 save), and 147534 is drawn as an outline where the 2024 save draws its record box (`partition_element_record_bbox`).

**The ceiling 147534.** The 2024 save draws it as its record box. At base the 2026 save drew an outline (no Sketch curve list was read there, so nothing held an outline made of lines the Sketch does not list to the record box). On the head it is neither compared as an outline nor counted as left out, so the 2026 and 2027 saves no longer draw it as an outline either: the guard that keeps such an outline only where it spans the record box (`profiles.retain` after `relisted` in `solve_sketch_plan_profiles`) now fires there as on 2024. That reading is an inference from these counts. The check now compares the ceiling under the source each save gives it and prints, in every save, its class, `BodySource`, `ProfileResolved` and volume (`named 147534` lines), so a run of the current head states it.

## 5. Sloped roofs are left out, and roofs keep the record-box solve

The first head widened the roof slope list to 2026 and 2027. The current head does not, for four reasons.

1. **The comment was wrong and the report was right.** `rac_advanced`'s sloped roof on 2026 and 2027 is its 2024 save's body (532 triangles, 5.33 m³) and not its 2025 save's (12 triangles, 6.88 m³). The two saves disagree and no Revit export scores either: the only Revit export of the sample is an IFC2X3 export of its 2015 to 2020 state (`research/rac-advanced-export.tsv`), which predates both saves.
2. **The edge reader forks on release and the 2026 and 2027 edges take the 2024 arm.** On 2025 the f64 at +0x3b of an edge that defines the slope is how far the edge's plane is raised; on 2024 it is −rise/run, and the edge is read only where that equals −tan of its angle within 1e-9. The fascia depth (`scan_fascia_depths`), the rafter cut (`scan_rafter_cuts`) and the several-arrow roofs (RE-307) are read on 2025 only. So on 2026 and 2027 the plate offset is 0 and none of a fascia, a two-cut rafter or several arrows can be told. The R2025 models show each exists (RE-199, RE-307), and a single-arrow shed with a fascia or a two-cut rafter would be drawn as a plain sloped slab with its eave cut missing. The record-box height check (`ROOF_SLOPE_CLOSURE_FEET`) rejects most of those but is the only guard.
3. **Nothing scores a 2026 or 2027 roof against Revit's export.** The held-out digest (`tools/ci/scrub-holdout-results.sh`) keeps no `roof_bodies_vs_ifc` line.
4. **The rule.** A rule right on most elements but wrong on some is narrowed until it is never wrong. With the fascia and rafter cut unreadable on these releases, every roof is a roof whose cut cannot be ruled out, so the narrowing is the whole list.

**The third held-out model's roof.** At head f9035152 (run 42004827803), with the slope list reverted, its `IfcRoof` still leaves the record-box list (`box bodies` 45 to 44), so the slope did not draw it. Revit's roof volume there is 14.2% of its record box, so it is very likely sloped. What the code draws for a roof whose slope is not read and whose sketch outline is recovered is that outline extruded to the record box's **full height** (the record body with `profile_override`, `export_content.rs`): a level block at the roof's highest point. That is the body 2024 and 2025 ship for such a roof (RE-50, RE-98; the support matrix says roofs sloped some other way "stay level plates of their record box's height", and #884 is the hip roofs over a T or U footprint), with its slab quantities withheld unless the type's layers fill the body (RE-298); it was measured there as a plan outline against Revit's upward-face area, never as a volume. Nothing measures it on 2026 or 2027, and for a sloped roof it is not Revit's body: **a guess, not proof.** So on 2026 and 2027 a roof takes neither the Sketch curve list nor the lines' exact ends (`roof_reads_exact_sketches`) and no shaft void is cut from it; it is read as on `main`. In run 42010304828 (head e56d31b5) that model's export is byte for byte `main`'s (`ifc.sha256` equal) and its `IfcRoof` is a record box again (`box bodies` row `IfcRoof 1`, fill 0.142): the sketch readers moved it, and holding roofs out returns it. On `rac_advanced` the one `IfcRoof` compared under the sketch outlines and the 4 left as boxes are the same on the base side and the head side, so roofs there are on the reader `main` has. `bodies_across_saves.py` does not reach held-out models.

So `ROOF_SLOPE_SUPPORTED_REVIT_VERSIONS` is 2024 and 2025 again, and the 2025 forks (`== 2025`) are untouched. `rac_advanced`'s 2026 and 2027 roof equalling its 2024 save's is kept as a finding. Open: why the 2025 save's roof differs from the 2024 save's (not investigated).

## 6. A Sketch's curve list on 2026 and 2027

Widening the beam and sketch line lists made the exact line ends readable on 2026 and 2027, but the reader of a Sketch's curve list (RE-97, `sketch_list_copies_at`) stayed `2024 | 2025`. On 2026 and 2027 its lists were empty, so the guard in `solve_sketch_plan_profiles` for an element whose Sketch lists other curves than those naming it (it keeps an outline only where it spans its record box) could not fire, and a shaft took its membership from the lines' owner references alone. The current head reads the lists on 2026 and 2027 for floors, ceilings and slabs. A list is taken only where it validates (RE-97: the sketched element at +77 is not a sketch line, at least 3 listed ids that are sketch lines, keys rising), so a release whose Sketch layout differs gets none, as before. Not yet measured: whether `bodies_across_saves.py` finds `rac_advanced`'s ceiling and floor equal to the 2024 save's with it.

## 7. Not measured

- A beam, landing, stair run, sketch outline, shaft void or sloped roof of a 2026 or 2027 file against Revit's own export: none is scored. The held-out office models are unchanged; the only held-out elements that move are one stair flight each and the roof of section 5.
- The current head, after run 42010304828's head e56d31b5: the ceiling 147534 compared under its source, `tools/re/save_pair_bodies.py`'s counts of bodies never paired.
- How many floors, ceilings and slabs the sketch readers change on 2026 and 2027: their body sources do not differ from the box's. The check counts the ones it compares.
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

Runs 41988011289 and 42004827803 were dispatched with the Measure command above, at 1d5bbf03 and f9035152. The check is part of Measure's sample shard from the head that adds it; a run of that head names its own commit.
