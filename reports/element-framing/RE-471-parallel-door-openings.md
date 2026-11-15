# RE-471: a door's opening in a second wall along its own is ten feet deep, centred on its own opening's face on the door's −Y side

**Date:** 2026-11-15
**Issues:** #1165 (the openings nothing fills: a door's further cut in a second wall); follows RE-321 and RE-338 (the walls' cut steps), RE-398 section 3 (Snowdon's 38 steps of this kind)
**Scorer:** `tools/re/unfilled_openings_vs_ifc.py` with a research trace on `research/1165-parallel-door-trace` (not to land; its last commit is archived as `refs/archive/2026-11-15/cited/9bf4c959c8de`)
**Artefacts:** the BIM4LCA office (concrete and timber, as built and at building permit) and housing (concrete and timber, as built and at building permit), with Revit 24's IFC4 exports (CC BY-SA 4.0, as Measure fetches them); Autodesk's Snowdon Towers Architectural 2024 sample (sha256 `33271010919acb2a0d0d040851393a511d86ea7fd9a1f4c054797ae03e5e44a1`) and Revit's IFC4 export of it (local only, never committed)
**Runs:** Measure [43666219918](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43666219918) (first trace, the office), [43667090229](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43667090229) (the planes in Revit's export, eight files; rvt-rs's IFCs kept, Snowdon's among them), [43668183253](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43668183253) (the rule from rvt-rs's own export, eight files), all on `main`; the change: [43668574318](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43668574318)
**Status:** Positive. Measured on the change in section 5.

## 1. The steps

A wall's record names each door that cuts it (RE-321). A door whose `m_bCutSomething` is 0 in a wall other than its host cuts that second wall too, and rvt-rs writes the cut as the door's own opening box where the second wall runs across the door's wall. Where the second wall runs along it, rvt-rs wrote nothing and counted the step as `door_second_wall_parallel_to_door_wall`: 9 and 9 on the office as built and at permit, 7 and 7 on the timber office, 10 and 10 on the timber housing, 9 and 9 on the concrete housing, and 38 on Snowdon Towers Architectural (Measure 43664481547 on `0e581f05`).

## 2. What Revit's export writes for them

The trace lists each door opening in a wall, keyed by the door's Tag and the wall's, that Revit's export writes and rvt-rs's does not. Beside each it prints the door's body, the opening the door fills, and the door's placement, all from Revit's export and in internal feet through each export's site placement. On the eight BIM4LCA files, 82 of these openings are ten feet deep along one plan axis. On every one of the 82:

- the depth is 10 ft to within 1e-4 ft;
- the other plan axis and the height equal the opening the door fills, to within 1e-3 ft;
- the depth is centred on a face of the opening the door fills.

| file | openings | centred on the face on the door's local −Y side |
|---|---:|---:|
| concrete housing, as built | 8 | 8 |
| concrete housing, permit | 8 | 8 |
| timber housing, as built | 12 | 12 |
| timber housing, permit | 12 | 12 |
| office, as built | 11 | 11 |
| office, permit | 9 | 9 |
| timber office, as built | 11 | 11 |
| timber office, permit | 11 | 11 |

The other planes tested fail:

- **The face farther from the door's body** holds on 81 of 82. It misses door 3513424 in wall 3477168 on both timber office files: its body (162.479 to 162.742 ft) sits in the middle of its opening (162.151 to 162.893 ft).
- **The door's placement** lies on the face on 36 of the 82. It is 0.0656 ft (20 mm) past the face, away from the wall, on 38, and 0.879 ft inside it on 8.
- **The lower or the upper face** splits 54 to 28 across the files.

The other door openings the trace lists are not ten feet deep: they equal the door's own opening or are a door's cut in a wall it is not hosted in (`door_cuts_a_wall_it_is_not_hosted_in_as_its_own`). They are not this case.

## 3. The rule from rvt-rs's own export

The same trace builds the box from rvt-rs's export alone. It takes the opening the door fills there, and the Y axis of the door's `IfcAxis2Placement3D`, which is the door's rotation about Z turned a further quarter turn. The box is that opening widened across the door's wall to 10 ft, centred on the opening's face on the side the Y points away from. It equals Revit's box to within 0.01 ft (the scorer's tolerance) on 82 of 82 BIM4LCA openings: 8, 8, 12, 12, 11, 9, 11 and 11, in the order of the table. On Snowdon Towers Architectural, scored locally on rvt-rs's export from run 43667090229 against Revit's local export, it holds on 24 of 24.

## 4. The rule as written

`door_cut` (`src/ifc/wall_cut_openings.rs`) widens the door's own box across the door's wall to `DOOR_PARALLEL_DEPTH_FEET`, 10 ft, about that face. It does so for a second wall running along the door's wall, where the door's Y (`door_y_axis`) lies across the walls. A door whose Y lies along its walls keeps the reason `door_second_wall_parallel_to_door_wall`: which face Revit takes is then not known.

## 5. Measured on the change

To be filled from Measure 43668574318 before the pull request is ready.

## 6. Not claimed

- Where the ten feet come from in Revit's exporter, and whether another depth occurs on other files: every opening read is ten feet.
- Snowdon holds 38 steps of this kind and 24 openings Revit writes for them. What the rule writes for the other 14 is read in section 5.

## Reproduce

```bash
# the scorers come from a branch: put the archived trace on one first
git push origin refs/archive/2026-11-15/cited/9bf4c959c8de:refs/heads/re-471-trace
gh workflow run measure.yml --ref re-471-trace -f ref=afbaabb5326c5f5ce56e519755df309503dc624c -f base=none \
  -f nordic=true -f models="nordic-office-* nordic-housing-*" -f scorers=unfilled_openings_vs_ifc
# each model's unfilled_openings_vs_ifc.txt: the `trace` lines, `trace rule tally` and `trace rvt-rs rule tally`
```
