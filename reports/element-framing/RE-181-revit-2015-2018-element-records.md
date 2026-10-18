# RE-181: Revit 2015 to 2018 element records, read in the 32-bit layout of Revit 2019 to 2023

**Date:** 2026-10-07
**Issues:** #421 (element records of every release); follows RE-81 (Revit 2023 records), RE-107 to RE-116 (the 2023 readers built on them) and RE-180 (Revit 2019 to 2022 in the same layout, every constant a schema class tag).
**Artefacts:** Autodesk's own sample projects, pinned in `research/autodesk-sample-projects.tsv` and fetched by `tools/fetch-autodesk-samples.sh`, never stored: `rac_basic`, `rst_basic` and `rac_advanced` of 2016 (`8e6d1a76…`, `c2fe9b5e…`, `187608af…`), 2017 (`a1d3d077…`, `2c103da4…`, `fa1ee9b9…`), 2018 (`8fce035a…`, `47e1d228…`, `12b00efb…`) and 2019 (the RE-180 copies); all six of 2015, `rac_basic`, `rst_basic`, `rac_advanced`, `rme_basic`, `rme_advanced` and `rst_advanced` (`8ff35b3c…`, `e7063b58…`, `0d1d77eb…`, `e146321f…`, `825cb89f…`, `59d11091…`, pinned by #801), with the 2019 `rme_basic`, `rme_advanced` and `rst_advanced` (`7285e752…`, `976d3e35…`, `af89f67f…`). Also measured, by the contributor: Autodesk's 2015 templates `DefaultMetric.rte` and `Structural Analysis-DefaultMetric.rte` and family template `Metric Casework.rft` (Revit 2015, build 20140117_1515), Autodesk's `rac_basic_sample_family-2016.rfa` (as phi-ag/rvt holds it), and one local 2014 project that is not distributable. That project's origin is not documented, so it is no evidence for Revit 2014, which stays out of `REVIT_32BIT_RELEASES` (fail closed); its rows below are kept as the contributor measured them, marked as such.
**Probe:** `examples/probe_re181_revit_2015_2018.rs`.
**Status:** positive against the 2019 copy of each model. **There is no Revit IFC export of a 2015 to 2018 file here.** The oracle is the same model saved by Revit 2019, which RE-180 measured against its 2023 copy.

## 1. Before

With RE-180 a 2019 to 2023 project is read from its records, and every partition reader is still gated to 2019 and later. On a 2014 to 2018 file `rvt-elements` lists only materials named by the schema walker (939 on the 2017 `rac_basic`, 855 on the 2015 `DefaultMetric.rte`), and `rvt-ifc` writes the spatial scaffold and no building element.

## 2. The records are 2019's

For every record marker of the file's own schema (`Outline`, `ff`×4, `ElementParents`, RE-80), the probe looks back 9 to 120 bytes for a declared `u32` ElementId with the tag of `ElementHeader` 8 bytes after it:

| file | release | markers | id 52 bytes before | none |
|---|---|---:|---:|---:|
| `rac_basic` | 2016 / 2017 / 2018 | 1,238 | 1,234 | 4 |
| `rac_basic` | 2019 | 1,238 | 1,234 | 4 |
| `rst_basic` | 2016 | 6,387 | 6,372 | 15 |
| `rst_basic` | 2017 / 2018 | 6,476 | 6,367 | 109 |
| `rst_basic` | 2019 | 5,839 | 5,730 | 109 |
| `rac_advanced` | 2016 / 2017 / 2018 / 2019 | 8,541 | 8,536 | 5 |
| `rac_basic` | 2015 | 1,259 | 1,255 | 4 |
| `rst_basic` | 2015 | 1,469 | 1,465 | 4 |
| `rac_advanced` | 2015 | 8,541 | 8,536 | 5 |
| `rme_basic` | 2015 / 2019 | 11,227 / 11,222 | 11,223 / 11,218 | 4 / 4 |
| `rme_advanced` | 2015 / 2019 | 11,199 / 11,182 | 11,195 / 11,178 | 4 / 4 |
| `rst_advanced` | 2015 / 2019 | 5,093 / 4,959 | 5,087 / 4,869 | 6 / 90 |
| `rac_basic_sample_family-2016.rfa` | 2016 | 16 | 16 | 0 |
| `DefaultMetric.rte` | 2015 | 168 | 162 | 6 |
| `Structural Analysis-DefaultMetric.rte` | 2015 | 255 | 187 | 68 |
| `Metric Casework.rft` | 2015 | 4 | 4 | 0 |
| local project (not evidence) | 2014 | 259 | 259 | 0 |

No id is found at any other distance. The sample projects' rows were measured on GitHub's runners by Measure's paired probe (runs 39164971874 and, for 2015 and the 2019 MEP and `rst_advanced` projects, 39173916922); the templates', the family's and the 2014 project's are the contributor's. `REVIT_32BIT_RELEASES` is now 2015 to 2023: 2014 is left out, since only the undocumented local project backs it.

## 3. The constants

Each release's tags of the five classes behind the 32-bit constants (RE-180 §3), read from its schema:

| class | 2014 | 2015 | 2016 | 2017 | 2018 | 2019 |
|---|---|---|---|---|---|---|
| `CellList` | 0x0251 | 0x024f | 0x0255 | 0x025d | 0x0263 | 0x0264 |
| `Material` | 0x080e | 0x0871 | 0x088b | 0x08b7 | 0x08f8 | 0x0924 |
| `PhysicalParamSet` | 0x09a7 | 0x0a0b | 0x0a34 | 0x0a73 | 0x0aba | 0x0aef |
| `PatternHelper` | 0x01bc | 0x01b9 | 0x01be | 0x01bf | 0x01ca | 0x01ca |
| `VerticalRegionsStructure` | 0x0d5a | 0x0dc4 | 0x0e06 | 0x0e5e | 0x0eeb | 0x0f37 |

`schema_tags` holds 2015 to 2019. The probe finds each file's tags equal to the table on every file of §2. The 2014 column rests on the undocumented local project and is not in `schema_tags`; the 2015 column, first read from three templates of one build, is also the tags of all six 2015 sample projects (run 39173916922).

## 4. Three layouts that differ

**A Level's placement kind.** A Level record of 2015 to 2018 holds `0xffffef6f` where 2019 and later hold `0xffffef7f`, the placed-instance kind of RE-21. The bytes are otherwise the same: the 7 Levels of the 2017 `rac_basic` are those of its 2019 copy, at the same offsets from the category and the marker. Element records keep `0xffffef7f` (839 of the 2017 `rac_basic`'s records and of its 2019 copy's). Read as it was, no 2015 to 2018 Level was a Level element: no storey, and every element of the 2017 `rac_basic` unbound from its Level. `LEVEL_PLACEMENT_KIND_2015` is read as the instance kind on 2015 to 2018.

**No word before the wall flip.** Revit 2015 to 2018 write a wall's flip right after its location line, as 2019 and 2020 do (RE-180 §4): past the anchor, every wall of the 2017 `rac_basic` holds its 2019 copy's bytes. Read with the word, 20 of its walls had another exterior side than their 2019 copies, or none. `wall_orientation_has_word` is false from 2015 to 2020, and every wall of the 2016 to 2018 samples then has its 2019 copy's exterior side. The local 2014 project's walls (not evidence) hold zeros past the anchor and read the same either way.

**Two pattern slots in a material's shading.** RE-116's shading frame is the transparency, 0.5, four pattern slots of `u32` ElementId and COLORREF, then the shading colour. 2015 to 2018 write two slots where 2019 writes four: Revit 2019 split each fill pattern into a foreground and a background. Material 23 of the 2017 `rac_basic` holds `00 00 00 3f · ff ff ff ff 00 00 00 00 · ff ff ff ff 00 00 00 00 · 7f 7f 7f 00` and its 2019 copy holds the same with two more empty slots. Read with four, none of the 157 materials of the 2017 `rac_basic` had a colour. `pattern_slots_32` gives 2 before 2019.

## 5. Element-data names

A system-family type's name is the first framed string of its element data (RE-111), searched for 0x600 bytes past its header. On 2015 to 2018 two more misreadings of that search show:

- **Past the next element's data.** Level 311 of the 2018 `rac_basic` has no framed name of its own. The first framed name after its header is that of wall type 397, `Exterior - Brick on Mtl. Stud`, about 200 bytes past the next element's data header. Named, the Level became a candidate type of every wall that names it, and RE-109's rule (the candidate named by a strict subset of the categories naming each other) found none for 9 walls. On 2019 the same search hits a one-unit name first, which RE-180 drops.
- **A one-id list before the name.** Roof type 507024 of the 2016 `rac_basic` frames `兎` (a one-id list, RE-180 §5) before `SG Metal Panels roof`. RE-180 drops a one-character name, so the type had none, and 2 roofs of `rac_basic` and 3 of `rac_advanced` had no type.

On 2015 to 2018 the name is now looked for in the element's own data only, up to the next element's data header, and a one-unit entry is passed over rather than taken. 2019 to 2023 keep RE-180's reading.

## 6. The 2016 to 2018 projects against their 2019 copies

`probe_re181_revit_2015_2018` compares the production elements of the two files by ElementId. It compares every field except the three that locate a record (stream, offset, class tag), with floats to nine significant digits.

| model | elements, old / 2019 | same ElementId and class | fields that agree | fields that differ |
|---|---:|---:|---:|---:|
| `rac_basic` 2016 | 621 / 621 | 621 | 6,613 | 63 |
| `rac_basic` 2017 | 621 / 621 | 621 | 6,669 | 7 |
| `rac_basic` 2018 | 621 / 621 | 621 | 6,676 | 0 |
| `rst_basic` 2016 | 512 / 529 | 511 | 6,411 | 224 |
| `rst_basic` 2017 | 529 / 529 | 528 | 6,623 | 73 |
| `rst_basic` 2018 | 529 / 529 | 528 | 6,623 | 73 |
| `rac_advanced` 2016 | 5,773 / 5,773 | 5,773 | 62,795 | 12 |
| `rac_advanced` 2017 | 5,773 / 5,773 | 5,773 | 62,806 | 1 |
| `rac_advanced` 2018 | 5,773 / 5,773 | 5,773 | 62,807 | 0 |

The table was measured on GitHub's runners by Measure's paired probe (#799, run 39164971874), which also gives section 2's marker counts on the sample projects and section 3's tags for 2016 to 2018. As in RE-180 §6, the contributor's own run counted one more `rac_basic` element (622) and 79 fewer agreeing fields, which no commit of this branch gives on the runners.

Every ElementId both files hold has the same class in both. Every remaining difference is in the bytes of the two files:

- **Boxes and reference lists.** 2 furniture elements and a plumbing fixture of `rac_basic` 2016 and 2017, 2 generic models and a stair and its run of `rac_basic` 2016, and 17 columns and 14 beams (31 on 2016) of `rst_basic` 2016 to 2018 have other boxes in their records. On 2016 and 2017 one stair run of `rac_basic` and one beam of `rac_advanced` list one id fewer. One column only `rst_basic` 2016 to 2018 hold, and one only its 2019 copy holds.
- **Material colours.** On 2016, 12 `rac_basic` and 8 `rac_advanced` materials hold another shading colour (material 198374: `18 14 0a` on 2016, `80 40 20` on 2017 and 2019, in the same frame), one `rac_advanced` material another transparency (0.080 against 0.082), and 10 `rac_basic` materials no shading frame. The layer sets of 12 walls and 2 floors of `rac_basic` and 2 floors of `rac_advanced` differ by those colours only.
- **Model edits in `rst_basic` 2016.** The 2016 file differs from its 2017 copy as it does from its 2019 copy: the column and beam families were renamed (`HSS-Hollow Structural Section-Column` to `M_HSS Square-Column`, `88.9×5.0CHS` to `88.9x5.0CHS`), 5 materials hold other colours, and 17 analytical and zone materials were added on upgrade, as RE-180 §6 found for 2019. The 18 beams of the renamed family have no base-constraint Level on 2016 and two Levels on 2017 and 2019; this was not traced further.

**Revit 2015.** The six 2015 sample projects against their 2019 copies (run 39173916922):

| model | elements, 2015 / 2019 | same ElementId and class | fields that agree | fields that differ |
|---|---:|---:|---:|---:|
| `rac_basic` | 627 / 621 | 621 | 6,471 | 206 |
| `rac_advanced` | 5,773 / 5,773 | 5,773 | 62,772 | 70 |
| `rme_basic` | 6,903 / 6,904 | 6,899 | 80,793 | 2,640 |
| `rme_advanced` | 6,860 / 6,860 | 6,842 | 80,061 | 2,586 |
| `rst_advanced` | 953 / 1,025 | 953 | 13,279 | 109 |
| `rst_basic` | 283 / 529 | 0 | | |

The 2015 `rst_basic` shares no ElementId with its 2019 copy: it is another model, so there is nothing to compare. On the other five, every ElementId both files hold has the same class. The differing fields are not traced:

- 5,174 are type materials from RE-82's map scan: 2,585 on `rme_basic`, 2,531 on `rme_advanced` and 58 on `rac_advanced`. That scan has no per-release constant, and RE-180 §6 lists the same 30 plants and 23 beams of `rac_advanced`.
- 68 of `rst_advanced`'s 109 are renamed framing and column families and types.
- 111 of `rac_basic`'s 206 are boxes and locations of curtain-wall mullions and panels, and 32 are material colours and transparencies.

## 7. The exports

`rvt-ifc` exports every file of §2, and each export passes IfcOpenShell 0.9.0's schema validation with no issue. On `rac_basic`, `rst_basic` and `rac_advanced` of 2016 to 2018 the building elements are those of the 2019 copy's export, class for class: on `rac_basic` 56 `IfcWall`, 16 `IfcDoor`, 17 `IfcWindow`, 144 `IfcMember`, 44 `IfcPlate`, 14 `IfcSpace`, 11 `IfcSlab`, 2 `IfcRoof`, 3 `IfcStair`, 10 `IfcRailing`, 3 `IfcColumn`, 32 `IfcFurniture`, 66 `IfcBuildingElementProxy` and 3 storeys. With GlobalIds and the three record-locating properties set aside, the 2018 `rac_basic` and the 2017 and 2018 `rac_advanced` exports are line for line their 2019 copies'; the 2017 `rac_basic` export differs in the 3 elements of §6, and the 2017 and 2018 `rst_basic` exports in §6's boxes. Only material counts differ on 2016, by §6's materials. The six 2015 sample projects and the 2019 MEP and `rst_advanced` ones export too (run 39173916922), each passing IfcOpenShell's validation with no finding: on the 2015 `rac_basic` 56 `IfcWall`, 16 `IfcDoor`, 17 `IfcWindow`, 148 `IfcMember`, 11 `IfcSlab`, 3 `IfcColumn` and 2 `IfcCovering` with a predefined type. The 2015 templates and the 2016 family export their materials and no building element; the local 2014 project exported 8 walls, 12 columns, 4 windows, 3 furniture, 2 slabs and a roof on 2 storeys while 2014 was read; it no longer is.

## 8. Nothing else moves

With these changes the IFC export of 2019 to 2027 is byte-identical to that of RE-180's branch, owner-history timestamp aside, on every local 2019 to 2027 file RE-180 §8 lists, as is the export of the files of 2008 to 2013. Only the 2016 family changes, now read from its records.

## 9. What is not shown

- **Revit's own export of a 2015 to 2018 file.** The 2019 copies are the oracle. Where 2015 to 2018 read as 2019 does, they inherit RE-180's measurements; RE-180 in turn rests on the 2023 copies and RE-81 to RE-116.
- **2014 projects.** Autodesk's download host has no 2014 sample project. Revit 2014 is not read: its row of §3 rests on one project whose origin is not documented.
- **Why the 2015 fields of §6 differ.** They are counted, not explained.
- Revit 2013 and earlier. The local 2013 file reports no release in `BasicFileInfo` that the reader takes.
- Anything 2019 to 2023 do not read: design options, layered joins, IFC export overrides, parameters.

## 10. Reproduce

```bash
tools/fetch-autodesk-samples.sh samples 2016 2019
cargo run --release --example probe_re181_revit_2015_2018 -- samples/2017-rac_basic_sample_project.rvt samples/2019-rac_basic_sample_project.rvt
```

On GitHub's runners, every pair at once (each sample's `probe-pair.txt`):

```bash
gh workflow run measure.yml -f ref=main -f base=none -f samples=true -f sample_years="2015 2019" -f probe=probe_re181_revit_2015_2018 -f probe_pair_release=2019
```
