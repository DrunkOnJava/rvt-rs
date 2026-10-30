# RE-248 — A curtain mullion's NetSurfaceArea

**Date:** 2026-10-26
**Issues:** #1016 (quantities), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-243 (mullions drawn as an extrusion of their section) and RE-241 (the mullion records)
**Artefacts:** the BIM4LCA office as built (Revit 2024, Revit's IFC4 export), as Measure fetches it
**Runs:** Measure [40359749620](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40359749620) (`tools/re/member_surfaces_list.py`), [40361254271](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40361254271), [40362011371](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40362011371) and [40364293181](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40364293181) (`tools/re/member_surface_ends_list.py`), and [40363065053](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40363065053) (`examples/probe_re241_mullion_record_scalars.rs` with `tools/re/mullion_ends_vs_records.py`), each on `main` after v0.9.0, and [40372730294](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40372730294), both again once the tools carry mapped items' transformations and index boxes with their margin: every count the same
**Result:** negative: a mullion's `NetSurfaceArea` is its section's perimeter times its length plus 0, 1 or 2 times its section's area, the ends its Revit solid keeps; neither its records nor rvt-rs's boxes of the mullions say which ends those are. None is written.

## 1. What Revit's value is

revit-ifc's `NetSurfaceAreaCalculator` (CLEANROOM.md source 5) sums the faces of the element's Revit solid, and Revit's solid of a mullion lacks the faces where it is joined to another element. On the office as built Revit's export writes `Qto_MemberBaseQuantities.NetSurfaceArea` on all 10,003 curtain mullions. On the 9,631 drawn as one extrusion whose side along their `Length` rvt-rs's box gives, it is the section's perimeter times the `Length` plus a whole number of the section's areas:

| mullion | ends counted: 0 | 1 | 2 |
|---|---:|---:|---:|
| not upright (6,045) | 4,416 | 736 | 893 |
| upright (3,586) | 0 | 438 | 3,148 |

and it follows the mullion's type: of the 50 × 50 mm painted aluminium type, 4,403 of 5,443 mullions that are not upright count no end and 3,042 upright ones both; of the plain 50 × 50 mm type, 417 not upright count both and 364 upright ones one. The rest are 252 whose length rvt-rs's box does not give along an axis (on curved or turned curtain walls) and 120 whose ends are not a whole number (cut mullions, RE-243).

## 2. What does not tell which ends

- **Rvt-rs's boxes.** Counting the ends of each mullion's box that no other mullion's box covers (a point 2 mm past the end's centre) gives Revit's count on 4,416 + 621 + 485 of 9,631; counting only boxes of mullions running across it, on 1,035. 4,057 mullions that are not upright count no end though both their ends are free of every crossing box, and 2,177 upright ones count both where crossing boxes cover both.
- **The records.** `mullion_ends_vs_records.py` ranks every field of each mullion's native record, its type's and its host's (RE-241's probe) against each count. The best is right on 4,992 of the 6,119 mullions that are not upright and on 3,579 of the 3,677 upright ones (it takes the count from Revit's own body, so it scores the 9,796 drawn as one extrusion). A mullion's own record holds no join: its `MullionFamRef` names only its host, a driver and an edge tag.

Which mullion is continuous where two meet is the curtain wall type's join condition and each mullion's own Make Continuous or Break at Join setting, and the faces dropped follow from the joins at each end. Neither was read when this was first written; the type's join condition is read in section 3 and says nothing, which leaves the host curtain wall's grid (its line segments, the edge tags `MullionFamRef.m_edgeTag` names) and each mullion's own setting.

## 3. The host curtain wall's type (added 2026-10-27)

RE-241's probe now also prints the record of each mullion's host's type (`HT.`, its `NewCurtainWallType` and `SpacingRule` objects) and that type's saved parameters (`HTP.`), and `mullion_ends_vs_records.py` ranks them with the rest (Measure [40446361666](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40446361666), the office as built, 10,263 mullion records). The type's join condition, `NewCurtainWallType.m_autoJoinCond`, is 0 on every one of them, so it says nothing of which ends count; of its saved parameters 403070 is empty and -1140422 takes two values (1312 on 9,980 mullions, 1245 on 283) that do not follow the counts. No field of the type ranks above the best rules of section 2: 4,992 of 6,119 for the mullions that are not upright and 3,579 of 3,677 for the upright ones. What is left is each mullion's own Make Continuous or Break at Join setting, which no field ranked yet carries, and the grid the mullions lie on.

## 4. What rvt-rs does

Nothing changes: `NetSurfaceArea` is not written, and stays missing on all 10,003 mullions. Writing the section's sides plus the ends no box covers would be wrong on about half of them.

Update (2026-10-30, RE-268): the ends follow from the boxes after all, read as which end coincides with its own continuation's end, not which end a box covers; 6,606 of the 10,003 are written.

## 5. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=d6d5f4b5bd4ada9149a18aa22dfd7a81736a5db7 -f base=none -f nordic=true \
  -f models="nordic-office-as-built" -f scorers="member_surfaces_list member_surface_ends_list"
gh workflow run measure.yml --ref main -f ref=d6d5f4b5bd4ada9149a18aa22dfd7a81736a5db7 -f base=none -f nordic=true \
  -f models="nordic-office-as-built" -f scorers=none \
  -f probe=probe_re241_mullion_record_scalars -f probe_scorer=mullion_ends_vs_records
gh workflow run measure.yml --ref main -f ref=d7a09ac35f8337d899d5b8c718389407b82a94ab -f base=none -f nordic=true \
  -f models="nordic-office-as-built" -f scorers=none \
  -f probe=probe_re241_mullion_record_scalars -f probe_scorer=mullion_ends_vs_records
```
