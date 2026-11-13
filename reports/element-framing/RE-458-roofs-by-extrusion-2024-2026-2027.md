# RE-458: a roof by extrusion is a solid on Revit 2024, 2026 and 2027 as on 2025, held to the same project's 2025 save

**Date:** 2026-11-13
**Issues:** #1479 (roofs by extrusion are solids only on Revit 2025), #1864 (6 bodies and 12 level binds that differ between Autodesk's 2025 and 2026 saves)
**Artefacts:** Autodesk's sample projects `rac_basic`, `rac_advanced`, `rme_basic`, `rme_advanced` and `rst_basic`, saved by Revit 2024, 2025, 2026 and 2027 (`research/autodesk-sample-projects.tsv`); no Revit export of any of them
**Probe:** `probes/examples/probe_1864_body_diff.rs`
**Runs:** pair runs of the probe, base `f61496f4` (the element diff's digest fix) against the same plus the release list: 2024 [43468180710](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43468180710) and [43465404958](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43465404958), 2026 [43471836274](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43471836274) and [43465340319](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43465340319), 2027 [43468191067](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43468191067) and [43465413771](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43465413771); the second experiment, the RE-341 hold lifted, [43473718161](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43473718161) (2026) and [43473726713](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43473726713) (2027); the unfiltered run of this change [43477851375](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43477851375)
**Status:** positive for the one roof that reads the same way on every release (`rac_basic` 243274); negative for the premise that a second roof (`rac_advanced` 201262) was held by the same list; the RE-341 hold stays (section 4)

## 1. The question

`SKETCH_SPLINE_SUPPORTED_REVIT_VERSIONS` was `[2025]`: RE-211 measured a sketch line's B-spline on puzzbobb's Revit 2025 models, and the roof reader that draws a roof by extrusion as its sketch's section (`partition_roof_extrusion`) runs only where that list holds the release. On a 2024, 2026 or 2027 save such a roof was its saved mesh (its record box before #1191 made the mesh the default). #1479 was written when it was the box, and named two roofs, `rac_basic`'s 243274 (430.617 m3 as a box, 35.424 m3 as the 2025 save draws it, 34.974 m3 as its mesh on the 2026 and 2027 saves) and `rac_advanced`'s 201262 (48.766 as a box against 5.332), and asked for the list to be widened where the same project's 2025 save holds each roof per element.

## 2. What the pair runs say

`probe_1864_body_diff` names the elements whose body, in rvt-rs's export, differs between two saves of one project: the 2025 sample first, then its save by the partner release. Before the release list was widened, and after:

| Pair (2025 save against) | `rac_basic` | `rac_advanced` | `rme_advanced` | `rme_basic` | `rst_basic` |
|---|---|---|---|---|---|
| 2024 | 6 listed, 243274 among them; 5 after | 10; 10 | 133; 133 | 132; 132 | 1; 1 |
| 2026 | 1 (243274); 0 after | 1 (201262); 1 | 0; 0 | 0; 0 | 0; 0 |
| 2027 | 17, 243274 among them; 16 after | 19; 19 | 0; 0 | 209; 209 | 0; 0 |

- **243274 is listed in all three pairs before and in none after.** On the 2024, 2026 and 2027 saves it is drawn by `partition_roof_extrusion` from its sketch, and its body is the 2025 save's: the same `BodySource`, profile and digest.
- **Nothing else changes.** No element is listed after that was not listed before, in any project of any pair (the 2027 run also pairs the 2026 saves against the 2027 ones: the same). The other listed elements, stairs, mullions, columns of the MEP projects and the rest, are the cross-release differences #1864 counted and are not touched.
- **201262 is not a roof by extrusion.** On the 2025 save it is `partition_roof_slope`, drawn from 134 sketch vertices (a 30 ft arc roof cut by walls); on 2026 and 2027 it is its saved mesh, equal to the 2025 body in volume to 0.1% and 3 mm (5.33205264 m3 against 5.33213143, #1706), which is what the cross-save check measures, and it is not listed in the 2024 pair at all. The list never held it: #1479 took it for a roof by extrusion from its box volumes, and the pair runs show otherwise.

## 3. The hold on 2026 and 2027 roofs (RE-341)

`roof_reads_exact_sketches` draws a 2026 or 2027 roof from the record-box solve alone, since "no roof is scored against Revit's export, and a roof whose slope is not read is drawn as its outline at the record box's full height". A second experiment made it true for every release, on top of the widened list. The 2026 and 2027 pair runs then list exactly what they listed before, 201262 included: its 2026 side is still a saved mesh with no resolved profile, so the sketch readers that draw it on 2025 are gated by more than this function. Lifting the hold alone changes no body on any local file, and a roof of a file we do not hold could change in a way no scorer here checks, so it stays.

## 4. The change

- `SKETCH_SPLINE_SUPPORTED_REVIT_VERSIONS` is `[2024, 2025, 2026, 2027]`. The roof reader and the B-spline scan both read it.
- `ROOFS_NOT_THE_2025_BODY` in `tools/re/readers_across_saves.py` is empty: 243274 was its last entry, and the cross-save check fails a pinned roof that becomes the 2025 body ("exactly those"). `tools/ci/check_same_project_projects.py` already holds it to the entries it held before and none added, so an empty list passes.
- The documents that said a roof by extrusion is read on Revit 2025 only say 2024 to 2027.

## 5. The unfiltered Measure

Measure [43477851375](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43477851375) at `99bb949c` (this change on `main` at `2ecff1de`) against `main`, with every flag (146 models). Rows registered on #1479 before it was dispatched:

1. **Only the exports of 2024, 2026 and 2027 saves move. Held.** Three of the 146 exports differ (`ifc.sha256`, and `diagnostics.json` the same three): `rac_basic` 2024, 2026 and 2027. No 2015 to 2023 export, no 2025 export and none of the other 143 (the BIM4LCA models, Snowdon Towers, Core Interior, RE1, puzzbobb's 24 Revit 2025 models, the held-out and BIMcollab models, the families) differs.
2. **Roof 243274 is the 2025 save's body, and the cross-save check passes with the list empty. Held; the check's own run is row 5.** The census of `rac_basic`'s 2026 and 2027 saves read, on `main`, "same 440, differ 1 ... roofs drawn otherwise as pinned 1 of 1 expected" with `IfcRoof 243274 saved_graphics_mesh here, partition_roof_extrusion in the 2025 save: volume 34.974096 here, 35.423568 there`. With this change it reads "same 441, differ 0": all 441 bodies are the 2025 save's. That run used `main`'s scorers, whose list still pins 243274, so it ended "roofs the 2025 save draws otherwise: [] here, ['243274'] expected as pinned, and exactly those" and failed, which is the check doing what it says when a pinned roof heals. This change empties the list; the rerun with its scorers is below.
3. **Any other roof that moves is named. Nothing to name.** No export of a model with a Revit export changes, so `roof_bodies_vs_ifc` and the held-out counts have no roof moving.
4. **The walls under the roof stay as they are. Held.** The 441 bodies of the 2026 and 2027 saves include the 16 walls of #1471, all the 2025 save's (differ 0); 243274 is not a plate roof (RE-351) before or after, so no wall is cut at it.
5. **The same-project gates check run passes. Held.** [43482212285](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43482212285), `samples=true` with this branch's scorers at `ae9870a9`: the `same-project gates` check run on that commit is a success, with the 15 saves of 2025 to 2027 (five projects in three releases) and the two Areas checks all PASS. `rac_basic`'s 2026 and 2027 saves read "census: 441 bodies paired by Tag with the 2025 save's: same 441, differ 0 (... roofs drawn otherwise as pinned 0 of 0 expected)", which is the empty list holding. The gate is exercised by one roof: 243274 is the only roof by extrusion of the sample projects on a release this change opens, and `rac_advanced`'s 201262 reads as before (a saved mesh on 2026 and 2027, equal to the 2025 body in volume and box).

## 6. Not changed

- A roof by extrusion on Revit 2023 and earlier: not read, as before (the list does not hold them, and RE-211's layout was not measured there).
- No Revit export of a 2024, 2026 or 2027 roof by extrusion is scored here: the check is the same project's 2025 save, which cannot catch an error both saves share (the `rac_basic` sample holds one such roof).
- `roof_reads_exact_sketches` (RE-341): the hold stays, and `rac_advanced`'s 201262 stays a saved mesh on 2026 and 2027, equal to the 2025 body in volume and box but not drawn from its sketch.

## 7. Reproduce

```bash
python3 .claude/scripts/measure-dispatch.py --ref <this change's commit> --base <main at its merge-base> --scorers-ref main -f samples=true -f nordic=true -f r2025=true -f racadv=true -f holdout=true -f bimcollab=true -f families=true
python3 .claude/scripts/measure-dispatch.py --ref <this change's commit> --base none --scorers-ref main -f samples=true -f sample_years="2025 2026" -f models=core-interior -f scorers=none -f probe=probe_1864_body_diff -f probe_pair_release=2026
```
