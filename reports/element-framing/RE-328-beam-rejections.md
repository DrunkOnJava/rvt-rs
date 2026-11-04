# RE-328 — Why `beam_body` declines a beam's one location line, and the body that is safe to draw

**Date:** 2026-11-04
**Issues:** #1247; follows RE-49 and RE-306
**Runs:** Measure [41787230413](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41787230413) (Autodesk's 2024 samples) and [41787251285](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41787251285) (the BIM4LCA office), the probes at 16f6ac40; [41789828849](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41789828849) and [41789832200](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41789832200), the rule at 71949fcd against main at fadd80b5; [41791148790](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41791148790), the rule merged onto main at 71c0677a (samples); [41791336726](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41791336726), the final probes at d3781de6 (samples); [41792442622](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41792442622), the final rule at c3efaaf9 (samples); [41797196513](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41797196513), the unfiltered run at b22d43b1 (section 5.1).
**Oracle:** the mesh Revit saved in the file for the same element, read with `native_saved_scene` — Revit's own tessellation, **not an IFC export**. There is no Revit IFC export of a Revit 2024 structural model in the corpus.
**Result:** positive for one class, negative for the rest.
- On every file measured, one test declines the line of every beam `beam_body` declines for a reason other than a missing line: the closure of the section solve (`open_closure`), except 2 Snowdon beams whose line leaves the record box.
- 34 of 52 declined beams are level, have no Cross-Section Rotation and have a record box longer than their line (by at most a foot). 25 of them have a saved mesh that decodes whole, and on all 25 the section across the line is the mesh's to 0.01 ft. They are now drawn as that box along the line.
- On 16 of those 25 the mesh's ends are the body's to 0.01 ft; on 9 Revit's mesh ends inside the body (section 4.2). The other 9 of the 34 have no decodable mesh and sit on a model axis, where the body is their record box.
- RE-49's explanation of the 17 Snowdon beams it left as boxes (15 with oblique end cuts) is not borne out: all 17 are rectangles along the line whose box is longer than the line.
- 18 beams keep their box: 16 sloped, rolled beams, 2 whose line leaves the box.

## 1. The question

RE-306 found, on a private Revit 2024 project (#1247), 830 beams drawn as their record box, every one with exactly one location line and none with copies that disagree. So `beam_body` declined all 830. It declines a line for one of seven tests, in the order `beam_check` now takes them (`BeamReject`):

1. `NotFinite`: a value of the box or the line is not finite;
2. `InvertedBox`: the box's low bound is above its high one on some axis;
3. `EndOutside`: an end lies more than 1e-6 ft past a face of the record box;
4. `ZeroLength`;
5. `TooSteep`: less than 0.1 of the line's length is in plan;
6. `NoSection`: the solved width or depth is not positive;
7. `OpenClosure`: the plan extent the solve does not use is more than 0.01 ft off the solved box's.

Upstream of those, a beam also keeps its box when the release is not 2024, when no partition names it, when its data holds no bounded line, when the first curve record is an arc, and when its copies disagree (RE-306).

## 2. The probes

`probe_re328_beam_rejections` sorts every structural-framing element the partition reader recovers by what its data holds and, for one line, by the first test `beam_check` fails, with a decade histogram of how far each test fails, the line's orientation, the Cross-Section Rotation, whether the type is an I section, whether the data holds another line that passes, and what the rule of section 5 does with it. It prints counts only, opens the file with `--large-model`'s limits and is the probe for the reporter's file. `MISMATCH` lines count beams whose attached line differs from what the check and the rule say; there are none.

`probe_re328_beam_meshes` scores each candidate body against the beam's saved mesh where all of it decodes (`partial` meshes are counted and not scored, as `saved_meshes::attach` keeps their box), taking the mesh's extent along the line, across it level and across it in the line's vertical plane. It scores the beams the check declines (`target`) and, as a control, the first 400 whose line it accepts.

## 3. Measured: what declines the line

Autodesk's 2024 sample projects, `.rvt` sha256 from `research/autodesk-sample-projects.tsv`:

| sample (sha256 prefix) | beams | line accepted | `open_closure` | `end_outside` | no line |
|---|---:|---:|---:|---:|---:|
| Snowdon Towers structural (`6dc833f9`) | 942 | 923 | 17 | 2 | 0 |
| rst_basic (`aab838f8`) | 370 | 345 | 25 | 0 | 0 |
| rac_advanced (`ff791052`) | 38 | 29 | 8 | 0 | 1 |
| the other seven 2024 samples | 0 | | | | |

The BIM4LCA office, as built and at building permit (Revit 2024): 8 beams each, all accepted. Core Interior and the RE1 models have none. Of the seven tests only `end_outside` and `open_closure` decline any line on any file here.

The 52 `open_closure` and `end_outside` beams split on every axis the probe prints:

| class | beams | rst_basic | rac_advanced | Snowdon |
|---|---:|---:|---:|---:|
| level, rotation 0, box longer than the line by at most 1 ft | 34 | 9 (on a model axis) | 8 (on a model axis) | 17 (2 on a model axis, 15 rotated in plan) |
| sloped, rotation not 0, box shorter than the line by at most 0.1 ft | 16 | 16 | | |
| `end_outside`: line leaves the box by up to 100 ft, across the line | 2 | | | 2 |

The data of each of the 52 holds two lines and no other line passes the check. The 17 Snowdon `open_closure` beams all have an I section for their type.

## 4. Measured: the candidate bodies against the saved mesh

### 4.1 Controls

The 782 beams the check accepts that were scored as controls (the first 400 by id of Snowdon's 923, rst_basic's 345, rac_advanced's 29 and the office's 8) score as RE-49 scored them against a VIM, now against the file's own meshes, and the rule changes none of them (the check is the same function):

| | Snowdon (400) | rst_basic (345) | rac_advanced (29) | office (8, each file) |
|---|---:|---:|---:|---:|
| whole mesh | 399 | 331 | 6 | 8 |
| section equal, ends equal | 156 | 27 | 0 | 8 |
| section equal, ends mesh inside | 124 | 239 | 6 | 0 |
| section mesh inside, ends equal | 8 | 0 | 0 | 0 |
| section mesh inside, ends mesh inside | 111 | 65 | 0 | 0 |
| section differs, or ends past | 0 | 0 | 0 | 0 |

The run that made the change (41789828849) reads the same counts as the one before it (41787230413) on every file.

### 4.2 The 34 beams the rule draws

| | beams | whole mesh | section equal | ends equal | ends: mesh inside the body | no mesh |
|---|---:|---:|---:|---:|---:|---:|
| rac_advanced, level on a model axis | 8 | 8 | 8 | 4 | 4 | 0 |
| Snowdon, level on a model axis | 2 | 2 | 2 | 0 | 2 | 0 |
| Snowdon, level, rotated in plan | 15 | 15 | 15 | 12 | 3 | 0 |
| rst_basic, level on a model axis | 9 | 0 | | | | 9 |
| **total** | **34** | **25** | **25** | **16** | **9** | **9** |

- **25 of 25** have the mesh's section across the line to 0.01 ft (width, depth and centre). None `differs`, and the mesh runs past no end.
- **16 of 25** have the mesh's ends as the body's to 0.01 ft. **9 of 25** have the mesh inside the body at one end or both: Revit trimmed the beam, or the beam is shorter than its record box. On the 9 the body runs past the mesh by more than 0.01 ft and at most 1 ft at its worse end (`body_overshoot`: 0 on 16 beams, up to 1 ft on 9; 4 of the 9 are rac_advanced's, 2 Snowdon's on a model axis, 3 Snowdon's rotated). For comparison the bodies RE-49 draws, which the rule leaves alone, run past the mesh by up to 10 ft: Snowdon's controls are within 0.01 ft on 164 of 399, within 1 ft on 207 more and within 10 ft on 28.
- The 12 rotated beams whose mesh ends equal the body's are the 12 whose record box equals the mesh's axis-aligned box.
- **The 9 rst_basic beams have no saved mesh that decodes** (the scene returns none for them). They sit on a model axis, where the box along the line is the record box itself, so the body is the box they had. They are the only drawn beams without an oracle, and nothing is claimed for them beyond that.
- `rule` and `free` score identically on all 25: the rule's extra tests (the line lies in the body, a plan angle within 25 degrees of an axis, rotation 0) removed none. The angle limit is `cos 50°` (|cos 2θ| at least 0.643): the beams measured are on a model axis or 24.5 degrees off one (0.656), and wider angles are unmeasured, so they stay boxes. (The first rule, at 71949fcd, took 30 degrees; run c3efaaf9 below reads the same 34 at 25.)

### 4.3 What the rule leaves as boxes

| | beams | why |
|---|---:|---|
| rst_basic, sloped, rotation not 0 | 16 | a box along the line holds the mesh (`mesh_inside`, section and ends, 16 of 16) but is not it: a rolled section's envelope across the line is wider than the section |
| Snowdon, `end_outside` | 2 | the line leaves the box by up to 100 ft; it is not the beam's location line, and no mesh decodes |
| rac_advanced, no line | 1 | |

## 5. The rule, and what the body is

`beam_body_in_box` draws a beam whose line `beam_check` declines for `OpenClosure` only, where the line is level, the Cross-Section Rotation (RE-276) is read and exactly 0, the plan angle is within 25 degrees of a model axis (|cos 2θ| at least cos 50° = 0.643; the rotated beams measured are 24.5 degrees off an axis, 0.656, and nothing wider is measured), the box along the line whose extents are the record box's (`box_along`, length free) is longer than the line, and the line's ends lie inside it. A beam whose rotation is unread stays a box.

**What the body is, and is not:**

- Its **direction** and its **section** (width, depth) come from the line and the record box. Its section across the line is Revit's on 25 of 25 beams measured.
- Its **length is the record box's, not the beam's.** The body is longer than the location line by at most a foot, and on 9 of the 25 measured Revit's own mesh ends inside the body at one end or both. The record box's own extent along the line overshoots the mesh by more than 0.01 ft and up to 1 ft on 21 of the 25 (`box_overshoot`; 0.01 ft or less on 4, all rac_advanced's), so the body is never longer past the mesh than the box it replaces: the same on a model axis, and for the 15 rotated beams the turned box, which overshoots on 3 where the axis-aligned box overshoots on all 15. Read the body as an envelope along the line that can run past the beam at its ends.
- `BodySource` is `partition_beam_box_along_axis`, not `partition_beam_axis`. The property set holds `AxisLength`, the location line's length, `BodyLength`, the body's (the record box's) length, and `SectionWidth` and `SectionDepth`.
- It gets **no** `Qto_BeamBaseQuantities.Length`, and so no `Pset_BeamCommon.Span`, `Slope` or `Roll`: no Revit export in the corpus measures a beam longer than its line, and the quantities of `partition_beam_axis` beams are measured on beams drawn the line's length. It gets no I section either; the section is the envelope. (An I section for these 17 Snowdon beams is a follow-on: the mesh measured the envelope, not the I.)
- Under `--saved-meshes` a mesh that decodes whole still replaces it, as it replaces a record box. `ProfileResolved` is false for it.

## 5.1 What the export changes

Run [41791148790](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41791148790) (the rule merged onto main at 71c0677a, against main, the 2024 samples): exactly three exports move, Snowdon structural, rst_basic and rac_advanced; the other seven samples are byte for byte as before. The first comparison, against main at fadd80b5 (41789828849), also moved rac_basic, which has no beams: that was main's own RE-326 room-phase change, gone once the branch carried it. The BIM4LCA office files, as built and at building permit and their two terrain variants, are byte for byte as before (41789832200). The moved files differ in the beams' `BodySource` counts, `partition_beam_box_along_axis` 17, 9 and 8, taken from `partition_element_record_bbox`; before the follow-up commit d7cdd524, `exported.bounding_box_bodies` also lost those 17, 9 and 8, which are still record-box extents, and it no longer does. `record_base_elevation_feet` (the base elevation these beams bind a storey by) lists the new source too, so a written value that was bound by a beam's base stays bound by it.

The final comparison is [41792442622](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41792442622) at c3efaaf9 (this branch merged onto main at 2bfccaba, against main), after the 25-degree limit and the diagnostics fix. It reads the same as the first: the rule draws 34 beams (Snowdon 17: 2 on a model axis, 15 rotated; rst_basic 9; rac_advanced 8), the rule rows score as in section 4.2 (section equal on all 25, ends equal 4, 12 and 0, mesh inside 4, 3 and 2), every control row is the same as in 4.1, exactly Snowdon structural, rst_basic and rac_advanced move, and their diagnostics differ in `body_sources` alone (`partition_beam_box_along_axis` 17, 9 and 8, taken from `partition_element_record_bbox`): `exported.bounding_box_bodies` is as before. There is no `MISMATCH` line on any file.

The unfiltered Measure, [41797196513](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41797196513) at b22d43b1 (this branch merged onto main at 9a57172f and 77867c0c, against main, with `nordic`, `r2025`, `samples` and `holdout`), moves `ifc.sha256` on **3 of 111 exports**: the 2024 samples rac_advanced, rst_basic and Snowdon structural. The other 108 are byte for byte as before: the BIM4LCA office files, Core Interior and the RE1 models, the R2025 models, the held-out models, the other seven 2024 samples and every sample of 2008 to 2023 and 2025 to 2027. The three differ in `body_sources` alone, the diagnostics diff: `partition_beam_box_along_axis` 17 (Snowdon), 9 (rst_basic) and 8 (rac_advanced), taken from `partition_element_record_bbox` (256 to 239, 82 to 73, 5,165 to 5,157); `bounding_box_bodies` and every other count are as before, and the scorer comparison is empty.

## 6. Why the other 18 stay boxes

Fail closed: a sloped, rolled beam's box along the line is wider than its section (16 of 16 `mesh_inside`); a line 100 ft outside its box is not a location line (2); and no line is no body (1).

## 7. Not claimed

- Whether #1247's 830 beams are in the 34's class. The probe is for that file; its `open_closure rule` lines say how many the rule would draw there.
- That the 9 rst_basic beams are drawn right: no mesh decodes. They keep the record box's extents.
- Any end trim. Revit trims a beam at its supports (RE-49: a median 0.625 ft at the larger end); the line does not carry it.
- Anything on a release other than 2024. The line layout is measured on 2024 only (`supports_revit_version`).
- That the 341 beams v0.6.1 drew differently were this class. v0.6.1's `saved_meshes::attach` had no partial-mesh guard (#995 added it), so with `--saved-meshes` it drew a beam from a partial mesh that now keeps its box.

## 8. Reproduce

```bash
# the probe for a private file (counts only); commits from d3781de6 on hold the same rejection probe
cargo run --profile ci -p rvt-probes --example probe_re328_beam_rejections -- model.rvt
# the measurement: the 2024 samples and the office, both probes (code: the SHA named, a commit, not a branch)
gh workflow run measure.yml --ref main -f ref=c3efaaf94aa8cc142f491532c1b0bb8c4a04279f -f base=main \
  -f samples=true -f sample_years="2024 2024" -f scorers=none \
  -f probe="probe_re328_beam_rejections probe_re328_beam_meshes"
gh workflow run measure.yml --ref main -f ref=c3efaaf94aa8cc142f491532c1b0bb8c4a04279f -f base=main \
  -f nordic=true -f models="nordic-office-*" -f scorers=none \
  -f probe="probe_re328_beam_rejections probe_re328_beam_meshes"
```
