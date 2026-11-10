# RE-397: the instances of a railing's saved graphics that the walk skipped, and the flip control that is no surface

**Date:** 2026-11-10
**Issues:** #1635 (railings are written as record boxes, about a thousandth of Revit's volume), #1624, #1247; follows RE-224 (what keeps a saved mesh from being whole), RE-303 (a saved flip control holds no geometry, but marks railings whose mesh is not whole) and RE-318 (the branches the visibility flags hide)
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024; Revit 24.0.5's IFC4 exports, CC BY-SA 4.0, as Measure fetches them); puzzbobb's four Revit 2025 `Rooms_Stairs` models (Revit 25.0.2's IFC4 exports); RE1 Architecture (Revit 2025, IFC2X3); Autodesk's sample projects, which Measure fetches for a run and never stores
**Runs:** Measure [42874785566](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42874785566) (`probe_re397_railing_saved_graphics`), [42876011110](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42876011110) (`probe_re397_railing_graphics_tree`), [42877186261](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42877186261) (`probe_re397_railing_hidden_graphics`), [42878121436](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42878121436) (`probe_re397_railing_symbol_meshes`), [42880570119](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42880570119) (`probe_re397_railing_instances`) and [42882772399](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42882772399) (`probe_re397_symbol_graphics_tree`), each on the office as built with the probe of this change's branch; Measure [42884022203](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42884022203) for the change, and [42889243281](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42889243281) with [42889259936](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42889259936) (`probe_re397_mesh_classes`, on the change and on `main`) for the classes it moves
**Status:** Positive where Revit's export checks it. The instances of a railing's saved graphics that the walk skipped, hidden in the normal draw and forbidden the targets 53246, hold the part of its body RE-303 found missing, and a flip control is no surface. Drawn, with the flip control left out of the walk, 24 railings that Revit's own exports hold come from their saved meshes (15 of the office's 41, in both of its files, 2 on each of the four Revit 2025 stair models and 1 on RE1 Architecture), all with their bounding box within 0.01 ft of Revit's and 19 of 24 with their volume within 1% (the other five are two curved railings whose Revit mesh is 7 times coarser and three whose volume is 2% off), and the four Revit 2025 stairs that aggregate their railings go from 2.6 to 3.0 times Revit's volume to Revit's. 224 more bodies on 19 of Autodesk's samples change from a record box to a saved mesh by the same walk, and no Revit export is there to score them. 26 railings of the office keep their boxes.

## 1. The question

#1635: the 41 railings of the office are written as their record boxes, about a thousandth of Revit's volume (`box_bodies_vs_ifc`: fill 0.001), because no railing's saved mesh is whole. RE-303 found that a `GBiFlipControl` is reachable in the 14 meshes it alone keeps as boxes, that the control holds no geometry, and that leaving it out draws meshes none of which is Revit's body (12 cover about 40% of Revit's volume). It left open why a railing's mesh lacks parts of Revit's body.

## 2. What a railing's saved graphics are

A railing's channel 103 record is a `GElement` with its `GInstance`s, their `InstanceInfo`s and an empty `Geometry`. Each instance names another element, by `m_symbolId`, as its symbol, and the railing's mesh is the meshes of those symbols (`probe_re397_railing_graphics_tree`). On the `KAI` railings the visible symbol is the railing's own ElementId plus one; a second symbol has a higher id, and the one read (`2620342`) is a wrapper whose own graph is one instance of yet another symbol.

`probe_re397_railing_instances` lists every instance of the 41 railings and the volume of the symbol it names, read as an element of its own. The 240 instances are of two kinds only, all rigid (their matrices are orthonormal, so a symbol's volume is the instance's):

| kind | flags | visibility | `m_targets` | instances |
|---|---:|---:|---:|---:|
| visible, unrestricted | 557060 | 0 | 0 | 19 |
| hidden in the normal draw, restricted | 557092 | 1 | 53246 | 221 |

The traversal skips the second kind twice: as hidden in the normal draw (visibility 1, RE-318), and, were they walked (`extract_including_hidden`), by refusing a non-zero `m_targets` (`instance target restrictions are unsupported`), so even the hidden decode of `probe_re397_railing_hidden_graphics` finds no primitive more than the default one on any railing.

## 3. Their volume against Revit's

For each railing, the sum of the volumes of the symbols its visible instances name, and of those plus its restricted instances', over the volume of its body in Revit's export:

| railings | visible instances alone | visible plus restricted |
|---|---:|---:|
| `KAI-4`, 10 railings | 0.385 to 0.437 | 1.0000 to 1.0016 |
| `KAI-5`, 2903645 and 2903804 | 0.541 | 0.980 |
| `KAI-1`, 1485880 (no restricted instance) | 1.0002 | 1.0002 |
| `KAI-3`, 2614259 and 2618205 | 1.195 and 1.198 | the same |
| `KAI-2`, 1749890 and 2902063 | 0.612 | the same |
| `KAI-5`, 2598350 | 0.170 | 0.170 |
| `KAI-6`, 3076853 | 0 | 0 |
| `KJO-1`, 22 stair railings | 0 | 0.020 to 0.029 |

The 59% of a `KAI-4` railing's body that RE-303 found missing is in the restricted instance. Where the symbols are flat-faced the sum is Revit's to 0.2%; a curved body reads over Revit's, since Revit's own mesh is several times coarser than the saved one (`KAI-3`: 60,860 triangles against Revit's 8,634). The rest are not an instance question: `KAI-2`, `KAI-5` and `KAI-6` lose faces to the parametric reader (`shared curved edge ...`, 105 diagnostics on `KAI-6`), and the 22 `KJO-1` railings hold only restricted instances whose symbols are mostly nested detail-family filters and refused faces.

## 4. The flip control

`probe_re397_symbol_graphics_tree` prints the whole graph of the visible symbol of a `KAI-4`, the `KAI-1`, the `KAI-3` and the small `KAI-5` railing. Each holds exactly one `GBiFlipControl`, a leaf under the root `GElement`: category 193, command 38798, an origin, a base vector and a `m_length` of 16.0, and no pointer to anything. It is the grip drawn in a view to flip the railing, as RE-303 read from its fields; it is no surface, and the railing's mesh without it is the railing's body, which was not the case when RE-303 left it out because the restricted instances were still missing.

## 5. The change

`native_graphics_traversal` takes an instance flagged hidden in the normal draw and forbidden the targets 53246 as drawn (`drawn_hidden_instance`), and walks a `GBiFlipControl` as a branch with no surface, as it does a curve or a point. Every other hidden instance stays skipped and every other target mask stays refused.

Measure 42884022203, unfiltered (`nordic`, `r2025`, `samples`, `racadv`, `bimcollab` and `holdout`), head `c12f408c` against `main` at `2713ebcb`, 118 models on each side. 26 exports move, with no census regression and no body or placement regression, and 263 bodies go from a record box to a saved mesh: 15 on each of the office's two files, 2 on each of the four Revit 2025 stair models, 1 on RE1 Architecture and 224 on 19 of Autodesk's samples (101 on Snowdon Towers Architectural 2024). The same-project gates pass on the head (15 samples held to their earlier saves).

| file | record boxes | railings drawn from a mesh | against Revit's export |
|---|---|---:|---|
| office as built and at permit | 183 to 168 and 184 to 169 | 15 of 41 each | bounding box within 0.01 ft on 15 of 15; 11 with their volume within 1% (median 1.0009) and Revit's vertices on the surface within 0.01 ft: the ten `KAI-4` and the `KAI-1`; the two `KAI-3` read 1.195 and 1.198 (on the larger, Revit's mesh has 8,634 triangles against the saved 60,860, and its vertices lie within 0.004 ft of the saved surface), the two small `KAI-5` read 0.980 (their vertices within 0.0008 ft; what the 2% is is not known) |
| Revit 2025 `Rooms_Stairs` 13 to 16 | 12 to 10 each | 2 of 2 each | volume ratio 1.0000, Revit's vertices within 3e-6 ft of the surface, 8 of 8 |
| RE1 Architecture | 4 to 3 | 1 of 1 | bounding box within 0.01 ft; volume 1.0175, vertices 0.011 ft |

The `IfcStair` volume of the four Revit 2025 stair models, which aggregates the railings, goes from 25.92, 23.76, 30.93 and 24.99 ft3 (Revit's 8.78, 8.71, 9.30 and 15.29) to 8.782099, 8.707642, 9.297138 and 15.289523: the stair volume negative RE-390 disclosed, the railing boxes the stair carries, is closed on the 2025 stairs (all four are within 1e-6 of Revit's volume and every vertex of Revit's stair body lies within 3e-6 ft of rvt-rs's surface; one matches by the scorer's bar of the nearest vertex and the volume together, and the other three keep a vertex of their flights that is 0.033, 0.033 and 0.579 ft from rvt-rs's nearest, as the flight rows show unchanged, the second model's stair from 0.922 ft). The office's stairs are as they were: their railings are the 22 `KJO-1`, which keep their boxes.

**Which classes move.** `probe_re397_mesh_classes` counts the bodies of the default export by entity, on `main` and on the change (Measure [42889259936](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42889259936) and [42889243281](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42889243281): the office as built and the 2024 samples). The only entity whose saved-mesh bodies change is `IfcRailing`: 15 on the office, 8 on rac_advanced, 7 on rac_basic, 6 on each rme sample and 101 on Snowdon Towers Architectural, where `main` drew none from a mesh; no other class gains or loses one on those files, so leaving the flip control out of the walk changes no door, window, fixture or furniture body there. The 224 bodies on the 19 samples are railings, the 2023 and 2025 to 2027 saves of the same projects giving the same counts.

## 6. Not shown

- **Other projects.** The pair of visibility 1 and `m_targets` 53246 is the only restricted kind the office holds, and it holds it in railings only; the Revit 2025 stair models and RE1 Architecture check it on 11 more railings, and Autodesk's samples move 224 bodies that no Revit export scores, so what the pair means in a project whose railings Revit's export does not hold is the same walk, not a measurement. Every other hidden kind and every other mask stays refused.
- **What the mask means.** Which bits of `m_targets` name which draw target is not decoded; the rule is the measured pair, not a reading of the bits.
- **The railings that stay boxes.** `KAI-2` (2), `KAI-5` (1), `KAI-6` (1) and the 22 `KJO-1` keep their boxes for the reasons of section 3. Each of them has the diagnostics of its symbols, and #1645 puts the reason on the element.
- **The 2% and the 1.75%.** The two small `KAI-5` railings read 0.980 and RE1's one 1.0175 of Revit's volume with every vertex within 0.011 ft of Revit's surface; the part that differs is not located, and a quantity is not written for a railing, so none carries a wrong one.

## 7. Reproduce

The probes come from `-f ref`, the commit of this change; the scorers from `--ref main`.

```bash
gh workflow run measure.yml --ref main -f ref=5fe47e45ee2c404e2cc6de2661fa772f12f843d2 -f base=none \
    -f nordic=true -f models=nordic-office-as-built -f scorers=none \
    -f probe=probe_re397_railing_instances -f probe_scorer=-
```

The other five probes name the same run with their own `-f probe`.
