# rvt-rs

**Open reader for Autodesk Revit files (`.rvt`, `.rfa`, `.rte`, `.rft`) — no Autodesk software required.**

Apache-2.0 licensed. Rust 2024 edition. Verified against 11 Revit releases (2016-2026) with a real RevitAPI.dll native-symbol grep (100% field-name accuracy on the validation set). **28 unit tests + 8 integration tests pass on the full 11-version corpus.** Seven CLIs ship in the box (`rvt-analyze`, `rvt-info`, `rvt-schema`, `rvt-history`, `rvt-diff`, `rvt-corpus`, `rvt-dump`), plus fourteen reproducible probes under `examples/` that cover every claim in this README.

### TL;DR — what's in the box

- **No Autodesk dependency.** Reads .rvt / .rfa / .rte / .rft directly from bytes. Zero API calls, zero cloud, zero Revit install.
- **Every Revit release 2016-2026.** 11-year corpus in CI; integration tests run on each.
- **Phase D moat proved:** class tags from `Formats/Latest` occur in `Global/Latest` at **~340× uniform-random rate** (`AbsCurveGStep` = 19,415 hits in 938 KB). Schema IS the live type dictionary.
- **First public RVT tag-drift table** — 122 classes × 11 releases, CSV + SVG heatmap.
- **First publicly-documented Revit format-identifier GUID** — `3529342d-e51e-11d4-92d8-0000863f27ad`, stable in 98.8% of `Global/PartitionTable` bytes since 2000.
- **Revit 2021 was a silent format rewrite** — Global/Latest grew 27× and Forge Design Data Schema namespaces debuted. No public changelog mentions this.
- **Byte-preserving round-trip writer works today** — 13/13 streams identical after read-modify-write.
- **Hands-clean by default.** Every CLI has `--redact` that scrubs usernames, Autodesk-internal paths, and project IDs while preserving path shape so claims remain verifiable. Committed demo output is pre-scrubbed.

## Why this exists — the openBIM gap

For over a decade the openBIM community — anchored by [buildingSMART International](https://www.buildingsmart.org/) and the IFC standard — has worked to break Autodesk's Revit lock-in. The official answer, Autodesk's [revit-ifc](https://github.com/Autodesk/revit-ifc) exporter, runs **inside** Revit using the Revit API, so it can only emit what the API chooses to expose. That's why real-world IFC exports from Revit are described, routinely and publicly, as *"very limited"* (thinkmoult.com), *"data loss"* (Reddit r/bim), and *"out of the box, just crap"* (the [OSArch Wiki's guide to Revit for openBIM](https://wiki.osarch.org/index.php?title=Revit_setup_for_OpenBIM)). BIM coordinators have spent years working around lossy IFC exports — private families, complex assemblies, proprietary parameter types, and internal geometric relationships are all dropped by the API-surface exporter.

`rvt-rs` reads the actual on-disk RVT bytes. That is a *strict superset* of what Revit's API exposes. An rvt-rs → IFC pipeline built on top of it (exporter scaffolded in [`src/ifc/`](src/ifc/); full emission gated on Layer 4c completion) is the full-fidelity path to IFC that the openBIM movement has been waiting for — and a natural partner for [IfcOpenShell](https://ifcopenshell.org/), [BIMvision](https://bimvision.eu/), and anyone participating in buildingSMART's annual openBIM Hackathon.

## Quick demo

One command produces the full forensic picture — identity, upgrade history, format anchors, schema table, Phase D link histogram, content metadata, and a disclosure scan:

```bash
cargo build --release
./target/release/rvt-analyze --redact path/to/your.rfa
```

**Sample output** (all pre-scrubbed with `--redact`, committed for review):

- **One-screen teaser**: [`docs/demo/rvt-analyze-2024-teaser.txt`](docs/demo/rvt-analyze-2024-teaser.txt) — the four highlight sections fit in one terminal screen (identity, format anchors, Phase D linkage, disclosure scan)
- **Full terminal report**: [`docs/demo/rvt-analyze-2024-redacted.txt`](docs/demo/rvt-analyze-2024-redacted.txt) — 130 lines of structured output
- **JSON report**:    [`docs/demo/rvt-analyze-2024-redacted.json`](docs/demo/rvt-analyze-2024-redacted.json) — machine-readable version
- **Tag-drift heatmap**: [`docs/data/tag-drift-heatmap.svg`](docs/data/tag-drift-heatmap.svg) — visual proof of class-ID drift across 11 Revit releases

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

Six reproducible discoveries, all documented in [`docs/rvt-moat-break-reconnaissance.md`](docs/rvt-moat-break-reconnaissance.md) and reproducible from `examples/`:

1. **The schema indexes the data.** Class names do not appear as ASCII in `Global/Latest`; class tags from `Formats/Latest` (u16 after class name, with 0x8000 flag set) occur ~340× the uniform-random rate. The top tag, `AbsCurveGStep`, appears 19,415 times in 938 KB of decompressed Global/Latest. [`examples/link_schema.rs`]

2. **Tags drift across releases** but are stable-sort-assigned. `ADocWarnings` = 0x001b 2016→2026 because no class sorted alphabetically before it has ever been added. `AbsCurveGStep` shifted 0x0053 → 0x0066 across the decade as 19 new A-class entries were inserted. Full 122-class × 11-release drift table: [`docs/data/tag-drift-2016-2026.csv`](docs/data/tag-drift-2016-2026.csv), visualised in [`docs/data/tag-drift-heatmap.svg`](docs/data/tag-drift-heatmap.svg). First publicly-available version of this data. [`examples/tag_drift.rs`]

3. **Revit 2021 was a major undocumented format transition.** Global/Latest grew 27× (~26 KB → ~715 KB) while simultaneously the Forge Design Data Schema namespaces (`autodesk.unit.*`, `autodesk.spec.*`) debuted in Partitions/NN. Two symptoms, one event. Any reader built for 2016-2020 silently drops 30× more data when pointed at 2021+.

4. **Parameter-group namespace shipped separately in Revit 2024.** `autodesk.parameter.group.*` identifiers appear in 2024+ only — three releases after units/specs. Dating the Forge schema rollout from on-disk bytes: [`examples/tag_drift.rs`](examples/tag_drift.rs), [`src/object_graph.rs`](src/object_graph.rs).

5. **The stable Revit format-identifier GUID.** `Global/PartitionTable` is 167 bytes decompressed, and **165 of those bytes are byte-for-byte identical across every Revit release 2016-2026** (98.8% invariant). The invariant region contains a never-before-published UUIDv1: `3529342d-e51e-11d4-92d8-0000863f27ad`. The MAC suffix `0000863f27ad` matches a known Autodesk-dev-workstation signature from circa 2000. Useful as a magic number for file-type sniffers that can't distinguish RVT from other CFB containers. [`examples/partition_full.rs`]

6. **Tagged class record structure decoded.** Every class declaration in `Formats/Latest` carries an explicit tag (u16 with 0x8000 flag), optional parent class, and declared field count, followed by N field records each with name + C++ type encoding. `HostObjAttr` now resolves to `{tag=107, parent=Symbol, declared_field_count=3}` with all three field names (`m_symbolInfo`, `m_renderStyleId`, `m_previewElemId`) extracted byte-for-byte. [`examples/record_framing.rs`, `src/formats.rs`]

Three unintended disclosure patterns also surfaced in Autodesk's shipped reference content — the specific values are withheld from this README to avoid re-broadcasting them; they are documented in [`docs/rvt-moat-break-reconnaissance.md`](docs/rvt-moat-break-reconnaissance.md) for security-research reproducibility:

- A customer-facing OneDrive path that leaks the directory structure of an Autodesk employee's personal sample-authoring workflow.
- A build-server path baked into C++ assertion strings inside the public `RevitAPI.dll`.
- A creator-name field inside the `Contents` stream that travels with every copy of the sample family, preserving the name of one of Revit's original 1997 developers.

**Downstream safety:** the `rvt-analyze` CLI ships with a `--redact` flag (on by default for any of the committed demo output in this repo) that rewrites creator paths, Autodesk-internal paths, and build-server paths to `<redacted>` markers while preserving the surrounding structure. Any tool consuming rvt-rs output and displaying it publicly should do the same.

---

## What works today

Library surface (all modules compile; see `src/` for type docs):

| Module | What it does |
|---|---|
| `reader` | Open any Revit file, enumerate every OLE stream, fetch raw stream bytes |
| `compression` | Truncated-gzip decode (`inflate_at`) + multi-chunk (`inflate_all_chunks`) |
| `basic_file_info` | Version, build tag, GUID, creator path, locale |
| `part_atom` | Atom XML with Autodesk `partatom` namespace — title, OmniClass, taxonomies |
| `formats` | Parse `Formats/Latest` into `{name, offset, fields, tag, parent, declared_field_count}` |
| `object_graph` | `DocumentHistory`, string-record extractor for Global/Latest + Partitions/NN |
| `class_index` | Quick class-name inventory (BTreeSet) |
| `corpus` | Cross-version byte-delta classifier |
| `elem_table` | `Global/ElemTable` header parser + rough record enumeration |
| `partitions` | Partitions/NN 44-byte header decoder + gzip-chunk splitter |
| `writer` | Byte-preserving round-trip `copy_file` through the OLE container |
| `ifc` | IFC export scaffold: `IfcModel`, `Exporter` trait, `NullExporter`, mapping plan |
| `streams` | Named constants for every invariant OLE stream in a Revit file |
| `error` | Structured error type (`Error` / `Result`) |

Runtime capabilities:

- Open any Revit file from disk (magic `D0 CF 11 E0 A1 B1 1A E1`)
- Enumerate every OLE stream; find the version-specific `Partitions/NN`
- Decompress any stream (truncated-gzip format — standard gzip header, no trailing CRC/ISIZE)
- Parse `BasicFileInfo`, `PartAtom`, extract preview PNG
- Extract **375 class records** from `Formats/Latest` with tag + parent + declared field count for every tagged class
- Decode the 167-byte `Global/PartitionTable` structure including the stable Revit format-identifier GUID
- Decode the 307-byte `Contents` stream including the embedded UTF-16LE metadata chunk
- Produce a byte-for-byte round-trip copy of any `.rfa` / `.rvt` file
- Run across the full 11-release corpus in < 500 ms per file (release build)

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

Fourteen reproducible probes live in `examples/` — one per FACT in the recon report:

```bash
cargo build --release --examples

# --- schema ↔ data linkage (Phase D) ---
./target/release/examples/probe_link              <file>           # null-hypothesis: class names absent from Global/Latest
./target/release/examples/tag_bytes               <file>           # hex around known class names in Formats/Latest
./target/release/examples/tag_dump                <file>           # statistical sweep of post-name u16 patterns
./target/release/examples/link_schema             <file>           # tag-frequency histogram in Global/Latest (340× non-uniformity)
./target/release/examples/tag_drift               <sample-dir> <out.csv>   # per-class drift table 2016-2026
./target/release/examples/tag_drift_svg           <in.csv> <out.svg>       # render drift table as colour-coded SVG heatmap

# --- record framing (Phase 4c) ---
./target/release/examples/record_framing          <file>           # dump bytes at tagged-class defs + first tag occurrence
./target/release/examples/elem_table_probe        <sample-dir>     # Global/ElemTable structural sweep across releases
./target/release/examples/partitions_header_probe <sample-dir>     # 44-byte Partitions/NN header + chunk offsets
./target/release/examples/contents_probe          <file>           # Contents stream decoder (creator name + build tag)

# --- stable anchors ---
./target/release/examples/partition_invariant     <sample-dir>     # find 165-byte invariant in Global/PartitionTable
./target/release/examples/partition_diff          <sample-dir>     # show the 2 varying bytes per release
./target/release/examples/partition_full          <file>           # full annotated hex dump + UUID decode

# --- write path (Phase 6) ---
./target/release/examples/roundtrip                                # copy 2024 sample, verify all 13 streams identical
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
| 3 · Stream framing | Per-stream custom headers, `Partitions/NN` chunk layout, `Contents` / `Preview` / `PartitionTable` wrappers | **Done** — 165/167 bytes of `PartitionTable` invariant; 44-byte `Partitions/NN` header decoded; `62 19 22 05` wrapper magic confirmed on `Contents` + `RevitPreview4.0` |
| 4a · Schema table | Class names + fields + C++ type signatures from `Formats/Latest`; per-class tag + parent + declared field count; cross-release tag-drift map | **Done** |
| 4b · Schema→data link | Tags from `Formats/Latest` occur at ~340× the noise rate in `Global/Latest`; schema IS the live type dictionary for the object graph | **Done** |
| 4c.1 · Record framing | Tagged class records in `Formats/Latest` parse into structured records (HostObjAttr → `{tag=107, parent=Symbol, declared_field_count=3}`); Global/Latest class-tag directory identified | **Done** |
| 4c.2 · Field-body decoding | Individual field bytes → typed value (`double`, `int`, `ElementId`, `std::pair<A,B>`, `std::vector<T>`, `std::map<K,V>`) | **Open — this is the remaining moat layer** |
| 4d · ElemTable | `Global/ElemTable` header parser + rough record enumeration; record semantics TBD | **Partial** |
| 5 · IFC export | `IfcModel`, `Exporter` trait, `NullExporter`, full Revit→IFC mapping plan; emission gated on 4c.2 | **Scaffolded** |
| 6 · Write path | Byte-preserving read-modify-write round-trip (13/13 streams identical); modifying-write gated on 4c.2 | **Scaffolded** |

Layer 4c.2 is where the remaining focused work is — narrow, incremental
bit-level hypothesis testing against the 11-version corpus. Every unknown
has a specific falsifiable test:

- *"What does the `0x0025` flag word mean?"* — appears in every tagged
  class's preamble; dump its value for all 79 tagged classes and look
  for the distinguishing property.
- *"Is `m_symbolInfo: 0e 02 00 00` an 8-bit type discriminator + 24-bit
  value?"* — look at the same field across HostObjAttr instances and
  see if the first byte stays 0x0e while the trailing bytes vary.
- *"Is `double` encoded as 8-byte IEEE 754 little-endian?"* — pick a
  class with `double` fields (`APropertyDouble3` has three), find its
  instances, compare extracted values to Revit-API-visible values.

The full analysis narrative lives in [`docs/rvt-moat-break-reconnaissance.md`](docs/rvt-moat-break-reconnaissance.md)
with six dated addenda covering Phase D link proof, Forge schema dating,
the 2021 format transition, the 122-class tag drift table, the
`Global/PartitionTable` decoding, and the `Contents` stream structure.
Session-length synthesis for the Phase 4c work is in
[`docs/rvt-phase4c-session-2026-04-19.md`](docs/rvt-phase4c-session-2026-04-19.md).

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

## Running the tests

```bash
cargo test --release
```

Expected output:

```
test result: ok. 28 passed; 0 failed   (unit tests, in-tree)
test result: ok.  8 passed; 0 failed   (integration tests, 11-version corpus)
```

Integration tests are skipped if the sample RFAs are absent.

## License

Apache-2.0. See `LICENSE`. Autodesk is not affiliated with this project.
"Revit", "Autodesk", "DWG", and related marks are trademarks of Autodesk, Inc.
This is a clean-room reimplementation under the interoperability exception of
17 U.S.C. § 1201(f) and the Autodesk v. ODA settlement (2006).
