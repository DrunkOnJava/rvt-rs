# RE-491: Revit names a monolithic landing type's family `Monolithic Landing`, and the family words of a stair's parts are the exporting Revit's, not the file's

**Date:** 2026-11-19
**Issues:** #2046 (Revit 2027: stair landings are written with no `IfcSlabType` where Revit types them `LANDING`, CHL, 14 landings), #1829; follows RE-65 (stair component type names, Revit 2024) and RE-476 (landing types on Revit 2025 to 2027).
**Artefacts:** the BIM4LCA office (VTT, CC BY-SA 4.0; Revit 2024) with Revit's own IFC4 export of each file, whose header names `Autodesk Revit 2024 (ENU)`; Autodesk's sample projects of Revit 2024 to 2027, Snowdon Towers Architectural among them (counts and words only); the confidential Revit 2027 model CHL as counts and one word from its owner only.
**Probe:** `probe_re491_stair_family_words`.
**Status:** positive. A landing of a monolithic type now carries the family Revit's English export gives it, `Monolithic Landing`, with its `ObjectType` and its `IfcSlabType`: 86 landings on the eight BIM4LCA files, all 86 equal to Revit's `ObjectType`, and `rac_basic`'s one landing the same on its 2024 to 2027 saves. The family words of a stair's parts are not in the file: negative, on 43 files. The rows of section 4 were registered in the draft pull request before any run was read.

## 1. The question

RE-476 gave a landing of Revit 2025 to 2027 its type name and, where its landing type is not monolithic, the family `Non-Monolithic Landing`. It left a monolithic landing out: "the family of a monolithic landing type is not measured on any release". HWT-25 then counted CHL's 14 landings on 0.25.0: all 14 are of a monolithic type, Revit's export types each by an `IfcSlabType` `LANDING` whose family word is `Descansillo monolítico`, in a Spanish Revit 2027, and rvt-rs writes none of the 14 a type. Two questions follow:

1. What family word does an English Revit write for a monolithic landing type?
2. Is the family word in the file, so that a reader could write the word of the Revit that saved it?

## 2. What Revit's exports hold

- **The BIM4LCA office, concrete, as built** (Revit's export, IFC4 Reference View, application `Autodesk Revit 2024 (ENU)`): 27 `IfcSlab` are `.LANDING.`. 21 of them carry the `ObjectType` family `Monolithic Landing` and are typed by one `IfcSlabType` `LANDING`; 1 carries `Non-Monolithic Landing`; 5 are floors (`Floor`) that the export types as landings.
- **Snowdon Towers Architectural** (Revit 2024, local only): 20 landing slabs, all 20 with the `ObjectType` family `Non-Monolithic Landing`, 17 of them typed. None is monolithic, so this file could not answer question 1 (RE-65).
- **CHL** (Revit 2027, Spanish; its owner's counts): 14 of 14 landings monolithic, the family word `Descansillo monolítico`.

So an English Revit names the family `Monolithic Landing`, 21 of 21 on the one open file that holds such landings.

## 3. Whether the file holds the word

Revit's export of Snowdon Towers Architectural names the stair of its 20 landings `Assembled Stair`. Searched as UTF-16 and as ASCII in every stream of the `.rvt`, as stored and in every gzip member, that word is found nowhere, while the names of the file's own stair and landing types are found (in the partition that holds the type records). The words `Monolithic Run` and `Non-Monolithic Run`, which the export gives the runs, are found nowhere either. `probe_re491_stair_family_words` repeats the search on any file; section 4, row 5, registers it on the sample projects Measure reads.

Together with CHL's Spanish word for the same system family, this says the word is the exporting Revit's, in its own language, and not a value of the file. A reader of the file cannot know which Revit would export it, so rvt-rs writes one language's words: the English ones, as it already does for a stair (`Assembled Stair`, `Cast-In-Place Stair`), a run (`Monolithic Run`, `Non-Monolithic Run`) and a support (`Stringer`, `Carriage`). Not measured: the same file exported by two Revits of different languages, which is the direct test.

## 4. The change, and the rows registered before any run

- Revit 2024 (`partition_stairs::component_type_at`): a landing type whose flag at `LANDING_TYPE_MONOLITHIC_OFFSET` is set names the family `Monolithic Landing`; it named none.
- Revit 2025 to 2027 (`native_stairs::native_stair_parts`): a `StairsLandingType` whose `m_hasMonolithicSupport` is true names the family `Monolithic Landing`; it named none.

Rows:

1. **The office's monolithic landings carry Revit's family.** On each BIM4LCA office file, every landing whose `ObjectType` in Revit's export starts `Monolithic Landing:` carries the same `ObjectType` in rvt-rs's export, and is typed: 21 of 21 on the concrete office as built, and the counts the run gives on the three other office files.
2. **Nothing else moves on Revit 2024.** The only exports that differ from `main`'s are files that hold a landing of a monolithic type, and in them only those landings' `ObjectType`, type object and the property sets that hang on the type. No scorer's "different" or "values Revit does not write" count rises.
3. **Revit 2025 to 2027 held to the same project's 2024 save.** `rac_basic`'s one landing, of a monolithic type (RE-476, row 5), carries on its 2025, 2026 and 2027 saves the `Family:Type` its 2024 save carries, by ElementId, and has a type object on each; on 2026 and 2027 its `ObjectType` stays `$` (#1472).
4. **Held-out models.** With `holdout=true`, no held-out pair gains a "different" value.
5. **The family words are not in the files.** On the sample projects of Revit 2024 to 2027 and the office files, `probe_re491_stair_family_words` finds `Assembled Stair`, `Cast-In-Place Stair`, `Monolithic Run` and `Non-Monolithic Run` only where a type of the file is itself named so; the count of files with any hit of each word is reported, whatever it is.

## 5. Results

Measure 44475053774 ran `f3d414a6` against `main` at `62cc17e0` on 120 models: the reference models, the BIM4LCA office and housing, the 24 Revit 2025 models, Autodesk's samples of every release and the held-out pairs.

1. **Met.** Every landing whose `ObjectType` Revit writes as `Monolithic Landing:<type>` now carries it (`names_vs_ifc`, `ObjectType differs` to `ObjectType equal`), and every one is typed (`type_objects_vs_ifc`, untyped `IfcSlab` to none):

   | File | Landings | `ObjectType` equal before | after | `IfcSlabType` `LANDING`, rvt-rs before, after, Revit |
   | --- | ---: | ---: | ---: | --- |
   | office, concrete, as built | 21 | 0 | 21 | 2, 3, 3 |
   | office, concrete, permit | 21 | 0 | 21 | 2, 3, 3 |
   | office, timber, as built | 17 | 0 | 17 | 2, 3, 3 |
   | office, timber, permit | 17 | 0 | 17 | 2, 3, 3 |
   | housing, concrete, as built | 4 | 0 | 4 | 1, 2, 2 |
   | housing, concrete, permit | 4 | 0 | 4 | 1, 2, 2 |
   | housing, timber, as built | 1 | 0 | 1 | 1, 2, 2 |
   | housing, timber, permit | 1 | 0 | 1 | 1, 2, 2 |

   86 landings, two buildings. The registered count for the concrete office as built, 21 of 21, is met; the housing holds such landings too, which the row did not name.
2. **Met, with one count that rises and is not one of the two the row names.** 108 of 120 exports are byte-identical and 102 of 102 `diagnostics.json`. The 12 that differ are the eight files above and `rac_basic` saved by Revit 2024, 2025, 2026 and 2027, each of which holds a landing of a monolithic type. Scorer output moves on the eight files only, in `names_vs_ifc`, `predefined_types_vs_ifc`, `psets_vs_ifc` and `type_objects_vs_ifc`. On each file `psets_vs_ifc`'s "different" (1) and "values Revit does not write" are unchanged, and "equal" is unchanged. "Missing" rises by one on each: Revit writes `Pset_SlabCommon.IsExternal` (`False`) on the landing's type object, which did not exist in rvt-rs's export before and now does without that value. It is reported here and not written: no reader gives a landing type's `IsExternal`.

   A landing that is not named as a part of its stair also takes the Name every element with a family takes, `Family:Type:ElementId`, where it had the placeholder `StairsLanding-<id>`. That is `rac_basic`'s one landing and the housing's; on the housing `names_vs_ifc` counts their Name as differing from Revit's before and after, since Revit names a landing after its stair. The office's 76 are named after their stair and equal Revit's Name before and after.
3. **Met.** Measure 44480695855 (`probe_2046_landing_types`, exports kept) on `rac_basic` and `rac_advanced` of Revit 2024 to 2027: `rac_basic`'s landing 725396 has no family and no type object on `main` and, with the change, the family and type `Monolithic Landing:220mm Thickness` and one `IfcSlabType` of that name on each of the four saves. Its `ObjectType` is that `Family:Type` on 2024 and 2025 and stays `$` on 2026 and 2027. `rac_advanced`'s landings, of a type that is not monolithic, are unchanged (21 of 25 exports of that run identical, the four `rac_basic` saves the others).
4. **Met.** The 18 held-out pairs of run 44475053774: no scorer line of any of them moves.
5. **Met for the two words no type of these files is named after; the others are found once a file, as a type's name would be.** Measure 44482570891 ran `probe_re491_stair_family_words` at `58511189` on 43 files (Core Interior, Einhoven, the four RE1 models, the 12 BIM4LCA files and the 25 sample projects of Revit 2024 to 2027): 1,084 streams and 84,361 gzip members searched.

   | Word | Files with the word as a string of its own | Hits |
   | --- | ---: | ---: |
   | `Monolithic Landing` (not after `Non-`) | 0 of 43 | 0 |
   | `Non-Monolithic Run` | 0 of 43 | 0 |
   | `Assembled Stair` | 8 | 8 |
   | `Cast-In-Place Stair` | 5 | 5 |
   | `Monolithic Run` | 17 | 17 |
   | `Non-Monolithic Landing` | 39 | 39 |

   - `Monolithic Landing` stands alone in no file, the eight BIM4LCA office and housing files among them, whose Revit exports write it for 1 to 21 landings each. `Non-Monolithic Run` is in no file, Snowdon Towers Architectural among them, whose Revit export writes it on 46 flights.
   - Each of the other four words is found at most once in a file, every time as a whole string. The files that hold `Assembled Stair` are the eight `rme_basic` and `rme_advanced` samples, and those that hold `Cast-In-Place Stair` are Core Interior, Einhoven and three building-services samples of Snowdon Towers; Snowdon Towers Architectural, whose Revit export names 26 stairs `Assembled Stair` and 3 `Cast-In-Place Stair`, holds neither. On the BIM4LCA office, Revit's export shows a flight type named `Monolithic Run` and a landing type named `Non-Monolithic Landing`, the two words its files hold. So a file holds one of these words where one of its types is named so, and not for the elements whose family Revit's export names with it.
   - Not decoded: which record holds each of those single strings. `Stringer` and `Carriage` are words of many type names (711 and 304 hits) and say nothing either way.

`same-project gates` passes on run 44475053774.

## 6. What this decides and does not

- **Decided.** A monolithic landing's family is `Monolithic Landing`, by its type's flag on Revit 2024 and by `m_hasMonolithicSupport` on Revit 2025 to 2027. The word is measured against Revit's own export on Revit 2024 files of two buildings, both exported by an English Revit 2024.
- **Decided.** A reader of the file cannot give the family word of the Revit that exports it. rvt-rs writes the English words.
- **Not decided.** CHL's 14 landings: its owner's next counts say whether they are typed, and their family word will read `Monolithic Landing` where that Spanish Revit's export reads `Descansillo monolítico`. A Revit export of a 2025 to 2027 model with a monolithic landing, by an English Revit: none is in Measure. The `IsExternal` of a landing's type. The Name of a landing that is not joined to its stair.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=f3d414a602a5f0765a4df3ffedce6fe2ce911eb5 -f base=62cc17e0d8bc8781cfab482ca50e8216e0a3ce1d \
  -f nordic=true -f r2025=true -f samples=true -f holdout=true
# rows 1, 2 and 4: names_vs_ifc, predefined_types_vs_ifc, psets_vs_ifc and type_objects_vs_ifc of the nordic-* models
gh workflow run measure.yml --ref main -f ref=f3d414a602a5f0765a4df3ffedce6fe2ce911eb5 -f base=62cc17e0d8bc8781cfab482ca50e8216e0a3ce1d \
  -f samples=true -f sample_years='2024 2027' -f models='sample-*-racbasicsampleproject sample-*-racadvancedsampleproject' \
  -f scorers=none -f probe=probe_2046_landing_types -f keep_ifc=true
# row 3: the `landing <id> type <name>` lines of each save
gh workflow run measure.yml --ref main -f ref=58511189ceafcff83eecaa972a87169b49fd924c -f base=none \
  -f nordic=true -f samples=true -f sample_years='2024 2027' -f scorers=none -f probe=probe_re491_stair_family_words
# row 5: the `total <hits> alone <hits not after Non-> word "<word>"` lines of each file
```
