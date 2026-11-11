# RE-432: a continuation bent off a mullion's axis does not hide its end face

**Date:** 2026-11-11
**Issues:** #1735 (8 mullion `NetSurfaceArea` values one end face short; its duplicate #1734 is closed), #1046 (W7: the BIM4LCA office's quantities), #1636 (the rules rest on the BIM4LCA models); follows RE-248 (a mullion's `NetSurfaceArea` counts the end faces its solid keeps), RE-268 (which ends are kept, from the boxes), RE-404 (the value from the saved mesh) and RE-421 (the oriented-box mullions are offered their saved mesh)
**Artefacts:** the BIM4LCA concrete office as built (Revit 2024) and its permit state, with Revit's IFC4 exports of both
**Scorers:** `tools/re/mullion_bent_partners.py` (new), `tools/re/mullion_ends_vs_boxes.py`
**Runs:** Measure [43034275062](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43034275062) (the census whose base side first read 9 different), [43049477290](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43049477290) (the office as built exported with `--no-saved-meshes` on `main` at `4fe00620`, which `mullion_bent_partners.py` and `mullion_ends_vs_boxes.py` read against Revit's export); the change's own run is in section 5
**Status:** Positive. All 8 values are RE-268's partner bent off the axis; the change's Measure result is in section 5.

## 1. The question

The census of Measure run 43034275062 read the office as built at 36,321 of 39,754 quantities equal and **9 different** (at permit 36,680 of 40,006 and 9), where the criteria page said 1. The 9 are the `Length` of mullion 3027671 (4.269995 against 4.270047 m, older and not this) and the `Qto_MemberBaseQuantities.NetSurfaceArea` of 8 mullions (tags 3029300, 3028703, 3028014, 3028863, 3028399, 3029291, 3024631, 3029290), each 0.0025 m² (one 50 mm square end face) under Revit's. Bisecting the earlier censuses of the office as built: at `5d291201` (the base of RE-423) the quantities read 1 different, with `NetSurfaceArea` equal 9,166, different 0, missing 837; at `16411282`, `main` after RE-423, 9 (equal 9,249, different 8, missing 746), and 9 at every base since. RE-421 (#1696) offers the office's 325 and 327 oriented-box mullions the saved mesh, which RE-404's rule then meets: of the 91 values it writes on them, 83 are equal and 8 are one end face short. RE-404 was measured against RE-418's head, which holds no RE-421, and RE-421 without RE-404, so the pair landed unmeasured. What separates the 8 from the 83?

## 2. What the 8 have that the 83 do not

RE-404 writes `NetSurfaceArea` as the faces along the mullion's axis plus the end faces its solid keeps, and takes how many it keeps from RE-268, which hides an end where the mullion has a partner: a mullion that runs the same way (a cosine above 0.999, which allows 2.56 degrees), has an end within 3 mm of the end and its body beyond. Reading the office as built exported as boxes (run 43049477290), all 8 mullions have one end with a partner (the others' ends meet a perpendicular post's face, or nothing), and the partner's axis is **not** the mullion's: it is bent off it by 1.83 degrees (3029300), 1.94 (3024631, 3029290, 3029291) or 2.19 (the other four). `tools/re/mullion_bent_partners.py` counts, over the whole office, the ends that have a partner by the angle between the axes:

| angle between the axes | ends with a partner | mullions |
|---|---:|---:|
| under 0.01 degrees | 10,098 | 5,668 |
| 0.01 to 1 | 0 | 0 |
| 1 to 2 | 290 | 151 |
| 2 to 2.56 | 8 | 8 |

There is no partner between 0.01 and 1 degrees: a partner either coincides with the end or is bent by at least 1.8 degrees. The 159 mullions that have a bent partner (20 with one bent end, 139 with two, none with a coinciding partner as well) are the ones to hold against Revit's value, and Revit's solid (its own geometry, the surface area less the value over the end face, the NetVolume over the Length) leaves out **no end face on any of the 159**: a bent continuation hides none of the end, as a partner that coincides does (RE-248: Revit's value hides an end only where it coincides with the end of its continuation). RE-268's rule counted the bent partner as hiding the end, so the 8 mullions with one bent end and a value written read one end face short; the other 151 have no value on `main` (section 5 says how many the change gives one). Every mullion that has a value equal to Revit's has coinciding partners only: the 9,249 equal values of the office as built and the 7,530 of the mullions drawn as their box's extrusion (`mullion_ends_vs_boxes.py`).

## 3. The change

A partner hides an end only where it coincides with it, a cosine above 0.99999 (0.26 degrees, between the 0.01 degrees every coinciding partner is within and the 1.8 of the least bent), in RE-268's `kept_ends` (`src/research/partition_schema_mvp/mullion_ends.rs`, the boxes) and in RE-404's `partner_ends` (`src/ifc/mesh_volume.rs`, the meshes, whose consistency check leaves a mullion without a value where a partner stands at an end that `kept` counts). A bent neighbour is neither: the end stays counted, which is Revit's value on all 159. The two pure helpers have failure-first tests: a continuation bent by two degrees is no partner, and one a hundredth of a degree off it is.

## 4. What the registered rows say

Registered on #1636 before the run was read: https://github.com/DrunkOnJava/rvt-rs/issues/1636#issuecomment-6800586926. They said 157 mullions whose Revit value counts every end; Revit's own geometry gives 159 of 159 (the 2 more had no value in the rvt-rs export the first count used).

## 5. What Measure found

Not yet read: the unfiltered Measure of the change against `main` is running.

## 6. Not done

- The older `Length` of mullion 3027671 (4.269995 against 4.270047 m, 5e-5 m) is not this and stays.
- A bent partner on another model is not measured: the rule rests on the office, one project in two states.

## 7. Reproduce

```bash
# the office as built exported as boxes, kept
gh workflow run measure.yml --ref main -f ref=4fe0062012716869438a67f6c000b1fc9b28c0f4 -f base=none -f nordic=true -f models="nordic-office-as-built" -f scorers=none -f keep_ifc=true -f ifc_flags=--no-saved-meshes
# then, with the kept model.ifc and Revit's export of the office as built
python3 tools/re/mullion_bent_partners.py model.ifc ARK_NordicLCA_Office_Concrete_As-Built_Revit.ifc
python3 tools/re/mullion_ends_vs_boxes.py model.ifc ARK_NordicLCA_Office_Concrete_As-Built_Revit.ifc --list
```
