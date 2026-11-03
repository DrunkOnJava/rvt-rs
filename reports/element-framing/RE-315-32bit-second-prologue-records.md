# RE-315: Revit 2008 to 2023 frames with no ElementId of their own sit inside a declared partition record

**Date:** 2026-11-03
**Issues:** #1261 (a Revit 2018 project exports no element), #408 (W5); follows RE-30 and RE-35 (the second prologue on 2024 and later), RE-81, RE-180 and RE-181 (the 32-bit layout), RE-107 (a Level named by an element's list), RE-135 (loaded families' documents)
**Artefacts:** BIM-Speed's Vitoria `ALD26 Before.rvt` (Revit 2018, build 20190510_1515, sha256 `d3e5b04631e67265927be98352021ed6290b33ae67d39b1b6727fd76f014ff6f`, 23,699,456 bytes) and Revit's own IFC2X3 export of it (`a42c6f3b0c9b7575d4c37c675155ff53c02571ca81357a87023f9da5fbf45181`), CC BY 4.0, TU Berlin DepositOnce, doi:10.14279/depositonce-16441; BIM Whale's `AdvancedProject_Revit21.rvt` (`eba808f9dc16fa9bb6519dca836bde9820a84a3b9305b76968f236e9e6fff4b2`) and its IFC (`86b068a7ca6947e398c1daaa8bede9d31048c3990adc308429faeb56ee01db94`), both pinned in `research/holdout-archive-models.tsv` and fetched by Measure with `-f holdout=true`; the held-out `ALD26 Project`, Frigento, Gdynia and Warmond, measured by counts only.
**Probe:** `probes/examples/probe_re315_32bit_record_chain.rs`, scored by `tools/re/instances_vs_ifc_tags.py`.
**Status:** positive on Revit 2018's `ALD26 Before` against Revit's own export, and on its held-out sibling by counts. Releases 2008 to 2017 and 2019 to 2023 are not measured against an export for this kind of frame: BIM Whale's 2021 project and Frigento, Gdynia and Warmond (2020, 2021) export byte for byte what they did, since the rule finds no frame on them to give an id.

## 1. Before

Run 41671242397, base `cef24f52` (main). Revit's export of `ALD26 Before` holds 1,394 paired elements and spaces (1,321 elements with a numeric `Tag`, 73 `IfcSpace`). rvt-ifc wrote none of them: 0 building elements, 0 storeys, 84 materials, scaffold confidence 0.25. The held-out `ALD26 Project` did the same against Revit's 1,520.

The cause is the reader, not the file. `partition_element_records_2023::scan_records` took a frame only where a declared `u32` ElementId sits 52 bytes before the record marker with the tag of `ElementHeader` 8 bytes after it. On this file 652 frames open that way; 3,992 more frames in the `BuiltInCategory` band do not, 3,351 of them placed instances.

## 2. The records

Every record of a partition is `u32` ElementId, `u32` size, the `u16` tag of `ElementHeader` (1259 on this file), a `u16` count, the body, and a 12-byte trailer: a `u32`, a flag word and the size again. `ALD26 Before`'s `Partitions/0` (112,541,521 bytes) opens with one run of 12,626 of them from offset 0 to 2,534,666, 12,606 with an id `Global/ElemTable` declares. Revit's loaded families' own documents follow (RE-135), as runs of records with undeclared ids from 62,303,917 on. `Partitions/3` is one run of 941 records, `Partitions/1` one of 9. All 12,629 declared ids have a record.

The 2024 walk cannot read this run: it accepts a flag word of `0` or `0x0100_0000` only, and the trailers of this partition's 86,081 records also hold `0x010008bd` (78), `0x010011cc` (22), `0x01000c54` (8) and `0x0100089b` (7). The flag test stops after 115 records, at offset 31,871 on `0x010011cc`. Testing the tag and the echo alone, and resuming at the next place the tag sits 8 bytes into a record that validates (`record_spans`), walks the whole run, and finds the same records as testing every place the tag sits (86,081 on `Partitions/0`, none only one way).

A first measurement of this file stopped at offset 741,926 of 100,855,918 with a record-looking structure that repeated every few hundred bytes. It was the reader of that measurement, not the file: `Partitions/*` and the `Global` streams are stored in pages of 65,249 bytes that end in a 353-byte checksum, which `compression::strip_revit_page_checksums` removes before inflating, and a reader that does not loses the gzip members that straddle a checksum (11.7 MB of this partition). Nothing between runs in the stripped stream is a different record form.

## 3. Frames with no id of their own

Of the frames in the band, per partition:

| frames | `Partitions/0` | `Partitions/3` | `Partitions/1` |
|---|---:|---:|---:|
| open with their declared id and the tag | 652 | 0 | 0 |
| no id, not a placed instance | 628 | 13 | 0 |
| no id, placed instance, in a declared record whose id the frame's reference list names | 2,567 | 781 | 2 |
| no id, placed instance, in no declared record | 1 | 0 | 0 |

(The 2,567 split into 1,657 whose first word is `ff ff ff ff` and 910 whose first word is a count, 0 to 6; 679 and 102 in `Partitions/3`.) Every placed-instance frame a declared record encloses names that record's id in its own counted list of `u32` ElementIds, as an element's does (RE-23): 3,350 of 3,350 on the three partitions, in every category. The record that encloses it is therefore the frame's element, and `scan_records` gives the frame its id, with `id_from_enclosing_record` set, when:

1. the frame is a placed instance (no container reference and the placed-instance kind, the RE-21 rule);
2. a declared record encloses the frame's start;
3. the frame's list names that record's id;
4. the frame is not a curtain panel whose first word is a count (§5).

A frame that fails one gets no id, as on 2024. The same rule gives `OST_Levels` frames an id (§6).

## 4. The ids against Revit's export

Measured by the probe on rvt-rs's own bytes (run 41672953553): `scan_records` gives 3,900 records of `ALD26 Before`, 3,248 of them with the enclosing record's id, and 1,870 selected instances (one per ElementId, newest, with volume). Against Revit's 1,321 `Tag`s, 1,314 are among them; the 7 missed are 4 curtain-panel walls (§5) and 3 `IfcElementAssembly`.

By frame, whether the id from the enclosing record is a `Tag` of Revit's export (a Python reading of the correctly inflated stream with the same rule; a frame is not an element, and copies in `Partitions/3` count again):

| category | first word `ff ff ff ff` | first word a count |
|---|---|---|
| walls | 80 true | 201 true |
| doors, windows | 69, 76 true | none |
| floors, roofs | 4 true (roofs) | 9 floors, 6 roofs true |
| stairs, runs, railings | 6, none, 14 true | 6 runs true |
| structural columns | 22 true | 109 true |
| mullions | 303 true | none |
| curtain panels | 99 true | 6 true, **96 false** |
| pipes, pipe fittings | 232, 222 true | 24, 17 true |
| generic models, equipment | 22, 6, 30 true | none |

The frames of categories rvt-rs does not export read as false against the `Tag`s, and no category it exports does but the panels: rooms (`IfcSpace` carries no `Tag`) and four categories (-2008072, -2008045, -2008139, -2008141) whose elements sit next to every pipe, pipe fitting, conduit and conduit fitting with the id one higher. rvt-rs's export has 73 `IfcSpace` and no element that Revit's lacks.

## 5. Curtain panels

On this file the curtain panels of one wall are pairs of elements: an even id that Revit exports as an `IfcPlate`, and the odd id before it, which owns the even one (`Global/ElemTable`'s owner word) and whose record holds a second frame in the panel category. 96 of the 102 panel frames whose first word is a count are that second frame (64 ids, each record 699 bytes with a count of 37, the word at marker-6 3291), and none of those 64 is in Revit's export; the other 6 are panels it does write, as `IfcWallStandardCase` where the panel's type is a basic wall. All 99 whose first word is `ff ff ff ff` are in the export. The rule therefore leaves a panel frame with a counted first word without an id: 96 false frames avoided, 6 true ones lost, which are the 4 panel walls listed as missing in §7. The fields compared (first word, the word at marker-44, the word at marker-6) do not tell those 6 from the 96, so no rule is written for them.

## 6. Levels

All seven `OST_Levels` frames that are standalone placed records, container unset and the 2015 to 2018 Level placement kind `0xffffef6f` (RE-181), are frames that do not open with their declared id (first word `ff ff ff ff`). `find_level_records_32` read first-prologue frames only, so the storey set was never recovered: with elements alone (run 41668542833) 687 of 1,136 elements sat on no storey, and rvt-rs wrote 9 storeys measured from element boxes, with no Level name, against Revit's 7. A Level frame that is a standalone placed record and sits in a declared record now takes that record's id, which the Level's name and elevation block, carrying the Level's own id, is joined to (`levels_from_records_and_blocks`) and checked against (`recovered_levels_are_a_storey_set`).

## 7. Result

Run 41671242397, base `cef24f52`, head `8fc583a1` (the source of `3a33d116`, which changes only comments and the probe):

| `ALD26 Before` | base | head | Revit's export |
|---|---:|---:|---:|
| building elements written | 0 | 1,209 | 1,394 |
| paired by `Tag` (elements; spaces counted apart) | 0 | 1,136 | 1,321 |
| of those, in Revit's class or IFC4's for it | 0 | 1,136 | 1,321 |
| written that Revit's export lacks | 0 | 0 | |
| storeys | 0 | 7 (`P0` to `P6`, at Revit's elevations) | 7 |
| elements on Revit's storey | 0 | 1,073 of 1,136 | |

By class, of Revit's elements: found 1,136 of 1,321. Found in the same class: beams 1, building element proxies 43, columns 114, doors 68, members 206, plates 67, railings 14, roofs 8, slabs 9, stairs 6, stair flights 6, walls (`IfcWall`) 12, windows 70. Found in the IFC4 class of Revit's IFC2X3 one (counted found by `Tag`, as Measure counts them): 9 `IfcCurtainWall` as `IfcWall`, 128 `IfcFlowFitting` as `IfcPipeFitting`, 140 `IfcFlowSegment` as `IfcPipeSegment`, 235 `IfcWallStandardCase` as `IfcWall`. **Missing, 185**: 104 `IfcFlowSegment` (102 conduits, `OST_Conduits` -2008132, and 2 gutters, -2001391), 74 `IfcFlowFitting` (conduit fittings, -2008128), 4 `IfcWallStandardCase` (§5), 3 `IfcElementAssembly`. The probe attributes all 102 conduits, 74 conduit fittings and 2 gutters (the `Canalón` family, category -2001391) to Revit's `Tag`s, and they are not written, because their categories are not in `RECOVERED_CATEGORIES`.

Of the 1,136 written, on a storey: 1,073 are Revit's; 26 are on one where Revit contains them in an `IfcSpace`; 21 are on another (14 columns, 4 pipe segments, 3 pipe fittings); 16 are on none (9 building element proxies, 6 railings, 1 stair flight), against 687 on none before the Level change. Storey GlobalIds are not Revit's (#878).

**Held out, counts only** (`ALD26 Project`, the same building after its renovation, Revit's 1,520): base 0 paired; head 1,534 paired elements and spaces. The same classes are found in the same class (beams, proxies, columns, coverings, doors, members, plates, railings, roofs, slabs, stairs, stair flights, walls, windows), the same classes are the ones missing (74 `IfcFlowFitting`, 104 `IfcFlowSegment`, 3 `IfcElementAssembly`, 3 `IfcWallStandardCase`), and 198 elements are written that Revit's export does not hold: 95 pipe segments, 90 pipe fittings, 8 building element proxies, 2 roofs, 2 windows, 1 wall. `ALD26 Before` has none; this is not tuned on and not explained here. 1,181 of its 1,461 elements are on Revit's storey and 16 on none.

**Unchanged:** the IFC hashes of Frigento (2020), Gdynia (2021), Warmond (2021) and BIM Whale's `AdvancedProject` (2021, spent) are byte for byte those of main, with and without the Level change. BIM Whale's probe finds 4,317 records, none with the enclosing record's id, and 103 frames with no id, none a placed instance.

Run 41671247628 (`-f samples=true -f nordic=true -f r2025=true`, base `cef24f52`) exports 90 models: Autodesk's sample projects of 2015 to 2027, Core Interior, Einhoven, both Nordic offices, the RE1 models and puzzbobb's 23. 89 are byte for byte main's. `r2025-20_T_U_Roof` differs, by main's #1276 (RE-307, roofs sloped by several arrows), which this branch's base `d82fa3b8` lacks: its hash is the same on this branch with and without the Level change (runs 41668601536 and 41671247628, both heads), and the same as on main before #1276 (run 41668601536, base `8c9be026`: 0 of 90 moved, the Level change not yet in). On the 33 Autodesk samples of Revit 2015 to 2023 no frame takes an enclosing record's id (`scan_records` finds 1,166 to 12,072 records on each and none from an enclosing record): the 4 to 11 frames per file in the category band that open with no id are none of them a placed instance, and no Level hash moves.


## 8. Reproduce

```bash
gh workflow run measure.yml --ref main -f ref=3a33d1161acb76a2346dadf926b843b81b951f27 -f base=main -f holdout=true -f models="spent-* holdout-bimspeed-*" -f keep_ifc=true -f probe=probe_re315_32bit_record_chain -f probe_scorer=instances_vs_ifc_tags
```

## 9. What stays missing

- 178 conduits, conduit fittings and gutters, which are Revit's exact `Tag`s on this file and are not written because their categories are not recovered. They are the next category census (RE-33), measured on more than one file.
- 4 curtain-panel walls and the 96 panel shells (§5).
- 3 `IfcElementAssembly`, which are not instances.
- Revit 2008 to 2017 and 2019 to 2023 files with frames of this kind: none is measured against an export. The records and the rule are the same bytes on every release of the 32-bit layout; the claim here is 2018's.
- `ALD26 Project`'s 198 extra elements.
