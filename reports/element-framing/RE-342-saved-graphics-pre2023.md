# RE-342: the saved-graphics reader on Revit 2015 to 2022, and what a Revit 2020 family of imported geometry needs of it

**Date:** 2026-11-05
**Issues:** #1158 (the native readers on Revit 2015 to 2022), #1190 (W2, W6 and W7 on 2015 to 2023); follows RE-280 and RE-281 (framing, parameters and phases on 2015 to 2022), RE-300 (a `GPolyMesh` with normal binding 2 met only inside a light source's filter) and RE-329 (what keeps a saved mesh from decoding whole). Discussion [#112](https://github.com/DrunkOnJava/rvt-rs/discussions/112), STE1200's "Set of findings".
**Artefacts:** Autodesk's sample projects of 2015 to 2027 and the family corpus, as `research/autodesk-sample-projects.tsv` and Measure's `families` pin them (fetched for a run, never stored); the reference, BIM4LCA and Revit 2025 models. The contributor's family is not one of them (section 2).
**Runs:** none yet measured on this head.
**Probe:** `probes/examples/probe_re342_saved_graphics_pre2023.rs`.
**Status:** in progress. Sections 1 to 3 are read from the thread, the patch and the contributor's vectors; the measured sections are marked where they are not yet.

## 1. What was sent

STE1200 (Steffen, SLIK Architekten) posted on 2026-10-29 that four small changes let the saved-graphics reader read a Revit 2020 family whose bodies are imported geometry only: no sketch, no DirectShape, 36 `ImportInstance` elements on 29 `MasterImportSymbol` elements with no CAD source, so that the geometry exists only in the saved graphics. On 2026-11-02 the maintainer asked whether the patch was offered under Apache 2.0 and whether the family's terms allowed an automated fetch. On 2026-11-03 STE1200 answered that the patch and the example are offered under Apache 2.0, and that the family's terms (BIMobject's, as the contributor reads them: no passing on to third parties, no access outside the Bim.com interface) do not allow it; the maintainer removed the link and said the family would not enter the repository or CI. At 14:57 UTC on 2026-11-03 STE1200 attached a zip with only the patch, moved to `a98bd316`, and a per-instance JSON, both under Apache 2.0.

The zip (sha256 `1fdbb1f0…c6e1` as downloaded on 2026-11-05) holds `rvt-rs_a98bd316_revit2020_saved_graphics.patch` (15,958 bytes) and `vectors_revit2020_import_instances.json` (19,836 bytes). It was read as data and nothing in it was run. It applies to `main` at `ce0b2398` with offsets of 72 and 263 lines and no conflict (`git apply --check`).

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
| `native_index::creation_episodes`: `m_oEpisodes` holds deferred `Episode` pointers, their bodies follow the root, the counted section repeats their GUIDs | refused (`inline history episodes require a different contract`) | commit `ca48b95a`, its own commit so that it can be dropped; measured section 5 (not yet) |
| `native_saved_mesh`: topology classes 0, 1, 8 and 9, normal binding 0 or 2 | topology 0 and binding 0 only | commit `9d8b9dc6`, narrowed to topology 0 and 9: the contributor measured 9 and says 1 and 8 come from the schema only |
| `native_graphics_traversal`: an `InstanceInfo` with no `m_GRepId` is the default representation | refused: `m_GRepId` must be 0 | commit `f58d4710` |
| the same: `m_bHasScale` set is read when `m_GRepId` is absent, the full 3x3 applied | refused | commit `f58d4710`, narrowed: only with a rigid (orthonormal) 3x3, so that a scale stays refused |
| `examples/saved_scene_dump.rs` (OBJ in millimetres, one status line per element) | not on `main` | not taken: what it measures is the probe's counts; an OBJ dump of a Revit 2020 file is not a measurement |

The `m_bHasScale` change is where the narrowing matters. On Revit 2023 to 2027 a set flag is a scale that is not read, and the 2024 office has 10 sanitary terminals refused for it (RE-329 §1). The contributor reports that on the one 2020 family the flag is set on all 36 instances and that all 36 matrices are rotations, and says what the flag means on 2020 is open. The patch applies the whole 3x3 whatever it holds; this reads the flag only with a matrix that scales nothing.

## 5. Measured

Not yet measured on this head. The runs, the counts of what each change meets in a file Measure can fetch, the pairs against the same projects saved by Revit 2023 and the exports' hashes against `main` go here.

## 6. What the contributor's JSON can and cannot stand for

Checked in the JSON alone (36 instances): its totals are the sums of its rows; the 29 distinct symbols hold 325 meshes and 47,968 triangles, and the 36 placed instances 347 meshes and 48,520 triangles, which are the two figures the comment gives (`325 GPolyMesh objects, 48,520 triangles` is a count of objects and a count of placed triangles; the `47,968 facets` of the normals check is the first count of triangles); the union of the 36 boxes is the box it states; all 36 matrices have determinant 1 and orthonormal rows to 0.0 (26 are the identity, 10 other rotations); `has_scale` is set on all 36. It is a consistent record of one file.

It stands for what the contributor offers it as: the numbers a second reader should get from that file, so that anyone who obtains the family under its terms can compare the reader's output with them. It cannot stand in CI: nothing Measure fetches reads it, so a check against it would run on no file. It is not a Revit export of the family, which does not exist here, and the agreement of the contributor's two readers is the agreement of two readings of the same bytes, not of Revit's geometry. The manufacturer's parameter values and the box they fit are a check on extents. For those reasons no vector is committed.

## 7. Not claimed

- Anything about the family itself: it is not read here. The figures in section 2 are the contributor's.
- That the saved-graphics reader is admitted to Revit 2015 to 2022: `Options::releases` and `saved_meshes::default_scope` are unchanged, and no pre-2023 file's export reads its saved graphics.
