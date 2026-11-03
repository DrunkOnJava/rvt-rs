# RE-315: Revit 2008 to 2023 frames with no ElementId of their own sit inside a declared partition record (measured on Revit 2018), and the export phase and design options on 2016 to 2023

**Date:** 2026-11-03
**Issues:** #1261 (a Revit 2018 project exports no element), #408 (W5); follows RE-30 and RE-35 (the second prologue on 2024 and later), RE-81, RE-180 and RE-181 (the 32-bit layout), RE-107 (a Level named by an element's list), RE-135 (loaded families' documents)
**Artefacts:** BIM-Speed's Vitoria `ALD26 Before.rvt` (Revit 2018, build 20190510_1515, sha256 `d3e5b04631e67265927be98352021ed6290b33ae67d39b1b6727fd76f014ff6f`, 23,699,456 bytes) and Revit's own IFC2X3 export of it (`a42c6f3b0c9b7575d4c37c675155ff53c02571ca81357a87023f9da5fbf45181`), CC BY 4.0, TU Berlin DepositOnce, doi:10.14279/depositonce-16441, pinned in `research/holdout-archive-models.tsv` and fetched by Measure with `-f holdout=true`; Autodesk's `rst_basic_sample_project` of 2016 to 2023 (`research/autodesk-sample-projects.tsv`) for the design option layout; and, by counts only, the held-out `ALD26 Project`, Frigento, Gdynia, Warmond and BIM Whale's `AdvancedProject`.
**Probe:** `probes/examples/probe_re315_32bit_record_chain.rs`, scored by `tools/re/instances_vs_ifc_tags.py`.
**Status:** positive on Revit 2018's `ALD26 Before` against Revit's own export, and on its held-out sibling by counts. Releases other than 2018 are not measured against an export for this kind of frame. Design options (§10) are measured on Autodesk's `rst_basic` samples of 2016 to 2023 as diagnostics only, with no Revit export of them, and the phase filter (§10) only by the held-out `ALD26 Project`'s counts.

## 1. Before

Run 41707934595, main's side (`8664b94a`). Revit's export of `ALD26 Before` holds 1,394 elements and spaces (1,321 elements with a numeric `Tag`, 73 `IfcSpace`). rvt-ifc wrote none of them: 0 building elements, 0 storeys, 84 materials, scaffold confidence 0.25. The held-out `ALD26 Project` did the same against Revit's 1,520.

The cause is the reader, not the file. `partition_element_records_2023::scan_records` took a frame only where a declared `u32` ElementId sits 52 bytes before the record marker with the tag of `ElementHeader` 8 bytes after it. On this file 652 frames open that way; 3,992 more frames in the `BuiltInCategory` band do not, 3,351 of them placed instances.

## 2. The records

Every record of a partition is `u32` ElementId, `u32` size, the `u16` tag of `ElementHeader` (1259 on this file), a `u16` count, the body, and a 12-byte trailer: a `u32`, a flag word and the size again. `ALD26 Before`'s `Partitions/0` (112,541,521 bytes) opens with one run of 12,626 of them from offset 0 to 2,534,666, 12,606 with an id `Global/ElemTable` declares. Revit's loaded families' own documents follow (RE-135), as runs of records with undeclared ids from 62,303,917 on. `Partitions/3` is one run of 941 records, `Partitions/1` one of 9. All 12,629 declared ids have a record.

What reads this run is a walk that tolerates any flag word. The 2024 walk cannot: it accepts a flag word of `0` or `0x0100_0000` only, and the trailers of this partition's 86,081 records also hold `0x010008bd` (78), `0x010011cc` (22), `0x01000c54` (8) and `0x0100089b` (7). The flag test stops after 115 records, at offset 31,871 on `0x010011cc`. Testing the tag and the echo alone (`record_spans`) walks the whole run. Where a record fails to validate the walk resumes at the next place the tag sits 8 bytes into one that does; that only matters after the run, through the loaded families' documents, and adds no declared span. Both ways find the same records as testing every place the tag sits (86,081 on `Partitions/0`, none only one way).

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

A frame that fails one gets no id, as on 2024. `OST_Levels` frames are given an id by a related rule whose guard is a different one (§6).

## 4. The ids against Revit's export

Measured by the probe on rvt-rs's own bytes (run 41707939060): `scan_records` gives 3,900 records of `ALD26 Before`, 3,248 of them with the enclosing record's id, and 1,870 selected instances (one per ElementId, newest, with volume). Against Revit's 1,321 `Tag`s, 1,314 are among them; the 7 missed are 4 curtain-panel walls (§5) and 3 `IfcElementAssembly`.

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

All seven `OST_Levels` frames that are standalone placed records, container unset and the 2015 to 2018 Level placement kind `0xffffef6f` (RE-181), are frames that do not open with their declared id (first word `ff ff ff ff`). `find_level_records_32` read first-prologue frames only, so the storey set was never recovered: at the commit that gave elements their enclosing id and no more (`9ff5e950`, run 41707946855) 687 of the 1,136 elements sat on no storey, and rvt-rs wrote 9 storeys measured from element boxes, named `Elevation 0.000 ft` and so on, against Revit's 7.

A Level frame that is a standalone placed record and sits in a declared record now takes that record's id, but the guard of §3 does not exist for it: a Level frame's counted reference list is empty (0 entries on all 24 Level-category frames of this file, those with their own id as well), and the Level's name block does not lie in its record (the records are 1.5 MB into `Partitions/0`, the name blocks 11.9 MB). What stands in its place is two checks that need no constant of this file: the frame's class word, 6 bytes before its record marker, is the tag of the schema's class `Level`, and the stream holds a name and elevation block of the enclosing record's id. Without them a stray frame of the Level category inside another element would add an id with no name, and `recovered_levels_are_a_storey_set` would then drop every storey. The probe prints the schema's `Level` tag and the class word of every Level-category frame: on this file `Level` is tag 2130 and the frames are 2130 standalone x7, 2130 other x16 (container members), and 2131 x1 (the type symbol); all 7 survive and 7 Levels are recovered with a name and an elevation (run 41707939060). The same holds on Autodesk's `rst_basic_sample_project` of 2018 to 2023, whose 9 standalone Level frames each carry `Level`'s tag (2130, 2164, 2198, 2199, 2299 and 2377 on 2018 to 2023, 54 of 54), though those open with their own id and are not held to the check. A frame that opens with its declared id is read as before.

## 7. Result

Run 41707934595, main's side `8664b94a`, head `01b5af37` (the code of this report; the later commits change the export warning's text, cap the option sets searched at 64, merge main and change documents, and §10 repeats the held-out and identity runs at the final head):

| `ALD26 Before` | main | head | Revit's export |
|---|---:|---:|---:|
| building elements and spaces written | 0 | 1,209 | 1,394 |
| elements written and paired by `Tag` (spaces counted apart) | 0 | 1,136 | 1,321 |
| of those, in Revit's class or IFC4's for its IFC2X3 one | 0 | 1,127 | |
| written that Revit's export lacks | 0 | 0 | |
| storeys | 0 | 7 | 7 |
| elements on Revit's storey | 0 | 1,073 of 1,136 | |

By class, of Revit's elements: 1,136 of 1,321 are written, **1,127 in Revit's class or IFC4's for it**. In the same class: beams 1, building element proxies 43, columns 114, doors 68, members 206, plates 67, railings 14, roofs 8, slabs 9, stairs 6, stair flights 6, walls (`IfcWall`) 12, windows 70 (624). In IFC4's class for Revit's IFC2X3 one, counted found by `Tag` as Measure counts them: 128 `IfcFlowFitting` as `IfcPipeFitting`, 140 `IfcFlowSegment` as `IfcPipeSegment`, 235 `IfcWallStandardCase` as `IfcWall` (503). **In another class: 9 `IfcCurtainWall`, written as `IfcWall`** (Revit's export has IfcCurtainWall; IFC4 does too). **Missing, 185**: 104 `IfcFlowSegment` (102 conduits, `OST_Conduits` -2008132, and 2 gutters, -2001391), 74 `IfcFlowFitting` (conduit fittings, -2008128), 4 `IfcWallStandardCase` (§5), 3 `IfcElementAssembly`. The probe attributes all 102 conduits, 74 conduit fittings and 2 gutters (the `Canalón` family, category -2001391) to Revit's `Tag`s, and they are not written, because their categories are not in `RECOVERED_CATEGORIES`.

Of the 1,136 written, on a storey: 1,073 are Revit's; 26 are on one where Revit contains them in an `IfcSpace`; 21 are on another (14 columns, 4 pipe segments, 3 pipe fittings); 16 are on none (9 building element proxies, 6 railings, 1 stair flight), against 687 on none before the Level change. The 7 storeys are as many as Revit's; the run does not score their names or elevations, and their GlobalIds are not Revit's (#878).

**Held out, counts only** (`ALD26 Project`, Revit's 1,520): main writes 0 elements and spaces. The first reading of the branch, before the filters (run 41713761592, commit `8fc583a1`), wrote 1,534 elements and spaces, **198 of them elements that Revit's export does not hold** (95 pipe segments, 90 pipe fittings, 8 building element proxies, 2 roofs, 2 windows, 1 wall). `ALD26 Before` has none. With the export phase and design options of §10 it writes 1,336 (1,263 elements and 73 spaces), **no element that Revit's export lacks**, with every class found before still found: the same classes in the same class (beams, proxies, columns, coverings, doors, members, plates, railings, roofs, slabs, stairs, stair flights, walls, windows), the same classes missing (74 `IfcFlowFitting`, 104 `IfcFlowSegment`, 3 `IfcElementAssembly`, 3 `IfcWallStandardCase`), 8 `IfcCurtainWall` written as `IfcWall`, so 1,255 of its 1,263 elements are in Revit's class or IFC4's. 1,181 of the 1,263 are on Revit's storey and 14 on none. The counts cannot say which of the two filters removed the 198, and the held-out file is not opened to find out.

**Unchanged:** the IFC hashes of Frigento (2020), Gdynia (2021), Warmond (2021) and the held-out BIM Whale `AdvancedProject` (2021) are byte for byte main's, and BIM Whale's counts lines are main's (run 41707934595).

Run 41707942875 (`-f samples=true -f nordic=true -f r2025=true`, main's side `8664b94a`, head `01b5af37`) exports the Autodesk sample projects of 2015 to 2027, Core Interior, Einhoven, both Nordic offices, the RE1 models and puzzbobb's 23: 92 models, 0 differ from main's IFC, the 33 Autodesk samples of 2015 to 2023 among them. No sample frame takes an enclosing id, no phase excludes an element on any sample, and the only diagnostics that change are the eight `rst_basic` saves of 2016 to 2023 (§10).

## 8. Reproduce

```bash
# counts, held-out models by counts only (BIM Whale among them)
gh workflow run measure.yml --ref main -f ref=01b5af37f7e7d186ac7b60e4c49b85ffdab36dd6 -f base=main -f holdout=true -f models="spent-* holdout-bimspeed-* holdout-bimwhale-*" -f keep_ifc=true
# the probe, on Before and the samples of 2018 to 2023 only
gh workflow run measure.yml --ref main -f ref=01b5af37f7e7d186ac7b60e4c49b85ffdab36dd6 -f base=main -f holdout=true -f samples=true -f sample_years="2018 2023" -f models="spent-*" -f keep_ifc=true -f probe=probe_re315_32bit_record_chain -f probe_scorer=instances_vs_ifc_tags
# byte identity
gh workflow run measure.yml --ref main -f ref=01b5af37f7e7d186ac7b60e4c49b85ffdab36dd6 -f base=main -f samples=true -f nordic=true -f r2025=true
# the final head: every held-out pair, counts only, and byte identity
gh workflow run measure.yml --ref main -f ref=68ae7904d5b97350cd2aae670d262b8e982edcaf -f base=main -f holdout=true
gh workflow run measure.yml --ref main -f ref=68ae7904d5b97350cd2aae670d262b8e982edcaf -f base=main -f samples=true -f nordic=true -f r2025=true -f scorers=none
# the held-out Project before the filters of §10
gh workflow run measure.yml --ref main -f ref=8fc583a165267ef842138d299472e7daecdd2bfe -f base=none -f holdout=true -f models="holdout-bimspeed-vitoria-project-2018"
# the elements-only commit of §6
gh workflow run measure.yml --ref main -f ref=9ff5e9502b874685bd89360155023fb8e739fac2 -f base=none -f holdout=true -f models="spent-*"
```

## 9. What stays missing

- 178 conduits, conduit fittings and gutters, which are Revit's exact `Tag`s on this file and are not written because their categories are not recovered. They are the next category census (RE-33), measured on more than one file.
- 4 curtain-panel walls and the 96 panel shells (§5).
- 3 `IfcElementAssembly`, which are not instances.
- 9 curtain walls written as walls.
- Revit 2008 to 2017 and 2019 to 2023 files with frames of this kind: none is measured against an export. The records and the rule are the same bytes on every release of the 32-bit layout; the claim here is 2018's.
- Extras on the held-out pairs that the filters of §10 do not remove: Frigento 22 and Gdynia 3 elements that Revit's export lacks, their exports byte for byte main's. They are another cause, not looked into here.

## 10. The export phase and design options on Revit 2016 to 2023

The first reading of the held-out `ALD26 Project` wrote 198 elements that Revit's export does not hold (95 pipe segments, 90 pipe fittings, 8 proxies, 2 roofs, 2 windows, 1 wall), `Before` none. The cause sought was a filter 2024 applies and the 32-bit path does not. Of what 2024 filters, the 32-bit path had the instance rule, the volume test and the nested-in-opening test, and a symbol frame never takes an enclosing id. It had neither of the other two, whether a frame opens with its id or sits in a record:

- **The export phase** (RE-184). `native_phases::compute_phase_filter` reads 2015 to 2027 (RE-281), but only 2024 records were tested with `PhaseFilter::excludes`. **Evidence that applying it is right on a 32-bit file: the held-out `ALD26 Project`'s counts and nothing else.** On `ALD26 Before` the filter reads 1 phase and no element outside it (run 41707939060), and on every one of Autodesk's 33 samples of 2015 to 2023 it reads 2 or 3 phases and no element outside the last, with no read error, so neither can show it; the counts of `Project` fell from 198 extra elements to none and cannot separate the phase from the design options below.
- **Design options** (RE-40). 2024 left out the elements of a set's non-primary options. The bytes carry over with `u32` ids, read on Autodesk's `rst_basic_sample_project`, the one Autodesk model with an option set (2015 has none), where the 2024 and 2025 copies are the oracle: the set record has its category at `+14` and its list at `+58`; the name entry is the 2024 one with `u32` ids ("Option Set 1", options 875635, 875638 and 875641, primary 875635); and the `u32` 26 bytes before a frame's marker is its option, `ff ff ff ff` for the main model (the first `u32` of that 8-byte slot is an object a line belongs to, and is not read). **Evidence: eight saves of one model, 2016 to 2023: 1 set and 4 frames naming the non-primary option, 875638, on each (a type symbol and three instances, one of them with volume, a `Mass`); Revit does not export a `Mass`, so no export effect is observed, and no Revit export of these files is measured.** The 2024 and 2025 copies count 1 `Mass` in a non-primary option as well. `ALD26 Before` has no option set (the category ids occur only in a loaded family's document). A value that is not an option of a resolved set excludes nothing, and an unresolved set keeps its elements, as on 2024.

Both are applied on Revit 2016 to 2023 only (`partition_element_records_2023::EXPORT_FILTER_RELEASES`): the layout is read on those releases' samples and the phase filter measured on a 2018 file. On 2008 to 2015 no set is read and no element is left out, because there is no file of those releases with a set or an element outside the phase to measure on. (BIMcollab's Revit 2015 example has 8 elements outside its last phase and none of them a written instance, so it shows nothing either way.) `recover_2023_records` drops a selected instance that either filter excludes, rooms never by phase. The counts of the diagnostics (`element_record_not_in_export_phase`, `element_record_in_non_primary_design_option`) are read on the same releases.

**What changes on the eight `rst_basic` samples** (2018 to 2023: run 41707939060, 2016 and 2017: run 41707942875): the exported IFC is byte for byte main's and `building_elements` is 454 on each. The diagnostics change: a skipped item `element_record_in_non_primary_design_option` appears with 1 `Mass`; `decoded.production_class_counts` loses its `Mass` and `production_walker_elements` falls by 1 (530 to 529 on 2018 and 2019, 545 to 544 on 2020 and 2021, 546 to 545 on 2022, 547 to 546 on 2023, 513 to 512 on 2016, 530 to 529 on 2017); `decoded.mean_element_confidence` moves by less than 0.0001. The 2015 sample, which has no set, and the 2024 to 2027 samples do not change.

Result (run 41707934595, head `01b5af37`, counts only for the held-out):

- `ALD26 Project`: 1,336 elements and spaces written (1,263 elements and 73 spaces) against Revit's 1,520; **no element written that Revit's export lacks** (198 before the filters); every class found in the same class as before, none lost; 1,255 of the 1,263 elements in Revit's class or IFC4's, 8 curtain walls as walls; 1,181 of 1,263 on Revit's storey and 14 on none.
- `ALD26 Before`: unchanged by the filters and by the Level checks of §6, 1,136 of 1,321, none extra, 7 storeys; its IFC is the one of the previous head.
- Frigento, Gdynia, Warmond and BIM Whale: IFC byte for byte main's. Frigento (22) and Gdynia (3) keep elements Revit's export lacks.
- **The export warning** (`diagnostics.warnings`) said design options were not read for every 2008 to 2023 export. It now says per release: on 2016 to 2023 the export phase and design options are applied, on 2008 to 2015 neither is. A file's option sets are searched for at most 64 (`OPTION_SETS_MAX`); the sets past it, by ElementId, stay unresolved and keep their elements, as a set with no name entry does on 2024. The most any file here has is 2.

**At the final head** (`68ae7904`, the merge of main `53e06c02` into `c04c92dc`):

- Run 41722201615 (`-f holdout=true -f base=main`, no model filter: Core Interior, Einhoven, the four RE1 models, the spent `ALD26 Before` and every held-out pair of Measure, counts only for the held-out): the exported IFC of 23 of the 25 models is byte for byte main's. The two that move are the two Vitoria saves. The `counts.txt` of the other 17 held-out models is main's line for line, 0 lines differ: the Revit 2023 project `modelo_bim` (the only 2023 pair in Measure's held-out set; the other 2023 project of the 255ribeiro set, `Exemplo_data`, is local only and not measured here), the 2022 and 2024 to 2027 pairs and offices, Frigento, Gdynia, Warmond, BIM Whale, `AA-SingleDwellingHouse` and `teste_export_2025`. `ALD26 Before`'s class sets and storeys and `ALD26 Project`'s counts are those of run 41707934595.
- Run 41722205132 (`-f samples=true -f nordic=true -f r2025=true -f scorers=none`): 92 models, 0 differ from main's IFC. The diagnostics change on the 33 samples of 2015 to 2023 (the warning's text) and, in the skipped items, on the eight `rst_basic` saves of 2016 to 2023.
