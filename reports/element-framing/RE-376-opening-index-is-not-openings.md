# RE-376 — The 2024 "ArcWallRectOpening index" names system elements, not wall openings

**Date:** 2026-11-08
**Issues:** #89 (RE-15's opening index), #32 (RE-19's Door/Window negative); the `opening_index_to_ifc_openingelement_host_join` claim the B49 audit (`docs/archive/BACKLOG.md`) left "checked against what it asserts, not changed by default"; follows RE-15, RE-19, RE-321 and RE-338
**Probe:** `probes/examples/probe_re376_opening_index_ids.rs`
**Result:** negative for the carrier. The 60-byte rows `rect_opening_index` finds on Revit 2024 are not wall openings, so there is no host to join them to. Every id they name is a system element, none is an element that cuts a wall, and the rectangular wall openings of every other 2024 file measured give no row. The rows are no longer merged into `iter_elements`, and the export no longer claims the join as a missing feature.

## 0. The question

The export lists `opening_index_to_ifc_openingelement_host_join` among `unsupported_features` on every file, whatever it holds. It names the rows RE-15 decoded as a Revit 2024 `ArcWallRectOpening` index (`src/research/rect_opening_index.rs`): rows merged into `iter_elements` as class `ArcWallRectOpening`, never written to the IFC, "until host Wall ElementIds join". RE-15 took a row's `related_id_a` as the host candidate. Which element does each row name, and is it a wall and its opening?

## 1. What was measured

For each Revit 2024 file, the probe:

- decodes every row `ArcWallRectOpeningIndex::find_all_for_revit_version` finds in every partition, as the export's recover did;
- reads the native record (`native_document::extract_graphs`) of every id a row names and counts the pairs of classes of `related_id_a` and `related_id_b`;
- counts the native records of the classes `ArcWallRectOpening`, `SWallRectOpening` and `VWallRectOpening`;
- reads every wall's (`SWall`, `ArcWall`) cut steps (`native_wall_cuts::wall_cut_steps`, RE-321), the steps that name the element cutting it, and counts by step kind how many of those elements any row names.

Files: the six Revit 2024 projects held locally. `2024_Core_Interior.rvt` (magnetar, the CI project), Autodesk's `rac_basic_sample_project` and `rst_basic_sample_project` 2024, and the BIM4LCA office as built and at building permit and housing terrain (as `research/bim4lca-measure.tsv` pins them). Measured locally, with `cargo run --profile ci -p rvt-probes --example probe_re376_opening_index_ids -- <file>`.

## 2. The rows name system elements

Only Core Interior gives rows: 4,917 rows, in `Partitions/46`, `51` and `59`, naming 648 distinct ids in 324 distinct pairs. The export merged the 3,777 rows of `Partitions/46`, the largest partition (the recover stopped after the first partition with rows).

| class of `related_id_a` | class of `related_id_b` | rows |
|---|---|---:|
| `ConceptualConstructionType` | `ConceptualConstructionType` | 2,000 |
| `ConceptualConstructionType` | `ConceptualSurfaceType` | 224 |
| `ConceptualSurfaceType` | `ConceptualSurfaceType` | 830 |
| `ConceptualSurfaceType` | `ListConceptualSurfaceTypes` | 83 |
| `FillPatternElem` | `FillPatternElem` | 254 |
| `FillPatternElem` | `LinePatternElem` | 88 |
| `GStyleElem` | `GStyleElem` | 1,076 |
| `LinePatternElem` | `LinePatternElem` | 202 |
| `MaterialElem` | `GStyleElem` | 5 |
| `MaterialElem` | `MaterialElem` | 155 |
| **all** | | **4,917** |

Every id has a native record and none is a wall, a door, a window or an opening. The ids are small (the most repeated pairs are 54/55 and 56/57, 224 rows each), `related_id_b` is `related_id_a + 1` in all but a handful of rows (RE-19's 3,164 of 3,167), and `m_index` counts 0, 1, 2 … within each run of rows and restarts. That is the shape of lists of consecutive system ids, not of one record per opening; RE-19 already saw that the pairs repeat up to 131 times.

## 3. The elements that cut walls are named by no row

| 2024 file | rows | walls | walls with cut steps | elements the steps name, by step kind (classes) | of them named by a row |
|---|---:|---:|---:|---|---:|
| Core Interior | 4,917 | 1,209 | 129 | cutout 192 (`FamilyInstance`) | 0 |
| `rac_basic` | 0 | 60 | 13 | cutout 26 (`FamilyInstance`); embed 5 (`SWall`); rectangular opening 2 (`SWallRectOpening`) | 0 |
| `rst_basic` | 0 | 9 | 0 | none | 0 |
| office as built | 0 | 1,084 | 371 | cutout 814 (`FamilyInstance`); embed 112 (`SWall`); rectangular opening 87 (85 `SWallRectOpening`, 2 `ArcWallRectOpening`); shadow cutout 2 | 0 |
| office at permit | 0 | 1,139 | 408 | cutout 839 (`FamilyInstance`); embed 125 (`SWall`); rectangular opening 80 (78 `SWallRectOpening`, 2 `ArcWallRectOpening`); shadow cutout 1 | 0 |
| housing terrain | 0 | 18 | 0 | none | 0 |

The native records agree: Core Interior holds no `ArcWallRectOpening`, `SWallRectOpening` or `VWallRectOpening` record at all, while the two office files each hold two real `ArcWallRectOpening` records (3024607 and 3028398, rectangular openings in curved walls) and 85 and 78 `SWallRectOpening`s. The finder gives no row on either office file. It finds rows on the one file that has no rectangular wall opening, and none on the files that have them.

**Re-run on CI.** Measure run [42601800171](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42601800171) runs the probe on 15 Revit 2024 files from CI, at `44fa3fde`: five of the six above (not the housing terrain file, which Measure does not read) and ten more: the office's two terrain variants at building permit, Autodesk's `rac_advanced`, `rme_advanced` and `rme_basic`, and five Snowdon Towers samples (Architectural, Electrical, HVAC, Plumbing and Structural). It reproduces the figures of sections 2 and 3 for the five files they share and widens them. Core Interior alone gives rows: 4,917, naming 648 ids in 324 pairs, with the ten class pairs at section 2's counts. The other 14 files give none, among them Snowdon Towers' architectural sample, whose 1,248 walls (238 with cut steps) hold 11 `SWallRectOpening` records. Over the 15 files the walls' cut steps name 3,089 cutter ids (summed over step kinds) and the files hold 181 native `ArcWallRectOpening`, `SWallRectOpening` and `VWallRectOpening` records, and a row names none of them.

To reproduce it from a landed commit that holds the same probe and rows code: `gh workflow run measure.yml --ref main -f ref=a672e371f3b39b4345d68a1e0f701992aa5c5e32 -f base=none -f nordic=true -f samples=true -f sample_years="2024 2024" -f models="core-interior nordic-office-* sample-2024-racbasic* sample-2024-rstbasic*" -f scorers=none -f probe=probe_re376_opening_index_ids`, which runs the 15 files above. Run [42606540256](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42606540256) ran it on `a672e371` without the `models` filter: its probe output for the 15 files is the same, byte for byte, and the five other files it adds (Einhoven, Revit 2023, and the four RE1 files, Revit 2025) end `exit 1` with `no opening index`.

## 4. Why RE-15 named the rows

A row is found by its first two bytes, `a7 01`, followed by two zero bytes and the marker `0x40088204` at `+0x10`. `0x01a7` is the class tag the 2024 schema gives `ArcWallRectOpening` (`tests/re15_geometry_invariants.rs`), and that is the only thing tying the rows to the class: it is a two-byte value that the same 60-byte structure carries on Core Interior for lists of system ids. Nothing else in a row is an opening's (RE-15 F89b already found no width or height in it, RE-19 no door or window discriminator).

## 5. What changes

- `partition_schema_mvp` no longer merges the rows into `iter_elements` (`PartitionSchemaMvp::rect_openings` is gone). The decoder stays in `research::rect_opening_index`, with this finding in its documentation, for the RE-15 and RE-19 tests and the fuzz targets.
- The export no longer lists `opening_index_to_ifc_openingelement_host_join`. Real wall openings are written from the walls' own records (RE-321, RE-338), and the ones left out are counted in `exported.wall_cut_openings_left_out`. The `opening_index_without_host_geometry` gap bucket, which nothing filled after the rows stopped being written, is removed, as are the never-called `opening_index_property_set`, the `ArcWallRectOpening` category mapping and the viewer's `ArcWallRectOpening` class count.
- On Core Interior, `decoded.production_class_counts` loses `ArcWallRectOpening` 3,777, `production_walker_elements` goes 4,833 to 1,056, `source_coverage.exported_element_fraction` 0.20 to 0.92, `decoded_element_fraction` 0.18 to 0.04 (now counting elements alone against the declared ids) and `mean_element_confidence` 0.72 to 0.81. No other diagnostic moves there, and on every other file measured only the claim goes. The IFC of every file measured is unchanged (section 6).

## 6. Exports

`rvt-ifc` at `main` (a0d44755) and at this change, on the files of section 3 and the CI and reference projects (Core Interior, Einhoven, RE1 Architecture, Mechanical, Electrical and Plumbing) and Autodesk's `rac_basic` of 2019, 2023, 2025, 2026 and 2027 and `rst_basic` of 2019, 2023 and 2027 (19 exports): every IFC is the same but for the two timestamps (`FILE_NAME`, `IfcOwnerHistory.CreationDate`). The witness observations of `magnetar-2024-core-interior` and `-slim` differ by the one claim alone.

## 7. Not claimed

- What the rows are. They are lists of consecutive system ids in some 60-byte structure; which class owns that structure is not read here.
- Anything about releases other than 2024. The finder is gated to 2024, and the openings of other releases are not this carrier's.
- That nothing else in Revit 2024 holds an opening index. Only that this one is not one.
