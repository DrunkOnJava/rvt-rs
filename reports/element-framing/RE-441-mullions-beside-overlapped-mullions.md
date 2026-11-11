# RE-441: the mullions beside an overlapped mullion are read like any other, but for two kinds: 353 and 320 values on the offices, none different

**Date:** 2026-11-11
**Issues:** #1794 (the neighbours of overlapped mullions: most of the 507 mullions of the BIM4LCA concrete office as built and the 480 of the timber office that still have no `NetSurfaceArea`), #1767 and #1046 (W7: the BIM4LCA office's quantities), #1636 (the rules rest on the BIM4LCA models); follows RE-400 (the neighbours of overlapped and left-out mullions are not read), RE-438 and RE-439
**Artefacts:** the BIM4LCA concrete office as built and at permit and the timber office as built and at permit (Revit 2024), with Revit's IFC4 exports of the two as built
**Scorers:** `tools/re/qtos_vs_ifc.py`, `tools/re/ifc_element_diff.py` and `tools/re/mullion_ends_vs_boxes.py`
**Runs:** Measure 43115049478 (head `eb0fcdb1` against `4c2bb990`, section 5)
**Status:** Positive, with a bound exceeded: the concrete office as built and at permit gain 353 `NetSurfaceArea` values and the timber office 320 (as built) and 316 (at permit), every one equal to Revit's, none different. The offline count (279 and 242) was an undercount: all of it is written, and 74 and 78 more, equal too. 120 and 127 mullions are still without a value. The 17 exports that move include 13 with no Revit export to meet.

## 1. The question

RE-268's rule leaves a mullion out when one of its ends lies within 61 mm (`NEAR_FEET`) of the box of a mullion that is left out or overlapped, another mullion lying over it. RE-400 chose that for the 159 overlapped mullions of the timber office at permit and their neighbours, where reading the neighbours was wrong on some. After RE-438 these neighbours are the largest part of what the offices still lack in `Qto_MemberBaseQuantities.NetSurfaceArea`, itself the largest single gap of either office's quantity census (507 and 480 values without one, against 280 and 285 for the next quantity). Is the rule wrong for the neighbours of overlapped mullions, and where?

## 2. The offline reading

The second debug build of RE-438 printed, for every mullion of the offices, what lies at each of its ends (`mullion-kd`) and the outcome the rule gave it. Joined with Revit's count of kept ends per mullion (from its mesh and its `NetSurfaceArea`, RE-438 section 2), the mullions whose only unread neighbours are overlapped ones can be read as any mullion is, and the result set against Revit's count: **314 mullions of the concrete office as built and 280 of the timber office; the rule gives Revit's count on 312 and 272, is wrong on 0 and 7, and Revit gives the other 2 and 1 no value** (3029185 and 3029301, pieces 75.6 mm long to which Revit gives no `NetVolume` either).

By what lies at their ends (right / wrong):

| ends | concrete | timber |
| --- | --- | --- |
| a partner at both | 116 / 0 | 117 / 0 |
| a partner at one, nothing at the other (a chain end) | 135 / 0 | 81 / 4 |
| nothing at either | 32 / 0, and 2 with no value | 49 / 0, and 1 |
| a mullion across centred on an end | 29 / 0 | 25 / 3 |

The seven wrong, all on the timber office:

- 3639879, 3592337 and 3539461 have a mullion across centred on an end, and Revit keeps an end the rule hides. The 54 right ones of that kind (25 timber, 29 concrete) all have a partner at the other end, as 3539461 does, so nothing tells that one apart; 3639879 and 3592337 have none.
- 3089951, 3089980, 3090009 and 3135427 are the ends of chains of 0.1 m pieces, and Revit hides the free end, where nothing that the boxes show lies. Two overlapped verticals (50 x 50 mm) stand on the bar there, with their bottom faces on its top face, and the axis of one lies on the plane of the free end (0.0 m from it, the other 0.025 m): the end's point, at the middle of the end face and 25 mm under the vertical's box, shows neither. Of the 216 chain ends the rule gets right (81 timber, 135 concrete), none has an overlapped mullion with its axis on the plane of an end (the axes lie 0.025 m or more from it, the end at a side face). The 9 right mullions with no partner at either end and such an axis (5 timber, 4 concrete) are left out with the 4.

## 3. The change

A mullion with an end within `NEAR_FEET` of a mullion that was left out is not read, as before. One beside an overlapped mullion is read like any other unless it is one of the two kinds above (`kept_ends_with_partners`, `src/research/partition_schema_mvp/mullion_ends.rs`): a mullion across is centred on one of its ends (`Neighbours::centre`), or an end that has no partner has the axis of an overlapped mullion near it on its plane (`centred_on_end_plane`: the overlapped mullion runs across, and its centre is within `TOLERANCE_FEET` of the end's plane along the mullion's axis). Offline that releases 279 of the concrete office as built's mullions and 242 of the timber office's, Revit's count on every one, and leaves out the 7 wrong, the 54 right ones with a centred end, and the 9 with an axis on the plane of a free end. Failure-first tests of the pure `kept_ends_with_partners`: a stub beside an overlapped pair is read when the pair's axes lie at its side face and left out when one is centred on the plane of its free end; a mullion with a bar centred on an end is read alone and left out beside the pair; a mullion beside a mullion left out is still not read.

## 4. The rows (registered on #1636 before the run)

Measured against #1791's head (`base=4c2bb990`, RE-439), unfiltered with `holdout=true` and `keep_ifc`, so that the delta is this change's alone:

1. The released mullions are written, each equal: `NetSurfaceArea` equal rises on the concrete office as built by up to 279 and on the timber office as built by up to 242, and at permit by the same kind, none different; one whose box or mesh the path does not read stays without, counted.
2. Nothing gets worse: no census row gains a `different`, equal never falls, `rvt-rs values Revit does not write` stays 0 (3029185 and 3029301 get no value, or are shown to be left out by name), the property census does not move, and no mullion that had a `NetSurfaceArea` has it changed (changed 0 in the element diff).
3. What moves: only mullions' `NetSurfaceArea`; no element is added or removed; only exports that hold such a mullion move (the four offices, and possibly `rac_advanced` and Snowdon Towers Architectural, which RE-438 moved).
4. What stays out: the seven wrong mullions, the mullions with a centred end or an axis on the plane of a free end, the neighbours of a mullion left out, the 19 and 23 with a lone inside end and the 18 with no axis have no value, as before.

## 5. What Measure found

Measure run [43115049478](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43115049478), unfiltered (`samples`, `nordic`, `r2025`, `holdout`, `bimcollab`, `racadv` and `keep_ifc`): head `eb0fcdb1` (this change on #1791's head `4c2bb990`) against `4c2bb990`. `same-project gates` pass on `eb0fcdb1`. **126 exports: 109 identical, 17 moved**: the four BIM4LCA offices, Snowdon Towers Architectural and the 2024 to 2027 saves of `rac_advanced`, `rme_advanced` and `rme_basic`. `ifc_element_diff` on the 17: no element is added or removed, and the only change is `Qto_MemberBaseQuantities.NetSurfaceArea` of an `IfcMember`, added on 353 (concrete as built), 353 (concrete permit), 320 (timber as built), 316 (timber permit), 50 (Snowdon), 122 (each `rac_advanced` save) and 3 (each `rme_advanced` and `rme_basic` save), changed on none.

`NetSurfaceArea` of the offices' mullions against Revit's (equal, different, missing), base to head:

| office | base | head |
| --- | --- | --- |
| concrete, as built | 9,530, 0, 473 | 9,883, 0, 120 |
| concrete, permit | 9,558, 0, 473 | 9,911, 0, 120 |
| timber, as built | 7,429, 0, 447 | 7,749, 0, 127 |
| timber, permit | 7,313, 1, 558 | 7,629, 1, 242 |

The one different at timber permit is 3029161 (RE-438). Over the 42 models with a Revit export the quantity census's equal rises from 155,876 to 157,218 (+1,342) with different at 26 and missing from 17,938 to 16,596; `rvt-rs values Revit does not write` stays 29 (the concrete offices' 0 and the timber offices' 3, as on both sides); the property census does not move (0 of 42 differ).

The rows of section 4, read on [#1636](https://github.com/DrunkOnJava/rvt-rs/issues/1636#issuecomment-6809537179):

1. **Held in substance, over its bound.** The 353 and 320 values written on the offices as built are each equal to Revit's. Of them 279 and 242 are the offline set of section 3, all of it (the set is contained in what the run writes; the 29 and 28 mullions with a centred end are not written, nor are the 4 free ends with an overlapped axis on their plane on the concrete office and 8 of the 9 on the timber office, the ninth, a right one, being written because the rule's distance is not the offline search's); 74 and 78 more are written, which the bound did not count: 29 and 29 are the released mullions of RE-439's 36 that lie beside overlapped mullions only (28 with a partner end, and 3304572, the 35 mm piece whose cause RE-438 had not examined), and 45 and 49 are neighbours of the mullions RE-438 released, which the debug build that the offline count reads (before RE-438) put with the neighbours of mullions left out and which now lie beside overlapped mullions only.
2. **Held.** No census row gains a `different`, equal never falls, the property census does not move, `rvt-rs values Revit does not write` does not change, 3029185 and 3029301 (Revit gives them no value) get none, and no mullion that had a value has it changed (changed 0 in every element diff).
3. **Held, with exports I had not listed.** Every export that moved holds such a mullion, but the row named the offices, `rac_advanced` and Snowdon: the 2024 to 2027 saves of `rme_advanced` and `rme_basic` move too (3 values each). None of the 13 exports other than the offices has a Revit value to meet, so their 488 (`rac_advanced`, 122 a save), 24 (`rme_advanced` and `rme_basic`) and 50 (Snowdon) values are additions of a rule that is right on all 673 of the offices as built, not checks of it. The held-out pairs and the housing do not move.
4. **Held.** The mullions with a mullion across centred on an end, and the free ends under an overlapped axis, are not written; the 19 and 23 with a lone inside end, the 18 with no axis and the mullions beside a mullion still left out have no value, as before.

What is still without a value, on the concrete office as built (120, from 473): 19 with a lone inside end and 18 with no axis (RE-438); 8 of RE-439's 36, which lie beside mullions left out; and 75 left out for a neighbour: 29 with a centred end, 4 with a partnerless end under an overlapped axis, the 2 to which Revit gives no value, 23 beside mullions still left out (19 beside the lone-inside mullions, 4 that have an overlapped one as well), and 17 for which the offline search finds no unread mullion beside them (the rule's own distance, the box of the neighbour within 6 cm of the end's point, is not the search's, which takes the axis within 9 cm). The timber office as built has 127 (from 447) by the same causes (23, 18, 8 and 78).

The change adds no read, so it is not timed.

## 6. Not done

- The 54 mullions with a mullion across centred on an end that the rule reads right (29 concrete, 25 timber) stay out with the 3 it reads wrong: nothing in the boxes tells them apart (section 2). Revit's join condition at the crossing, which the boxes do not hold, is the likely input.
- The relation that tells the 4 chain ends, a mullion standing on the face of a bar and centred on its end plane, is looked for beside overlapped mullions only. RE-268 tests the end's centre point (3 mm), which a mullion standing on the bar's face, 25 mm off the point, does not hold; beside a single mullion no value is different, so none of the written values is wrong for it, and a test of the end face rather than its point is not tried.
- 120 and 127 mullions without a value: the 19 and 23 with a lone inside end (Revit counts such an end on 14 and hides it on 5 that read alike), the 18 with no axis, the neighbours of those mullions, and the 17 of the concrete office that the rule leaves unread for a reason the offline search does not find.
- The 13 exports other than the offices carry 562 values that no Revit export checks (RE-438 added 32 a save to `rac_advanced` and 24 to Snowdon on the same footing).

## 7. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=eb0fcdb168d5f4882f9bb21a8b32a0869ef42551 \
  -f base=4c2bb990bf105b206e5590da2c284bab37cc0ffb -f samples=true -f nordic=true -f r2025=true \
  -f holdout=true -f bimcollab=true -f racadv=true -f keep_ifc=true
```

`tools/re/ifc_element_diff.py base/<model>/model.ifc head/<model>/model.ifc` on each moved pair gives the added values by tag, and `tools/re/qtos_vs_ifc.py`'s lines of the run (`Qto_MemberBaseQuantities.NetSurfaceArea`) the census above. The measured commit is archived as `refs/archive/2026-11-11/cited/eb0fcdb168d5` (the branch is deleted when the pull request lands). The offline reading of section 2 is the second debug build of RE-438 (commit `72f6a699f2f0`, `refs/archive/2026-11-11/cited/72f6a699f2f0`, Measure run 43090090858) joined with Revit's counts of kept ends; the distance of an overlapped mullion's axis from the end plane is read from the mullions' end points of the same build.
