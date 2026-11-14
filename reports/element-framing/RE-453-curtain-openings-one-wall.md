# RE-453: a door or window with no curtain grid found whose list names one wall in all, and that wall a curtain wall, is a part of it

**Date:** 2026-11-13
**Issues:** #1912 (the 84 doors of the BIM4LCA offices that Revit's export aggregates into a curtain wall and RE-452 left out); follows RE-452 and RE-46
**Artefacts:** the four BIM4LCA office files that Measure reads with Revit's own IFC4 export of each (VTT's ARCH archive, CC BY-SA 4.0: the concrete office as built and at building permit, the timber office as built and at building permit; `research/bim4lca-measure.tsv`); Autodesk's `Snowdon Towers Sample Architectural.rvt` (Revit 2024, sha256 `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1`) with Revit's own IFC4 export of it (bldrs-ai/test-models, no licence: held locally and never committed, so only counts are given here); Autodesk's `rac_advanced` saves of 2025 to 2027 (public, `research/autodesk-sample-projects.tsv`).
**Scorers:** `tools/re/curtain_openings_vs_ifc.py` (new) with `probes/examples/probe_1912_curtain_openings.rs`; `tools/re/aggregates_vs_ifc.py`
**Runs:** Measure [43439124110](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43439124110) and [43441941672](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43441941672) (the probe and its scorer on the offices, the `rac_advanced` saves and Snowdon Towers); of the change, [43444354218](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43444354218) (unfiltered), [43444395366](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43444395366) (Snowdon Towers, export kept) and [43450027177](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43450027177) (the exports of both sides kept for every `rac_advanced`, `rac_basic`, `rme_advanced` and `rme_basic` save of 2015 to 2027)
**Status:** Positive. Against Revit's export, 268 of the 268 office doors and 6 of the 6 Snowdon Towers doors that it aggregates into a curtain wall are in the same wall now, and none is put in a wall it does not put them in; the saves of every Autodesk sample project that holds such doors now agree with each other.

## 1. The question

RE-452 gave a door or window with no curtain grid found the one curtain wall its list names when the element data of an id in its list names that wall too. Against Revit's export of the four BIM4LCA offices it put 184 of the 268 doors that Revit aggregates into a curtain wall in the same wall and left 84 (21 in each office) out (RE-452 section 6). What do those 84 say about their wall, and what separates them from the doors that Revit leaves in an ordinary wall although their list names a curtain wall: the Vitoria 2018 door that RE-452's first form got wrong, and the three Snowdon Towers doors RE-46 found?

## 2. What the lists say

`probe_1912_curtain_openings` prints, for every door and window with an instance record, the walls its list names, the curtain walls among them, the grid it finds, the walls its element data names, the walls whose record box holds it, and what else its list holds; `curtain_openings_vs_ifc.py` sets each against the wall Revit's export aggregates it into.

- **All 84 are alike, and unlike the 184.** Each names Revit's wall in its list, rvt-rs counts that wall as a curtain wall, and it is the only wall the list names. The element data of the ids in the list names it at no place at all, not one, and the door's record box is not inside the wall's (the box rule gives 0 of 66 on each office). What else the list holds does not separate them from the doors RE-452 has: the same kinds of ids, two door records with element data, a dozen with no element record, two to five with element data.
- **Every door of the offices that names a curtain wall names it alone.** There are 66, 68, 67 and 67 of them, and Revit aggregates every one into the wall its list names. No door of the offices names a curtain wall together with another wall.
- **Snowdon Towers Architectural separates the two cases.** Revit's export aggregates 6 doors into a curtain wall. Each of the 6 names that wall alone; rvt-rs had 5 and missed the sixth, of the same kind as the 84. The three doors that name curtain wall 1506500 and that Revit leaves standalone (RE-46) name two walls each, the curtain wall and another.

| rule | the four offices (268 doors Revit aggregates) | Snowdon Towers (6, and the 3 standalone) |
| --- | --- | --- |
| RE-452 (the element data must name the wall) | 184 right, 0 wrong, 0 false, 84 missed | 5 right, 0 false, 1 missed |
| the list names one curtain wall, whatever else it names (RE-452's first form) | 268 right, 0 false | 6 right, **3 false** |
| the list names one wall in all, a curtain wall | 268 right, 0 wrong, 0 false, 0 missed | 6 right, 0 wrong, 0 false, 0 missed |
| the record box holds the door | 0 right, 268 missed | 0 right, 6 missed, 3 false |

## 3. The rule

An opening with no grid found whose list names exactly one wall in all, and that wall is a curtain wall, is a part of that wall. An opening whose list names a curtain wall and another wall keeps RE-452's rule: it is a part of the curtain wall when the element data of an id of its list names it too. (`attach_curtain_walls`, `walls_named`.)

The reason is what a door in a curtain wall is. It is a panel of the grid, so its list names the curtain wall that holds it and nothing else; a door in an ordinary wall names its host, and may name a curtain wall beside it. On the models measured the two cases do not overlap.

## 4. Rows, registered on #1912 before the Measure of this change was dispatched, and what the runs read

Registered with the change ([comment on #1912](https://github.com/DrunkOnJava/rvt-rs/issues/1912#issuecomment-6855955943)), read on the runs above:

1. **Holds.** On each office the doors only Revit aggregates go from 21 to 0 and the doors in the same whole from 46, 48, 47 and 47 to 67, 69, 68 and 68; no door is in a different whole, none is aggregated only by rvt-rs, and no window changes.
2. **Holds.** Snowdon Towers Architectural, from the kept export scored against Revit's own: 6 of 6 in the same wall (5 before), 126 neither, 0 only rvt-rs; the three doors that name curtain wall 1506500 and their host stay standalone.
3. **Holds.** No model with Revit's export changes in `aggregates_vs_ifc`, `opening_hosts_vs_ifc` or any other scorer beyond the four offices' door lines, and the spent Vitoria 2018 model's openings are as on `main`.
4. **Holds for the scored models, and read for the others.** 50 of the 146 exports of the unfiltered run change, more than the four offices and Snowdon Towers I named: the `rac_basic`, `rme_basic` and `rme_advanced` samples, the `rac_advanced` saves of 2015 to 2023 (where RE-452's element-data arm cannot read the data) and the six `racadv` models. `diagnostics.json` is identical on all 121 that have one and the same-project gates pass. None of those has a Revit export, so the exports of both sides were kept for every save of the four sample projects and the doors and windows that are parts of a curtain wall counted:

| project | saves | parts before (RE-452) | parts now |
| --- | --- | --- | --- |
| `rac_advanced` | 2015 to 2027 (13) | 0 on 2015 to 2023, 11 on 2024 to 2027 | 17 on all 13 |
| `rac_basic` | 2015 to 2027 (13) | 0 on 2015 to 2023, 2 on 2024 to 2027 | 5 on all 13 |
| `rme_advanced` | 2015, 2019, 2024 to 2027 (6) | 0, 0, then 7 | 17 on all 6 |
| `rme_basic` | 2015, 2019, 2023 to 2027 (7) | 0, 0, 0, then 7 | 17 on all 7 |

   No part on the base side is lost. The saves of one project now agree where they disagreed by release, which is what a rule right on the elements it reaches would give.

## 5. Not changed, and what is left out

- Windows. No window of the offices or of Snowdon Towers is in a curtain wall in Revit's export, so the rule has nothing to be right or wrong about for them; it applies to them as RE-452 does.
- A door whose list names no wall, or two curtain walls, is as before.
- The Vitoria 2018 model leaves the runner as counts: whether its door names two walls is read from the change's Measure, not from the model.

## 6. Reproduce

```bash
gh workflow run measure.yml --ref main \
  -f ref=<sha of this change> -f base=none -f nordic=true -f samples=true -f sample_years="2024 2024" \
  -f models="nordic-office-as-built nordic-office-permit nordic-office-timber-as-built nordic-office-timber-permit sample-2024-snowdon*architectural*" \
  -f scorers=none -f probe=probe_1912_curtain_openings -f probe_scorer=curtain_openings_vs_ifc
python3 tools/re/curtain_openings_vs_ifc.py <probe.txt> <Revit's export>.ifc --list
```
