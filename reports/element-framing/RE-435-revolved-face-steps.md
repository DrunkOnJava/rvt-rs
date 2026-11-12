# RE-435: a revolved face's steps around its axis take the reach of the profile over the face's own v range, not the centre plus twice the radius: 40 of 126 exports move, and their triangles from saved graphics fall

**Date:** 2026-11-11
**Issues:** #1737 (the triangle density of the faces RE-430 and RE-426 draw: this is its first lever); follows RE-430 (#1715, section 8), RE-426 (#1709), RE-302 (2026-11-03) and #1683
**Artefacts:** the BIM4LCA office, housing and terrain models, Autodesk's samples of Revit 2023 to 2027 and the reference models, and Revit's IFC4 exports of those that have one (RE1, the office, the housing: scored by Measure itself; Snowdon Towers Architectural's is local only: ratios, distances and triangle counts only here); rvt-rs's exports by `main` and by the change, which Measure kept (`keep_ifc`)
**Probe and scorer:** a diagnostic build of the grid (a scratch branch, not for `main`, kept at `refs/archive/2026-11-11/scratch/1737-density`); `tools/re/mesh_bodies_vs_ifc.py` (Measure's own results) and `tools/re/ifc_body_diff.py`; the pure helper `profile_reach` is tested in `src/research/native_parametric_mesh.rs` (three failure-first tests: a maximum between two steps, a part of the profile on the near side, a range of one parameter and a reversed one)
**Runs:** the diagnostics, Measure [43055380122](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43055380122), [43056577200](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43056577200) and [43057867596](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43057867596) (the office at permit, and the 2024 samples for the first); the change, [43062414024](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43062414024) (unfiltered, `keep_ifc`, head `e3dba3fd` against `main` `aa5dc4c6`) and its `timing` [43062418900](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43062418900)
**Status:** Positive, and short of a registered row on the count that matters most to Revit's vertices. No element drawn at `main` stops being drawn, and 40 of 126 exports move by the meshes of elements with an arc or an ellipse for a profile: the triangles of the 35 whose results were kept fall by 1% to 20% (the office by 9.7%, an office toilet by 19.8%, Snowdon Towers Architectural by 8.5%) and their IFCs by 1% to 13%, with no class's median volume against Revit's moving by more than 0.0006. Rows 3 and 5 miss: one body of RE1 Architecture, a shower head, crosses the 0.001 ft line (Revit's vertices 0.00073 ft from the mesh before, 0.00103 ft now), and two files read 1.04 of `main`'s peak memory, one of them byte for byte `main`'s.

## 1. The question

A toilet of the BIM4LCA office is 17,494 triangles where Revit's export of it has 1,668 (RE-430 section 8, #1737). Where do the triangles go, and which of them does the chord the grid keeps not need? After this change the toilet is 14,025 (-19.8%).

## 2. Where a toilet's triangles go

A diagnostic build printed, once, every face drawn through `face_grid` (Measure 43055380122, 43056577200, 43057867596: the office at permit and the 2024 samples). The office holds 10 distinct trimmed faces of 19,582 triangles and 151 distinct rectangular faces of 49,147. A toilet has two trimmed faces, of 3,310 and 837 triangles (they occur once in each of the 44 toilets of the diagnostic's first run), so the other 13,347 of its 17,494 triangles are in faces that are not trimmed or are planar: the larger part, and what this change reads. The office's biggest rectangular faces are revolved faces of 2,256, 2,064, 1,125 and 1,050 triangles and two ruled faces of 2,484 and 2,050, and the first of them has 95 columns around its half turn.

A revolved face whose profile is an arc or an ellipse takes its steps around the axis from `angular(c + 2 R)`, where `c` is the distance of the arc's centre from the axis and `R` its radius (an ellipse's longer axis): the farthest the profile can lie from the axis, so that the chord of a step stays within the 0.001 ft the grid keeps at the farthest radius. No point of the profile is farther than `c + R` (the point is within `R` of the centre, whatever the plane of the profile), and the face holds only the part of the profile its own `v` range covers, so the bound was generous by the radius, and by more where the face covers a small part of a large arc: the 95 columns of the first of those faces were cut for a reach near 7 ft, where the steps along its arc read a radius of about 3 ft and the face covers 0.6 radian of that arc.

## 3. The rule

`profile_reach` takes the farthest of 64 equal steps of the face's `v` range of the profile's distance from the axis, and adds half a step's travel (the profile's speed, `R` for an arc and the longer axis for an ellipse, times the step) to it, so that the true maximum, which can lie between two steps, is never short: the reach is never less than the farthest the profile gets, and the chord of a step around the axis is within the 0.001 ft the grid keeps at every radius of the face. The steps along the profile (`angular(R)`) and the other surfaces (a line's radius at the ends of its `v` range, a cone's) are unchanged. Three failure-first tests: the maximum between two steps (the reach is `c + R` at `t = 0`, which no step ends on), a part of the profile on the near side (reach 2.4 against 3.7 for the whole circle, and against the 6.7 the old bound gave), a range of one parameter and a reversed one.

## 4. Against Revit's export

Measure scores the changed bodies against Revit's export itself for the office, the housing and the reference models (`mesh_bodies_vs_ifc.py`); the diff of its output between the two builds is in the run's `compare-scorer-diff`. Over the 13 results whose scorer output moves:

| what | before | now |
|---|---|---|
| office, `IfcSanitaryTerminal` (the toilets; 90 of 90 within 1%) | median volume 1.0017 | 1.0018 |
| office, `IfcFlowTerminal` | median 1.0019 | 1.0021 |
| housing toilets `WC-istuin:WC-1` (8 and 7 per file) | volume 1.0060, Revit's vertices 0.0050 ft from the mesh, rvt-rs's own up to 0.0218 ft from Revit's surface | 1.0060, 0.0050 ft, up to 0.0211 ft |
| RE1 Plumbing `IfcFlowFitting` | median 1.1758 | 1.1752 |
| RE1 Architecture `IfcFlowTerminal` (7 bodies) | every vertex within 0.01 ft: 3; Revit's vertices within 0.001 ft: 4 | 5; 3 |

No other class's row moves, and the largest change of any one body's volume ratio is 0.0015 (RE1 Plumbing). The one body that loses a count is the shower head of RE1 Architecture: Revit's vertices lie 0.00073 ft from its mesh before and 0.00103 ft now. The grid's chord at the farthest radius is the 0.001 ft the scorer compares at, so a body whose Revit vertices lie at the middle of a chord reads at it, and the old bound's margin (by up to the square root of 2 in the steps) kept such a body inside. The same change took two sinks of that file from 0.00395 to 0.00366 ft, and the farthest vertex of the toilet's own from Revit's surface from 0.0126 to 0.0084 ft.

## 5. The whole run, and what the change costs

Measure 43062414024, unfiltered with `keep_ifc` and every model set, `e3dba3fd` against `main` `aa5dc4c6`: 126 exports, **86 identical and 40 moved** (35 with a kept IFC, 5 held-out ones with counts only). The same-project gates and the semantic check pass. `tools/re/ifc_body_diff.py` on the 35: no element drawn from a saved mesh stops being so (BodySource changes: 0), and the meshes of 7,231 of the 126,381 objects drawn from a saved mesh on both sides (5.7%, an element of each save counted each time) differ: 4,167 `IfcPipeFitting`, 1,065 `IfcLightFixture`, 929 `IfcDuctFitting`, 464 `IfcSanitaryTerminal`, 184 `IfcRailing`, 133 `IfcFurniture`, 133 `IfcBuildingElementProxy`, 115 `IfcCableCarrierFitting`, 36 `IfcCovering` and 5 `IfcElectricAppliance`.

| exports | triangles from saved graphics, `main` to the change | IFC |
|---|---|---|
| BIM4LCA office (4) | 1,586,184 to 1,432,889 (-9.7%); the other three alike | 122.5 to 117.1 MB (-4.4%); 112.5 to 107.2 and 112.0 to 106.7 (timber) |
| BIM4LCA housing, the four large | 397,561 to 372,251 (-6.4%) | 24.2 to 23.3 MB (-3.6%) |
| BIM4LCA housing, the two terrain | 295,330 to 291,458 (-1.3%) | 14.1 to 13.9 MB (-0.9%) |
| Snowdon Towers Architectural | 6,228,638 to 5,696,311 (-8.5%) | 269.5 to 251.3 MB (-6.8%) |
| Snowdon Towers Electrical | 3,644,240 to 3,129,488 (-14.1%) | 135.9 to 118.7 MB (-12.7%) |
| Snowdon Towers HVAC | 1,767,336 to 1,601,880 (-9.4%) | 70.8 to 65.5 MB (-7.5%) |
| Snowdon Towers Plumbing | 2,673,053 to 2,470,237 (-7.6%) | 124.7 to 118.5 MB (-5.0%) |
| `rac_basic` 2023 to 2027 | 791,718 to 713,280 (-9.9%) | 32.4 to 29.4 MB (-9.3%) |
| `rme_advanced`, `rme_basic` 2024 to 2027 | 1,658,056 to 1,563,935 (-5.7%) and 1,771,509 to 1,695,180 (-4.3%) | 99.9 to 96.9 MB (-3.0%) and 105.9 to 103.4 MB (-2.3%) |
| `rac_advanced` 2023 to 2027 | -20 triangles | 0.0% |
| RE1 Plumbing, Architecture | 38,961 to 31,217 (-19.9%), 24,260 to 23,466 (-3.3%) | 2.0 to 1.8 MB (-11.5%), 1.4 MB (-1.9%) |

Export time and peak memory (`timing` 43062418900, three rounds of both builds on one runner, 74 files) read 0.94 to 1.02 and 0.96 to 1.04 of `main`'s; Snowdon Architectural reads 1.02 and 0.99. The two files above 1.03 are Core Interior (418 to 434 MiB), whose export is byte for byte `main`'s, and `rac_basic` 2026 (361 to 375 MiB), whose triangles fall by 9.9%: the noise of the measure on peak memory is 4%.

## 6. The registered rows

Registered on #1737 before the run ([comment 6801418240](https://github.com/DrunkOnJava/rvt-rs/issues/1737#issuecomment-6801418240)):

1. **Held, on the results that can show it.** No export grows: of the 35 moved exports with a kept result none has more triangles than `main`'s (the 5 held-out ones that move hold counts only), and their sizes are in section 5.
2. **Held.** Every element drawn at `main` is still drawn from its saved mesh (BodySource changes: 0 of the 35 exports), and the meshes that change are those of elements with a revolved arc or ellipse profile (the classes of section 5).
3. **Missed by one body.** No class's median volume ratio moves by more than 0.002 (the largest, RE1 Plumbing's fittings, is 0.0006), and the counts of bodies with the volume within 1% fall on no class; but the count with every vertex of Revit's within 0.001 ft falls by one, the shower head of RE1 Architecture (0.00073 to 0.00103 ft), on a class whose count within 0.01 ft rises by two (section 4).
4. **Held.** The office's triangles from saved graphics fall by 9.7% and Snowdon Towers Architectural's by 8.5% (5% asked); the IFC sizes are in section 5.
5. **Missed on the letter, noise on the cause.** Export time reads 0.94 to 1.02 and peak memory 0.96 to 1.04 of `main`'s (1% asked); Core Interior, whose export is byte for byte `main`'s, reads 1.04.

## 7. What the change leaves

The chord stays 0.001 ft, and it is used up at the farthest radius of every revolved face: the margin the old bound gave by accident is gone, which is the shower head's. Two levers remain, as #1737 says. The trimmed faces' cells cost 4,147 of a toilet's 17,494 triangles because every sample of a trim is a knot of both axes; an offline prototype of cutting only the cells the trim crosses (the toilet's 837- and 3,310-triangle faces to 90 and 470) comes to about a seventh of them. And the chord itself is a choice: Revit's own export holds its vertices up to 0.005 ft from the surface, and a chord of 0.005 ft would take a toilet to about 3,500 triangles (the grid's triangles scale with 1/chord), at the price of the vertex agreement the 0.001 ft was chosen for.

## 8. Reproduce

```bash
# the change's run (section 5): the scorers come from main, the code from the archived head
gh workflow run measure.yml --ref main -f ref=e3dba3fd7833fb677047cdd5ce15038635abc585 -f base=aa5dc4c6d93e861015c10e0a0d8f9f16c8b57e71 \
  -f samples=true -f nordic=true -f r2025=true -f holdout=true -f bimcollab=true -f racadv=true -f keep_ifc=true
gh workflow run measure.yml --ref main -f ref=e3dba3fd7833fb677047cdd5ce15038635abc585 -f base=aa5dc4c6d93e861015c10e0a0d8f9f16c8b57e71 \
  -f samples=true -f nordic=true -f timing=true -f scorers=none
python3 tools/re/ifc_body_diff.py <main model.ifc> <change model.ifc>
```

`e3dba3fd7833` is archived as `refs/archive/2026-11-11/cited/e3dba3fd7833` (the branch's head when it was measured; the commits after it merge `main` and add this report, the changelog and the reconnaissance addendum). The diagnostic build is `refs/archive/2026-11-11/scratch/1737-density`.
