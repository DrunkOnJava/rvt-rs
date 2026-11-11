# RE-439: the mesh path writes the NetSurfaceArea of a mullion whose partner end the boxes keep

**Date:** 2026-11-11
**Issues:** #1790 (70 mullions of the BIM4LCA concrete office as built and 69 of the timber office that RE-438 reads right and the mesh path refuses), #1767 and #1046 (W7: the BIM4LCA office's quantities), #1636 (the rules rest on the BIM4LCA models); follows RE-438 (#1771), RE-404 (`NetSurfaceArea` from the saved mesh) and RE-433
**Artefacts:** the BIM4LCA concrete office as built and at permit and the timber office as built and at permit (Revit 2024), with Revit's IFC4 exports of the two as built
**Scorers:** `tools/re/qtos_vs_ifc.py` and `tools/re/ifc_element_diff.py`
**Runs:** not yet run (section 5)
**Status:** Not yet measured: the change is written and its rows are registered on #1636 before the run.

## 1. The question

After RE-438 the rule that reads a curtain mullion's kept ends gives Revit's count on all 131 mullions of the concrete office as built that RE-268 left out for an end of their own (130 on the timber office), and 60 of each are written. The other 70 (69) each have a partner end that an oblique mullion or a mullion across inside keeps (a kept count of 2: 50 have two partner ends and 20 have one) and get no value, although their meshes are Revit's solids and the value is the whole surface of the mesh. Why are they refused, and is the refusal right?

## 2. The check

`add_mullion_surface_areas` (`src/ifc/saved_meshes.rs`) draws a mullion's `NetSurfaceArea` from its mesh: the faces along it plus the end faces its solid keeps, `kept` of them, where `kept` is RE-268's count on the boxes of its curtain wall. The boxes do not see a continuation on another curtain wall, so a mullion whose mesh finds a partner (`partner_ends`: the placed ends of two meshes coincide, along one axis, with equal areas) at an end that `kept` counts is refused: `with_partner > 2 - kept`. The check is right for a partner the boxes missed. A partner the boxes saw and kept (RE-438: Revit counts such an end where an oblique mullion or one across inside lies at it) is counted by `kept` for a reason, and the same check refuses the mullion all the same. A mullion with a kept count of 2 and a partner end is refused whenever its mesh finds the partner: the 70 of the concrete office as built are exactly the mullions with a kept count of 2 and a partner end among the 131 released, and none of the 108 mullions RE-438 gave a value has a partner end (the cause is read from the check and from these sets; no build printed the refusal).

## 3. The change

`kept_ends_with_partners` (`src/research/partition_schema_mvp/mullion_ends.rs`) returns, with each mullion's kept count, how many of its kept ends have a partner (`KeptEnds`); `kept_ends` is its first number. The reader keeps both for the mesh path, and the check subtracts the kept ends with a partner from the partners the mesh finds: a mullion is refused when its mesh finds more partner ends than the ends the boxes hide and the partners they kept account for. A partner at a hidden end is still expected, and one the boxes did not see still refuses. A failure-first test of the pure helper: the joint two pieces of one line make is hidden and counts no kept end with a partner, and an oblique brace at the joint keeps it and the end of each piece at the joint counts one.

## 4. The rows (registered on #1636 before the run)

Measured against #1771's head (`base=276716ab`), unfiltered with `holdout=true` and `keep_ifc`, so that the delta is this change's alone ([comment 6807805750](https://github.com/DrunkOnJava/rvt-rs/issues/1636#issuecomment-6807805750)):

1. The refused mullions are written, each equal: `NetSurfaceArea` equal rises by up to 70 on the concrete office as built and up to 69 on the timber office as built, and by the same kind at permit, none different; one whose mesh the path does not read as a prism stays without, counted.
2. Nothing gets worse: no census row gains a `different`, equal never falls, the property census does not move, no mullion that had a value has it changed.
3. What moves: only mullions' `NetSurfaceArea` appears, no element is added or removed, and only the exports that hold a mullion with a kept partner end move.
4. What stays refused: a partner the boxes did not see still refuses; the 399, the 19 and 23 and the 18 are unchanged.

## 5. What Measure found

Not yet run.

## 6. Not done

Pending.

## 7. Reproduce

Pending.
