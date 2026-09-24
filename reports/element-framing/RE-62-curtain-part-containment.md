# RE-62 — Curtain-wall parts whose record names no wall

**Date:** 2026-09-24
**Result:**
- RE-46 gives a curtain-wall panel or mullion the curtain wall its reference list names. On Snowdon Towers, 89 mullions and 12 panels name no wall at all. Their lists name only sibling mullions, and following those siblings reaches no wall either.
- Revit's own export aggregates 89 of these mullions and 3 of these panels under a curtain wall, mostly the "Curtain Wall:Solar Panels" walls. Each lies inside that wall's record box. So a part whose record names no curtain wall now takes the one curtain wall whose record box contains its own, within 0.05 ft.
- Against Revit's `IfcRelAggregates` on Snowdon (a scratch join by `Tag`, as in #370):

| | before | after |
|---|---:|---:|
| mullions with Revit's parent | 1,505 | 1,594 |
| panels with Revit's parent | 462 | 463 |
| parts Revit aggregates and rvt-rs does not | 107 | 17 |
| parts with a different parent from Revit's | 1 | 1 |

- The other panels the rule attaches (46) are empty curtain panels that Revit's export leaves out (#309).
- The different parent is RE-59's nested curtain wall, 1593696.
- Core Interior, RE1 Architecture and the MIT house export byte-identical IFC apart from the timestamp. Storey containment is unchanged.

-----

## 1. The reading

`partition_schema_mvp::attach_curtain_walls` now handles three cases:
- a part whose list names exactly one curtain wall takes it (RE-46);
- a part whose list names none takes the one curtain wall whose record box (`element_record_bbox`) contains the part's, within `CURTAIN_PART_BOX_TOLERANCE_FEET` (0.05 ft);
- a part naming two or more takes none.

The tolerance is measured:
- at 0.001 or 0.01 ft, 86 of the 92 parts Revit aggregates are placed;
- at 0.05 ft all 92 are, as they are at 0.25 ft;
- no tolerance gives a wrong parent.

## 2. What this does not do

- 17 parts that Revit aggregates stay unparented: 16 panels (some name two or more curtain walls, 9 lie inside no curtain wall's box) and a mullion that Revit aggregates under a stair.
- A nested curtain wall, one used as another's panel, is not modelled (#370).
