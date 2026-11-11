# RE-436: a partner of another section hides no end face

**Date:** 2026-11-11
**Issues:** #1745 (the timber office's mullion `NetSurfaceArea` values one end face short with no bent partner), #1046 (W7: the BIM4LCA office's quantities), #1636 (the rules rest on the BIM4LCA models); follows RE-248 (Revit's value hides an end only where it coincides with its continuation's end face), RE-268 (which ends are kept, from the boxes), RE-404 (the same from the meshes) and RE-432 (a bent continuation hides none)
**Artefacts:** the BIM4LCA timber office as built and at permit and the concrete office as built (Revit 2024), with Revit's IFC4 exports
**Scorers:** `tools/re/mullion_ends_vs_boxes.py`
**Runs:** Measure [43076493051](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43076493051) (the timber office exported with `--no-saved-meshes` on `main` at `39826420`, read against Revit's export); the change's own run is in section 4
**Status:** Positive for the cause: the 2 values one end face short on the timber office as built are the only mullions with a hiding partner of another section, and both are wrong; the change's Measure result is in section 4.

## 1. The question

After RE-432 and RE-433 the BIM4LCA timber office still reads 2 `Qto_MemberBaseQuantities.NetSurfaceArea` values different as built (6 at permit), each one end face short: mullions 1912157 (rvt-rs 0.0200 m², Revit 0.0225) and 1912158 (0.0400, Revit 0.0475). Neither has a partner bent off its axis. What do they have?

## 2. What the two have

Read from the timber office exported as boxes (run 43076493051) against Revit's IFC, they are two of a row of four pieces 0.1 m long meeting end to end along one axis, every angle 0.0 degrees: 1912156 (50 x 50 mm), 1912157 (50 x 50), 1912158 (150 x 50) and 1912159 (150 x 50). Each of the two has a partner (RE-268: a mullion running the same way with an end within 3 mm and its body beyond) at both ends, so both ends read hidden, and rvt-rs writes the faces along the mullion alone. Revit's value counts one whole end face on each: 1912157's is 0.0225 = 0.02 + 0.0025 (a 50 x 50 mm face), 1912158's 0.0475 = 0.04 + 0.0075 (a 150 x 50 mm face). The end where the two meet counts whole, on the 50 x 50 mm piece too, whose face the larger piece covers entirely, and the end where each meets a piece of its own section is hidden. Revit's value hides an end only where the end coincides with its continuation's end face (RE-248), and a 50 x 50 mm face does not coincide with a 150 x 50 mm one.

`tools/re/mullion_ends_vs_boxes.py`'s boxes say the same over the whole office. Of the plain mullions with an end hidden by a partner and a Revit value that is a whole number of end faces:

| office as built | partner has the mullion's section | partner has another section |
|---|---:|---:|
| timber | 5,660 | **2** |
| concrete | 7,793 | 0 |

The 2 are 1912157 and 1912158 and are wrong; no mullion with a hiding partner of another section is right.

## 3. The change

A partner hides an end only where its section is the mullion's: in RE-268's `kept_ends` (`src/research/partition_schema_mvp/mullion_ends.rs`, the boxes), the partner's extent along each of the mullion's two sides across it equals the mullion's half extent there to `TOLERANCE_FEET` (3 mm); in RE-404's `partner_ends` (`src/ifc/mesh_volume.rs`, the meshes, which carry each end face's area since the placement gained `areas`), the areas of the two end faces that meet are equal to 0.1%. Both pure helpers have failure-first tests: a continuation of another section hides no end, within 3 mm (and 0.1%) it does.

## 4. What Measure found

Not yet read: the unfiltered Measure of the change against `main` is running. The rows it is held to are registered on #1636 (https://github.com/DrunkOnJava/rvt-rs/issues/1636#issuecomment-6803976440).

## 5. Reproduce

```bash
# the timber office exported as boxes, kept
gh workflow run measure.yml --ref main -f ref=39826420aec6d59c696b9320d9471b7d03ed620e -f base=none -f nordic=true -f models="nordic-office-timber-as-built nordic-office-timber-permit" -f scorers=none -f keep_ifc=true -f ifc_flags=--no-saved-meshes
# then, with the kept model.ifc and Revit's export of the timber office as built
python3 tools/re/mullion_ends_vs_boxes.py model.ifc ARK_NordicLCA_Office_Timber_As-Built_Revit.ifc --list
```
