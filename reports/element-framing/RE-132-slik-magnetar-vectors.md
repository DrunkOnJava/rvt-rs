# RE-132: STE1200's `element_header` vectors on Einhoven and Core Interior

**Date:** 2026-09-30
**Issues:** #152, #421; Discussion #112
**Artefacts:** `Revit_IFC5_Einhoven.rvt` (Revit 2023) and `2024_Core_Interior.rvt` (Revit 2024), both MIT (`magnetar-io/revit-test-datasets`), SHA-256 `d3a0c6d3…a948` and `c805df44…8014`, the values his pack gives and `tests/fixtures/project-counts/` records. Measured on GitHub's hosted runners with the Measure workflow, run 36725925873.
**Probe:** `examples/probe_re132_slik_vectors.rs`, with his eight vectors in it.
**Status:** positive. 6 of 8 vectors agree on every field; the other two are ElementIds `Global/ElemTable` does not declare, which rvt-rs does not read.
**Credit:** Steffen (STE1200, SLIK Architekten), who cut the vectors with his own reader and gave them for use in the repository.

## 1. The vectors

Steffen's pack (`slik_rvt_vectors_magnetar_260928`) gives, for four records of each file, the record's offset in the page-stripped inflated stream, its ElementId, class (by the file's own schema), `BuiltInCategory`, flags word and box (length, thickness and height in millimetres). The probe looks each up in what rvt-rs reads from the file with that hash and prints, per vector, which fields agree. The flags word is `m_abFlags4Bytes`, the `u32` at `+0x46` of a record laid out as 2024 lays it out (RE-128); a 2023 record's start in that layout is 28 bytes before its ElementId.

## 2. Result

| file | id | class | offset | category | flags | box |
|---|---:|---|---|---|---|---|
| Core Interior | 87758 | SWall (3669) | 2458768, delta 0 | -2000011 | `0x1929` | 44805.6 × 304.8 × 9144.0 |
| Core Interior | 87762 | SWall | 2461236, delta 0 | -2000011 | `0x1929` | 44805.6 × 304.8 × 9144.0 |
| Core Interior | 22071 | SWall | 3044354, delta 0 | -2000011 | `0x1929` | 44805.6 × 304.8 × 9144.0 |
| Core Interior | 20953 | Floor (2017) | 8144, delta 0 | -2000032 | `0x0928` | 55575.2 × 37067.6 × 50.8 |
| Einhoven | 2808 | SWall (3625) | 240469, delta 0 | -2000011 | `0x0929` | 6000.0 × 200.0 × 2998.6 |
| Einhoven | 5317 | FamilyInstance (1856) | 258214, delta 0 | -2000014 | `0x0928` | 609.6 × 574.5 × 1219.2 |

**Core Interior 4 of 4 and Einhoven 2 of 4 agree on every field.** rvt-rs frames each record at Steffen's offset exactly, on the 2024 envelope and on the 2023 one (where the offset is the ElementId's). The box sides agree to 0.05 mm, which is inside the 0.1 mm his values are rounded to. His flags are the word RE-128 reads: `0x1929` on Core Interior's walls and `0x0928` on its floor and family instances are the values RE-128 lists.

## 3. The two vectors that are not returned

Einhoven's walls 2921 and 3637 are not in what rvt-rs returns. Both ElementIds are absent from `Global/ElemTable`, and rvt-rs reads a record only where its id is declared there (fail closed, RE-35). The bytes are there: at his offsets the ElementId is 2921 and 3637, the record marker is where a 2023 record has it, and so is the `ElementHeader` tag. What is not is a `BuiltInCategory`: read where a 2023 record keeps it, the two values are 12494057864053 and 15569254447989, outside the range of categories (-2,100,000 to -1,990,000). With the ids undeclared, neither is a project element as rvt-rs reads one. His pack gives them the flags `0x0939`, one bit (`0x10`) more than the declared wall's `0x0929`. What they are is not established here.

Einhoven has no Revit export in the corpus, so nothing here says whether Revit exports them.

## 4. Not measured

The vectors hold each record's identity, class, category, flags and box; the ElemTable frame and record marker rows of his pack are covered elsewhere (RE-31, RE-80). His earlier pack (30 vectors of five kinds over files from 2014 to 2026) is on files that are not in the CI corpus and is not measured here.
