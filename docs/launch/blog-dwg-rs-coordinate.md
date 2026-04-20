# Reading the two proprietary formats that run building design — dwg-rs + rvt-rs

*Draft blog post — April 2026. To be published under DrunkOnJava's
personal blog / Medium / dev.to.*

---

AutoCAD's `.dwg` format has been the dominant 2D drafting exchange
format since 1982. Autodesk Revit's `.rvt` has been the dominant 3D
BIM (Building Information Modeling) format since 2000. Between them,
they cover roughly 80% of the professional construction industry's
digital design output.

Neither is openly documented. Autodesk has never published a spec
for either. For `.dwg` there's an unofficial reverse-engineered
spec maintained by the [Open Design Alliance](https://www.opendesign.com/)
(ODA) — useful, but their SDK is commercial and its license
prohibits clean-room derivative work. For `.rvt` even the unofficial
side is sparser — until now no public documentation covered the
full internal schema.

Over the last ~60 days I wrote two Apache-2 Rust libraries to read
both formats directly from bytes: **[dwg-rs](https://github.com/DrunkOnJava/dwg-rs)**
and **[rvt-rs](https://github.com/DrunkOnJava/rvt-rs)**. This post
covers what they do, what they don't, and what unblocks when both
exist in the open.

## What dwg-rs reads

DWG's container is a custom binary format with its own LZ77 variant
(not standard deflate), a 128-byte file header carrying the version
magic (`AC1032` for AutoCAD 2018+, `AC1027` for 2013, `AC1024` for
2010, back to `AC1.0` for 1986), and a section table indexed by
GUID that groups related records: Header, Preview, AppInfo,
FileDepList, SummaryInfo, AcDb:AcDbObjects, etc.

dwg-rs v0.1 ships:

- Full identification + LZ77 decompression across R13 → AC1032
  (2018+) — 21 format versions.
- Section extraction with reassembly for multi-page sections
  (`AcDb:Preview` 1.5 KB, `AcDb:Header` 870 B, `AcDb:AcDbObjects`
  1.15 MB typical for mid-size drawings).
- Metadata parsers: SummaryInfo (Title / Author / Keywords /
  Comments), AppInfo, FileDepList with UTF-16 auto-detection,
  Preview with carved PNG thumbnail extraction.
- `ObjectType` enum covering 80+ DWG object-type codes.
- HandleMap + ClassMap parsers — the indirection layer DWG uses
  to reference objects across sections.

What dwg-rs does NOT do yet:

- Per-entity field decoders. Every entity object's body bytes are
  identified and extracted, but the semantic fields inside them
  (LINE start/end points, CIRCLE center/radius, 3DSOLID brep) are
  still raw bytes.
- Write path. Read-only today.
- RS-FEC on R2013+ — the error-correcting-code layer that ODA's
  SDK handles silently. dwg-rs reads R2013+ files that haven't
  been corrupted, but doesn't repair bit rot.
- R2007 full decode — the page-key-encrypted sections in the 2007
  format use an obfuscation layer that's only partially decoded.

## What rvt-rs reads

Revit's container is Microsoft Compound File Binary (OLE2) — the
same format Word 97-2003 `.doc` files use — but every internal
stream is compressed with *truncated gzip*: a standard 10-byte
gzip header followed by raw DEFLATE, without the trailing CRC32 +
ISIZE that conforming gzip writers emit. Standard gzip parsers
refuse these streams. The compression alone blocked most prior
reverse-engineering efforts.

rvt-rs v0.1.2 ships:

- OLE container open + truncated-gzip decompression.
- The full `Formats/Latest` schema: **395 classes, 13,570 fields,
  100% type classification** across every Revit release from 2016
  through 2026. That's the dictionary — what classes exist, what
  fields each class has, and how each field's bytes encode. No
  field falls through to `FieldType::Unknown`.
- 72 typed per-class decoders: walls, floors, roofs, ceilings,
  doors, windows, columns, beams, stairs, railings, rooms,
  furniture, **11 MEP classes** (ducts, pipes, fixtures,
  equipment), annotations, parameters, styling, project
  organization, drafting, curtain walls, levels, grids,
  reference planes.
- Schema-directed `ADocument` walker across all 11 releases — the
  document-level instance data that contains project-wide pointers
  (Element IDs, handles, ref containers, raw bytes fields).
- **rvt → IFC4 export** (`rvt-ifc input.rfa input.ifc`): structural
  hierarchy (project / site / building / storey) plus per-element
  IFC4 entities with extruded / revolved / boolean / faceted-brep
  geometry, material layer sets + profile sets, property sets with
  area / volume / count / time / weight quantities, openings with
  void / fill relationships. Pure Rust — no IfcOpenShell or ODA
  runtime dependency. Output opens cleanly in IfcOpenShell and
  BlenderBIM.
- Python bindings via pyo3 / maturin (`pip install rvt`) — single
  abi3-py38 wheel per OS/arch, ships PEP-561 type stubs so editors
  autocomplete.
- 357 library tests + 3-layer IFC validation CI gate (Rust-side
  structural checks + IfcOpenShell independent parse + 357 unit
  tests per commit, ubuntu/macOS/windows).

What rvt-rs does NOT do yet:

- Per-element geometry *extraction* from the object graph. Location
  curves, profile shapes, and arbitrary brep geometry don't yet
  flow from the walker into the IFC bridge — the extrusion / solid
  helpers in `from_decoded.rs` take caller-supplied dimensions for
  exactly this reason. Adding the pointer-walking layer is the
  open frontier.
- Write path. Byte-preserving stream-level patching round-trips
  (13/13 streams identical on the 2024 sample), but field-level
  semantic writes (edit a specific Wall's height and round-trip
  to a Revit-openable `.rvt`) don't ship yet.
- Format versions before Revit 2016.

## Why both at once

**AEC (architecture / engineering / construction) workflows almost
always touch both formats.** An architect ships a Revit model to
a structural engineer; the engineer's Tekla or AutoCAD Civil 3D
workflow produces DWG details; the general contractor consumes
both. Any "open the full project programmatically" pipeline needs
to ingest both sides, and until now both sides had been closed to
non-Autodesk / non-ODA toolchains.

Concrete examples of what unblocks with these libraries:

### 1. License-free interoperability pipelines

A coordination tool that reads a Revit `.rvt`, extracts the
building element metadata, cross-references against a DWG
civil-grading plan's georeferencing, and flags site-level clashes.
Before: requires either Revit + AutoCAD licenses on a build server
(~$5K/yr/seat each) or an ODA license (commercial, license fee +
per-machine activation). After: static Rust binaries, zero
per-machine cost, hostable on any Linux box.

### 2. Archival readers for legacy files

AEC firms have decades of archived DWG / RVT files. Autodesk's
archive policy is LIFO — they support opening files roughly 3
versions back; anything older requires version-migration dances
through intermediate releases that eventually require paid
subscriptions. dwg-rs reads back to R13 (1994); rvt-rs reads back
to 2016.

### 3. Independent IFC exporters

Autodesk's own IFC exporter is maintained as an optional plugin
and historically has had ... let's call them *spec-conformance
variations*. BuildingSMART community reports drift from the IFC4
reference implementation every few releases. rvt-rs's Rust STEP
writer is 100% spec-directed (no intermediate IfcOpenShell
dependency) and the 3-layer validation gate catches drift on
every commit. It's not certified-software status — that requires
a formal buildingSMART cycle — but the evidence trail is public
and reproducible.

### 4. Programmatic forensic analysis

"What did this drawing look like before the contractor changed
the callouts?" "What parameter values live on this family type?"
"Are these two Revit files byte-compatible at the element level?"
Today these require Revit/AutoCAD running interactively. With
dwg-rs + rvt-rs, they're shell-pipeable.

### 5. Research + education

Open implementations of two of the most important binary formats
in professional computing. The reverse-engineering pipeline
itself — hypothesis → byte probe → schema → validation — is
documented in both projects' reconnaissance reports. If you're a
CS student, binary-format researcher, or just someone who wants
to know what the format LOOKS like on disk, these are now open
questions instead of closed ones.

## The reverse-engineering cadence

Both projects grew in the same rhythm: ship findings, validate,
publish corrections. rvt-rs's recon report includes a public
refutation of its own earlier hypothesis about where `ADocument`
starts in `Global/Latest` (§Q6.3, §Q6.5). dwg-rs's log has
analogous corrections on the R2007 page-key derivation and the
AC1032 handle-map framing.

Neither library is a triumphalist "we solved it" write-up.
They're both more like field journals — dated hypothesis,
contradicting evidence, revised hypothesis, reproducible probe.
When a future reader asks *how do we know this*, the answer
should be traceable to a commit, a probe example, and a
corpus sample.

## What next

dwg-rs and rvt-rs are Apache-2 and actively welcome contribution.
The lowest-hanging fruit in each:

- **dwg-rs**: per-entity field decoders. LINE, CIRCLE, POLYLINE,
  INSERT are the first four and would unlock ~60% of typical
  drawing content by entity count.
- **rvt-rs**: object-graph traversal beyond ADocument. The
  `m_elemTable` pointer is decoded; the individual element
  byte-offset table it points into is the next research target.

If you work in AEC tooling, openBIM, IfcOpenShell, FreeCAD,
BlenderBIM, or just have an interest in reverse-engineering
meaty proprietary formats, both repos have `good-first-issue`
labels and the reconnaissance reports explain how new findings
land.

Code:

- [github.com/DrunkOnJava/dwg-rs](https://github.com/DrunkOnJava/dwg-rs)
- [github.com/DrunkOnJava/rvt-rs](https://github.com/DrunkOnJava/rvt-rs)

— Griffin Long, April 2026
