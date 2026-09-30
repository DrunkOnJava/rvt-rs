# RE-133: the class numbers of jakobhirn-bit's table, on eleven releases

**Date:** 2026-09-30
**Issues:** #154, #96; Discussion #112
**Artefacts:** the eleven Autodesk family files of `phi-ag/rvt` at revision `df58bff1eec50c995beb5a99fc3a4b2ae6026be4` (`rac_basic_sample_family-2016` to `racbasicsamplefamily-2026`; Autodesk-owned, fetched for the run and never stored), and the six MIT reference models: `Revit_IFC5_Einhoven.rvt` (2023, `d3a0c6d3…a948`), `2024_Core_Interior.rvt` (2024, `c805df44…8014`) and `RE1-Architecture`, `RE1-Electrical`, `RE1-Mechanical`, `RE1-Plumbing` (2025, `4a78bdd6…1c8ce4`, `9875e765…c9630b`, `d5597201…9c014b`, `03689df2…f9e54c`). Measured on GitHub's hosted runners with the Measure workflow and its new `families` option, run 36731528117.
**Probe:** `examples/probe_re133_class_ordinals.rs`.
**Status:** positive. All 20 class numbers of his table (ten classes, Revit 2024 and 2026) are the tags rvt-rs reads; the same ten are measured on nine more releases below.
**Credit:** jakobhirn-bit, who derived the table and the rule for it (Discussion #112).

## 1. The table and the rule

jakobhirn-bit gave the class numbers his reader uses for ten classes in Revit 2026 (`rac_basic_sample_family-2026`) and Revit 2024 (Snowdon Towers), and the rule that yields them: count the class definitions of `Formats/Latest` in stream order starting at 12, an inline parent or field class taking the next number. That is the rule `formats::schema_classes` implements (#154, from STE1200's grammar, measured against 2014 to 2026 schemas), where a class's `tag` is its definition ordinal, and it is how an element record names its class (RE-76). The probe prints each of the ten classes' tag in the file it runs on.

## 2. Result

The tag of each class in the eleven Autodesk family files, one per release:

| class | 2016 | 2017 | 2018 | 2019 | 2020 | 2021 | 2022 | 2023 | 2024 | 2025 | 2026 |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| `ElementHeader` | 1201 | 1218 | 1259 | 1261 | 1272 | 1299 | 1340 | 1398 | 1439 | 1479 | 1509 |
| `Family` | 1539 | 1577 | 1622 | 1625 | 1649 | 1677 | 1772 | 1834 | 1872 | 1929 | 1967 |
| `FamilyInstance` | 1561 | 1599 | 1644 | 1647 | 1671 | 1699 | 1794 | 1856 | 1894 | 1951 | 1989 |
| `FamilySymbol` | 1593 | 1631 | 1679 | 1682 | 1706 | 1734 | 1829 | 1890 | 1928 | 1984 | 2022 |
| `ContentMarker` | 744 | 758 | 771 | 772 | 779 | 801 | 817 | 869 | 891 | 913 | 931 |
| `ElementParents` | 1210 | 1227 | 1268 | 1273 | 1284 | 1311 | 1352 | 1410 | 1451 | 1491 | 1521 |
| `FamilyInstancePatternHelper` | 1567 | 1605 | 1650 | 1653 | 1677 | 1705 | 1800 | 1861 | 1899 | 1956 | 1994 |
| `InstanceInfo` | 1975 | 2016 | 2070 | 2075 | 2110 | 2139 | 2238 | 2307 | 2353 | 2415 | 2463 |
| `RbsPipeCurve` | 2848 | 2915 | 2990 | 3044 | 3097 | 3099 | 3210 | 3305 | 3349 | 3432 | 3503 |
| `RbsCurveConnectorManager` | 2775 | 2842 | 2918 | 2972 | 3025 | 3027 | 3138 | 3233 | 3277 | 3361 | 3432 |
| classes in the schema | 3792 | 3879 | 4024 | 4098 | 4164 | 4168 | 4307 | 4418 | 4492 | 4600 | 4690 |

Each of the ten is defined once in every file, and the first class is tag 12 in every file measured.

## 3. Agreement with his table

His 2026 column equals the 2026 column above on all ten classes, and his 2024 column equals the 2024 column on all ten: 20 of 20. His 2026 column is labelled `rac 2026`, the Autodesk family of that release. His 2024 values were read on Snowdon Towers, which is not in the run; the run's 2024 values come from `2024_Core_Interior.rvt` and the 2024 family file, which agree with each other and with his.

STE1200 gave `ElementHeader`'s tag for six of these releases in the same discussion, read off each file's own schema: `0x4C2` for 2017, `0x4EB` for 2018, `0x53C` for 2022, `0x576` for Einhoven (2023), `0x59F` for Core Interior (2024) and `0x5E5` for 2026. They are 1218, 1259, 1340, 1398, 1439 and 1509, the first row of the table.

## 4. Within a release the file does not matter

Each MIT project has the same ten tags and the same class count as the family file of its release: Einhoven and the 2023 family (4,418 classes), Core Interior and the 2024 family (4,492), and all four RE1 models and the 2025 family (4,600). The tags are a property of the release build, not of the file.

## 5. Not measured

- His 2024 column on Snowdon Towers, a local-only model.
- Anything beyond these ten classes. Two of them, `RbsPipeCurve` and `RbsCurveConnectorManager`, are the classes RE-131 uses; the others are the ones his reader keys on.
