# RE-433: a mullion with two mitred ends and both kept gets the whole surface

**Date:** 2026-11-11
**Issues:** #1746 (151 mullions with a bent partner have Revit's `NetSurfaceArea` and none from rvt-rs), #1046 (W7: the BIM4LCA office's quantities), #1636 (the rules rest on the BIM4LCA models); follows RE-404 (a mullion's `NetSurfaceArea` from its mesh), RE-421 (the oriented-box mullions are offered their saved mesh) and RE-432 (a bent continuation hides no end face)
**Artefacts:** the BIM4LCA concrete office as built and at permit and the timber office as built and at permit (Revit 2024), with Revit's IFC4 exports of them
**Runs:** Measure [43062004581](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43062004581) (a throwaway build of RE-432's head at `04524fc4` that prints, for each mullion, the stage that refuses it, on the office as built and the timber office as built); and the change [43063558746](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43063558746) (unfiltered, `keep_ifc`, head `46db8574` against RE-432's head `4e83ba2e`)
**Status:** Positive for the cause: the 131 mullions of the concrete office as built that have a bent partner and a known `kept` and still get no value are all refused for one reason, two mitred end faces, and so are 134 of the timber office as built; and the change gives 528 mullions across the four office files their `NetSurfaceArea`, each Revit's, none different.

## 1. The question

RE-432 makes a continuation hide an end face only where it coincides with it, and leaves 151 mullions of the concrete office as built whose partner is bent with Revit's `NetSurfaceArea` and none from rvt-rs (#1746): their ends are now counted, yet no value is written. Why?

## 2. Which stage refuses them

`add_mullion_surface_areas` writes a value for a mullion that has a mesh that is a closed solid of flat faces and a prism along its axis (RE-418), a `kept` count (RE-268), a `Qto_MemberBaseQuantities` set, a partner check that agrees with `kept` (RE-404), and a prism whose ends `mullion_surface_area` reads. The throwaway build prints each mullion's outcome. On the concrete office as built, 10,003 mullions:

| outcome | mullions |
|---|---:|
| `NetSurfaceArea` already written by the boxes (RE-268's extruded mullions) | 7,532 |
| written from the mesh with `kept` 0 | 1,037 |
| written with `kept` 2 | 676 |
| written with `kept` 1 | 12 |
| refused, `kept` none (RE-268 leaves the mullion out) | 595 |
| **refused at `mullion_surface_area` with `kept` 2** | **131** |

All 131 of the last row are among the 159 mullions with a partner bent off the axis (RE-432): of those 159, 8 are the mullions RE-432 corrects, 131 are these, 18 have `kept` none and 2 have no mesh volume. The timber office as built has 134 of the last row. `mullion_surface_area` returns none where neither end face of the prism is square to the axis: both ends are mitred, as they are on a mullion with a bent joint at each end. RE-404 measured that a mitred end always counts when the other end is flat (184 of the housing's and 58 of the office's permit files') and left two mitred ends unread, for want of a measurement. Revit's own solid leaves out no end face of any of the 159 (RE-432), the 131 included, so the value is the whole surface: the faces along the mullion and both end faces.

## 3. The change

`mullion_surface_area` (`src/ifc/mesh_volume.rs`) gives a prism with two mitred ends the area of the faces along it plus both end faces where `kept` is 2, and none where it is 0 or 1, which contradicts that a mitred end always counts, as for one mitred end today. The pure helper has a failure-first test: two mitred ends with `kept` 0 or 1 are none, with 2 are the whole surface.

## 4. What Measure found

Run 43063558746 (126 exports each side; the base is RE-432's head, so only this change moves). The quantity census of the 42 models that have a Revit export: Revit's quantities 173,840 each side; equal 154,623 to 155,151 (+528); missing 19,184 to 18,656; **different 33 to 33**; the values rvt-rs writes that Revit does not 29 to 29. The property census does not move. By file:

| file | quantities | equal | different | missing |
|---|---:|---:|---:|---:|
| BIM4LCA concrete office, as built | 39,754 | 36,329 to 36,460 | 1 to 1 | 3,424 to 3,293 |
| BIM4LCA concrete office, at permit | 40,006 | 36,688 to 36,819 | 1 to 1 | 3,317 to 3,186 |
| BIM4LCA timber office, as built | 33,800 | 30,703 to 30,837 | 3 to 3 | 3,094 to 2,960 |
| BIM4LCA timber office, at permit | 33,663 | 30,297 to 30,429 | 23 to 23 | 3,343 to 3,211 |

All of it is `Qto_MemberBaseQuantities.NetSurfaceArea` (equal 32,731 to 33,259, different 8 to 8, missing 3,043 to 2,515): +131 and +131 on the concrete office, +134 and +132 on the timber office, the counts the debug build predicted for the mullions refused for two mitred ends. **4 of 126 exports move** (the four office files) and 122 are byte-identical, the held-out, Revit 2025 and Autodesk models among them. `tools/re/ifc_element_diff.py` on the 4 pairs: no element is only in one file and 528 elements differ, each a mullion that gains `Qto_MemberBaseQuantities.NetSurfaceArea` in the set it holds, nothing removed or changed; the only entity count that moves is `IfcQuantityArea`, by 528. `same-project gates` pass on the measured head. The concrete office as built reads 36,460 of 39,754 quantities equal and 1 different, at permit 36,819 of 40,006 and 1.

Held against the rows registered on #1636 (https://github.com/DrunkOnJava/rvt-rs/issues/1636#issuecomment-6801975912):

| row | result |
|---|---|
| 1 the mullions refused for two mitred ends gain Revit's value | **Held**: +131 on the concrete office as built (9,257 to 9,388) and at permit (9,285 to 9,416), +134 on the timber office as built (7,153 to 7,287) and +132 at permit (7,036 to 7,168), each equal, none different |
| 2 no census row gains a `different` | **Held**: `NetSurfaceArea` different 0, 0, 2 and 6 before and after; the property census does not move |
| 3 what moves | **Held**: 528 `NetSurfaceArea` values added, no value changed, no element added or removed, 122 of 126 exports byte-identical |
| 4 what stays out | **Held**: 615 mullions of the concrete office as built, 615 at permit, 587 and 698 on the timber office stay missing (RE-268 leaves them out, `kept` none) |

## 5. Not done

- The mullions RE-268 leaves out (`kept` none: 595 of the concrete office as built, with 18 of them bent-partner mullions) stay without a value; what leaves them out is [#1046](https://github.com/DrunkOnJava/rvt-rs/issues/1046)'s remaining mullion work.

## 6. Reproduce

```bash
# the change against RE-432's head
gh workflow run measure.yml --ref fix/1746-two-mitred-ends -f ref=46db857440d43ba2a70475c95725f5fb463fa029 -f base=4e83ba2e9aa27e16abb36210473b9853dc6dcebc -f samples=true -f nordic=true -f r2025=true -f holdout=true -f bimcollab=true -f racadv=true -f keep_ifc=true
# the throwaway build that prints why each mullion is refused (kept under refs/archive/2026-11-11/cited/04524fc48c8f)
gh workflow run measure.yml --ref research/1746-nsa-refusals -f ref=04524fc48c8f9574a42286507f6e634a7cdddca5 -f base=none -f nordic=true -f models="nordic-office-as-built nordic-office-timber-as-built" -f scorers=none
# then, in each model's rvt-ifc.log, the lines `mullion-dbg <tag> ...`
```
