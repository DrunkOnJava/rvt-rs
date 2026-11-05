# RE-345: Revit 2025 beams, tread landings and sketch arcs, held to the 2024 save

**Date:** 2026-11-05.
**Issues:** #1440; follows #1426 and RE-341 (the same readers on 2026 and 2027), RE-49 (beam axes), RE-96 and RE-100 (sketch arcs and circles), RE-99 (shaft openings) and #824 (tread landings).
**Artefacts:** Autodesk's own sample projects, pinned in `research/autodesk-sample-projects.tsv` and fetched by Measure, never stored: `rac_advanced` 2024 (`ff791052…e5afc`) and 2025 (`512e9188…e25d1`), `rac_basic` 2024 (`bb62c161…45ee55`) and 2025 (`41f1893a…d4293`), `rst_basic` 2024 (`aab838f8…c616e`) and 2025 (`5089b2c6…31acb`). For the Revit-export side: puzzbobb's 23 Revit 2025 models (`research/r2025-models.tsv`) and RE1 Architecture (Revit 2025), and the held-out models (counts only).
**Scorers:** `tools/re/bodies_across_saves.py` (the same-project gate, section 3), the suite's scorers on the R2025 models and RE1, and `probe_re96_sketch_arcs`, `probe_824_slab_arcs` and `probe_re99_shaft_openings` for the arcs and shafts.
**Status:** positive for beams and tread landings on `rac_advanced` and `rst_basic`, and for the sketch arcs of `rac_advanced`, **against the same projects' 2024 saves only**. **No Revit export checks any of it on 2025.** The 24 models with Revit's own 2025 exports (the 23 R2025 models and RE1 Architecture) hold no beam, no tread landing, no landing slab, no shaft opening and no floor, ceiling or roof whose sketch holds an arc or circle (section 4), so none of them scores a beam, a landing, an arc or a shaft, and their exports are byte for byte `main`'s. The same-project gate cannot catch an error the 2024 save shares with the 2025 one (section 3). Of the elements whose sketch holds an arc, the gate compares **two** as drawn outlines on 2025 (rac_advanced's floor 139803 and roof 201262), plus a ceiling and a railing that stay record boxes in both saves (section 4). On run 42062038214 (head 6ea88323, this PR's code), none of the 19 held-out and spent models, three of them Revit 2025 files, changes (section 5).

## 1. Before

On `main`, `BEAM_AXIS_SUPPORTED_REVIT_VERSIONS` and `NATIVE_LANDING_REVIT_VERSIONS` were `[2024, 2026, 2027]` (#1426), so a Revit 2025 save drew its beams (`partition_beam_axis`, `partition_beam_box_along_axis`) and tread landings (`native_landing_tread`) as record boxes. The same beam list also gated three sketch readers, so a 2025 save read a sketch line's record only as a line:

| reader | what it does | gated by |
|---|---|---|
| `scan_sketch_curves` (RE-96) | reads a sketch line's record as an arc where it reads as one inside its record box and not as a line | the beam list |
| `scan_sketch_circles` (RE-100) | reads a full circle | the beam list |
| the shaft line reader (`scan_bounded_lines` in `shaft_outlines`, RE-99) | the exact ends of a shaft's lines | the beam list |

Without the arc reader the record-box solve closes a sketch whose line is an arc with the arc's chord. rac_advanced's floor 139803 is such a sketch.

## 2. The change

- `BEAM_AXIS_SUPPORTED_REVIT_VERSIONS` and `NATIVE_LANDING_REVIT_VERSIONS` gain 2025. On 2025 only a tread landing is drawn: a monolithic landing's lines are read through `scan_landing_lines`, which reads 2024 only, so it stays its box (`rac_basic`'s one monolithic landing slab).
- The arc, circle and shaft line readers get their own list, `SKETCH_CURVE_SUPPORTED_REVIT_VERSIONS`, now `[2024, 2025, 2026, 2027]`, so each reader's release is a decision of its own; `shaft_outlines` follows it. Run A (section 3) measured the beams and landings with that list at `[2024, 2026, 2027]`, run B with 2025 in it.
- `tools/re/bodies_across_saves.py` holds a 2025 save to the 2024 save, with `REQUIRED` and `OUTLINE_LEAST` per candidate year; `tools/ci/check-bodies-across-saves.sh`, `tools/ci/measure-reference-models.sh` and `tools/ci/same_project_verdict.py` expect the 2025 samples.

## 3. The 2025 samples against their 2024 saves

The gate pairs each 2025 save's beams, landings and sketch outlines by `Tag` with the 2024 save's and fails on any difference in class, GlobalId, storey, `BodySource`, `ProfileResolved`, representation, placement (0.001 ft), volume (1e-6 relative) and vertices (0.001 ft, both ways); `tools/re/bodies_across_saves.py` has the rest. It compares bodies, not quantities or property sets, and **it cannot catch an error the 2024 save shares with the 2025 save** (a reading wrong in the same way on both, a project edited the same way between saves): nothing in it is a Revit export.

**Run A** (42056591627, head 382934a3; beams and landings on 2025, the arc readers still off): beams and landings 0 differences, outlines 3.

| sample | beams and landings against 2024 | sketch outlines against 2024 | verdict |
|---|---|---|---|
| `rac_advanced` | 42 equal (37 `IfcBeam`, 5 `IfcSlab` tread landings) | 86 compared, 3 differ | FAIL |
| `rst_basic` | 354 `IfcBeam` equal | 5 `IfcSlab` equal | PASS |
| `rac_basic` | none to compare; its monolithic landing slab (1 `IfcSlab`) stays a box, counted | 13 equal | PASS |

The 3 outlines are the only `rac_advanced` elements that differ, and each is an element whose sketch holds an arc that the 2025 save drew as a chord: floor 139803 (440.512245 m3 against 441.111228 m3, 24 vertices against 170, up to 9.69 ft apart), roof 201262 (6.87512864 m3 against 5.33213143 m3, up to 25.8 ft apart) and ceiling 147534 (an outline of 4.15431293 m3 on 2025, a record box of 5.14728698 m3 on 2024). The roof's difference is the "why the 2025 save's roof differs from the 2024 save's" that RE-341 section 5 left open.

**Run B** (42056599984, head 14a03c02, whose code is this PR's): with the arc, circle and shaft readers read on 2025, all three samples pass. `rac_advanced`: 42 beams and landings, 86 outlines (64 `IfcCovering`, 5 `IfcRoof`, 17 `IfcSlab`) and ceiling 147534 equal to the 2024 save's; `rac_basic` 13 outlines (2 `IfcCovering`, 1 `IfcRoof`, 10 `IfcSlab`) and `rst_basic` 354 beams and 5 outlines equal. **Run E** (42062038214, head 6ea88323, the same code) repeats it with the gate as this PR has it, which requires floor 139803, roof 201262 and ceiling 147534 equal to the 2024 save's on 2025 and counts the elements whose sketch holds an arc: all three samples PASS, with the same counts as run B (42 beams and landings, 86, 13 and 5 outlines, 354 beams on `rst_basic`), and for `rac_advanced` "of the 6 elements whose sketch holds an arc, 2 are outlines compared equal here (139803, 201262); the other 4 are boxes here or not outlines".

What moves on the 2025 samples, against `main` (runs A and B; exported diagnostics):

| sample | `partition_beam_axis` | `partition_beam_box_along_axis` | `native_landing_tread` | record-box bodies | entities | parameter values |
|---|---|---|---|---|---|---|
| `rac_advanced` | 0 to 29 | 0 to 8 | 0 to 5 | 5,194 to 5,152 | 26,795 to 26,833 | 20,747 to 20,847 |
| `rst_basic` | 0 to 345 | 0 to 9 | 0 | 427 to 73 | 2,236 to 2,581 | 2,178 to 3,103 |
| `rac_basic` | 0 | 0 | 0 | 341 to 341 | 2,179 to 2,181 | 1,585 to 1,588 |

`rac_basic` draws no beam or tread landing; its 2 more entities and 3 more parameter values are, I read, its monolithic landing's type (`attach_landing_slabs` records the type of every landing on a listed release), not otherwise checked. The 2024 samples are byte for byte `main`'s, and so are the 2025 MEP samples (`rme_advanced`, `rme_basic`).

## 4. What the arc reading is

Floor 139803's sketch holds 14 lines. Thirteen read as lines in their record boxes and not as arcs. Line 141561 reads as an arc inside its record box and **not** as a line (its line reading is outside the box):

```
centre (16.758119, -11.782846, 12.467192)  radius 11.482940 ft  angles -0.671678 to 0.671678 rad
start (25.746721, -4.637012)  end (25.746721, -18.928679)
```

The record is the same in the 2024 and 2025 saves, to the six places printed (`probe_824_slab_arcs`, run B). The chord between those ends leaves out a circular segment of 24.33 ft2 (2.26 m2). The volume gap between the saves, 441.111228 m3 less 440.512245 m3, is 0.5990 m3, which is that segment times a floor 0.265 m thick: the thickness is implied, I did not check it against the floor's type. Ceiling 147534's sketch holds a half circle (line 147612, radius 10.1403 ft, centre (-18.654, 86.887)) and roof 201262's an arc of radius 30 ft (line 201270), both the same in both saves. In the 2024 save the ceiling is a record box and the floor and roof are drawn with their arcs; the 2025 save now draws them as the 2024 save does.

Six `rac_advanced` owners and one `rac_basic` owner hold an arc in their record box in both the 2024 and the 2025 save (`probe_re96_sketch_arcs`, run B): `rac_advanced` 111931 (46 lines, 16 arcs), 139803, 147534, 176909, 201262 and 201312, `rac_basic` 947498 (radius 110.3 ft). `rst_basic` has none. What the gate holds for each, in the 2025 and 2024 saves alike (run E's `named` lines): floor 139803 an `IfcSlab` outline of 441.111228 m3 and 170 vertices; roof 201262 an `IfcRoof` (`partition_roof_slope`, `ProfileResolved`) of 5.33213143 m3 and 268 vertices; ceiling 147534 an `IfcCovering` record box of 5.14728698 m3 (with the arc read, its chord outline is not drawn, as in the 2024 save); 176909 an `IfcRailing` record box of 4.14763839 m3; 111931, 201312 and 947498 have no exported product with a body in either save.

**The count the gate stands on.** Two elements with an arc are compared as drawn outlines on 2025, a floor and a roof, both on one project. The ceiling is compared as a named element and is a record box in both saves, so it checks that the arc reading keeps it a box and not that an arc is drawn right; the railing is printed and is a box in both. Seven owners hold an arc, and these are the ones whose bodies show it. Before the change the 2025 save differed from the 2024 save on three of them (floor, roof, ceiling), which is run A's failure.

## 5. What checks the readings, counted

| | 24 models with Revit's own 2025 exports (23 R2025 models, RE1 Architecture) | the 2025 Autodesk samples |
|---|---|---|
| beams (`partition_beam_axis`, `partition_beam_box_along_axis`) | 0 | 391 (`rac_advanced` 37, `rst_basic` 354; 17 of them drawn as their box along the line, `partition_beam_box_along_axis`) |
| tread landings | 0 | 5 (`rac_advanced`) |
| monolithic landing slabs | 0 | 1 (`rac_basic`, a box on 2025) |
| shaft openings (`probe_re99_shaft_openings`, run D 42061388847, head 9464a840) | 0 | 2 (`rac_advanced`, ids 156935 and 157484; `rac_basic`, `rst_basic` and the two MEP samples hold none) |
| floors, ceilings and roofs whose sketch holds an arc | 0 of 2 `IfcSlab` (RE1) and 22 roofs | 7 owners hold an arc (6 `rac_advanced`, 1 `rac_basic`); 2 are drawn outlines (a floor, a roof), 1 a ceiling kept as its box, 1 a railing box, 3 have no body |

- The 24 models' exports are **byte for byte `main`'s** in both run A and run B (`ifc.sha256`, 24 of 24), so every scorer reads what it read on `main`: 22 roofs whose body Revit's export holds (one in each of 20 R2025 models, two in `20_T_U_Roof`), RE1's 2 slab outlines, and the rest. That is no regression, and it is not evidence for the readers added here: the models never reach them. The only arcs among them are two railing owners' in `17_Rooms_Stairs_Spiral` (13680 and 13684), which no outline, body or shaft uses.
- The two `rac_advanced` shafts (a 189 ft by 176 ft one spanning the building's plan and a 9 ft by 98 ft one) are the only shaft openings in any 2024 or 2025 sample or model measured here. `probe_re99_shaft_openings` finds no void equal to a shaft's outline in the 2024 save or the 2025 one, at head 9464a840 (run D, 2025 reader off) and at head 6ea88323 (run E, 2025 reader on), and the 2025 save's gate output equals the 2024 save's, so the shaft reader's 2025 list entry is exercised on two shafts that cut nothing: **it is unchecked on 2025**.
- **Held-out models.** Run E (42062038214, head 6ea88323, `holdout=true` with the spent pair): the exports of all 19 held-out and spent models are byte for byte `main`'s (`ifc.sha256`), so none moves in any category and none moves toward or away from Revit's export. Three of them are Revit 2025 files (named so in `research/holdout-models.tsv`); their counts hold slabs and roofs and no beam property set, and their exports do not change, as a drawn beam or a typed landing would change them. So they do not exercise the beam, landing or arc readers either, and the sentence "held-out models do not move" is no evidence for those readers.

## 6. Not measured

- A 2025 beam, tread landing, arc outline or shaft against a Revit export: none exists here (section 5).
- A natively authored Revit 2025 beam: the readers have run only on two Autodesk samples saved from 2024.
- The quantities and property sets of the newly drawn beams, landings and outlines: the gate compares bodies.
- An error the 2024 and 2025 saves share, or a project edited the same way between saves.
- A monolithic landing on 2025 (its lines are read on 2024 only); a roof's slope on 2026 and 2027 (RE-341 section 5).
- Whether the shaft reader is right on 2025: no model draws a shaft void there.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref <branch holding the scorers> -f ref=<full commit SHA> -f base=main \
    -f samples=true -f sample_years="2024 2025" -f r2025=true -f holdout=true \
    -f models="re1-architecture r2025-* holdout-* spent-*" \
    -f probe=probe_re99_shaft_openings -f probe_scorer=-
```

A run for a change to a reader of Revit 2024 to 2027 is `samples=true` with every year and no `ifc_flags` (`docs/maintaining.md`, "Bodies across saves"). The probes `probe_re96_sketch_arcs` and `probe_824_slab_arcs` (which now lists rac_advanced's floor 139803 and ceiling 147534 by default) run the same way.
