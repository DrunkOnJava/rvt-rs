# RE-447: a wall whose line drifts a hair off a model axis: neither its record box's ends nor its line's are Revit's, and no per-end rule read from its joins chooses right at every end

**Date:** 2026-11-12
**Issues:** #1806 (two wall ends #1807 moved; folded into #1814), #1814 (the wall ends the box's join trim leaves short), #1807 (RE-446, which gave these walls their type's thickness), #1636 (rows registered before each run)
**Artefacts:** the BIM4LCA housing at building permit in concrete (`ARK_NordicLCA_Housing_Concrete_BuildingPermit.rvt`, sha256 `c6b53726a2d064654bfd9ebaa769bea5b85d8258ca980294a7f528bb239564e8`; Revit's IFC4 export `ARK_NordicLCA_Housing_Concrete_BuildingPermit_Revit.ifc`, `6a54e82f12f5101d1c46bf83257116c7151e52b76a680652d33f9982c63bab57`) and in timber (`ARK_NordicLCA_Housing_Timber_BuildingPermit.rvt`, `3df759b8b986c1e2ca71a691cdab16bcc7063a07e273003bf02cac559573c6d4`; export `9f107414642bc57b4403e58ec53f4a289ce8a955a7acba35a047ec15ef9e33ef`), the timber housing as built (`ARK_NordicLCA_Housing_Timber_As-Built.rvt`, `bfdc0b166cc127f59e13b975e063359f7c6b1223ea6fa260d792e51d09e2182a`; export `95d4eb0b5aa8073cd574e28235841a9ca187b330d4cb2b386071ad521e90364b`) and the concrete office as built (`ARK_NordicLCA_Office_Concrete_As-Built.rvt`, `747120005b4b35c7cb66ec61158aab50ded95e052e172cb387b86cd15b31d906`; export `b2f530eba8b4424c4542b98e9a2c7bbcc726367f21b2740f3074e4a6ce917382`); all in `research/bim4lca-models.tsv` and `research/bim4lca-measure.tsv`
**Probe:** `probes/examples/probe_re252_wall_ends.rs` (each wall's line, its native joins per end and the reader's reach fields)
**Runs:** Measure [43264161874](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43264161874) (`d052a376`, unfiltered), [43272620592](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43272620592) (`0860153e`, unfiltered), [43273535656](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43273535656) (`0860153e`, the four moved models, every wall end and the probe), [43162054233](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43162054233) (the probe on `7b577489`); all against `main` `314eeace`. The three commits are archived as `refs/archive/2026-11-12/cited/{d052a37668f9,0860153e9307,5c2f53a0685d}`.
**Status:** Measured negative. Nothing ships; `main` is unchanged. Where the two bodies differ, each is nearer Revit's at some ends and further at others, the same wall prefers the box at one end in one file and at the other end in another, and no rule read from the wall's joins chooses right at every end. What is wrong at those ends is the end itself (#1814).

## 1. The question

After #1807 gave two walls of the concrete housing at permit their type's thickness, an end of each moved from exact to 0.935 and 0.886 ft off Revit's (#1806). Both walls' lines run along a model axis but drift 4.9e-8 and 6.9e-8 ft across it over 16.0 and 26.9 ft. `wall_centreline_body` (`src/ifc/export_content.rs`) takes a line as axis-parallel only when a component of its unit vector is within 1e-9, a test of the angle, which these miss (3.1e-9, 2.6e-9). In the angled branch the box closure divides by that component and fails, and the fallback draws the wall from one end of its line to the other, where an axis-parallel wall keeps its record box's extent, which its joins trimmed. Can such a wall be drawn so that every end is at least as near Revit's as on `main`?

## 2. Three bodies tried

| Commit | Rule | Run | Result |
| --- | --- | --- | --- |
| `d052a376` | the axis tests take the drift in feet (1e-6 ft): such a wall is axis-parallel, box extent, no rotation | 43264161874 | the two ends exact again; four other ends further off; three window type GlobalIds broke (below) |
| `0860153e` | in the angled branch only, a drifting line keeps the box's extent at the line's own angle | 43272620592, 43273535656 | the same ends as `d052a376`; window types as on `main` |
| `5c2f53a0` (not pushed) | as `0860153e`, but only at an end whose join (`m_controlJoinsSet`) names a wall the reader gives no line; the line's end elsewhere | predicted from 43273535656 | 4 exact ends gained, 2 further off |

The window types: `door_symbol_flipped` reads a door's or window's flip from the side of its host wall's line its centre lies on, within 1e-9 ft. Snapping a drifting wall to the axis through its start moved that line by the drift, and a window centred on the line crossed it: "global_id agrees" fell 1,289 to 1,287 (concrete permit) and 1,417 to 1,416 (timber permit). Keeping the line's angle (`0860153e`) leaves them as on `main`.

## 3. Every end where the box and the line differ

From the unbounded `wall_ends_list` of run 43273535656 (error against Revit's export in feet, + = rvt-rs's wall shorter there):

| File | Wall | End | `main` (line) | box | Nearer Revit's | Join names a wall with no line | Unlined rule |
| --- | ---: | --- | ---: | ---: | --- | --- | ---: |
| concrete housing, permit | 1346777 | start | +0.886 | +0.000 | box | none | +0.886 |
| concrete housing, permit | 1347016 | start | +0.886 | +0.000 | box | names 1347017 | +0.000 |
| concrete housing, permit | 1347016 | end | +0.394 | +1.280 | line | none | +0.394 |
| concrete housing, permit | 1347019 | start | +0.443 | +1.378 | line | none | +0.443 |
| concrete housing, permit | 1347019 | end | -0.935 | -0.000 | box | names 1347018 | -0.000 |
| concrete housing, permit | 1424143 | end | +0.344 | +1.280 | line | none | +0.344 |
| concrete housing, permit | 1426918 | end | +0.344 | +1.280 | line | none | +0.344 |
| timber housing, permit | 1346757 | start | +0.633 | +0.000 | box | names 1346762 | +0.000 |
| timber housing, permit | 1346757 | end | +0.326 | +0.702 | line | names 2188775 | +0.702 (worse) |
| timber housing, permit | 1346777 | start | +0.376 | +0.000 | box | none | +0.376 |
| timber housing, permit | 1347019 | start | +0.697 | +0.000 | box | none | +0.697 |
| timber housing, permit | 1347019 | end | +0.205 | +0.902 | line | names 1347018 | +0.902 (worse) |
| timber housing, permit | 1347020 | start | +0.328 | -0.000 | box | none | +0.328 |
| timber housing, permit | 1347020 | end | +0.435 | +1.132 | line | none | +0.435 |
| timber housing, permit | 1347022 | start | +0.697 | +0.000 | box | none | +0.697 |
| timber housing, permit | 1347022 | end | +0.633 | +0.000 | box | none | +0.633 |
| timber housing, permit | 1437591 | end | -0.891 | -0.515 | box | none | -0.891 |
| timber housing, permit | 1437807 | end | +0.326 | +0.702 | line | none | +0.326 |
| timber housing, permit | 1566544 | end | +0.326 | +0.702 | line | none | +0.326 |
| timber housing, as built | 1437591 | end | -0.891 | -0.515 | box | none | -0.891 |
| concrete office, as built | 3016448 | end | -0.246 | +0.000 | box | names 1795247 | +0.000 |

21 ends: the box is nearer at 12, the line at 9. Wall 1347019's start is nearer with the line in the concrete file and with the box in the timber file; its end the other way round.

## 4. Per file

| File | Walls | Ends within 0.001 ft: `main` / box at both ends / unlined rule | Ends within 1.0 ft: `main` / box at both ends |
| --- | ---: | --- | --- |
| concrete housing at permit | 345 | 608 / 611 / 610 | 665 / 661 |
| timber housing at permit | 355 | 585 / 591 / 586 | 689 / 688 |
| timber housing as built | 367 | 598 / 598 / 598 | 694 / 694 |
| concrete office as built | 607 | 1,110 / 1,111 / 1,111 | 1,190 / 1,190 |

The other 115 of 119 exports are byte for byte `main`'s with either body.

## 5. The rules tried

Each rule picks the box or the line at each of the 21 ends, from the fields `probe_re252_wall_ends` prints for the wall and the walls its end's control join names (run 43273535656). A rule is right at an end where it picks the nearer of the two.

| Rule: take the box's end where | Right | Wrong at |
| --- | ---: | --- |
| the box lengthens the wall | 17 / 21 | the four ends where rvt-rs's wall is too long (negative in §3), where the box's cut-back is nearer |
| the end's control join names a wall the reader gives no line (`5c2f53a0`) | 11 / 21 | two ends where the line is nearer (timber permit 1346757 end, 1347019 end), and eight where the box is |
| the end's own `JoinEndGStep` is not empty: a weak index, an omitted wall or an exposed face | 19 / 21 | timber permit 1347022 start and office 3016448 end, where the step is empty and the box is nearer |

The third rule never takes the box where the line is nearer, so at these ends it is never worse than `main`. It moves 10 ends nearer, eight of them to Revit's end exactly. It is not shipped. It was found by fitting these same 21 ends, and no other wall of the 119 exports drifts, so nothing outside the fit tests it. Adding either of the other two rules to it makes 20 of 21, which is a fit, not a rule.

Where Revit's end lies says why no rule built from the box or the line can be right everywhere. The check takes each end of §3, moves the line's end by its error to Revit's end, and finds the nearest plane of `planes` (`gline` and its layer offsets) on the walls the end's control join names that crosses the wall's line:

- At all 9 ends where the line is nearer, Revit's end lies within 0.0001 ft of a named wall's layer plane, inside its faces: 0.049, 0.262, 0.492 or 0.590 ft in from the face. For example, concrete housing at permit, 1347016's end: 1346715's plane at x = -88.76405 ft, 0.4921 ft inside its face at -88.27192 ft. A box end, which lies on a face, cannot reach it.
- At 4 of the 12 ends where the box is nearer, Revit's end is a named wall's face. The other 8 name walls the reader gives no line, so their planes cannot be checked.

Which plane Revit stops at is the layer-priority join: the joined walls' layers by function. The record gives `planes` as offsets with no layer function or priority, so the choice cannot be made from it.

## 6. What follows

The ends in the table are wrong on `main` as well; the box and the line are two wrong answers there. #1814 is the work: where Revit's wall ends at a join the reader resolves no reach for, measured against the faces of the walls it names. #1806's two ends (1347019's end, 1347016's start of the concrete housing at permit, 0.935 and 0.886 ft off since #1807) are acceptance rows of #1814. The 1e-9 tests of `wall_centreline_body` stay; any fix of them has to come with the end rule, since alone it only trades one end for the other.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=0860153e9307b6bf0be133bb100d7284013fa946 -f base=main \
  -f nordic=true -f models="nordic-housing-concrete-permit nordic-housing-timber-permit nordic-housing-timber-as-built nordic-office-as-built" \
  -f scorers=wall_ends_list -f probe=probe_re252_wall_ends -f probe_scorer=-
```

`tools/re/wall_ends_list.py` lists every wall with an end off by more than 0.001 ft from this change on; it printed the 40 largest, which hid most of these ends (run 43273535656 took the scorer from the branch that first made the change).
