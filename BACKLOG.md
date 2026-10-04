# Backlog

Working backlog for maintainer sessions. The public plan is the GitHub
milestones and issues; this file ranks what a session can do now, with the
evidence that put each item here. Statuses: `open`, `committed`,
`in progress`, `done`, `blocked`.

## Session settings (2026-10-04, session 3, from 10:15 AM)

- Subagents: off.
- Tests: mirror tests banned; existing suites (a), left untouched.
- Autonomy: the whole committed backlog, with a checkpoint at each milestone.
- Committed, in order: B49, B48, B51, B59, B62, B61, B47, B44, B63, B66, B54,
  B53, B60, B64, B50, B52, B57. After them, the next-highest open items in the
  same lanes (B45, B46, B56, B55).
- Changes to an existing test or fixture (B49's Core Interior `property_sets`
  rows, any pinned count) wait for the maintainer's approval and go in their own
  commit.
- Red/green, compute and the unit-test ban are as in session 2 below.
- Added during the session: B67 (approved by the maintainer) and B68 (it
  unblocks B59 and B43).
- CI reruns: none. pr-queue's reruns are off (`/queue reruns off`), and while
  a session still runs pr-queue 0.1.2 the queue is paused and branches are
  updated by hand (`gh api -X PUT repos/DrunkOnJava/rvt-rs/pulls/N/update-branch`).

## Session settings (2026-10-03, session 2, from 2:40 PM)

- Subagents: off
- Tests: mirror tests banned; existing suites (a): the remaining suites stay
  untouched (session 1 chose (c) and deleted the mirror suites in B23).
- Autonomy: the whole committed backlog, with a checkpoint at each milestone.
- Committed, in order: B27, B26, B34, B36, B37, B30, B31, B32, B41, B28, B29,
  B35, B38, B39, B40, B33.
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

## Phase 0 findings, session 2 (2026-10-03, 2:30 PM, on `main` at `b798497`)

- **Git.** `main` is green through #605 (CI run on #606's merge in progress). Open PRs: #608 (SerialNumber, armed, in the strict queue) and rosejn's #255, whose work landed as #420 on 2026-09-30 (the contributor said to merge as we see fit) but which is still open. Remote branches: only the two item branches and `re/re157-pipe-inner-diameter`.
- **Local branches.** About 60 local branches track remotes that are gone (merged and deleted), plus `b09-impl-wip`, `fix/b04-current-record-fallback-wip`, `fix/b06-wip`, `red-b21-duct` and five `feat/b2x` leftovers ahead of an old `main`. Local `main` is 63 behind.
- **Tracker.** 25 open issues, 0 unanswered community comments (every thread active since 2026-10-02 ends with a maintainer reply, Discussion #112 included).
- **Markers.** No `TODO`, `FIXME`, `HACK` or `XXX` in `src`.
- **Build, lint, types.** Green on `main`'s last completed CI run (#605). Not run locally.
- **Integration and E2E.** 52 targets in CI: 33 `real`, 6 `cli`, 7 `contract`, 6 `unit` (kept as source, not run). A full CI run takes about 3 minutes (corpus tier 2 170 s, IfcOpenShell 107 s, wheel 186 s); Measure about 8 to 10. Fast enough to iterate on. No flaky test seen this session.
- **Unit suites.** 1,936 lines of inline test modules in 14 `src` files (about 170 tests: vector maths, clipping, colour, compression bounds, redaction, round trip, two CLI parsers), and the 6 `unit` targets (fuzz regressions, proptest, graceful degradation, synthetic projects). None runs in CI or blocks a merge. Sampled: all test pure logic from outside; the mirror suites went in B23 (24,373 lines).
- **Property-set gap on `main`** (`psets_vs_ifc`, before #603, #605 and #608): Architecture 101 missing, Mechanical 45, Plumbing 74, Electrical 66. #603, #605 and #608 close about 30 + 8 × 3 + 25 + 51. What remains: `InvertElevation` (69), the boolean sets (`IsExternal`, `LoadBearing`, `ExtendToStructure`: 36 on Architecture), `Finish` (6), `FireRating` (14), `PitchAngle` (2), `NumberOfPoles` (2), and single Reference gaps on a railing, two openings and a few proxies.
- **Research found this session.** RE-157 (branch `re/re157-pipe-inner-diameter`, Measure run 37142956936): a pipe's own data object holds its inner diameter at byte 549 from the header, followed by its outer diameter, on every pipe size of RE1 Mechanical and Plumbing (11 sizes); the segment's size table holds nominal, inner and outer in a row.

## Phase 0 findings, session 3 (2026-10-04, 10:10 AM, on `main` at `3503a60`)

- **Git.** `main` is clean. Its last full CI run (#633's merge, run 37158263129) passed every job; the two later commits are docs-only. The nightly corpus (37188508028) and fuzz (37192161099) runs this morning passed. No open pull requests. The only unmerged remote branch is `pr/255` (closed in B36; its work landed as #420).
- **Local branches.** 16 local branches track remotes that are gone (session 2's squash-merged item, `re/` and `red/` branches), plus `wip/b33-systems`, whose work landed as #631. The shared checkout's `main` is 85 commits behind `origin/main`.
- **Tracker.** 23 open issues. `tools/maintainer/audit.py`: 11 of 12 checks ok; the Actions cache is at 10.1 GB of its 10 GB limit (B51).
- **Community.** One comment has no answer: puzzbobb on #421 (2026-10-03, 6:13 PM UTC, about 9,800 characters with a probe). It explains RE-148's one exception: RE1 Electrical's 60 block keys missing from `Global/ContentDocuments` belong to three `Bell Wall` families that were deleted and the 57 fire-alarm families nested in them. A block is listed exactly when the `Family` record at the top of its chain is declared (B48).
- **Markers and stubs.** No `TODO`, `FIXME`, `HACK` or `XXX` in `src`, `tools`, `tests`, `examples`, `viewer/src`, `rvt-py/src` or `python`. No `todo!`, `unimplemented!` or `#[ignore]`. Two `#[allow(dead_code)]` functions: `src/formats.rs:807` and `src/ifc/export_content.rs:835`, the latter "retained for when opening→host IFC emission lands", which has landed (B50).
- **Build, lint, types.** Green on `main`'s last CI run (fmt, clippy `-D warnings`, `cargo doc`, the docs.rs preview, MSRV 1.87, three OSes). Not re-run locally (no compute on the Mac).
- **Honesty drift in the export diagnostics** (B49).
  - `unsupported_export_features` (`src/ifc/mod.rs:3630`) claims `revit_element_parameters_to_ifc_property_sets` whenever no element's *own* set is a parameter set. The `Pset_` sets session 2 wrote are `ElementPropertySet` entities and do not count, so Core Interior claims it while writing 2,111 `Pset_` values.
  - `decoded.parameter_value_count` counts walker `AProperty` instances and is 0 on every measured model. That makes the viewer's Parameters row say "Revit's parameter table is not read yet" (`viewer/src/main.ts:2399`).
  - The Core Interior count fixtures pin the claim as the `property_sets` row's `known_gap` (`tests/fixtures/project-counts/2024-core-interior.json:163`, and the slim fixture), so changing it changes an existing test and needs the maintainer's approval.
  - Two other claims are made unconditionally, and each may be narrowly true:
    - `revit_compound_assemblies_and_walltype_widths` (`:3575`) is about layer *widths* (#358 is open; 319 of Core Interior's walls draw one layer);
    - `opening_index_to_ifc_openingelement_host_join` (`:3609`) is about the opening-index rows, whose emission (`src/ifc/export_content.rs:835`) never landed.

    They are checked against what they assert, not changed by default.
- **Integration and E2E coverage.**
  - 58 targets in `tools/ci/test-targets.txt`: 39 `real`, 6 `cli`, 7 `contract` and 6 `unit` (kept as source, not run). Three `real` targets skip 5 synthetic-fixture tests by name (`elem_table_corpus`, `iter_elements_typed`, `partition_scanner`); this is documented in the manifest.
  - CI takes about 7 minutes wall time (corpus tier 2 is the longest job, at 370 s). Measure takes about 8 to 10. Both are fast enough to iterate on, so speeding them up is not an item.
  - The viewer's Playwright suite (3 specs) passed on its last 8 runs (`deploy-viewer.yml`). It covers the journey from opening a file through inspect, schedule and export to diagnostics, on Einhoven and Core Interior.
  - The Python suite (`tests/python`, 67 tests) runs in the wheel job.
  - No flaky test is known. Every CI failure since 2026-10-02 was a deliberate red run or a fmt, clippy or doc fix made inside its own PR.
  - **Critical flows with no real-file E2E** (B53). Six of the 19 shipped binaries are run by no test except `--help` (`tests/cli_ergonomics.rs:181`) and by no workflow: `rvt-diff`, `rvt-history`, `rvt-analyze`, `rvt-elem-table`, `rvt-ifc-compare` and `rvt-write`.
- **Unit suites.** None runs in CI, so none blocks a merge.
  - 141 inline `#[test]`s in 14 `src` files (1,936 lines of test modules).
  - The 6 `unit` targets (2,275 lines).
  - Sampled estimate: about 85% test pure logic from outside: vector maths, clipping, compression and inflate limits, redaction, encode-then-decode round trips, property tests, fuzz regressions, corrupted input and cancellation.
  - About 15% are mirror-like: `ifc_synthetic_project` and `ifc_synthetic_structural` (934 lines) pin exact counts and strings of hand-built models, and are kept as the generators of `synthetic-project.ifc` and the viewer's structural demo (#577). The two CLI parser modules in `src/bin` are the rest.
  - The Python suite is an integration suite on real files and is not counted here.
- **Record drift.** B23's table lists `control_cancellation`, `ifc_synthetic_project` and `ifc_synthetic_structural` among the targets #577 deleted. #577 kept them, and its description says so (B57).
- **Docs drift.** `docs/launch/reddit-r-rust.md:20` and `docs/launch/twitter-x-thread.md:84` give MSRV 1.85 (it is 1.87), and the thread says 9 CLIs (19 binaries ship) (B57).
- **Half-finished.** Native record extraction admits Revit 2023, 2024 and 2027. Revit 2025 element records come from the partition record path, so the native phase fields (`m_createdPhaseId`, `m_demolishedPhaseId`) are not read on 2025 (B46).
- **Measured on `main`** (Measure run 37207000836, `3503a60`, no base). Revit's values equal in `psets_vs_ifc`, none different:

  | model | equal | missing, by property |
  |---|---|---|
  | RE1 Architecture | 195 of 253 | `IsExternal` 21 (walls 7, members 10, plates 2, curtain wall 1, railing 1), `FireRating` 14 (B42), `LoadBearing` 9, `ExtendToStructure` 7, `PitchAngle` 2, railing 3 (B45), two openings' `Reference` (B20), one `Pset_QuantityTakeOff.Reference` |
  | RE1 Mechanical | 249 of 298 | `Shape` 31 (B43), `ConnectionType` 2 × 6 (B31), duct-type `Length` 6 |
  | RE1 Plumbing | 444 of 612 | `ConnectionType` 2 × 63 (B31), fitting `NominalDiameter` 42 (B44) |
  | RE1 Electrical | 151 of 153 | `NumberOfPoles` 2 (B32) |

  RE-158 found the walls' booleans need a model where they take both values; they stay with B28's note. The other scorers show these gaps:
  - **Ducts.** RE1 Mechanical's 25 ducts carry a `Name` and `ObjectType` that differ from Revit's, and have no type object (`names_vs_ifc`, `type_objects_vs_ifc`: typed 49 of 74, untyped `IfcFlowSegment` 25) (B59).
  - **Type entities.** 18 type objects are a different IFC entity from Revit's: 5 door types and 7 flow terminals on Architecture, 6 flow terminals on Plumbing (B61).
  - **Type GlobalIds.** 545 of 1,147 typed elements carry a type object whose GlobalId differs from Revit's type: Core Interior 416 of 854 (columns, doors, windows, slabs, shading devices), Plumbing 54 of 123, Mechanical 37 of 49 typed, Architecture 36 of 72, Electrical 2 of 49. System-family types (walls) agree, and element GlobalIds all equal Revit's (B60).
  - **Ports.** Revit writes 53 ports on Electrical, rvt-rs 0. Mechanical is 132 of 150 ports and 66 of 73 connections; Plumbing 242 of 260 and 120 of 126. One Plumbing connection rvt-rs writes is not in Revit's export (B54, B62).
  - **Spaces.** 39 of RE1 Architecture's 73 elements are on the right storey, but Revit contains them in an `IfcSpace` (B63).
  - **Materials.** RE1 Architecture's 10 mullions, 2 curtain panels, a proxy and the railing get no materials (58 of 72) (B66). On Core Interior, 6 windows get a material set that differs from Revit's (836 of 854 equal).
  - **Door host.** One of RE1 Architecture's 5 doors has no host wall in rvt-rs's file (B64).
  - **Exact.** Wall bodies, wall ends, room names, storeys, element GlobalIds and opening hosts are exact on every model.

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
| 5 | B10 | #154: the Q4 word is the grandparent's tag; the GUID list starts at 2014 | research | S | done |
| 6 | B11 | #152: the Stertil 2014 family as a fetchable, pinned corpus file; tick the 2014 and 2026 items | corpus/research | M | done |
| 7 | B12 | #421 §3: a record's family document is the `ContentDocuments` section whose GUID keys its block | research | M | done |
| 8 | B06 | Native path on 2016 to 2022: continuation bits `flags & 3`, pre-2018 size accounting, id width per group | feature | M | done |
| 9 | B01 | Answer the 13 open community comments with measured replies, each as its item lands | community | M | done |
| 10 | B17 | `src/writer.rs:25` names a `TODO-BLINDSIDE.md` that does not exist | debt | S | done |
| 11 | B18 | `src/elements/structural.rs:210`: curved-beam length | debt | S | done |
| 12 | B19 | `TODO.md` (1,310 lines): mark items done or superseded by issues | docs | M | done |
| 13 | B23 | Unit-suite audit (keep or mirror), then delete mirror suites area by area | tests | L | done |
| 14 | B09 | #355: family-instance materials from `FamilySymbol.m_geomTag2MaterialId`, instance material parameters, render type 4 | feature | L | done |
| 15 | B20 | Geometry slices of milestone 0.4.x, starting with #227 | feature | XL | in progress |
| 16 | B08 | #309: door and window types with an empty material map are bare `IfcOpeningElement`s | feature | M | blocked |
| 17 | B07 | #223: the flag word at `+0x46` is not Room Bounding | research | S | done (reply, unverified) |
| 18 | B13 | #421 §2: six curtain system type records on 2021 `rac_advanced` that ElemTable does not list | research | S | done (reply) |
| 19 | B21 | Parameters (0.5.0: #35, #155, #223, #228) | feature | XL | in progress |
| 20 | B22 | Held-out validation on licensed models (#408) | test | L | blocked |
| 21 | B24 | `ClassEntry::tag` is the base's tag (RE-146), but the schema-directed walker, `SchemaTable::tagged_ancestor` and `rvt-analyze` use it as the class's own | fix | M | done (measured, no change) |
| 22 | B25 | Run the Python API integration suite (`tests/python`, 64 tests) in the wheel job | tooling | S | done |

Session 2 (2026-10-03, afternoon):

| # | id | title | lane | size | payoff | evidence | depends | status |
|---|---|---|---|---|---|---|---|---|
| 1 | B26 | `InvertElevation` from the pipe's inner diameter (RE-157 report, then the field and the property) | feature | M | 69 values (Plumbing 63, Mechanical 6) | RE-157 run 37142956936 | - | done |
| 2 | B27 | Run the slim witness verdict even when the full one drifts, so a pinned-count change regenerates both observations in one cycle | tooling | S | halves the approval round trips of every count change | `ci.yml:495`, #603 | - | done |
| 3 | B28 | `IsExternal`, `LoadBearing`, `ExtendToStructure` from the wall and slab types' and instances' flags (probe, then sets) | research, feature | M | 36 values on Architecture | `psets_vs_ifc` on run 37138175399 | - | done |
| 4 | B41 | `Pset_CoveringCommon.Finish`: test whether it is the covering's layer materials joined with `;` | research, feature | S | 6 values | RE-156 §3 | - | done |
| 5 | B30 | `psets_vs_ifc` also scores enumerated values (`Shape`, `ConnectionType`) | tooling | S | shows the gaps the scorer cannot see | `tools/re/psets_vs_ifc.py:87` | - | done |
| 6 | B31 | `Pset_DuctConnection` and `Pset_PipeConnection` `ConnectionType` on pipes | feature | S | 2 × 69 enumerated values | Revit's RE1 exports | B30 | blocked (B47) |
| 7 | B29 | Single gaps: one Plumbing element's Reference, three Electrical proxies' Reference, a railing's sets, two openings' Reference | research | M | about 14 values | `psets_vs_ifc` | - | done |
| 8 | B32 | `NumberOfPoles`: where an integer project-parameter value is stored (parameter 987675) | research | S | 2 values, and the integer layout for #155 | RE-156 table | - | blocked |
| 9 | B40 | Every project and shared parameter, not only Serial Number: definitions table, typed values, Pset names by match | feature | L | generalises B21 (#85, #155) | RE-156 | B32 | blocked (a model) |
| 10 | B33 | #528 what is left: measure ports and port connections against Revit after RE-141, then `IfcSystem` per system | feature | L | 73 + 126 connections, 26 systems | #528 | - | done |
| 11 | B38 | RE1 Plumbing 442378: why rvt-rs writes an element Revit's export lacks | research | M | one wrong element, maybe a class of them | RE-35 §, `tests/element_records_2025.rs:165` | - | blocked (B46) |
| 12 | B39 | #228 remainder: the leading 3 of a reference list | research | M | closes #228 | RE-155 | - | done (narrowed; #228 open on ElementId 3's class) |
| 13 | B34 | #501: `probe_elem_table_ownership` takes its files from the command line | fix | S | the RE-152 probe runs anywhere | #501 | - | done |
| 14 | B35 | #502: Python `part_atom_json_strict` and `schema_json_strict` | feature | S | closes a documented API gap | #502, `docs/python.md` | - | done |
| 15 | B36 | Close #255 with a pointer to #420 | community | S | tracker hygiene | #255, #420 | - | done |
| 16 | B37 | Prune local branches whose remotes are gone; inspect the 9 leftovers before deleting any | debt | S | local hygiene | `git branch -vv` | - | done |
| 17 | B44 | `Pset_PipeFittingTypeCommon.NominalDiameter` on fittings: a fitting's nominal size from its family | research | M | 42 values on Plumbing | B30's Measure run 37144839798 | - | open |
| 18 | B45 | Railing 462556: resolve its type, for its Reference and `Pset_RailingCommon.Height` | research | M | 3 values on Architecture | B29 | - | open |
| 19 | B46 | Revit 2025 native element fields (`m_createdPhaseId`, `m_demolishedPhaseId`), for phase filtering | feature | L | unblocks B38 and #328 | RE-160 | - | open |
| 20 | B47 | The connection-type element pipes route to (`Generic`) | research | M | unblocks B31, 2 × 69 values | RE-159 | - | open |
| - | B42 | `FireRating`: needs a model whose fire rating is longer than one character | research | S | 14 values | RE-154 | a model | blocked |
| - | B43 | `Pset_DuctSegmentTypeCommon.Shape`: needs the round versus rectangular switch | feature | S | 25 values | RE-134 §5 | a model | blocked |

Session 3 (2026-10-04). Evidence runs: Measure 37207000836 on `main` at `3503a60`. Carried items keep their ids and rows above; they are ranked again here.

| # | id | title | lane | size | payoff | evidence | depends | status |
|---|---|---|---|---|---|---|---|---|
| 1 | B49 | Export diagnostics count the Revit parameters the export writes: the parameters claim and `parameter_value_count` see the `Pset_` sets | hardening | M | every diagnostics reader (viewer status and element panels, `rvt-inspect`, `rvt-ifc --diagnostics`) on every model; Core Interior claims no parameters while writing 2,111 `Pset_` values | `src/ifc/mod.rs:3630`, `:3679`; `viewer/src/main.ts:2399`; Measure core-interior `diagnostics.json:31`, `:257` | approval to change the Core Interior fixtures' `property_sets` rows | done (#638) |
| 2 | B48 | #421: reproduce puzzbobb's `ContentDocuments` rule with a fresh probe, RE-163, reply | community, research | M | the one unanswered contributor comment; settles which embedded family documents a reader may report | #421 comment of 2026-10-03, 6:13 PM UTC; `src/native_document.rs:412` | - | done (#642) |
| 3 | B51 | Actions cache at 10.1 GB of 10 GB: clear stale caches | ops | S | the audit's one failing check; cache eviction slows every CI run | `tools/maintainer/audit.py` | - | done |
| 4 | B59 | RE1 Mechanical's 25 ducts: Revit's `Name`, `ObjectType` and a duct type object | fix | M | 25 elements named and typed as Revit's | `names_vs_ifc`, `type_objects_vs_ifc` | - | blocked (B68, B43: a duct's system family follows its shape) |
| 5 | B62 | RE1 Plumbing: the one port connection rvt-rs writes that Revit's export lacks | fix | S | removes a false connection | `ports_vs_ifc` | - | blocked (B38: its one extra connection is to tank 442378, which Revit does not export) |
| 6 | B61 | Type objects: 5 door types and 13 flow-terminal types are a different IFC entity from Revit's | fix | M | 18 types | `type_objects_vs_ifc` | - | done (#641: a scorer fix, not the exporter) |
| 7 | B47 | The connection-type element pipes route to (`Generic`) | research | M | unblocks B31, 2 × 69 values | RE-159 | - | in progress (RE-164: Generic is held by the pipe fitting types a pipe type routes through; needs B68's samples) |
| 8 | B44 | `Pset_PipeFittingTypeCommon.NominalDiameter`: a fitting's nominal size | research | M | 42 values on Plumbing | `psets_vs_ifc` | - | done (#651) |
| 9 | B63 | Elements Revit contains in an `IfcSpace` | feature | M | 39 elements on RE1 Architecture | `storeys_vs_ifc` | - | done (#645, #656) |
| 10 | B66 | Curtain wall mullion and panel materials | research, feature | M | 12 elements' materials (10 members, 2 plates) | `element_materials_vs_ifc` | - | done (#657) |
| 11 | B54 | #528: the 65 ports with nothing connected (Electrical 53, Plumbing 8, Mechanical 4) and the 13 connections not read (Mechanical 7, Plumbing 6) | research, feature | L | 65 ports, 13 connections | `ports_vs_ifc` | B62 shares the probe | committed |
| 12 | B53 | Real-file E2E for the six CLIs only `--help` checks: `rvt-elem-table`, `rvt-history`, `rvt-analyze`, `rvt-diff`, `rvt-ifc-compare`, `rvt-write` | hardening | L | six shipped tools gain their first real-file check | `tests/cli_ergonomics.rs:181`; no workflow runs them | - | done (#653) |
| 13 | B60 | Type GlobalIds of family types: 545 of 1,147 typed elements' types differ from Revit's | research | L | the type GlobalIds IFC consumers key on, on every model | `type_objects_vs_ifc` | - | in progress (RE-167: a type takes the GlobalId of its instance's original symbol) |
| 14 | B64 | RE1 Architecture: one door with no host wall in rvt-rs's file | fix | S | one door's opening and body | `door_bodies_vs_ifc` | - | in progress (#659: a curtain wall's door is its part; the maintainer approved the curtain_walls test change) |
| 15 | B50 | Dead code: `scan_fields_until_next_class` (unused), and `opening_index_property_set` (kept for an emission that never landed) | debt | S | less dead code | `src/formats.rs:807`, `src/ifc/export_content.rs:835` | B49's check of the opening-index claim | done (#647) |
| 16 | B52 | Local branches: 16 whose remotes are gone, and `wip/b33-systems` | debt | S | local hygiene | `git branch -vv` | - | done |
| 17 | B57 | Docs drift: the launch drafts' MSRV and CLI count, B23's record of #577 | docs | S | public text matches the code | `docs/launch/reddit-r-rust.md:20`, `docs/launch/twitter-x-thread.md:84`; #577 | - | done (#646) |
| 18 | B45 | Railing 462556: resolve its type, for its `Reference`, `Pset_RailingCommon.Height` and material | research | M | 4 values on Architecture | B29, `type_objects_vs_ifc` (untyped `IfcRailing`) | - | open |
| 19 | B46 | Revit 2025 native element fields (`m_createdPhaseId`, `m_demolishedPhaseId`), for phase filtering | feature | L | unblocks B38 and #328 | RE-160 | - | open |
| 20 | B56 | #228: which internal element ElementId 3 is | research | M | closes #228 | RE-161 | - | open |
| 21 | B55 | #227's 63rd opening: wall 55840's edited elevation profile | research, feature | L | the last of Revit's 63 unfilled openings | RE-151 | - | open |
| - | B67 | `tools/ci/validate-real-ifc.py:214` requires a no-material marker the exporter never emits | tooling | S | removes a false failure waiting for the first project without materials | `src/ifc/mod.rs:3577` | the maintainer's approval (given) | done (#639) |
| - | B68 | Pin Autodesk's MEP sample projects (`rme_basic`, `rme_advanced`, 2024 to 2027) for Measure | corpus | S | ducts of every shape, fittings, open connectors and systems: the evidence B43, B59, B44, B47 and B54 lack | `revit.downloads.autodesk.com` (all 8 answer 200) | - | blocked (B70: rvt-ifc took 33.5 minutes per MEP sample) |
| - | B69 | `rvt-write` exits 0 on a patch whose re-encoded stream spans a checksummed page, which no rvt-rs reader can decode | fix | S | a writer that never writes an unreadable stream | B53's test: an identity patch of each family's `Formats/Latest` reads back as "unexpected end of file"; `src/writer.rs` | blocks B53 | done (#653) |
| - | B70 | `rvt-ifc` takes 33.5 minutes on each Autodesk MEP sample (5 s on the architecture ones): two scans search every stream once per id | perf | M | MEP models export in about a minute; unblocks B68 | timing run 37216027753: `attach_mep_systems` 1,007 s and `attach_pipe_type_names` 24 s per walk | blocks B68 | in progress |
| - | B71 | `rvt-ifc` walks the file twice: once to export, again for the diagnostics' class counts (`production_walker_stats`) | perf | S | halves every export's walk | timing run 37216027753: two walks of 1,037 s and 971 s | - | open |


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
- **B10, done** (#563, merged). RE-146: `ClassEntry::tag` is the base's tag and `ancestor_tag` the grandparent's, on all 53 files (run 37116713027). This opened B24.
- **B11, done** (#568, merged). RE-147: the Stertil 2014 family is pinned in `research/public-families.tsv`, and the ElemTable invariants hold on 54 files of 2014 to 2027 (run 37117029420).
- **B12, done** (#570, merged). RE-148: block keys are in `Global/ContentDocuments` (163, 52 and 121 on every sample of 2019 to 2027), and the `ContentMarker` count rule holds on all 3,520 blocks; RE1 Electrical's 60 keys are the exception (run 37118135955).
- **B06, done** (#571, merged).
  - Red: `tests/native_partitions_walk.rs` on draft #566: 2016 and 2017 stop on `SignatureMarker` (2018's schema stop was fixed by #561).
  - Measure run 37118576160: all outputs identical to `main`.
- **B17, B18, done** (#562, merged). These are comment-only changes, verified by CI's `cargo doc` and `cargo clippy`.
- **B19, done** (#567, merged). `TODO.md` is marked as the archived decomposition. All 60 items name their issues: 59 closed, and M4-07 (#35) open.
- **B01, done.** All 13 comments are answered:
  - puzzbobb: #152, #154 (twice), #548 (twice, then closed), #421, #223, #309, #355;
  - STE1200: #152 and Discussion #112.

  The checklists of #152 and #154 are ticked: the 2014 and 2026 layouts, the first record, and the 3,619-class 2014 schema (reproduced on the Stertil family, run 37120169355).
- **B13, done.** RE-145's walk finds 7 undeclared channel-101 ids on 2021 `rac_advanced` against 1 on every other sample, consistent with puzzbobb's six. Reported on #421.
- **B07, done as a reply.** puzzbobb's consecutive-save model is not public; the reply asked for the files, and the finding is recorded as unverified.
- **B23, done.** Mirror suites deleted, each with its covering targets green on its PR:
  - `elem_table`: 336 lines (#559);
  - partition decoders: 4,443 (#572);
  - typed elements: 4,636 (#573);
  - schema and walker: 3,356 (#574);
  - IFC writers: 8,443 (#575);
  - CLIs and writer: 1,063 (#576);
  - synthetic-fixture targets: 2,096 (#577).

  That is 24,373 lines in all. The Python suite is an integration test over the public API on real files, so it is kept and now runs in CI (B25).
- **B24, done (measured, no change)** (#581). RE-150: the walker's `Global/Latest` scan finds 0 candidates at the production threshold on every measured file, with base tags and with own tags alike, so the base-tag lookup changes no output.
- **B25, done** (#578). `tests/python` (64 tests, plus 1 needing `jsonschema`, now installed) runs on the built wheel in the wheel job against the tier-1 fixtures and the family corpus. First run: 64 passed, 1 skipped.
- **B09, done** (#580, merged).
  - Red on `main` (draft #579): `tests/element_materials.rs` gave 0 of 256 columns Revit's material set.
  - Measure (run 37120595647): element materials with Revit's set went from 580 to 836 on Core Interior, 0 to 47 on RE1 Electrical, 0 to 43 on RE1 Mechanical and 0 to 60 on RE1 Plumbing, with no wrong set added.
  - With the maintainer's approval, the pinned material count went from 86 to 87, and the witness observations were regenerated on the runners.
- **B20, in progress.** Milestone #227, the openings Revit's export leaves unfilled (RE-151):
  - #584: each hole in a floor's or shading device's sketch is an opening voiding it, tagged with the lowest id of the sketch lines on the hole.
    - Red on `main` (draft #583, run 37126870895): `tests/unfilled_openings.rs` found 62 of Revit's 62 slab and shading-device openings missing.
    - Measure (run 37128180603): 62 of Revit's 63 unfilled openings, all within 0.001 ft of Revit's box. Every other model's outputs are identical to `main`.
    - With the maintainer's approval, the Core Interior witness observations were regenerated (surface unchanged, 0 diffs).
  - Left on #227: the 63rd opening, a hole in wall 55840's elevation-profile sketch. Reading it needs that wall's edited profile, which rvt-rs does not read.
  - The other 0.4.x milestones are blocked here:
    - #90, #94, #309, #323, #328, #357 and #358 are measured only on Snowdon, which is local-only;
    - #356 needs a pitched-roof oracle;
    - #156 waits on reported format evidence;
    - #96's open parts need a model with round or oval ducts.
- **B21, in progress.** Milestone #35, parameters, measured against the property sets of Revit's RE1 exports:
  - #588 (RE-153): a room's Floor Finish is the text parameter -1006903, joined to the room through its Adler-32-verified data object (11 of 11 on RE1 Architecture).
  - #589: each room's `Pset_SpaceCommon` (`Reference`, `FloorCovering`).
    - Red: draft #587, run 37131559218.
    - `tests/space_common.rs`.
    - With the maintainer's approval, Core Interior's pinned property sets went from 970 to 1086, and the witness observations were regenerated.
  - #590 (RE-154): every `Pset_*Common.Reference` and `Pset_QuantityTakeOff.Reference` is the type's name, not a stored parameter.
  - #592: those `Reference` sets on every element named `Family:Type`.
    - Red: draft #591, run 37133676030.
    - `tests/common_reference.rs` on all four RE1 models.
    - With the maintainer's approval, the pinned property sets went from 1086 to 2670, and the witness was regenerated.
  - #593: the `psets_vs_ifc` scorer in Measure compares every property Revit writes with rvt-rs's value.
  - #595: a pipe's `Length` in both flow-segment sets.
    - Red: draft #594, run 37136045232.
    - `tests/segment_length.rs`.
  - #597: the scorer compares numbers within a relative 1e-5, so a micrometre of writer rounding is no longer "different".
  - #599: RE-134's curve fields read in code (width, height, type at the curve's connector-manager anchor); each duct gets its type, its `Length` and its `Reference`.
    - Red: draft #598, run 37136951323.
    - `tests/duct_fields.rs`.
  - Measured on `main` at #599's merge (Measure run 37138175399), Revit values equal in `psets_vs_ifc`, none different:

    | model | equal |
    |---|---|
    | RE1 Architecture | 152 of 253 |
    | RE1 Mechanical | 210 of 255 |
    | RE1 Plumbing | 370 of 444 |
    | RE1 Electrical | 87 of 153 |

    Progress posted on #35, #96 and #227.
  - #601 (RE-155, #228): an element record's reference list has no fixed slot for its type or Level; within a category the slot is steady, across categories and releases it moves. No code change: the type comes from the name entries and the Level by value. The leading 3 stays unattributed.
  - #605: a duct's `Length` also in `Pset_DuctSegmentTypeCommon` (25 of 25 RE1 Mechanical ducts).
    - Red: draft #604, run 37140175260.
    - `tests/duct_segment_type_common.rs`.
  - #603: storeys (`Pset_AirSideSystemInformation.Name`, `Pset_ProductRequirements.Name`, `Pset_BuildingStoreyCommon.AboveGround` unknown), the building (`Pset_BuildingCommon`) and spaces (the two `Name` sets).
    - Red: draft #602, run 37139848516 (29 of 29 missing on RE1 Architecture).
    - `tests/spatial_property_sets.rs` on all four RE1 models.
    - With the maintainer's approval, Core Interior's pinned property sets went from 2670 to 2948 (15 × 3 + 1 + 116 × 2), and both witness observations were regenerated.
  - #606 (RE-156): `SerialNumber` is a text entry whose id is the shared parameter `Serial Number`'s ElementId (490488 on Electrical, 447886 on Plumbing), in the element's own data object; the id's own object holds the parameter's group, name and GUID. `Finish` is stored nowhere as text.
  - #608, done: `Pset_ManufacturerOccurrence.SerialNumber`, and `Pset_PrecastConcreteElementGeneral.SerialNumber` on proxies (50 on Electrical, 1 on Plumbing).
    - Red: draft #607, run 37142425779.
    - `tests/serial_number.rs`.
  - Open, each needing evidence first:
    - `FireRating` is on the wall type, but RE1's one-character value meets four parameter ids;
    - `IsExternal`, `LoadBearing`, `ExtendToStructure` and `PitchAngle` are not stored text entries (B28);
    - `InvertElevation` needs the pipe's inner diameter, now found (RE-157, B26);
    - `Finish` is assembled by the exporter, likely from the covering's layer materials (B41).
  - #155 and #223 are untouched.

Session 2. Each green line is the CI corpus job running `tools/ci/verify-real-files.sh`, which runs `cargo test --profile ci --test <target>` for every target in `tools/ci/test-targets.txt`:

- **B26, done** (#613). RE-157 (#611): a pipe's own `RbsPipeCurve` object holds its inner diameter 549 bytes after its header, then its outer diameter (11 sizes on RE1). `InvertElevation` is a horizontal pipe's axis less half its inner diameter, or a vertical pipe's lower end, above its storey.
  - Red: draft #612, run 37143783049 (6 of 6 Mechanical pipes missing).
  - Green, run 37144139467: `test re1_pipe_invert_elevations_are_revits ... ok`, `test result: ok. 1 passed; 0 failed`.
- **B27, done** (#614). The slim witness verdict runs whenever the full one ran.
  - Red: #603's run 37140112338 skipped the slim step after the full one drifted.
  - Green: scratch #610, run 37144054315, with a drift injected; both observations were published by the one run, each with `IFCPROPERTYSET` 2963.
- **B34, done** (#616, closes #501). Red on main, Measure run 37144220896: `open error: ... /workspace/... No such file or directory`. Green, Measure run 37144235760: `header element_count=1411 record_count=26425 parsed=26425`.
- **B35, done** (#617, closes #502). `part_atom_json_strict`.
  - Red: draft #615, run 37144711753, `2 failed, 65 passed` (AttributeError).
  - Green, run 37145055434: `python3 -m pytest tests/python -q -rs` gave `67 passed in 2.83s`.
- **B36, done.** #255 closed with a pointer to #420.
- **B37, done.** 49 local branches whose PRs merged were deleted. With the maintainer's approval, the 14 wip and pre-rebuild leftovers were bundled to `~/Developer/archive/rvt-rs-local-branches-2026-10-03.bundle` (verified, 14 refs), then deleted.
- **B30, done** (#619). psets_vs_ifc scores enumerated and list values. Measure run 37144839798 shows `Shape` (31), `ConnectionType` (2 × 69) and `Pset_PipeFittingTypeCommon.NominalDiameter` (42).
- **B28, done** (#621). RE-158 (#620): a door's and a floor's `IsExternal` is its type's `FUNCTION_PARAM` (-1001006), 5 of 5 doors (both values) and 2 of 2 floors.
  - Red: draft #618, run 37145114521 (7 of 7 missing).
  - Green, run 37146230808: `test re1_door_and_slab_is_external_are_revits ... ok`, `test result: ok. 1 passed; 0 failed`.
  - With the maintainer's approval, both witness observations were regenerated: +132 property values on Core Interior's doors.
- **B41, done** (#623). A ceiling's `Pset_CoveringCommon.Finish` is its finish layers' materials, each followed by `;`.
  - Red: draft #622, run 37145628503 (6 of 6 missing).
  - Green, run 37146317561: `test re1_ceiling_finishes_are_revits ... ok`, `test result: ok. 1 passed; 0 failed`.
  - Bugbot found the finish waited on the instance height check meant for the slab layers; it now comes first (6ca485f).
- **B29, done** (#625). Reference on panel-named equipment (3 Electrical elements), and a pipe's type from its curve object (pipe 444719).
  - Red: draft #624, run 37146624857.
  - Left: railing 462556 (type not resolved) and two Revit-made openings that share their host's Tag (B20).
- **B31, blocked.** RE-159 (Measure run 37145371415): `Generic`, Revit's `ConnectionType`, occurs 1,250 times in Plumbing's main partition but never in a pipe's or its type's own object. It is likely the name of a connection-type element the routing preferences point to. That link needs a probe; writing the constant would be hard-coding.
- **B32, blocked** (RE-158). `NumberOfPoles` is 1 on both RE1 elements and matches no entry. Two definitions share the name `Number of Poles`, so the name lookup drops it (B40 must key definitions by id).
- **B38, blocked.** RE-160 (Measure run 37146818151): no record on any model carries a `PHASE_CREATED` or `PHASE_DEMOLISHED` parameter entry. Phases are the native fields `m_createdPhaseId` and `m_demolishedPhaseId`, which rvt-rs reads only on Revit 2023, 2024 and 2027. 442378 is also not in the partitions' leading record chain.
- **B33, done** (#631). Before it, rvt-rs wrote 132 of Revit's 150 ports and 66 of 73 connections on Mechanical, 242 of 260 and 121 of 126 on Plumbing, none of 53 ports on Electrical, and no `IfcSystem` (Revit: 5, 8 and 13). The ports left are #528's; the systems are now written.
  - RE-162 (#632): each of Revit's 26 systems is one `RbsHvacSystem`, `RbsPipingSystem` or `RbsElectricalSystem` data object, and every member Revit groups holds its id in its own object. HVAC and piping systems are named by the object's first ASCII string, electrical systems by their circuit number (empty on Revit's `<unnamed>` circuit). Measure runs 37148070432, 37148641247, 37148993751.
  - Red: draft #629, run 37148704032 (Mechanical: 5 of 5 systems missing).
  - Green, run 37149305337: `test re1_mep_systems_are_revits ... ok`, `test result: ok. 1 passed; 0 failed` (all 26 systems, same name and members).
- **B39, done** (#630). RE-161: the leading 3 of a reference list is most likely ElementId 3, an internal element every ElemTable declares with no name entry; it is not a workset (no file is workshared) and not a parameter value. Narrowed, not attributed: #228 stays open on which element ElementId 3 is, whose record is outside the leading chain.
- **B40, blocked.** `tools/re/reference_property_sets.py` (#633, Measure run 37149211122): Revit's RE1 exports write no property sets besides `Pset_` ones (0 values on all four models). The project and shared parameters they carry are `SerialNumber` (done) and `NumberOfPoles` (B32). Writing every parameter has no reference values without a model whose export writes Revit's parameter-group sets.

Session 3:

- **B48, done** (#642). RE-163, Measure run 37209219989: a keyed block's key is in `Global/ContentDocuments` exactly when the `Family` at the top of its chain is declared, both ways, on all 42 files (6 reference models, 36 samples). RE1 Electrical's 60 unlisted blocks go up to the undeclared `Family` elements 937132, 950798 and 957600. Reply on #421 (issuecomment-5981148955). No reader change: rvt-rs reads no keyed block.
- **B51, done.** `tools/maintainer/clean_caches.py --delete`: deleted 1,588 caches (2.15 GB) of closed pull requests and deleted branches, 0 failed. `python3 tools/maintainer/audit.py`: `12 of 12 checks ok`.
- **B67, done** (#639, with the maintainer's approval). No red run: no corpus file reaches the branch.
- **B49, in progress** (#638).
  - Red: draft #637. Corpus run 37208793324: every model counted 0 against 35 to 2,111 `Pset_` values in its file, and claimed no parameters. Viewer run 37208793339: the row read "Revit's parameter table is not read yet".
  - Green: CI run 37209439743. `test diagnostics_count_the_revit_parameters_the_export_writes ... ok`, then `test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 19.04s`. The viewer spec passed in the same run.
  - With the maintainer's approval, the Core Interior fixtures' `property_sets` rows name `partial_revit_element_parameters`, and the witness observations and verdicts were regenerated, each in its own commit.
- **B59, blocked.** Revit names RE1 Mechanical's ducts `Rectangular Duct:<type>:<id>`, and a duct's system family follows its shape (Rectangular, Round, Oval Duct). rvt-rs reads the type but not the shape (B43), and RE1 holds rectangular ducts only, so nothing separates the three. Writing "Rectangular Duct" would be hard-coding. B68 brings the MEP samples that hold every shape.
- **B61, in progress** (#641). The RE1 reference exports are IFC2X3 CoordinationView 2.0. There, a door's type is an `IfcDoorStyle` and a sanitary terminal's is the supertype `IfcDistributionElementType`. rvt-rs writes IFC4, where `IfcDoorType` and `IfcSanitaryTerminalType` are right. The 18 disagreements were the scorer comparing across schemas, so the scorer now accepts the IFC4 counterpart or a subtype. Measure run 37209892719 verifies it.
- **pr-queue and the session's rerun rule.** The pr-queue mod reruns a CI job once when Jev calls the failure an infrastructure flake, and this session forbids rerunning a check to green. No run was rerun today (no run attempt above 1). pr-queue 0.1.3 adds `/queue reruns off`, now set. It also fixes a recursion that kept its band from ever drawing. The session that started before the update still runs 0.1.2, so the queue is paused too, and branches are updated by hand until it restarts. A failed check is logged here, never rerun.
- **B49, done** (#638). The approved fixture rows and the regenerated witness went in their own commits.
- **B61, done** (#641). Measure run 37209892719: the type entity agrees on every typed element, Architecture 72 of 72, Mechanical 49 of 49, Plumbing 123 of 123, Electrical 49 of 49, Core Interior 854 of 854.
- **B63, in progress** (#645 landed 38 of 39). A family instance written as furniture, a sanitary terminal or a proxy, hosted by nothing, is contained in the `IfcSpace` whose plan outline holds its location on its storey.
  - Red: draft #644, run 37211033155 (RE1 Architecture, 39 of 60 contained differently).
  - Not green: Corpus health on #645 (run 37212742187), and on `main` since 24ba0d5, fails `space_containment` with 1 of 60: element 375441 is in Revit's space `RE-6` and in rvt-rs's storey. That check is not required, so auto-merge landed #645 on the required checks alone. A clippy `type_complexity` failure was fixed in the PR.
- **B57, done** (#646). `docs/launch/README.md` marks the seven 0.1.2 drafts; B23's row corrected.
- **B50, done** (#647). The unused scanner was removed; `opening_index_property_set` stays, tied to the still-open opening-index host join.
- **B52, done.** 24 local branches deleted, each tied to a merged pull request or a closed red scratch PR whose test lives in its merged item PR. The three local-only branches are bundled in `~/Developer/archive/rvt-rs-local-branches-2026-10-04.bundle` (verified, 3 refs).
- **B62, blocked.** RE1 Plumbing's one extra connection joins pipe 444124 to water closet tank 442378, the element Revit's export does not hold (B38); Revit writes no port 0 on that pipe. It goes when B38 is resolved.
- **B44, in progress** (#651). RE-165 (Measure runs 37211684944, 37212205913): a fitting's own object holds a record per connector (unit direction, origin, radius, a further length, diameter = 2 × radius); the diameters are Revit's NominalDiameter on all 42 Plumbing fittings.
  - Red: draft #649, run 37212982834 (42 of 42 missing).
  - The property is an `IfcPropertyListValue`, through a new `PropertyValue::List`.
  - Not green: Corpus health on #651 (run 37214338189) fails its own test, 42 of 42 (`443813: Revit [0.015], rvt-rs None`). The connector records are right (RE-165's dumps decode to two 0.0492 ft records on elbow 443813), so the value is lost between the scan and the written IFC. #651 is disarmed until it is found.
- **B66, in progress.** RE-166: on RE1 Architecture a mullion's material is its type's (`SysMullionFamSym`) `u64` at byte 135 of the type's data object, 10 of 10. The survey across the samples finds a material there on every `SysMullionFamSym` object of 2024 to 2026 (353 of 353 on each `rac_advanced`, 66 of 66 on `rac_basic`, 1 of 1 on `rst_basic`), and on none of 2027, whose layout moved. The panels' `Glass` is not at that offset in `SysPanelFamSym`; the next survey lists every offset that holds a material.
- **B47, in progress.** RE-164 (Measure run 37211483646): no element is named `Generic`. The text sits in hundreds of unnamed objects; on RE1 Plumbing five pipe fitting types (category -2008049, named `-`) hold it, and pipe type 191045 holds all five ids, most likely its routing preferences. RE1 has `Generic` only, so the rule waits for B68's MEP samples, where connection types vary.
- **B53, in progress.** `tests/cli_real_files.rs` runs the six CLIs on the family corpus and the projects. Red: draft #648 injects one fault per binary. Run 37212719105: `rvt-history` releases out of order `[2018, 2017, 2017]`, `rvt-analyze` 2017 for 2016, `rvt-elem-table` 1595 of 1596, `rvt-diff` no differing stream. Without faults (#650) five pass and `rvt-write` fails on main: B69.
- **B69, in progress (with B53).** Revit stores `Formats/Latest`, `Global/Latest` and the partitions in 65,249-byte pages, each ending in a 353-byte checksum rvt-rs cannot compute. The writer wrote re-encoded streams without trailers and verified them with a decoder that does not strip pages, so `rvt-write` exited 0 on files rvt-rs cannot read back. The writer now refuses a patch whose stored bytes reach a page (`Error::WriteRefused`).
- **Rule slip.** While copying two files between scratch branches I used `git checkout <ref> -- <file>`, which the maintainer's rules forbid. Nothing was lost (fresh branch, and the manifest's only difference was the intended line); later copies use `git show <ref>:<path>`.

- **B63, done** (#656). 375441 is a medicine cabinet recessed into RE-6's wall: its record box's point is 0.078 ft outside RE-6's outline and its insertion point (its transform's origin, Revit's placement exactly) 0.032 ft outside. Across RE1 Architecture's 39 candidates the box point agrees with Revit on 38 and the insertion point puts 6 in the wrong room, so Revit uses neither alone; 375441's plan box overlaps exactly one space, RE-6's. An element whose point is in no space now takes the one space its plan box overlaps. Probe runs 37214796046, 37215316216, 37215827984. Corpus health is green on `main` again.
- **B53, B69, done** (#653). B69 had two causes. A patch reaching a full 65,249-byte stored page is refused (its checksum is not computable). And every patched stream was written as truncated gzip, without the CRC32 and ISIZE trailer that Revit's own streams carry and `native_document::read_single` checks: a one-page `Global/Latest` identity patch still failed (run 37214636238) until the writer stored complete gzip members (`compression::gzip_member_encode`). Cursor Bugbot flagged the second cause on the PR; the thread was answered with the fix and resolved, which the ruleset's thread-resolution rule required before the merge.
- **B44, done** (#651). Three readers: connector records alone lost the fittings to false records with a denormal radius (run 37214338189, 42 of 42 missing); parameter pairs alone never agreed, since a fitting also holds its body's outside diameter (run 37215748458); the connectors' one diameter, or on a fitting with no connector record (the cap) its smallest pair's, passes 42 of 42 (run 37217026749). Stage probe runs 37215079929, 37216600762.
- **B66, done** (#657). `partition_curtain_materials`: a mullion type's material at +135 and RE1's panel type's at +139, 2024 to 2026. Red 12 of 12 on #655 (run 37215541120), green in Corpus health (run 37216785023). rvt-rs's panel type id is right (12611); Revit's `IfcPlateType` takes the panel's own id as its Tag (probe run 37216524399). A Bugbot thread saying the class word could not match was answered with RE-166's measurements and the passing test.
- **B64, in progress** (#659). The unhosted door is `Door-Curtain-Wall-Double-Glas` 445975, which Revit's export aggregates under curtain wall 445961 with no opening and no host (probe run 37216876941): no host wall is right, the gap was the aggregation. Doors are in the schema pass's door list, which `attach_curtain_walls` never saw. Naming a curtain wall is not enough (RE-46: three Snowdon doors name 1506500 and stay standalone, as Bugbot pointed out), so only a door its record places on a curtain grid (RE-72) is a part. Red on #658 (run 37217383444), green for the new test (run 37218029548); the pre-existing `tests/curtain_walls.rs` expected parts gain IfcDoor 1, Revit's own 13, with the maintainer's approval, in its own commit.
- **B70, in progress.** Measure with the MEP samples (B68, run 37209757116) timed out at 90 minutes. Timing checkpoints on one sample (scratch run 37216027753): `rvt-info` 0.02 s, `rvt-ifc` 33.5 minutes, two walks (B71) each spending 1,007 s in `attach_mep_systems` (a search per system id, then a backward search of up to 200,000 bytes per occurrence) and 24 s in `attach_pipe_type_names` (a search per referenced id). Both are now one pass per stream; with the first, a walk took 30 s (run 37218556707), and the three RE1 MEP models' IFC are byte-identical to `main`'s (run 37218534368).
- **B60, in progress.** RE-167 (Measure runs 37217821647, 37218306887, 37218947202). rvt-rs gives a type its symbol's own GlobalId. revit-ifc's MD5 keys (`GUIDUtil.CreateInternal`) reproduce none of the mismatched ones. Instead, every mismatched type except the doors and the spaces takes the GlobalId of another declared element of the file: a FamilySymbol, the instance's original symbol (`ExporterIFCUtils.GetOriginalSymbol`), often the instance's id plus one, while the type's Tag stays the shared symbol's (RE1 Architecture 28, Mechanical 14, Plumbing 15, Electrical 2). Revit therefore writes one type object per original symbol: RE1's 10 mullions share Tag 446539 across 9 type objects. The instance's GElement data object holds the original symbol's id 20 bytes before its end (+300 of 320 bytes, +330 of 350, +392 of 412).
- **Bugbot review threads block merges.** The ruleset requires every review thread resolved, so a Cursor Bugbot comment holds an armed PR in BLOCKED with every check green. Each thread on #653, #657 and #659 was answered with evidence and resolved.

## Findings logged this session

- **RE1 Plumbing: an element Revit does not export.** rvt-rs writes water closet tank 442378 (`Water Closet Tank_M`); Revit's export holds no element with that `Tag`. Probably a nested family instance; not yet investigated.
- **#227's 63rd opening** is a hole in wall 55840's elevation-profile sketch (RE-151); it needs the wall's edited profile read.
- **Revit writes `Pset_DuctSegmentTypeCommon` on RE1 Mechanical's 6 pipes but on none of RE1 Plumbing's 63.** An exporter quirk; rvt-rs writes the set on ducts only, as IFC4 defines it (#605).
- **Witness regeneration takes two CI cycles.** The slim verdict step runs only when the full-project step passes, so a pinned-count change shows the slim drift one push later (#603). B27.

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
  | schema, walker, metadata | `src/walker.rs`, `src/formats.rs`, `src/class_index.rs`, `src/class_tag_map.rs`, `src/es_refs.rs`, `src/metadata.rs`, `src/basic_file_info.rs`, `src/part_atom.rs`, `src/reader.rs`, `src/parse_mode.rs` | `samples`, `field_type_coverage`, `json_schema_contracts`, `rvt_info_cli`, `rvt_inspect_cli`, `rvt_dump_cli`, `schema_registry_catalogs` | done in #574, 3,356 lines |
  | IFC, glTF, schedule writers | `src/ifc/` except the keeps, `src/geometry/`, `src/level_bind.rs`, `src/relations.rs` | `walker_to_ifc_integration`, `ifc_roundtrip`, `ifc_export_overrides`, `rvt_ifc_diagnostics_cli`, `rvt_schedule_cli`, the IfcOpenShell job, Measure | done in #575, 8,443 lines |
  | CLIs, writer, small modules | `src/bin/*.rs`, `src/writer.rs`, `src/capability.rs`, `src/cli.rs`, `src/corpus.rs`, `src/evidence.rs`, `src/identity.rs` | the six `cli` targets, `cfb_roundtrip_delta`, `binary_inventory` | done in #576, 1,063 lines |
  | synthetic-fixture targets | `tests/` unit class except the keeps: `cfb_patch_corpus`, `checksum_page_framing`, `corpus_tier1_health`, `es_remap_golden`, `gen_fixture_roundtrip`, `geometry_recovery`, `ifc_export_modes`, `typed_decoders`. #577 kept `control_cancellation` and the fixture generators `ifc_synthetic_project` (it regenerates `synthetic-project.ifc`) and `ifc_synthetic_structural` (the viewer's structural demo); this row listed them as deleted until B57 | as above per subject | done in #577, 2,096 lines |
  | Python | `tests/python/` (pytest, unit-level, not run) | the wheel smoke on real files (`tools/ci/wheel-smoke.py`) | kept: an integration suite on real files, run in the wheel job (B25) |

### B21 Parameters (XL)

#35 and #155 (wire formats and units), #223 (flag word) and #228 (reference
list). Milestone 0.5.0.

### B22 Held-out validation (blocked)

#408 needs licensed models not used to develop the decoder. It is blocked on
obtaining them.

### Session 3 items

Every new test is seen red on CI before the change it covers. For B53, which covers behaviour that already exists, red is shown by pushing the test with a fault injected into the binary on a scratch branch.

- **B49 Diagnostics on parameters.**
  - *Definition of done:* a `cli` test runs the built `rvt-ifc` with `--diagnostics` on Core Interior and the four RE1 models and checks the diagnostics against the IFC file the same run wrote:
    - `decoded.parameter_value_count` equals the property values in its `Pset_` sets;
    - `revit_element_parameters_to_ifc_property_sets` is claimed only when there are none. Where some are written and others are not, a narrower claim says so, the way `partial_element_geometry` does.
  - The viewer suite checks that Core Interior's Parameters status row reports the count instead of "Revit's parameter table is not read yet".
  - The Core Interior fixtures' `property_sets` rows change from `known_gap` with that feature, after the maintainer approves the change to those existing fixtures. Their notes' "no set carries a Revit element parameter yet" is corrected, and both witness observations are regenerated.
  - The two unconditional claims (compound widths, opening-index host join) are each checked against what they assert, and changed only if false.
  - Measure diff: only the diagnostics change.
- **B48 #421 `ContentDocuments`.**
  - *Definition of done:* a probe written from the stated rule (not from the comment's code, per `CLEANROOM.md`) runs with Measure on RE1 Electrical, Core Interior, Einhoven and the samples. It counts keyed blocks, listed blocks, and the declared state of the top `Family` of each block's chain.
  - Report RE-163 gives the counts, and the reply on #421 links the run. rvt-rs reads no keyed block today (`native_document.rs:412`), so no reader changes; the report says so.
- **B51 Actions cache.** *Definition of done:* `tools/maintainer/clean_caches.py --delete` removes caches of merged branches; `audit.py` reports 12 of 12.
- **B59 Ducts.** *Definition of done:* a `real` test reads RE1 Mechanical's export and Revit's, and every duct has Revit's `Name`, `ObjectType` and a type object of Revit's entity and name. Measure: `names_vs_ifc` 25 equal, typed 74 of 74.
- **B62 False connection.** *Definition of done:* a `real` test checks that every connection rvt-rs writes on RE1 Plumbing and Mechanical is in Revit's export. Measure `ports_vs_ifc`: 0 written that are not in Revit's.
- **B61 Type entities.** *Definition of done:* a `real` test checks that each typed element's type object is Revit's entity on the four RE1 models. Measure: entity agrees on every typed element.
- **B47, B44.** As in session 2: a probe and report first, then the property, with a `real` test against Revit's export.
- **B63 Spaces.** *Definition of done:* a `real` test checks that each RE1 Architecture element Revit contains in an `IfcSpace` is contained in the same space by `IfcRelContainedInSpatialStructure`, with no element moved off its storey on the other models. Measure `storeys_vs_ifc`: 73 same.
- **B66 Curtain materials.** A probe for where a mullion's and a panel's material is stored, then the material, with a `real` test against Revit's sets.
- **B54 Ports.** A probe for where an unconnected connector is stored (RE-138 read only joined pairs), then the ports, with a `real` test against `ports_vs_ifc`'s counts.
- **B53 CLI E2E** (milestones, one per CLI). Each is a `cli` target on corpus files, asserting what the tool's own documentation promises:
  - `rvt-elem-table`'s count equals the table header's;
  - `rvt-history` ends with the file's own release;
  - `rvt-analyze --json` parses and agrees with `rvt-info`;
  - `rvt-diff` of a file with itself reports no differing stream;
  - `rvt-ifc-compare` of an export with itself exits 0 with no divergence, and against Revit's export with `--fail-on-diff` exits 2;
  - an identity `rvt-write` patch round-trips.
- **B60 Type GlobalIds.** A probe that compares rvt-rs's type GlobalIds with Revit's on all five models. It finds the rule for family types, such as a per-symbol suffix or the instance's mapped geometry, and tests it before any change.
- **B64, B50, B52, B57.**
  - B64: a probe of the unhosted door, then a fix or a finding.
  - B50: delete the unused scanner, and the opening-index set unless B49 finds its emission is planned work; `cargo clippy` and `cargo doc` stay green.
  - B52: delete the local branches whose PRs merged. Each branch's tree is diffed against `origin/main` first, because `--no-merged` cannot see squash merges.
  - B57: correct the two launch drafts and B23's row.
