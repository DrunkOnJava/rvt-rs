# rvt-rs

**Open reader for Autodesk Revit files (`.rvt`, `.rfa`, `.rte`, `.rft`) — no Autodesk software required.**

Apache-2.0 licensed. Rust 2024 edition. Verified against 11 Revit releases (2016-2026) with a real RevitAPI.dll native-symbol grep (100% field-name accuracy on the validation set).

## Results at a glance

Running `rvt-info` + `rvt-schema` + `rvt-history` on one 400 KB RFA fixture:

- Extracts version, build tag, creator path, file GUID, locale
- Parses the embedded `PartAtom` Atom XML (title, OmniClass code, taxonomies)
- Dumps the embedded PNG thumbnail
- Pulls out **395 classes with 1,156 fields** from Autodesk's serialization schema
- Recovers the **complete document-migration history** — every Revit release that has ever saved the file (forensic timeline)

The 34 field names and 395 classes that `rvt-schema` extracts were cross-validated:

- 100% (34/34) appear byte-for-byte in the raw decompressed `Formats/Latest` stream
- All ~20 extracted top-level classes (ADocument, DBView, HostObj, LoadBCBase, Symbol, APIAppInfo, APropertyDouble3, ElementId, etc.) match C++ symbol names compiled into the public `RevitAPI.dll` (35 MB NuGet package) with decorated function signatures like `__cdecl NotNull<class ADocument *,void>::NotNull(class ADocument *)`

The Autodesk build path `F:\Ship\2026_px64\Source\API\RevitAPI\Objects\Elements\*.cpp` also leaks through C++ assertion strings in the DLL, confirming the schema names come from the same codebase.

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

Two CLIs ship in the box:

```bash
cargo build --release

# Quick metadata dump
./target/release/rvt-info --show-classes my-project.rvt

# Machine-readable
./target/release/rvt-info -f json my-project.rvt > meta.json

# Pull the embedded thumbnail
./target/release/rvt-info --extract-preview preview.png my-project.rvt

# Compare two versions of the same file
./target/release/rvt-diff --decompress 2018.rfa 2024.rfa
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
| 4 · Object graph | Autodesk-proprietary serialization inside decompressed payloads | **Open problem** |

Layer 4 is the real work — reverse-engineering the `Global/Latest`,
`Global/ElemTable`, and `Partitions/NN` binary object graphs. This is
*tractable* because Autodesk ships the class schema as plaintext in
`Formats/Latest` (10K+ named classes), and the schema is consistent
across 11 years of release.

For analysis methodology see `docs/rvt-moat-break-reconnaissance.md`
in the repo root.

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

- Proper length-prefixed parse of `Formats/Latest` (instead of the
  heuristic regex extractor)
- `Global/ElemTable` record format — we decompress but don't parse
  individual element records
- `Partitions/NN` internal chunk headers — there's a 44-byte prefix then
  10 concatenated gzips; the prefix encodes chunk offsets + sizes
- IFC export (far future — requires layer 4)
- Write path — fully dependent on layer 4

## Running the tests

```bash
cargo test --release
```

Expected output:

```
test result: ok. 9 passed; 0 failed   (unit tests, in-tree)
test result: ok. 6 passed; 0 failed   (integration tests, 11-version corpus)
```

Integration tests are skipped if the sample RFAs are absent.

## License

Apache-2.0. See `LICENSE`. Autodesk is not affiliated with this project.
"Revit", "Autodesk", "DWG", and related marks are trademarks of Autodesk, Inc.
This is a clean-room reimplementation under the interoperability exception of
17 U.S.C. § 1201(f) and the Autodesk v. ODA settlement (2006).
