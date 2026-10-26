# 1.0.0 release criteria

Measured 2026-10-21, on `main` at `e56245fc`; the stability rows S1, S2 and S4
and the viewer row W9 were measured again on 2026-10-23, in the pull requests
that changed them, and the install row W1 and the API documentation row S3
on 2026-10-25, from what v0.8.0 published.

1.0.0 is the first release with a stability commitment
([`RELEASE.md`](../RELEASE.md#versioning)). Its milestone states the goal: a
new user installs rvt-rs, opens a model, gets correct elements, geometry and
properties, and can reproduce the measurements. The maintainer set the scope
on 2026-10-21: 1.0 supports every Revit release from 2015 to 2027, and it
ships with no gaps or limitations. Every body is Revit's own shape, every
property Revit's export writes is written, and no release in that range is
experimental. On 2026-10-26 the maintainer excluded the values a project's
own export-time property set file places, which the model does not store
(W7). This page breaks that goal into requirements that can be checked.
1.0.0 is tagged when every requirement has passed.

Each requirement has a bar, the value measured now, a status and its
evidence. A status is one of three values:

- **passed**: measured, and the bar is met.
- **failed**: measured, and the bar is not met.
- **not measured**: no evidence either way, or no bar set yet.

A closed issue or a milestone's completion percentage is not evidence. A bar
marked *proposed* is a draft until the maintainer sets it. A status changes
only in a pull request that links the new evidence.

## Workflows

| | Requirement | Bar | Measured now | Status |
|---|---|---|---|---|
| W1 | A new user installs it | Every channel (crates.io, the PyPI wheels, the CLI archives, the container image, the viewer) is checked from what was published, as [Post-publish install verification](../RELEASE.md#post-publish-install-verification) describes | All five checked from what was published for v0.8.0 ([`install.md`](install.md)) | **passed** for 0.8.0; repeated at the 1.0 tag |
| W2 | Every release from 2015 to 2027 is supported | No release in that range is experimental. Each has at least one project with Revit's own IFC export of that release that meets W4, W6 and W7 | 2024 and 2025 have exact element sets against a same-release export (RE-21, RE-32). 2023 has 37 of 37 and 37 of 45, the other 8 being window trims nested in families (RE-81). 2026 has 113 of 115 on one project (RE-124). 2015 has one: BIMcollab's structural example against Revit 2015's own export, all 241 of its elements by ElementId and class, plus 3 it leaves out ([RE-198](../reports/element-framing/RE-198-revit-2015-bimcollab-export.md), [#879](https://github.com/DrunkOnJava/rvt-rs/issues/879)). 2016 to 2022 and 2027 have no same-release export, and are measured only against the same models saved by a neighbouring release (RE-179 to RE-181, [#421](https://github.com/DrunkOnJava/rvt-rs/issues/421)) | **failed** |
| W3 | Family and template files are read like projects | A family (`.rfa`) or template (`.rte`, `.rft`) of every supported release yields its types, parameters and geometry, measured against Revit's export of the family placed in a project | Metadata, previews, schema and OmniClass only ([supported profile](supported-profile.md#by-input)) | **failed** |
| W4 | Correct elements on the models the decoder was developed on | On every reference model, the elements written and their IFC classes equal Revit's export of that model in every category: none missing, none extra | Core Interior (2024): exact in its six instance categories (RE-21, run in CI). RE1 (2025): exact on Architecture, and no element outside Revit's export on six 2025 files (RE-32). Snowdon Towers (2024): 5,945 of 5,974 in Revit's export; the other 29 are 23 curtain panels with no resolved type and 6 slabs in the Legends phase ([#309](https://github.com/DrunkOnJava/rvt-rs/issues/309), [#328](https://github.com/DrunkOnJava/rvt-rs/issues/328); [README](../README.md#what-rvt-rs-reads-from-real-projects)). Class differences remain on the Nordic LCA office ([#792](https://github.com/DrunkOnJava/rvt-rs/issues/792)) | **failed** |
| W5 | Correct on models it was not developed on ([#408](https://github.com/DrunkOnJava/rvt-rs/issues/408)) | At least two projects of every supported release, used in no RE report, each with Revit's own IFC export of that release and measured once before any fix is made for them, meet W4, W6 and W7. The results are published as measured | One held-out measurement: Snowdon Towers Plumbing, HVAC and Electrical (2024). It scored against a VIM of the 2027 edition rather than Revit's IFC export of these files: all 10,385 exported elements in the expected category, storeys and GlobalIds equal, every body a box ([report](../reports/validation/held-out-snowdon-mep-2026-09-27.md)). The first held-out project with Revit's own export of its release is BIMcollab's Revit 2015 structural example (RE-198), measured before any fix. It fails W4: 3 elements written that Revit's export leaves out, and 16 footings on no storey ([#879](https://github.com/DrunkOnJava/rvt-rs/issues/879)). It also fails on identity: the project's, site's, building's and five storeys' GlobalIds are Revit's, from the values the file stores ([RE-220](../reports/element-framing/RE-220-stored-global-ids.md)), but none of its 241 elements' ([#878](https://github.com/DrunkOnJava/rvt-rs/issues/878)) | **failed** |
| W6 | Geometry | Every element's body is Revit's own shape, measured per category against Revit's export of the same model at the tolerance its report states. No body is a bounding box | Exact where the element's data is decoded, the record's box elsewhere; the diagnostics count each ([#409](https://github.com/DrunkOnJava/rvt-rs/issues/409)). Loadable families are boxes unless `--saved-meshes` draws the mesh Revit saved, which is Revit's own shape where it is complete (every vertex within 0.001 ft on all 138 of Core Interior's doors and windows and 1,294 of 1,709 bodies scored on the BIM4LCA office) but part of the body elsewhere (RE-78). A face the saved graphics do not decode marks a partial mesh, which keeps its box since RE-224: on the office 1,294 of the 1,352 bodies then drawn are exact. Some partial meshes record nothing, so the option stays opt-in. Walls: 360 of 360 with Revit's ends on Core Interior, 482 of 607 on the office as built (RE-222, RE-223). Per-category results are in the [supported profile](supported-profile.md#geometry-on-revit-2024-and-2025-projects); the residuals are the issues under the 0.9.0 milestone, and `tools/re/box_bodies_vs_ifc.py` counts in Measure, per class, the bodies still drawn as their box and how much of it Revit's body fills | **failed** |
| W7 | Properties | Every property set, property and quantity Revit's export writes for an element is written, with Revit's value, unit and set. A value that the export's own user-defined property set file maps into a set is out of scope (the maintainer, 2026-10-26): the model does not store the file, and RE-239 lists those values per reference model | Names, types, storeys, materials, layers, GlobalIds, `IsExternal`, curtain mullions' `Span` (RE-226), walls' `ExtendToStructure` (RE-228), railings' `Height` (RE-231), level slabs' `PitchAngle` (RE-238), stair flights' treads, tread lengths, nosing and `Reference`, landings' `Reference` and parameters named for a standard property set are read. On the BIM4LCA office the export's own property set file places 2,013 values as built and 1,938 at permit that rvt-rs does not write as Revit does (glazing on walls, curtain walls and coverings, rooms' `OccupancyType` and the value type of their `OccupancyNumber`, the `FI_` sets), out of scope; no other reference model has one (RE-239). Quantities are measured by `tools/re/qtos_vs_ifc.py` ([#1016](https://github.com/DrunkOnJava/rvt-rs/issues/1016)): of the 50,580 Revit's export writes on objects rvt-rs writes on the office as built, 11,993 are written equal (curtain mullions' `Length`, doors' and windows' `Width`, `Height` and `Area` where the symbol drawing each and its type agree on its size, walls' `Width`, pipe segments' `Length`), 1 differs (the mullion whose `Span` differs) and 38,586 are missing, and RE1's 94 IFC2X3 pipe and duct lengths all are written; Core Interior's and RE1 Architecture's exports were made with Revit's "Export base quantities" off, so the census counts the quantities rvt-rs writes on them apart (715 and 29), and `rvt-ifc --no-base-quantities` leaves them out, base quantities being on by default as in each of Revit's built-in IFC4 setups (RE-242); the census scores type objects' own sets too ([#1018](https://github.com/DrunkOnJava/rvt-rs/issues/1018)): of the values Revit's export writes on the type objects rvt-rs writes, 184 on the office as built, mostly `IsExternal`, none is written yet. Where Revit's export places a property outside the set IFC4's definitions give, W7 is measured with `rvt-ifc --revit-set-placement`, the default output following buildingSMART's definitions (RE-235); a value Revit's export writes that is known to be wrong (`WalkingLineOffset`) is left out and listed in RE-235 §5. On the office as built, 13,882 of the 58,006 property values Revit's export writes on objects rvt-rs writes are missing, 10,003 of them curtain mullions' `IsExternal`. Most instance and type parameters are not ([#35](https://github.com/DrunkOnJava/rvt-rs/issues/35), [#155](https://github.com/DrunkOnJava/rvt-rs/issues/155), the 0.7.0 milestone) | **failed** |
| W8 | Nothing is approximated without saying so | A supported input gets no approximation. An input 1.0 does not support (Revit 2014 and earlier, a damaged file) is refused or reported as such by the diagnostics, `rvt-inspect` and the viewer | The diagnostics count boxed bodies (#409) and unattributed records (`unexported_element_records`). Since [#871](https://github.com/DrunkOnJava/rvt-rs/issues/871), all three say when a file's release is read experimentally. Supported inputs are still approximated: boxed bodies and unread properties (W6, W7) | **failed** |
| W9 | The viewer journey | Open, inspect, schedule, export and diagnostics, verified end to end on the reference models (0.7.0) | `viewer/tests/journey.spec.ts` runs the whole journey in Chromium on the six redistributable reference models: Einhoven, Core Interior and the four RE1 models (Deploy viewer, run [40001472629](https://github.com/DrunkOnJava/rvt-rs/actions/runs/40001472629)). It opens each, inspects an element, and downloads the schedule, IFC, GLB, plan SVG and diagnostics. The inspected element is in the schedule and in the IFC under its ElementId. The schedule lists exactly the elements the diagnostics count by type. On Einhoven and Core Interior, the export has the counts `tests/project_count_fixtures.rs` pins for the CLI's. The local-only reference models ([reference models](reference-models.md)) are not run | **passed** |
| W10 | The measurements reproduce | *Proposed:* every number in the README and the supported profile names its model and says whether anyone can fetch it. Numbers on redistributable models re-run with one documented command | Redistributable models are fetched by `tools/fetch-reference-models.sh` and scored in CI and Measure, and the witness verdicts are checked in CI. Revit's IFC export of Snowdon Towers, the MIT house and the 255ribeiro projects are local only, so the numbers resting on them cannot be re-run by others ([reference models](reference-models.md)). Nothing yet checks that every number names its model | **not measured** |

## Interfaces 1.0 stabilizes

| | Requirement | Bar | Measured now | Status |
|---|---|---|---|---|
| S1 | The stable surface is named ([#873](https://github.com/DrunkOnJava/rvt-rs/issues/873)) | `RELEASE.md` names the surfaces 1.0 commits to (the crate, the Python package, the CLIs and their JSON, the output formats, the wasm API) and those it does not | [`RELEASE.md`](../RELEASE.md#what-10-keeps-stable) names all five and what each leaves out. The crate keeps 27 modules stable; the other 67 are under `rvt::research`, behind the `unstable-research` feature, and `rvt::cli` is hidden. CI fails if a stable item hands out a research type (`unnameable_types` on the default-features build) | **passed** |
| S2 | Breaking changes are caught (#873) | Each stable surface has a check that fails on an unannounced breaking change | CI's `semver checks` job compares each pull request with its base and fails on a breaking change the title does not mark with `!`: the crate's public API (cargo-semver-checks), the Python type stubs (`tools/ci/python-api-compat.py`; stubtest holds the stubs to the built wheel), the 22 wasm bindings (`tools/ci/wasm-api.py`) and the CLIs' JSON schemas (`tools/ci/schema-compat.py`). All twelve CLIs that print JSON have a schema their output is validated against on Linux with the family corpus. The output formats: IfcOpenShell validates the exports, `rvt-gltf` writes a glTF 2.0 container, the schedules begin with the columns the user guide lists, and every writer gives the same bytes twice on the same file (`tests/reproducible_outputs.rs`, `rvt-ifc` with `SOURCE_DATE_EPOCH`) | **passed** |
| S3 | The Rust API is documented with runnable examples ([#872](https://github.com/DrunkOnJava/rvt-rs/issues/872)) | docs.rs shows the user entry points, each with an example CI runs | On `main`, these entry points have an example CI runs as a doctest against `corpus/tier1/` (`cargo test --profile ci --doc`, #900): `RevitFile::open`, `open_with_limits`, `open_bytes` and `open_bytes_with_limits`, `metadata::read_metadata`, `walker::iter_elements`, `RvtDocExporter::export_with_diagnostics`, `write_step`, `model_to_glb`, `elements_csv`, `sheet::render_plan_svg` and the writer's `copy_file`, and `write_with_patches` with `verify_patches_applied`. Other public items have none yet. `examples/` has three user examples, `basics_inspect`, `basics_elements` and `basics_export`, which CI runs (#904). docs.rs shows 0.8.0 (2026-10-25) with every one of them, `write_with_patches` included (0.7.0's page had none), read from docs.rs's rustdoc JSON for the version | **passed** for 0.8.0 |
| S4 | The Python API is documented with runnable examples (#872) | Every Python block in [`python.md`](python.md) runs in CI | The 3 runnable blocks run in CI against `corpus/tier1/` (`tests/python/test_docs_python.py`, in the wheel job); the other 18 show a signature or a result shape and are fenced `text` | **passed** |

## Evidence from outside the project

| | Requirement | Bar | Measured now | Status |
|---|---|---|---|---|
| P1 | Practitioners' reports are part of the supported profile ([#874](https://github.com/DrunkOnJava/rvt-rs/issues/874)) | At least three BIM or AEC practitioners have run the documented workflows on their own models and reported, and each finding is an issue or a line in the supported profile | None yet. The request for reports went out on 2026-10-25 ([Discussion #982](https://github.com/DrunkOnJava/rvt-rs/discussions/982), pinned). Outside activity before it was format research (Discussion #112, #421), code contributions (0.5.0, 0.6.0) and one show-and-tell (Discussion #393) | **failed** |

## Limitations

None. The maintainer decided on 2026-10-21 that 1.0 ships with no gaps or
limitations, so every row of the [supported profile](supported-profile.md)
has to read as exact before the tag. The work that blocks this is the set of
issues open under the 0.9.0 (geometry and element data) and 1.0.0
milestones.

Out of scope, and so not counted as limitations: Revit 2014 and earlier,
and what [`ROADMAP.md`](../ROADMAP.md#out-of-scope) rules out. That covers
Autodesk SDK internals, a Revit API-compatible surface, field-level writes
before [ADR-002](decisions/ADR-002-semantic-write-api-gate.md)'s gate opens,
and cloud worksharing, licensing and linked-model semantics. Also out of
scope, decided on 2026-10-26: a property value that a project's export-time
user-defined property set file maps into a set. The model stores the
parameter and the export setting that names the file, but not the file, so
which parameter feeds which set on which element is not in the input; [RE-239](../reports/element-framing/RE-239-export-file-values.md)
lists those values per reference model.

## Keeping this page

A pull request that moves a status updates this page and links the new
evidence. The monthly checklist in [`maintaining.md`](maintaining.md) compares
the page with the 1.0.0 milestone, and the release checklist
([`RELEASE.md`](../RELEASE.md#the-10-gate)) does not tag 1.0.0 while any
requirement is not passed.
