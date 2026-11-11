# RE-441: the mullions beside an overlapped mullion are read like any other, but for two kinds

**Date:** 2026-11-11
**Issues:** #1794 (the neighbours of overlapped mullions: most of the 507 mullions of the BIM4LCA concrete office as built and the 480 of the timber office that still have no `NetSurfaceArea`), #1767 and #1046 (W7: the BIM4LCA office's quantities), #1636 (the rules rest on the BIM4LCA models); follows RE-400 (the neighbours of overlapped and left-out mullions are not read), RE-438 and RE-439
**Artefacts:** the BIM4LCA concrete office as built and at permit and the timber office as built and at permit (Revit 2024), with Revit's IFC4 exports of the two as built
**Scorers:** `tools/re/qtos_vs_ifc.py`, `tools/re/ifc_element_diff.py` and `tools/re/mullion_ends_vs_boxes.py`
**Runs:** not yet run (section 5)
**Status:** Not yet measured: the change is written and its rows are registered on #1636 before the run.

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

Not yet run.

## 6. Not done

Pending.

## 7. Reproduce

Pending.
