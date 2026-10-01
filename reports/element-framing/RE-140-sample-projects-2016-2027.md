# RE-140: Autodesk's sample projects of 2016 to 2027, and puzzbobb's measurements

**Date:** 2026-10-01
**Issues:** #421, #152, #548
**Artefacts:** Autodesk's own `rac_basic`, `rst_basic` and `rac_advanced` sample projects for every release from 2016 to 2027 (36 files, pinned in `research/autodesk-sample-projects.tsv`, on the host the family corpus comes from, fetched for a run and never stored) and the 11 Autodesk families of the phi-ag/rvt corpus. No Revit export exists for any of them. Measured on GitHub's hosted runners with the Measure workflow (`-f samples=true -f families=true`), runs 36899534445, 36900586913, 36901574754, 36903389204 and 36904178406.
**Probe and tools:** `examples/probe_re135_undeclared_records.rs` (the RE-135 scan, extended), `tools/fetch-autodesk-samples.sh`.
**Status:** positive, and no release is admitted by it. Every measurement puzzbobb posted on #421 that these files can test reproduces; the same model read in 2016 to 2023 gives the same classes as 2024 does and, on the records that carry an element marker, the same categories (99.9 to 100 per cent) and nearly the same boxes (89.6 to 100 per cent).
**Credit:** puzzbobb (#421), whose 10:24 AM UTC comment of 2026-10-01 set out what is measured here; STE1200, whose 2023 envelope RE-81 and RE-136 read.

RE-136 said the corpus held no project file of 2016 to 2022 and that none of them could be scored. Autodesk publishes its sample projects for every release on the host the family corpus already comes from, so there are 36 of them, among them the 2017 `rac_basic` project STE1200 measured (sha256 `a1d3d077…5fefb`, the hash in `docs/reference-models.md`).

## 1. puzzbobb's measurements against the files

| puzzbobb | measured here |
|---|---|
| The 32-bit chain `[u32 id][u32 size][u16 header tag][u16 count] … [u32][u32 flag][u32 size]` covers every id `Global/ElemTable` declares, 2014 to 2023 | A strict walk (the header tag, and a trailer whose last word repeats the size, the flag word not checked) holds a record for every id read from `0x06` on all 47 files measured (36 projects, 11 families), 2016 to 2027; none is left over on 46 of them. 2014 and 2015 are not in the corpus. |
| The ids per file: 7,965 (2019 `rac_basic`), 13,810 (`rst_basic`), 17,197 (`rac_advanced`), 8,008 and 13,853 (2020), 8,081 and 13,924 (2021), 8,155 and 14,034 (2022), 8,276 and 13,821 (2023), 1,596 (the 2016 family) | Equal on all of them (section 2). |
| `ElementHeader` tags are RE-133's for 2016 to 2023 | Equal, and 2027's is 1540, as puzzbobb gives. |
| The flag word is `0` or `0x01000000` on all but one record per structural model (`0x01001269`) | On every record of all 47 files but the 8 `rst_basic` projects of 2016 to 2023, each of which has exactly one `0x01001269`. No file of 2024 to 2027 has one. Every trailer's last word repeats the size (0 mismatches). |
| Every `[Outline][0xFF x 4][ElementParents]` marker of 2016 to 2023 has the 32-bit header 52 bytes before it and a well-formed box after it | True of every marker of every 2016 to 2023 file: 16 on a family (32 on 2023's), 1,238 to 1,269 on `rac_basic`, 5,839 to 6,476 on `rst_basic`, 8,541 to 12,246 on `rac_advanced`. |
| 2027 keeps the 2024 to 2026 chain with the marker from its schema: 8,417, 13,958 | Equal; `rac_advanced` 17,247 as well. |
| `elem_table::parse_records` misses the first record | True on all 47 files; fixed in #553. The claim about two thirds missing on some 2008 to 2012 files is not testable here. |

Not reproduced, because the files are not available here: 2014, 2015 and the 2008 to 2012 files, and the two files whose origin puzzbobb records as unrecorded (`Revit2014.rvt`, `Technical_school`). `Technical_school` has 17,197 ids as `rac_advanced` of 2018 and 2019 does, but puzzbobb counts 8,623 markers on it where those have 8,541, so it is another file; it is not one of Autodesk's published samples. A file of unrecorded origin cannot be added to the corpus.

## 2. The projects

ElementIds the table declares, read from `0x06`, per release and project (every one has a record in a strict walk of the chain):

| release | `ElementHeader` tag | `rac_basic` | `rst_basic` | `rac_advanced` |
|---|---:|---:|---:|---:|
| 2016 | 1201 | 7,904 | 14,360 | 17,099 |
| 2017 | 1218 | 7,945 | 14,396 | 17,132 |
| 2018 | 1259 | 8,010 | 14,461 | 17,197 |
| 2019 | 1261 | 7,965 | 13,810 | 17,197 |
| 2020 | 1272 | 8,008 | 13,853 | 17,240 |
| 2021 | 1299 | 8,081 | 13,924 | 16,946 |
| 2022 | 1340 | 8,155 | 14,034 | 17,019 |
| 2023 | 1398 | 8,276 | 13,821 | 17,108 |
| 2024 | 1439 | 8,288 | 13,819 | 17,120 |
| 2025 | 1479 | 8,327 | 13,861 | 17,157 |
| 2026 | 1509 | 8,401 | 13,936 | 17,231 |
| 2027 | 1540 | 8,417 | 13,958 | 17,247 |

- **The table's first record.** `parse_records` reads its ids from frames that start 24 bytes into each record, so the first record's ids are in none of them: on all 47 files exactly one id the table declares had a record in the chain and was not in the declared set. The 2016 to 2023 tables have 28-byte records and 2024 to 2027's 40-byte ones, as RE-136 found, and the first record's ids are at `+0` and `+4` or `+20` from `0x06`. The probe's own chain rule, which counts a record only when the next one follows, leaves the last record of a run out; the strict walk does not.
- **One exception.** The 2021 `rac_advanced` sample has 6 chain ids that no table record declares (its strict chain holds 16,952 ids, the table 16,946) and 2 declared ids with no record in the probe's chain.

## 3. The same model in every release

Each of the three projects is one model Autodesk re-saves in each release. The probe lists every chain record's ElementId, class, category and box (`--records`), and `R` lines are compared with 2024's by ElementId. Class names are compared on every id both releases hold. Category and box are compared only where both records carry the element marker (22 bytes further for each declared parameter entry: Revit 2027's records place it at 80, 102, 124 … bytes, the 2024 offset plus a multiple of the entry size), which are the records those fields are read for; reading them elsewhere gives other bytes. A shared id agrees when any of its copies matches any of the other's.

Category and box agreement with 2024 over the ids both releases hold with a marker (class names agree on 100.0 per cent of the ids both hold in 2016 to 2023 and on 98.7 to 100.0 in 2025 to 2027, where the model has changed):

| release | `rac_basic` category / box | `rst_basic` category / box | `rac_advanced` category / box |
|---|---|---|---|
| 2016 | 100.0 / 97.3 | 99.9 / 96.0 | 99.9 / 89.6 |
| 2017 | 100.0 / 97.7 | 99.9 / 96.4 | 99.9 / 89.6 |
| 2018 | 100.0 / 97.7 | 99.9 / 96.4 | 99.9 / 89.6 |
| 2019 | 100.0 / 97.7 | 99.9 / 97.6 | 99.9 / 89.6 |
| 2020 | 100.0 / 97.7 | 99.9 / 97.6 | 99.9 / 89.6 |
| 2021 | 100.0 / 97.7 | 99.9 / 97.6 | 99.9 / 99.8 |
| 2022 | 100.0 / 97.7 | 99.9 / 99.1 | 99.9 / 99.8 |
| 2023 | 100.0 / 99.8 | 99.9 / 100.0 | 100.0 / 100.0 |
| 2025 | 100.0 / 100.0 | 100.0 / 98.3 | 100.0 / 100.0 |
| 2026 | 100.0 / 100.0 | 100.0 / 98.3 | 100.0 / 100.0 |
| 2027 | 100.0 / 100.0 | 99.7 / 98.3 | 100.0 / 100.0 |

- The 2023 envelope's class, category and box fields read the same on 2016 to 2022 projects as 2024 reads them on the same model, apart from the box differences. Those are likelier the model's own edits between releases than a reading error, since the percentage is the same release after release (`rac_advanced` is 89.6 per cent in each of 2016 to 2020 and 99.8 in 2021 and 2022; `rst_basic` is 98.3 in 2025 and 2026), but which elements differ was not examined. This is agreement with an admitted reader of the same model, not recovery against Revit's export.
- **2027 is read as 2024 is.** The records carry parameter entries more often (the marker offsets on `rac_basic` are 80 on 331 records and 102 to 366 on the rest, where on 2026 every marked record is at 80), and, with the entries allowed for, 1,265, 5,987 and 9,043 records have the marker on `rac_basic`, `rst_basic` and `rac_advanced`, the numbers of 2024, 2025 and 2026.

What rvt-rs exports today (Measure's `diagnostics.json`, no scoring): on 2023, 465, 454 and 5,657 building elements for the three projects; on 2024 to 2026, 464, 454 and 5,657; on 2016 to 2022 and on 2027 none, since the record readers fail closed for any other release.

## 4. Copies of a record

The ids that open two or more records of the chain (cross-partition copies, #548) occur only in the files with more than one partition: 63 ids in the 2017 `rac_basic` (3 partitions), 37 in its `rst_basic`, 50 and 35 in the 2019 pair (2 partitions each) and 4,037 in the 2021 `rac_advanced` (2 partitions, 20,987 records for 16,946 ids). The number of `ContentDocuments` section headers is constant for a project in every release (163, 52 and 121 for `rac_basic`, `rst_basic` and `rac_advanced`).

## 5. What this does not claim

- **No release is admitted.** There is no Revit export of any of these files, so 2016 to 2022 and 2027 are measured against 2024's reading of the same model, not against Revit. Jakob's offer of an IFC4 export of the 2021 or 2022 `rac_basic` project (Discussion #112, asked for as 2022) is what would settle them; `scan_records` and the other record readers stay closed to those releases.
- **The box agreement is below 100 per cent on 2016 to 2022**, and what differs is not explained element by element.
- The corpus is three models re-saved by Autodesk, not 36 independent files.
- 2014, 2015, files before 2014, and the two unrecorded-origin files puzzbobb lists are not measured.
