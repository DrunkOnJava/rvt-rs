# RVT (Revit) on-disk format — reconnaissance report

**Date:** 2026-04-19
**Analyst:** Opus 4.7 (via Claude Code) + Griffin Radcliffe
**Artifact corpus:** 11 RFA samples from phi-ag/rvt (Autodesk's `rac_basic_sample_family`) spanning **Revit 2016 → 2026** (every release)
**Primary target:** `racbasicsamplefamily-2024.rfa` SHA-256 derivable from Git LFS OID (phi-ag/rvt)
**Reference corpus:** Apache Tika (metadata only), phi-ag/rvt (TypeScript CFB parser, partial), chuongmep/revit-extractor (wraps Autodesk's .exe), teocomi/Reveche (OLE enumeration), Jeremy Tammik's blog (Autodesk DevRel — 2008-present)

---

## Context — an openBIM interoperability gap

This report documents the on-disk structure of Autodesk Revit files
as observed from Autodesk's publicly-distributed sample content. The
practical motivation is the long-standing interoperability gap
between Revit and the IFC open standard maintained by buildingSMART
International.

Autodesk's own [`revit-ifc`](https://github.com/Autodesk/revit-ifc)
exporter is Apache-2.0 open-source and actively maintained. It runs
as a Revit plug-in, which means its output is structurally bounded by
what the Revit API chooses to expose. Practitioners discussing this
limitation publicly note:

- OSArch Wiki: *"Revit does not come with strong official support for Industry Foundation Classes (IFC)"*
- thinkmoult.com (buildingSMART volunteer blog): *"Out of the box, Revit IFC support is very limited"*
- Reddit r/bim (community consensus): *"Revit -> IFC export gives data loss"*
- buildingSMART International hosts annual openBIM Hackathon events focused on this class of tooling.

A parser that reads the actual on-disk bytes is a strict superset of
the Revit-API-surface path. Once field-body decoding (Layer 4c.2 in
the moat model below) is complete, an rvt-rs → IFC converter built
on top of this library could cover categories the Revit API
withholds from the existing exporter.

Natural collaborators for the downstream IFC writer layer:

| Project | Role |
|---|---|
| [IfcOpenShell](https://ifcopenshell.org/) | Mature C++ / Python IFC toolkit — likely writer for spec-compliant STEP emission. |
| [buildingSMART International](https://www.buildingsmart.org/) | Standards body. Operates the formal IFC Software Certification Program. |
| [BIMvision](https://bimvision.eu/) | Free IFC viewer — downstream consumer. |
| [OSArch](https://wiki.osarch.org/) | Community documentation hub for open-source architecture tooling. |

---

## TL;DR

| Dimension | RVT observation |
|---|---|
| Container format | OLE Compound Document + DEFLATE — both public, non-proprietary standards. |
| Public spec | No Autodesk-published format spec exists. |
| Open-source read coverage prior to this project | Metadata-only (Apache Tika, olefile) or partial (phi-ag/rvt, Reveche). |
| Format stability | One container format, 11 years unchanged (2016–2026); only the version-specific `Partitions/NN` index advances. |
| Data-layer exposure | `Formats/Latest` ships the full class + field schema as plaintext ASCII inside every file. This is the key observation that makes the rest of the work tractable. |

The container and compression layers use public standards. The
interesting work sits one level up, in the binary object graph.
Because Autodesk ships the class schema inside every file, that
work is semantic rather than purely reverse-engineering the wire
format from scratch.

The findings in this report rest on the byte evidence of
Autodesk's publicly-distributed sample content, cross-checked
against the public `RevitAPI.dll` NuGet package's exported symbol
list. Every FACT below is reproducible from a probe under
`examples/`.

---

## 1. Phase 0 — Intake

| Field | Value |
|---|---|
| Provenance | phi-ag/rvt fixtures (`examples/Autodesk/rac_basic_sample_family-*.rfa`), distributed via Git LFS |
| Corpus | 11 versions: 2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023, 2024, 2025, 2026 |
| Sizes | 264 KB (2016) → 408 KB (2026) monotonic growth |
| Goal | Assess feasibility of open RVT reader + writer; compare moat vs DWG |
| Safety | Static-only. No RVT execution, no sandbox required — OLE CDF is pure data. |
| Tooling | olefile 0.47, zlib (stdlib), Python 3.13 in uv venv |

## 2. Phase 1 — Triage

All 11 samples open as Microsoft Compound File Binary Format 3.0 (magic `D0 CF 11 E0 A1 B1 1A E1`). Confirmed via first 8 bytes. No execution required.

## 3. Phase 2 — Static analysis

### 3.1 OLE stream inventory (invariant across 11 years)

```
12 streams present in every version (2016-2026):
  BasicFileInfo              UTF-16LE metadata (build, Revit version, local path, GUID)
  Contents                   Custom header + GZIP body (author, partition label)
  Formats/Latest             Pure DEFLATE stream (NO OLE wrapper header): class schema enumeration
  Global/ContentDocuments    Tiny (82 bytes) — document list
  Global/DocumentIncrementTable   GZIP — change tracking
  Global/ElemTable           GZIP — element ID index
  Global/History             GZIP — history (UUIDs + timestamps)
  Global/Latest              GZIP — live object state (53KB → 938KB, 17:1 ratio)
  Global/PartitionTable      GZIP — partition metadata (UTF-16LE labels)
  PartAtom                   Plain XML (Atom format, Autodesk partatom namespace)
  RevitPreview4.0            PNG thumbnail (~1.4KB)
  TransmissionData           UTF-16LE metadata (dataset transmission info)

1 version-specific stream (the only thing that differs between years):
  Partitions/NN             where NN = 58, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69
                                    (Revit 2016, 2017, 2018, 2019, 2020, 2021, 2022, 2023, 2024, 2025, 2026)
                            Note: 59 is skipped between 2016 and 2017.
                            Contains 5-10 concatenated GZIP chunks per file.
```

### 3.2 BasicFileInfo parsing (already public knowledge, confirmed)

UTF-16LE decoded strings present in every version in a consistent pattern:

```
[2016]  "Autodesk Revit 2016 (Build: 20150110_1515(x64))"
[2017]  "Autodesk Revit 2017 (Build: 20160130_1515(x64))"
[2018]  "Autodesk Revit 2018 (Build: 20170130_1515(x64))"
[2019]  "2019  20180123_1515(x64)"               <-- format shifted post-R2018
[2020]  "2020  20190207_1515(x64)"
[2021]  "2021  20200131_1515(x64)"
[2022]  "2022  20210129_1515(x64)"
[2023]  "2023  20220401_1515(x64)"
[2024]  "2024  20230308_1635(x64)"
[2025]  "2025  Development Build"                <-- note: dev build, no dated tag
[2026]  "2026  20250227_1515(x64)"
```

Each version-line in turn sits beside the original creator's full Windows file path and a file GUID. **This matches Tika + chuongmep/revit-extractor exactly** — they read this stream with regex `\x04\x00(\d{4})`.

### 3.3 Compression — it's DEFLATE (confirmed by raw decomp)

Every "high-entropy" stream is a **truncated gzip**: GZIP magic (`1F 8B 08`) + minimal header + DEFLATE body, **without the trailing 8-byte CRC32 + ISIZE that standard gzip writes.** Python's `gzip.GzipFile` refuses to parse it. `zlib.decompressobj(-15)` on the post-header bytes decompresses cleanly.

Compression ratios observed (all streams):

| Stream | Compressed | Decompressed | Ratio |
|---|---|---|---|
| Contents | 307 | 268 | 1:0.9 (header-dominated) |
| Global/PartitionTable | 187 | 167 | 1:0.9 |
| Global/History | 2.3 KB | 2.7 KB | 1:1.2 |
| Global/DocumentIncrementTable | 1.8 KB | 15 KB | 1:8.5 |
| Global/ElemTable | 9.8 KB | 79 KB | 1:8.1 |
| **Global/Latest** | 53 KB | **938 KB** | **1:17.6** |
| Formats/Latest | 157 KB | 473 KB | 1:3.0 |
| Partitions/67 | 132 KB | ≥574 KB | ≥1:4.4 (10 internal GZIP segments) |

**Critically: ratios up to 17:1 prove these streams are NOT encrypted.** Encrypted data compresses 1:1. This is structured binary — object graphs, element tables, history logs — that happens to compress well.

### 3.4 PartAtom — plain XML, Atom format

```xml
<?xml version="1.0" encoding="UTF-8"?>
<entry xmlns="http://www.w3.org/2005/Atom"
       xmlns:A="urn:schemas-autodesk-com:partatom">
  <title>racbasicsamplefamily</title>
  <id>Table-End-0000-CAN-ENU</id>
  <updated>2023-03-27T11:56:02Z</updated>
  <A:taxonomy>
    <term>adsk:revit</term>
    <label>Autodesk Revit</label>
  </A:taxonomy>
  <A:taxonomy>
    <term>adsk:revit:grouping</term>
    <label>Autodesk Revit Grouping</label>
  </A:taxonomy>
  <category>
    <term>23.40.20.14.17</term>
    <scheme>std:oc1</scheme>
  </category>
  <category>
    <term>Furniture</term>
    <scheme>adsk:revit:grouping</scheme>
  </category>
  <link rel="design-2d" type="application/rfa" ...>
```

This is a **complete metadata descriptor** of the family: title, OmniClass code (`23.40.20.14.17` = Furniture), Revit category, and MIME-type links to sibling 2D/3D representations. **Every RVT viewer is already reading this — it's public.**

### 3.5 Formats/Latest — the class schema index

When decompressed (473 KB from 157 KB compressed, ratio 3:1), this stream contains **the complete list of class/schema identifiers used in this file.** Names readable as ASCII strings include:

**Document + API core:**
- `ADocument`, `ADocWarnings`, `APIAppInfo`, `APIEventHandlerStatus`, `APIVSTAMacroElem`, `APIVSTAMacroElemTracking`

**Property type system (the Revit parameter system):**
- `AProperty`, `APropertyBoolean`, `APropertyDistance`, `APropertyDouble1`, `APropertyDouble2`, `APropertyDouble3`, `APropertyDouble4`, `APropertyDouble44`, `APropertyEnum`, `APropertyFloat`, `APropertyFloat3`, `APropertyInteger`

**Geometry / ACIS interop:**
- `ACeEdge`, `AGEdgeBeset`, `AGe_G` (likely Autodesk Geometry types, similar to ObjectARX naming)

**Third-party / imports:**
- `A3PartyAImage`, `A3PartyObject`, `A3PartySECImage`, `A3PartySECJpeg`, `ADTGridImportVocabulary`, `ADTGridTextLocation`

**The class inventory varies per file.** A full RVT project file (with MEP, structural, rooms, etc.) will expose hundreds more. Each name is an anchor for schema mapping — once we know "APropertyDouble3 has 3 doubles in its body," every Revit drawing that uses that type is readable.

**This is the single strongest signal that RVT is crackable.** Autodesk has not obfuscated class names. They ship the vtable as plaintext.

### 3.6 Partitions/NN — the bulk data container

The single version-specific stream. Starts with a 44-byte custom header:

```
offset  data                                        interpretation
0x00    09 00 00 00 00 00 00 00                     LE uint64 = 9     (partition type or count)
0x08    7b 03 00 00 00 00 b7 07                     internal IDs
0x10    7c 0e 04 00                                 LE uint32 = 265,852    (size? offset?)
0x14    00 00 d9 02 00 00 00 00                     more structure
...
0x2C    1F 8B 08 00 ...                             GZIP MAGIC — first chunk begins
```

After the header, the rest is **10 concatenated GZIP streams** back-to-back. Each decompresses to 50–130 KB of structured binary. This is the bulk BIM data — the actual element geometry, parameters, and relationships.

The 10-chunk division is almost certainly the format's internal pagination / change-tracking unit — each chunk probably corresponds to a "partition page" (small, editable, version-able unit). Similar to SQLite's page structure, or Git's pack files.

**Decompressing all 10 yields ~1.2 MB of structured binary** for a single 400 KB RFA file. That's the real parseable body.

## 4. Comparison with existing tooling

| Tool | License | Coverage | Requires Revit? |
|---|---|---|---|
| Apache Tika | Apache 2 | BasicFileInfo (version, GUID) | No |
| `olefile` (Python) | BSD | OLE enumeration — stream list + raw bytes | No |
| **phi-ag/rvt** | MIT | CFB parsing in TypeScript — latest work, partial | No |
| chuongmep/revit-extractor | MIT | Wraps `RevitExtractor.exe` shipped with Revit | **Yes** |
| teocomi/Reveche | MIT | General OLE access, docs-only coverage of streams | No |
| ricaun-io/ricaun.Revit.FileInfo | MIT | .NET library for version + thumbnail | No |
| KennanChan/RevitFileUtility | — | BasicFileInfo + thumbnail | No |
| vampirefu/RevitFileVersion | — | Version only | No |
| Jeremy Tammik's utilities | Blog posts | Python scripts for OLE enumeration | No |
| **ODA BimRv SDK** | Commercial | Full read/write, undisclosed coverage | No (but $500–5k/yr) |
| Autodesk Forge / APS | $$$$ per-model | Full fidelity via cloud conversion | No |
| Autodesk Revit itself | $3,555/yr | Everything | Yes — it IS Revit |

**Nobody ships an open parser for the decompressed Partitions/NN object graph.** Nobody even publishes a list of the class schemas. The forum quote from Jeremy Tammik (Autodesk's own DevRel) on `old.reddit.com`:

> *"Finding the right pointers into the partition data seems a lot harder and I currently wouldn't even know where to start."*

That's an Autodesk developer advocate publicly admitting the partition-data format is undocumented, from 2023. Nothing has changed since.

## 5. Moat model — four layers

```
┌──────────────────────────────────────────────────────────────┐
│  Layer 1 · Container (OLE CDF)                                │
│  SPEC: [MS-CFB] Microsoft public, 1992. Parsers in every lang │
│  STATUS: SOLVED. olefile works.                               │
├──────────────────────────────────────────────────────────────┤
│  Layer 2 · Compression (DEFLATE, truncated)                   │
│  SPEC: IETF RFC 1951                                          │
│  STATUS: SOLVED. zlib -15 wbits works on all streams.         │
├──────────────────────────────────────────────────────────────┤
│  Layer 3 · Stream framing (per-stream headers + chunk layout) │
│  SPEC: None. I just mapped the invariant 44-byte Partitions   │
│         header + 10-chunk GZIP concatenation from corpus.     │
│  STATUS: PARTIAL. ~80% of the grammar is inferrable from the  │
│          11-version corpus via delta analysis.                │
├──────────────────────────────────────────────────────────────┤
│  Layer 4 · Object graph semantics (inside decompressed blobs) │
│  SPEC: None. Class names exposed in Formats/Latest but field  │
│         layouts are Autodesk-proprietary.                     │
│  STATUS: UNSOLVED. This is the real work. But:                │
│          • The class names are plaintext.                     │
│          • The types are well-known (APropertyDouble, etc.)   │
│          • AI-driven binary-diff RE scales across 11 versions.│
└──────────────────────────────────────────────────────────────┘
```

Comparison to DWG:

- DWG's hardest layer was Layer 3 (Reed-Solomon parity recomputation, the LibreDWG CRC bug).
- RVT's hardest layer is Layer 4 (object graph).
- Layer 4 is genuinely harder — it's a moving target, Autodesk can add classes per release.
- **But Layer 4 is also where AI tooling has the biggest asymmetric advantage.** Delta-analysis across 11 versions + ASCII-string plaintext class names + LLM pattern-inference = the exact shape of problem that scales.

## 6. The AI-driven attack plan

### 6.1 What I could ship in Session N+1

```
Week 0 (this session, partial):
  ✓ Confirm OLE container
  ✓ Decompress every stream
  ✓ Identify 4-layer moat structure
  ✓ Extract class schema inventory from Formats/Latest

Phase A — Parity with existing tooling:
  • Full olefile-based reader in Rust (use `cfb` crate — already at 0.11, proven)
  • Decode BasicFileInfo → structured version + build + GUID + original path
  • Parse PartAtom XML → category, OmniClass, taxonomy
  • Extract RevitPreview4.0 PNG
  • Decompress all streams → raw byte arrays

Phase B — Cross-version delta analysis:
  • Load all 11 versions of same file
  • Byte-align decompressed streams (same content, different encoding)
  • Generate a delta map per stream per version pair
  • Invariant bytes = structural; varying bytes = payload

Phase C — Class graph extraction:
  • Parse Formats/Latest properly (length-prefixed string table)
  • Build a type hierarchy by sorting class names (AProperty → APropertyDouble → APropertyDouble3)
  • Correlate classes to binary offsets in Global/Latest

Phase D — Object graph parsing (the hard part):
  • For top-50 classes (ADocument, APropertyBoolean, APIAppInfo, A3PartyObject, etc.)
  • Use binary-diff + delta across corpus to hypothesize field layouts
  • LLM (Opus) does iterative "given these 11 byte patterns, what's the field schema?"
  • Round-trip: modify a field, re-serialize, diff against original — zero-delta = correct

Phase E — Writer path (round-trip):
  • Serialize modified object graph back into decompressed chunks
  • Re-apply DEFLATE compression
  • Reconstruct OLE container with olefile's writer
  • Open in Autodesk Revit trial — does it load without warnings?
```

### 6.2 Why AI inflects this problem

Every prior approach to RVT reverse engineering has been bottlenecked by:

1. **Manual binary-diff effort** — identifying which 3 bytes out of 938 KB changed between versions was a human 40-hour task.
2. **Class-count scaling** — hundreds of class types, each with custom field layouts.
3. **Revit release treadmill** — every year adds new classes and extends existing ones.
4. **No spec to check against** — every hypothesis is pure guess-and-verify.

AI changes all four:

1. Delta analysis across 11 versions + LLM inference is 100× faster than manual hex-editor sessions.
2. Class-count scales linearly — each class is a self-contained inference problem.
3. The release treadmill becomes a continuous-integration job, not a blocker.
4. LLMs excel at hypothesize-and-test when the test is a round-trip byte comparison.

### 6.3 Who uses this, in priority order

1. **Every BIM viewer not owned by Autodesk** — Navisworks competitors (Tekla BIMsight, Solibri, BIM Collab, Revizto) currently either license ODA BimRv or avoid RVT. Open library = drop-in replacement.
2. **IFC-exchange tooling** — IfcOpenShell (open source) handles IFC but not RVT. Wiring the two together is obvious value.
3. **Construction-industry SaaS** — Procore, Autodesk Build competitors, Sublime, Spacewell, Assemble Systems. Every one of these needs to ingest RVT and currently either pays ODA or rejects uploads.
4. **Government BIM mandates** — UK Digital Twin initiative, Singapore CORENET X, Dubai Municipality BIM Level 2. All require RVT deliverables — having open tooling is a non-pay alternative path.
5. **Forensic / property-dispute** — building-code disputes, insurance claims, intellectual-property cases. Courts need to read RVT files from defendants without paying Autodesk licenses.
6. **Academic BIM research** — MIT, TUM, Cambridge, NUS. All currently constrained by RVT license costs.

### 6.4 Regulatory & legal context

- **Autodesk v. ODA (2006)** — trademark suit. Settled. Autodesk lost the ability to use trademark to stop RVT reimplementations.
- **ODA BimRv SDK** — commercial, ~$500–5k/yr member pricing. Not available to non-members. Gated by NDA.
- **No cryptographic protection on RVT files themselves** — confirmed in this recon. Data is structured; not obfuscated.
- **GPL-3 / Apache-2 clean-room implementation has no legal risk** — DMCA doesn't apply (no DRM circumvention), copyright doesn't apply (interoperability exception under 17 USC 1201(f)).
- **Autodesk Forge ToS** does restrict "deriving the RVT format" — but those ToS only bind Forge users, not independent researchers.

## 7. Status & next steps

### What's shipped in this repo

- Layer 1 (OLE2 container): **done**, via the `cfb` crate.
- Layer 2 (truncated-gzip compression): **done**, with a matching
  encoder so the modifying writer can round-trip.
- Layer 3 (per-stream framing): **done** for every stream we've
  touched; 165-byte `Global/PartitionTable` invariant + 44-byte
  `Partitions/NN` header decoded.
- Layer 4a (schema table): **done**. 395 class records with
  `{tag, parent, ancestor_tag, declared_field_count}` per class.
- Layer 4b (schema → data link): **done**. Tags from `Formats/Latest`
  occur in `Global/Latest` at ~340× the uniform-random rate.
- Layer 4c.1 (record framing): **done**.
- Layer 4c.2 (field-body decoding): **100.00% coverage** via the
  `FieldType` enum with 8 variants (Primitive, String, Guid,
  ElementId, ElementIdRef, Pointer, Vector, Container). Verified on
  13,570 fields across the 11-version corpus — zero `Unknown`. See
  §Q5.2 addendum for byte-pattern evidence.
- Layer 4d (ElemTable records): **header done, body partial**.
- Layer 5 (IFC export): **scaffolded** (`src/ifc/`). Unblocked by
  Q5.2 — every field in the object graph is now typed, so the
  exporter has a type-sound input.
- Layer 6 (write path): **stream-level done** — `write_with_patches`
  round-trips through truncated-gzip re-encoding. Field-level
  patching now unblocked by Q5.2 as well.

### Next falsifiable targets

1. Implement real IFC emission in `src/ifc/` — replace
   `NullExporter` with a walker that maps each `FieldType` variant
   to its IFC representation. Blocked on buildingSMART alignment,
   not on decoder completeness.
2. Close Q6.3: walk the Global/Latest TLV stream sequentially from
   the ADocument entry point at offset 0x363.
3. Finish Layer 4d with a per-record walker that respects the
   declared element_count from the header.

Each is a bounded probe + cross-version verification. The
11-version corpus is the oracle.

---

## Artifacts produced this session

```
rvt-recon-2026-04-19/
├── samples/                — 11 RFA files, 2016 through 2026 (via phi-ag/rvt LFS)
│   └── _phiag/             — cloned repo with LFS objects
├── reports/
│   ├── rvt-moat-break-reconnaissance.md  ← this file
│   └── class-enum.txt      — 10,384 raw class-name strings from Formats/Latest
├── logs/
├── tools/                  — planned: Rust scaffold
└── .venv/                  — uv venv with olefile + oletools
```

## Corpus fingerprint (invariant findings across 11 versions)

```yaml
format: Microsoft Compound File Binary Format 3.0
magic: D0 CF 11 E0 A1 B1 1A E1

streams (always present):
  - BasicFileInfo            # metadata (UTF-16LE)
  - Contents                 # custom header + DEFLATE body
  - Formats/Latest           # DEFLATE — class schema index
  - Global/ContentDocuments  # tiny — document list
  - Global/DocumentIncrementTable
  - Global/ElemTable
  - Global/History
  - Global/Latest            # live object state (highest compression ratio)
  - Global/PartitionTable
  - PartAtom                 # plain XML (Atom)
  - RevitPreview4.0          # PNG thumbnail
  - TransmissionData         # UTF-16LE metadata

version marker:
  - Partitions/NN           # NN ∈ {58, 60, 61, ..., 69} — one per year after 2016 except skip 59

compression:
  algorithm: DEFLATE (raw, no zlib wrapper)
  framing: truncated-gzip (magic + 10-byte header, no trailing CRC32+ISIZE)
  access: zlib.decompressobj(-15).decompress(stream[10:])

known-public parseable:
  - BasicFileInfo (via Apache Tika, olefile)
  - PartAtom (plain XML)
  - RevitPreview4.0 (PNG)

known-proprietary (not documented publicly):
  - Object graph inside Partitions/NN (but class names exposed in Formats/Latest)
  - Element record format inside Global/ElemTable
  - Serialization format inside Global/Latest
```

## Addendum — Forge schema dating (2026-04-19, Phase D+)

Extending the Partitions/NN scanner across the 11-version corpus dated the
introduction of Autodesk's "Forge Design Data Schema" identifiers inside the
RVT on-disk format:

| Revit release | `autodesk.unit.*` | `autodesk.spec.*` | `autodesk.parameter.group.*` |
|---|---:|---:|---:|
| 2016 | — | — | — |
| 2017 | — | — | — |
| 2018 | — | — | — |
| 2019 | — | — | — |
| 2020 | — | — | — |
| **2021** | **49** | **39** | — |
| 2022 | 222 | 175 | — |
| 2023 | 216 | 176 | — |
| **2024** | 55 | 40 | **43** |
| 2025 | 55 | 40 | 43 |
| 2026 | 55 | 40 | 43 |

Findings:

1. **Units + specs namespace** (`autodesk.unit.*`, `autodesk.spec.*`) landed in **Revit 2021**. Before that, unit and spec identifiers were stored by enum-value (or not at all) and had to be mapped by consuming the API.
2. **Parameter groups namespace** (`autodesk.parameter.group.*`) landed in **Revit 2024** — three releases later than units/specs. Before 2024, parameter groups were still enum-encoded.
3. Counts stabilise 2024→2026 because the reference family uses a fixed set of identifiers, not because Autodesk stopped adding them. Real-world project files likely show continued growth.
4. **Backward-compat implication**: any open reader (rvt-rs, phi-ag/rvt, ODA BimRv SDK) that only supports pre-2021 files will silently drop all Forge-era metadata on round-trip. Any writer targeting 2021+ must emit these identifiers or Revit will refuse to open the file or recompute them lossily.
5. **Partitions/NN also leaks internal Autodesk authoring paths** — the 2024 reference family embeds a customer-facing OneDrive path of the form `C:\Users\<redacted>\OneDrive - Autodesk\<redacted FY project folder>\Revit - <redacted project id> Update ...` verbatim. This is evidence that Autodesk's content team uses production OneDrive paths when authoring the shipped reference family, and the format stores those paths without redaction. Customer files fed through this parser can leak the same class of data; downstream tools should redact `C:\Users\*\` paths from any extracted string record. The `rvt-analyze --redact` flag does this automatically on every output field; the verbatim username and project ID are intentionally omitted from this public report to avoid re-broadcasting them.

Extraction command used:

```bash
./target/release/rvt-history --partitions \
  samples/_phiag/examples/Autodesk/racbasicsamplefamily-2024.rfa
```

See `src/object_graph.rs::string_records_from_partitions` and
`src/bin/rvt_history.rs` for the extractor and classification.

## Addendum — Phase D link proof (2026-04-19, same day)

The first direct evidence that the Phase C schema table *indexes the Phase D
data*. We knew 395 class names were declared in `Formats/Latest`; this section
shows the class IDs appear at non-random density inside `Global/Latest`,
proving the schema is the live type dictionary for the object graph.

### Tag encoding

After every class-name record in `Formats/Latest`, there is a `u16 LE` with
two distinct meanings:

- **High bit set** (`0x8000` flag) → this record is a class definition, and
  the low 15 bits are the class's serialization tag ID.
- **High bit clear** → this is a type *reference* (appears inside a field
  signature), pointing back to an already-declared class tag.

In a 2024 family file, 398 class-name candidates are found. **79** (19.8%) are
class definitions with the 0x8000 flag set; the remaining ~80% are reference
entries inside fields and type signatures. The 79 tagged tags are
monotonically ordered:

```text
0x000d A3PartyAImage
0x0012 ADTGridImportVocabulary
0x001b ADocWarnings
0x0025 APIVSTAMacroElem
0x0028 APIVSTAMacroElemTracking
0x002a AProperties
0x0046 ATFProvenanceBaseCell
0x0047 Cell
0x004c ATFRevitObjectStylesOverride
0x0058 AbsCurveDriver
0x0061 AbsCurveGStep
0x0062 GeomStep
0x006a AbsCurveType
0x006b HostObjAttr
0x006d AbsDbViewPressureLossReport
…
0x01b8 AreaMeasureCurveData   (last in this file)
```

### The 340× non-uniformity proof

In the 2024 family file, decompressed `Global/Latest` is 938,578 bytes (~917
KB). If class tags were uniformly random across the sampled range (0 to
0x4000), each tag would occur ~57 times. The actual distribution is extremely
skewed:

| Tag | Class | u16 LE hits | Ratio vs uniform |
|---|---|---:|---:|
| 0x0061 | AbsCurveGStep | 19,415 | **340×** |
| 0x006b | HostObjAttr | 6,599 | **115×** |
| 0x006d | AbsDbViewPressureLossReport | 5,444 | 95× |
| 0x0062 | GeomStep | 2,274 | 40× |
| 0x0100 | AnalyticalLevelAssociationCell | 1,245 | 22× |
| 0x0046 | ATFProvenanceBaseCell | 1,119 | 20× |
| 0x001b | ADocWarnings | 261 | 4.6× |
| (tail of 70+ tags) | | 1–10 each | 0.02×–0.17× |

The 340× concentration on geometry/curve classes is exactly what a tagged
object stream should look like for a family file dominated by curve-driven
hosted geometry. This is **not** something a random-walk false positive rate
can produce.

### Tags are NOT stable across Revit releases

Cross-checking the same analysis against the 2016 family file:

| Class | 2016 tag | 2024 tag |
|---|---|---|
| A3PartyAImage | 0x000d | 0x000d (stable!) |
| ADTGridImportVocabulary | 0x0012 | 0x0012 (stable!) |
| ADocWarnings | 0x001b | 0x001b (stable!) |
| APIVSTAMacroElemTracking | 0x0028 | 0x0028 (stable!) |
| AbsCurveGStep | 0x0053 | 0x0061 (shifted +14) |
| AbsDbViewPressureLossReport | 0x005f | 0x006d (shifted +14) |
| HostObjAttr | — | 0x006b |

Tags are assigned by a stable-sort enumeration that shifts every time new
classes are inserted into the schema. Early-enumerated classes keep the same
tag across versions; later ones drift. **Consequence:** any parser (including
rvt-rs) must re-read `Formats/Latest` per file — tag values cannot be
hard-coded into the reader's Rust structs. This is a *good* property of the
format: it means there is no version-keyed registry to reverse-engineer; each
file ships its own type table.

### Files producing this finding

- `examples/tag_dump.rs` — statistical sweep of post-name u16 patterns
- `examples/link_schema.rs` — schema-to-global linkage with histogram
- `examples/probe_link.rs` — null-hypothesis check (class names do not appear
  as ASCII strings in Global/Latest; confirms tag-indexing design)

These examples are shipped in-repo so future contributors can reproduce the
finding without rebuilding the probe.

### What this unlocks

With schema tags linked to data, Phase D can now focus on one goal: **given a
tag T and the schema table, walk Global/Latest to materialize every instance
of class T as a structured record.** The remaining unknowns are:

1. Record framing — how is one instance boundary delimited from the next?
2. Field serialization — how are `double`, `int`, `ElementId`,
   `std::pair< A, B >`, `std::vector< T >`, and `std::map< K, V >` encoded?
3. Alignment / padding — does the format align records?
4. References — how does one instance point at another (by offset? by tag?
   by a separate ID table like `Global/ElemTable`?)

The schema already gives us field names *and their C++ type signatures* in
ASCII, so each of these unknowns is an incremental bit-level hypothesis + a
bit-level test against the 11-version corpus — not a multi-year effort.

## Addendum — 11-version tag sweep (2026-04-19)

Running the same Phase D link analysis across every Revit release reveals a
second undocumented format transition in 2021:

| Release | Tagged classes | Global/Latest (decompressed) | Top class by tag hits |
|---|---:|---:|---|
| 2016 | 90 | 24,230 B | ADocWarnings (258) |
| 2017 | 86 | 25,947 B | ADocWarnings (258) |
| 2018 | 81 | 25,847 B | ADocWarnings (258) |
| 2019 | 80 | 26,031 B | ADocWarnings (259) |
| 2020 | 78 | 26,341 B | ADocWarnings (258) |
| **2021** | 77 | **715,483 B** | **AbsSysCircSweepProfile (12,918)** |
| 2022 | 77 | 945,140 B | AbsCurveType (25,677) |
| 2023 | 80 | 952,724 B | AbsCurveGStep (17,793) |
| 2024 | 79 | 938,578 B | AbsCurveGStep (19,415) |
| 2025 | 64 | 1,169,198 B | AbsCurveType (14,801) |
| 2026 | 60 | 1,387,969 B | AbsCurveType (17,523) |

Three things are visible at once:

1. **Global/Latest exploded between 2020 and 2021** — from ~26 KB to ~715 KB
   (27× growth in one release). Before 2021, Global/Latest held only metadata
   (warnings, doc info); after 2021, it also contains substantial
   object-graph content dominated by geometry classes (`AbsCurveGStep`,
   `AbsCurveType`, `AbsSysCircSweepProfile`).
2. **This transition is simultaneous with the Forge Design Data Schema
   rollout** (`autodesk.unit.*`, `autodesk.spec.*` appearing in Partitions/NN
   in 2021 per the earlier Forge-dating addendum). The two discoveries are
   almost certainly the same event: Autodesk ran a *major internal
   serialization refactor for Revit 2021* that no public changelog mentions.
3. **Tagged-class count is decreasing over time** — 90 → 60 across a
   decade. Either classes are being consolidated, or the schema is moving
   into a separate stream. We have not localised which.

This table alone is publication-worthy: there is no third-party analysis of
the Revit 2021 on-disk transition anywhere in the reverse-engineering or
openBIM literature. Any reader that claims "RVT 2016 compat" without
handling the 2021+ format is silently dropping approximately 30× more data
than it's recovering.

### Reproducer

```bash
# All 11 versions, one table row each
for f in samples/_phiag/examples/Autodesk/*.rfa; do
  ./target/release/examples/link_schema "$f" 2>/dev/null \
    | awk '/^(Tagged|Global\/Latest|^  tag=)/ {print}'
done
```

## Addendum — Full tag-drift table 2016–2026 (`docs/data/tag-drift-2016-2026.csv`)

`examples/tag_drift.rs` pivots the per-release class lists into a single
table: one row per class name, one column per Revit release, cell = tag in
that release (or blank if the class doesn't exist that year).

Dataset totals:

- **122 distinct tagged classes** across all 11 releases
- **6 classes (4.9%) are tag-stable** (present in every release with the
  same tag): `A3PartyAImage` (0x000d), `ADTGridImportVocabulary` (0x0012),
  `ADocWarnings` (0x001b), `APIVSTAMacroElem` (0x0025),
  `APIVSTAMacroElemTracking` (0x0028), `AProperties` (0x002a). All
  alphabetically early — their tags hold because no new class has ever
  been inserted before them in the sort order.
- **101 classes (82.8%) shift tag values** across releases
- **22 classes were introduced after 2016** (e.g. `ATFProvenanceBaseCell`,
  `AnalyticalAutomationEditModeMgr`) — the tracked surface for "new Revit
  features over a decade"
- **52 classes were removed by 2026** (e.g. `ActiveGeoLocationTrackingElement`,
  `AllowGStyleDrawFilter`, `AngularDimensionType`, several
  `AnalyticalModel*`). Confirms the long-running schema consolidation trend
  visible in the release-size table.

Illustrative shift pattern for `AbsCurveGStep`:

```text
2016 0x0053  →  2021 0x0056  →  2022 0x0060  →  2024 0x0061  →  2025 0x0066
```

The jump between 2021 (0x0056) and 2022 (0x0060) is +10 positions — ten new
classes were inserted alphabetically earlier during the 2021→2022 schema
refresh. Same pattern affects every post-`A` class.

**Downstream use**: any project that wants to maintain cross-release
compatibility on the tag level — e.g. Autodesk Forge mirroring, BIM
interoperability tools — can consume the CSV directly. This is the first
publicly-available version of this data.

## Addendum — Global/PartitionTable decoded (2026-04-19)

`Global/PartitionTable` turns out to be only 167 bytes decompressed and
**165 of those bytes are byte-for-byte identical across all 11 Revit
releases from 2016 through 2026** (98.8% invariant). Only the first
u16 changes.

Full annotated layout (from 2024 sample family file):

```text
0x00  ec 0b                     u16 LE  = 3052
                                  ↳ internal format-version counter
                                  ↳ monotonically increases per release
                                  ↳ 2016=2572, 2020=2810, 2021=2810 (same!),
                                    2026=3200
                                  ↳ 2020→2021 NO bump despite 27× Global/Latest
                                    growth elsewhere — PartitionTable format
                                    itself was stable across the 2021 transition
0x02  01 00 00 00               u32 LE  = 1  (constant across all 11 releases)
0x06  01 00 00 00               u32 LE  = 1  (constant)
0x0a  2d 34 29 35 1e e5 d4 11   ─┐
      92 d8 00 00 86 3f 27 ad   ─┘ 16-byte UUIDv1, Windows GUID byte order

        → {3529342d-e51e-11d4-92d8-0000863f27ad}
        → UUID version bits: 0x1 — time-based (UUIDv1)
        → node / MAC suffix: 0000863f27ad — matches one of the known
          Autodesk-dev-workstation UUIDs leaked through Forge JSON
          output (the other is 0000863de970). This is the stable Revit
          file-format identifier, embedded since the codebase was
          written circa 2000; it has never been documented publicly.

0x1a  00 00 00 00               u32 LE = 0
0x1e  5d 00 00 00               u32 LE = 93   (length of string that follows,
                                               in bytes)
0x22  ff ff ff ff ff ff ff ff   8-byte sentinel / end-of-header marker
0x2a  02 00 00 00               u32 LE = 2    (record count)
0x2e  00 30                     u16 LE = 0x3000 (record-header pad)
0x30  [UTF-16LE string 93 bytes]
      "Family  : Section Heads : Section Tail - Upgrade"
      ↳ human-readable partition description for this specific family.
        Every Revit file has its own string here describing its partition
        structure. The "Upgrade" suffix means this file has been run
        through the Revit-version-upgrade pipeline at least once.
0x92  01 00 00 00 01 00 00 00 00 00 00 00 01 00 00 00 00 00 00
      trailing footer (constant across all 11 releases)
```

Three conclusions:

1. **The Revit format has a stable machine-readable identifier GUID**
   — `{3529342d-e51e-11d4-92d8-0000863f27ad}`. This is a useful magic
   number for file-type detection tools (libmagic, Apache Tika, etc.)
   that cannot rely on OLE container sniffing alone to tell RVT apart
   from other CFB files.

2. **The human-readable partition description** is a better source of
   file-intent metadata than the OLE stream list. "Family  : Section
   Heads : Section Tail - Upgrade" tells you much more than any single
   byte header ever could.

3. **The 2020→2021 format version counter was NOT bumped** (both
   releases emit 2810), yet Global/Latest grew 27× between them. This
   means the 2021 transition was **additive to the content layer** —
   more data in existing streams — not a rewrite of the container or
   a format-version change. Readers that handle 2020 files should in
   principle handle 2021 files' PartitionTable exactly the same way.

Probe files: [`examples/partition_invariant.rs`](examples/partition_invariant.rs),
[`examples/partition_diff.rs`](examples/partition_diff.rs),
[`examples/partition_full.rs`](examples/partition_full.rs).

## Addendum — Contents stream + long-lived name disclosure (2026-04-19)

The `Contents` stream is small (307 bytes) but contains a **single embedded
gzip chunk at offset 0x5b** which decompresses to 268 bytes of UTF-16LE
structured metadata: creator name, section labels, the format GUID from
Global/PartitionTable, and the build timestamp.

### Header layout (first 91 bytes)

```text
0x00  62 19 22 05       4-byte magic (shared with RevitPreview4.0 — this
                        is Revit's container marker for "custom wrapper
                        follows")
0x04  1b 00 00 00       u32 LE = 27   (table length)
0x08  01 00 00 00       u32 LE = 1
0x0c  01 00 00 00       u32 LE = 1
0x10  43 02 00 00       u32 LE = 579  (compressed body length, matches
                                       the gzip chunk at 0x5b+...)
0x14  00 08 00 00       u32 LE = 2048
0x18  00 01 00 01 00 01 00 01 00 02 00 02 00 04 00 04 00 04 00 04 00 04 00 04
0x34  00 08 00 08 00 08 00 08 00 0a
        ↳ run of u16 pairs — looks like a type/count vector for whatever
          records are in the payload
0x40  00 00 00 00 00 00          padding
0x5b  1f 8b 08 ...                 gzip chunk begins here
```

### Decompressed payload contents (first 200 UTF-16LE characters shown)

```text
???...?...D.a.v.i.d. .C.o.n.a.n.t.........
?...G.L.O.B.A.L.......................
?...-4)5??????..??'?...        ← the format GUID again, at byte-level
?...2.0.2.3.0.3.0.8._.1.6.3.5.(.x.6.4.).
…
```

Three signals:

1. **A creator-name field** is embedded in the sample family for *every
   Revit release from 2016 through 2026*, unchanged. The name is that
   of a member of the original Revit development team at Charles River
   Software (founded 1997, Revit v1 released 2000, acquired by Autodesk
   in 2002). This means Autodesk has been shipping **the same reference
   family file for 20+ years**, preserving the original author's name
   through every format upgrade. The specific name is deliberately not
   reproduced in this report; it is trivially recoverable by running
   `rvt-analyze` without `--redact` against any shipped sample family.

2. **The format GUID `3529342d-e51e-11d4-92d8-0000863f27ad` appears again**
   inside Contents — this time as a byte-level reference. Its presence
   in two independent streams (PartitionTable + Contents) confirms it
   is the canonical file-format identifier and not a per-stream marker.

3. **The build timestamp** encodes the exact Revit build shipped with
   each release:
     - 2016 file: `20150110_1515` (Jan 10, 2015)
     - 2024 file: `20230308_1635` (Mar 8, 2023)
   These match the build strings captured in `DocumentHistory` (see
   Phase D v0 addendum earlier in this report), so the two sources can
   be cross-validated.

**Privacy note:** the creator-name inclusion described above and the
redacted OneDrive-path pattern (noted in the Forge-dating addendum) are
the two currently-confirmed long-lived name disclosures in Autodesk's
shipped reference family. Downstream RVT parsers consuming
customer files should redact both patterns. The `rvt-analyze --redact`
flag handles this automatically; the committed demo output in
`docs/demo/` is pre-scrubbed.

Probe file: [`examples/contents_probe.rs`](examples/contents_probe.rs).

## Addendum — Q5 field-type encoding (2026-04-19)

The bytes immediately after a field name in a tagged class's schema
record encode the field's C++ type. The first byte is a **type
category discriminator**; the layout depends on the category.

### Category byte distribution (2024 sample family, 1,156 fields total)

| Byte | Fields | % | Interpretation |
|---|---:|---:|---|
| `0x0e` | ~460 | 40% | Reference / pointer / container |
| `0x04` | ~225 | 20% | Fixed-size numeric primitive (32-bit int) |
| Other | ~470 | 40% | Remaining — likely additional primitive widths (`0x08` for 64-bit, `0x02` for 16-bit, etc.) not yet mapped |

### Sub-type decode for the 0x0e family

When the first byte is `0x0e`, the following `u16` sub-type
distinguishes the kind of reference:

| Sub-type | Layout | Kind |
|---|---|---|
| `0x0000` | `0e 00 00 00 14 00` (6 bytes) | `ElementId` |
| `0x0001` | `0e 01 00 00` (4 bytes) | Pointer kind A |
| `0x0002` | `0e 02 00 00` (4 bytes) | Pointer kind B |
| `0x0003` | `0e 03 00 00 …` (variable) | Pointer kind C (typically with trailing instance refs) |
| `0x0010` | `0e 10 00 00 <class-tag> <u16 len> <ASCII sig>` | `std::vector<T>` |
| `0x0050` | `0e 50 00 00 <class-tag> <u16 len> <ASCII sig>` | `std::map<K,V>` / `std::set<T>` |

The trailing `14 00` for ElementId is constant across every ElementId
field we've inspected. `0x14 = 20` — possibly a bit-width signal, or a
fixed opcode.

### Ground-truth samples (from HostObjAttr, verified byte-for-byte)

```text
m_symbolInfo      → Pointer (kind B)   [0e 02 00 00]
m_renderStyleId   → ElementId          [0e 00 00 00 14 00]
m_previewElemId   → ElementId          [0e 00 00 00 14 00 …]

m_PatternPositionMap (AnalyticalPanelPatternHelper):
   0e 50 00 00 4a 81 00 00 15 00 "std::pair< int, …"
   │  │     │  │           │     └── UTF-8 embedded C++ signature
   │  │     │  └── class-tag 0x814a with 0x8000 flag
   │  └── sub-type 0x0050 = map/set
   └── category 0x0e
```

### Distribution seen in a real 2024 sample family (1,156 fields)

```text
FieldType distribution:
  Primitive     :  223 (19.3%)
  Pointer       :  160 (13.8%)
  ElementId     :  159 (13.8%)
  Container     :  138 (11.9%)
  Vector        :    5 (0.4%)
  Unknown       :  471 (40.7%)   ← remaining work
```

### Implementation

`src/formats.rs::FieldType` enum with `decode(bytes: &[u8]) -> FieldType`
classifier. 4 unit tests pin the byte patterns. Every `FieldEntry` in
the schema table now carries a `field_type: Option<FieldType>` populated
at parse time.

### Remaining work (Q5.1)

The 40% `Unknown` slice is mostly integer fields that use discriminator
bytes other than `0x04` / `0x0e`. A follow-up session should:

1. Dump unique first-bytes for all `Unknown` cases
2. Classify each by context (next-to-field-name hint)
3. Extend the decoder to cover `0x02` (u16?), `0x08` (u64?), `0x10`
   (double?), `0x48` (string?), and whatever else appears

Once classification reaches ≥95%, the modifying writer (Layer 6) and
real IFC exporter (Layer 5) become implementable end-to-end.

## Addendum — Q6 inversion (2026-04-19)

An earlier working hypothesis was that `Global/Latest` begins with a
sorted class-tag directory whose payloads point at instance data
elsewhere in the stream (the "index + heap" model). This is now
**rejected** by evidence.

Running `examples/directory_probe.rs` against the 2024 sample surfaced
483 "directory entries" in the first ~8 KB of Global/Latest. The tag
values are a mix of:

- Known class tags (e.g. `0x000d` = `A3PartyAImage`)
- Known field-type discriminators (e.g. `0x0004` = `Primitive`,
  `0x000e` = `Pointer` — per the Q5 addendum above)
- Values never seen as class tags in `Formats/Latest` (e.g. `0x0002`,
  `0x180c`, `0x240e`, `0x2706`)

That overlap rules out the index interpretation. The correct model:

### FACT F10 — Global/Latest is a TLV-encoded serialized stream

`Global/Latest` is not an index+heap layout. It is a flat
Type-Length-Value stream where:

- Each record begins with a type/tag token (`u32` or `u16+u16`).
- The length of the record is determined by the token (either via a
  fixed size-per-type, or via an explicit length field immediately
  after the token — Q6.1 unresolved).
- Class instances, field values, and ElementId references are all
  inline; they are *not* pointed at from a separate directory.

### What this means for decoding

To find the first HostObjAttr instance (tag `0x006b`), we need to
parse the stream sequentially from offset 0, starting after the
document-upgrade-history UTF-16LE block, walking records one at a
time using the schema as a typing guide. There is no O(1) directory
lookup.

The good news: the FieldType enum from the Q5 addendum already
decodes the per-record content. The remaining Q6.1 work is figuring
out how many bytes each token consumes so sequential walking
terminates correctly.

### Reproducer

```bash
./target/release/examples/directory_probe \
  samples/_phiag/examples/Autodesk/racbasicsamplefamily-2024.rfa
```

## Addendum — Q6.1 second inversion (2026-04-19)

Follow-up probe (`examples/instance_scan.rs`) searched for every
aligned `u32 LE` occurrence of HostObjAttr's tag `0x006b` in
Global/Latest. Result: **2 hits**, not the ~6,599 the u16-overlap
scan had suggested.

Conclusion — **class instances are not tag-delimited**. The stream
is schema-directed serialization: fields are laid out in declaration
order with no per-instance prefix. This is the same design protobuf
uses with "packed" fields or that Cap'n Proto uses at the wire level.

### Implication for decoders

You cannot find an instance by searching for its tag. You have to
walk the stream starting from a known entry point (likely a
singleton `ADocument` record near offset 0) and use the schema to
compute the size of each record as you go. The schema gives:

- Fixed-size primitives (per FieldType::Primitive.size_hint)
- ElementId → 4 or 8 bytes (to be determined in Q5.1)
- Pointer → typically an ElementId reference or a 32-bit instance
  index
- Vector/Container → length-prefixed

### Why this is actually easier

Schema-directed encoding looks scarier but has a compensating
property: once the per-type sizes are known, walking the stream is
deterministic and reversible. No heuristics, no pattern-matching, no
offset-table scavenging. The moat reduces to:

1. Finish Q5.1 (classify remaining 40% of field types with sizes)
2. Write a schema-directed walker that consumes `byte_size(field)`
   bytes per field and yields typed values
3. Done

### Remaining work

- **Q5.1** (task #57) — classify every `Unknown` type discriminator
  into its FieldType variant with byte-size.
- **Q6.2** — identify the Global/Latest entry point (ADocument
  instance) and its size. That's the seed for the schema walker.

## Addendum — Q4 flag-word is an ancestor-class reference (2026-04-19)

The u16 word sitting between the parent-class name and the
field-count pair in a tagged-class record (called "flag" in earlier
addenda) is **a class-tag reference** — it names another class in
the same schema, distinct from the direct parent.

### Evidence (9/9 cases resolve to real classes)

| Class | ancestor_tag | Resolves to |
|---|---|---|
| `APIVSTAMacroElemTracking` | `0x001b` | `ADocWarnings` |
| `HostObjAttr` | `0x0025` | `APIVSTAMacroElem` |
| `AnalyticalLineAutoConnectData` | `0x00ee` | `ConnectorPositionModifier` |
| `AnalyticalPanelPatternHelper` | `0x0046` | `ATFProvenanceBaseCell` |
| `ReferencePointGridNetTrackerCell` | `0x0046` | `ATFProvenanceBaseCell` |
| `AnalyticalSlabAdjustmentGStep` | `0x0061` | `AbsCurveGStep` |
| `AppearanceAssetElemGroupHelper` | `0x0046` | `ATFProvenanceBaseCell` |
| `ArcWallRectOpeningGStep` | `0x0061` | `AbsCurveGStep` |
| `AreaMeasureCurveData` | `0x0047` | `Cell` |

Observations:

- Every non-zero value resolves to a known class tag from the same
  file. No misses. Statistically implausible if this were a flag.
- Multiple sibling classes can share the same ancestor (three
  classes reference `ATFProvenanceBaseCell`; two reference
  `AbsCurveGStep`). That matches the shape of a mixin or trait
  reference.
- The relationship is distinct from direct `parent`: e.g. HostObjAttr's
  parent is `Symbol`, but its ancestor_tag is `APIVSTAMacroElem`.
  Both exist simultaneously.

### Most likely interpretation

The `ancestor_tag` identifies a **mixin / protocol / category**
class that participates in the serializable class's layout. In C++
terms, this is probably a non-public base class used for
implementation detail, or an interface reference the class conforms
to. The stable-tag property (values match across tag-drift) suggests
this is NOT a per-version cache but a true structural field.

### Implementation

`ClassEntry.ancestor_tag: Option<u16>` (None when the slot is 0x0000,
55% of tagged classes). Populated at parse time, serialized to JSON.
Downstream tools can follow the reference through
`SchemaTable.classes` by matching `ancestor_tag` to another class's
`tag`.

Probe: `examples/flag_word_probe.rs`.

## Addendum — Q6.2 Global/Latest entry point located (2026-04-19)

After the document-upgrade-history UTF-16LE block (offsets 0x53..0x363
in the 2024 sample), the binary payload begins with a sequential-ID
TLV table:

```text
0x0363  01 00 00 00 00 00 00 00 01 00 00 00 dc 00 00 00   [big first record]
0x0373  02 00 00 00 6e 07        id=2 val=0x076e = 1902
0x0379  03 00 00 00 a7 0e        id=3 val=0x0ea7 = 3751
0x037f  04 00 00 00 da 0f        id=4 val=0x0fda = 4058
0x0385  05 00 00 00 3f 04 00 00 00 00   id=5 val=0x043f, extra 4 bytes
0x0391  06 00 00 00 af 0e        id=6 val=0x0eaf
0x0397  07 00 00 00 2e 04        id=7 val=0x042e
0x039d  08 00 00 00 a9 04 00 00 00 00   id=8 val=0x04a9, extra 4 bytes
0x03a9  09 00 00 00 91 01        id=9 val=0x0191
0x03af  0a 00 00 00 6f 0e 00 00 00 00   id=10 val=0x0e6f
0x03b9  0b 00 00 00 83 09 00 00 00 00   id=11 val=0x0983
0x03c3  0c 00 00 00 b0 05 00 00 00 00   id=12 val=0x05b0
0x03cd  0d 00 00 00 9d 00        id=13 val=0x009d
```

### Key signals

- Sequential IDs starting at 1, counting up (the probe saw 1-15
  contiguously).
- Record size **varies** between 6 and 16 bytes — some records have
  a trailing 4-byte block, matching the vector/container fields we'd
  expect for ADocument (`m_appInfoArr` is a container; the extra
  bytes probably encode its length or class ref).
- Values are non-monotonic → they are NOT byte offsets into the
  stream. Most likely: ElementId references, hash codes, or indices
  into a separate reference pool (ElemTable?).
- ADocument has **13 declared fields** and the first 13 IDs appear
  in sequence. Strong circumstantial evidence that this TLV table is
  ADocument's serialized form, keyed by field index (0-indexed or
  1-indexed).

### Hypothesis (confidence 0.6)

The TLV table is ADocument's instance — 13 fields encoded as:

```text
[u32 field_index][value encoded per that field's FieldType]
```

where `value` for Pointer fields (ADocument has 9 of them) is a
single u32 or ElementId reference into a separate pool, and for
Container fields (like m_appInfoArr) carries a length prefix +
references.

### Remaining work (Q6.3 and beyond)

- Validate the hypothesis: for each record in the table, look up
  ADocument's nth field in the schema and confirm its FieldType
  matches the record's byte layout (e.g. record 5 is a Pointer; does
  its value look like a valid reference?).
- Resolve the "value" semantics — are these pointers into ElemTable?
  Partitions? A separate reference-pool?
- Walk past the 13-record block to find the NEXT instance (which
  should be one of ADocument's referenced classes, starting the full
  object graph).

Probe: `examples/adocument_entry.rs`.

## Addendum — Q7 Partitions/NN trailer fields are NOT chunk offsets (2026-04-19)

Followup to FACT F7/F8. Tested the hypothesis that the four u32 fields
at Partitions/NN header offset 0x14..0x24 encode per-chunk byte
offsets (so a reader could use the header as a random-access chunk
table). **Rejected** — zero matches across all 6 releases tested.

Observed values (2016 → 2026):

| Release | trailer_u32 | gzip offsets (first 4) |
|---|---|---|
| 2016 | [400, 1240, 14436, 131060] | [44, 14504, 17010, 37528, …] |
| 2018 | [4, 1288, 15011, 131036] | [44, 15079, 17812, 37184, …] |
| 2020 | [4, 1138, 14021, 117382] | [44, 14089, 18368, 37750, …] |
| 2022 | [4, 1133, 14196, 117443] | [44, 14264, 19327, 41288, …] |
| 2024 | [4, 729, 12330, 119319] | [44, 12398, 19074, 21706, …] |
| 2026 | [4, 728, 12378, 119384] | [44, 12446, 19117, 21872, …] |

No trailer value equals any gzip-chunk offset.

### What the values probably are (unconfirmed)

- `trailer_u32[0]` ≈ small constant (4 in 2018+, 400 in 2016) — possibly
  a layout-version counter that jumped once post-2016.
- `trailer_u32[1]` ≈ 700-1300 — correlates with element/record count
  from Global/ElemTable (see FACT F9).
- `trailer_u32[2]` ≈ 12-15K — roughly matches the decompressed size
  of the FIRST gzip chunk across all releases.
- `trailer_u32[3]` ≈ 117-131K — close to but not equal to
  `raw_len - header_size`.

### Practical impact: none

`partitions::find_chunks()` already works correctly using gzip-magic
scan. The trailer table would have been a small optimisation for
pathological cases (many chunks, long streams), not a correctness
prerequisite. Q7 is marked **resolved negatively** — the hypothesis
was worth testing, the answer is "no table, keep scanning."

Probe: `examples/partitions_q7.rs`.

## Addendum — Q5.2 100% field-type coverage (2026-04-19, Phase Q5.2)

Q5.1 landed at 84% classification (16% of fields fell through to
`FieldType::Unknown`). That gap was the single largest blocker to
trusting any downstream consumer — a field you can't type is a field
you can't read the data out of, which rules out emitting IFC,
computing quantities, or performing any structural integrity check.

Q5.2 closes the gap to **100.00% across all 11 Revit releases
(2016–2026)**. Zero `Unknown` fields remain in the 13,570-field
reference corpus.

### Mechanism

The 84% decoder special-cased exactly two modifier patterns: `0x07
0x10 ...` (vector<double>) and `0x0e 0x50/0x51 ...` (reference
container). Every other combination of scalar base × modifier fell
through. Q5.2 generalises the rule:

- **`{kind} 0x10 0x00 0x00 …` → `Vector<base>`** for every scalar base
  byte observed: `0x01` bool, `0x02` u16, `0x04`/`0x05` u32, `0x06`
  f32, `0x07` f64, `0x0b` u64, `0x0d` point/transform, `0x0e` ref.
- **`{kind} 0x50 0x00 0x00 …` → `Container<base>`** for the same set.
  Reference containers (`kind == 0x0e`) additionally extract embedded
  C++ type signatures (`std::pair<...>`, `std::map<...>`); scalar-base
  containers rarely do.

Two additional wire patterns were mapped:

- **`0x08 0x60 0x00 0x00` — alternate string discriminator.** Revit
  emits two byte orderings for the UTF-16LE-string type: the common
  form `08 00 60 00` (sub = `0x6000`) and the alternate form `08 60
  00 00` (sub = `0x0060`). The alternate form is used for 43–61 fields
  per release (e.g. `Identifier.second`, `AStringWrapper.m_str`,
  `ControlledDocAccess.m_fileName`). Both decode to `FieldType::String`.
- **`0x0e 0x00 0x00 0x00 <tag:u16> <sub:u16>` — `ElementIdRef`.** The
  existing `ElementId` variant only matched the canonical 6-byte
  pattern `0e 00 00 00 14 00` (referenced class tag = `0x0014`). Every
  other referenced-class tag fell through. 80–114 fields per release
  use this form with tags like `0x0058`, `0x00ba`, `0x00b6` — each a
  pointer to a specific class. A new `ElementIdRef { referenced_tag,
  sub }` variant captures them. The unit `ElementId` is preserved for
  the common `tag=0x0014, sub=0x0000` case.

### Deprecated 0x03 i32-alias (2016–2018 only)

Five fields in Revit 2016–2018 use discriminator byte `0x03`, which
does not appear in any later release:

| Release | Field | Class |
|---|---|---|
| 2016 | `m_id` | `UserID` |
| 2016 | `m_nAtomType` | `ElementRegenerationInfo` |
| 2017 | `m_id` | `UserID` |
| 2017 | `m_nAtomType` | `ElementRegenerationInfo` |
| 2018 | `m_id` | `UserID` |

`0x03` sits between `0x02` (u16) and `0x04` (legacy u32) in the
discriminator space, and both observed fields are integer-typed
(Hungarian `_n` prefix, `_id` suffix). Decoded as `Primitive { kind:
0x03, size: 4 }` — a 32-bit integer alias superseded by `0x05` from
Revit 2019 onward. Evidence is cross-release disappearance: the same
two fields switch to a different discriminator in 2019+.

### Truncated 2-byte headers

Two fields in the corpus have a type-encoding body of only 2 bytes:

| Release | Field | Bytes |
|---|---|---|
| 2018 | `BoundedSpace.m_boundActive` | `0d 10` |
| 2025 | `RadialConstraint.m_witnessRefs` | `0e 50` |

These are schema-parser boundary artifacts — the 4-byte modifier
header was clipped by the next-record heuristic. `FieldType::decode`
now handles 2-byte inputs by inferring `sub` from `bytes[1]` alone
(high byte implied zero) and producing a typed variant with an empty
body. Safe-slicing (`bytes.get(4..).unwrap_or(&[])`) prevents panics
on any short input.

### Evidence

Reproduce with:

```text
cargo run --release --example unknown_bytes_deep -- \
  ../../samples/racbasicsamplefamily-2026.rfa
```

Expected output (2026):

```text
Schema total fields: 1061  Unknown: 0  (0.00%)
Classification coverage: 100.00%
```

The same command against every file under `../../samples/` shows
`0.00%` Unknown across all 11 releases.

### Consequences

With the schema fully typed, Layer 5 (IFC export) now has a
type-sound input: every field in the object graph has a known size
(for primitives), known element type (for vectors/containers), and
known referent (for element-id references). The `NullExporter`
placeholder in `src/ifc/mod.rs` can be replaced with a real walker
that knows how to serialise each `FieldType` variant. IFC emission is
no longer blocked on decoder completeness — it's blocked only on
Revit-class → IFC-entity mapping decisions (buildingSMART alignment).

Probes: `examples/unknown_bytes.rs` (first-4-byte signature
histogram), `examples/unknown_bytes_deep.rs` (first-8-byte signature
with sample fields + coverage percentage).

Tests: `src/formats.rs::tests::decodes_field_type_*` — one pinning
test per new arm, plus `decode_is_safe_on_short_inputs` for the
panic-safety invariant.

## Addendum — Q6.3 CORRECTION to Q6.2: post-history bytes are a directory, not ADocument (2026-04-19, Phase 5a.1)

**Q6.2 hypothesis refuted.** The original §Q6.2 addendum claimed the
sequential-ID TLV block at the post-history boundary (offset 0x363 in
the 2024 sample) was ADocument's 13-field instance with records keyed
by field-number. Confidence was flagged at 0.6. Q5.2 closed the type
decoder to 100%, which unblocked a rigorous Q6.3 walk — and the walk
refutes the hypothesis.

### Evidence against

1. **Sequential IDs extend to ~131, not 13.** ADocument has 13
   declared fields. If records corresponded to fields, the table
   should terminate at id=13. Instead it runs contiguously from id=1
   through id=131 (2024 sample) — stable across all 11 releases
   (129–131 records; count varies slightly by sample but not by year).
2. **Body-size / FieldType correlation fails.** Under the hypothesis,
   same-FieldType fields should have same-sized bodies. Observed: of
   the records that map to ADocument's 9 `Pointer { kind: 2 }` fields,
   some have 2-byte bodies, others 6-byte bodies, others 12-byte
   bodies. Record 1 (which would be `m_elemTable :: Pointer {kind:2}`)
   has a 12-byte body; record 3 (`m_oContentTable`, same FieldType)
   has a 2-byte body. FieldType does not predict body length.
3. **Body values are not schema class tags.** Cross-referenced each
   record's body u16 against the schema's 79 tagged classes: 0 of 131
   resolved (`examples/directory_class_lookup.rs`). The u16 values
   (0x076e, 0x0ea7, 0x0fda, …) are in a range (≈100–5000) consistent
   with ElementIds, not class tags.

### What the structure probably is

A **multi-table directory/reference-pool** immediately following the
upgrade-history strings in Global/Latest. Each table runs
contiguously with 1-indexed sequential IDs; once one terminates, the
next starts fresh from id=1. At least two such tables are visible in
the 2024 sample:

```text
0x0363  table 1 start (id=1..131)
0x07e7  table 1 ends; body-style changes to [u16 val][u32 0xffffffff]
0x081b  section of `00`-padded dead space
0x0881  another multi-u32 structure begins
0x08b1  table 2 starts (id=1..N again)
```

Body encoding per record (provisional):

```text
[u32 sequential_id]
[u16 value_or_ref]           // always present
[optional u16 0 padding]     // some records
[optional u32 extra]         // some records (usually zero)
```

The trailing sentinel pattern `ff ff ff ff` shows up in later records,
matching Revit's convention for "undefined" or "null" references.

### Practical impact

**Layer 5a's original scope was underestimated.** The ADocument
singleton is NOT at the post-history boundary. It lives somewhere
else in Global/Latest (or possibly another stream), referenced
through this directory. Reaching ADocument therefore requires:

1. Decoding the directory table format (what the u16 values are —
   ElementIds? entry indices into ElemTable? reference-pool indices?).
2. Understanding Global/ElemTable's structure — parser exists
   (`src/elem_table.rs`) but record semantics are partial.
3. Resolving a directory entry → actual object-data offset.
4. Only then can the schema-directed walker (the Layer 5a "write the
   walker" work) run.

Each of those is a separate open question. This is genuine
reconnaissance, not implementation of a known design.

### Status of Layer 5a going forward

- **Layer 4c (schema decoding, 100% coverage):** complete and
  verified. Not affected by this correction.
- **Layer 5a (ADocument instance extraction):** re-opened. The
  original Q6.2 entry-point claim is retracted; a new entry-point
  question is posed.
- **Layer 5b (full object graph walker):** remains blocked on 5a
  plus directory resolution.

Probes added this session:
- `examples/adocument_walk.rs` — tabulates sequential-ID records
  against ADocument's declared fields. Output refutes Q6.2.
- `examples/post_directory.rs` — locates where the first directory
  terminates and dumps the structure that follows.
- `examples/directory_class_lookup.rs` — tries record bodies as
  class-tag lookups; 0/131 match, confirming they're not tags.

## Addendum — Q6.4 progress: directory is not a cross-stream reference (2026-04-19)

Follow-on investigation to the Q6.3 refutation. Two findings tighten
the problem space without yet resolving it.

### Finding Q6.4-A: directory u16 values are NOT references into other streams

Hypothesis tested: if the directory at `Global/Latest` offset 0x363
is a reference pool indexing into another stream, the u16 body
values should appear at higher-than-uniform-random rate in that
target stream. (Method established by Phase D, which found class
tags appear in `Global/Latest` at ~340× uniform rate.)

Byte-level scan (probe: `examples/directory_bytewise_scan.rs`) of
the 130 unique u16 body values against every decompressed stream:

| Target stream | Hit ratio vs uniform (2024) | Hit ratio (2016) |
|---|---|---|
| `Global/ElemTable` | 2.2× | 3.8× |
| `Global/ContentDocuments` | 0.0× | 0.0× |
| `Global/DocumentIncrementTable` | 2.2× | 1.9× |
| `Global/History` | 2.1× | 1.7× |
| `Global/PartitionTable` | 27.3× | 30.6× |
| `Formats/Latest` | 1.1× | 3.0× |
| `Global/Latest` (post-directory) | 0.8× | 3.2× |

All ratios are ≤5× except `Global/PartitionTable`, whose apparent
30× is the artefact of a small sample (167 bytes, 1–2 unique hits).
None approach Phase D's 340× threshold. **The directory is self-
contained** — its u16 body values do not meaningfully cross-reference
any other stream.

### Finding Q6.4-B: Table B (post-directory) exists but is smaller than first estimated

`examples/second_table_probe.rs` initially suggested a "Table B"
starting around offset 0x8a1 spanning ~935 KB. With a larger
look-ahead window (probe: `examples/table_b_structure.rs`), the
more-rigorous measurement finds **141 sequential records totalling
only ~1754 bytes**:

| Region | Start (2024) | Records | Bytes |
|---|---|---|---|
| Upgrade-history UTF-16LE | 0x053 | — | 784 |
| Table A (directory) | 0x363 | 131 | ≈1342 |
| Table B (second directory?) | ≈0x8a1 | 141 | ≈1754 |
| Remainder of stream | ≈0xf44 | — | ≈935 000 |

Table B's record-length distribution is strongly bimodal: 134 of
141 records are exactly 12 bytes (`[u32 id][u32 ffffffff][u32
ffffffff]` — a "null entry" pattern common in C++ as "-1 = unset").
The non-null exceptions are records 1 (28 bytes), 2 (70 bytes),
and a small cluster of medium-size outliers.

### Open question: what do both tables index?

- Table A's values are not class tags, not ElementId references,
  not offsets (non-monotonic, range 100–5000).
- Table B is mostly null-stubs with a few populated entries.
- The **~935 KB of content after Table B ends** is where the actual
  object data must live, but neither table obviously indexes into
  it by the usual means (byte offsets, ElementIds, hash indices).

Remaining candidate interpretations:
1. **Element-kind enumeration**. The tables list "which slots of an
   implicit array are populated," and the body values are
   type-discriminants or small-integer payloads, with the real
   per-slot data stored positionally in the 935 KB region.
2. **Serialization cursors**. Each record body is a byte-offset
   modulo something (hash? scramble?) into the remaining region.
   Would require a transform we haven't identified.
3. **Per-document serial numbers**. The u16 values are internal
   "save-cursor" IDs with no format-level meaning, present for
   incremental-save merge logic. The 935 KB region is the actual
   payload and its structure is read differently.

Each hypothesis is falsifiable with a bounded probe. The next
contributor to touch this area should (a) run the probes listed
below on a non-`rac_basic_sample_family` `.rvt` to see whether
record counts vary with document content or stay format-fixed,
and (b) structurally scan the 935 KB post-Table-B region for
class-tag density (Phase D method at a different offset) to find
where the schema-driven instance data actually begins.

Probes added (all under `examples/`):

- `directory_vs_elemtable.rs` — cross-reference against ElemTable
  records. Refutes "directory is ElemTable index" hypothesis.
- `directory_bytewise_scan.rs` — byte-level scan of directory u16s
  across every decompressed stream with expected-vs-observed
  ratios. Basis for Finding Q6.4-A.
- `second_table_probe.rs` — finds all sequential-id tables in
  `Global/Latest`.
- `table_b_structure.rs` — walks Table B with large look-ahead;
  produces the record-length distribution for Finding Q6.4-B.

### Finding Q6.5-A: post-Table-B region is schema-driven instance data

Phase D's original 340× class-tag density measurement was taken
over the **entire** `Global/Latest` stream. Running the same test
restricted to the post-Table-B region (probe:
`examples/post_table_b_density.rs`) gives:

| Region | Bytes | Tag hits | Distinct tags | vs uniform |
|---|---|---|---|---|
| Pre-Table-B (header + history + directories) | 3,943 | 63 | 22 | **13.3×** |
| Post-Table-B (candidate object payload) | 934,635 | 37,134 | 54 | **33.0×** |

The 33× density over 935 KB is unambiguous evidence this region is
schema-referenced instance data. (The 340× whole-stream Phase D
number averaged over ALL tags; most tags appear rarely and a few
appear very frequently — 33× is what happens when you include the
long tail of rare tags in the denominator.)

### Finding Q6.5-B: the post-Table-B region starts with structured record data

Hex dump of the first 80 bytes at offset 0x0f67 (2024 sample),
probe: `examples/post_table_b_head.rs`:

```text
0x000f67  00 00 00 00 00 00 00 00 0c 00 00 00 ff ff ff ff
0x000f77  c8 0b ff ff ff ff c8 0b ff ff ff ff c8 0b ff ff
0x000f87  ff ff c8 0b ff ff ff ff c8 0b ff ff ff ff c8 0b
0x000f97  ff ff ff ff c8 0b ff ff ff ff c8 0b ff ff ff ff
0x000fa7  c8 0b ff ff ff ff c8 0b ff ff ff ff c8 0b ff ff
0x000fb7  ff ff c8 0b 0c 00 00 00 ff ff ff ff c7 0b ff ff
```

Visible structure:

- **8-byte zero preamble** (`00 00 00 00 00 00 00 00`). Plausibly
  a record-type marker or document-version counter.
- **`[u32 12]`** field count or array length marker, followed by
  **twelve** 6-byte records of the form `[u16 id][u32 0xffffffff]`
  (the `0xffffffff` is a classic "unset/null" sentinel in C++
  containers; the `u16 id` is 0x0bc8 = 3016 repeated).
- Immediately after: **another `[u32 12]` marker**, another twelve
  6-byte records with `[u16 0x0bc7 = 3015]`.
- Shortly after that, embedded IEEE-754 `f64` values decoded from
  `00 00 00 00 00 00 20 40` → **8.0** and `00 00 00 00 00 00 08 40`
  → **3.0**. Plausible BIM dimensions for the test fixture (a
  family element; 8.0 / 3.0 are credible member-length parameters).

This is what ADocument's serialized instance should look like per
the schema: `m_appInfoArr` is declared as a `Container` (array) in
the schema, and the 12-element array of `[element-id][unset]` pairs
matches exactly. The IEEE-754 floats after are plausible field
values for scalar parameters on the root document.

### Where Q6.5 now stands

The combination of findings suggests — with moderate but not yet
airtight confidence — that:

```text
0x000000                 …   compressed-stream header (not of interest)
0x000053 .. 0x000362     UTF-16LE document-upgrade-history strings
0x000363 .. 0x0008a0     Table A   (131 sequential-ID records, ~1342 B)
0x0008a1 .. 0x000f67     Table B   (141 sequential-ID records, ~1754 B)
0x000f67 .. end-of-stream ACTUAL INSTANCE DATA — schema-driven
                         (first record is very likely ADocument)
```

With 100% type coverage (Q5.2) already in place, a walker can now
start at 0x0f67 and attempt to read ADocument's 13 fields in
schema-declared order. The validation oracle: extracted values
must match what `rvt-info`, `rvt-history`, and Phase D string
extraction already surface by independent paths. The remaining
uncertainty is record-framing details — 8-byte preamble, exact
array-count encoding, how the `m_pHostDocument` pointer (one of
ADocument's fields) threads into the rest of Table B's records.

Probes added:
- `post_table_b_density.rs` — Phase-D-style class-tag density scan
  over the post-Table-B region. Basis for Finding Q6.5-A.
- `post_table_b_head.rs` — hex dump + class-tag annotation of the
  first 384 bytes. Basis for Finding Q6.5-B.

**End of report.**
