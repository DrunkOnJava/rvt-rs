# RE-147: the ElemTable's invariants on a 2014 family and every file from 2016 to 2027

**Date:** 2026-10-03
**Issues:** #152 (the 2014 and 2026 checklist items). STE1200 pointed to the 2014 family (#152, issuecomment-5967778938), and puzzbobb described the frame (#152).
**Artefacts:** 54 files, read on GitHub's hosted runners with the Measure workflow, run 37117029420:
- the Stertil 2014 family, newly pinned in `research/public-families.tsv` (zip sha256 `6e65aae1…`, `.rfa` sha256 `b3b46ded…`, 4,923,392 bytes) and fetched by `tools/fetch-public-families.sh`;
- the eleven Autodesk families of 2016 to 2026;
- Autodesk's 36 sample projects of 2016 to 2027;
- Core Interior, Einhoven and the four RE1 models.

**Probe:** `examples/probe_re147_elem_table_invariants.rs`, on the reader of #559, which reads the table from `0x06`.
**Status:** positive on every file.

## 1. What was checked

`Global/ElemTable` is a `u16` tag, a `u32` record count at `0x02`, and that many records from `0x06`. Each record closes with its owner: `[u32 id][u32 id][16 B][u32 owner]` (28 bytes) or `[u64 id][12 B][u64 id][u64 owner][u32]` (40 bytes). That is STE1200's frame, which `elem_table::parse_records` reads since #559. For each file the probe prints:
- the stated count, the records read, the record size, and the bytes after the last record;
- whether the ids rise strictly;
- the first record's id and owner;
- how many records name an owner, how many of those owners are ids of the table, self-owned records, and records in an owner loop.

## 2. Result

| files | releases | record | records read = stated | tail (bytes) | ids rise | owners declared | self-owned | in loops | first record (id, owner) |
|---|---|---:|---|---:|---|---|---:|---:|---|
| Stertil family | 2014 | 28 B | 11,331 = 11,331 | 19 | yes | 10,208 of 10,208 | 0 | 0 | 1274, unset |
| Autodesk families | 2016 to 2023 | 28 B | all 8 | 451 | yes | all | 0 | 0 | 0, 17 |
| Autodesk families | 2024 to 2026 | 40 B | all 3 | 600 | yes | all | 0 | 0 | 0, 17 |
| Autodesk sample projects | 2016 to 2023 | 28 B | all 24 | 19 | yes | all | 0 | 0 | 1, unset |
| Autodesk sample projects | 2024 to 2027 | 40 B | all 12 | 24 | yes | all | 0 | 0 | 1, unset |
| Einhoven | 2023 | 28 B | 2,615 = 2,615 | 19 | yes | 339 of 339 | 0 | 0 | 0, unset |
| Core Interior | 2024 | 40 B | 26,425 = 26,425 | 24 | yes | 22,368 of 22,368 | 0 | 0 | 0, unset |
| RE1 Architecture | 2025 | 40 B | 4,668 = 4,668 | 24 | yes | 1,384 of 1,384 | 0 | 0 | 1, unset |
| RE1 Electrical, Mechanical, Plumbing | 2025 | 40 B | all 3 | 24 | yes | all | 0 | 0 | 0, unset |

The per-file numbers are in the run's artifact. Two examples: the 2026 `rac_basic` project has 8,401 records with 3,884 owners, all declared; the 2026 family has 1,992 records with 1,977 owners.

- **The 2014 family has the 2016 to 2023 layout.** It has 28-byte records, a 19-byte tail, rising ids, and every owner declared. Its first record is id 1274 with an unset owner (`ff ff ff ff`), not 17. That is what STE1200 reported, and it fits puzzbobb's reading that the first record is the lowest surviving id.
- **The 2026 project satisfies the tree invariants.** Every set owner is an id of the table, no record owns itself, and no chain loops.
- **The record size changes once, between 2023 and 2024,** on families and projects alike, from 2014 to 2027.
- **The first record's id depends on the file's history** (#152, STE1200 and puzzbobb):
  - id 1 on Autodesk's sample projects and on RE1 Architecture;
  - id 0 on Einhoven, Core Interior, the other three RE1 models and the families;
  - id 1274 on the 2014 family.

## 3. What this does not claim

- **The 2014 family is not admitted as a reference model.** It has no Revit export to score against. It is fetched for a run, never stored, and its maker's terms for the download were not checked.
- **Nothing before 2014 was read.** puzzbobb's 2008 to 2012 files are not in the corpora.
