# RE-439: the mesh path writes the NetSurfaceArea of a mullion whose partner end the boxes keep: 34 and 33 values, none different

**Date:** 2026-11-11
**Issues:** #1790 (70 mullions of the BIM4LCA concrete office as built and 69 of the timber office that RE-438 reads right and gives no value), #1767 and #1046 (W7: the BIM4LCA office's quantities), #1636 (the rules rest on the BIM4LCA models); follows RE-438 (#1771), RE-404 (`NetSurfaceArea` from the saved mesh) and RE-433
**Artefacts:** the BIM4LCA concrete office as built and at permit and the timber office as built and at permit (Revit 2024), with Revit's IFC4 exports of the two as built
**Scorers:** `tools/re/qtos_vs_ifc.py` and `tools/re/ifc_element_diff.py`
**Runs:** Measure 43110403389 (head `4c2bb990` against `main` at `6439e62f`, section 5)
**Status:** Positive, with a measured limit: the concrete office as built and at permit gain 34 `NetSurfaceArea` values and the timber office as built and at permit 33, every one equal to Revit's, none different. The other 36 and 36 of the 70 and 69 mullions lie beside a mullion the rule leaves unread and are #1794's (RE-441); RE-438 took all of them for the mesh path's refusals, which this run corrects.

## 1. The question

After RE-438 the rule that reads a curtain mullion's kept ends gives Revit's count on all 131 mullions of the concrete office as built that RE-268 left out for an end of their own (130 on the timber office), and 60 of each are written. The other 70 (69) each have a partner end that an oblique mullion or a mullion across inside keeps (a kept count of 2: 50 have two partner ends and 20 have one) and get no value, although their meshes are Revit's solids and the value is the whole surface of the mesh. Why are they refused, and is the refusal right?

## 2. The check

`add_mullion_surface_areas` (`src/ifc/saved_meshes.rs`) draws a mullion's `NetSurfaceArea` from its mesh: the faces along it plus the end faces its solid keeps, `kept` of them, where `kept` is RE-268's count on the boxes of its curtain wall. The boxes do not see a continuation on another curtain wall, so a mullion whose mesh finds a partner (`partner_ends`: the placed ends of two meshes coincide, along one axis, with equal areas) at an end that `kept` counts is refused: `with_partner > 2 - kept`. The check is right for a partner the boxes missed. A partner the boxes saw and kept (RE-438: Revit counts such an end where an oblique mullion or one across inside lies at it) is counted by `kept` for a reason, and the same check refuses the mullion all the same. A mullion with a kept count of 2 and a partner end is refused whenever its mesh finds the partner. RE-438 took the check for the cause of all of the 70 of the concrete office as built, which are exactly the mullions with a kept count of 2 and a partner end among the 131 released (none of the 108 mullions it gave a value has a partner end); no build printed the refusal, and section 5 shows that was true of 34 of them.

## 3. The change

`kept_ends_with_partners` (`src/research/partition_schema_mvp/mullion_ends.rs`) returns, with each mullion's kept count, which of its two ends are kept and have a partner (`KeptEnds`: the box end at its `-axis` side, then the one at its `+axis` side, the order `end_points` takes them in); `kept_ends` is its first number. The reader keeps both for the mesh path, and the check (`partners_the_boxes_missed`, `src/ifc/mesh_volume.rs`) counts the partner ends the mesh finds that are not an end the boxes kept with a partner they saw: a mullion is refused when that count is more than the ends the boxes hide. A partner at a hidden end is still expected, and one at any other end, where the boxes saw none or hid the end, still refuses.

The first version of this change subtracted counts, the kept ends with a partner from the partner ends the mesh finds. A count cannot tell which end it is at: a kept count of 2 with a partner the boxes saw at one end allows one mesh partner end, at either end, so a partner the mesh missed at the end the boxes saw it (the two end faces' areas differ by more than 0.1%, where the boxes read the sections to 3 mm) together with a continuation on another curtain wall that it found at the other end passes with a value one end face too large. rvtReview's read of that commit named the slack. Telling the ends apart closes it, and the mesh's ends come in no order along the axis: `prism_along` lists the two end planes in the order the triangles first meet them, so the check does not match them to the boxes' by index but by the side of the other each lies on along the mullion's record axis (`Placed.axis`, the box's own `axes[along]` that `remember_mullion_axes` stores, so the sign is the boxes'); two ends with no length between them are not told apart, and a partner at either is counted as missed. The check sits between the old one and the first version: it refuses whatever the old one refused except a partner at an end the boxes saw one at and kept.

Failure-first tests of the pure helpers: the joint two pieces of one line make is hidden and counts no kept end with a partner, an oblique brace at the joint keeps it and the end of each piece at the joint counts one, in the `-axis`, `+axis` order (`mullion_ends.rs`); and for `partners_the_boxes_missed` (`mesh_volume.rs`) the cancellation case (a partner at the end the boxes kept with none is missed whatever lies at the other), the mesh's ends listed against the axis and the axis turned over, and ends with no length between them.

## 4. The rows (registered on #1636 before the run)

Measured against main with #1771 landed (`base=6439e62f`, the squash of #1771's head `276716ab`; main then also holds #1769 and #1773, which this change does not touch), unfiltered with `holdout=true` and `keep_ifc`, so that the delta is this change's alone ([comment 6807805750](https://github.com/DrunkOnJava/rvt-rs/issues/1636#issuecomment-6807805750)). The rows were registered for the first version of the check; the run measures the per-end version of section 3, which they describe the same way (row 4 is now exact):

1. The refused mullions are written, each equal: `NetSurfaceArea` equal rises by up to 70 on the concrete office as built and up to 69 on the timber office as built, and by the same kind at permit, none different; one whose mesh the path does not read as a prism stays without, counted.
2. Nothing gets worse: no census row gains a `different`, equal never falls, the property census does not move, no mullion that had a value has it changed.
3. What moves: only mullions' `NetSurfaceArea` appears, no element is added or removed, and only the exports that hold a mullion with a kept partner end move.
4. What stays refused: a partner the boxes did not see still refuses; the 399, the 19 and 23 and the 18 are unchanged.

## 5. What Measure found

Measure run [43110403389](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43110403389), unfiltered (`samples`, `nordic`, `r2025`, `holdout`, `bimcollab`, `racadv` and `keep_ifc`): head `4c2bb990` (this change with `main` at `6439e62f` merged in) against `main` at `6439e62f`. `same-project gates` and the semantic check pass on `4c2bb990`. **126 exports: 121 identical, 5 moved**, the four BIM4LCA offices and Snowdon Towers Architectural. `ifc_element_diff` on the five: no element is added or removed, and the only change is `Qto_MemberBaseQuantities.NetSurfaceArea` of an `IfcMember`, added on 34, 34, 33 and 33 mullions of the offices (concrete as built and at permit, timber as built and at permit) and on 4 of Snowdon (whose Revit export has none to meet), changed on none.

`NetSurfaceArea` of the offices' mullions against Revit's (equal, different, missing), base to head:

| office | base | head |
| --- | --- | --- |
| concrete, as built | 9,496, 0, 507 | 9,530, 0, 473 |
| concrete, permit | 9,524, 0, 507 | 9,558, 0, 473 |
| timber, as built | 7,396, 0, 480 | 7,429, 0, 447 |
| timber, permit | 7,280, 1, 591 | 7,313, 1, 558 |

The one different at timber permit is 3029161 (RE-438). Over the 42 models with a Revit export the quantity census's equal rises from 155,742 to 155,876 (+134) with different at 26 and missing from 18,072 to 17,938, and the property census does not move (0 of the 42 differ).

The rows of section 4, read on [#1636](https://github.com/DrunkOnJava/rvt-rs/issues/1636#issuecomment-6809106303):

1. **Held on its bound, with a shortfall the mesh path does not explain.** The 34 and 33 mullions written are each equal, none different. Of the 70 (concrete) and 69 (timber) released mullions that keep a partner end, 29 and 28 of those written have a partner end at both ends (15 with an oblique mullion at each, 14 and 13 with a mullion across inside at each) and 5 have one (an oblique mullion at each end). The other 36 and 36 are not written, and the mesh check is not the reason: each has a mullion the rule leaves unread within about 9 cm of an end (the second debug build of RE-438, with the new rule's left-out set): beside overlapped mullions only on 28 and 28, beside mullions left out only on 2 and 2, beside both on 6 and 5, and on 1 of the timber office none was found by that search. A mullion beside an unread mullion is not read (`meets_one_not_read`), which suffices; whether the check would also refuse any of the 36 is not told by this run (RE-441's, which reads those beside overlapped mullions, will tell it).
2. **Held.** No census row gains a `different`, equal never falls, the property census does not move, and no mullion that had a value has it changed (changed 0 in the element diff).
3. **Held.** 5 of 126 exports move and 121 are byte for byte `main`'s; only mullions' `NetSurfaceArea` appears. `rac_advanced` does not move (RE-438 moved all four of its saves), and Snowdon Towers Architectural gains 4.
4. **Held.** The 399 mullions of the concrete office as built left out for a neighbour are as they were (48 given a value by RE-438, 399 without one), the 19 and 23 with a lone inside end and the 18 with no axis have no value, and no value of the run is different.

The 507 values the concrete office as built lacked are now 473: 19 with a lone inside end, 18 with no axis, 399 left out for a neighbour, the 36 above, and 3304572 (RE-438: a 35 mm piece read right and not written, cause not examined). The timber office's 480 are 447 (23, 18, 369, 36 and 1).

The change adds no read, so it is not timed.

## 6. Not done

- The 36 and 36 released mullions beside an unread mullion and the 399 and 369 left out for a neighbour: [#1794](https://github.com/DrunkOnJava/rvt-rs/issues/1794) (RE-441, draft PR #1795), which reads the neighbours of overlapped mullions (28 and 28 of the 36 lie beside overlapped mullions only). The 2 and 2 beside mullions left out only wait for the lone-inside mullions.
- A slack the per-end check leaves, and the first check had: a mullion with one kept end and no partner the boxes saw, whose mesh misses the boxes' partner at the hidden end and finds a continuation on another curtain wall at the kept end, has one mesh partner end for one hidden end and is written with a value one end face too large. Telling it needs the boxes' hidden ends as well as the kept ones. No value of this run is different, so none of the 67 written is in it.
- 3304572, the 19 and 23 with a lone inside end, and the 18 with no axis: as RE-438 left them.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=4c2bb990bf105b206e5590da2c284bab37cc0ffb \
  -f base=6439e62fd908c620e7d337f497a7918d88a2e26b -f samples=true -f nordic=true -f r2025=true \
  -f holdout=true -f bimcollab=true -f racadv=true -f keep_ifc=true
```

`tools/re/ifc_element_diff.py base/<model>/model.ifc head/<model>/model.ifc` on each moved pair gives the added values by tag, and `tools/re/qtos_vs_ifc.py`'s lines of the run (`Qto_MemberBaseQuantities.NetSurfaceArea`) the census above. The measured commit is archived as `refs/archive/2026-11-11/cited/4c2bb990bf10` (the branch is deleted when the pull request lands), and the classification of the 70 and 69 by what lies beside them reads the second debug build of RE-438 (commit `72f6a699f2f0`, `refs/archive/2026-11-11/cited/72f6a699f2f0`, Measure run 43090090858) with the rule of section 4 of RE-438 applied offline.
