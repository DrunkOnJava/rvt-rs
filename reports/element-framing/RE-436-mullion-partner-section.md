# RE-436: a partner of another section hides no end face

**Date:** 2026-11-11
**Issues:** #1745 (the timber office's mullion `NetSurfaceArea` values one end face short with no bent partner), #1046 (W7: the BIM4LCA office's quantities), #1636 (the rules rest on the BIM4LCA models), #1767 (the 4 values still different at permit); follows RE-248 (Revit's value hides an end only where it coincides with its continuation's end face), RE-268 (which ends are kept, from the boxes), RE-404 (the same from the meshes) and RE-432 (a bent continuation hides none)
**Artefacts:** the BIM4LCA timber office as built and at permit and the concrete office as built (Revit 2024), with Revit's IFC4 exports
**Scorers:** `tools/re/mullion_ends_vs_boxes.py`
**Runs:** Measure [43076493051](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43076493051) (the timber office exported with `--no-saved-meshes` on `main` at `39826420`, read against Revit's export); and the change [43078146763](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43078146763) (unfiltered, `keep_ifc`, head `8283ee3c` against `main` at `39826420`)
**Status:** Positive: the 2 values one end face short on the timber office as built are the only mullions with a hiding partner of another section, and both were wrong; the change makes them Revit's, as built and at permit (the same two mullions), none different, and nothing else moves. The 4 values still different at permit have another cause (section 5).

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

A partner hides an end only where its section is the mullion's: in RE-268's `kept_ends` (`src/research/partition_schema_mvp/mullion_ends.rs`, the boxes), the partner's extent along each of the mullion's two sides across it equals the mullion's half extent there to `TOLERANCE_FEET` (3 mm); in RE-404's `partner_ends` (`src/ifc/mesh_volume.rs`, the meshes, which carry each end face's area since the placement gained `areas`), the areas of the two end faces that meet are equal to 0.1%. Both pure helpers have failure-first tests: a continuation of another section hides no end, within 3 mm (and 0.1%) it does. The mesh test compares areas, not outlines, so two sections of equal area and different shape would be read as one; the two tests agree on every pair of the table above (the only 2 pairs of another section by the boxes' extents differ in area too, 0.0025 against 0.0075 m²), and no pair of the offices has equal areas and different extents.

## 4. What Measure found

Run 43078146763 (126 exports each side; the base is `main` at `39826420`, which holds RE-433, so only this change moves). The quantity census of the 42 models that have a Revit export: Revit's quantities 173,840 each side; equal 155,151 to 155,155 (+4); missing 18,656 to 18,656; **different 33 to 29**; the values rvt-rs writes that Revit does not 29 to 29. The property census does not move (213,756 values each side; equal 201,845, different 61, missing 11,850). By file:

| file | quantities | equal | different | missing |
|---|---:|---:|---:|---:|
| BIM4LCA timber office, as built | 33,800 | 30,837 to 30,839 | 3 to 1 | 2,960 to 2,960 |
| BIM4LCA timber office, at permit | 33,663 | 30,429 to 30,431 | 23 to 21 | 3,211 to 3,211 |

The concrete office as built and at permit and the 40 other models with a Revit export read the same on both sides. All of the change is `Qto_MemberBaseQuantities.NetSurfaceArea` (as built 7,287 to 7,289 equal of 7,876, different 2 to 0; at permit 7,168 to 7,170 of 7,872, different 6 to 4): mullions 1912157 and 1912158 in both files, 0.0200 and 0.0400 m² before and 0.0225 and 0.0475 now, which are Revit's. **2 of 126 exports move** (the two timber office files) and 124 are byte-identical, the concrete offices, the held-out, Revit 2025 and Autodesk models among them. `tools/re/ifc_element_diff.py` on the 2 pairs: no element is only in one file and 4 elements differ (the two mullions in each file), each a `Qto_MemberBaseQuantities.NetSurfaceArea` changed in the set it holds, nothing added or removed; no entity count moves. `same-project gates` pass on the measured head.

Held against the rows registered on #1636 (https://github.com/DrunkOnJava/rvt-rs/issues/1636#issuecomment-6803976440):

| row | result |
|---|---|
| 1 the 2 become equal | **Held**: timber office as built `NetSurfaceArea` different 2 to 0, equal +2; at permit different 6 to 4, equal +2, the same two mullions, 1912157 and 1912158, each Revit's now |
| 2 nothing gets worse | **Held**: no census row gains a `different` and no equal falls on any of the 42 models; the property census does not move on any |
| 3 what moves | **Held**: 4 `NetSurfaceArea` values change (the two mullions in each of the two files), no element is added or removed, 124 of 126 exports are byte-identical |
| 4 what stays out | **Held**: the 587 mullions of the timber office as built and 698 at permit that have no value stay without, and the 4 other values different at permit read as before (the same four numbers on both sides) |

## 5. Not done

The 4 values still different at permit are not this cause and are not read here: mullions 3539474, 3539475, 3135318 and 3029161, none of which this change moves. The first three are pieces of a chain of 50 x 50 mm mullions on one curtain wall that meet end to end along one axis (3539474 is 0.075 m long, the other two 0.1 m): Revit's value hides every joint (0.0175, 0.0200 and 0.0200 m², each the faces along the mullion and, on 3539474, the one free end) and rvt-rs counts 1, 2 and 1 joint ends more (0.0200, 0.0250 and 0.0225); why RE-268's and RE-404's partner reading does not hide them is not measured. The fourth, 3029161, is a short piece with mitred ends whose value is 0.0025 m² under Revit's (0.0102 against 0.0127), which is the whole surface of its saved mesh. [#1767](https://github.com/DrunkOnJava/rvt-rs/issues/1767) holds them.

## 6. Reproduce

```bash
# the change against main as it was (RE-433's merge), kept under refs/archive/2026-11-11/cited/8283ee3c15c9
gh workflow run measure.yml --ref main -f ref=8283ee3c15c93ac29360bc6e3ebb7c83fa92d4ec -f base=39826420aec6d59c696b9320d9471b7d03ed620e -f samples=true -f nordic=true -f r2025=true -f holdout=true -f bimcollab=true -f racadv=true -f keep_ifc=true
# the timber office exported as boxes, kept
gh workflow run measure.yml --ref main -f ref=39826420aec6d59c696b9320d9471b7d03ed620e -f base=none -f nordic=true -f models="nordic-office-timber-as-built nordic-office-timber-permit" -f scorers=none -f keep_ifc=true -f ifc_flags=--no-saved-meshes
# then, with the kept model.ifc and Revit's export of the timber office as built
python3 tools/re/mullion_ends_vs_boxes.py model.ifc ARK_NordicLCA_Office_Timber_As-Built_Revit.ifc --list
```
