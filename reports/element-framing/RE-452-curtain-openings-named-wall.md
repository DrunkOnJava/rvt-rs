# RE-452: a curtain wall's door or window whose grid is not found takes the one curtain wall its list names when the element data names it too: 184 of the 268 doors the BIM4LCA offices leave out of their curtain walls, and the curtain door of `rac_advanced` 2027, are parts of their wall again

**Date:** 2026-11-13
**Issues:** #1865 (the curtain door 216844 of Autodesk's `rac_advanced` is a part of its curtain wall on the 2025 and 2026 saves and of none on the 2027 save); follows RE-46 (a part names one curtain wall), RE-72 (a part's grid names its wall) and RE-449 (a part that names two curtain walls takes the one its grid's element data names), and corrects RE-449's statement about this door
**Artefacts:** Autodesk's sample projects `racadvancedsampleproject.rvt` of Revit 2025 (sha256 `512e9188b245ba58fcc918630e4388b836df37042aa07de6db0e223b48de25d1`, 17,068,032 bytes) and Revit 2027 (`a88eb9f85e1e2d47bbc91ee214b178c2d7075822b6d08694f59e3e6f259024ed`, 17,133,568), both public and redistributable (`research/autodesk-sample-projects.tsv`); the four BIM4LCA office files that Measure reads with Revit's own IFC4 export of each (VTT's ARCH archive, CC BY-SA 4.0: the concrete office as built and at building permit, the timber office as built and at building permit; `research/bim4lca-measure.tsv`); the spent Vitoria 2018 model, which has Revit's export and is reported as counts only.
**Scorer:** `tools/re/aggregates_vs_ifc.py` counts `IfcDoor` and `IfcWindow` from this change on (it counted `IfcMember`, `IfcPlate`, `IfcCurtainWall`, `IfcStairFlight` and `IfcRailing`)
**Runs:** Measure [43424342015](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43424342015) (the kept exports of the 2025 and 2027 saves), [43425870343](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43425870343) and [43429686082](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43429686082) (the first form of the rule, unfiltered, the second scored on doors), [43429692923](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43429692923) (the rule as it ships, unfiltered, scored on doors) and [43433248231](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43433248231) (its kept exports of the 2025 to 2027 saves)
**Status:** Positive for the rule as it ships: no door or window of any model with a Revit export is put in a wall Revit's export does not put it in, and 184 of the 268 doors that Revit aggregates into a curtain wall and the four offices did not are now in the same wall. The first form of the rule, which took the named wall alone, put 268 of 268 right and one door of a Revit 2018 model wrong, and is a measured negative (section 4). 84 of the 268 are still left out (section 6).

## 1. The question

Of the 80 curtain parts that #1865 found in a curtain wall on the 2025 save of `rac_advanced` and in none on the 2027 save, 79 were corner mullions (RE-449). The 80th is a door, `M_Curtain Wall Sgl Glass:216844`. Why is it a part of the curtain wall 162023 on 2025 and 2026 and not on 2027?

## 2. What the exports say

On the 2025 save the `IfcRelAggregates` of the curtain wall 162023 lists the door among its parts, and the door has no spatial containment of its own. On the 2027 save the same relation lists the wall's mullions and not the door, and the door is contained in the storey directly. The wall, the door's property sets and its type are the same on both saves.

## 3. Why

`attach_curtain_walls` gives a curtain part its wall in two places. A curtain mullion or panel (`products`) takes the curtain wall its grid belongs to, or with no grid the one curtain wall its reference list names, or with none the one whose record box holds it (RE-46, RE-62, RE-72), and since RE-449 the one the element data of an id in its list names where the list names two. A door or window (`openings`) took a curtain wall only through the grid: one grid wall, or nothing. The grid comes from `scan_curtain_grid_walls`, whose offsets (RE-72, 85 and 478) are not found on 2027, so a curtain door lost its grid there for the reason the mullions lost theirs, and the fallback RE-449 added was never reached by an opening.

RE-449 said this door "names that one wall on both saves and is `(0, 1)` on both, so the match gives it the wall on both". That holds for a mullion or panel. The door is an opening, and `probe_1865_part_walls`, which counts it among the parts, did not run the code the door goes through.

## 4. The first form of the rule failed

The rule as first written gave an opening with no grid whose list names exactly one curtain wall that wall (`cedf41be`). Rows were registered on #1865 before its run, and two of them failed:

- **52 of the 146 exports changed** (94 identical) and 2 of 121 diagnostics files, where the registered row said no model whose openings have a grid would move. The rule fires wherever an opening names one curtain wall and finds no grid, on every release from 2015 on.
- **One door of a model with a Revit export is put in a curtain wall that Revit's export does not put it in.** On the spent Vitoria 2018 model (counts only) `opening_hosts_vs_ifc` "the wall Revit fills" goes from 130 to 129, "no host; Revit fills an opening" from 3 to 4, the openings rvt-rs writes from 133 to 132 (Revit's 136), and `aggregates_vs_ifc`, which this change teaches to count doors, reports one `IfcDoor` that only rvt-rs aggregates. A door or window in an ordinary wall can name a curtain wall in its list.
- **What it got right:** scored on doors for the first time, the four BIM4LCA offices go from 66, 68, 67 and 67 doors that only Revit aggregates to none, and from 1 door in the same whole to 67, 69, 68 and 68: 268 of 268, with no door in a different whole.

One wrong door in about 270 is not a rule that is never wrong, and the rule stays narrowed until it is (CLAUDE.md, "Every Revit value a reader adds should measure as equal").

## 5. The rule as it ships

An opening with no grid whose list names exactly one curtain wall is a part of it **when the element data of an id in its list names that wall too**, read as RE-449 reads a mullion's (`scan_walls_in_element_data`, no fixed offset). The element-data header exists on Revit 2024 and later, so no opening of a 2015 to 2023 save takes a wall by this arm.

Rows, registered on #1865 for this form after its runs were dispatched and before either was read, and what the runs read:

1. **Holds.** The 2027 `rac_advanced` aggregates the door 216844 into 162023 as the 2025 and 2026 saves do (kept exports of 43433248231), and its `Pset_DoorCommon.IsExternal` is true, as on 2025.
2. **Holds.** No door or window that Revit's export puts in an ordinary wall moves: `opening_hosts_vs_ifc` and `openings_vs_ifc` do not change on the spent Vitoria 2018 model or on any other model with a Revit export, and the Vitoria exports are identical.
3. **Holds.** On the models with a Revit export `aggregates_vs_ifc` shows no `IfcDoor` or `IfcWindow` in a different whole, and none that only rvt-rs aggregates. The four offices go from 66, 68, 67 and 67 doors that only Revit aggregates to 21 each, and from 1 door in the same whole to 46, 48, 47 and 47: 184 of 268 now match.
4. **Holds.** 125 of 146 exports are byte for byte as before and 121 of 121 diagnostics files are identical. The 21 that change are the four offices, the `rac_advanced`, `rac_basic`, `rme_advanced` and `rme_basic` samples of 2024 to 2027 (16) and Snowdon Towers Architectural; none of 2015 to 2023, no held-out model, no housing model.

## 6. Not changed, and what is left out

- **84 of the 268 office doors** that Revit aggregates into a curtain wall (21 in each of the four files) are still left out. The rule needs the element data of an id in the door's list to name the wall; for these it does not, or their list does not name the wall, and which it is has not been read. The first form would take them, and also the Vitoria door. (Read in RE-453: each names the wall in its list, alone, and its element data never names it; a door whose list names one wall in all takes it, which the Snowdon Towers doors Revit leaves standalone do not meet.)
- **Revit 2015 to 2023.** The element-data reader the rule needs does not exist for the 32-bit releases, so their openings keep the grid arm only. No Revit export of a 2015 to 2023 curtain door exists in Measure to say what the named wall would give there beyond the one Vitoria door.
- **The 5 mullions** of #1865 whose lists name no wall (RE-449) are unchanged.
- No Revit export of `rac_advanced`, the housing or Snowdon exists to score their openings: those 17 exports that change (the samples and Snowdon) are held to the same project's other saves, not to Revit's.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref <a branch holding tools/re/aggregates_vs_ifc.py with IfcDoor and IfcWindow> \
  -f ref=<sha of this change> -f base=<its merge-base with main> \
  -f samples=true -f nordic=true -f r2025=true -f racadv=true -f holdout=true -f bimcollab=true -f families=true
gh workflow run measure.yml --ref main -f ref=<sha> -f base=none -f scorers=none -f samples=true \
  -f sample_years="2025 2027" -f models="sample-2025-racadvancedsampleproject sample-2027-racadvancedsampleproject" -f keep_ifc=true
```
