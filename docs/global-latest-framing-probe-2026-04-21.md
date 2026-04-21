# Global/Latest instance framing — probe notes 2026-04-21

Follow-on to the ElemTable record-layout RE earlier today. With
`elem_table::parse_records` now returning 26,425 clean sequential
ElementIds on the 34 MB project file, the next question is whether
Global/Latest uses a similar self-describing per-record framing that
would let us enumerate elements with a simple scan.

**Short answer:** No clean framing. Class tags appear densely inside
the stream (6.5 tag-hits / KB on project 2023), but they're dominated
by cross-references *within* records, not record-start markers.
Building a record-enumerator needs schema-directed reads, not naïve
tag scanning.

## What I probed

`examples/probe_latest_framing.rs` — for each file, find the
ADocument entry via the walker, then scan the 256 bytes before and
2 KB after for u16 values that match known class tags.

## What I saw

### Project 2023 (`Revit_IFC5_Einhoven.rvt`)

- ADocument at `0x1ee5`
- Bytes immediately before are UTF-16LE text (a sheet / schedule name
  — `"Rooms : Schema 1"`). Sequential strings are packed inline in
  Global/Latest, not in a separate string table.
- 256 bytes before ADocument contain 13 known-class-tag u16 hits,
  scattered at non-uniform offsets. Most are `0x0061` (AbsCurveGStep)
  and `0x0046` (ATFProvenanceBaseCell) — the schema's most
  frequently-referenced classes.
- 2 KB after ADocument+32: 13 hits / 2 KB = 6.5 tags/KB density.

### Project 2024 (`2024_Core_Interior.rvt`)

- ADocument at `0x157e0`
- Bytes immediately before: repeating `55 55 75 3f` pattern (looks
  like a float / coordinate table padding), then `ff ff ff ff ff 01`
  transition. This matches the ADocument starting with `01 00 00 00`
  more cleanly than on 2023.
- 256 bytes before ADocument contain 8 class-tag hits — mostly
  `0x0100` (AnalyticalLevelAssociationCell). Possibly the final N
  records in the preceding block.
- 2 KB after ADocument+32: zero hits. Notable — the ADocument region
  looks isolated from the surrounding element mass on the 2024
  project.

## What this means for the walker → IFC path

**Naïve approach — scan for class tags, treat each as a record
start — doesn't work.** The signal is too noisy:

- `tag 0x0061` (AbsCurveGStep) appeared 20,035 times in ~1 MB of
  Global/Latest on the 2024 project during an earlier session's
  probe. That's far more than the 26,425 total records declared in
  ElemTable. Most of those 20K occurrences are field-value pointers
  into the class, not record-start bytes.

**Schema-directed scan is the right approach.** Each class's
instance has a declared field layout. A scanner that:

1. Starts at a known offset (e.g., the ADocument region)
2. Trial-walks each schema-declared class at each byte-aligned offset
3. Uses `walk_score` to rank candidates
4. Picks high-confidence matches as record starts

... can in principle enumerate every element. The `walker` module
already has `trial_walk` + `walk_score` infrastructure for ADocument
detection; generalising it to all classes is the work still
remaining for L5B-11.

**Cost concern.** 80 registered decoders × 1 MB stream × per-offset
trial_walk ≈ lots of work. Probably want to:
- Pre-filter candidate offsets by "u16 matches a known class tag"
- Then trial_walk only those candidates
- Aggregate results across multiple candidate start offsets

## Test-corpus-dependent next step

This probe establishes that the naïve approach won't work, but does
not itself land a scanner. The full closure of L5B-11 needs a
schema-directed scanner + element-count validation against
`declared_element_ids()`. That's the remaining work.

Reproduce: `cargo run --release --example probe_latest_framing` with
`RVT_PROJECT_CORPUS_DIR` set to the magnetar corpus path.
