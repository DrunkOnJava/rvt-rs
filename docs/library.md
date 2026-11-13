# Library and tools

The command-line tools, the Rust library's modules, the research probes, and a longer account of the browser viewer. The [README](../README.md) carries the short version of each: its "Install and quick start", "In the browser" and "Command-line tools" sections. These sections are moved here verbatim.

## Quick demo

One command produces a full file report — identity, upgrade history, format anchors, schema table, schema-to-data link histogram, content metadata, and a disclosure scan:

```bash
cargo build --release
./target/release/rvt-analyze --redact path/to/your.rfa
```

### From Python

```python
import rvt

f = rvt.RevitFile("my-project.rfa")
print(f.version, f.part_atom_title)      # 2024 "0610 x 0915mm"
print(f.read_adocument()["fields"][-1])  # {name: m_devBranchInfo, kind: element_id, tag: 0, id: 35}
open("out.ifc", "w").write(f.write_ifc())
```

Install: `pip install rvt` — or build from source with [`maturin build --release --manifest-path rvt-py/Cargo.toml`](python.md#from-source). Full API + Jupyter notebook walkthrough: [`docs/python.md`](python.md) and [`docs/rvt-python-quickstart.ipynb`](rvt-python-quickstart.ipynb).
See [`docs/install.md`](install.md) for cargo, PyPI, source, and viewer
install/smoke-test paths, and for the prebuilt Linux / macOS / Windows CLI
archives each GitHub Release carries (from [v0.2.0](https://github.com/DrunkOnJava/rvt-rs/releases/tag/v0.2.0) on).

### In the browser

Drop a `.rvt` / `.rfa` / `.rte` / `.rft` at <https://drunkonjava.github.io/rvt-rs/>: the file is read and converted inside the tab and is not uploaded. The viewer runs the core library compiled to WebAssembly by the `rvt-wasm` crate (`wasm-pack build rvt-wasm --target web --out-dir ../viewer/pkg --out-name rvt`, as [`docs/install.md`](install.md) describes), parses in a dedicated worker, and renders 3D with Three.js. One-click buttons export the model as **glTF 2.0 binary**, **IFC4 STEP**, or **plan-view SVG**. URL state (camera pose + category filters) is shareable via the hash fragment. Multi-megabyte projects fit inside wasm32's 4 GiB linear-memory ceiling: each gzip member reserves at most 1 MiB before decoding and the decoded buffer is trimmed (#256), which is what lets the 33.7 MB Core Interior demo finish at 419 MB of linear memory instead of growing to 4057 MB in 139 ms and trapping. A Rust panic in the wasm build is reported through `console.error` with its message and source location instead of a bare `RuntimeError: unreachable`.

The landing dropzone also includes a **demo gallery** staged from [`docs/viewer-demos.json`](viewer-demos.json) (license/provenance + expected quality labels). It opens two real projects, `Revit_IFC5_Einhoven.rvt` (2023) and `2024_Core_Interior.rvt` (2024) from the MIT-licensed [magnetar-io/revit-test-datasets](https://github.com/magnetar-io/revit-test-datasets), hash-verified at staging time, alongside the tier1 synthetics. Einhoven is 913 KB and opens in about half a second; Core Interior is 33.7 MB and decodes in about 3 seconds on the deployed site since #266 (about 7 seconds including the 32 MB download), reporting 889 entities and 854 elements carrying geometry. Two Playwright tests gate both cards, and staging refuses a file whose sha256 does not match the catalog. Demo bytes are same-origin static assets only.

The deploy workflow (`.github/workflows/deploy-viewer.yml`) and the viewer tests (`.github/workflows/viewer-test.yml`, part of `CI passed`) check the viewer's privacy posture on every build, in two ways. `viewer/scripts/audit-wasm-imports.mjs` reads each import of the compiled `.wasm` and its shim in the JavaScript glue, and fails if any reaches `fetch`, `XMLHttpRequest`, `WebSocket`, `EventSource` or `sendBeacon` (the check before #735 could not fail). A Playwright test opens a real project in the built viewer and fails on any request to another origin, or on any request at all after the file is open. The page's Content-Security-Policy also limits its connections to its own origin. Together these cover the WebAssembly module's imports, the page's connection policy and the flow the test drives; they are checks, not a proof about every possible code path. [`docs/viewer-privacy-posture.md`](viewer-privacy-posture.md) states the invariant and what each check covers.

### Supported MVP workflow

The supported end-to-end shell is intentionally honest about partial decode:

1. Open a supported Revit file locally (drop, file picker, or a redistributable gallery demo).
2. Read the File status / confidence panel before trusting geometry.
3. Inspect decoded entities in the scene tree or by picking in the 3-D view.
4. Export IFC / glTF / plan only after checking the export-quality label.
5. Download diagnostics when the export is scaffold-only or partial.

What still depends on decoder work: files from before Revit 2008, which are not read; 2008 to 2022's, 2026's and 2027's element records, which are experimental (2008 to 2014 outside the 1.0 range); and on 2023 geometry beyond each element's box, wall joins and beam cuts, layered or angled joins, design options and IFC export overrides; roofs by extrusion outside Revit 2025 or over a T or U footprint, family geometry and opening profiles; most element parameters; and compound layers of elements drawn whole or sloped, which get no layer set. **RE-19 / RE-20 (2026-08-29) closed negative** on the magnetar corpora: there is no Door vs Window discriminator *in the opening-index bytes* and no recoverable Level ElementId map there, so do not re-probe those without a new corpus or signal. Typed categories come from each element record's `BuiltInCategory` instead (RE-21 onward). On Revit 2008 to 2022, 2026 and 2027 the export is experimental: typed elements as listed [by input](evidence.md#what-you-can-rely-on-by-input), whose rows say what each release is measured against; on 2023 elements are their boxes with the joins and properties listed there. See [`docs/status.md`](status.md) and [`docs/supported-profile.md`](supported-profile.md).

**Sample output** (all pre-scrubbed with `--redact`, committed for review):

- **One-screen teaser**: [`docs/demo/rvt-analyze-2024-teaser.txt`](demo/rvt-analyze-2024-teaser.txt) — the four highlight sections fit in one terminal screen (identity, format anchors, schema-to-data linkage, disclosure scan)
- **Full terminal report**: [`docs/demo/rvt-analyze-2024-redacted.txt`](demo/rvt-analyze-2024-redacted.txt) — 130 lines of structured output
- **JSON report**:    [`docs/demo/rvt-analyze-2024-redacted.json`](demo/rvt-analyze-2024-redacted.json) — machine-readable version
- **Tag-drift heatmap**: [`docs/data/tag-drift-heatmap.svg`](data/tag-drift-heatmap.svg) — visual proof of class-ID drift across 11 Revit releases

The `--redact` flag (on by default in every committed artifact) scrubs Windows usernames, Autodesk-internal paths, and project-ID folder names to `<redacted>` markers while preserving path shape so claims remain verifiable. Omit the flag when running privately against your own files.

## Library surface

Three runnable examples show the library end to end: [`basics_inspect`](../examples/basics_inspect.rs) reads what a file is, [`basics_elements`](../examples/basics_elements.rs) lists its building elements by IFC type with their ElementIds, and [`basics_export`](../examples/basics_export.rs) writes IFC4, glTF, a CSV schedule and the export diagnostics. `cargo run --example basics_inspect -- model.rvt` runs one on your file, and CI runs all three on the synthetic project the repository ships and on a real one, Einhoven (MIT). The other programs under `examples/` are research probes ([`examples/README.md`](../examples/README.md)).

All modules build for native targets and for `wasm32-unknown-unknown`, which the `rvt-wasm` crate compiles for the browser viewer. The type documentation is on [docs.rs](https://docs.rs/rvt):

| Module | What it does |
|---|---|
| `reader` | Open any Revit file with `OpenLimits`, enumerate every OLE stream, fetch raw stream bytes, bounded reads |
| `compression` | Truncated-gzip decode (`inflate_at`, `inflate_at_auto`, `inflate_at_with_limits`) + multi-chunk (`inflate_all_chunks_with_limits`) + truncated-gzip encoder for write-back (`truncated_gzip_encode`) |
| `basic_file_info` | Version, build tag, GUID, creator path, locale — read path + byte-back encoder (`BasicFileInfo::encode`) |
| `part_atom` | Atom XML with Autodesk `partatom` namespace — title, OmniClass, taxonomies — read + encode |
| `formats` | Parse + encode `Formats/Latest` with `FieldType` classification (all but 2 or 3 fields per release classified across the 12-release corpus, #410, RE-323; 2027's flagged fields by their base type, RE-264) |
| `walker` | Schema-directed instance walker + generic `decode_instance` + `detect_adocument_start` entry-point finder; candidates of the 7 classes in `elements::MVP_TYPED_CLASSES` go through their typed decoder from the registry (fail closed), and every other class decodes generically |
| `elements` | 81 `ElementDecoder` registry entries in `all_decoders()`, all `SimpleDecoder` (Wall, Floor, Door, Window, Column, Beam, Stair, Railing, Rebar, Room, Furniture, …), and typed views of Level, Wall, Floor, Door, Window, Room and Material; the walker uses the 7 `MVP_TYPED_CLASSES` decoders, and no test exercises the rest (#573). Real project elements come from partition element records |
| `geometry` | `Point3`, `Curve` (Line, Arc, Circle, Ellipse, Hermite spline, NURBS, cylindrical helix) and `CurveLoop`, and `recovery`, which reads wall location curves, floor boundaries, opening hosts and level elevations from the typed views |
| `object_graph` | `DocumentHistory`, string-record extractor for Global/Latest + Partitions/NN |
| `class_index` | Quick class-name inventory (BTreeSet) |
| `corpus` | Cross-version byte-delta classifier |
| `elem_table` | `Global/ElemTable` header and records: the declared ElementIds and each record's owner, in 28-byte records through Revit 2023 and 40-byte records from 2024 (#152, RE-41) |
| `partitions` | Partitions/NN 44-byte header decoder + gzip-chunk splitter |
| `writer` | Byte-preserving round-trip `copy_file` + `write_with_patches` (atomic temp-file rename, stream-hash verification) + GUID + history preservation |
| `round_trip` | Per-class encoder round-trip verification (`verify_instance_round_trip`) |
| `ifc` | Full IFC4 spatial tree + elements + materials + properties + openings + extrusion geometry + glTF 2.0 binary (`gltf::model_to_glb`) + plan-view SVG (`sheet::render_plan_svg`) + viewer data model (`scene_graph`, `camera`, `clipping`, `sheet`, `share`, `pbr`) |
| `streams` | Named constants for every invariant OLE stream in a Revit file |
| `redact` | Shared PII scrubbers for all CLIs (`--redact` flag) |
| (`rvt-wasm` crate) | the JS-callable wasm-bindgen bindings powering the browser viewer, which [`docs/viewer-api.md`](viewer-api.md#the-javascript-bindings) lists, a crate of its own so `rvt` builds as a plain `rlib` |
| `error` | Structured error type (`Error` / `Result`) |

Runtime capabilities:

- Open any Revit file from disk (magic `D0 CF 11 E0 A1 B1 1A E1`)
- Enumerate every OLE stream; find the version-specific `Partitions/NN`
- Decompress any stream (truncated-gzip format — standard gzip header, no trailing CRC/ISIZE)
- Parse `BasicFileInfo`, `PartAtom`, extract preview PNG
- Read every tagged class record in `Formats/Latest` with its tag, parent and declared field count
- Decode a family file's 167-byte `Global/PartitionTable`, including its stable format-identifier GUID
- Decode a family file's 307-byte `Contents` stream, including its embedded UTF-16LE metadata chunk
- Produce a byte-for-byte round-trip copy of a `.rfa` / `.rvt` file
- Run across the 11-release family corpus in under 500 ms per file (release build, measured in April 2026; for large projects see [Performance on large projects](format.md#performance-on-large-projects))

The CLIs that ship are the ones the [README](../README.md#command-line-tools) lists. Examples:

```bash
cargo build --release

# One-shot file report — all subsystems in one report
./target/release/rvt-analyze --redact my-project.rvt
./target/release/rvt-analyze --redact --json my-project.rvt > report.json

# Quick metadata + schema summary
./target/release/rvt-info --show-classes my-project.rvt

# Machine-readable (JSON)
./target/release/rvt-info -f json my-project.rvt > meta.json

# Element and room schedules for Excel / Sheets / LibreOffice
./target/release/rvt-schedule my-project.rvt
./target/release/rvt-schedule my-project.rvt --schedule rooms --metric --excel

# Inventory every Revit file under a folder: release, worksharing, last saved
./target/release/rvt-info projects/
./target/release/rvt-info projects/ -f csv --redact > inventory.csv

# Pull the embedded thumbnail
./target/release/rvt-info --extract-preview preview.png my-project.rvt

# Plain-language file health and IFC export readiness
./target/release/rvt-inspect my-project.rvt
./target/release/rvt-inspect my-project.rvt --json

# Compare two versions of the same file (cross-version byte diff)
./target/release/rvt-diff --decompress 2018.rfa 2024.rfa

# Two saves of one project, element by element: what was added, removed or
# changed (class, storey, type, material, placement, body, properties), keyed
# by the GlobalId the IFC export writes (#1843). Where in the file rvt-rs read
# an element (its stream, its offsets) is not compared: a later save moves it.
# Compare with one build: a change in how rvt-rs reads a file shows as a change
# too.
./target/release/rvt-diff --elements monday.rvt tuesday.rvt --json changes.json

# Dump the full class schema (4,126 classes and 11,562 fields on a Revit 2024 file)
./target/release/rvt-schema my-project.rvt

# Document upgrade history (which Revit releases have opened this file)
./target/release/rvt-history my-project.rvt

# Pull every UTF-16LE string record out of Partitions/NN
# (categories, OmniClass, Uniformat, Autodesk unit identifiers, …)
./target/release/rvt-history --partitions my-project.rvt

# Hex-dump every decompressed stream (for format research)
./target/release/rvt-dump my-project.rvt

# IFC4 STEP export — everything decoded; the default mode accepts a scaffold-only result, with warnings
./target/release/rvt-ifc my-project.rvt -o out.ifc

# Require a stronger quality gate before writing IFC: exits non-zero, writing nothing,
# while the diagnostics list unsupported features (true of every reference model today)
./target/release/rvt-ifc my-project.rvt -o out.ifc --mode strict

# IFC4 export with a shareable JSON readiness/support sidecar
./target/release/rvt-ifc my-project.rvt -o out.ifc --diagnostics out.diagnostics.json

# Diagnostic IFC export — include low-confidence proxy candidates with provenance
./target/release/rvt-ifc my-project.rvt -o diagnostic.ifc --diagnostic-proxies

# Compare an rvt-rs IFC export against a Revit (or other) reference IFC
./target/release/rvt-ifc-compare out.ifc revit-reference.ifc --json /tmp/ifc-compare.json

# glTF 2.0 binary export — loads in Three.js / Blender / any glTF viewer
./target/release/rvt-gltf my-project.rvt -o out.glb

# 2D plan-view SVG — per-category colours, ready for plot/laser-cut/printing
./target/release/rvt-sheet my-project.rvt -o out.svg

# Global/ElemTable dump — declared element-ids + record layout (family 12B / project 28B/40B)
./target/release/rvt-elem-table my-project.rvt --limit 20

# Production decoded elements / class counts (JSON; mirrors Python element_counts)
./target/release/rvt-elements my-project.rvt --counts

# Stream-level write path — patch named OLE streams via JSON manifest
./target/release/rvt-write my-project.rvt --patches patches.json -o patched.rvt

# ADocument's instance fields, as text or JSON
./target/release/rvt-doc my-project.rvt > doc.txt

# Cross-version corpus analysis (12 releases in one pass; 3 or more files)
./target/release/rvt-corpus /path/to/corpus-dir/*.rfa

# Triage a folder of files into failure buckets
./target/release/rvt-corpus doctor /path/to/corpus-dir --json
```

Every format finding has a probe under [`probes/examples/`](../probes/README.md), whose module documentation states the fact it proves and how to check it; the `probe_re*.rs` files carry the numbered findings in `reports/element-framing/`. The probes of the first reconnaissance:

```bash
cargo build --release --examples -p rvt -p rvt-probes

# --- schema ↔ data linkage ---
./target/release/examples/probe_link              <file>           # null-hypothesis: class names absent from Global/Latest
./target/release/examples/tag_bytes               <file>           # hex around known class names in Formats/Latest
./target/release/examples/tag_dump                <file>           # statistical sweep of post-name u16 patterns
./target/release/examples/link_schema             <file>           # tag-frequency histogram in Global/Latest (340× non-uniformity)
./target/release/examples/tag_drift               <sample-dir> <out.csv>   # per-class drift table 2016-2026
./target/release/examples/tag_drift_svg           <in.csv> <out.svg>       # render drift table as colour-coded SVG heatmap

# --- record framing ---
./target/release/examples/record_framing          <file>           # dump bytes at tagged-class defs + first tag occurrence
./target/release/examples/elem_table_probe        <sample-dir>     # Global/ElemTable structural sweep across releases
./target/release/examples/partitions_header_probe <sample-dir>     # 44-byte Partitions/NN header + chunk offsets
./target/release/examples/contents_probe          <file>           # Contents stream decoder (creator name + build tag)

# --- stable anchors ---
./target/release/examples/partition_invariant     <sample-dir>     # find 165-byte invariant in Global/PartitionTable
./target/release/examples/partition_diff          <sample-dir>     # show the 2 varying bytes per release
./target/release/examples/partition_full          <file>           # full annotated hex dump + UUID decode

# --- write path ---
./target/release/examples/roundtrip                                # copy 2024 sample, verify all 13 streams identical
```
