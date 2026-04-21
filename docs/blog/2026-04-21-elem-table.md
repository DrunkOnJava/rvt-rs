# The ElemTable record layout shifts between family and project files

**Draft — 2026-04-21.** Pending review before publication.

One of the quieter milestones in reading a `.rvt` without Revit is
enumerating the elements. You open the CFB container, decompress the
streams, parse the schema, walk the 13-field `ADocument` record — and
then you need a list of element IDs, because the whole point of reading
the file is to do something with those elements.

That list lives in `Global/ElemTable`. In family files (`.rfa`) it's
been stable for five years: a 48-byte header followed by 12-byte
records. Three `u32` fields each. Scan until you hit a `0xFFFF_FFFF`
sentinel and stop. Easy.

Until you open a real `.rvt` project file and get two records back on a
stream that claims to hold 26,425.

This is the reverse-engineering path from "two records, probably a bug"
to "oh, the layout shifts per file variant, here's the fix."

## The broken parser

`elem_table::parse_records_rough` on the 11-release family corpus
returned 45 records on the 2024 sample — the declared count from the
header. Run it against a real Revit 2024 project file (a 34 MB building
model with 1,411 elements), and it returns two.

The header parses cleanly: `element_count = 1411`, `record_count =
26,425`. So the file is claiming 26,425 records in a 1 MB decompressed
stream. And the parser sees two.

Something is wrong with the scanning logic.

## Hex-dump the actual bytes

The existing parser had two suspect behaviours:

1. It scanned for the `0xFFFF_FFFF` sentinel starting at byte zero.
2. It assumed 12-byte records.

Pulling the first 160 bytes of the decompressed stream told the story.
On the family 2024 sample:

```
0x0000  83 05 b7 07 00 00 00 00 00 00 00 00 00 00 00 00
         └─ 0x0583 = 1411 (element_count)
               └─ 0x07B7 = 1975 (record_count)
0x0020  00 00 11 00 00 00 00 00 00 00 00 00 00 00 01 00
               └─ header_flag = 0x0011 at offset 0x22
0x0030  00 00 00 00 00 00 3f 00 00 00 3f 00 00 00 3f 00
         └─ record-area begins here
```

This is the expected family-file shape. Records start at `0x30`, no
per-record marker, 12-byte stride (`3f 00 00 00` repeating).

Now the Revit 2023 project file (`.rvt`, 913 KB):

```
0x0000  5a 05 37 0a 00 00 00 00 00 00 00 00 00 00 00 00
         └─ 0x055A = 1370 (element_count)
               └─ 0x0A37 = 2615 (record_count)
0x0010  00 00 00 00 00 00 00 00 00 00 00 00 00 00 ff ff
                                                    ├─ record 0 marker
0x0020  ff ff 01 00 00 00 01 00 00 00 00 00 00 00 00 00
         ┘   └─ id_primary = 1       └─ id_secondary = 1
0x0030  00 00 00 00 00 00 00 00 00 00 ff ff ff ff 02 00
                                      └─ record 1 marker
0x0040  00 00 02 00 00 00 00 00 00 00 00 00 00 00 00 00
              └─ id_primary = 2
```

Records start at `0x1E`. Each record is **28 bytes**. Every record
begins with a 4-byte `FF FF FF FF` marker.

And the Revit 2024 project file (`.rvt`, 34 MB):

```
0x0000  83 05 39 67 00 00 00 00 00 00 00 00 00 00 00 00
               └─ 0x6739 = 26,425 (record_count — 13× family scale)
0x0020  00 00 ff ff ff ff ff ff ff ff 00 00 00 00 01 00
               └─ record 0 marker (8 bytes of 0xFF)
0x0050  ff ff 00 00 00 00 02 00 00 00 00 00 00 00 00 00
              …payload…  └─ id_primary = 2
```

Records start at `0x22`. Each record is **40 bytes**. Every record
begins with an **8-byte** `FF` marker.

Three variants, three different layouts. The pre-probe parser's
sentinel scan was treating the *first* `FF FF FF FF` pattern as the
end-of-records sentinel. On the 2023 project, that's at offset `0x1E`
— the start of record 0. Parser reads zero records. On the 2024
project, the first `FF` sequence past the arbitrary `0x30` start
cutoff is at `0x4A` — the start of record 1. Parser reads one record.

## The fix

The record layout is self-describing: locate the first two consecutive
`FF` markers in the stream, take their spacing. Stride 28 → 4-byte
marker, stride 40 → 8-byte marker. Fall back to the implicit 12-byte
layout at `0x30` when no markers are present (family files).

That's what `detect_layout` does now, and it fits in about 30 lines of
Rust. The record count comes from the header; the layout determines
the stride; the walker emits `(id_primary, id_secondary, payload)`
tuples until the declared count is reached.

Result: the 34 MB project file that returned 2 records now returns all
26,425, with clean sequential IDs starting at 1. The 913 KB 2023
project returns 2,614 (one short of the declared 2,615 — probably a
trailer record, tracked as an open question in the docs). Family files
still work via the 12-byte implicit fallback.

## What the payload doesn't tell us

Once the parser works end-to-end, the natural follow-on question is
"where in `Global/Latest` does each of these elements live?" The
walker → IFC emission path needs `id_primary` → byte-offset mapping to
actually decode instances.

A second probe dumped the 16 / 28 payload bytes across all 29,039
records on the two project files and looked for u32s that fall inside
the valid `Global/Latest` offset range. The answer is no: at any given
4-byte position in the payload, 3-4% of records have a value that
*could* be an offset, which is no better than random. The payload on
the first N element-index records is overwhelmingly zero.

So `Global/ElemTable` tells you *what* IDs exist, not *where* they
live. The offset binding must come from scanning `Global/Latest`
itself — reading each schema-described class header, pulling each
element's self-id field, and building a `HandleIndex` as you go.

## What changes, concretely

- `elem_table::parse_records(&mut RevitFile) -> Vec<ElemRecord>` now
  returns the full declared record set on all three observed file
  variants. The old `parse_records_rough` kept for backward compat.
- `elem_table::declared_element_ids(&mut RevitFile) -> Vec<u32>` for
  coverage-validation downstream: diff against the walker's
  `HandleIndex` to find declared-but-not-located elements.
- A new `rvt-elem-table` CLI (the 14th shipped binary) dumps the
  detected layout + first N records for any `.rvt` or `.rfa`:
  ```
  $ rvt-elem-table /path/to/project.rvt --limit 5
  Global/ElemTable · /path/to/project.rvt
    declared element_count=1411  declared record_count=26425
    header_flag=0x0000  decompressed=1059812 B
    parsed records: 26425 (of 26425 declared)
    layout: Explicit (40 B stride, 8-byte FF marker)  first record offset: 0x22
  ```
- Python bindings: `RevitFile.elem_table_header()`,
  `elem_table_records()`, `declared_element_ids()`.
- New Q-07 criterion bench shows `parse_records` takes 5.6 ms on a
  26,425-record 34 MB file — about 212 ns per record.

Full hex-dump evidence + open questions are in
[`docs/elem-table-record-layout-2026-04-21.md`](../elem-table-record-layout-2026-04-21.md).

## What this doesn't do

- **It doesn't wire the walker through to IFC4 element emission.**
  That still needs the `Global/Latest` scanner, which decodes each
  class instance via the schema and records `(id, offset)` into
  `HandleIndex`. A real multi-session feature, not a same-day probe.
- **It doesn't handle the trailer-region records on project 2024.**
  The last ~50 of 26,425 records on the 2024 sample use a different
  byte pattern — probably type-definition rows or version-history
  entries packed into the same stream. Treated as open work in the
  RE notes.
- **It doesn't claim the GUID from `Global/PartitionTable` is stable
  across file variants.** That earlier claim holds on family files
  across 2016–2026, but project files carry distinct GUIDs per file
  (corrected in the README during the corpus probe).

## Why this matters for the broader project

rvt-rs's thesis is that Revit ships the schema inside every file, and
that lets an external parser read any future release without
Autodesk's cooperation. That thesis held on the family corpus. The
project-file corpus probe (3 files; 10+ wanted) pressure-tests it in
two useful ways: the per-record `FF` framing was new, and the absence
of a `header_flag` on project files forced the parser to degrade
gracefully rather than error. Both paths now have regression tests
and graceful fallbacks.

The 10-minute lesson: scope-creep on RE is a lie. What looks like a
hard problem ("decode this 34 MB project file") has a 3-step path
once you let the corpus talk — detect the layout, use the declared
count, walk the records. The hours go into proving the pattern holds
by staring at hex, not into writing parser code.

*— Next: close the walker → `HandleIndex` loop by scanning
`Global/Latest` with schema direction, then diff the coverage against
`declared_element_ids` to show exactly which elements the scanner is
finding vs missing.*
