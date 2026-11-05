# RE-342: the saved-graphics reader on Revit 2015 to 2022, and what a Revit 2020 family of imported geometry needs of it

**Date:** 2026-11-05
**Issues:** #1158 (the native readers on Revit 2015 to 2022), #1190 (W2, W6 and W7 on 2015 to 2023); follows RE-280 and RE-281 (framing, parameters and phases on 2015 to 2022), RE-300 (a `GPolyMesh` with normal binding 2 met only inside a light source's filter) and RE-329 (what keeps a saved mesh from decoding whole). Discussion [#112](https://github.com/DrunkOnJava/rvt-rs/discussions/112), STE1200's "Set of findings".
**Artefacts:** Autodesk's sample projects of 2015 to 2027 and its family corpus, as `research/autodesk-sample-projects.tsv` and Measure's `families` pin them (fetched for a run, never stored); Einhoven, the reference, BIM4LCA and Revit 2025 models; the held-out models, as counts only. The contributor's family is not one of them (section 2).
**Runs:** Measure [42015502520](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42015502520) (2015 to 2023 census and pairs), [42015507053](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42015507053) (2024 to 2027 census), [42015511974](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42015511974) and [42015516241](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42015516241) (exports against `main`, default and `--saved-meshes`) and [42015541431](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42015541431) (held-out census, counts), all at `ab9503b1`; [42013683146](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42013683146) (the same pairs on `main`'s readers, probe at `5a814865`), [42013687044](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42013687044) (the pairs with all four of the patch's changes, `ecc302d2`), [42013885483](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42013885483) (the offices with `FacetedTopology8` read, `6b0f6726`) and [42011570902](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42011570902) (the family corpus, `2e7a9c8e`). The source the runs at `ab9503b1` measured is the source of this report's commit: it adds the report, the changelog fragment and the addendum only.
**Probe:** `probes/examples/probe_re342_saved_graphics_pre2023.rs`.
**Status:** positive for one change, not measurable for three. Reading an `InstanceInfo` that has no `m_GRepId` as the default representation draws 5,459 to 5,756 of the 6,241 to 6,433 `FamilyInstance` elements of each release's three sample projects, where `main`'s readers draw 68 to 70, and the meshes equal the same projects' 2023 copies' on 11,482 of 11,482 elements of the 2021 and 2022 saves. The other three changes the patch makes (deferred history episodes, `FacetedTopology9` with normal binding 2, a set `m_bHasScale`) are met by no file Measure fetches, or by none that they draw, and are not in this change. Nothing in any export moves, and saved graphics stay a research reading on Revit 2015 to 2022.

## 1. What was sent

STE1200 (Steffen, SLIK Architekten) posted on 2026-10-29 that four small changes let the saved-graphics reader read a Revit 2020 family whose bodies are imported geometry only: no sketch, no DirectShape, 36 `ImportInstance` elements on 29 `MasterImportSymbol` elements with no CAD source, so that the geometry exists only in the saved graphics. On 2026-11-02 the maintainer asked whether the patch was offered under Apache 2.0 and whether the family's terms allowed an automated fetch. On 2026-11-03 STE1200 answered that the patch and the example are offered under Apache 2.0, and that the family's terms (BIMobject's, as the contributor reads them: no passing on to third parties, no access outside the Bim.com interface) do not allow it; the maintainer removed the link and said the family would not enter the repository or CI. At 14:57 UTC on 2026-11-03 STE1200 attached a zip with only the patch, moved to `a98bd316`, and a per-instance JSON, both under Apache 2.0.

The zip (sha256 `1fdbb1f0…c6e1` as downloaded on 2026-11-05) holds `rvt-rs_a98bd316_revit2020_saved_graphics.patch` (15,958 bytes) and `vectors_revit2020_import_instances.json` (19,836 bytes). It was read as data and nothing in it was run. The patch applies to `main` at `ce0b2398` with offsets of 72 and 263 lines and no conflict (`git apply --check`).

## 2. The family, and what is not in this repository

The family is a manufacturer's lift family that a configurator generates, saved by Revit 2020. It is not downloaded, read, stored or named here beyond the contributor's public figures; the thread has its name, hash and source. What this report and its PR carry of it are counts the contributor published in the thread: 2,204 element records in one partition, 17 groups, 36 `ImportInstance` elements on 29 `MasterImportSymbol` elements, 325 `GPolyMesh` objects with 47,968 facets and, placed, 48,520 triangles, 109 deferred history episodes, and 36 `GInstance` objects with `m_bHasScale` set. The per-instance JSON is not committed: it names the family's symbols and holds its geometry, and nothing CI can fetch reads it.

## 3. Provenance and licence

The contributor wrote on 2026-08-21 that the work is "clean-room black-box work on publicly available files — no decompilation, no NDA material", and on 2026-08-30 that language models, given matching IFC files, did the decoding. The patch reads as that says: every class and field it names (`m_oEpisodes`, `Episode`, `FacetedTopology9`, `m_normalsFlag`, `m_edgeVisFlagsArr`, `InstanceInfo`, `m_GRepId`, `m_bHasScale`) is a name in the file's own `Formats/Latest` schema (CLEANROOM.md, accepted source 1), its comments cite only measurements on the one family, and the independent reader the contributor checked against was written from this repository's `native_segments.rs`. Nothing in it points to a forbidden source. This report did not check the contributor's statements about how the file was obtained or decoded beyond reading the patch for such a reference, and found none.

## 4. The patch, change by change

Against `main` at `ce0b2398`.

| patch change | on `main` | here |
|---|---|---|
| the release gate widened to 2015 to 2027, ids of 4 bytes up to 2023 (the 29 October patch's first change) | landed: `NATIVE_RELEASES` and `group_records` (RE-280, RE-281) | nothing to take |
| `Options::releases` left at 2023 to 2027; only the example names `NATIVE_RELEASES` | the default is 2023 to 2027 (RE-281); `saved_meshes::default_scope` is 2023 to 2027 | unchanged: no export reaches a pre-2023 file's saved graphics |
| `native_graphics_traversal`: an `InstanceInfo` with no `m_GRepId` is the default representation | refused: `m_GRepId` must be 0 | **taken** (`default_representation`), measured in section 5.2 |
| the same: `m_bHasScale` set is read when `m_GRepId` is absent, the whole 3x3 applied | refused | not taken. `f58d4710` read it narrowed, with a rigid 3x3 only; `ab9503b1` removes it. 3 elements of each of 7 pairs pass the check and none is drawn (section 5.3) |
| `native_saved_mesh`: topology classes 0, 1, 8 and 9, normal binding 0 or 2 | topology 0 and binding 0 only | not taken. `9d8b9dc6` read 0 and 9 and `6b0f6726` added 8; `59ce1cb6` and `bc9cf037` revert them. No file draws a body from either (sections 5.1 and 5.3) |
| `native_index::creation_episodes`: `m_oEpisodes` holds deferred `Episode` pointers, their bodies follow the root, the counted section repeats their GUIDs | refused (`inline history episodes require a different contract`) | not taken. `ca48b95a` reads it; `b486a9ba` reverts it. No file has one (section 5.1) |
| `examples/saved_scene_dump.rs` (OBJ in millimetres, one status line per element) | not on `main` | not taken: what it measures is the probe's counts; an OBJ dump of a Revit 2020 file is not a measurement |

The branch `research/1158-ste1200-all` (`e08acf30`) holds all four changes, the narrowed `m_bHasScale` reading and `FacetedTopology8`, with the probe, so that the contributor can run the probe on the family at it and anyone can restore them: `git revert ab9503b1 b486a9ba 59ce1cb6 bc9cf037` on this change does it.

The `m_bHasScale` change is where the narrowing mattered. On Revit 2023 to 2027 a set flag is a scale that is not read, and the 2024 office has 10 sanitary terminals refused for it (RE-329 §1). The contributor reports that on the one 2020 family the flag is set on all 36 instances and that all 36 matrices are rotations, and says what the flag means on 2020 is open. The patch applies the whole 3x3 whatever it holds; `f58d4710` read the flag only with a matrix that scales nothing. Section 5.1 has what the flag is on in files that can be fetched.

## 5. Measured

The probe prints counts only, so that it can run on a held-out model. `main`'s readers with `Options::releases` widened to `NATIVE_RELEASES` are the "before"; every sample project of 2015 to 2022 is read next to the same project saved by Revit 2023.

### 5.1 What each change meets

Run 42015502520 (34 project files of 2015 to 2023, Einhoven among them) and 42015507053 (59 files of 2023 to 2027: the sample projects, Core Interior, RE1, the BIM4LCA office and its terrain variants, the Revit 2025 models), both at `ab9503b1`; the family corpus from 42011570902.

| what the patch changes | what a file needs | files that have it |
|---|---|---|
| `InstanceInfo` with no `m_GRepId` | Revit 2015 to 2022 | all 30 project files of 2015 to 2022: 81,964 `InstanceInfo` objects, none with the field. The 45 files of 2023 to 2027 that have any: 220,686 objects, all with `m_GRepId` 0, none other |
| deferred `Episode` pointers in `m_oEpisodes` | the family of section 2 | 0 of the 93 files above and 0 of the 13 Autodesk and maker families (12 of 2016 to 2027 fail the preamble earlier, at `Global/DocumentIncrementTable`: `invalid increment episode range`, as RE-281 §7 found; the 2014 one is outside the releases) |
| `FacetedTopology1` or `9` | | 0 objects |
| `FacetedTopology0` or `8` with `m_normalsFlag` 2 | | `0`: 185 objects in 28 files, all 185 in a graph that also holds a `GImposter`, which the traversal leaves out (RE-300). `8`: 336 objects in 31 files, 307 in such a graph and 29 outside, in the office as built (16) and at permit (13) |
| a stored normal against the facet's winding normal, on those | | `8`: 183,078 of 183,078 facets within 0.1 degree. `0`: 239,650 of 244,268; the other 4,618 are in two Revit 2024 files (2,210 between 0.1 and 1 degree, 2,360 over 1 degree, 48 degenerate) |
| `GInstance.m_bHasScale` set | | 69 `GInstance` objects in 15 files of 2015 to 2022 (identity, mirror or uniform scale matrices: 9 in each save of 2016 to 2022 and in the 2023 copies, 6 in 2015), 11,839 in 14 files of 2023 to 2027 |

So the change that reads `InstanceInfo` is met everywhere in Revit 2015 to 2022, and every other change is met by no file or by objects the traversal never reaches. The 29 objects of `FacetedTopology8` that lie outside a light source's filter were read in 42013885483 (`6b0f6726`, both offices with `--saved-meshes` and `mesh_bodies_vs_ifc`, `saved_meshes_vs_ifc` and `box_bodies_vs_ifc`): the exports are byte for byte `main`'s and the scorer diff is empty, so the elements they belong to stay boxes for other causes (RE-329 §1 has two sanitary terminals held by `unsupported faceted topology` and a plane edge together). The stored normals of `FacetedTopology0` in two Revit 2024 files are not the winding normals, which is a reason not to take a binding-2 normal as it is stored.

### 5.2 The sample projects against their 2023 copies

A `FamilyInstance`, `ImportInstance` or `DirectShape` element is meshed as `saved_meshes::attach` meshes a body (detail level 3, `native_saved_scene`), and counted drawn whole when its status is `decoded_saved_graphics` with at least one primitive. Two elements are equal when the triangle count is the same, the world box differs by at most 0.0001 ft and the surface area by at most 0.0001 of the 2023 copy's. Run 42015502520 for the readings with the change; 42013683146 for `main`'s readers.

| release | `FamilyInstance` elements, 3 projects | drawn whole, `main` | drawn whole, this change | whole in both files | same triangles | equal | not equal |
|---|---:|---:|---:|---:|---:|---:|---:|
| 2015 | 6,241 | 68 | 5,459 | 5,302 | 5,302 | 4,626 | 676 |
| 2016 | 6,433 | 70 | 5,732 | 5,703 | 5,652 | 5,061 | 642 |
| 2017 | 6,433 | 70 | 5,732 | 5,703 | 5,696 | 5,109 | 594 |
| 2018 | 6,433 | 70 | 5,732 | 5,703 | 5,696 | 5,109 | 594 |
| 2019 | 6,433 | 70 | 5,756 | 5,728 | 5,728 | 5,150 | 578 |
| 2020 | 6,433 | 70 | 5,756 | 5,728 | 5,728 | 5,150 | 578 |
| 2021 | 6,421 | 70 | 5,741 | 5,741 | 5,741 | 5,741 | 0 |
| 2022 | 6,421 | 70 | 5,741 | 5,741 | 5,741 | 5,741 | 0 |

`main` stops 380 of the 455 `FamilyInstance` elements of the 2020 `rac_basic` at `external symbol has unsupported representation/document context` and 11 more at `unsupported component representation/document context`; with the change none stops there, 313 are drawn whole, the 2023 copy draws the same 313, and all 313 are equal with a world box and surface area that differ by exactly 0. The stop reasons that remain are the mesher's, the same in the 2020 file and its copy (disconnected UV boundaries, ellipse profiles, `SerializedDummy` and the rest of RE-329's list).

- **The 2021 and 2022 projects** (6 pairs): 11,482 of 11,482 elements drawn whole by both are equal, every one exactly.
- **`rac_basic` 2016 to 2022 and `rst_basic` 2019 to 2022:** all equal (311 of 311 or 313 of 313 and 403 of 403 in each).
- **The 3,662 not equal** are 3,465 in the `rac_advanced` saves of 2015 to 2020 (577 or 578 in each, and the same count of both files' elements whole in both), 99 in the `rst_basic` saves of 2016 to 2018 (65, 17, 17) and 98 in the 2015 `rac_basic`. None differs by more than 1 ft in its world box (3,581 by more than 0.01 ft and up to 1 ft, 69 by up to 0.01 ft, 12 by no more than 0.0001 ft); the triangle counts differ on 65 `rst_basic` elements. They are not traced to a cause. The `rac_advanced` set is the same size in every save up to 2020 and zero in 2021 and 2022, which read through the same `InstanceInfo` shape; RE-281 §7 found the 2015 to 2020 saves of `rac_advanced` and `rst_basic` differ from their 2023 copies in thousands of fields, which a model edited between the saves would do. This report does not show the edit.
- **The 2015 `rst_basic`** is another file under the same name (RE-280 §7): none of its elements is in both.
- **Elements drawn whole by one file only:** 95 by the 2023 copy (2 in each `rac_basic` of 2015 to 2018, 21 or 22 in each `rac_advanced` of 2015 to 2018), 7 by the older file (2015).

### 5.3 What the three changes left out do to these readings

Run 42013687044 at `ecc302d2` read the same pairs with all four of the patch's changes (the `m_bHasScale` clause narrowed to a rigid matrix, `FacetedTopology0` and `9` with normal binding 2, the deferred episodes). Its table, row for row and file for file, equals run 42015502520's at `ab9503b1`, which has only the `InstanceInfo` change (51,248 elements, 45,649 drawn whole, 45,349 in both, 41,687 equal): the three changes alter not one count. In 42013687044 the narrowed `m_bHasScale` clause let 3 elements of each of 7 pairs past the scale check (`main` refuses them as scaled) and none of them is drawn whole, since the tables are equal; 1 element of each of 8 pairs, with a uniform scale matrix, stayed refused. The clause draws no body in any file here.

### 5.4 Exports

No export moves. Run 42015511974 (`ab9503b1` against `main` at `f8612274`, `-f nordic=true -f r2025=true -f samples=true -f scorers=none`): the `ifc.sha256` of all 92 exports is the same on both sides, the sample projects of every release 2015 to 2027 (6, 3, 3, 3, 6, 3, 3, 3, 3, 10, 5, 5 and 5), Einhoven, Core Interior, the four RE1 models, the office as built and at permit, their two terrain variants and the Revit 2025 models. The same run with `--saved-meshes`, 42015516241: 92 of 92 identical. The flag moves the export of 40 of the 92 files, all of Revit 2023 to 2027 (the two runs at the same head against each other), so the second run exercises 40; of the 52 others, the 30 files of 2015 to 2022 are stopped before the change by `Options::default()` and `saved_meshes::default_scope`, and 22 files of 2023 to 2027 are not moved by the flag. A file of 2023 to 2027 has `m_GRepId` in every `InstanceInfo` (220,686 of 220,686), so the changed branch is not reached on it either. Both runs are without scorers (`scorers=none`): no export moved, so the unfiltered run with every scorer and the held-out models was not made.

### 5.5 Held-out models

18 held-out models were read by the probe (run 42015541431, counts only, nothing scored): 7 of Revit 2018 to 2022 with 8,101 `InstanceInfo` objects and no `m_GRepId`, 11 of 2023 to 2027 with 2,308 and all 0. None has deferred episode pointers (0 of 18), none a `FacetedTopology1` or `9`; 10 have `FacetedTopology8` with binding 2 (144 objects, 29 in a graph with a `GImposter`); `m_bHasScale` is set on 1,592 `GInstance` objects of 3 of the pre-2023 models and 283 of one later model. No held-out export was compared or scored: the change can reach no export (section 5.4), so a comparison would exercise nothing. The counts above are what those files hold.

## 6. What the contributor's JSON can and cannot stand for

Checked in the JSON alone (36 instances): its totals are the sums of its rows; the 29 distinct symbols hold 325 meshes and 47,968 triangles, and the 36 placed instances 347 meshes and 48,520 triangles, which are the two figures the comment gives (`325 GPolyMesh objects, 48,520 triangles` is a count of objects and a count of placed triangles; the `47,968 facets` of the normals check is the first count of triangles); the union of the 36 boxes is the box it states; all 36 matrices have determinant 1 and orthonormal rows to 0.0 (26 are the identity, 10 other rotations); `has_scale` is set on all 36. It is a consistent record of one file.

It stands for what the contributor offers it as: the numbers a second reader should get from that file, so that anyone who obtains the family under its terms can compare the reader's output with them. The probe at `research/1158-ste1200-all` prints the counts that correspond to the figures of section 2 (`history deferred episode pointers`, `topology FacetedTopology9 normals_flag 2 objects` and `facets`, the `ginstance m_bHasScale true` rows, `scene owners drawn whole` and `scene triangles of whole owners`). It cannot stand in CI: nothing Measure fetches reads it, so a check against it would run on no file. It is not a Revit export of the family, which does not exist here, and the agreement of the contributor's two readers is the agreement of two readings of the same bytes, not of Revit's geometry. The manufacturer's parameter values and the box they fit are a check on extents. For those reasons no vector is committed.

## 7. Not claimed

- Anything about the family itself: it is not read here. The figures in section 2 are the contributor's, and the three changes the family needs are not in this change, so this change does not read it: its history is refused before its graphics are reached.
- That the saved-graphics reader is admitted to Revit 2015 to 2022: `Options::releases` and `saved_meshes::default_scope` are unchanged, no export reads a pre-2023 file's saved graphics, and no Revit export of a 2015 to 2022 file has been compared with these meshes. The nearest oracle is the IFC2X3 export of `rac_advanced` (RE-330, `-f racadv=true`), which scores a body only once `saved_meshes::attach` admits the release.
- That the 3,662 elements that differ from their 2023 copies are model edits, or that the 2023 copy is Revit's geometry: it is this reader's reading of a later save.
- That `m_bHasScale` means nothing on the older schema: it is set on instances with a uniform scale matrix, an identity and a mirror in the same files, and what it adds to the matrix is not read.
- The contributor's note that `m_continuationBits`' count rises from 100 to 113 on his 2020 family, where RE-280 §4 found it 1 from Revit 2018 on the sample projects: not measured here. The framing does not depend on it.
- The decoding of any element class other than `FamilyInstance`, `ImportInstance` and `DirectShape`: walls, floors and the rest of the 2015 to 2022 projects are not meshed here.

## 8. Reproduce

```text
gh workflow run measure.yml --ref main -f ref=ab9503b14d87d2b2d0c8fb8a016fd093fd0282de -f base=none \
  -f samples=true -f sample_years="2015 2023" -f probe_pair_release=2023 -f models=einhoven -f scorers=none \
  -f probe=probe_re342_saved_graphics_pre2023 -f probe_scorer=-
gh workflow run measure.yml --ref main -f ref=ab9503b14d87d2b2d0c8fb8a016fd093fd0282de -f base=none \
  -f samples=true -f sample_years="2024 2027" -f nordic=true -f r2025=true -f scorers=none \
  -f probe=probe_re342_saved_graphics_pre2023 -f probe_scorer=-
gh workflow run measure.yml --ref main -f ref=ab9503b14d87d2b2d0c8fb8a016fd093fd0282de -f base=main \
  -f samples=true -f nordic=true -f r2025=true -f scorers=none [-f ifc_flags=--saved-meshes]
gh run download <run id>   # measure-head/sample-*/probe.txt and probe-pair.txt
```

For `main`'s readers ("before"), `-f ref=5a8148654f8a4e3675cecac64bbe5828244a8099`, the probe on `main`. The held-out census needs `--ref research/1158-saved-graphics-pre2023` (or a branch with `tools/ci/scrub-holdout-results.sh` as this change has it) and `-f holdout=true -f models="holdout-*"`. On a file of your own: `cargo run --profile ci -p rvt-probes --example probe_re342_saved_graphics_pre2023 -- FILE.rvt`, or `OLDER.rvt NEWER.rvt` for a pair.
