# rvt-rs

**Open reader for Autodesk Revit files (`.rvt`, `.rfa`, `.rte`, `.rft`) — no Autodesk software required.**

Apache-2.0 licensed. Rust 2024 edition. Verified against 11 Revit releases (2016-2026) with a real RevitAPI.dll native-symbol grep (100% field-name accuracy on the validation set).

## Why this exists — the openBIM gap

For over a decade the openBIM community — anchored by [buildingSMART International](https://www.buildingsmart.org/) and the IFC standard — has worked to break Autodesk's Revit lock-in. The official answer, Autodesk's [revit-ifc](https://github.com/Autodesk/revit-ifc) exporter, runs **inside** Revit using the Revit API, so it can only emit what the API chooses to expose. That's why real-world IFC exports from Revit are described, routinely and publicly, as *"very limited"* (thinkmoult.com), *"data loss"* (Reddit r/bim), and *"out of the box, just crap"* (the [OSArch Wiki's guide to Revit for openBIM](https://wiki.osarch.org/index.php?title=Revit_setup_for_OpenBIM)). BIM coordinators have spent years working around lossy IFC exports — private families, complex assemblies, proprietary parameter types, and internal geometric relationships are all dropped by the API-surface exporter.

`rvt-rs` reads the actual on-disk RVT bytes. That is a *strict superset* of what Revit's API exposes. An rvt-rs → IFC pipeline built on top of it (planned; not yet shipped) is the full-fidelity path to IFC that the openBIM movement has been waiting for — and a natural partner for [IfcOpenShell](https://ifcopenshell.org/), [BIMvision](https://bimvision.eu/), and anyone participating in buildingSMART's annual openBIM Hackathon.

## Quick demo

One command produces the full forensic picture — identity, upgrade history, format anchors, schema table, Phase D link histogram, content metadata, and a disclosure scan:

```bash
cargo build --release
./target/release/rvt-analyze --redact path/to/your.rfa
```

**Sample output** (pre-scrubbed, committed for review):

- Terminal report: [`docs/demo/rvt-analyze-2024-redacted.txt`](docs/demo/rvt-analyze-2024-redacted.txt) — 130 lines of structured output
- JSON report:    [`docs/demo/rvt-analyze-2024-redacted.json`](docs/demo/rvt-analyze-2024-redacted.json) — machine-readable version
- Tag-drift heatmap: [`docs/data/tag-drift-heatmap.svg`](docs/data/tag-drift-heatmap.svg) — visual proof of class-ID drift across 11 Revit releases

The `--redact` flag (on by default in every committed artifact) scrubs Windows usernames, Autodesk-internal paths, and project-ID folder names to `<redacted>` markers while preserving path shape so claims remain verifiable. Omit the flag when running privately against your own files.

## Results at a glance

Running the shipped CLIs against one 400 KB RFA fixture:

- **Metadata**: version, build tag, creator path, file GUID, locale (`rvt-info`)
- **Atom XML**: title, OmniClass code, taxonomies (`rvt-info` parses `PartAtom`)
- **Preview**: clean PNG thumbnail, 300-byte Revit wrapper stripped (`rvt-info --extract-preview`)
- **Schema**: 395 classes + 1,156 fields + C++ type signatures (`rvt-schema`)
- **History**: every Revit release that ever saved this file (`rvt-history`)
- **Bulk strings**: 3,746 length-prefixed UTF-16LE records from Partitions/NN — Autodesk unit/spec/parameter-group identifiers, OmniClass + Uniformat codes, Revit category labels, localized format strings (`rvt-history --partitions`)

The 34 field names and ~400 classes that `rvt-schema` extracts were cross-validated:

- 100% (34/34) appear byte-for-byte in the raw decompressed `Formats/Latest` stream
- ~20 extracted top-level classes (ADocument, DBView, HostObj, LoadBCBase, Symbol, APIAppInfo, APropertyDouble3, ElementId, etc.) match C++ symbol names compiled into the public `RevitAPI.dll` (35 MB NuGet package) with decorated function signatures like `__cdecl NotNull<class ADocument *,void>::NotNull(class ADocument *)`

The Autodesk build path `F:\Ship\2026_px64\Source\API\RevitAPI\Objects\Elements\*.cpp` also leaks through C++ assertion strings in the DLL, confirming the schema names come from the same codebase.

## Phase D findings (what makes this project different)

Four reproducible discoveries, all documented in `docs/rvt-moat-break-reconnaissance.md` and reproducible from `examples/`:

1. **The schema indexes the data.** Class names do not appear as ASCII in `Global/Latest`; class tags from `Formats/Latest` (u16 after class name, with 0x8000 flag set) occur ~340× the uniform-random rate. The top tag, `AbsCurveGStep`, appears 19,415 times in 938 KB of decompressed Global/Latest. [`examples/link_schema.rs`]

2. **Tags drift across releases** but are stable-sort-assigned. `ADocWarnings` = 0x001b 2016→2026 because no class sorted alphabetically before it has ever been added. `AbsCurveGStep` shifted 0x0053 → 0x0066 across the decade as 19 new A-class entries were inserted. Full 122-class × 11-release drift table: [`docs/data/tag-drift-2016-2026.csv`](docs/data/tag-drift-2016-2026.csv). This is the first publicly-available version of this data. [`examples/tag_drift.rs`]

3. **Revit 2021 was a major undocumented format transition.** Global/Latest grew 27× (~26 KB → ~715 KB) while simultaneously the Forge Design Data Schema namespaces (`autodesk.unit.*`, `autodesk.spec.*`) debuted in Partitions/NN. Two symptoms, one event. Any reader built for 2016-2020 silently drops 30× more data when pointed at 2021+.

4. **Parameter-group namespace shipped separately in Revit 2024.** `autodesk.parameter.group.*` identifiers appear in 2024+ only — three releases after units/specs. Dating the Forge schema rollout from on-disk bytes: [`examples/tag_drift.rs`](examples/tag_drift.rs), [`src/object_graph.rs`](src/object_graph.rs).

Two unintended disclosure patterns also surfaced in Autodesk's shipped reference content — details are withheld from this README to avoid re-broadcasting them; they are documented in [`docs/rvt-moat-break-reconnaissance.md`](docs/rvt-moat-break-reconnaissance.md) for security-research reproducibility. They are:

- A customer-facing OneDrive path that leaks the directory structure of an Autodesk employee's personal sample-authoring workflow.
- A build-server path baked into C++ assertion strings inside the public `RevitAPI.dll`.

**Downstream safety:** the `rvt-analyze` CLI ships with a `--redact` flag (on by default for any of the committed demo output in this repo) that rewrites creator paths, Autodesk-internal paths, and build-server paths to `<redacted>` markers while preserving the surrounding structure. Any tool consuming rvt-rs output and displaying it publicly should do the same.

---

## What works today

- Open any Revit file from disk or memory (magic `D0 CF 11 E0 A1 B1 1A E1`)
- Enumerate every OLE stream
- Parse `BasicFileInfo` → version, build, GUID, creator path, locale
- Parse `PartAtom` XML → title, ID, OmniClass code, taxonomies, categories
- Extract `RevitPreview4.0` → clean PNG thumbnail (skips the 300-byte Revit wrapper)
- Decompress any stream (truncated-gzip format — standard gzip header, no trailing CRC/ISIZE)
- Extract class/schema inventory from `Formats/Latest` (8-10K class names per file)
- Find the version-specific `Partitions/NN` stream (58, 60-69 for 2016-2026, skipping 59)

Seven CLIs ship in the box:

```bash
cargo build --release

# One-shot forensic analysis — all subsystems in one report
./target/release/rvt-analyze --redact my-project.rvt
./target/release/rvt-analyze --redact --json my-project.rvt > report.json

# Quick metadata + schema summary
./target/release/rvt-info --show-classes my-project.rvt

# Machine-readable (JSON)
./target/release/rvt-info -f json my-project.rvt > meta.json

# Pull the embedded thumbnail
./target/release/rvt-info --extract-preview preview.png my-project.rvt

# Compare two versions of the same file (cross-version byte diff)
./target/release/rvt-diff --decompress 2018.rfa 2024.rfa

# Dump the full class schema (395 classes, 1156 fields)
./target/release/rvt-schema my-project.rvt

# Document upgrade history (which Revit releases have opened this file)
./target/release/rvt-history my-project.rvt

# Pull every UTF-16LE string record out of Partitions/NN
# (categories, OmniClass, Uniformat, Autodesk unit identifiers, …)
./target/release/rvt-history --partitions my-project.rvt

# Hex-dump any decompressed stream (for Phase D work)
./target/release/rvt-dump my-project.rvt --stream Global/Latest
```

A handful of reproduction/probe binaries live in `examples/`:

```bash
cargo build --release --examples

./target/release/examples/probe_link      <file>           # null-hypothesis: class names absent from Global/Latest
./target/release/examples/tag_bytes       <file>           # hex around known class names in Formats/Latest
./target/release/examples/tag_dump        <file>           # statistical sweep of post-name u16 patterns
./target/release/examples/link_schema     <file>           # tag-frequency histogram in Global/Latest (340× non-uniformity)
./target/release/examples/tag_drift       <sample-dir> <out.csv>   # per-class drift table across all 11 Revit releases
```

## Format overview

Every Revit file is a Microsoft Compound File Binary (OLE2) container with
this stream layout (constant across 11 years of Revit releases):

```
<root>
├── BasicFileInfo                 UTF-16LE metadata
├── Contents                      custom 4-byte header + DEFLATE body
├── Formats/Latest                DEFLATE — class schema inventory
├── Global/
│   ├── ContentDocuments          tiny document list
│   ├── DocumentIncrementTable    DEFLATE — change tracking
│   ├── ElemTable                 DEFLATE — element ID index
│   ├── History                   DEFLATE — edit history (GUIDs)
│   ├── Latest                    DEFLATE — current object state (17:1 ratio)
│   └── PartitionTable            DEFLATE — partition metadata
├── PartAtom                      plain XML (Atom + Autodesk partatom namespace)
├── Partitions/NN                 bulk data: 5-10 concatenated DEFLATE segments
│                                 NN = 58, 60-69 for Revit 2016-2026
├── RevitPreview4.0               custom header + PNG thumbnail
└── TransmissionData              UTF-16LE transmission metadata
```

All compressed streams use a "truncated gzip" format — the standard 10-byte
gzip header (magic `1F 8B 08 ...`) followed by raw DEFLATE, but *without*
the trailing 8-byte CRC32 + ISIZE that conforming gzip writers produce.
Python's `gzip.GzipFile` and Rust's `flate2::read::GzDecoder` both refuse
these streams. The fix is to skip the 10-byte header manually and use
`flate2::read::DeflateDecoder` on the raw body.

## Reverse engineering state

| Layer | Description | Status |
|---|---|---|
| 1 · Container | OLE2 / Microsoft Compound File ([MS-CFB]) | **Done** |
| 2 · Compression | Truncated gzip → raw DEFLATE | **Done** |
| 3 · Stream framing | Per-stream custom headers, `Partitions/NN` chunk layout, `Contents`/`Preview` wrappers | **80% mapped** (from 11-version corpus) |
| 4a · Schema table | Class names + fields + C++ type signatures from `Formats/Latest`, plus per-class tag and the cross-release tag-drift map | **Done** |
| 4b · Schema→data link | Tags from `Formats/Latest` occur at ~340× the noise rate in `Global/Latest`; schema IS the live type dictionary for the object graph | **Done** |
| 4c · Object records | Record framing, field encoding (double / int / ElementId / std::pair / std::vector / std::map), alignment, inter-instance references | **In progress** |
| 5 · IFC export | rvt-rs → IfcOpenShell bridge, buildingSMART certification | **Planned** |

Layer 4c is where the remaining focused work is — and it's now narrow,
incremental bit-level hypothesis testing against the 11-version corpus,
not a multi-year effort. Every unknown has a specific falsifiable test:

- *"Does a class record start with `[u16 tag][u32 length]`?"* — take the
  `link_schema` histogram's top tag, look at the bytes around each
  occurrence, compare 11 versions of the same field.
- *"Is double encoded as 8-byte IEEE 754 little-endian?"* — pick a class
  with `double` fields (`APropertyDouble3` has three), find its instances,
  compare extracted values to values visible through the Revit API.

The full analysis narrative lives in [`docs/rvt-moat-break-reconnaissance.md`](docs/rvt-moat-break-reconnaissance.md)
with four dated addenda covering Phase D link proof, Forge schema dating,
the 2021 format transition, and the 122-class tag drift table.

## Sample corpus

Integration tests run against 11 versions of Autodesk's public
`rac_basic_sample_family` RFA fixture (one per Revit release from 2016
through 2026). These are distributed via Git LFS in the `phi-ag/rvt`
repository. To pull them:

```bash
cd /path/to/rvt-recon/samples
git clone https://github.com/phi-ag/rvt.git _phiag
cd _phiag && git lfs pull
cd .. && cp _phiag/examples/Autodesk/*.rfa .
```

The integration tests in `tests/samples.rs` skip any year whose RFA file
is absent, so partial corpora are okay — you'll just see
`skipping 2024: sample not present` messages.

## Design choices

- **cfb crate over custom OLE parser** — the `cfb` crate is mature,
  tested against Office documents, and handles both short and regular
  sectors. Faster than writing our own.
- **flate2 over miniz_oxide direct** — `flate2` wraps both `miniz_oxide`
  (pure Rust) and libz backends. We pick the default pure-Rust build to
  avoid a C toolchain dependency.
- **quick-xml over xml-rs** — ~3x faster, zero-copy friendly, and the
  `.from_str` + event-loop pattern is closer to what Go/Python parsers do.
- **encoding_rs over stdlib** — Revit's UTF-16LE streams sometimes have
  malformed pairs at boundaries (single-byte markers get interleaved).
  `encoding_rs` recovers gracefully where stdlib panics.
- **BTreeSet for class names** — deterministic ordering in output (plus
  sorted JSON) matters for diffable CLI output.

## Not yet implemented

The things below are tractable but deferred. Issues / PRs welcome.

- Record framing inside `Global/Latest` — we know tags delimit instance
  classes, still need to recognise the boundary between adjacent records
- Per-field byte decoding (see Layer 4c in the previous section)
- `Global/ElemTable` element-by-element parse — we decompress but don't
  parse individual element records
- `Partitions/NN` internal chunk headers — there's a ~44-byte prefix then
  5-10 concatenated gzips; the prefix encodes chunk offsets + sizes
- IFC export (Layer 5 — natural successor once Layer 4c is done)
- Write path — fully dependent on Layer 4c

## Running the tests

```bash
cargo test --release
```

Expected output (as of commit `f043e72`):

```
test result: ok. 21 passed; 0 failed   (unit tests, in-tree)
test result: ok.  8 passed; 0 failed   (integration tests, 11-version corpus)
```

Integration tests are skipped if the sample RFAs are absent.

## License

Apache-2.0. See `LICENSE`. Autodesk is not affiliated with this project.
"Revit", "Autodesk", "DWG", and related marks are trademarks of Autodesk, Inc.
This is a clean-room reimplementation under the interoperability exception of
17 U.S.C. § 1201(f) and the Autodesk v. ODA settlement (2006).
