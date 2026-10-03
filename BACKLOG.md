# Backlog

Working backlog for maintainer sessions. The public plan is the GitHub
milestones and issues; this file ranks what a session can do now, with the
evidence that put each item here. Statuses: `open`, `committed`,
`in progress`, `done`, `blocked`.

## Session settings (2026-10-03)

- Subagents: off
- Tests: mirror tests banned; existing suites (c): mirror suites are deleted
  area by area, each after the E2E or integration coverage of its area
  passes; keep suites stay (B23).
- Autonomy: the whole committed backlog, with a checkpoint at each milestone.
- Red/green: a new test is pushed alone to a scratch branch with a draft PR.
  CI shows it red, and the commit then goes onto the item branch with the red
  output in its message (same tree, no force push). The scratch branch is
  deleted afterwards.
- Compute: every build, test and measurement runs on GitHub's hosted runners
  (CI on the pull request, `measure.yml` for the reference models and the
  Autodesk corpora). Nothing is compiled or measured on the maintainer's Mac.
- Unit tests are banned in this repository (`tools/ci/test-targets.txt`).
  Verification is a `real` target (real Revit files from the corpora CI
  fetches), a `cli` target, a `contract` target, a Measure run, or the viewer's
  Playwright suite.

## Phase 0 findings (2026-10-03, on `main` at `c5f7e43`)

- **Git.** `main` is clean and green. The only unmerged remote branch is
  `pr/255` (rosejn's PR #255, `DIRTY`, labelled `community`, `needs-split`).
- **Tracker.** `tools/maintainer/audit.py`: 12 of 12 checks ok. There are 28
  open issues, and the only open PR is #255.
- **Community.** Thirteen comments since the last replies (2026-10-01, 6:30 PM
  UTC) have no answer yet: puzzbobb on #548 (2), #223, #152 (3), #154 (2),
  #309, #355 and #421, and STE1200 on #152 (2). Each carries measured claims and
  some carry probes or a patch. These are the main source of the items below.
- **Markers.** Two in source: `src/writer.rs:25` (it names a
  `TODO-BLINDSIDE.md` that does not exist) and `src/elements/structural.rs:210`
  (curved-beam length).
- **Build, lint, types.** Green on `main` at CI's last push run. Not re-run
  locally (no compute on the Mac).
- **Integration and E2E coverage.**
  - 21 `real`, 6 `cli` and 7 `contract` targets run in CI (`tools/ci/verify-real-files.sh`) against the phi-ag family corpus and the magnetar project corpus (Einhoven 2023, Core Interior 2024).
  - The viewer's Playwright suite (3 specs, 999 lines) runs on PRs that touch `src/**` or `viewer/**` (`deploy-viewer.yml`). It is not a required check.
  - The Python wheel gets an end-to-end smoke on a tier-1 file and a 2024 family (`ci.yml:681`).
  - CI takes about 5 to 6 minutes, and Measure about 4 to 5. Both are fast enough to iterate on, so no speed-up item.
  - No flaky test is known; Bugbot never runs (usage limit).
- **Unit suites.** None is run in CI, so none blocks a merge.
  - 1,205 inline `#[test]`s in 99 `src` files (about 27,700 lines of test modules).
  - 14 `unit`-class `tests/*.rs` targets (4,371 lines).
  - 3 pytest files (594 lines, "unit-level and not run", `ci.yml:684`).
  - Sampled estimate: about 55% mirror the implementation (hand-built byte buffers shaped to the parser's own layout, asserting internal fields, as in `walker.rs`'s test module). About 45% test pure logic from outside (vector math in `ifc/measure.rs`, bounds, parsers on adversarial input, proptest).
- **Half-finished.** Native record extraction (`native_document::extract_records`) admits Revit 2023, 2024 and 2027 only. The 2016 to 2022 native path is latent work that puzzbobb's #421 comment maps out (B06).

## Ranked items

Rank is payoff over cost, and items that unblock others rank higher. Size: S
under an hour, M one to three hours, L a session, XL several sessions (split
into milestones).

| # | id | title | lane | size | status |
|---|---|---|---|---|---|
| 1 | B02 | ElemTable records from `0x06`: drop the tail frame on 40-byte files, keep the first record, and expose its owner | fix | M | done |
| 2 | B03 | `schema_registry` reads Revit 2018's `SiteSurface.m_facets` trailing class reference | fix | S | done |
| 3 | B04 | Single-copy elements routed to another partition are dropped as historical (Core Interior, 2024) | fix/research | M | done |
| 4 | B05 | #548: does `route_episode` pick the newer copy? Measure on the Autodesk samples and Core Interior | research | M | done |
| 5 | B10 | #154: the Q4 word is the grandparent's tag; the GUID list starts at 2014 | research | S | in review |
| 6 | B11 | #152: the Stertil 2014 family as a fetchable, pinned corpus file; tick the 2014 and 2026 items | corpus/research | M | in review |
| 7 | B12 | #421 §3: a record's family document is the `ContentDocuments` section whose GUID keys its block | research | M | in review |
| 8 | B06 | Native path on 2016 to 2022: continuation bits `flags & 3`, pre-2018 size accounting, id width per group | feature | M | in review |
| 9 | B01 | Answer the 13 open community comments with measured replies, each as its item lands | community | M | in progress |
| 10 | B17 | `src/writer.rs:25` names a `TODO-BLINDSIDE.md` that does not exist | debt | S | done |
| 11 | B18 | `src/elements/structural.rs:210`: curved-beam length | debt | S | done |
| 12 | B19 | `TODO.md` (1,310 lines): mark items done or superseded by issues | docs | M | in review |
| 13 | B23 | Unit-suite audit (keep or mirror), then delete mirror suites area by area | tests | L | in progress |
| 14 | B09 | #355: family-instance materials from `FamilySymbol.m_geomTag2MaterialId`, instance material parameters, render type 4 | feature | L | committed |
| 15 | B20 | Geometry slices of milestone 0.4.x, starting with #227 | feature | XL | committed |
| 16 | B08 | #309: door and window types with an empty material map are bare `IfcOpeningElement`s | feature | M | blocked |
| 17 | B07 | #223: the flag word at `+0x46` is not Room Bounding | research | S | open |
| 18 | B13 | #421 §2: six curtain system type records on 2021 `rac_advanced` that ElemTable does not list | research | S | open |
| 19 | B21 | Parameters (0.5.0: #35, #155, #223, #228) | feature | XL | open |
| 20 | B22 | Held-out validation on licensed models (#408) | test | L | blocked |
| 21 | B24 | `ClassEntry::tag` is the base's tag (RE-146), but the schema-directed walker, `SchemaTable::tagged_ancestor` and `rvt-analyze` use it as the class's own | fix | M | open |


## Progress log

Each item lists its PR, its red run on `main` (the test commit's message has the full output), its green run, and its Measure evidence.

- **B02, done** (#559, merged).
  - Red: `tests/elem_table_frame.rs` on draft #558, run 37115866869.
    - `2016 family: ids rise strictly, but record 1595 has id 18 after 7259 (of 1596)`
    - `Revit_IFC5_Einhoven.rvt: parse_records returns the 2615 records the table states, left: 2614, right: 2615`
  - Green: every required check, plus corpus tier 2.
  - Measure against `main` (run 37117234625, families and samples): all 6 IFCs are byte-identical, and every scorer, rvt-info and diagnostics output is identical on 6 models, 11 families and 36 samples.
  - An earlier run (37116833078) caught one change: RE1 Architecture's curtain wall type took a `Type Mark`. The value-block fix in the same PR restored the old output.
  - Five existing tests were updated with the maintainer's approval, in their own commit.
- **B03, done** (#561, merged).
  - Red: `tests/schema_registry_catalogs.rs` on draft #560, run 37116554570: `["2018: schema name byte budget at 351466: 786452"]`.
  - Green: every check on #561.
- **B04, done** (#565, merged).
  - Red: `tests/native_current_records.rs` on draft #564 (corpus tier 2): `Revit_IFC5_Einhoven.rvt: 6 declared elements have no emitted record: [2846, 5973, 5974, 5976, 5978, 5979]`.
  - Green: corpus tier 2 and the required checks.
  - Measure (run 37117901879, samples): all outputs are identical to `main`.
- **B05, done.** RE-145 (#569) answers #548:
  - the route takes the latest copy on 1,518 of 1,522 differing ids;
  - it sends 16 elements (Einhoven 6, Core Interior 10) to partitions without their records, which B04 fixed.

  Runs 37116666890 and 37117317288.
- **B10, in review** (#563). RE-146: `ClassEntry::tag` is the base's tag and `ancestor_tag` the grandparent's, on all 53 files (run 37116713027). This opened B24.
- **B11, in review** (#568). RE-147: the Stertil 2014 family is pinned in `research/public-families.tsv`, and the ElemTable invariants hold on 54 files of 2014 to 2027 (run 37117029420).
- **B12, in review** (#570). RE-148: block keys are in `Global/ContentDocuments` (163, 52 and 121 on every sample of 2019 to 2027), and the `ContentMarker` count rule holds on all 3,520 blocks; RE1 Electrical's 60 keys are the exception (run 37118135955).
- **B06, in review** (#571).
  - Red: `tests/native_partitions_walk.rs` on draft #566: 2016 and 2017 stop on `SignatureMarker` (2018's schema stop was fixed by #561).
  - Measure run 37118576160.
- **B17, B18, done** (#562, merged). These are comment-only changes, verified by CI's `cargo doc` and `cargo clippy`.
- **B19, in review** (#567). `TODO.md` is marked as the archived decomposition. All 60 items name their issues: 59 closed, and M4-07 (#35) open.
- **B23, in progress.** Areas, each deleted only with its covering targets green:
  - `elem_table`, done in #559: 336 lines;
  - partition and element-record decoders, this PR: 4,443 lines.
- **B01, in progress.** Replies are posted as the items above merge.

## Item details

### B02 ElemTable records from `0x06`

- **Evidence.**
  - `src/elem_table.rs:486` frames records from `0x1E`, and each frame carries the ids of the record after the one it opens in.
  - On 40-byte files the last frame is read from the 24-byte tail: `(0, 0)` on 2024 to 2027 `rac_basic`, `(18, 0)` on the 2026 family. This is puzzbobb, #152, issuecomment-5939876282 and 5951500722.
  - The first record's owner goes nowhere.
  - On 2008 to 2012, `detect_layout` takes an 84-byte stride (no such file in our corpora).
  - Filtering id 0 would be wrong: Core Interior's `AllProjectPhases` is element 0 (STE1200 and puzzbobb, #152).
- **Payoff.** Declared ids and ownership feed the element index, RE-41 second ids and the #548 copy question. A wrong last record is a silent false id on every 2024+ file.
- **Definition of done.** A `real` test in `tests/elem_table_corpus.rs` reads every family and project file of the CI corpora and asserts the table's own invariants from the spec:
  - the records number the count at `0x02`;
  - ids rise strictly;
  - the first record is the lowest id;
  - every set owner is a declared id;
  - no id is declared that no record holds.

  It fails on `main` and passes after the fix. The Measure diff against `main` shows no regression.
- **Dependencies.** None.

### B03 `schema_registry` and Revit 2018

- **Evidence.** puzzbobb, #421 issuecomment-5956962887 and #154 issuecomment-5957671410.
  - `schema_registry::parse` stops at byte 351,466 of the 2018 catalog (`schema name byte budget`) on the 2018 family and on 2018 `rac_basic`.
  - `SiteSurface.m_facets` (`0x500d` over `0x100e`) writes its class reference twice. No other member from 2016 to 2027 has that shape.
- **Payoff.** It unblocks the native path on 2018 (B06) and the #154 checklist item "every schema is read to its end".
- **Definition of done.** A `real` test parses every phi-ag family's `Formats/Latest` with `schema_registry::parse` to its end. It fails on 2018 before the fix and passes after. The fix is written fresh from the byte rule; the patch in the comment is not copied (`CLEANROOM.md`).
- **Dependencies.** None.

### B04 Single copies routed away

- **Evidence.** `src/native_document.rs:376` skips a record whose `route_episode` partition is not its own. puzzbobb (#548 issuecomment-5957284268) measures that on 2025 multi-partition files some elements with one chain record are routed to the lowest-numbered partition, which holds none of their records: 1,777 to 8,671 per file. Core Interior (2024, admitted) has several partitions. This is untested, and if it holds those elements are dropped.
- **Payoff.** A possible live loss of elements in `native_document` consumers: saved meshes and saved scene (`src/ifc/saved_meshes.rs:87`, `src/native_saved_scene.rs`).
- **Definition of done.**
  1. A Measure probe on Core Interior and the 2024 and 2027 samples counts `skipped_historical_records` for ids with no other copy.
  2. If that count is not zero, a `real` test asserts that every declared element with exactly one chain record is emitted, and the fix is to fall back to the only copy.
  3. If it is zero, record the measurement in the report and in #548.
- **Dependencies.** None. It informs B05.

### B05 #548: which copy is current

- **Evidence.** RE-143 (the later partition's copy is newer on 2021 `rac_advanced`). puzzbobb: `route_episode` picks the newer copy 887 of 887 there and 56 of 56 on Golden Nugget, the latter not in our corpora.
- **Definition of done.** A probe that routes every repeated id as `native_document` does and compares it with RE-143's dating, run with Measure on `samples=true` (2020 to 2022 `rac_advanced`) and on Core Interior. The result goes in a report, and #548 is closed or kept with the numbers.
- **Dependencies.** B04, which shares the probe.

### B10 #154: the Q4 word

- **Evidence.** puzzbobb, #154 issuecomment-5951329667:
  - `ancestor_tag` is the grandparent's tag, 1,488 of 1,488;
  - `ClassEntry::tag` is the base's tag, 6,192 of 6,192;
  - files from 2008 to 2013 have no GUID list.
- **Definition of done.** A probe run on the phi-ag families (2016 to 2026) and the samples reproduces or refutes each count. #154's checklist is reconciled. The 2008 to 2013 part is not reproducible here: no such file is in our corpora.
- **Dependencies.** None.

### B11 #152: 2014 and 2026 evidence

- **Evidence.** STE1200, #152 issuecomment-5967778938: Stertil's public 2014 family. The zip's sha256 is `6e65aae1…`, and the `.rfa` inside is `b3b46ded…`. Autodesk's 2026 `rac_basic` is already pinned (RE-140).
- **Definition of done.** The family is pinned in a research corpus list with its source URL and hash and fetched by Measure. Then:
  - the ElemTable invariants of B02 hold on it;
  - #152's 2014 and 2026 items are ticked with run links.
- **Dependencies.** B02.

### B01 Community replies

- **Definition of done.** Each of the 13 comments has a reply that states only measured claims, with links to the run or report, or says plainly what cannot be reproduced here and why. The trackers (#152, #154, #548, #421, #223, #309, #355) are updated.
- **Dependencies.** B02, B03, B04, B05, B10 and B11 supply the measurements. Replies on B07, B08 and B09 ask for files or note limits.

### B12 #421 §3: family document of a record

- **Evidence.** puzzbobb: block keys are the `Global/ContentDocuments` section GUIDs on every sample project (163 of 163, 52 of 52, 121 of 121). `ContentMarker.m_nElementCount` is the block's channel-101 records less the id −1.
- **Definition of done.** A probe reproduces it on the samples and Core Interior with Measure. A report answers RE-135's open question.
- **Dependencies.** None.

### B06 Native path 2016 to 2022

- **Evidence.** `src/native_segments.rs:135` tests `(4..=7).contains(&flags)`. Before 2018, `m_continuationBits` is `(n << 2) | flags` with `n` ≥ 100. `src/native_document.rs:329` gives 8-byte ids on 2016 to 2022.
- **Definition of done.** `native_segments::walk` reads every partition of the 2016 to 2022 samples, measured with Measure. Admitting those releases into `extract_records` stays out of scope until a Revit export exists to check against.
- **Dependencies.** B03.

### B17, B18, B19 Debt

- **B17.** Replace the dead reference in `src/writer.rs:25` with what it means today.
- **B18.** Keep the curved-beam note and link it to #94. Close the marker when #94's curve work lands.
- **B19.** Walk `TODO.md`. Mark each item done (with the PR), superseded (with the issue), or still open, and link the issue.

### B09 #355: family-instance materials

- **Evidence.** puzzbobb's rule scores 1,939 of 2,058 instances on Snowdon (local-only, not measured here).
- **Definition of done.** Measure on Core Interior (8 family materials) and RE1 shows a gain in element materials against Revit's export, with no regression.
- **Dependencies.** None.

### B08 #309 (blocked)

Only measurable on Snowdon, a local-only model that is not measured (the
maintainer's decision, 2026-09-28). It is blocked until a redistributable model
with a schematic opening type and a Revit export exists. Reply on #309 and ask
for one.

### B07, B13 (notes)

- **B07.** puzzbobb's consecutive-save model is not public. Reply and ask for the files; until then, record the finding as unverified.
- **B13.** Record the six ids in RE-140's report as an open observation.

### B20 Geometry slices (XL)

The milestones are the open issues, each with a Measure baseline against
Revit's export of the CI reference models:

- #227 opening cut from the location curve;
- #90 room solids;
- #94 framing sections;
- #96 MEP route geometry;
- #357 stair flights and #323 aggregates;
- #358 wall thickness;
- #356 sloped roofs (needs an oracle);
- #156 sketch to solid.

Snowdon-only parts (#328, #309, #358's 165 walls) are not measurable here.

### B23 Unit suites: audit, then mirror deletion

- **Evidence.** Phase 0 counts (above). None of these suites runs in CI.
- **Definition of done.**
  - Each suite is tagged keep or mirror in the table below, with a reason.
  - Mirror suites are deleted one area per commit, and the message records the lines removed.
  - An area is deleted only after its `real` or `cli` targets pass on the PR's CI run. The run link goes in the area's row.
  - A suite is tagged keep when it states a property from the format's specification or tests pure, stable logic from outside (geometry math, bounds against hostile input, property tests, fuzz regressions).
- **Audit (2026-10-03).** When in doubt, a suite is tagged mirror: the session's rule is "if unsure whether a test is banned, treat it as banned".

  **Keep.** Pure logic or spec-level behaviour checked from outside:

  | suite | reason |
  |---|---|
  | `src/ifc/measure.rs` | vector maths |
  | `src/ifc/clipping.rs` | plane geometry |
  | `src/ifc/body_geometry.rs` | triangulated areas and volumes |
  | `src/ifc/camera.rs` | view maths |
  | `src/ifc/pbr.rs` | colour conversion |
  | `src/redact.rs` | PII redaction of paths |
  | `src/compression.rs` | RFC 1952 headers, inflate bomb limits |
  | `src/control.rs` | cancellation semantics |
  | `src/project_information.rs` | ZIP entries and the inflate limit on hostile input |
  | `src/round_trip.rs` | encode then decode, byte-exact |
  | `src/elem_table.rs` (1 test) | duplicate ids keep the first |
  | `src/revit_global_ids.rs` (1 test) | Revit's own GlobalIds for two Snowdon walls |
  | `tests/fuzz_regressions.rs` | regression inputs from fuzzing |
  | `tests/proptest_parsers.rs` | property tests |
  | `tests/graceful_degradation.rs` | corrupted inputs fail closed |

  **Mirror.** These build synthetic byte buffers in a decoder's own layout, assert on internal structs or private helpers, or pin exact STEP/glTF/JSON text from hand-built models (change detectors). Deleted area by area, each with the covering targets named:

  | area | suites | covering targets | status |
  |---|---|---|---|
  | `elem_table` | `src/elem_table.rs` (16 of 17 tests) | `elem_table_frame`, `elem_table_corpus` | done in #559, 336 lines |
  | partition and element-record decoders | `src/partition_*.rs`, `src/partitions.rs`, `src/rect_opening_index.rs`, `src/arc_wall_record.rs`, `src/compound_framing.rs`, `src/object_graph.rs`, `src/element_record_*.rs`, `src/revit_global_ids.rs` (2 of 3), `src/transmission_data.rs` | `partition_record_chain`, `element_records_2025`, `arc_wall_corpus`, `iter_elements_typed`, `design_options`, `revit_global_ids`, `rooms_floors_from_records_only`, `re15_geometry_invariants`, `re19_door_window_wall_negative`, `level_names`, `element_names`, `curtain_walls`, `ifc_export_overrides`, `partition_scanner`, `project_count_fixtures`, Measure | this PR, 4,443 lines |
  | typed elements | `src/elements/*.rs` | `iter_elements_typed`, `element_names`, `level_names`, `curtain_walls`, `rooms_floors_from_records_only`, Measure | next |
  | schema, walker, metadata | `src/walker.rs`, `src/formats.rs`, `src/class_index.rs`, `src/class_tag_map.rs`, `src/es_refs.rs`, `src/metadata.rs`, `src/basic_file_info.rs`, `src/part_atom.rs`, `src/reader.rs`, `src/parse_mode.rs` | `samples`, `field_type_coverage`, `json_schema_contracts`, `rvt_info_cli`, `rvt_inspect_cli`, `rvt_dump_cli`, `schema_registry_catalogs` | open |
  | IFC, glTF, schedule writers | `src/ifc/` except the keeps, `src/geometry/`, `src/level_bind.rs`, `src/relations.rs` | `walker_to_ifc_integration`, `ifc_roundtrip`, `ifc_export_overrides`, `rvt_ifc_diagnostics_cli`, `rvt_schedule_cli`, the IfcOpenShell job, Measure | open |
  | CLIs, writer, small modules | `src/bin/*.rs`, `src/writer.rs`, `src/capability.rs`, `src/cli.rs`, `src/corpus.rs`, `src/evidence.rs`, `src/identity.rs` | the six `cli` targets, `cfb_roundtrip_delta`, `binary_inventory` | open |
  | synthetic-fixture targets | `tests/` unit class except the keeps: `cfb_patch_corpus`, `checksum_page_framing`, `control_cancellation`, `corpus_tier1_health`, `es_remap_golden`, `gen_fixture_roundtrip`, `geometry_recovery`, `ifc_export_modes`, `ifc_synthetic_project`, `ifc_synthetic_structural`, `typed_decoders` | as above per subject | open |
  | Python | `tests/python/` (pytest, unit-level, not run) | the wheel smoke on real files (`tools/ci/wheel-smoke.py`) | open |

### B21 Parameters (XL)

#35 and #155 (wire formats and units), #223 (flag word) and #228 (reference
list). Milestone 0.5.0.

### B22 Held-out validation (blocked)

#408 needs licensed models not used to develop the decoder. It is blocked on
obtaining them.
