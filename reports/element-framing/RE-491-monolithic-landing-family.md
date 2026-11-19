# RE-491: Revit names a monolithic landing type's family `Monolithic Landing`, and the family words of a stair's parts are the exporting Revit's, not the file's

**Date:** 2026-11-19
**Issues:** #2046 (Revit 2027: stair landings are written with no `IfcSlabType` where Revit types them `LANDING`, CHL, 14 landings), #1829; follows RE-65 (stair component type names, Revit 2024) and RE-476 (landing types on Revit 2025 to 2027).
**Artefacts:** the BIM4LCA office (VTT, CC BY-SA 4.0; Revit 2024) with Revit's own IFC4 export of each file, whose header names `Autodesk Revit 2024 (ENU)`; Autodesk's sample projects of Revit 2024 to 2027, Snowdon Towers Architectural among them (counts and words only); the confidential Revit 2027 model CHL as counts and one word from its owner only.
**Probe:** `probe_re491_stair_family_words`.
**Status:** the rows of section 4 were registered in the draft pull request before any run was read; their results are filled in below as the runs are read.

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

Not yet read.

## 6. What this decides and does not

Filled in with section 5.

## 7. Reproduce

Filled in with section 5: the Measure runs and the commit they measured.
