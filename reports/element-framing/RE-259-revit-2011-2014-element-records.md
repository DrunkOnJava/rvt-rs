# RE-259: Revit 2008 to 2014 files open, and 2011 to 2014 element records read in the 32-bit layout of Revit 2015 to 2023

**Date:** 2026-10-28
**Issues:** #421 (element records of every release); follows RE-180 (Revit 2019 to 2022) and RE-181 (Revit 2015 to 2018), which read every release in the 32-bit layout of Revit 2023 (RE-81) with its own schema tags.
**Artefacts:** eleven local files written by Revit 2008 to 2013 and one by Revit 2014. **They are closed source and are measured locally only. None may be published, attached or fetched by CI.** Among them is a copy of Autodesk's 2013 `rac_advanced_sample_project.rvt` (build 20121003_2115). Autodesk's download host has no sample project before 2015, so this copy is the only same-model pair. Its 2015 counterpart is the pinned `2015-rac_advanced_sample_project.rvt` (`0d1d77eb…`, `research/autodesk-sample-projects.tsv`).
**Probe:** `examples/probe_re259_revit_2011_2014.rs`.
**Status:** positive against the 2015 copy of the one 2013 sample project. **There is no Revit IFC export of a file written before 2015 here.** The oracle is the same model saved by Revit 2015, which RE-181 measured against its 2019 copy.

## 1. Before

No file written before Revit 2014 opened. `BasicFileInfo` was read only for a year from 2014 on, so all eleven failed with `Malformed BasicFileInfo: no 4-digit Revit version found`, in `rvt-info`, `rvt-elements` and `rvt-ifc`. The local 2014 file opened, but 2014 is outside `REVIT_32BIT_RELEASES` (RE-181), so its export was the spatial scaffold and no building element.

## 2. The release a file states, and the one it does not

| release | build | `BasicFileInfo` says | local files |
|---|---|---|---:|
| 2008 | `20070324_1700` | `Revit Build: 20070324_1700` only | 1 |
| 2009 | `20080126_1900`, `20080321_1900` | `Revit Build: <build>` only | 2 |
| 2010 | `20090317_2115` | `Autodesk Revit MEP 2010 (Build: 20090317_2115)` | 1 |
| 2011 | `20100326_1700`, `20100903_2115` | `Autodesk Revit Architecture 2011 (Build: …(x64))` | 4 |
| 2012 | `20110309_2315` | `Autodesk Revit Architecture 2012 (Build: …(x64))` | 2 |
| 2013 | `20121003_2115` | `Autodesk Revit 2013 (Build: …(x64))` | 1 |
| 2014 | `20130308_1515` | `Autodesk Revit 2014 (Build: …(x64))` | 1 |

- **2010 to 2013 state their year** as 2014 to 2018 do. Only the reader's lower bound, 2014, left them out. It is now 2008.
- **The year must stand alone.** The version scan took the first four digits in range wherever they were. With 2008 in range, a 2009 file's `20080321_1900` would read as 2008. A year is now a run of exactly four digits.
- **2008 and 2009 state no year at all.** The release is then the build year plus one. That holds on every file of this corpus that states both, from 2010 (`20090317`) to 2016 (`20150110`). It gives 2008 for `20070324_1700` and 2009 for `20080126_1900` and `20080321_1900`. The `Revit Build` line is also now read as the build where no other build string is found, so these files report one.

`rvt-info` now opens all eleven pre-2014 files. On every file of 2014 and later its output is byte-identical to main's (173 local files).

## 3. A schema without GUID lists

The schema grammar (#154) reads a class as a name, base, version, field count, fields and a `u32`-counted list of 16-byte GUIDs. On every file before 2014 it stopped at the first class with `field count 983040 at 0x36`. The schemas written before 2014 have no GUID list. A class ends with its last field and the next `00 00` separator follows. `A3PartyObject`, the second class, is `0d 00 "A3PartyObject" · 00 00 · 00 00 00 00 · 00 00 00 00` then `00 00` on 2013. On 2014 the same bytes are followed by `00 00 00 00`, a GUID count of zero.

Nothing in the stream says which form it takes. The grammar now reads with GUID lists first. When that stops early it reads without them, and keeps whichever gets further. Every schema of 2014 and later reads to its end in the first form, as before.

| release | classes | read to the end |
|---|---:|---|
| 2008 | 2,239 | yes |
| 2009 | 2,494 | yes (both builds) |
| 2010 | 2,647 | yes |
| 2011 | 2,853 | yes (both builds) |
| 2012 | 3,154 | yes |
| 2013 | 3,519 | yes |
| 2014 | 3,619 | yes, unchanged |

Each ends on `testclass`, as every later schema does.

## 4. The records are 2015's

With its schema read, each file gives the record marker of RE-80 (`Outline`, `ff`×4, `ElementParents`) and the probe looks for RE-181's framing: a declared `u32` ElementId before the marker with the tag of `ElementHeader` 8 bytes after it.

| file | release | markers | id 52 bytes before | none |
|---|---|---:|---:|---:|
| Autodesk's `rac_advanced_sample_project` | 2013 | 8,487 | 8,487 | 0 |
| its 2015 copy (pinned) | 2015 | 8,541 | 8,536 | 5 |
| local project | 2014 | 259 | 259 | 0 |
| local projects (2) | 2012 | 37 / 208 | 37 / 208 | 0 |
| local files (4) | 2011 | 39 / 106 / 74 / 66 | all | 0 |
| local file | 2010 | 12 | 12 | 0 |
| local files (2) | 2009 | 171 / 0 | 170 / 0 | 1 / 0 |
| local file | 2008 | 120 | 118 at 48 bytes | 2 |

From 2009 to 2014 the id sits 52 bytes before the marker, as on 2015 to 2023. On 2008 it is 48. `REVIT_32BIT_RELEASES` is now 2011 to 2023. 2009 and 2010 frame their markers as 2011 does, but no 2009 or 2010 file here has a same-model copy to check their fields against, and their readers' layouts are not measured. One of the 2009 files has no record marker at all. They stay unread, as does 2008.

## 5. The constants

Each release's tags of the five classes behind the 32-bit constants (RE-180 §3), read from its schema. Every file of a release gives the same tags:

| class | 2011 | 2012 | 2013 | 2014 | 2015 |
|---|---|---|---|---|---|
| `CellList` | 0x01d5 | 0x0217 | 0x0250 | 0x0251 | 0x024f |
| `Material` | 0x0668 | 0x0702 | 0x07e2 | 0x080e | 0x0871 |
| `PhysicalParamSet` | 0x07ab | 0x0865 | 0x095e | 0x09a7 | 0x0a0b |
| `PatternHelper` | 0x013d | 0x0183 | 0x01b9 | 0x01bc | 0x01b9 |
| `VerticalRegionsStructure` | 0x0aab | 0x0b8c | 0x0cfc | 0x0d5a | 0x0dc4 |

The 2014 column is RE-181 §3's. `schema_tags` now holds 2011 to 2023.

## 6. What differs from 2015

The five release lists that began at 2015 (`PARTITION_LEVEL_SUPPORTED_REVIT_VERSIONS`, `WALL_LINE_SUPPORTED_REVIT_VERSIONS`, `ROOM_SOLID_SUPPORTED_REVIT_VERSIONS`, `CATEGORY_MATERIAL_SUPPORTED_REVIT_VERSIONS` and `layer_layout`) now begin at 2011, and `wall_orientation_has_word` is false from 2011. With only that, the 2013 sample against its 2015 copy agreed on 61,441 fields and differed on 714. Two layouts account for 632 of them.

**Two ids before the pattern slots of a material's shading.** RE-116's frame is the transparency, 0.5, the pattern slots, then the colour. Revit 2011 to 2013 write two `u32` ElementIds between the 0.5 and the slots: Carbon Steel is `00 00 00 3f · ff ff ff ff ff ff ff ff · ff ff ff ff 00 00 00 00 · ff ff ff ff 00 00 00 00 · 7f 7f 7f 00` on 2013 and the same without the second group on 2015. On every frame of the local files whose colour is found (an `ff ff ff ff` slot after the 0.5, and a colour followed by shininess 64 within 48 bytes), the four 2011 files put 24 bytes between the 0.5 and the colour (68, 47, 44 and 93 frames), as do the two 2012 files (87 and 69) and the 2013 sample (98). The 2014 file puts 16 (45 frames), as does the 2015 copy (80). One 2013 material sets the second id (`0x00031298`, element 201368). Read without them, none of the 2013 sample's materials had a colour, and the layers of 142 walls, 16 floors and 65 ceilings read none. `shading_ids_32` gives 2 before 2014.

**Blocks that hold no type's map.** RE-113 reads a type's materials from the first map of the value block an ElementId and a run of `ff` open. On 2011 to 2013 the same id and mark also open `00 00 00 00 00 01`, which no value block of 2014 or later does. The 2013 sample holds 5,081 such marks of 9,490, its 2015 copy none of 5,410. The 2011 and 2012 files hold 8 to 99 each of 1,978 to 2,706, and the 2014 file 1 of 2,910, which is still read as a block. Read as blocks, their first maps gave 101 of the 2013 sample's elements (84 doors, 15 furniture, 2 masses) type materials their 2015 copies do not have, such as `Default` and `Phase - Temporary` on the doors of type 49480, whose own map is byte for byte the 2015 one. Not read, no element's type materials differ from its 2015 copy's. They now end the block before them but are not read.

**A Level's placement kind.** 2011 to 2013 Levels hold the instance kind, `0xffffef7f`, as 2019 and later do. 2014 Levels hold RE-181's `0xffffef6f`, as 2015 to 2018 do. The 5 Levels of the 2013 sample hold `ef7f` where their 2015 copies hold `ef6f`. Both read as Level elements with no change: only `ef6f` is rewritten before 2019.

## 7. The 2013 sample against its 2015 copy

`probe_re259_revit_2011_2014` compares the production elements of the two files by ElementId. It compares every field except the three that locate a record (stream, offset, class tag), with floats to nine significant digits.

| model | elements, 2013 / 2015 | same ElementId and class | fields that agree | fields that differ |
|---|---:|---:|---:|---:|
| `rac_advanced` | 5,702 / 5,775 | 5,698 | 62,071 | 82 |

Every ElementId both files hold has the same class in both. 4 materials only the 2013 file holds. The 2015 copy holds 48 more curtain-wall mullions, 20 more panels and 9 more materials. Every remaining difference is in the bytes of the two files' records:

- **A re-divided curtain wall.** 12 mullions and 8 panels have other boxes and locations: mullion 147850's box is 5.17 ft across on 2013 and 0.92 ft on 2015. The 2015 copy's 68 more mullions and panels are in the same wall.
- **A shorter wall.** Wall 140012's box starts at x = 23.77 ft on 2013 and 25.75 ft on 2015.
- **Two beams' Level.** Beams 201309 and 201320 list Level 311 among their references on 2015 and not on 2013, so on 2013 they bind to Level 694 or to none.

## 8. The exports

`rvt-ifc` exports the 2013 sample with the building elements of its 2015 copy's export, entity for entity, except 48 `IfcMember` and 20 `IfcPlate`: 180 `IfcWall`, 100 `IfcDoor`, 24 `IfcWindow`, 17 `IfcSlab`, 6 `IfcRoof`, 176 `IfcColumn`, 38 `IfcBeam`, 91 `IfcSpace`, 65 `IfcCovering`, 22 `IfcRailing`, 5 `IfcStair`, 112 `IfcBuildingElementProxy`, 3,228 `IfcMember`, 1,329 `IfcPlate`, 10 material layer sets and 5 storeys. Main writes 1,635 bytes for it, the scaffold of a file it cannot open. The 2013, 2014, a 2012 and a 2011 export pass IfcOpenShell 0.9.0's validation with no issue.

Revit 2008 to 2010 files export the spatial scaffold and their materials' names, as releases whose records are not read do (RE-181 §1).

**Nothing else moves.** `rvt-ifc` was run with this branch and with main on 132 distinct local files of 2008 to 2027, among them the pinned 2015 and 2016 sample projects, the 2015 templates and the 2016 family, and every 2017 to 2027 project at hand. With the owner-history and file-name lines set aside, which carry timestamps, the 120 exports of 2015 to 2027 are byte-identical to main's. The 12 that change are the files of 2008 to 2014.

## 9. What is not shown

- **Revit's own export of a file before 2015.** The 2015 copy is the oracle for 2013. Where 2013 reads as 2015 does, it inherits RE-181's measurements and through them RE-180's.
- **2011, 2012 and 2014 against a same-model copy.** No such pair exists here. Their schemas, tags and framing are measured (§3 to §5) and their exports validate, but their fields are not compared with anything.
- **Any public evidence.** Every file before 2015 here is closed source. The measurements cannot be repeated on CI.
- **2008 to 2010 element records.** The files open and report their release, schema and metadata. 2009 and 2010 frame their records as 2011 does, but with no same-model copy their fields are not checked, so they are not read. 2008 frames them 4 bytes closer.
- Anything 2015 to 2023 do not read: design options, layered joins, IFC export overrides, parameters.

## 10. Reproduce

With a local 2013 copy of Autodesk's `rac_advanced_sample_project.rvt`:

```bash
tools/fetch-autodesk-samples.sh samples 2015 2015
cargo run --release --example probe_re259_revit_2011_2014 -- rac_advanced_sample_project-2013.rvt samples/2015-rac_advanced_sample_project.rvt
```

A single file prints its schema, tags and framing (§3 to §5):

```bash
cargo run --release --example probe_re259_revit_2011_2014 -- file.rvt
```
