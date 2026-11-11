# RE-433: a mullion with two mitred ends and both kept gets the whole surface

**Date:** 2026-11-11
**Issues:** #1746 (151 mullions with a bent partner have Revit's `NetSurfaceArea` and none from rvt-rs), #1046 (W7: the BIM4LCA office's quantities), #1636 (the rules rest on the BIM4LCA models); follows RE-404 (a mullion's `NetSurfaceArea` from its mesh), RE-421 (the oriented-box mullions are offered their saved mesh) and RE-432 (a bent continuation hides no end face)
**Artefacts:** the BIM4LCA concrete office as built and at permit and the timber office as built and at permit (Revit 2024), with Revit's IFC4 exports of them
**Runs:** Measure [43062004581](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43062004581) (a throwaway build of RE-432's head at `04524fc4` that prints, for each mullion, the stage that refuses it, on the office as built and the timber office as built); the change's own run is in section 4
**Status:** Positive for the cause: the 131 mullions of the concrete office as built that have a bent partner and a known `kept` and still get no value are all refused for one reason, two mitred end faces, and so are 134 of the timber office as built; the change's Measure result is in section 4.

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

Not yet read: the unfiltered Measure of the change against RE-432's head is running. The rows it is held to are registered on #1636 (https://github.com/DrunkOnJava/rvt-rs/issues/1636#issuecomment-6801975912).

## 5. Reproduce

```bash
# the throwaway build that prints why each mullion is refused (kept under refs/archive/2026-11-11/cited/04524fc48c8f)
gh workflow run measure.yml --ref research/1746-nsa-refusals -f ref=04524fc48c8f9574a42286507f6e634a7cdddca5 -f base=none -f nordic=true -f models="nordic-office-as-built nordic-office-timber-as-built" -f scorers=none
# then, in each model's rvt-ifc.log, the lines `mullion-dbg <tag> ...`
```
