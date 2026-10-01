# RE-135: the records after a project's chain are the loaded families' own documents

**Date:** 2026-10-01
**Issues:** #152, #421; Discussion #112
**Artefacts:** the four RE1 models (Revit 2025), `Revit_IFC5_Einhoven.rvt` (Revit 2023), `2024_Core_Interior.rvt` (Revit 2024), all MIT, and the Autodesk family corpus (`phi-ag/rvt`, 2016 to 2026; 2023 to 2026 are read). Measured on GitHub's hosted runners with the Measure workflow, run 36865908576.
**Probe:** `examples/probe_re135_undeclared_records.rs`.
**Status:** positive, with corrections to what the thread (and RE-35) said. Steffen's reading that the records after a project's chain are the loaded families' own documents holds on every project file measured, and the discriminator is the position in the record chain, not the declaration. His reading of bit `0x10` as the host-stub marker does not hold.
**Credit:** Steffen (STE1200, SLIK Architekten), who explained Einhoven's walls 2921 and 3637 (RE-132) as host-wall stubs of embedded family documents.

## 1. The claim

RE-132 could not say what Einhoven's walls 2921 and 3637 are: their ElementIds are not declared in `Global/ElemTable`, so rvt-rs does not read them. Steffen's reading (Discussion #112): the header records whose ids are undeclared are the inventory of the family documents embedded in the project, with the class mix of a standalone family; these two are the host-wall stubs of wall-hosted families; bit `0x10` of the flag word marks the stubs; and there are as many documents as `Global/ContentDocuments` has section headers.

## 2. Method

The probe finds every header record of every `Partitions/*` stream and splits them by the leading record chain (RE-35). A 2024 or later record is the chain's own: the header constant at `+0x0c`, the size at `+0x08` and a trailer repeating it. A 2023 record is `[u32 id][u32 size][u16 tag][u16 entries]` followed by `size` bytes (Steffen's measurement), counted when the next record follows at once, so the last record of each 2023 run is not counted. Records are found anywhere in the stream, not only until the chain breaks.

## 3. Result

| file | Revit | records in the chain | of them declared | after the chain | of them declared | `0x10` in the chain | `0x10` after | ids opening two chain records | after-chain ids also in the chain | section headers |
|---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
| Core Interior | 2024 | 28,560 | 27,823 | 22,114 | **0** | 0 | 757 | 1,444 | **0** | 46 |
| Einhoven | 2023 | 2,624 | 2,623 | 2,633 | **0** | 0 | 1,359 | 13 | **0** | 6 |
| RE1 Architecture | 2025 | 4,672 | 4,671 | 33,770 | **0** | 1,421 | 12,233 | 4 | **0** | 77 |
| RE1 Electrical | 2025 | 11,999 | 10,388 | 96,908 | **0** | 0 | 51,581 | 109 | **0** | 228 |
| RE1 Mechanical | 2025 | 4,775 | 4,775 | 15,599 | **0** | 0 | 4,231 | 32 | **0** | 40 |
| RE1 Plumbing | 2025 | 4,858 | 4,858 | 16,397 | **0** | 0 | 5,752 | 31 | **0** | 39 |
| Autodesk family 2023 to 2026 (four files) | | 1,963 to 1,992 | all but 1 | 0 | | 1,177 to 1,178 | | 0 | | 0 |

- **No record after the chain has a declared id**, on six projects and 187,421 records. None of their ids occurs in the chain, and none occurs twice after it.
- **A standalone family has no records after its chain**: all 1,963 to 1,992 records of each of the four Autodesk families (2023 to 2026) are in it.
- **The three Core Interior walls Steffen names (23857, 17716, 18429) and Einhoven's two (2921, 3637) all lie after the chain**, with the same neighbours. Core Interior: `DBDrawing`, `Viewport`, a wall type (`WallAttributes` for two, `BasicWallType` for one), the `SWall`, `Viewer`, `DBViewSection`, `DBDrawing`. Einhoven: `DBDrawing`, `Viewport`, `BasicWallType` (2920 and 3636), the `SWall`, `Viewer`, `DBViewSection`, `DBDrawing`. These are a family's template views, its host wall type and its host wall.
- **The class mix is a family's.** After the chain on Core Interior the classes are `CategoryElem` 3,266, `GStyleElem` 3,266, `CurveElem` 2,112, `FontElem` 1,464, `LinearDimString` 1,280, `DBViewType` 932, `SketchPlane` 804, `SketchGrid` 730 and so on; the 2024 Autodesk family holds the same classes, with 70 `CategoryElem`. Per section header the two files have 71 `CategoryElem` each (3,266 over 46, 427 over 6).
- **The section headers of `Global/ContentDocuments` are 46 on Core Interior and 6 on Einhoven, Steffen's counts.** The header is `[ContentMarker tag][FF x4][tag - 1][FF x4]` (jakobhirn-bit's description), and the RE1 models have 77, 228, 40 and 39. This does not match a section to a run of records.
- **His counts are close to ours.** On Einhoven's `Partitions/0` we find 4,376 records, 2,519 of them in the chain (his 4,386 and 2,522); on Core Interior's `Partitions/46` 42,270, against his 42,319. A 2023 chain here loses the last record of each run (section 2), which is part of the difference.

## 4. Two corrections

**Undeclared is not the same as after the chain.** The chain itself holds records with an undeclared id: 737 on Core Interior (266 `RoomElem`, 213 `PostedWarningElem`, 56 `CurveElem`, 45 `FamilyInstance`, 36 `Text3dElem`, then smaller classes) and 1,611 on RE1 Electrical. They are the project's own records, so a reader that takes every undeclared record for an embedded family's takes 3 per cent of Core Interior's undeclared records wrongly. The chain's end is the discriminator, and rvt-rs reads the declared ids in it (RE-35). What those undeclared chain records are is not established here.

**Bit `0x10` is not a host-stub marker.** Einhoven's two walls carry `0x939`, Steffen's pair; Core Interior's three carry `0x929`, the word of its ordinary project walls. The bit is set on none of Core Interior's 28,560 chain records, Einhoven's, or RE1 Electrical, Mechanical and Plumbing's, but on 1,421 of RE1 Architecture's 4,672, and on 1,177 or 1,178 of a standalone family's 1,963 to 1,992. After the chain it is set on 757 of Core Interior's 22,114 records (3 per cent) and 1,359 of Einhoven's 2,633 (52 per cent). It follows the kind of record, not the host stub; which kinds is not measured here.

**The ids are not a separate space per document.** No id after the chain occurs twice, and none occurs in the chain, on any file: the embedded documents' records have ids that are unique in the file. RE-35's "their own ElementId space" is true only in that they are not declared. Steffen's three ids on Einhoven and four on Core Interior that occur in two header records are not reproduced: after the chain none repeats here. Within the chain, ids do repeat (section 5).

## 5. Not established

- **1,444 ElementIds on Core Interior (1,340 of them declared), and 13, 4, 109, 32 and 31 on the others, open two or more records in the chain.** RE-35 counted the declared ids that have a record in a chain, not that each has one. The class of those records is not measured, and neither is whether one of the two is what the ElemTable's second id (RE-41) points at. Filed as #548, and explained in RE-136: they are copies of one element's record in different partitions, never two in one stream.
- Which family document a record belongs to. The section headers give a count only.
- 2023 records with declared parameter entries: none were found on Einhoven, so the 22-byte shift is unmeasured there.
- Whether Revit's export holds the undeclared walls. Einhoven has no Revit export in the corpus. Core Interior has one, and its three could be looked up in it; that was not done.
