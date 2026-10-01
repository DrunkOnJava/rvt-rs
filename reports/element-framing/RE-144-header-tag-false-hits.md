# RE-144: the positions that carry the header tag and are not a record

**Date:** 2026-10-01
**Issues:** #152, #421; Discussion #112 (STE1200)
**Artefacts:** the redistributable reference models Core Interior (Revit 2024), Einhoven (2023) and the four RE1 models (2025). Measured on GitHub's hosted runners with the Measure workflow, run 36905901479.
**Probe:** `examples/probe_re135_undeclared_records.rs`, extended.
**Status:** positive, descriptive. It answers Steffen's question whether RE1 Architecture's non-record positions come in repeated groups: mostly they do not, and Core Interior's do.

Steffen's header reader is a scanner, and on Core Interior the false hits it takes are six positions that repeat seven times in stretches of float64 data (Discussion #112, 2026-10-01). He asked whether the 1,707 positions of RE1 Architecture that carry the header tag and are not accepted by RE-136's scan (1,707 of 40,149 candidates) come in repeated groups too.

## 1. Result

For every position that carries the header tag, lies outside an accepted record and is not accepted as one (its size, its trailer or the record after it fails), the probe takes the header's id and size words and groups them:

| model | candidates | accepted | not accepted | distinct (id, size) pairs | pairs seen 3 or more times | positions in those pairs |
|---|---:|---:|---:|---:|---:|---:|
| RE1 Architecture | 40,149 | 38,442 | 1,707 | 1,431 | 32 | 268 |
| Core Interior | 50,951 | 50,674 | 277 | 86 | 26 | 210 |
| Einhoven | 5,277 | 5,257 | 20 | 18 | 1 | 3 |

- **RE1 Architecture's are mostly single.** 1,431 pairs for 1,707 positions, and 268 positions in repeated pairs. The most frequent size words are 16,777,215 (111 positions), 167,772,159 (29), 1,644,167,167 (25) and 1,023,410,176 (13): 0x00FFFFFF, 0x09FFFFFF and 0x61FFFFFF are size words made of 0xFF bytes, so they are not plausible record sizes.
- **Core Interior's repeat.** 210 of its 277 positions are in 26 repeated pairs; the most frequent size words are 7 (35 positions), 557,060 (28), 87,752,704 (14), 0 (13) and 4,294,967,295 (13).

## 2. What this does not claim

- **Not Steffen's count.** A position is counted here where the tag matches outside an accepted record and its size or trailer fails, over every partition; his 49 for Partitions/46 are the positions that lie in no run of records on his rule. The numbers are not comparable one to one.
- **The groups were not opened.** What is at a repeated position, and whether the seven copies Steffen found on Core Interior are among these 26 pairs, was not examined.
