# RE-280: the native record framing on Revit 2015 to 2022

**Date:** 2026-10-30
**Issues:** #1158; follows RE-163 (content documents 2016 to 2027), RE-180 and RE-181 (the partition element records of 2015 to 2022 against newer copies), RE-198 and RE-208 (BIMcollab's Revit 2015 example)
**Artefacts:** Autodesk's sample projects of 2015 to 2023 as `research/autodesk-sample-projects.tsv` pins them (33 files: `rac_basic`, `rst_basic` and `rac_advanced` of every release, and the 2015 and 2019 `rme_basic`, `rme_advanced` and `rst_advanced`), fetched by `tools/fetch-autodesk-samples.sh` and never stored; BIMcollab's Revit 2015 structural example (`02_BIMcollab_Example_STR.rvt`, sha256 `8cd1f122…e193`, no licence stated: only its counts left the runner); Einhoven (2023) and Core Interior (2024) as controls.
**Runs:** Measure [41109577387](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41109577387) (the probe at `dcea0704`), [41111795982](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41111795982) (the probe at `7333d9f7`, which adds the declared-size check, the Element base read from the graph and the refusal text) and [41113517494](https://github.com/DrunkOnJava/rvt-rs/actions/runs/41113517494) (the same probe, 2015 against 2019 for the MEP and `rst_advanced` projects). The tables below are the second and third runs.
**Probe:** `probes/examples/probe_re280_native_framing_pre2023.rs`.
**Status:** positive. The framing `native_document` reads Revit 2023 with holds on every group of every file of 2015 to 2022, with one difference: before Revit 2018 a group's declared body size counts every record's trailing length word. The preamble parses on all 30 samples of 2015 to 2022 and on BIMcollab, bar the extensible-storage catalogue of nine, the object graphs decode on 98.8 to 100 per cent of the declared elements (97.8 to 97.9 on the 2015 MEP projects, whose extensible-storage catalogue does not read), and the records agree with the same project's 2023 copy. Three things stop the readers built on `extract_records` short of these releases, none of them its framing: its Element base contract refuses `Identifier` as 2015 to 2023 write it, the extensible-storage catalogue does not read before Revit 2021 where a file holds schemas, and `VarSketch` records of 2015 to 2017 use a pointer modifier it does not know. There is no Revit export of any sample here, so this is agreement with the 2023 copy of the same project, as RE-180 and RE-181 measured; BIMcollab has Revit's export and is the subject of the next report.

## 1. The question

`native_document::extract_records` admits Revit 2023 to 2027 and refuses every other release ("native record framing is unvalidated for Revit N"). It frames a group of records as `id · [u32] · u32 length · body · u32 length`, with a `u32` id on 2023 and a `u64` id from 2024, and it reads a preamble of five global streams: the schema registry, `Global/History`, `Global/ElemTable`, `Global/DocumentIncrementTable` and the extensible-storage catalogue in `Global/Latest`. Saved parameters, phases, types, stairs, symbol cut loops, mullion hosts and saved meshes are read from those records, so they are missing on eight releases, and #879's 16 footings need the parameter store of a Revit 2015 file.

Measured here, per release: does the preamble parse; does every group frame, and under which id width and header; do its declared counts and body bytes agree; do the records' graphs decode; and do the records of a project agree with the same project's 2023 copy.

## 2. Method

The probe rebuilds the preamble from the public pieces, one stream at a time (`read_single`, `schema_registry::parse`, `native_index::creation_episodes`, `parse` and `storage_increments`, `native_es_catalog::decode`), so a failure names its stream and, for a stream that does not read, the gzip member offsets 0, 4, 8 and 16 at which it has the magic and inflates. It walks every partition with `native_segments::walk_inflated` and, for each group of channels 101 to 103, plain and keyed, tries eight framings: an id of 4 or 8 bytes with 0, 4, 8 or 12 bytes before the length. For each it records whether the dual lengths walk the group exactly, whether the record count equals the group's declared object count, whether the bodies' bytes equal the declared body bytes, and whether they equal the declared bytes less 4 per record.

For the channel-102 records of declared elements it decodes the object graph (`decode_graph_with_catalog_usage`, the default budgets and the file's own catalogue), checks the owner id the graph names against the framed id, runs `native_element::decode_base`, and projects the saved parameter sets (`native_metadata::project_with_definitions`). The record a reader would take is the one `current_partitions` picks. With two files (the sample first, its 2023 copy second) it compares the projects by ElementId: class, every field of every graph object (offsets, pointer tokens and class tags left out), the Element base scalars read from the graph, the parameters by id, the element index (`unique_id`, original id, creation episode, owner, partition) and the schema (every class's parent and fields).

## 3. The preamble

| stream | 2015 to 2022 (30 samples and BIMcollab) |
|---|---|
| `Formats/Latest` | parses on all 31, to the terminator: 3,725 classes on 2015 (BIMcollab and the samples), 4,307 on 2022, 4,418 on 2023 |
| `Global/History` | parses on all 31 (807 episodes on the 2015 `rac_basic`, 291 on BIMcollab) |
| `Global/ElemTable` | parses on all 31 with 28-byte records (`ElemTable` has 4 fields and `ElemRec` 4, as on 2023; 2024's `ElemTable` has 5 and its records 40 bytes); the ids equal `elem_table::declared_element_ids` on all 31 (21,814 on BIMcollab) |
| `Global/DocumentIncrementTable` | parses on all 31 (30 rows on BIMcollab, 253 on the 2015 `rac_basic`) |
| `Global/Latest` (extensible-storage catalogue) | parses on 22 of 31, with 0 to 8 schemas (the 2023 samples and Einhoven parse it too); **fails on 9**, the 2016 to 2020 `rst_basic` and the 2015 and 2019 `rme_basic` and `rme_advanced`: `ES string m_typeId absent` |

The catalogue failure is a release difference: `ESField` has `m_specTypeId`, a `ForgeTypeId`, on 2021 and later (`shape ESField … m_specTypeId:14.0.0>ForgeTypeId`), and 2015 to 2020 save `m_unitType`, an `int32`, in its place. `Global/Latest`'s own graph decodes; only the catalogue's field table reads a name that does not exist. A record with an extensible-storage entity then fails closed ("ES entity schema catalog absent"): 7 records of each such `rst_basic` (6 `RebarShape`, 1 `DataStorage`) and, on the 2015 MEP projects, 561 `FamilyInstance` records of the `rme_basic` and 555 of the `rme_advanced`.

`Formats/Latest` is the only stream whose member starts at offset 0; the others start at 8, as `read_single` reads them, on every release. No stream needed another offset.

## 4. The framing

Every group of channels 101, 102 and 103 of every file of 2015 to 2022 frames, plain and keyed: 12,005 groups of the 13 files of 2015 to 2017 (12 samples and BIMcollab) and 15,757 of the 18 samples of 2018 to 2022, with 969,943 and 1,249,889 channel-102 records. Under the layout of Revit 2023:

- channel 101: `u32 id · u32 length · body · u32 length`;
- channels 102 and 103: `u32 id · u32 word · u32 length · body · u32 length`.

The id is 4 bytes on every release from 2015: the 16-byte header of an 8-byte id with the word (`id8+4`) frames no group of any file of 2015 to 2022, and no group frames at both widths (`both_widths=0` in every line). The 12-byte header read as an 8-byte id with no word (`id8+0`) is the same bytes, and its ids are not the table's: the `u32` ids equal the declared ElementIds, with every declared id holding a channel-102 record on all 34 files of 2015 to 2023 (`declared_without_record=0`) and 1 to 8 records of undeclared ids on each. The word before the length takes at least 64 distinct values on every channel of every file and is not read. Each element has one record on each of the three channels: 8,051 on each for the 2015 `rac_basic`, 21,897 for BIMcollab.

**The declared sizes differ before Revit 2018.** A group's segment markers declare an object count and a body size, and `group_records` requires the records to equal both. The count agrees on every group of every release. The body size is the records' bodies on 2018 to 2023 and **the bodies plus 4 bytes per record on 2015 to 2017**:

| release | groups | framed | count equals declared | bodies equal declared bytes | bodies plus 4 per record equal declared bytes |
|---|---:|---:|---:|---:|---:|
| 2015 to 2017 (12 samples, BIMcollab) | 12,005 | 12,005 | 12,005 | 0 | 12,005 |
| 2018 to 2022 (18 samples) | 15,757 | 15,757 | 15,757 | 15,757 | 0 |
| 2023 (3 samples, Einhoven) | 2,305 | 2,305 | 2,305 | 2,305 | 0 |

That is what the first run's `declared` column showed: 0 groups of 2015, 2016 and 2017 under `group_records`' rule, all of 2018 to 2023. The release is the one that adds `SignatureMarker` to the schema (absent on 2015 to 2017, present from 2018 with tags 3349, 3404, 3460, 3463, 3580 and 3681 on 2018 to 2023) and, as `native_segments` records (#421), sets `m_continuationBits`' count to 1: three changes at one release. The extra word, the id width and the header are the same on all 13 files before it and all 18 after.

## 5. The Element base

`native_element::decode_base` refuses every record of every file of 2015 to 2023: `unsupported Identifier value`, 2,615 of 2,615 records of Einhoven and every record of the 2023 samples. It requires `Identifier` to be a single `m_id64` of 8 bytes (descriptor 11), which is Revit 2024's. On 2015 to 2023 the schema has `Identifier` as one `m_id` of 4 bytes (`shape Identifier … fields=1: m_id:4.0.0`; Core Interior, 2024, has `m_id64:11.0.0`), and `ElementId`, `Element` (20 fields) and the pointer fields are the same. So the phase order and the phased elements that RE-184 reads are not read on Revit 2023 either, and `native_json::element_id` (`/m_id/m_id64`) finds nothing in a 2015 to 2023 graph.

The graph decoder is not affected: it follows the schema, and the owner id agrees with the framed id on every decoded record (16,967 of 16,967 on the 2015 `rac_advanced`, 8,153 of 8,153 on the 2022 `rac_basic`, 21,548 of 21,548 on BIMcollab; no disagreement on the older file of any of the 24 pairs). Read from the graph's root through either shape of `Identifier`, the Element base is there on all but 2 to 10 of the records that decode (8,000 of 8,002 on the 2015 `rac_basic`, 21,538 of 21,548 on BIMcollab). Against the 2023 copy it agrees on all but a few: `m_ownerDBViewId` is -1 on the older file and a view's id on 2023's for 7 elements of every `rac_basic`, 8 of every `rac_advanced` and 15 of every `rst_basic` from 2016 to 2022 (the same counts on every release), and 30 elements of the 2016 `rst_basic` differ in `m_assocLevelId`. The family, phases, design option and the three flags agree on every other shared record. The 2015 BIMcollab model has 427 elements with a created phase and 285 with an associated level.

## 6. The graphs

| refusal | releases | records | cause |
|---|---|---:|---|
| `VarSketch` | 2015 to 2017 | 45 of each `rac_basic`, 132 of each `rac_advanced`, 136 of each 2016 and 2017 `rst_basic` (49 of the 2015's), 99 of each 2015 `rme_*`, 75 of the 2015 `rst_advanced`, 263 on BIMcollab | `VarParam.m_pSubst` is `14.4.0`, a pointer with modifier 4, which the decoder does not know (`VarParam` is `m_val` alone on 2018 and later) |
| `ImageHolder` | all, 2023 included | 2 of each `rac_basic` (3 on BIMcollab) | `ARasterImage.m_compressedImage`: container count budget |
| `SiteSurface` | all, 2023 included | 1 of each `rac_advanced` | `TriangulationDriverMemento`: 100,000-value budget |
| `RebarShape`, `DataStorage` | 2016 to 2020, `rst_basic` | 7 of each | extensible-storage catalogue (section 3) |
| `FamilyInstance` | 2015 `rme_basic`, `rme_advanced` | 561, 555 | the same |

Everything else decodes: 8,002 of 8,049 on the 2015 `rac_basic` (99.4 per cent), 16,967 of 17,100 on the 2015 `rac_advanced`, 14,202 of 14,251 on the 2015 `rst_basic`, 21,548 of 21,814 on BIMcollab (98.8 per cent, the lowest outside the MEP projects), 31,389 of 32,044 on the 2015 `rme_advanced` and 29,744 of 30,405 on its `rme_basic` (97.9 and 97.8 per cent, with the catalogue failure); 99.98 per cent on the 2022 `rac_basic` (8,153 of 8,155) and 100 per cent on the 2021 and 2022 `rst_basic`. A refused record is an `unsupported_graph` record in `extract_records` and costs nothing else. The saved parameter sets project on every record that decodes: 17,715 parameters on 5,386 BIMcollab records, 5,418 on 1,260 of the 2015 `rac_basic`'s.

## 7. The same project in 2023

`rac_basic`, `rst_basic` and `rac_advanced`, each release against its 2023 copy, by ElementId. The ids both files hold:

| sample | shared ids, same class | Element base equal | fields agree / differ | records with parameters, identical / compared | parameter values agree / differ |
|---|---:|---:|---:|---:|---:|
| 2015 `rac_advanced` | 15,502 of 15,502 | 15,373 of 15,381 | 974,067 / 7,164 | 1,576 / 2,249 | 6,787 / 4 |
| 2015 `rac_basic` | 7,514 of 7,514 | 7,458 of 7,465 | 438,922 / 5,741 | 911 / 1,181 | 5,092 / 1 |
| 2016 `rac_advanced` | 15,508 of 15,508 | 15,379 of 15,387 | 996,635 / 7,084 | 1,578 / 2,249 | 6,789 / 2 |
| 2016 `rac_basic` | 7,532 of 7,532 | 7,476 of 7,483 | 441,484 / 5,392 | 946 / 1,174 | 5,113 / 0 |
| 2016 `rst_basic` | 12,660 of 12,660 | 12,470 of 12,515 | 757,235 / 12,248 | 685 / 891 | 4,842 / 23 |
| 2017 `rac_advanced` | 15,560 of 15,560 | 15,431 of 15,439 | 1,152,346 / 5,746 | 1,890 / 2,139 | 7,986 / 2 |
| 2017 `rac_basic` | 7,660 of 7,660 | 7,604 of 7,611 | 581,352 / 4,917 | 1,093 / 1,106 | 5,757 / 0 |
| 2017 `rst_basic` | 12,921 of 12,921 | 12,761 of 12,776 | 853,945 / 7,282 | 785 / 836 | 5,248 / 23 |
| 2018 `rac_advanced` | 15,625 of 15,625 | 15,610 of 15,618 | 1,303,397 / 6,214 | 1,892 / 2,141 | 7,997 / 2 |
| 2018 `rac_basic` | 7,725 of 7,725 | 7,714 of 7,721 | 627,029 / 4,614 | 1,095 / 1,108 | 5,768 / 0 |
| 2018 `rst_basic` | 12,921 of 12,921 | 12,897 of 12,912 | 930,935 / 6,296 | 797 / 836 | 5,248 / 23 |
| 2019 `rac_advanced` | 15,626 of 15,627 | 15,611 of 15,619 | 1,318,815 / 5,591 | 1,899 / 2,141 | 8,004 / 2 |
| 2019 `rac_basic` | 7,760 of 7,760 | 7,749 of 7,756 | 638,329 / 3,421 | 1,102 / 1,108 | 5,775 / 0 |
| 2019 `rst_basic` | 13,035 of 13,035 | 13,011 of 13,026 | 948,568 / 3,655 | 834 / 838 | 5,294 / 0 |
| 2020 `rac_advanced` | 15,626 of 15,627 | 15,611 of 15,619 | 1,458,604 / 6,332 | 1,899 / 2,141 | 8,004 / 2 |
| 2020 `rac_basic` | 7,803 of 7,803 | 7,792 of 7,799 | 662,890 / 2,885 | 1,109 / 1,109 | 5,783 / 0 |
| 2020 `rst_basic` | 13,078 of 13,078 | 13,054 of 13,069 | 955,542 / 3,417 | 835 / 839 | 5,296 / 0 |
| 2021 `rac_advanced` | 16,113 of 16,113 | 16,098 of 16,106 | 1,579,484 / 574 | 2,163 / 2,167 | 8,291 / 0 |
| 2021 `rac_basic` | 7,877 of 7,877 | 7,866 of 7,873 | 678,245 / 534 | 1,110 / 1,110 | 5,793 / 0 |
| 2021 `rst_basic` | 13,152 of 13,152 | 13,135 of 13,150 | 963,486 / 908 | 836 / 840 | 5,306 / 0 |
| 2022 `rac_advanced` | 16,181 of 16,181 | 16,166 of 16,174 | 1,587,451 / 643 | 2,167 / 2,167 | 8,311 / 0 |
| 2022 `rac_basic` | 7,946 of 7,946 | 7,935 of 7,942 | 684,473 / 984 | 1,110 / 1,110 | 5,793 / 0 |
| 2022 `rst_basic` | 13,257 of 13,257 | 13,240 of 13,255 | 996,595 / 1,225 | 840 / 840 | 5,310 / 0 |

- **Every ElementId both files hold has the same class**, 7,514 to 16,181 per pair, on all 23 comparable pairs; the one exception is a single `rac_advanced` element of 2019 and 2020. The `unique_id` the element index gives agrees with the 2023 copy's on every shared id of 19 of the 23 pairs, on all but 65 for the 2018 `rac_basic` and `rac_advanced` and on all but 1 for the 2019 and 2020 `rac_advanced`. So `Global/History` and `Global/ElemTable` give the same identities as 2023 reads.
- **The 2015 `rst_basic` is not the project the other releases hold.** It shares 561 ids with the 2023 copy, of which 222 have the same class, and none has the same `unique_id`: it is another file under the same name, as RE-181 saw from its marker counts (1,469 against 5,839). It is read here and not compared.
- **Parameters.** A saved parameter that both files hold has the same value in all but 0 to 4 per pair, and in all but 23 for the 2016 to 2018 `rst_basic`; every pair of 2021 and 2022 agrees on every shared parameter (5,793 of 5,793 on the 2021 and 2022 `rac_basic`, 5,310 of 5,310 on the 2022 `rst_basic`). A differing value is a model edit (parameter -1015043 is 49.5456 on the 2016 `rst_basic` and 49.2128 on its 2023 copy); the parameters one file holds and its copy does not are those of elements Revit added or removed between the saves.
- **Fields.** The fields that differ are 0.04 to 1.6 per cent of those compared, and name what Revit changed in the schema or the model, not a misreading: `ParamDef*.m_typeId` (`autodesk.parameter.aec.revit.family:16209-1.0.0` on 2022, `revit.local.family:…` on 2023: 308 `ParamDefValue` of the 2022 `rac_basic`), `TableCellData.m_formatOptions` (2,385 on 2020), `m_unitType` where 2021 and later have `m_specTypeId` (`ESField`; `ParamDefValue.m_unitType` is on the 2020 file only), categories and styles lists that grew, and the tags and view classes whose fields were reorganised. 72 to 378 classes' fields differ between a sample's schema and 2023's (`changed_fields` in the probe's `schema classes` line), and the object lists of 45 (2022 `rac_basic`) to 7,302 (2015 `rac_advanced`) graphs per pair differ in length for the same reason.
- **MEP and `rst_advanced`, 2015 against 2019** (run 41113517494; the 2019 copy is not an admitted oracle, only the one nearest in release):

| sample | shared ids, same class | Element base equal | fields agree / differ | records with parameters, identical / compared | parameter values agree / differ |
|---|---:|---:|---:|---:|---:|
| 2015 `rme_advanced` | 28,717 of 28,717 | 27,503 of 27,507 | 2,263,280 / 25,389 | 6,155 / 6,584 | 15,551 / 1 |
| 2015 `rme_basic` | 27,102 of 27,102 | 25,830 of 25,832 | 2,192,654 / 25,787 | 5,864 / 6,290 | 14,757 / 1 |
| 2015 `rst_advanced` | 13,046 of 13,046 | 12,998 of 12,998 | 777,371 / 7,617 | 1,790 / 2,188 | 5,405 / 0 |

## 8. BIMcollab's Revit 2015 structural example

The run keeps only counts for this model (`tools/ci/scrub-holdout-results.sh`). The preamble reads complete (291 episodes, 21,814 identities, 30 increment rows, an empty catalogue). All 1,079 groups of the three channels, plain and keyed, frame under the 2023 layout and agree under the 2015 to 2017 size rule; 21,897 records on each channel. Graphs: 21,548 of 21,814 declared elements decode (263 `VarSketch`, 3 `ImageHolder`), the owner id agrees on all; 5,386 records hold 17,715 saved parameters. The Element base read from the graph is there on 21,538 records with 285 associated levels and 427 created phases set. `decode_base` refuses all 21,814. Which of the 285 are the 16 footings #879 needs, and whether their level is the storey Revit's export puts them on, is measured with the readers admitted, not here.

## 9. What admits what

| | 2015 to 2017 | 2018 to 2022 |
|---|---|---|
| preamble | parses | parses |
| group framing | the 2023 layout | the 2023 layout |
| declared sizes | bodies plus 4 per record | bodies, as 2023 |
| graph decode | `VarSketch` refused (263 of 21,814 on BIMcollab) | 0 to 7 records per sample refused |
| Element base | `decode_base` refuses (`Identifier` is `m_id`) | the same, and on 2023 |
| catalogue | fails where a file holds schemas | fails on 2018 to 2020 `rst_basic` and 2019 `rme_*`; reads from 2021 |

Admitting 2015 to 2022 to `extract_records` therefore needs the declared-size rule of 2015 to 2017 in `group_records` and nothing else of the framing. The Element base, the ElementId reads of the readers built on it (`native_json::element_id` and the places that read `/m_id/m_id64`), and the catalogue's field table are the three contracts that need the older shapes, and each is measured against the 2023 copy in the report that admits the releases.

## 10. What this does not claim

- **No Revit export of any of these samples exists here.** The 2023 copy of a project is the oracle for 2015 to 2022, an admitted reader's reading of the same model, as RE-180 and RE-181 used it; the differences in section 7 are the model's edits and the schema's changes, which were not each traced.
- **The 2015 `rst_basic` is a different project** from the other releases', so 2015 `rst_basic` has no copy to compare with here; its graphs decode (14,202 of 14,251) and its framing agrees.
- **The reading of the `VarSketch` records (modifier 4) is not established**, and they stay refused.
- **Revit 2014 and earlier** have no sample project here and are not measured.
- BIMcollab is one Revit 2015 project: it shows the preamble, the framing and the graph decode on a file with Revit's own export, not that its storeys or parameters are right.

## 11. Reproduce

```text
gh workflow run measure.yml --ref main -f ref=7333d9f7a3104ee7150a3085939abb32877f5e5e -f base=none \
  -f samples=true -f sample_years="2015 2023" -f probe_pair_release=2023 -f bimcollab=true \
  -f models="bimcollab-str-2015 core-interior einhoven" -f scorers=none \
  -f probe=probe_re280_native_framing_pre2023 -f probe_scorer=-
gh run download <run id>   # measure-head/sample-*/probe.txt, probe-pair.txt; bimcollab-str-2015/counts.txt
```

For the MEP and `rst_advanced` projects of 2015 against 2019, add `-f sample_years="2015 2019" -f probe_pair_release=2019` and `-f models=einhoven`. The runs above were dispatched from the pull request's branch because `tools/ci/scrub-holdout-results.sh` kept the probe's `native framing:` lines for BIMcollab only from that branch; on `main` use `--ref main`.
