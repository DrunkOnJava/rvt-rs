# RE-480: A ceiling with no `GrossArea` takes the area of its saved mesh's face where that face is one piece

**Date:** 2026-11-15
**Issue:** #1209 (W7: coverings' `GrossArea` on cut ceilings and ceilings in pieces); follows RE-297 (a cut ceiling's `GrossArea` is the area left after the cut, and rvt-rs writes none), RE-465 (a ceiling in pieces: the piece its first-drawn lines lie on), RE-413 (floors and ceilings lose sketch holes from `GrossArea`).
**Artefacts:** the BIM4LCA office (concrete and timber, as built and at building permit, Revit 2024) and housing (Revit 2024) with Revit's own IFC4 exports; the Revit 2025 models, Core Interior, RE1, the Autodesk samples of Revit 2015 to 2027 and the held-out projects (counts only) for what else moves.
**Probes:** `probe_1209_ceiling_params` with `ceiling_area_params_vs_ifc` at `6cc358b59adb`, and `probe_1209_ceiling_mesh_faces` with `ceiling_mesh_areas_vs_ifc` at `3bddff0693df`, both archived under `refs/archive/2026-11-15/cited/`. Neither lands.
**Status:** positive, on one building. The 22 cut ceilings of the BIM4LCA office whose saved face is one piece now have Revit's `GrossArea`, 22 of 22 equal and none different; nothing else that a Revit export checks moves. On the Autodesk sample projects, which no Revit export checks, 15 ceilings gain one (section 6, row 5), each with the same value on every save of its project.

## 1. Where Revit's value comes from

revit-ifc's `GrossAreaCalculator` (CLEANROOM.md source 5, `Exporter/PropertySet/Calculators/GrossAreaCalculator.cs` at `7fe314fd`; cited, not copied) tries three sources in order:

1. an `IfcQtyGrossArea` override parameter;
2. the area the body exporter recorded for the element's extrusion, set only when the body was exported as an extrusion;
3. the element's own computed area (`HOST_AREA_COMPUTED`).

A ceiling drawn as one extrusion of its sketch takes source 2, the sketch's area, which `add_ceiling_quantities` writes (RE-297). A ceiling that a step cuts is no longer one extrusion, so its value falls to source 3: the area of what is left.

## 2. The computed area is not saved as a parameter

`probe_1209_ceiling_params` prints every saved parameter of every ceiling, and its scorer compares each, read as square feet, with Revit's `Qto_CoveringBaseQuantities.GrossArea` to 0.1%, reading the file's own `AREAUNIT` (Measure [43701094119](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43701094119)):

| file | ceilings | cut | a parameter equal to Revit's `GrossArea` |
|---|---:|---:|---:|
| concrete office, as built | 78 | 3 | 0 |
| concrete office, permit | 75 | 13 | 0 |
| timber office, as built | 76 | 5 | 0 |
| timber office, permit | 74 | 6 | 0 |
| concrete housing, as built | 36 | 0 | 0 |
| concrete housing, permit | 32 | 0 | 0 |
| timber housing, as built | 45 | 0 | 0 |
| timber housing, permit | 32 | 0 | 0 |

448 ceilings, 27 of them cut (`CutPairGStep`); the two BIM4LCA terrain files hold none. The values parse (the 8.2021 ft height offset is on every ceiling). A ceiling saves few parameters (584 lines over the 78 of the first file): a height offset, two flags, a type mark and a few texts. Negative: `HOST_AREA_COMPUTED` is not among them.

A first run of this scorer (Measure [43684110083](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43684110083)) scaled Revit's area by the file's length unit squared, which in a millimetre file with an m² area unit reads every value a million times too small. Its negative was re-measured with the area unit and stands. The one scorer on `main` with the same pattern, `save_pair_wall_quantities.py`, was fixed by #2054.

## 3. The saved mesh's face is Revit's area where it is one piece

`probe_1209_ceiling_mesh_faces` takes each ceiling's saved mesh, keeps the triangles whose outward normal is vertical (|n_z| at least 0.999 of the normal's length, turned outward by the saved normals), groups them by height (0.001 ft) and direction, and splits each group into its connected pieces (triangles sharing a vertex to 1e-6 ft). Its scorer sets the largest upward and downward face beside Revit's `GrossArea` (Measure [43701677731](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43701677731), the eight BIM4LCA files):

| | cut, one piece | uncut, one piece | cut, several pieces | uncut, several pieces |
|---|---:|---:|---:|---:|
| face equal to Revit's `GrossArea` | 22 of 22 | 348 of 348 | 0 of 5 | 0 of 73 |
| one piece of it equal | | | 5 of 5 | 73 of 73 |
| that piece is the largest | | | 0 of 5 | 45 of 73 |

Where the face is one piece it is Revit's area, on all 370 such ceilings. Where it is in pieces, Revit names one piece and not always the largest, as RE-465 found for the uncut ones; the mesh does not say which.

## 4. The rule

In the saved-mesh pass, an `IfcCovering` whose `RevitClass` is `Ceiling` and that holds no `Qto_CoveringBaseQuantities.GrossArea` (`ceilings_without_area`): today, the cut ceilings and the uncut ones whose sketch `add_ceiling_quantities` does not read. Where its saved mesh decodes with no diagnostic (`partial_refusal`), and its largest upward and largest downward horizontal faces are each one piece of the same area to 0.1% (`one_piece_face_area`), it gets that area as `GrossArea` (`add_ceiling_areas`), in the set it has or a new one. A ceiling in pieces, a sloped one (no horizontal face) and one whose two faces differ keep none, and no value already written is replaced.

## 5. Measured

Measure [43703086686](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43703086686), the change (`f6648170`) against its merge-base with `main` (`e594a2ba`): every reference model, the BIM4LCA office and housing, the 24 Revit 2025 models, the Autodesk samples of every release and the 19 held-out projects (counts only).

`qtos_vs_ifc`, `Qto_CoveringBaseQuantities.GrossArea`, before and after:

| file | ceilings | equal | different | missing |
|---|---:|---:|---:|---:|
| concrete office, as built | 90 | 72 → 74 | 0 → 0 | 18 → 16 |
| concrete office, permit | 75 | 61 → 71 | 0 → 0 | 14 → 4 |
| timber office, as built | 88 | 67 → 72 | 0 → 0 | 21 → 16 |
| timber office, permit | 74 | 65 → 70 | 0 → 0 | 9 → 4 |

- The 22 values are the 22 one-piece cut ceilings of section 3: 2, 10, 5 and 5. The ceiling at permit whose saved mesh has one face with no triangles (2095292) is among them: an unbounded face is not a decoder diagnostic, and its two faces are each one piece and agree.
- The whole census moves by those values alone: equal +2, +10, +5, +5 and missing as much less; `different` stays 1 on each office file and "rvt-rs quantities Revit does not write" stays 0, 0, 3 and 3.
- The 40 values still missing (16, 4, 16 and 4) are, by the probe of section 3 on each file, the 12 roofing coverings of each as-built file (ventilation units Revit writes as `IfcCovering` `ROOFING`, not ceilings), the 5 cut ceilings whose saved face is in pieces (1, 3, 0 and 1), and the 11 uncut ceilings whose saved face is in pieces and that RE-465's rule leaves out (3, 1, 4 and 3). None of them gained a value: on the head's exports kept by Measure [43706415189](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43706415189) (the run's own exports by `ifc.sha256`, which leaves out the `FILE_NAME` and owner-history lines), the 16 ceilings with no `GrossArea` are, by Tag, all in pieces, of 2 to 9 each. Every one-piece ceiling, cut or uncut, now has its `GrossArea`: the uncut ones (57, 45, 52 and 49) already had it from their sketch.
- The housing's ceilings all had their `GrossArea` already, and its four exports are identical.

What moves (`ifc.sha256`): 17 of 120 exports, the four office files and 13 Autodesk sample exports with no Revit export (section 6, row 5). Everything else is identical by `ifc.sha256`: Core Interior, RE1 Architecture, Einhoven, the BIM4LCA housing and terrain files and the 24 Revit 2025 models. Of the 17 `diagnostics.json` files that differ, the only field that moves is `exported.total_entities`, by one for each ceiling given a new quantity set (10 on the office at permit, 1 or 3 on a sample). The 19 held-out projects' counts are identical on both sides.

## 6. The registered rows

Registered on #1209 before the run (2026-11-15, comment 6892736291), read from Measure [43703086686](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43703086686). The rows named `base=main`; the run compares the head with its merge-base with `main` (`e594a2ba`), so that what moves is this change alone (`main` has since taken #2054 and #2055, which change no export).

1. **The cut ceilings of the four office files take Revit's `GrossArea`, at least 20 of the 22 one-piece ones `equal` and none `different`.** Met: 22 of 22, none different.
2. **`different` and "values Revit does not write" unchanged on every file with a Revit export, the held-out ones included.** Met: 1 and 0, 0, 3, 3 on the office files as before, no other scored file's export moves, and the held-out counts are identical.
3. **No ceiling in pieces gets a `GrossArea` it did not have.** Met: `missing` falls by exactly the 22, and the 16 ceilings left are all in pieces (the other 24 missing values are the roofing coverings).
4. **The exports that move are those holding such a ceiling, and only by that quantity.** Met: the four office files and the samples below, by `exported.total_entities` alone in their diagnostics, with every other scorer line unchanged.
5. **Where a ceiling of a model with no Revit export gains one, the count is reported per model.** Reported. On the 13 sample exports that move, both sides kept by Measure [43706292752](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43706292752) (both sides the same exports as 43703086686's by `ifc.sha256`, on all 25 sample exports of 2024 to 2027), each gained ceiling gets a new `Qto_CoveringBaseQuantities` holding `GrossArea` alone (one `IfcElementQuantity`, `IfcQuantityArea` and `IfcRelDefinesByProperties` each), no other ceiling's value changes, and the entity counts move by those alone:

| project | saves | ceilings | ceilings gaining a `GrossArea` | value (m²), on every save |
|---|---|---:|---|---:|
| `rac_advanced_sample_project` | 2024 to 2027 | 65 | 1 (147534) | 94.894 |
| `rme_advanced_sample_project` | 2024 to 2027 | 43 | 1 (579747) | 62.509 |
| `rme_basic_sample_project` | 2024 to 2027 | 43 | 1 (579747) | 62.509 |
| Snowdon Towers Architectural | 2024 | 68 | 3 (1558154, 1558379, 1558472) | 22.479, 54.602, 60.225 |

15 values in all, none checked by a Revit export: Snowdon's local-only Revit export holds no base quantities, and the other three projects have none. The same-project gates hold them to the project's other saves, where each value is the same.

## 7. Where it rests

The cut ceilings it reaches are those of the BIM4LCA office: four saves of one building, by Revit 2024. The evidence that a one-piece saved face is Revit's area also holds on the uncut ceilings of the office and of the BIM4LCA housing (348 of 348), but on the files with a Revit export the rule writes only on the office's cut ceilings, since every uncut one-piece ceiling there already had its area from its sketch. The rule is listed on `docs/one-project-rules.md`.

## 8. Reproduce

```bash
# the saved parameters (negative), the BIM4LCA offices and housing
gh workflow run measure.yml --ref main -f ref=6cc358b59adbe5116ece7d6ccac1f05fc9b2fa1c -f base=none -f nordic=true \
  -f models="nordic-office-* nordic-housing-concrete-* nordic-housing-timber-*" -f scorers=none -f probe=probe_1209_ceiling_params -f probe_scorer=ceiling_area_params_vs_ifc
# the saved mesh's faces by piece
gh workflow run measure.yml --ref main -f ref=3bddff0693dff27c11c635542f3713cd7eb8b6a5 -f base=none -f nordic=true \
  -f models="nordic-office-as-built nordic-office-permit nordic-office-timber-as-built nordic-office-timber-permit nordic-housing-concrete-as-built nordic-housing-concrete-permit nordic-housing-timber-as-built nordic-housing-timber-permit" -f scorers=none -f probe=probe_1209_ceiling_mesh_faces -f probe_scorer=ceiling_mesh_areas_vs_ifc
# the change
gh workflow run measure.yml --ref main -f ref=f6648170db707a4ecaac311f1421795139242424 \
  -f base=e594a2baa2e0baab1c1e248038e08f9f401ec421 -f nordic=true -f r2025=true -f samples=true -f holdout=true
```
