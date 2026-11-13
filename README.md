# rvt-rs

**Inspect Revit files without installing Revit.**

rvt-rs is an open-source toolkit for reading Autodesk Revit project, family and template files (`.rvt`, `.rfa`, `.rte`, `.rft`). It reads a file's metadata, preview images and embedded schema. On the Revit releases it supports, it also recovers the building's elements (walls, floors, roofs, doors, windows, stairs, curtain walls, furniture, MEP and structure) with the names, types, storeys, materials and identifiers Revit gives them, and exports them as IFC4 (the open building-model exchange format), glTF (3D), plan SVG and CSV schedules.

It comes as command-line tools, a Rust library, Python bindings and a browser viewer that reads your file inside the browser tab. It is built from public observations of the file format ([`CLEANROOM.md`](CLEANROOM.md)), licensed under the [Apache License 2.0](LICENSE), and not affiliated with Autodesk.

## Start here

| What you want to do | Where to start |
|---|---|
| See a model without installing anything | Open the [browser viewer](https://drunkonjava.github.io/rvt-rs/) and pick a demo, or drop in a file of your own. |
| Inspect or export files from the command line | The [installation guide](docs/install.md); prebuilt binaries are on [GitHub Releases](https://github.com/DrunkOnJava/rvt-rs/releases). |
| Work with files in Python | The [Python guide](docs/python.md) (`pip install rvt`). |
| Let an agent ask about a file | The [MCP server](docs/mcp.md), `rvt-rs-mcp`: local, read-only, over stdio. |
| Check what your file will give | [What you can rely on, by input](docs/evidence.md#what-you-can-rely-on-by-input), then the [user guide](docs/user-guide.md). |
| Use the Rust library | The [examples](examples/README.md) (open a file, list its elements, export it), the [library surface](docs/library.md#library-surface) and the [API documentation](https://docs.rs/rvt). |
| Tell us how it went on your own models | [Discussion #982](https://github.com/DrunkOnJava/rvt-rs/discussions/982): your diagnostics are enough, never the building. |
| Ask a question, report a problem or contribute | [Discussions](https://github.com/DrunkOnJava/rvt-rs/discussions) for questions, and the [contribution guide](CONTRIBUTING.md) for everything else. |

## Support and evidence

Support depends on the Revit release, the file type and the feature. Opening a file and reading its metadata does not mean its elements or geometry can be recovered, and every export comes with diagnostics that say what was approximated or left out: check them before relying on an export for measurement or coordination. The figures in [`docs/evidence.md`](docs/evidence.md) compare rvt-rs's export with Revit's own IFC export of the same file, element by element, and [`docs/support-matrix.json`](docs/support-matrix.json) records each capability with its evidence and its status ceiling. A "Measure run" is a run of the repository's [Measure workflow](.github/workflows/measure.yml), which exports the redistributable reference models on GitHub's runners and scores them against Revit's own exports with the scorers in `tools/re/`; an RE number is a report in [`reports/element-framing/`](reports/element-framing/). Every number on this page and on the [supported profile](docs/supported-profile.md) names the model it was measured on and says "local only" or "not shared" when nobody else can fetch that model; [`docs/reference-models.md`](docs/reference-models.md#model-index) lists the models and how to [re-run the numbers](docs/reference-models.md#re-run-the-numbers) on the ones anyone can fetch (nobody has yet re-run these pages' figures from that command and compared them), and CI checks the rule on both pages.

## Support by Revit release

Each row is rendered from [`docs/support-matrix.json`](docs/support-matrix.json), and CI fails when this table and the matrix disagree. "Projects" is the matrix's status for the release's project element records, "What a project gives" the capability it rates, and "Families" the status of the release's family files (metadata, previews and schema). The status words are the matrix's own: verified, partial, experimental, unsupported and unknown.

<!-- support-matrix:releases -->

| Revit release | Projects | What a project gives | Families |
|---|---|---|---|
| 2008 to 2010 | experimental | Revit 2008 to 2010 partition element records, read in the 32-bit layout of Revit 2011 to 2023, with storeys and named materials | not measured |
| 2011 to 2014 | experimental | Revit 2011 to 2014 partition element records, read in the 32-bit layout of Revit 2015 to 2023, with storeys, hosts, type names, layer sets and materials | not measured |
| 2015 to 2018 | experimental | Revit 2015 to 2018 partition element records, read in the 32-bit layout of Revit 2019 to 2023, with storeys, hosts, type names, layer sets and materials | not measured (2015); verified (2016 to 2018) |
| 2019 to 2022 | experimental | Revit 2019 to 2022 partition element records, read in Revit 2023's 32-bit layout, with storeys, hosts, type names, layer sets and materials | verified |
| 2023 | partial | Revit 2023 partition element records, with wall joins, beam cuts, room outlines, storeys, hosts, family and type names, materials and layer sets (RE-81) | verified |
| 2024 | partial | Revit 2024 partition element records as typed IFC elements, with joins, profiles and cuts where decoded, storeys, family and type names, materials and layers, GlobalIds and stair dimensions | verified |
| 2025 | partial | Revit 2025 partition element records (walls, slabs, rooms, and the rest of the RE-21 categories) | verified |
| 2026 | experimental | Revit 2026 partition element records, with storeys, rooms, doors and windows, type names and layer sets | verified |
| 2027 | experimental | Revit 2027 partition element records, with storeys, rooms, doors and windows, type names, layer sets and wall joins | verified |

<!-- /support-matrix -->

The measured detail behind each row is in [`docs/evidence.md`](docs/evidence.md), and `rvt-capabilities --matrix -f text` prints the matrix.

## Install and quick start

The latest release is [v0.21.0](https://github.com/DrunkOnJava/rvt-rs/releases/tag/v0.21.0) (2026-11-13). Install it with any of:

- the prebuilt Linux, macOS and Windows binaries on [GitHub Releases](https://github.com/DrunkOnJava/rvt-rs/releases), or the container image `ghcr.io/drunkonjava/rvt-rs`;
- `cargo install rvt --locked`;
- `pip install rvt`, for the Python bindings;
- `cargo install rvt-rs-mcp --locked`, for the [MCP server](#mcp-server) alone;
- nothing at all: the [browser viewer](https://drunkonjava.github.io/rvt-rs/) reads a file inside the tab.

[`docs/install.md`](docs/install.md) covers every channel, building from source and the smoke tests. This README describes `main`, which the browser viewer is also built from, so it can be ahead of the latest release: "Upgrading from 0.20" in the [changelog](CHANGELOG.md#upgrading-from-020) says what a user of 0.20 has to change, and each change `main` adds after 0.21.0 is a fragment in [`changelog.d/`](changelog.d/README.md), which the next release moves into `CHANGELOG.md`<!-- w10: release versions of this project, not measurements -->. To use `main` before the next release, build from source ([`docs/install.md`](docs/install.md#build-from-source)).

Three commands cover most of what the command line does:

```bash
rvt-info model.rvt
rvt-ifc model.rvt -o model.ifc --diagnostics model.diagnostics.json
rvt-elements model.rvt --counts
```

`rvt-info` reads the file's release, metadata and previews, `rvt-ifc` writes IFC4 and a diagnostics file that says what the export approximated or left out, and `rvt-elements --counts` counts the recovered elements by class as JSON. The same from Python, on a project file:

```python
import rvt

f = rvt.RevitFile("model.rvt")
print(f.version)                 # the Revit release year, or None
counts = f.element_counts()      # {"total": int, "by_class": {class name: count}}
elements = f.decoded_elements()  # dicts with id, class_name, byte_range, fields, parameters, typed
with open("model.ifc", "w") as out:
    out.write(f.write_ifc())     # IFC4 STEP as a str, in rvt-ifc's default mode
```

The [Python guide](docs/python.md) documents the rest, with a [Jupyter notebook](docs/rvt-python-quickstart.ipynb).

## In the browser

Drop a `.rvt`, `.rfa`, `.rte` or `.rft` at <https://drunkonjava.github.io/rvt-rs/>: the file is read and converted inside the tab and is not uploaded. The viewer is the core library compiled to WebAssembly, with the 13 JS-callable wasm-bindgen bindings of the `rvt-wasm` crate<!-- w10: a count tools/ci/wasm-api.py holds to the source, not a measurement --> ([`docs/viewer-api.md`](docs/viewer-api.md#the-javascript-bindings)); it parses in a worker, draws with Three.js and exports glTF 2.0 binary, IFC4 STEP and plan-view SVG. It is built from `main`, not from a release, and its demo gallery opens two MIT-licensed real projects, hash-verified when staged.

Every deploy checks the viewer's privacy posture in two ways. `viewer/scripts/audit-wasm-imports.mjs` reads each import of the compiled `.wasm` and its shim in the JavaScript glue, and fails if any reaches `fetch`, `XMLHttpRequest`, `WebSocket`, `EventSource` or `sendBeacon`. A Playwright test opens a real project in the built viewer and fails on any request to another origin, or on any request at all after the file is open. The page's Content-Security-Policy also limits its connections to its own origin. Together these cover the WebAssembly module's imports, the page's connection policy and the flow the test drives; they are checks, not a proof about every possible code path. [`docs/viewer-privacy-posture.md`](docs/viewer-privacy-posture.md) states the invariant and what each check covers.

## MCP server

`rvt-rs-mcp` is a local, read-only [Model Context Protocol](https://modelcontextprotocol.io) server over stdio, the `rvt-mcp` workspace member, so an agent can ask about a Revit file with no Revit install and no network. `open_model` decodes a file once, with the decode `rvt-ifc` runs, and every tool (`get_summary`, `list_elements`, `get_element`, `explain_element`, `list_rooms`, `list_levels`, `get_diagnostics`) answers from that same model, each element with its provenance, so an answer and the IFC export cannot disagree; `export_ifc` and `export_gltf` write to a path the agent names. It opens no socket and makes no network call, and it reports what it left out the way the CLIs do. It is a crate of its own, so `rvt` gains no async runtime, and it needs Rust 1.88, one above the CLIs. It ships from v0.14.0, in the release archives, the container image and as the `rvt-rs-mcp` crate; [`docs/mcp.md`](docs/mcp.md) has the install, the client configuration, the one folder it reads from and writes to (`--root`), the tools, the handle lifetime and what it does not read.

## What does not work yet

The full list, with each gap's status and evidence, is in [`docs/evidence.md`](docs/evidence.md#what-does-not-work-yet). In brief:

- Files from before Revit 2008 are not read, and 2008 to 2014 files are experimental, measured on local files only. 1.0 covers Revit 2024 to 2027 (since 2026-11-08), and 2015 to 2023 stay outside it until a 1.x release adds each year with its same-release Revit exports <!-- w10: a release range and version numbers, not a measurement -->.
- Revit 2015 to 2022 and 2027 projects are experimental: measured against the same models saved by a neighbouring release, not against Revit's own export (BIMcollab's 2015 example, RE-198, and Vitoria's 2018 `ALD26 Before`, RE-315, aside, and held-out projects of 2020, 2021, 2022 and 2027, whose counts are a regression check and no part of these results). Revit 2026 is experimental too, measured on one project, flowbim's `AA-SingleDwellingHouse` (local only), and two held-out projects, counts only.
- On Revit 2008 to 2023 projects an element's body is its bounding box, cut back at its joins where they are read, except a wall at an angle on Revit 2021 to 2023, or on 2015 to 2020 where its box closes on its line's rectangle, drawn along its line, a pipe on 2015 to 2023, drawn as its cylinder (RE-370, held to the same samples' 2024 saves), and a beam on 2015 to 2023 along its location line (RE-362): read on Autodesk's sample projects, held to the same samples' later saves, which cannot catch an error those share, and not checked by any Revit export; none of the 91 beams of BIMcollab's Revit 2015 example holds a line the reader finds (#1514), so its reach beyond those samples is not shown. On 2026 and 2027 it is that too, except for pipes as their cylinders (RE-370), beams along their axes, stair runs and tread landings (a monolithic landing stays its box), and floors, ceilings and slabs drawn from their sketch lines (RE-341; held to the same project's 2024 and 2025 saves, which cannot catch an error those saves share, and not scored against a Revit export except for one stair flight; quantities and property sets of those bodies are not compared). A roof there is drawn from its saved mesh where all of the mesh decodes (#1191, and #1612 for one drawn as the level block of its sketch outline), and keeps the record-box solve where it does not, without its slope: a sloped roof can still be drawn as its outline at the record box's full height, a level block, or as its box, and no 2026 or 2027 scorer checks it. On 2025 beams along their axes, tread landings (a monolithic landing stays its box) and a sketch's arcs are drawn too (RE-345), held to the same project's 2024 save alone, which cannot catch an error that save shares; no Revit export of a 2025 beam, tread landing or arc exists here: the 28 Revit 2025 models read (puzzbobb's 24 and RE1's four) hold no beam or tread landing, and the 25 of them RE-345 counts with Revit's own 2025 exports (puzzbobb's 24 and RE1 Architecture) hold no arc in a floor, ceiling or roof sketch. A shaft opening is cut from floors, ceilings and roofs on 2024 only (RE-345); on 2025 to 2027 they stay whole and the export counts the shafts. On 2024 and 2025 it is the box only where the bytes are not decoded: roofs by extrusion outside Revit 2025, hip roofs over a T or U footprint, monolithic spiral stair flights and straight riserless ones with a nosing, edited wall profiles, ducts and pipe fittings. From 2023, every element that would be a box (doors, windows, furniture, fixtures, equipment and the rest) is drawn from Revit's saved mesh where that decodes whole, and from 2024 so are the walls that embed a curtain wall where Revit raised their base (RE-340); `--no-saved-meshes` keeps the boxes, which reading every one's saved graphics costs 1.04 to 1.92 times the time and 1.04 to 1.69 times the peak memory of 0.17.0 on Autodesk's samples of Revit 2023 to 2027 (the BIM4LCA office as built from 21.2 s and 913 MiB to 24.3 s and 944 MiB, Measure run 43152271314, #1694), and `--family-extrusions` (opt-in; implies `--saved-meshes`) draws an instance whose family's extrusions reproduce its mesh from them, as extruded profiles; an arc is written as the arc, whatever its radius.
- `rvt-elements` and Python list each element's saved parameters on Revit 2023 to 2027 files, experimentally on 2015 to 2022 ones and not on 2008 to 2014, but built-in parameters have no name, a wall's or floor's type parameters are not joined to it, and only `rvt-ifc --revit-property-sets` writes the rest into IFC.
- Family files give metadata, previews and types. One visible positive revolution in a standalone Revit 2020 family can also export as an analytic IFC solid (experimental): six synthetic rectangular/circular files are measured locally against native references, including partial and tilted sweeps ([RE-394](reports/element-framing/RE-394-analytic-family-revolutions.md)). This does not establish general family geometry or project-instance placement. Template files (`.rte`, `.rft`) are not measured, because no template is in any corpus.
- A room is left out of the export when its own phase is not the export phase, the project's last unless `rvt-ifc --phase` names another, as Revit's exporter does (RE-326); a room whose phase is unset or unread is written, and so is an Area or an MEP space. Where no phase is named and the last phase holds no room, the rooms written are those of the latest phase that holds any, and the diagnostics, `rvt-ifc` and the Python bindings say so (#1359): Autodesk's `rac_basic_sample_project` has 14 rooms in Working Drawings and none in its last phase, and writes them by default; naming the last phase writes none. The phase rule is measured on no Revit export made with the default setup, because none has such a room: the one Revit export of a project with rooms outside its last phase, a Revit 2018 export of `rac_basic_sample_project` that a user posted on the IFC for Revit wiki (local only), holds the 14, and rvt-rs writes them with `--phase "Working Drawings"`. The fallback is a choice, not a measurement of Revit's default. The Python bindings and the viewer export the default phase.
- Semantic editing of a Revit file is not implemented: the write path patches named streams byte for byte, and field-level writes wait on ADR-002.

## Command-line tools

Rust 2024 edition (MSRV 1.87). **Nineteen CLIs ship** (`rvt-analyze`, `rvt-info`, `rvt-inspect`, `rvt-schema`, `rvt-history`, `rvt-diff`, `rvt-corpus`, `rvt-dump`, `rvt-doc`, `rvt-ifc`, `rvt-ifc-compare`, `rvt-write`, `rvt-gltf`, `rvt-sheet`, `rvt-elem-table`, `rvt-elements`, `rvt-capabilities`, `rvt-schedule`, `gen-fixture`) plus the reproducible probes under `probes/examples/`, one or more per format finding. Python bindings via pyo3+maturin in the `rvt-py` workspace member (so the core `rvt` crate can stay unconditionally `#![forbid(unsafe_code)]`) — `pip install rvt`.

`rvt-analyze --redact path/to/your.rfa` prints a file report: identity, upgrade history, format anchors, the schema table, the schema-to-data link histogram, content metadata and a disclosure scan. Every command with an example, the Rust library's modules and the research probes are in [`docs/library.md`](docs/library.md).

## For contributors

Use the [Codespaces development environment](docs/codespaces.md) for remote
Linux builds and the viewer from VS Code, a terminal, Codex or Claude Code.

rvt-rs is a clean-room reader for Revit files: an Apache-2.0 Rust core with
Python bindings and a WebAssembly viewer, with no Revit install or Autodesk
SDK at build or run time. Contributing does not need private files either —
the checked-in `corpus/tier1/` synthetic fixtures drive the default gates, and
corpus-backed tests skip themselves while `RVT_PROJECT_CORPUS_DIR` is unset.

- **Build:** stable Rust 1.87 or newer, then `cargo build`. Viewer:
  `cd viewer && pnpm install --frozen-lockfile`. Python bindings: [`docs/python.md`](docs/python.md).
- **Gate:** `tools/check-local.sh` runs the checks a pull request must pass:
  `cargo fmt --check`, `cargo clippy -D warnings`, rustdoc with `-D warnings`,
  a build of every target, and the real-file, CLI and contract checks.
  `--viewer`, `--corpus`, `--deny`, `--audit` add the optional gates.
- **How changes are verified:** against real Revit files and Revit's own
  exports, through the built CLIs, and by contract checks that keep the
  published claims honest. [`tools/ci/test-targets.txt`](tools/ci/test-targets.txt)
  classes every target under `tests/` (corpus-gated ones skip without their
  corpus); `tests/fuzz_regressions.rs` replays crash-shaped inputs on every
  pull request; libFuzzer targets are in `fuzz/`, Playwright browser tests in
  `viewer/tests/` and Python tests in `tests/python/`. The unit tests
  written before this policy were removed from the crate's source (#1126), and new ones are not
  accepted ([`CONTRIBUTING.md`](CONTRIBUTING.md) says why).
- **What "done" looks like:** the gate is green, the pull-request template's
  checklist is filled in, and any change to user-visible capability updates
  [`docs/status.md`](docs/status.md) and
  [`docs/support-matrix.json`](docs/support-matrix.json) in the same PR — the
  project does not claim what its tests cannot show.
- **Start here:** [`CONTRIBUTING.md`](CONTRIBUTING.md) walks from clone to a
  first pull request; [`docs/contribution-map.md`](docs/contribution-map.md)
  maps the larger areas; small tasks carry the
  [good first issue](https://github.com/DrunkOnJava/rvt-rs/labels/good%20first%20issue)
  label.

## License and trademarks

- **Code:** Apache License 2.0. See [`LICENSE`](LICENSE) for the full
  text and [`NOTICE`](NOTICE) for attribution.
- **Trademarks:** "Autodesk" and "Revit" are registered trademarks of
  Autodesk, Inc. rvt-rs is an independent project, **not affiliated
  with, endorsed by or sponsored by Autodesk**; it uses the names only to
  identify the file format it reads.
- **Provenance:** rvt-rs contains no Autodesk source code. Its knowledge of
  the format comes from the bytes of publicly available files, the exported
  symbol names of the public `RevitAPI.dll` NuGet package and previously
  published reverse-engineering work. buildingSMART's IFC specifications
  guide the IFC mapping, and Autodesk's open-source exporter `revit-ifc`
  (LGPL) is read, never copied, for what Revit's own IFC export writes, and
  never for the file format.
  [`CLEANROOM.md`](CLEANROOM.md) lists the sources contributions may use and
  the declaration a format finding carries, and [`NOTICE`](NOTICE) gives the
  project's notices. This section describes how the project is built; it is
  not legal advice.
