# RE-146: the Q4 word is the grandparent's tag, and `ClassEntry::tag` is the base's

**Date:** 2026-10-03
**Issues:** #154; reported by puzzbobb (#154, issuecomment-5951329667)
**Artefacts:** 53 files, each read on GitHub's hosted runners with the Measure workflow, run 37116713027:
- the eleven Autodesk family files of `phi-ag/rvt` (2016 to 2026);
- Autodesk's 36 sample projects of 2016 to 2027 (`research/autodesk-sample-projects.tsv`);
- Core Interior (2024), Einhoven (2023) and the four RE1 models (2025).

**Probe:** `examples/probe_re146_q4_word.rs`.
**Status:** positive, on every file. It answers RE-137 §4 ("what the legacy word is").

## 1. The claim under test

`formats::parse_schema`, the older parser, keeps two words on each class it tags: `ClassEntry::tag` and `ClassEntry::ancestor_tag`. The Q4 addendum read the second as an ancestor distinct from the parent; RE-137 found that reading did not reproduce and left the word open. puzzbobb reported, against `formats::schema_classes` (every class, its definition-ordinal tag and its base, RE-133):

1. `ClassEntry::tag` is the tag of the class's **base**, never the class's own.
2. `ClassEntry::ancestor_tag` is the tag of the **base's base**.

He found this on 1,488 of 1,488 `ancestor_tag`s and 6,192 of 6,192 tags over 19 files.

## 2. Method

For every class `parse_schema` tags whose name `schema_classes` also reads, the probe compares:
- `ClassEntry::tag` with the class's own tag and with its base's tag;
- `ClassEntry::ancestor_tag`, where set, with the tag of the base's base.

Both tags come from `schema_classes`. The probe keeps up to five counterexamples of each.

## 3. Result

| files | release | classes (`schema_classes`) | tagged by `parse_schema` | tag is the base's | tag is the class's own | with `ancestor_tag` | `ancestor_tag` is the grandparent's |
|---|---|---:|---:|---:|---:|---:|---:|
| family, 3 samples | 2016 | 3,792 | 328 | **328** | 0 | 77 | **77** |
| family, 3 samples | 2017 | 3,879 | 331 | **331** | 0 | 78 | **78** |
| family, 3 samples | 2018 | 4,024 | 343 | **343** | 0 | 78 | **78** |
| family, 3 samples | 2019 | 4,098 | 346 | **346** | 0 | 82 | **82** |
| family, 3 samples | 2020 | 4,164 | 350 | **350** | 0 | 83 | **83** |
| family, 3 samples | 2021 | 4,168 | 351 | **351** | 0 | 83 | **83** |
| family, 3 samples | 2022 | 4,307 | 373 | **373** | 0 | 94 | **94** |
| family, Einhoven, 3 samples | 2023 | 4,418 | 391 | **391** | 0 | 98 | **98** |
| family, Core Interior, 3 samples | 2024 | 4,492 | 400 | **400** | 0 | 98 | **98** |
| family, RE1 (four), 3 samples | 2025 | 4,600 | 403 | **403** | 0 | 98 | **98** |
| family, 3 samples | 2026 | 4,690 | 412 | **412** | 0 | 98 | **98** |
| 3 samples | 2027 | 4,757 | 415 | **415** | 0 | 97 | **97** |

Every file of a release gives the same counts, because files of one release share one schema. No counterexample was found on any file.

- **`ClassEntry::tag` is the base's tag.** This held on every tagged class of every file, and the tag was never the class's own.
- **`ancestor_tag` is the grandparent's tag.** This held on every class that has one. puzzbobb's explanation fits: the old parser lands on the base's own base reference when the base is defined inline right after the class. The Q4 addendum looked the word up among `parse_schema`'s tags, which are base tags. So each class it named was the class whose base is the grandparent, which is why its nine rows looked like distinct ancestors (RE-137).

## 4. What this changes

- `ClassEntry::tag` and `ClassEntry::ancestor_tag` are documented as the base's tag and the grandparent's tag.
- `docs/rvt-moat-break-reconnaissance.md`'s Q4 addendum now says what the word is.
- **Open, not fixed here.** Code that uses `ClassEntry::tag` as the class's own tag reads a base tag:
  - the schema-directed walker indexes instance headers by it (`walker::scan_candidates_with_control`);
  - so does `SchemaTable::tagged_ancestor`;
  - `rvt-analyze` reads the same word from its own scan.

  Element records name their class by its definition ordinal (RE-76, RE-133), which `schema_classes` gives. Moving those readers to it is backlog item B24.

## 5. What this does not claim

- **Before 2016.** puzzbobb also reports that schemas before Revit 2014 carry no per-class GUID list, and that `schema_classes` reads no class from 2008 to 2013 files. No file older than 2016 is in the corpora measured here, so that part is not reproduced. The 2014 family pinned in `research/public-families.tsv` (RE-147) can check the 2014 side.
