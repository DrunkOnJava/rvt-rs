# RE-178: Revit 2019 to 2022 element records, read in Revit 2023's 32-bit layout

**Date:** 2026-10-07
**Issues:** #421 (element records beyond 2023 to 2027); follows RE-81 (Revit 2023 records), RE-107 to RE-116 (the 2023 readers built on them), RE-124 and RE-177 (every per-release constant is a schema class tag).
**Artefacts:** Autodesk's own sample projects, pinned in `research/autodesk-sample-projects.tsv` and fetched by `tools/fetch-autodesk-samples.sh`, never stored: `rac_basic` of 2019 (`69726bb8…`), 2020 (`80abc5c4…`), 2021 (`8828b880…`), 2022 (`131c02b8…`) and 2023 (`94f98cc1…`); `rst_basic` of 2019 (`2d05d524…`), 2020 (`64ed9130…`), 2021 (`acd13fe3…`), 2022 (`054a6e58…`) and 2023 (`091ba54f…`); `rac_advanced` of 2019 (`3f6ec9f4…`), 2020 (`c6ba792d…`), 2021 (`a20b714a…`), 2022 (`430176be…`) and 2023 (`75e8738d…`). Local copies of the 2019 `rme_basic`, `rme_advanced` and `rst_advanced`, which the pin list does not hold, were exported too.
**Probe:** `examples/probe_re178_revit_2019_2022.rs`.
**Status:** positive against the 2023 copy of each model. **There is no Revit IFC export of a 2019 to 2022 file here.** The oracle is the same model saved by Revit 2023, which RE-81 to RE-116 measured against Revit's exports, and, where 2023 reads nothing, its 2024 or 2026 copy.

## 1. Before

On `main` a 2019 to 2022 project is opened and its schema read, but every partition reader is gated to 2023 and later. `rvt-ifc` writes the spatial scaffold and nothing else: on 2019 `rac_basic`, 48,750 bytes and readiness score 0.25, against 395 bodies on 2023.

## 2. The records are 2023's

For every record marker of the file's own schema (`Outline`, `ff`×4, `ElementParents`, RE-80), the probe looks back 9 to 120 bytes for a declared `u32` ElementId with the tag of `ElementHeader` 8 bytes after it:

| model | release | markers | id 52 bytes before | none |
|---|---|---:|---:|---:|
| `rac_basic` | 2019 / 2020 | 1,238 | 1,234 | 4 |
| `rac_basic` | 2021 / 2022 | 1,239 | 1,235 | 4 |
| `rac_basic` | 2023 | 1,269 | 1,265 | 4 |
| `rst_basic` | 2019 / 2020 | 5,839 | 5,730 | 109 |
| `rst_basic` | 2021 / 2022 | 5,840 | 5,731 | 109 |
| `rst_basic` | 2023 | 6,096 | 5,987 | 109 |
| `rac_advanced` | 2019 / 2020 | 8,541 | 8,536 | 5 |
| `rac_advanced` | 2021 | 12,246 | 12,235 | 11 |
| `rac_advanced` | 2022 | 8,583 | 8,578 | 5 |

No id is found at any other distance. The `BuiltInCategory` is 38 bytes before the marker, as on 2023. The 2023 reader (`partition_element_records_2023`) therefore reads 2019 to 2022 unchanged; `REVIT_32BIT_RELEASES` is 2019 to 2023.

## 3. The constants

Every constant the 2023 readers hold is 2023's tag of a class, and 2024's counterpart is 2024's tag of the same class:

| use | class | 2019 | 2020 | 2021 | 2022 | 2023 | 2024 |
|---|---|---|---|---|---|---|---|
| element-data header (RE-111) | `CellList` | 0x0264 | 0x026c | 0x027d | 0x028d | 0x02c0 | 0x02d3 |
| material object (RE-113) | `Material` | 0x0924 | 0x094d | 0x094b | 0x09af | 0x09fb | 0x0a28 |
| material name end (RE-116) | `PhysicalParamSet` | 0x0aef | 0x0b24 | 0x0b25 | 0x0b90 | 0x0beb | 0x0c17 |
| category material frame (RE-115) | `PatternHelper` | 0x01ca | 0x01d0 | 0x01e1 | 0x01f0 | 0x013f | 0x0149 |
| layer frame (RE-112) | `VerticalRegionsStructure` | 0x0f37 | 0x0f76 | 0x0f79 | 0x1003 | 0x106f | 0x10a6 |

The record marker and Level elevations were already read from the schema (RE-80, RE-107). `schema_tags` holds the table; the probe prints each file's tags and finds them equal to it on all 12 files read, and on the 2019 `rme` and `rst_advanced` files as well. Layer records keep 2023's 29-byte layout.

## 4. One layout that differs: the wall flip

RE-54's wall orientation is the location line, a word, and the flip, after `ff ff ff ff 01 00 00 00`. Revit 2019 and 2020 write no word. The `rst_basic` wall 627064 holds `03 00 00 00 · 00 01 ff ff` past the anchor on 2019 and 2020, and `03 00 00 00 · 01 00 00 00 · 00 01 ff ff` on 2021 to 2023. Read with the word, 14 of `rac_basic`'s 40 walls and all 7 of `rst_basic`'s had no exterior side on 2019 and 2020. `wall_orientation_has_word` is false for 2019 and 2020, and the word reads 0 there. The word only decides centred bodies, which the 32-bit path does not draw, and RE-86's taper.

## 5. One-unit names

In 32-bit data, a list of one ElementId, `u32 1 · u32 id`, has the shape of a name entry of one UTF-16 unit, which is the id's low half:

- The 2019 `rac_basic` frames `U+FDCE` three times after type 754016 (from `u32 1 · 754016 · u32 1 · 130510`). The type then has two names and so none, and 10 lighting fixtures had no type. A name entry of one character outside ASCII is now not taken (`lone_non_ascii`); a one-letter ASCII name, such as a grid's, still is.
- In element data, the 2019 to 2022 `rac_advanced` gave ceiling type 247676 the name `퉼` (id 250492), and the 2019 to 2022 `rac_basic` gave element 86961, "Working Drawings", the name `p`, which made it the type of five walls. On 2019 to 2022, an element-data name of one character is not taken. Four `rac_basic` walls and two `rac_advanced` ceilings then get the type their 2024 or 2026 copy gives (`Wall - Timber Clad`, `Furred Ceiling`), where 2023 gives none.

2023 keeps RE-111's reading. With the same rule on 2023, five `rac_basic` walls would take `Exterior - Brick on Mtl. Stud`, which their 2024 copies do not give, so nothing about 2023 is changed here. Note that 2023 `rac_basic` itself names four walls' type `p` today.

## 6. The 2019 to 2022 projects against their 2023 copies

`probe_re178_revit_2019_2022` compares the production elements of the two files by ElementId. It compares every field except the three that locate a record (stream, offset, class tag), with floats to nine significant digits.

| model | elements, old / 2023 | same ElementId and class | fields that agree | fields that differ |
|---|---:|---:|---:|---:|
| `rac_basic` 2019 | 622 / 639 | 622 | 6,569 | 43 |
| `rac_basic` 2020 | 637 / 639 | 637 | 6,629 | 43 |
| `rac_basic` 2021 | 637 / 639 | 637 | 6,629 | 43 |
| `rac_basic` 2022 | 638 / 639 | 637 | 6,629 | 43 |
| `rst_basic` 2019 | 529 / 546 | 529 | 6,707 | 70 |
| `rst_basic` 2020 | 544 / 546 | 544 | 6,767 | 70 |
| `rst_basic` 2021 | 544 / 546 | 544 | 6,832 | 5 |
| `rst_basic` 2022 | 545 / 546 | 544 | 6,767 | 70 |
| `rac_advanced` 2019 | 5,773 / 5,841 | 5,755 | 61,663 | 994 |
| `rac_advanced` 2020 | 5,788 / 5,841 | 5,755 | 61,663 | 994 |
| `rac_advanced` 2021 | 5,839 / 5,841 | 5,839 | 63,289 | 115 |
| `rac_advanced` 2022 | 5,840 / 5,841 | 5,839 | 63,313 | 91 |

Every ElementId both files hold has the same class in both. The ids only one file holds are model differences: they are not in the other file's `ElemTable`.
- **Materials.** Revit added 17 analytical and zone materials when it upgraded the 2019 files (`System-Zones`, `Analytical Spaces` and others, ids above any id of the 2019 file). Two more came in 2023 (`Analytical Panels`, `Electrical Analytical Loads`), and 2022 holds one that 2023 deleted (`Electrical Load Zones`).
- **`rac_advanced` 2019 and 2020** are an earlier edit of the model. Its stairs and 13 railings were rebuilt under new ids before 2021, and its curtain walls, windows and rooms moved by 0.0246 ft (7.5 mm). That accounts for most of their 994 differing fields.

The remaining differences are:
- **Type names** (`rac_basic`, 8 walls): 4 named on 2019 to 2022 as on 2024 and not on 2023, and 4 named `p` on 2023 (section 5). Their layers and exterior side follow. In `rac_advanced` 2021 and 2022 there are 2 ceilings, also as on 2026.
- **A faucet and a sink** (`rac_basic`, 2 plumbing fixtures) have no type on 2019 to 2022. The faucet type's record also lists two subcategories of its family, `Masking Lines` and `Model Lines`, which have name entries on 2019 to 2022 but not on 2023, so RE-109's family rule (exactly one named id) finds three.
- **Type materials** (`rst_basic` 65 beams of `Timber_50 x150 _ContentGenerator`; `rac_advanced` 30 plants, 23 beams, 3 entourage, 2 generic models). These come from RE-113's first-map scan, which has no per-release constant. The 2021 and 2023 `Default` of the 65 beams is a one-entry map 6.8 kB into the type's block, the kind of match RE-177 §5 describes; 2019, 2020 and 2022 have none.
- **Reference lists** (`rst_basic` 5, `rac_advanced` 21): the raw lists differ between saves.
- **12 walls of `rac_advanced` 2021** have no exterior side. Each has two saved copies, in `Partitions/7` and `/8`, whose location lines differ by the 7.5 mm move, and the line reader drops an id whose copies disagree, on every release.

## 7. The exports

On 2019 to 2022 `rac_basic` and `rst_basic` the IFC export has the same building elements as 2023's, class for class. On `rac_basic` that is 56 `IfcWall`, 16 `IfcDoor`, 17 `IfcWindow`, 144 `IfcMember`, 44 `IfcPlate`, 14 `IfcSpace`, 11 `IfcSlab`, 2 `IfcRoof`, 3 `IfcStair`, 10 `IfcRailing`, 3 `IfcColumn`, 32 `IfcFurniture`, 66 `IfcBuildingElementProxy` and 3 storeys. On `rst_basic` it is 370 `IfcBeam`, 30 `IfcColumn`, 9 `IfcWall`, 5 `IfcSlab`, 2 `IfcMember`, 1 stair and 2 storeys. Only the material counts differ, by the materials of section 6. The 2019 `rac_advanced`, `rme_basic`, `rme_advanced` and `rst_advanced` export as well: `rvt-elements` lists 5,773, 6,904, 6,860 and 1,025 elements, materials included. All of these exports pass IfcOpenShell 0.9.0's schema validation with no issue.

## 8. Nothing else moves

With these changes the IFC export of 80 local files is byte-identical to that of the 2027 branch (#740), owner-history timestamp aside. Those files are the 2023 to 2027 files that RE-177 §6 lists, the 2027 samples, and files of 2008 to 2016.

## 9. What is not shown

- **Revit's own export of a 2019 to 2022 file.** The 2023 copies are the oracle. Where 2019 to 2022 read as 2023 does, they inherit RE-81 to RE-116's measurements against Revit; where they differ (sections 5 and 6), the 2024 and 2026 copies decide only the type names.
- 2018 and earlier. The record marker's tags exist in their schemas, but no 2018 or earlier file is read here.
- Anything 2023 does not read: design options, layered joins, IFC export overrides, parameters.

## 10. Reproduce

```bash
tools/fetch-autodesk-samples.sh samples 2019 2023
cargo run --release --example probe_re178_revit_2019_2022 -- samples/2019-rac_basic_sample_project.rvt samples/2023-racbasicsampleproject.rvt
```
