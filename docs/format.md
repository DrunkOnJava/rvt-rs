# Format research

How the Revit file format reads, and what was found while reading it: why the schema matters, the first reconnaissance's findings, the stream layout, the state of the reverse engineering layer by layer, the sample corpus and the design choices. These sections were part of the [README](../README.md) and are moved here verbatim. Where one says "the tables above" or "the measured tables", it means the tables in [evidence.md](evidence.md). The long-form record of the reconnaissance is [format-reconnaissance.md](format-reconnaissance.md).

## Why the schema matters

The openBIM community, around [buildingSMART International](https://www.buildingsmart.org/) and the IFC standard, has long worked on getting building data out of Revit. Autodesk's [revit-ifc](https://github.com/Autodesk/revit-ifc) exporter runs inside Revit through the Revit API, so what it writes depends on what the API exposes and on how the export is configured; the [OSArch wiki's guide to setting up Revit for openBIM](https://wiki.osarch.org/index.php?title=Revit_setup_for_OpenBIM) covers the settings that matter. rvt-rs takes an independent route: it reads the file itself, so it runs where Revit is not installed, and its output can be checked against Revit's own export of the same file.

The schema work here, decoding the whole of `Formats/Latest` and classifying all but about ten field encodings per release across 12 Revit releases (2016 to 2027), is the dictionary a byte-level reader needs. The aim is an export that can also carry data the Revit API does not surface. Today the export reaches what the tables above measure, and the rest is open work ([`ROADMAP.md`](../ROADMAP.md) and the [issues](https://github.com/DrunkOnJava/rvt-rs/issues)).

If you are building BIM or AEC tooling and want an Apache-2.0 Revit reader to build on:

- **Revit 2016 to 2027 files**: container open, stream decompression, metadata, previews, schema introspection (the whole schema, all but about ten field encodings per release classified) and stream-level byte-preserving writes. Revit 2008 to 2015 files open too, with less measured ([by input](evidence.md#what-you-can-rely-on-by-input)).
- **Revit 2024 and 2025 projects** — typed elements with Revit's ElementIds, names, types, storeys, materials, layers and GlobalIds, as IFC4, glTF 2.0, plan SVG and CSV, each export with a diagnostics sidecar naming what was approximated or left out.
- **Revit 2023 projects** — typed elements with Revit's ElementIds and categories, each drawn as its bounding box (RE-81), walls cut back at their joins (RE-120), beams at their columns (RE-122), rooms as their outline (RE-102), on the storeys of Revit's own Levels (RE-107), family instances named as Revit names them (RE-109), walls, floors and roofs with their type, read from its layers (RE-111, RE-112).
- **Revit 2026 projects, experimentally (since 0.5.0)**: typed elements, storeys, rooms, doors and windows, type names and layer sets, measured on one project (RE-124).
- **Revit 2015 to 2022 and 2027 projects, experimentally (since 0.6.0)**: typed elements, measured against the same sample projects saved by Revit 2019 (for 2015 to 2018), 2023 (for 2019 to 2022) and 2026 (for 2027); Revit's own export is measured on one 2015 project (RE-198) and one 2018 project (RE-315) and on none of the others (RE-179 to RE-181).
- **Revit 2008 to 2014 projects, experimentally and outside the 1.0 range (2015 to 2027)**: typed elements, measured on a local copy of Autodesk's 2013 `rac_advanced` sample against its 2015 copy (RE-259); 2008 to 2010 only on their own files' consistency, with storeys and named materials (RE-260). The pre-2015 files are closed source and measured locally only.
- **Not yet**: element records of Revit 2007 and earlier, most parameters, family geometry, and semantic editing of a Revit file.

[`tests/fixtures/synthetic-project.ifc`](../tests/fixtures/synthetic-project.ifc) is a committed IFC from synthesized inputs, useful for testing a consumer without a Revit file. The [browser viewer](https://drunkonjava.github.io/rvt-rs/) runs the same pipeline in the tab: drop a 2023, 2024 or 2025 project, or a 2008 to 2022, 2026 or 2027 one experimentally, and Export IFC writes its typed elements; files from before Revit 2008 are not read. The viewer is built from `main`, not from a release.

## Results at a glance

Running the shipped CLIs against one 400 KB RFA fixture:

- **Metadata**: version, build tag, creator path, file GUID, locale, worksharing state, central model path, last saved (time and user), save counter (`rvt-info`)
- **Folder inventory**: one row per Revit file under a folder — release, worksharing, last saved — as a table, CSV, JSON or JSON Lines, reading only each file's two identity streams (`rvt-info <folder> -f csv`)
- **Atom XML**: title, OmniClass code, taxonomies (`rvt-info` parses `PartAtom`)
- **Preview**: clean PNG thumbnail, 300-byte Revit wrapper stripped (`rvt-info --extract-preview`)
- **Schema**: every class and field of the embedded schema with its typed encoding, 3,490 classes on the 2016 sample to 4,285 on 2026 (`rvt-schema`)
- **History**: every Revit release that ever saved this file (`rvt-history`)
- **Bulk strings**: 3,746 length-prefixed UTF-16LE records from Partitions/NN — Autodesk unit/spec/parameter-group identifiers, OmniClass + Uniformat codes, Revit category labels, localized format strings (`rvt-history --partitions`)

The class names `rvt-schema` extracts were cross-checked against the public `RevitAPI.dll` NuGet package's exported C++ symbol list, by sample rather than exhaustively: all top-level tagged class names inspected (ADocument, DBView, HostObj, LoadBCBase, Symbol, APIAppInfo, APropertyDouble3, ElementId, and the rest) appear in that export with their decorated signatures (e.g. `__cdecl NotNull<class ADocument *,void>::NotNull(class ADocument *)`), consistent with the on-disk schema naming the compiled classes.

A build-server path also appears in C++ assertion strings inside the same DLL; it is mentioned in the recon report for completeness and does not represent anything the reader extracts from .rvt / .rfa files.

### Performance on large projects

Partition streams are inflated once per file and cached on the `RevitFile` handle (#266). On the 33.7 MB `2024_Core_Interior.rvt`, `rvt-ifc --mode geometry` went from **26.07 s / 2641.4 MiB** peak RSS to **1.69 s / 490.1 MiB** (Apple Silicon, `/usr/bin/time -l`, best of three), and the exported IFC is byte-identical once the writer's two wall-clock stamps are normalised. In the browser the same file decodes in **about 3 s** on the deployed site, where it used to take about 28. Every `tools/perf_budget.py --require-category medium` row now passes with an order of magnitude of headroom, without any budget being loosened.

## Findings of the first reconnaissance

Six reproducible findings from the first reconnaissance (April 2026), all documented in [`docs/format-reconnaissance.md`](format-reconnaissance.md) and reproducible with the programs in `examples/` and `probes/examples/`:

1. **The schema indexes the data.** Class names do not appear as ASCII in `Global/Latest`; class tags from `Formats/Latest` (u16 after class name, with 0x8000 flag set) occur ~340× the uniform-random rate. The top tag, `AbsCurveGStep`, appears 19,415 times in 938 KB of decompressed Global/Latest. [`examples/link_schema.rs`]

2. **Tags drift across releases** but are stable-sort-assigned. `ADocWarnings` = 0x001b 2016→2026 because no class sorted alphabetically before it has ever been added. `AbsCurveGStep` shifted 0x0053 → 0x0066 across the decade as 19 new A-class entries were inserted. Full 122-class × 11-release drift table: [`docs/data/tag-drift-2016-2026.csv`](data/tag-drift-2016-2026.csv), visualised in [`docs/data/tag-drift-heatmap.svg`](data/tag-drift-heatmap.svg). [`examples/tag_drift.rs`]

3. **Revit 2021 changed the format substantially.** On the sample families, Global/Latest grew about 27× (about 26 KB to about 715 KB) from 2020 to 2021, and the Forge Design Data Schema namespaces (`autodesk.unit.*`, `autodesk.spec.*`) first appear in Partitions/NN in the same release.

4. **Parameter-group namespace shipped separately in Revit 2024.** `autodesk.parameter.group.*` identifiers appear in 2024+ only — three releases after units/specs. Dating the Forge schema rollout from on-disk bytes: [`examples/tag_drift.rs`](../examples/tag_drift.rs), [`src/object_graph.rs`](../src/object_graph.rs).

5. **A stable Revit format-identifier GUID in family files.** `Global/PartitionTable` is 167 bytes decompressed in `.rfa` family files, and **165 of those bytes are byte-for-byte identical across every Revit release 2016-2026** (98.8% invariant). The invariant region contains a version-1 UUID, `3529342d-e51e-11d4-92d8-0000863f27ad`, whose timestamp field decodes to 2001-01-08. Useful for family-file detection. **Scope correction (2026-04-21):** this invariant is a *family-file* anchor, not a universal Revit-file anchor. Three real `.rvt` project files we probed carry three different GUIDs (`6a6261fd-...` on Revit 2023, `552368c6-...` on 2024, all-zero on 2025) in a shorter 87-byte `PartitionTable`. File-type sniffers using the family GUID will correctly reject non-family files but can't identify them. See [`docs/project-file-corpus-probe-2026-04-21.md`](project-file-corpus-probe-2026-04-21.md). [`examples/partition_full.rs`]

6. **Tagged class record structure decoded.** Every tagged class declaration in `Formats/Latest` carries a tag word (u16 with 0x8000 flag), an optional parent class and a declared field count, followed by N field records, each with a name and a C++ type encoding. `HostObjAttr` resolves to parent `Symbol` with `declared_field_count=3` and all three field names (`m_symbolInfo`, `m_renderStyleId`, `m_previewElemId`) extracted byte for byte. RE-146 later showed that the tag word is the base class's tag and the word after it the grandparent's (#154). [`examples/record_framing.rs`, `src/formats.rs`]

Revit files, Autodesk's shipped samples among them, can carry authoring metadata: the path a file was saved from, a creator name in the `Contents` stream, and build-server paths in assertion strings of the public `RevitAPI.dll`. The values found are not repeated here.

**Downstream safety:** the `rvt-analyze` CLI has a `--redact` flag (used for all committed demo output in this repository) that rewrites creator paths, Autodesk-internal paths and build-server paths to `<redacted>` markers while preserving the surrounding structure. A tool that displays rvt-rs output publicly should do the same.

## Format overview

Every Revit file is a Microsoft Compound File Binary (OLE2) container with
this stream layout (constant across Revit 2016 to 2027):

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
│                                 NN = 58, 60-70 for Revit 2016-2027
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

The format, layer by layer, and how far each layer is understood. Some rows record the research history they come from; the current, evidence-backed status of each capability is in [`docs/support-matrix.json`](support-matrix.json).

| Layer | Description | Status |
|---|---|---|
| 1 · Container | OLE2 / Microsoft Compound File ([MS-CFB]) | **Done** |
| 2 · Compression | Truncated gzip → raw DEFLATE | **Done** |
| 3 · Stream framing | Per-stream custom headers, `Partitions/NN` chunk layout, `Contents` / `Preview` / `PartitionTable` wrappers | **Done** — 165/167 bytes of `PartitionTable` invariant; 44-byte `Partitions/NN` header decoded; `62 19 22 05` wrapper magic confirmed on `Contents` + `RevitPreview4.0` |
| 4a · Schema table | Class names + fields + C++ type signatures from `Formats/Latest`; per-class tag + parent + declared field count; cross-release tag-drift map | **Done** |
| 4b · Schema→data link | Tags from `Formats/Latest` occur at ~340× the noise rate in `Global/Latest`; schema IS the live type dictionary for the object graph | **Done** |
| 4c.1 · Record framing | Tagged class records in `Formats/Latest` parse into structured records: `{tag, parent, ancestor_tag, declared_field_count}`; HostObjAttr → `{parent=Symbol, declared_field_count=3}`. The `tag` a record carries is its base class's tag and `ancestor_tag` is its grandparent's, on 53 files of 2016 to 2027 (RE-146) | **Done** |
| 4c.2 · Field-body decoding | `FieldType` enum classifies all but 2 or 3 schema fields per release across 11 variants (Primitive, String, Guid, ElementId, ElementIdRef, Pointer, Vector, Container, Flagged, PointerArray, PointerContainer). 11 discriminator bytes mapped, including generalized scalar-base Vector/Container (`{kind} 0x10 ...` / `{kind} 0x50 ...`) and the `0x0d` point-type base. | **Done (all but 2 or 3 of 10,000 to 12,000 fields per release since #410 and RE-323; the earlier 100.00% on 13,570 fields covered the first 64 KB of each schema; zero `Unknown`)** |
| 4d · ElemTable | `Global/ElemTable`: the `u32` record count at `0x02`, then records from `0x06`, 28 bytes through Revit 2023 and 40 from 2024, each with its ElementId and owner (#152, RE-41, #737) | **Done**: where a 40-byte record's two ids differ, the second is the ElementId and the first the id the element was created with, which no element record of the project names, so only the second is declared (RE-189, #760) |
| 5 · IFC4 export | Full spatial tree + per-element IFC entities + `IfcLocalPlacement` + extruded, swept, revolved and B-rep bodies + compound material layers + typed property sets + `IfcOpeningElement`/`IfcRelVoidsElement`/`IfcRelFillsElement` for doors and windows. Deterministic ISO-10303-21 output; CI validates the exports of the reference projects with IfcOpenShell. | **Partial**: what each category gets is in [What rvt-rs reads from real projects](evidence.md#what-rvt-rs-reads-from-real-projects); a body that is not decoded is its element's bounding box |
| 6 · Write path | Byte-preserving copy for unchanged files; stream-level patching for named OLE streams with atomic temp-file rename, per-stream verification, grow/shrink/multi-stream coverage, and GUID/history preservation checks. Field-level semantic patching is not implemented (ADR-002). | **Done (stream-level); field-level pending** |
| 7 · Browser viewer | WebAssembly build of the core + Three.js + Vite + Pages deploy. Zero-upload, in-tab parse, export buttons for glTF/IFC/SVG, URL-state share. Live at <https://drunkonjava.github.io/rvt-rs/>. | **Done** |

The original research questions are resolved: those on field encodings, `Global/Latest` and `Partitions/NN` as first answered, and the class preamble's flag word by RE-146, after its first reading did not reproduce (RE-137). Layer 4c.2 classified 100.00% of the 13,570 fields in the first 64 KB of each schema; #410 found the rest of `Formats/Latest` was being inflated without its page strip, and on the whole schema (122,107 fields across the 11 releases of 2016 to 2026, and 134,794 across 12 since 2027 joined the corpus) 9 to 12 fields per release remain unclassified. In April 2026 the next frontier was real-world project files, where one `.rvt` probe caught a `gzip_header_len` bounds bug that family files never hit; the partition element records (RE-21 onward) and the measured tables above are where that work went.

Key findings from this phase:

- **Class preamble flag word** (superseded, RE-137) The u16 "flag" word in each tagged-class preamble was read as a class-tag reference distinct from the parent. That reading did not reproduce under the page-stripped schema grammar (#410, #154): the nine classes behind it have the bases the grammar gives (`HostObjAttr` → `Symbol`), not the "ancestor" named, on 17 files of 2016 to 2026, and every base reference resolves. RE-146 then identified the word: it is the grandparent class's tag, which an earlier addendum had looked up among base tags (#154).
- **Field type encoding** Each field's `type_encoding` is `[byte category][u16 sub_type][optional body]`. 9 category bytes mapped (`0x01` bool, `0x02` char (1 byte), `0x03` short (2 bytes; RE-325), `0x04/0x05` u32, `0x06` f32, `0x07` f64, `0x08` string, `0x09` GUID, `0x0b` u64, `0x0e` reference/container).
- **Field-type coverage** extended to 84% of fields,
  then reaches **100%** of the fields in the first 64 KB of each schema (13,570 across 11 releases; #410 later read the whole schema). Generalized `{scalar_base} 0x10 ...` / `{scalar_base} 0x50 ...` as vector/container modifiers; added `0x0d` point-type base; added `0x08 0x60 ...` alternate string encoding; added `ElementIdRef { referenced_tag, sub }` for references that carry a specific target-class tag; added `0x03`, which this entry first called a deprecated 4-byte i32 alias seen only in 2016–2018; RE-325 measured it as the C++ `short`, 2 bytes, present in every release 2014 to 2027. See `docs/format-reconnaissance.md` §Q5.2.
- **`Global/Latest` layout** It is **not** an index + heap — it's a flat TLV stream.
- **Instance data** is **schema-directed** (tag-less, protobuf-style). Decoding requires schema-first sequential walk from a known entry point.
- **`Partitions/NN` trailers** Their u32 fields are **not** per-chunk offsets. Gzip-magic scan remains correct.

The full analysis narrative with 12 dated addenda lives in [`docs/format-reconnaissance.md`](format-reconnaissance.md). Session-length synthesis in [`docs/rvt-phase4c-session-2026-04-19.md`](rvt-phase4c-session-2026-04-19.md).

## Sample corpus

The real-file checks read public corpora that are fetched, never committed:

- **Families:** Autodesk's `rac_basic_sample_family`, one per Revit release from 2016 to 2027, distributed through Git LFS in [`phi-ag/rvt`](https://github.com/phi-ag/rvt). `RVT_SAMPLES_DIR` points at them.
- **Projects:** the MIT-licensed models of [`magnetar-io/revit-test-datasets`](https://github.com/magnetar-io/revit-test-datasets) (Core Interior, Einhoven) and the RE1 models of [`Drshelden/IFC-ECS`](https://github.com/Drshelden/IFC-ECS), each with Revit's own IFC export. `RVT_PROJECT_CORPUS_DIR` points at them.

A check whose files are absent skips with a message, so a partial corpus is fine. [`CONTRIBUTING.md`](../CONTRIBUTING.md#corpus-env-vars) shows how to fetch both, and [`docs/reference-models.md`](reference-models.md) lists every model the measurements use.

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
  `encoding_rs` decodes the bytes directly and replaces a malformed pair
  with U+FFFD, where std's `String::from_utf16` rejects the whole string.
- **BTreeSet for class names** — deterministic ordering in output (plus
  sorted JSON) matters for diffable CLI output.
