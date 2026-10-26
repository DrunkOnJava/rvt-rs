# RE-241 — What decides a curtain mullion's IsExternal

**Date:** 2026-10-26
**Issues:** #35 (parameters), W7 of [`docs/release-1.0-criteria.md`](../../docs/release-1.0-criteria.md); follows RE-193 §6, RE-226 §2, RE-239 §5
**Artefacts:** the BIM4LCA office as built and at building permit (Revit 2024, Revit's IFC4 exports), as Measure fetches them
**Runs:** Measure [40293040792](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40293040792) (`examples/probe_re241_mullion_record_scalars.rs`: the mullion's, its type's and its host's record scalars, ranked by `tools/re/record_scalars_vs_ifc.py`), [40295387624](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40295387624) (the type's and host's ElementIds, and `tools/re/group_purity_vs_ifc.py`), [40297366460](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40297366460) (the host's saved parameters and its type's Function) and [40300458738](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40300458738) (the mullion's own saved metadata)
**Result:** negative. A mullion's `IsExternal` is that of the curtain wall holding it on 10,002 of 10,003 mullions, but no field of the mullion's record, its type's or its host's, no saved parameter of the host or the mullion, and not the host type's Function says which curtain walls' mullions are external.

## 1. The values

Revit's export of the office as built writes `Pset_MemberCommon.IsExternal` on all 10,003 curtain mullions, true on 498, and `Pset_PlateCommon.IsExternal` on its 260 curtain panels, true on 30 (at permit 10,031 mullions, 498 true). These are 10,003 of the 13,882 property values the office's export writes that rvt-rs does not (W7). revit-ifc (CLEANROOM.md source 5) decides `IsExternal` in `CategoryUtil.IsElementExternal`: a parameter of that name, then the type's Function, then rules by category, then for a family instance its host. RE-193 §6 and RE-226 §2 measured the host rule as written negative: most mullions' hosts are exterior curtain walls whose mullions are not external. RE-239 §5 found no saved Yes/No or text parameter on the mullions.

## 2. Measured

The probe prints, for each member and plate the export writes, every number, boolean and short string of its native record, its type's (`T.`) and its host's (`H.`), the type's and host's ElementIds, the host's saved parameters (`HP.`, RE-207), its type's Function (`HF.function`, RE-185), and the mullion's own saved metadata (`MP.`, `MR.`, `MF.`, `MC.`, `MD.`). `record_scalars_vs_ifc.py` ranks each field's best single value as a rule ("true where the field is this"), and `group_purity_vs_ifc.py` how many elements share the majority value of their field's group:

| field | best rule, as built | groups pure |
|---|---:|---:|
| the host's ElementId (`ID.host`) | 9,902 of 10,003 (true on host 1906414) | 10,002 of 10,003, 220 hosts |
| the mullion's `FamilyInstance.m_hostParam` | 9,626 | 9,787, 110 groups |
| any host record field (`H.SWall.m_isStructuralSignificant` best) | 9,603 | |
| any host saved parameter (`HP.-1001101` best) | 9,584 | |
| the host type's Function (RE-185: 1 exterior, 0 interior) | not ranked (under 9,513) | 9,140 mullions on exterior curtain walls, 1,123 on interior ones |
| the mullion's own saved metadata | no rule above the host's | 10 references, `MD.complete`, and on 4 mullions two empty IFC export parameters |
| every field of the mullion's own record and its type's | 9,525 (`MullionFamRef.m_edgeTag`, 1,838 groups) | |

Panels: 258 of 260 pure by host; 28 of the 30 true are curtain wall 1417009's.

`IsExternal` is true on every mullion of curtain wall 1906414 (397) and 1417009 (100) and on one of the four of 1417025, and false on the mullions of the other 217 curtain walls. No host field the probe prints holds a value shared by 1906414 and 1417009 and by none of the others: the nearest is also on 149 curtain walls whose mullions are false. At least 8,642 of the 9,505 mullions that are not external sit on exterior curtain walls, so it is not the host type's Function either.

## 3. What rvt-rs does

Nothing changes: rvt-rs writes no `IsExternal` on curtain mullions and panels, as before (RE-193). The 10,003 and 260 values stay missing in W7. What the probe has not read: the host's geometry against the building's envelope, and the panels' and mullions' saved graphics. A rule needs a signal that separates curtain walls 1906414 and 1417009 from the other 217.

## 4. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=main -f base=none -f nordic=true \
  -f models="nordic-office-*" -f scorers=none \
  -f probe=probe_re241_mullion_record_scalars -f probe_scorer=record_scalars_vs_ifc
gh workflow run measure.yml --ref main -f ref=main -f base=none -f nordic=true \
  -f models="nordic-office-*" -f scorers=none \
  -f probe=probe_re241_mullion_record_scalars -f probe_scorer=group_purity_vs_ifc
```
