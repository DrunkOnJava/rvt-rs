# RE-408 — A `GInstance` visible in the normal draw and forbidden the targets 10 is drawn

**Date:** 2026-11-10
**Issues:** #1645; closes the question RE-385 section 3.5 left open
**Runs:** Measure [42908686369](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42908686369) and [42908689958](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42908689958) (`probe_1645_instance_masks_by_class` on the BIM4LCA files and on Autodesk's 2024 samples), [42906883702](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42906883702) and [42906887386](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42906887386) (the refusals), [42914813217](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42914813217) (the change at `019a2c05` against `main` at `5dd1277d`, `keep_ifc=true`), [42918771368](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42918771368) and [42920035985](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42920035985) (what still refuses the housing's showers: the refusal property on the change, `probe_1645_box_reasons_by_class`), [42919909134](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42919909134) (the unfiltered Measure of the final source head `8a364ffc` against `main` at `20b0eb7b`: the BIM4LCA files, puzzbobb's Revit 2025 models and Autodesk's samples, 107 models, every scorer)
**Probe and scorer:** `probes/examples/probe_1645_instance_masks_by_class.rs`; `tools/re/changed_bodies_vs_ifc.py`
**Result:** positive where Revit's export checks it. A `GInstance` visible in the normal draw (visibility 0) and forbidden the targets 10 is part of Revit's body: with it drawn, 332 bodies of Snowdon Towers Architectural that were record boxes come from their saved meshes, all in Revit's export, 301 of them within 0.01 ft of it both ways and within 2% of its volume. A second release agrees: six of puzzbobb's Revit 2025 models gain nine window trims and muntins each (54 bodies), all in Revit's export at volume 1.0000 with every vertex within 2.03e-6 ft both ways (section 5.1). Not exercised on the housing's showers, which the next refusal, `scaled instance mode is not qualified`, still keeps as boxes.

## 1. The question

RE-385 section 3.5 left the 16 art-glass pendant drops of Snowdon Towers Electrical refused, each nesting a `GInstance` whose `m_forbiddenTarget.m_targets` is 10: "what would decide it: a file whose Revit export shows a nested instance with these targets drawn or left out". The traversal draws an instance only where the mask is 0, plus, since RE-397, the pair of visibility 1 and 53246.

## 2. The masks

`probe_1645_instance_masks_by_class` lists every `GInstance` of every exported element's graphics by its visibility (`graphics_visibility` of `m_GInfo.m_flags`) and `m_forbiddenTarget.m_targets`.

| Instances by (visibility, targets) | BIM4LCA office and housing (3 files) | Autodesk's 2024 samples (16 models) |
|---|---:|---:|
| 0, 0 (drawn) | 13,807 | 28,606 |
| 1, 53246 (RE-397, drawn) | 282 on 80 railings | 2,890 on 133 railings |
| **0, 10** | 22 on 22 sanitary terminals (the housing's showers, 11 per file) | 554: 318 building element proxies, 183 furniture, 32 light fixtures, 21 sanitary terminals |
| 0, 0 with the instance resolved in the view | | 17 on 17 walls |
| 3, 0 | 7 on 7 ramps | 2 on 2 ramps |

The 554 and the 22 are exactly the elements refused for `instance target restrictions are unsupported` in the refusal runs (Snowdon Architectural: 318 proxies, 123 furniture, 16 light fixtures and 21 sanitary terminals, the rest on the other samples). No other mask occurs, and none has the bit of target 0 set.

## 3. The change

`drawn_visible_instance` (`native_graphics_traversal.rs`): a `GInstance` of visibility 0 and `m_targets` 10 is drawn, beside `drawn_hidden_instance` of RE-397. Every other mask stays refused.

## 4. Rows and result

Registered on #1645 before the change was run on anything.

| Row | Result |
|---|---|
| The housing's 22 showers refused for this reason alone are drawn (at least 20) | **missed: 0 of 22.** The housing's exports do not move; the next refusal keeps them (section 6) |
| At least 90% of those drawn read within 0.01 ft both ways or `explained` on the housing | not exercised: none is drawn |
| On Snowdon Architectural (Revit's export is local only), of the elements refused for this reason alone that Revit's export holds, at least 90% drawn have every vertex within 0.01 ft both ways and their volume within 2% | **holds: 301 of 332 (90.7%)** |
| Nothing else moves | **holds on the files the row names, and the rule reaches further than the row named.** Of the 12 files scored in the registered run, the exports of 2 move (Snowdon Architectural by 332 bodies, record boxes 1,021 to 777 and turned boxes 206 to 118, saved-mesh bodies 2,618 to 2,950; Snowdon Electrical by 16) and the BIM4LCA files are byte-identical. The unfiltered run (42919909134, 107 models) moves 8 exports: those two, and six Revit 2025 models whose window trims and muntins also nest the instance (section 5.1). The other 99 are byte-identical |

## 5. Snowdon Architectural against Revit's export

`tools/re/changed_bodies_vs_ifc.py` takes the kept exports of the base and the change and Revit's own (local only) and scores the 332 bodies that the change draws and the base drew as boxes. All 332 are in Revit's export with a body.

| Class | Drawn | Revit's vertices within 0.01 ft of rvt-rs's surface | both ways within 0.01 ft | volume within 2% | both bars |
|---|---:|---:|---:|---:|---:|
| `IfcBuildingElementProxy` (refrigerators, dishwashers, cook tops...) | 274 | 274 | 274 | 274 | 274 |
| `IfcFurniture` (chairs) | 21 | 21 | 21 | 21 | 21 |
| `IfcSanitaryTerminal` (kitchen island sinks) | 21 | 21 | 6 | 21 | 6 |
| `IfcLightFixture` (art-glass pendants) | 16 | 16 | 16 | 0 | 0 |
| total | 332 | 332 | 317 | 316 | 301 |

The two groups that miss a bar do it in one bar each. The 15 sinks that miss it have rvt-rs's vertices 0.01062 ft from Revit's surface and Revit's 0.0037 to 0.0039 ft from rvt-rs's, with the volume 1.0033 of Revit's: a finer tessellation of a bowl. The 16 pendants, the drops RE-385 left refused, read 1.044 to 1.101 of Revit's volume (it grows along the run of the drops, 1612605 to 1612619) with every vertex within 0.0093 ft both ways. The shell reading of #1658 reads the sinks and the pendants `other` (their shells are bowls and spheres, with no round end face). 21 proxies read `explained` as round sections.

### 5.1 Six Revit 2025 models, from the unfiltered run

The unfiltered Measure of the final source head (42919909134, against `main` at `20b0eb7b`, 107 models, every scorer) moves 8 exports: the two Snowdon files of sections 4 and 5 and six of puzzbobb's Revit 2025 models, `r2025-12_Rooms_Door_Windows` and `r2025-13` to `r2025-17` (the stairs variants). The registered rows named the BIM4LCA files and Snowdon, so these are a part of the change's reach that the rows did not name, and they are the second release that scores it. In each of the six, nine `IfcBuildingElementProxy` go from record boxes to saved meshes: three `Muntin Pattern_2x2:Muntin Pattern_2x2`, three `Trim-Window-Exterior-Flat:with Sill` and three `Trim-Window-Interior-Flat:Picture Frame` (the boxes filled 0.144 of their bounding boxes). `mesh_bodies_vs_ifc` scores all nine in each file, 54 bodies in all, against Revit's export: every one is in it, at volume ratio 1.0000, with Revit's vertices within 2.03e-6 ft of rvt-rs's surface and rvt-rs's within 1.96e-6 ft of Revit's.

| Model | Saved-mesh bodies scored | Record-box bodies |
|---|---:|---:|
| `r2025-12_Rooms_Door_Windows` | 4 to 13 | 9 to 0 |
| `r2025-13_Rooms_Stairs` | 8 to 17 | 10 to 1 |
| `r2025-14_Rooms_Stairs_endwithriserOFF` | 8 to 17 | 10 to 1 |
| `r2025-15_Rooms_Stairs_NoRisers` | 8 to 17 | 10 to 1 |
| `r2025-16_Rooms_Stairs_Monolithic` | 6 to 15 | 10 to 1 |
| `r2025-17_Rooms_Stairs_Spiral` | 4 to 13 | 13 to 4 |

Nothing else of the six differs. Their other scorer outputs are byte-identical, `schema.txt` moves by the nine swept solids that became meshes (its `placement composition` count falls by 9 in each, for example 21 to 12), and the two `Stringer` lines that leave the printed farthest-15 list of `r2025-13` to `r2025-15` are displaced by the new rows (the list shows 15 of their 17 bodies; the `IfcMember` row of their class tables is unchanged). The instances of these files were not in the mask census of section 2, so that they carry visibility 0 and targets 10 is read from the exports moving with this one rule, not counted.

## 6. What remains

- **The housing's showers.** The 11 `Suihku 1:SU-1` of each housing file, whose instance carried the mask, are still boxes: the walk now passes the mask and meets the next refusal, `scaled instance mode is not qualified` (a nested instance with `m_bHasScale` set; run 42918771368 and 42920035985, the refusal property on the change). The 11 `Suihku 2:SU-2` have no saved mesh. So the row on the housing is missed as registered, and nothing there tests the mask.
- **Snowdon Architectural's other 146.** Of the 478 refused for the mask there (318 proxies, 123 furniture, 16 light fixtures, 21 sanitary terminals), 332 are drawn. The 102 chairs left (`Chair-Breuer`) meet `nonrectangular parametric trim`, and 42 of the 44 proxies left (dishwashers, washer-dryers) meet `face regions need subdivision`.
- **The pendants' volume.** 1.04 to 1.10 with every vertex within 0.0093 ft is the signature of a coarser tessellation of a revolved glass drop; there is no end face for the shell reading of #1658 to read.
- **Snowdon Electrical.** The change draws 16 more bodies there (the pendant drops of RE-385 section 3.5), which no export scores here.

## 7. Not shown

- **What the bits of `m_targets` mean.** Not decoded: the rule is the measured pair (visibility 0, 10), not a reading of the bits, and every other mask stays refused. No instance of the files has bit 0 set, so nothing here says whether a mask with it would be drawn.
- **That Revit's export draws the instance in every file.** It does in the files it was scored on: Snowdon Towers Architectural (Revit 2024, 332 of 332 in Revit's export) and six Revit 2025 models (54 of 54, at volume 1.0000). The BIM4LCA housing, the only project with the mask that Revit's export covers besides those, is not exercised: its showers stay boxes.

## 8. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=019a2c056148b4cc3b2680b511b2ff60e0634738 -f base=main \
  -f nordic=true -f samples=true -f sample_years="2024 2024" \
  -f models="nordic-housing-concrete-as-built nordic-housing-timber-as-built sample-2024-snowdon_towers_sample_architectural" \
  -f scorers="mesh_bodies_vs_ifc saved_meshes_vs_ifc box_bodies_vs_ifc" -f keep_ifc=true
gh run download <run> -n measure-base -D base && gh run download <run> -n measure-head -D head
python3 tools/re/changed_bodies_vs_ifc.py base/sample-2024-snowdon_towers_sample_architectural/model.ifc \
  head/sample-2024-snowdon_towers_sample_architectural/model.ifc "<local Revit IFC4 export of Snowdon Towers Sample Architectural>"

# the unfiltered run of section 5.1, at the last commit that changes the source
gh workflow run measure.yml --ref main -f ref=8a364ffc066cad9d20f214d9ebb4b1d2b1d21c1d -f base=main \
  -f nordic=true -f r2025=true -f samples=true
```

## 9. Sources

RE-385 section 3.5, RE-397, #1658. No Autodesk source was read.
