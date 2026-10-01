# RE-136: the record envelope before 2024, the ElemTable's two record lengths, and ids that repeat across partitions

**Date:** 2026-10-01
**Issues:** #152, #153, #421, #548; Discussion #112
**Artefacts:** the eleven Autodesk family files of `phi-ag/rvt` (2016 to 2026), the four RE1 models (Revit 2025), `Revit_IFC5_Einhoven.rvt` (Revit 2023) and `2024_Core_Interior.rvt` (Revit 2024). Measured on GitHub's hosted runners with the Measure workflow, runs 36868637766, 36869273382 and 36869717207.
**Probe:** `examples/probe_re135_undeclared_records.rs` (the RE-135 scan, extended).
**Status:** positive. Three of Steffen's measurements are confirmed on releases we had not measured, and the ids RE-135 found repeated in the chain are explained.
**Credit:** Steffen (STE1200, SLIK Architekten): the pre-2024 envelope `[u32 id][u32 size][u16 tag][u16 entries]`, the two ElemTable layouts, and the chaining rate; jakobhirn-bit: the 2024-and-later envelope.

## 1. The 2023 envelope reads every family from 2016 to 2023

RE-135 read Revit 2023 by `[u32 id][u32 size][u16 ElementHeader tag][u16 entries]` followed by `size` bytes. The probe now uses that envelope for every release up to 2023. On the Autodesk family of each release:

| release | `ElementHeader` tag | records found | ids `Global/ElemTable` declares | records after the chain | classes the schema lacks | ElemTable record length |
|---|---:|---:|---:|---:|---:|---:|
| 2016 | 1201 | 1,595 | 1,595 | 0 | 0 | 28 |
| 2017 | 1218 | 1,630 | 1,630 | 0 | 0 | 28 |
| 2018 | 1259 | 1,692 | 1,692 | 0 | 0 | 28 |
| 2019 | 1261 | 1,712 | 1,712 | 0 | 0 | 28 |
| 2020 | 1272 | 1,749 | 1,749 | 0 | 0 | 28 |
| 2021 | 1299 | 1,813 | 1,813 | 0 | 0 | 28 |
| 2022 | 1340 | 1,878 | 1,878 | 0 | 0 | 28 |
| 2023 | 1398 | 1,963 | 1,963 | 0 | 0 | 28 |
| 2024 | 1439 | 1,975 | 1,974 | 0 | 0 | 40 |
| 2025 | 1479 | 1,984 | 1,983 | 0 | 0 | 40 |
| 2026 | 1509 | 1,992 | 1,991 | 0 | 0 | 40 |

- **One record per declared id, none left over.** On 2016 to 2023 the number of records the envelope finds equals the number of ids the ElemTable declares, to the record, on all eight releases. Every record's class resolves through the file's own schema (0 unresolved on all 11 families and every project, about 265,000 records).
- **Steffen's measurement of the pre-2024 envelope (2014 to 2018 files) holds on 2016 to 2023**, seven releases we had not measured, in standalone families. No project file before 2023 is in the corpus, so projects of 2016 to 2022 are not measured (#421).
- **The tags are the ones RE-133 gives** for every release from 2016.

## 2. The ElemTable record length is 28 bytes up to 2023 and 40 from 2024

`ElemRecord` lengths in each file's `Global/ElemTable`: **28** on all eight families of 2016 to 2023 and on Einhoven (2023); **40** on the three families of 2024 to 2026, on Core Interior (2024) and on the four RE1 models (2025). There is no other length and no file with two. This is the version gate #152 asked for from corpus evidence: the transition is between 2023 and 2024, the release where the record header also changes (jakobhirn-bit, Discussion #112).

## 3. The chaining rate

Steffen reported that scanning for the header tag and validating by the length field chains 99.5 to 99.8 per cent of records on 2015, 2017 and 2026 files (#153 asked for the numerator and denominator on our files). Here a candidate is a position outside an accepted record that carries the header tag, and it is accepted when the record's size, and its trailer on 2024 and later or the next record's tag on 2023 and before, check out:

| file | release | candidates | accepted | rate |
|---|---|---:|---:|---:|
| Core Interior | 2024 | 50,951 | 50,674 | 99.46% |
| Einhoven | 2023 | 5,277 | 5,257 | 99.62% |
| the 11 families | 2016 to 2026 | 1,597 to 1,993 each | all but 1 or 2 each | 99.87% to 99.95% |
| RE1 Electrical, Mechanical, Plumbing | 2025 | 108,952 / 20,383 / 21,264 | 108,907 / 20,374 / 21,255 | 99.96% |
| RE1 Architecture | 2025 | 40,149 | 38,442 | 95.75% |

The rate agrees with Steffen's on Core Interior (2024) and Einhoven (2023). RE1 Architecture is the exception: 1,707 positions carry the header tag and are not records. What they are is not examined here.

## 4. Ids that repeat across partitions (#548)

RE-135 found ids that open more than one record in a project's leading chain: 1,444 on Core Interior, 109 on RE1 Electrical, 32, 31, 13 and 4 on the others. For each of them:

| file | ids | records in two or more streams, none in one | same class | same size | byte-identical | differ |
|---|---:|---:|---:|---:|---:|---:|
| Core Interior | 1,444 | 1,444 | 1,444 | 1,343 | 873 | 571 |
| RE1 Electrical | 109 | 109 | 109 | 99 | 80 | 29 |
| RE1 Mechanical | 32 | 32 | 32 | 31 | 22 | 10 |
| RE1 Plumbing | 31 | 31 | 31 | 28 | 17 | 14 |
| Einhoven | 13 | 13 | 13 | 11 | 8 | 5 |
| RE1 Architecture | 4 | 4 | 4 | 1 | 1 | 3 |

- **An id with two records always has them in different partitions, never in one** (none adjacent, none in one stream), always with the same class. They are copies of one element's record held in several partitions.
- **Most copies are the same bytes** (873 of 1,444 on Core Interior), and the rest differ: by class, `SWall` 236, `FamilyInstance` 184, `RoomElem` 111, `Floor` 12, `LevelRoomPlan` 9, `VarSketch` 9. On RE1 Architecture the differing ones are `RvtLinkSymbol`; on RE1 Mechanical and Plumbing they are `RoomElem` and `RvtLinkSymbol`.
- **No export value disagrees with Revit's because of it.** Core Interior's export has Revit's GlobalId on all 854 elements and all 360 walls within 0.001 ft of Revit's faces and ends (the Measure scorers, run 36869273382).

Which copy is the current one, and what differs between two copies of a wall, is not established; it matters if a differing copy ever carries a different box or type from the record rvt-rs reads.

## 5. Not established

- Projects of 2016 to 2022: no file in the corpus, and no Revit export to score against. Jakob's offer of an IFC4 export of a 2021 or 2022 sample project (Discussion #112) is the way to measure them.
- A 2014 file, for the 3,619-class parse of #154: none held; the 2016 family reads 3,792 classes.
- The first record of the ElemTable, at `0x06` (#152), is still skipped by the frame the parser reads.
