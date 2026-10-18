# RE-176 — A stair's Base and Top Level from the Levels its record lists

**Date:** 2026-10-13
**Follows:** RE-174, RE-175.
**Artefacts:** Autodesk's public Revit 2024 samples and the BIM4LCA Nordic models (CC BY-SA 4.0), each with Revit's own IFC export as the oracle.
**Clean-room provenance:** public sample files and Revit's own IFC export only. No SDK source and no decompiled or disassembled code.
**Result:**
- RE-174 reads a stair's Base and Top Level from its association record, and otherwise writes a Level only when the element record's chosen id list names exactly one recovered Level. A stair whose association record gives no pair lists its Base Level, its Top Level and, for some stairs, the Level above the top. **The Base Level is the lowest listed Level and the Top Level the next one above it**, by recovered elevation. Written only for class `Stair`, only when every listed Level has an elevation and the lowest three are at distinct elevations.
- **Additive.** Every other property is unchanged (survey below, all files).

Per stair, against Revit's `Constraints` (all `IfcStair` that rvt-rs decodes; Snowdon measured with open PR #737 cherry-picked on a throwaway probe):

| rule | right | wrong | skipped |
|---|---:|---:|---:|
| Base = lowest listed | 56 | 0 | 1 |
| Top = second-lowest listed | 54 | 0 | 3 |
| Top = highest listed (asked) | 41 | **13** | 3 |
| control: Base = highest listed | 0 | 54 | 3 |
| control: Base = random listed Level | 26 | 30 | 1 |
| control: Top = lowest listed | 0 | 54 | 3 |

Where the association record already gives Base and Top (89 stairs, 8 of them listing three Levels), lowest = its Base 89/89 and second-lowest = its Top 89/89; highest = its Top only 81/89. The skips are a Snowdon stair with no record lists and two Snowdon stairs listing one Level (Revit: Base = Top), which RE-174 already writes. BIM4LCA (no `Constraints` set; reference = Revit's storey): lowest listed = storey for 72 of 72 stairs.

`level_parameters_survey_are_revits` before / after (written / equal to Revit / Revit has it):

| file | Base Level | Top Level |
|---|---|---|
| Technicalschool | 196 / 190 / 195 -> 201 / 195 / 195 | 0 -> 5 / 5 |
| rmeadvanced | 141 / 141 / 149 -> 149 / 149 / 149 | 0 -> 8 / 8 |
| rmebasic | 141 / 141 / 149 -> 149 / 149 / 149 | 0 -> 8 / 8 |
| Nordic Office Concrete/Timber, As-Built/BuildingPermit | +1 each, agreeing with Revit's storey | 12 -> 13 each (no reference) |

Zero wrong values in any property on any file. The test now requires the stair Top Level counts on the five stair-bearing Autodesk samples; it fails on RE-175 (`Technicalschoolcurrentm: 0 Top Level written, expected at least 5`).

**Limits.** Revit 2024 only; elevations are the recovered Levels'. DACH exports no elements here, even with #737; its 12 stairs are all decoded by the association record (spike 255), and on the 4 whose listed Levels are both IFC storeys the lowest is the Base Level 4/4. Snowdon exports nothing on this base (#736). The Top Level on BIM4LCA stairs has no reference in their exports. `IfcStair`s that rvt-rs does not decode as class `Stair` are not covered: Housing_Concrete 31 of 38 per file, Office 2 of 16 per file; Housing_Timber exports no elements at all here.
