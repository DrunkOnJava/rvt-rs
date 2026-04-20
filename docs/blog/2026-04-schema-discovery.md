# Cracking open a .rvt without the Revit API

Revit files are black boxes. Autodesk ships a C# API that reads them
from inside a running copy of Revit on Windows, and for a quarter
century that has been the only supported path. Want to count walls,
diff two revisions, push geometry elsewhere? Buy a Windows VM,
install Revit, write plugins against the API.

This post walks through how `rvt-rs` cracks one open from the outside
— no Autodesk code, no decompiled DLLs, no API. Punch line: every
Revit file since 2016 carries a complete, machine-readable description
of its own on-disk format. You just have to read it.

## What's actually inside a .rvt

A `.rvt` (or `.rfa`, `.rte`, `.rft`) is a Microsoft Compound File Binary
Format 3.0 container. First eight bytes give it away:
`D0 CF 11 E0 A1 B1 1A E1` — the OLE2 magic every `.doc` from 1997 shares.
The `cfb` crate on crates.io enumerates the streams inside.

Twelve of those streams are invariant across every Revit release 2016
through 2026 — `BasicFileInfo`, `Formats/Latest`, `Global/Latest`,
`Global/PartitionTable`, and so on — plus one version-keyed
`Partitions/NN` (58 for 2016, 60 for 2017, up to 69 for 2026; 59 is
skipped). Each stream is compressed, but not quite with standard gzip:
Revit writes a valid ten-byte gzip header followed by raw DEFLATE, and
then omits the trailing CRC32+ISIZE that RFC 1952 requires. Standard
gzip parsers refuse these streams because the trailer is wrong. You
skip the header manually, feed the body through `DeflateDecoder`, and
you're done. `src/compression.rs` handles that.

So: open the container, pick a stream, skip ten bytes, raw-inflate.
You now have a buffer whose shape you don't yet know.

## The Formats/Latest stream

Here is the payoff. Every Revit file embeds, in a stream literally
named `Formats/Latest`, the complete serialization schema for its own
contents. Every class name. Every field name on every class. Every
field's C++ type — full STL generics like
`std::pair< ElementId, double >`. Every class's parent. Every class's
serialization tag — the 15-bit integer that identifies that class in
the Revit object graph.

In effect a bundled `.proto` for the entire Revit serialization format,
shipped inside every file that uses it. Once you have it parsed, you
don't need to know anything about the internal shape of the RevitAPI.
The file tells you how to read itself.

The header comment in `src/formats.rs` spells out the wire format as
inferred from an eleven-release corpus:

```rust
//! [uint16 LE name_len] [name_len bytes ASCII class_name]
//! [uint16 LE type_tag]                     // bit 0x8000 = flag; low byte = secondary length
//! [padding zeros]                          // variable — see field parser
//!
//! Followed by a field table. Each field entry:
//!
//! [uint16 LE fieldname_len] [fieldname_len bytes ASCII field_name]
//! [uint16 LE typename_len]  [typename_len bytes ASCII cpp_type]    // optional
```

Class IDs are UUIDv1s; their MAC suffixes match known Autodesk
workstation signatures from circa 2000 — strong evidence the schema
format has been stable since Revit was first built.

## How rvt-rs walks it

The parsed schema ends up in these types in `src/formats.rs`:

```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchemaTable {
    pub classes: Vec<ClassEntry>,
    /// Every unique C++ type signature seen in the schema (e.g.
    /// `std::pair< ElementId, double >`, `ElementId`, `Identifier`).
    pub cpp_types: Vec<String>,
    /// Raw count of parse-candidates skipped for validation reasons.
    pub skipped_records: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClassEntry {
    pub name: String,
    /// Stream offset where this class entry begins.
    pub offset: usize,
    /// Fields declared by this class (best-effort).
    pub fields: Vec<FieldEntry>,
    /// Serialization tag if this class has one set (u16, 0x8000 flag stripped).
    /// Absent = the class is not top-level serializable; it's an embedded type.
    pub tag: Option<u16>,
    /// Parent / superclass name if present. Determined by the `[u16 len][name]`
    /// block that follows the tag. For e.g. HostObjAttr → Some("Symbol").
    pub parent: Option<String>,
    /// Field-count value the schema itself declares (may disagree with
    /// `fields.len()` if the walker missed one).
    pub declared_field_count: Option<u32>,
    // ...
}
```

The parser is a linear scan. Start at byte 0, look for a
`[u16 length][length bytes ASCII]` that passes a class-name heuristic
(starts with uppercase ASCII, remaining chars alphanumeric or
underscore), and when one matches, read the rest of the record:

```rust
// Try to parse the tag word (u16) immediately after the name.
// If its 0x8000 bit is set, this is a TAGGED (top-level) class.
let raw_tag = u16::from_le_bytes([data[cursor], data[cursor + 1]]);
if raw_tag & 0x8000 != 0 {
    tag = Some(raw_tag & 0x7fff);
    cursor += 2;
    // Skip the 2-byte pad, then read u16 parent-name-length.
    if cursor + 4 <= data.len() {
        let pad = u16::from_le_bytes([data[cursor], data[cursor + 1]]);
        let plen = u16::from_le_bytes([data[cursor + 2], data[cursor + 3]]) as usize;
        if pad == 0 && (3..=40).contains(&plen) && cursor + 4 + plen <= data.len() {
            let p = &data[cursor + 4..cursor + 4 + plen];
            if looks_like_class_name(p) {
                // ... validate via the following preamble ...
```

Two subtleties the 64 KB scan limit and preamble validation protect
against. First: `Formats/Latest` is larger than the schema — beyond
~64 KB the stream contains binary object data whose byte patterns
occasionally look like class-name records. Cap the scan. Second: a
subclass with few own fields can be read past, bleeding into the next
class's field list. Before committing a parent reference, we peek at
the `[u16 flag][u32 field_count][u32 field_count_dup]` preamble that
follows; only if the two counts agree and the flag looks sane do we
record the parent and advance.

A second pass then synthesizes stub entries for every parent that was
referenced but never appeared with its own top-level declaration —
keeps the table closed over the class graph.

Field types aren't stored as strings in the common case; they're short
byte patterns that the `FieldType::decode` enum unpacks. A Phase 4c
sweep across 13,570 fields in an 11-release corpus mapped every
observed byte pattern to one of eleven discriminators — primitive
scalars, UTF-16 strings, GUIDs, `ElementId` references (with and
without a referenced-class tag), pointers, vectors, and associative
containers with embedded C++ signatures. 100% classification, zero
`Unknown` entries in the regression corpus.

## Why this matters

Until now, every serious Revit tool chain has gone through the Revit
API. That means a Windows VM, a Revit license, and whatever the C#
API chooses to expose at any given version. IFC output from Autodesk's
own exporter is routinely described in the openBIM community as
"very limited," "data loss," and "out of the box, just crap."

The schema work here is the dictionary a byte-level reader needs.
Given `ClassEntry` with its tag, parent, declared field count, and
typed field list, a downstream tool can walk `Global/Latest` instance
data without calling any Revit function. BIM analytics, lossless
migration, geometry extraction — on a Mac or a Linux server, no
Autodesk install, no C# runtime, and no proprietary code. `rvt-rs`
uses only Apache-licensed Rust crates and facts extracted by running
clean-room parsers against files users supplied; `CLEANROOM.md` has
the formal source-hygiene policy.

One fact worth noting about how far this goes: class tags from
`Formats/Latest` appear in `Global/Latest` at roughly 340× the
uniform-random rate. The schema doesn't just describe the
serialization format — it *indexes* the data. The file tells you
where its own content is.

## What's hard

Three things resist the clean parse.

Version drift. Class shapes aren't stable across releases. Tags drift —
`AbsCurveGStep` moved from 0x0053 in 2016 to 0x0066 in 2026 because
nineteen new A-class entries were inserted alphabetically over the
decade. Field layouts change too. The parser's preamble validation
handles this structurally, but a decoder wanting semantic field access
has to know the release. `docs/data/tag-drift-2016-2026.csv` publishes
the full 122-class × 11-release table — first public version of that
dataset.

Field-type gaps. The `FieldType` enum covers every pattern observed in
the reference corpus, but special-cased streams don't follow the
pattern — the 167-byte `Global/PartitionTable`, the metadata chunks
inside `Contents`, the Atom XML of `PartAtom`. Those get their own
parsers.

Revit 2021 was an undocumented format transition. `Global/Latest` grew
27× in that release, and a new Forge Design Data Schema landed
simultaneously in `Partitions/NN`. Readers built against 2016–2020
silently drop data on 2021+ files.

## Try it

If you have a `.rvt` or `.rfa` sitting around:

```bash
cargo install rvt --locked
rvt-schema --grep Wall path/to/your.rfa
```

You'll get every class whose name contains "Wall" — `Wall`,
`WallType`, `WallSweep`, `WallFoundation`, etc. — with tag, declared
field count, field names, and C++ type signatures. `--format json`
for machine-readable output; `--top 20` for the twenty largest
classes in the file.

Or in Python:

```python
import json, rvt
f = rvt.RevitFile("my-project.rfa")
schema = json.loads(f.schema_json())
for c in schema["classes"]:
    if c["name"] == "Wall":
        print(c["tag"], c["fields"])
```

Source and issues at [github.com/DrunkOnJava/rvt-rs](https://github.com/DrunkOnJava/rvt-rs).
Apache-2.0, clean-room, not affiliated with Autodesk. If you're
building BIM tooling and want an Apache-licensed Revit reader to
compose into your stack — or you have a file that breaks the parser
— open an issue.
