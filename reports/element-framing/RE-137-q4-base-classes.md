# RE-137: the Q4 addendum's "ancestor class" does not reproduce

**Date:** 2026-10-01
**Issues:** #154; Discussion #112
**Artefacts:** the eleven Autodesk family files of `phi-ag/rvt` (2016 to 2026), the four RE1 models (Revit 2025), `Revit_IFC5_Einhoven.rvt` (Revit 2023) and `2024_Core_Interior.rvt` (Revit 2024). Measured on GitHub's hosted runners with the Measure workflow, run 36869947677.
**Probe:** `examples/probe_re137_q4_base_classes.rs`.
**Status:** the Q4 addendum's reading is not reproduced on any of the 17 files. What the legacy word is remains open.
**Credit:** Steffen (STE1200, SLIK Architekten), whose grammar for `Formats/Latest` (Discussion #112) made the comparison possible.

## 1. The claim under test

The reconnaissance document's Q4 addendum (2026-04-19) called the `u16` between a class's parent-class name and its field count an "ancestor class reference", distinct from the direct parent, and listed nine classes whose reference resolved to another class of the same schema: for example `HostObjAttr`, whose parent is `Symbol` and whose "ancestor" is `APIVSTAMacroElem`. It was read from a schema that was inflated without its checksum-page strip (#410), with tags assigned by the old parser.

## 2. Method

`formats::schema_classes` reads `Formats/Latest` by the grammar of Steffen's report: a class is a `u16`-counted name, a `u16` base reference (bit 15 set: the base class is defined inline right after a zero `u16`), a `u32` version, a `u32` field count and the fields. The tag is the definition ordinal. The probe prints, for each of the addendum's nine classes, the class the grammar gives as its base and whether that is the class the addendum named, and for the whole file how many classes carry a base reference and how many of those resolve.

## 3. Result

| file | release | classes | with a base reference | resolve | of the nine present | base is the addendum's class | base is the next class defined |
|---|---|---:|---:|---:|---:|---:|---:|
| Core Interior | 2024 | 4,492 | 3,023 | 3,023 | 9 | **0** | 9 |
| Einhoven | 2023 | 4,418 | 2,976 | 2,976 | 9 | **0** | 9 |
| RE1 (all four) | 2025 | 4,600 | 3,070 | 3,070 | 9 | **0** | 9 |
| Autodesk family 2026 | 2026 | 4,690 | 3,127 | 3,127 | 9 | **0** | 9 |
| Autodesk families 2016 to 2022 | 2016 to 2022 | 3,792 to 4,307 | 2,502 to 2,885 | all | 7 to 8 | **0** | 5 to 7 |

(The 2023, 2024 and 2025 families equal the project files of their releases; each of the 17 files is in the run's artifact.)

- **None of the nine classes has the base the addendum named, on any file.** `HostObjAttr` has base `Symbol`; `APIVSTAMacroElemTracking` has `ElemSetTracking`; `AnalyticalLineAutoConnectData` has `AutoConnectData`; `AnalyticalPanelPatternHelper` has `PatternHelper`; `ReferencePointGridNetTrackerCell` has `GridNetTrackerCell`; `AnalyticalSlabAdjustmentGStep` has `AnalyticalSurfaceGStep`; `AppearanceAssetElemGroupHelper` has `GroupHelper`; `ArcWallRectOpeningGStep` has `VWallRectOpeningGStep`; `AreaMeasureCurveData` has `UserElemCurveData`.
- **Where the addendum said "parent", it was right about the base.** `HostObjAttr`'s base is `Symbol`, the class the addendum called its parent. The "ancestor" has no counterpart.
- **Every base reference resolves.** Of all the base references in a file, 100 per cent name a class of the same schema, on all 17 files (3,023 of 3,023 on a 2024 file). A base reference is a base class; nothing here supports a second, distinct ancestor.
- **In the nine, the base is usually the class defined right after**, the inline definition the grammar allows (its tag is the class's tag plus one): 9 of 9 on 2023 and later, 5 to 7 of the 7 to 8 present on 2016 to 2022.

## 4. What this does and does not say

The addendum's resolutions came from the old parser's numbering of classes over a schema inflated without its page strip; why its small `u16` values landed on those classes is not examined here. What the legacy "flag" word (`ClassEntry::ancestor_tag`) actually is, under the corrected grammar, is not established here; the field stays in the legacy parser's output, with its documentation corrected. The README, the reconnaissance document and `src/formats.rs` now say the reading did not reproduce.

Not measured: 2014, a release the grammar was first measured on (STE1200); no 2014 file is in the corpus.
