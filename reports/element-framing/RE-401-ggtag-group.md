# RE-401: a GGTag is a group of one point, and it alone kept four of Snowdon Towers' roofs a level block

**Date:** 2026-11-10
**Issues:** #1630 (roofs that stay a level block), #1612; follows RE-389 (a roof drawn from its saved mesh where the mesh decodes whole), RE-224 (what keeps a saved mesh from being whole), RE-300 (a light source's saved graphics) and RE-303 (a saved flip control)
**Artefacts:** Autodesk's Snowdon Towers Architectural 2024, which Measure fetches for a run and never stores; Revit's own IFC4 export of it is held locally, has no licence and is not committed, so what is said of it here is counts and ratios; Autodesk's `rac_advanced` sample project of Revit 2023 to 2027, which has no Revit export in a run
**Runs:** Measure [42903219538](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42903219538) (the BIM4LCA housing and the timber office, head against `main`; section 5.1), [42884358341](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42884358341) (`probe_re389_roof_blocks`, the 2024 samples) and [42887296804](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42887296804) (`probe_re389_roof_graphics_tree`); Measure [42890115878](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42890115878) for the change (unfiltered: `nordic`, `r2025`, `samples`, `racadv`, `bimcollab`, `holdout`; head `008437f5` against `main` at `38dff86c`, 118 models on each side) and [42895732087](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42895732087) (the 2024 samples with `keep_ifc`, head against `main` at `9066e124`, whose exports were scored against Revit's local export)
**Status:** Positive where Revit's own export checks it. A `GGTag` is a group of one `GPoint`, and walked as one it frees 6 bodies of Snowdon Towers Architectural from an extruded stand-in: the 4 roofs it alone kept (each `match` in `roof_bodies_vs_ifc`: every vertex of Revit's body within 0.000006 ft of rvt-rs's, the volume within 1.5e-7) and 2 slabs (bounding box within 0.0005 ft, volume within 0.009%), and 1 roof on each of `rac_advanced`'s five saves, which no Revit export scores. No other export of the 118 models moves, and no census or body scorer regresses.

## 1. The question

#1630: of the 20 roofs of Snowdon Towers Architectural (Revit 2024) #1612 draws 10 from their saved meshes, each within 0.4% of Revit's volume. The other 10 stay the level block of their sketch outline, 4.9% to 133% above Revit's volume. `saved_meshes::attach` keeps a block where the saved graphics report a diagnostic other than a view-semantics one, or give no triangle, and the export log counts those together for every class.

## 2. What keeps each of the 10

`probe_re389_roof_blocks` lists every roof the export writes with what `attach` reads of its saved graphics:

| reason the saved graphics report | roofs |
|---|---:|
| `unsupported reachable graphics class GGTag`, and nothing else | 4 |
| `GGTag` and `malformed ruled-surface edge is connected to face ...` | 4 |
| `GGTag` and `multiple cylinder trim loops unsupported` | 1 |
| `shared curved edge changes its fixed parameter`, and nothing else | 1 |

`GGTag` is the one reason of 9 of the 10 and the only one of 4.

## 3. What a GGTag is

`probe_re389_roof_graphics_tree` prints the whole graph of four of them. A `GGTag` is a group: on all 26 of the three roofs that hold any, flags 557252 and `m_subNodes` of exactly one `GPoint`, with `m_bUseModelTestPoint`, `m_modelTestPoint` and `m_selectByModelTestPoint` (a point to hit-test the group by, as its name says), and no surface. The roofs hold one `GGTag` per `GPoint` (12 and 12, 10 and 10, 4 and 4); the walk refused the class as unsupported, which kept each roof partial.

## 4. The change

`native_graphics_traversal` walks a `GGTag` as a group, like a `GGroup`: its children are visited, and its `GPoint` is excluded as a branch with no surface, as every point is. A `GGTag` whose child is anything else is walked as that child is, and refused if it is unsupported.

## 5. Measured

**The exports.** Run 42890115878, head `008437f5` against `main` at `38dff86c`: 6 of the 118 exports move, `rac_advanced` 2023, 2024, 2025, 2026 and 2027 and Snowdon Towers Architectural 2024; the other 112, the 18 held-out models, the 24 Revit 2025 models and the BIM4LCA offices among them, are byte-identical, and no census or body scorer is worse than on `main`. The export log of Snowdon moves `saved_graphics_mesh` from 2,517 to 2,523 bodies and `partition_element_record_bbox` from 1,122 to 1,116, and the reasons for a floor or roof kept as its outline fall (`corners_not_in_one_plane` 5 to 2, `roof_or_ceiling_edit_not_drawn` 2 to 1); each `rac_advanced` save gains one saved mesh.

**Which bodies.** Run 42895732087 kept the exports of the 2024 samples. Counting each product's representation (no meshing), the only two classes that change on Snowdon are `IfcRoof` (10 to 14 triangulated bodies, 10 to 6 extruded) and `IfcSlab` (3 to 5 triangulated, 215 to 213 extruded), 4 roofs and 2 slabs; on `rac_advanced` 2024 it is one `IfcRoof` (4 to 5 triangulated). No other class gains or loses a mesh on those files, so a `GGTag` in a door, fixture or furniture draws nothing new there.

**Against Revit's export** (Snowdon Towers Architectural, local only; `tools/re/roof_bodies_vs_ifc.py` with the IfcSite placement undone, as #1653 repairs it; the bodies' volume and bounding box in Revit's frame, a layered slab as the sum of its parts):

| product | base (`main`) | head | rvt-rs's volume against Revit's, bounding box |
|---|---|---|---|
| roof 1924305 | differs: 276.16 ft3 against 261.40, farthest vertex 13.4 ft | match | 1.0000001, within 0.000003 ft |
| roof 1924316 | differs: 122.51 against 115.85, 7.72 ft | match | 0.9999999, within 0.000003 ft |
| roof 1924325 | differs: 36.94 against 35.22, 0.281 ft | match | 1.0000000, 0 ft |
| roof 1935345 | differs: 33.91 against 32.13, 0.0833 ft | match | 1.0000000, 0 ft |
| slab 1405282 | an extruded solid | a mesh | 1.0000853, within 0.00046 ft |
| slab 1406102 | an extruded solid | a mesh | 0.9999529, within 0.00004 ft |

Of the 20 roofs, `roof_bodies_vs_ifc` reads `match` (vertices within 0.001 ft and volume within 1e-4) on 2 on `main` and 6 on the head, and every vertex of Revit's body on rvt-rs's surface on 13 and 15. The 10 roofs that were already meshes read within 0.4% of Revit's volume, as RE-389 states.

### 5.1 The second project

A row pre-registered on #1636 ([comment](https://github.com/DrunkOnJava/rvt-rs/issues/1636#issuecomment-6779306666), 2026-11-10 21:08 UTC, before any run of this rule on these models) held the rule to the BIM4LCA housing and the timber office: every roof the change draws must read `match`, and every other element it draws must have its bounding box within 0.01 ft and its volume within 1% of Revit's. Measure run [42903219538](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42903219538) (head `c8aeb4cd` against `main` at `0bb8067f`, the eight models, every default scorer): 0 exports move and no counter or scorer output differs on any of the six housing files or the two timber office files, so no body is drawn that was not. The rule is **not exercised** on them: none holds a `GGTag` that decides a body. That is not holding: the second project says nothing of this rule, which rests on Snowdon Towers Architectural.

## 6. Not shown

- **The other 6 roofs** keep their block, each with its reason of section 2: `malformed ruled-surface edge is connected to face ...` (4 roofs, 5 to 138 diagnostics each), `multiple cylinder trim loops unsupported` (1) and `shared curved edge changes its fixed parameter` (1: a shared edge whose end points hold one parameter of the curved face fixed and whose interior samples do not).
- **Revit's export of Snowdon is not in Measure.** The six bodies are scored against the local copy of Revit's export; Measure can score nothing on Snowdon Towers, so there the change is held by these numbers alone and, in Measure, by the same-project gates, which hold the 2025 to 2027 saves of the projects that have a Revit 2024 save. `rac_advanced`'s five roofs have no export.
- **The second project.** The rule is not exercised on the BIM4LCA housing or the timber office (section 5.1), so it rests on Snowdon Towers Architectural alone, and on `rac_advanced`'s five roofs that no Revit export scores.

## 7. Reproduce

The probes come from `-f ref`, the commit of this change; the scorers from `--ref main`.

```bash
gh workflow run measure.yml --ref main -f ref=5699252ea87805ed0d3f5a2483b0dc825aa1540e -f base=none \
    -f samples=true -f sample_years="2024 2024" -f scorers=none \
    -f probe=probe_re389_roof_blocks -f probe_scorer=-
gh workflow run measure.yml --ref main -f ref=008437f507fada3589e8fb4894fd73fc9a12ec50 -f base=main \
    -f samples=true -f sample_years="2024 2024" -f scorers=none -f keep_ifc=true
gh workflow run measure.yml --ref main -f ref=c8aeb4cdf964dbc9cf5b2a9283dab35538179016 -f base=0bb8067f5399071878e34c0718a2db371846adfb \
    -f nordic=true -f models="nordic-housing-* nordic-office-timber-*"
```

`probe_re389_roof_graphics_tree` names the first run with its own `-f probe`. The head of the second run is kept under `refs/archive/2026-11-10/cited/`.
