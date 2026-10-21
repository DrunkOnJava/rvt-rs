# 1.0.0 release criteria

Last measured 2026-10-21, on `main` at `e56245fc`.

1.0.0 is the first release with a stability commitment
([`RELEASE.md`](../RELEASE.md#versioning)). Its milestone states the goal: a
new user installs rvt-rs, opens a model, gets correct elements, geometry and
properties, and can reproduce the measurements. The maintainer set the scope
on 2026-10-21: 1.0 supports every Revit release from 2015 to 2027, and it
ships with no gaps or limitations. Every body is Revit's own shape, every
property Revit's export writes is written, and no release in that range is
experimental. This page breaks that goal into requirements that can be
checked. 1.0.0 is tagged when every requirement has passed.

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
| W1 | A new user installs it | Every channel (crates.io, the PyPI wheels, the CLI archives, the container image, the viewer) is checked from what was published, as [Post-publish install verification](../RELEASE.md#post-publish-install-verification) describes | All five checked from what was published for v0.6.1 ([`install.md`](install.md)) | **passed** for 0.6.1; repeated at the 1.0 tag |
| W2 | Every release from 2015 to 2027 is supported | No release in that range is experimental. Each has at least one project with Revit's own IFC export of that release that meets W4, W6 and W7 | 2024 and 2025 have exact element sets against a same-release export (RE-21, RE-32). 2023 has 37 of 37 and 37 of 45, the other 8 being window trims nested in families (RE-81). 2026 has 113 of 115 on one project (RE-124). 2015 has one: BIMcollab's structural example against Revit 2015's own export, all 241 of its elements by ElementId and class, plus 3 it leaves out ([RE-198](../reports/element-framing/RE-198-revit-2015-bimcollab-export.md), [#879](https://github.com/DrunkOnJava/rvt-rs/issues/879)). 2016 to 2022 and 2027 have no same-release export, and are measured only against the same models saved by a neighbouring release (RE-179 to RE-181, [#421](https://github.com/DrunkOnJava/rvt-rs/issues/421)) | **failed** |
| W3 | Family and template files are read like projects | A family (`.rfa`) or template (`.rte`, `.rft`) of every supported release yields its types, parameters and geometry, measured against Revit's export of the family placed in a project | Metadata, previews, schema and OmniClass only ([supported profile](supported-profile.md#by-input)) | **failed** |
| W4 | Correct elements on the models the decoder was developed on | On every reference model, the elements written and their IFC classes equal Revit's export of that model in every category: none missing, none extra | Core Interior (2024): exact in its six instance categories (RE-21, run in CI). RE1 (2025): exact on Architecture, and no element outside Revit's export on six 2025 files (RE-32). Snowdon Towers (2024): 5,945 of 5,974 in Revit's export; the other 29 are 23 curtain panels with no resolved type and 6 slabs in the Legends phase ([#309](https://github.com/DrunkOnJava/rvt-rs/issues/309), [#328](https://github.com/DrunkOnJava/rvt-rs/issues/328); [README](../README.md#what-rvt-rs-reads-from-real-projects)). Class differences remain on the Nordic LCA office ([#792](https://github.com/DrunkOnJava/rvt-rs/issues/792)) | **failed** |
| W5 | Correct on models it was not developed on ([#408](https://github.com/DrunkOnJava/rvt-rs/issues/408)) | At least two projects of every supported release, used in no RE report, each with Revit's own IFC export of that release and measured once before any fix is made for them, meet W4, W6 and W7. The results are published as measured | One held-out measurement: Snowdon Towers Plumbing, HVAC and Electrical (2024). It scored against a VIM of the 2027 edition rather than Revit's IFC export of these files: all 10,385 exported elements in the expected category, storeys and GlobalIds equal, every body a box ([report](../reports/validation/held-out-snowdon-mep-2026-09-27.md)). The first held-out project with Revit's own export of its release is BIMcollab's Revit 2015 structural example (RE-198), measured before any fix. It fails W4: 3 elements written that Revit's export leaves out, and 16 footings on no storey ([#879](https://github.com/DrunkOnJava/rvt-rs/issues/879)). It also fails on identity: none of Revit's GlobalIds rebuilt ([#878](https://github.com/DrunkOnJava/rvt-rs/issues/878)) | **failed** |
| W6 | Geometry | Every element's body is Revit's own shape, measured per category against Revit's export of the same model at the tolerance its report states. No body is a bounding box | Exact where the element's data is decoded, the record's box elsewhere; the diagnostics count each ([#409](https://github.com/DrunkOnJava/rvt-rs/issues/409)). Loadable families are boxes unless `--saved-meshes` draws the mesh Revit saved. Per-category results are in the [supported profile](supported-profile.md#geometry-on-revit-2024-and-2025-projects); the residuals are the issues under the 0.6.x milestone | **failed** |
| W7 | Properties | Every property set, property and quantity Revit's export writes for an element is written, with Revit's value, unit and set | Names, types, storeys, materials, layers, GlobalIds, `IsExternal` and parameters named for a standard property set are read. Most instance and type parameters are not ([#35](https://github.com/DrunkOnJava/rvt-rs/issues/35), [#155](https://github.com/DrunkOnJava/rvt-rs/issues/155), the 0.7.0 milestone) | **failed** |
| W8 | Nothing is approximated without saying so | A supported input gets no approximation. An input 1.0 does not support (Revit 2014 and earlier, a damaged file) is refused or reported as such by the diagnostics, `rvt-inspect` and the viewer | The diagnostics count boxed bodies (#409) and unattributed records (`unexported_element_records`). Since [#871](https://github.com/DrunkOnJava/rvt-rs/issues/871), all three say when a file's release is read experimentally. Supported inputs are still approximated: boxed bodies and unread properties (W6, W7) | **failed** |
| W9 | The viewer journey | Open, inspect, schedule, export and diagnostics, verified end to end on the reference models (0.7.0) | The browser tests open a sample with no network request, load Core Interior and check its Parameters row, and check the shell's layout and accessibility. They do not run the whole journey on the reference models | **not measured** |
| W10 | The measurements reproduce | *Proposed:* every number in the README and the supported profile names its model and says whether anyone can fetch it. Numbers on redistributable models re-run with one documented command | Redistributable models are fetched by `tools/fetch-reference-models.sh` and scored in CI and Measure, and the witness verdicts are checked in CI. Revit's IFC export of Snowdon Towers, the MIT house and the 255ribeiro projects are local only, so the numbers resting on them cannot be re-run by others ([reference models](reference-models.md)). Nothing yet checks that every number names its model | **not measured** |

## Interfaces 1.0 stabilizes

| | Requirement | Bar | Measured now | Status |
|---|---|---|---|---|
| S1 | The stable surface is named ([#873](https://github.com/DrunkOnJava/rvt-rs/issues/873)) | `RELEASE.md` names the surfaces 1.0 commits to (the crate, the Python package, the CLIs and their JSON, the output formats, the wasm API) and those it does not | Not named. The crate has 95 public modules, research modules among them | **failed** |
| S2 | Breaking changes are caught (#873) | Each stable surface has a check that fails on an unannounced breaking change | The CLI inventory and the JSON schema contracts are checked; the contract test skips without the family corpus. The crate and the Python API have no semver check | **failed** |
| S3 | The Rust API is documented with runnable examples ([#872](https://github.com/DrunkOnJava/rvt-rs/issues/872)) | docs.rs shows the user entry points, each with an example CI runs | docs.rs builds 0.6.1. The crate-level examples are `no_run`, and `examples/` holds research probes only | **failed** |
| S4 | The Python API is documented with runnable examples (#872) | Every Python block in [`python.md`](python.md) runs in CI | 21 blocks, none run | **failed** |

## Evidence from outside the project

| | Requirement | Bar | Measured now | Status |
|---|---|---|---|---|
| P1 | Practitioners' reports are part of the supported profile ([#874](https://github.com/DrunkOnJava/rvt-rs/issues/874)) | *Proposed:* at least three BIM or AEC practitioners have run the documented workflows on their own models and reported, and each finding is an issue or a line in the supported profile | None. Outside activity so far is format research (Discussion #112, #421), code contributions (0.5.0, 0.6.0) and one show-and-tell (Discussion #393) | **failed** |

## Limitations

None. The maintainer decided on 2026-10-21 that 1.0 ships with no gaps or
limitations, so every row of the [supported profile](supported-profile.md)
has to read as exact before the tag. The work that blocks this is the set of
issues open under the 0.6.x (geometry), 0.7.0 (properties) and 1.0.0
milestones.

Out of scope, and so not counted as limitations: Revit 2014 and earlier,
and what [`ROADMAP.md`](../ROADMAP.md#out-of-scope) rules out. That covers
Autodesk SDK internals, a Revit API-compatible surface, field-level writes
before [ADR-002](decisions/ADR-002-semantic-write-api-gate.md)'s gate opens,
and cloud worksharing, licensing and linked-model semantics.

## Keeping this page

A pull request that moves a status updates this page and links the new
evidence. The monthly checklist in [`maintaining.md`](maintaining.md) compares
the page with the 1.0.0 milestone, and the release checklist
([`RELEASE.md`](../RELEASE.md#the-10-gate)) does not tag 1.0.0 while any
requirement is not passed.
