# RE-472: no Revit 2015 to 2023 file of the corpus sets an IFC export parameter, so where those releases keep them is not decided

**Date:** 2026-11-15
**Issues:** #1695 (Revit 2022 and earlier: the export parameters are not read, so a furniture element carries no `PredefinedType` where Revit's export writes `NOTDEFINED`); follows RE-45 (the four export parameters), RE-111 (the element data header of a 32-bit release), RE-417 and RE-419 (`Export to IFC As` values, furniture `PredefinedType`) and RE-280 (the id shape before 2024).
**Artefacts:** every model Measure fetches with `samples`, `holdout` and `bimcollab` (85 models; Autodesk's sample projects of Revit 2015 to 2027, the reference models, the held-out models as counts only, BIMcollab's example), never stored.
**Probe:** `probe_1695_export_parameters_pre2024` (commit `b8715e75b644`, archived under `refs/archive/2026-11-15/cited/b8715e75b644`); `tools/ci/scrub-holdout-results.sh` keeps its count lines for a held-out model.
**Status:** negative, undecided. None of the 45 files of Revit 2015 to 2023 sets any of the four parameters, and neither do 38 of the 40 files of Revit 2024 to 2027, so the corpus cannot show where a 32-bit release keeps them. Nothing in the output changes.

## 1. The question

`scan_export_parameters` reads an element's four IFC export parameters (`Export to IFC As`, `IFC Predefined Type`, and the two a type sets: `-1019014` to `-1019017`) from Revit 2024 on, as entries `i64 BuiltInParameter · u32 n · n UTF-16 units` (RE-45). The 32-bit branch of the partition decode never calls it, and a release before 2024 has no 64-bit element data header, so a Revit 2022 furniture element cannot say `CHAIR` or `BED` where Revit's export does (RE-419: `PredefinedType` equal on 0 of 4 and 0 of 8 furniture on the held-out Revit 2022 pair, Revit writing `NOTDEFINED`). #1695 asks first where the 32-bit releases keep the two parameters.

## 2. What the probe counts

For each file, every partition stream:

- **The key, in two widths.** Each of the four keys as `i64` (the 2024 form) and as `i32` (a hit whose next four bytes are not `ff ff ff ff`), with a value after it that RE-45 accepts (ASCII letters, digits and underscores, an `Ifc` name for the two "as" keys).
- **The owner.** The last element data header before the entry within `EXPORT_PARAMETER_WINDOW` (`element_data_layout`: a `u32` id from Revit 2008 to 2023), and whether the id is declared in `Global/ElemTable`.
- **Without a key.** Every `u32 n · UTF-16` string equal to a word of `IfcFurnitureTypeEnum` (`BED`, `CHAIR`, `DESK`, `FILECABINET`, `SHELF`, `SOFA`, `TABLE`, `TECHNICALCABINET`, `USERDEFINED`, `NOTDEFINED`), with the 8 bytes before it, which would name the key's own encoding in a release that keeps it in another form.

## 3. Measured

Measure run 43678279528 (head `b8715e75b644`, `-f samples=true -f holdout=true -f bimcollab=true -f scorers=none`, the probe on all 85 models; the rows were registered in the pull request before the run was read).

| Release | Models | Entries in either width | Strings equal to a furniture word |
| --- | ---: | ---: | ---: |
| 2015 | 7 | 0 | 0 |
| 2016 | 3 | 0 | 0 |
| 2017 | 3 | 0 | 0 |
| 2018 | 5 | 0 | 0 |
| 2019 | 6 | 0 | 0 |
| 2020 | 5 | 0 | 0 |
| 2021 | 5 | 0 | 0 |
| 2022 | 5 | 0 | 0 |
| 2023 | 6 | 0 | 0 |
| 2024 | 13 | 58 | 0 |
| 2025 | 12 | 2 | 0 |
| 2026 | 8 | 0 | 18 |
| 2027 | 7 | 0 | 0 |

- **No file before 2024 sets a parameter, in either width.** All 45 files of Revit 2015 to 2023 print 0 entries and 0 furniture words, the held-out Revit 2022 pair among them (both files: 0 and 0, counts only). The element data layout the probe used on them is the `u32` one.
- **The 2024 form has one positive control.** `2024_Core_Interior.rvt` holds all 58 entries of Revit 2024, every one in the `i64` form (`IfcShadingDevice`, `IfcSpace`, `ifcSlab` and `ifcFooting` for `Export to IFC As`; `ROOF` and 7 texts that are not IFC words for the predefined type), and one held-out Revit 2025 project 2. The other 38 files of 2024 to 2027, Autodesk's sample projects of those years among them, set none. The BIM4LCA files (Revit 2024), which hold the `CHAIR` and `BED` of RE-419, are not in this run.
- **The keyless search finds words in one file only.** The single-dwelling house of the CIADD pair (Revit 2026, held out) holds 18 strings equal to a furniture word, 9 after `00 00 01 00 00 00 00 00` and 9 after `ff ff 00 00 00 00 00 00`. Neither prefix is one of the four keys, and Revit writes `NOTDEFINED` on all 6 of that house's furniture; the words are not export parameter entries.

## 4. The registered rows

1. **The 2024 form on a pre-2024 save.** Not met, and not refuted: 0 entries on all 45 files of 2015 to 2023, but the 2024 saves of the same projects hold none either, so a pre-2024 form that differs from the 2024 one would show the same zero.
2. **The narrow form.** Not met: 0 entries in the `i32` form on any file.
3. **The owner.** Not measurable: no entry before 2024 to resolve. The `u32` header rule was never exercised.
4. **The keyless search.** Not triggered (it was to run when a 2024 save held entries a pre-2024 one did not); run anyway on every file, it found no furniture word before 2024.
5. **The held-out Revit 2022 pair.** 0 entries and 0 furniture words in each file.

## 5. What this decides and does not

- **Decided.** Where a Revit 2015 to 2023 file keeps these parameters is not observed in 45 files, because none of them sets one. It is not shown absent: a release before 2024 may store them in the 2024 form, in a narrower one or elsewhere, and this corpus has nothing to tell the three apart.
- **The held-out Revit 2022 pair.** Neither file holds any string equal to a furniture word under any key, which is what 12 furniture with `NOTDEFINED` in Revit's export would leave if the parameters are stored as UTF-16 strings as on 2024. It cannot be shown that they are.
- **A rule that was considered and not written.** Give furniture and casework `NOTDEFINED` on a release before 2024 whose file holds no string equal to a furniture word anywhere. It would read 4 of 4 and 8 of 8 on the held-out Revit 2022 pair, and it is never wrong on the corpus because the corpus holds no file that sets the parameter. It is not written: its premise, that a parameter set on a 32-bit release is stored as a UTF-16 string a file-wide search finds, has no positive control, the measured set is 12 elements of two files of one release (RE-353), and a file that sets `CHAIR` in a form this search cannot find would be written `NOTDEFINED` wrongly. The standing rule is to leave out what the evidence does not decide.
- **The elements #1695 cannot tell**, and why: every furniture, casework and other class of a Revit 2015 to 2023 file whose `PredefinedType` depends on a parameter, because no file of the corpus sets one to read it by. The exports of those releases are unchanged.

## 6. What would decide it

A Revit 2015 to 2023 file that sets `IFC Predefined Type` or `Export to IFC As` on an instance or a type, with Revit's IFC4 export of it: run the same probe on it (the rows above apply as registered), and a reader for the form it shows is a change of its own with the held-out Revit 2022 pair and `holdout=true` before landing. The Revit Automation API route (Griffin, 2026-11-11) would produce such a file.

## 7. Reproduce

```bash
gh workflow run measure.yml --ref research/1695-export-parameters-pre2024 -f ref=b8715e75b644085c91e79da0b33e74b38b65b10c -f base=none \
  -f samples=true -f holdout=true -f bimcollab=true -f scorers=none -f probe=probe_1695_export_parameters_pre2024
# each model's probe.txt (counts.txt for a held-out model) holds its `export parameters pre2024:` lines
```

The measured commit `b8715e75b644` is archived under `refs/archive/2026-11-15/cited/` because the pull request is squashed.
