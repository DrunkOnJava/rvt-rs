# RE-24 — the Levels are elements, and their names and elevations are in a block keyed by their own ElementId

**Issue:** #218 (`Refs` #219, #33)
**Corpus:** `2024_Core_Interior.rvt`, sha256
`c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014`,
Revit 2024, eight `Partitions/*` streams, ~187 MiB inflated.
**Reference (scoring side only, never a decoder input):**
`Revit/2024_Core_Interior.ifc` (sha256 `d07c7462…`, 20 KB) and
`IFC Exports/2024_Core_Interior_slim.ifc` (sha256 `bfdf36ff…`, 19879
entities). Both carry the same 15 `IfcBuildingStorey`.
**Probe:** `examples/probe_level_records.rs`.
**Result:** 15 of 15 `(name, elevation)` pairs, exact.

-----

## 1. The question, and what #213 could not answer

#213 recovered storey *elevations* from the bounding boxes of the
element records the #211 instance rule selects: the 256 `OST_Columns`
records stand on exactly eleven distinct base `z` values —
0, 31, 46, 61, 76, 91, 106, 121, 136, 151, 166 ft — and every one of the
eleven is an `IfcBuildingStorey.Elevation` in Revit's own export. No
false positives, and no way to get the other four. The export's
−40, −20, 15 and 185.5 ft carry no column record at all, so no
distribution over column bases can ever contain them.

The names were worse. `partition_name_candidates` recovers twelve
Level-like strings from the partition string records, with no elevation
attached to any of them. Twelve names against eleven elevations is not a
pairing, and a rank join is actively wrong: it puts `Level 6` at 91 ft
where the export puts `Level 7`. So #213 shipped `Elevation 91.000 ft`
and recorded the names as unplaced — honest, and useless to a reader.

The question RE-24 asks is the obvious one #213 did not: **is the
`Level` itself in the file as an element?** A Revit `Level` has a
`BuiltInCategory` (`OST_Levels`, −2000240), so if it is an element it is
framed like one.

## 2. The record: an element prologue with no bounding box

Scanning all eight partition streams for the 8-byte little-endian
encoding of −2000240 finds **190** hits. Back-referencing each by
`CATEGORY_OFFSET` (`+0x12`) and validating through
`partition_element_records::decode_at` accepts **zero** of them.

The prologue is not the problem. On the first hit
(`Partitions/46 +228578`) every field the element-record reader checks
before the bounding box is exactly where #211 says it is:

```text
+0x00  87 3c 01 00 00 00 00 00   ElementId 81031, declared in Global/ElemTable
+0x08  8f 00 00 00               flags 0x8f
+0x0c  9f 05 00 00               0x0000059f, as on every element record
+0x10  00 00                     0
+0x12  90 7a e1 ff ff ff ff ff   BuiltInCategory −2000240 = OST_Levels
+0x1a  ff × 24                   sentinel padding
+0x32  85 3c 01 00 00 00 00 00   container ElementId 81029
+0x3a  ff × 8                    sentinel
+0x42  7f ef ff ff               placement kind, PLACED
+0x46  0a 00 00 00               unattributed
+0x4a  76 09                     unattributed
+0x4c  00 00 00 00               0   (0xffffffff on a bbox-bearing record)
+0x50  ff ff ff ff ab 05         record marker
+0x56  03 00                     record kind
```

The divergence is at `+0x4c`. A record with a bounding box carries
`ff ff ff ff` there, then `46 01` at `+0x50`, then `ff ff ff ff ab 05`
at `+0x52`, then 48 bytes of bbox doubles at `+0x58`. A `Level` record
carries `00 00 00 00` at `+0x4c` and the same six marker bytes two bytes
earlier, at `+0x50`, followed by a `u16` and then the next record. It
ends at `+0x97`; the records are 151 bytes apart in the stream.

That is exactly what one would expect of a datum plane. A level has no
solid, so it has no bounding box, so the reader that requires a bbox
marker at a fixed `+0x50` rejects it. `partition_level_records` is that
reader with the bbox requirement replaced by the six-byte marker at
`+0x50` — nothing else about the prologue changes.

With that reader, **75** records carry `OST_Levels` on this file. Which
of them is a *Level element* is not a new question: it is the #211
instance test, unchanged.

| selection | n |
|---|---:|
| `OST_Levels` records total | 75 |
| container member (`+0x32` set) | 59 |
| type / symbol envelope (`+0x42` = `0xffff8000`) | 1 (ElementId 1673) |
| **standalone placed instance** | **15** |

The 59 members belong to ten containers — 16229 (14), 81029 (11),
108205 (11), 26863 (7), 33696 (4), 21984 (3), 23117 (3), 21920 (2),
26908 (2), 87754 (2). Fifteen is the number of `IfcBuildingStorey` in
both reference exports. The fifteen ids are 20268, 20272–20277,
20302–20308 and 65128.

## 3. The name and elevation: a parameter block keyed by the owner

The record itself carries no double at all — 176 bytes from the record
start hold no value in any plausible elevation range. So the name and
elevation live elsewhere, and the framing that found them is the one
RE-22 already established for the per-instance `IFC Export As` override:
**an owning `ElementId` at a fixed negative offset from the value.**

Searching the streams for the export's own storey names as UTF-16LE
locates them immediately — `Level 3 - Wall Layouts 1` at
`Partitions/46 +8038256` and nine more names on a stride of
`746 + 2·len(name)` bytes. Reading backwards from a name gives a
constant framing:

```text
V-0x47  u64  owning ElementId
V-0x3f  56B  0xff sentinel run
V-0x07  3B   0x00
V-0x04  u32  name length, UTF-16 code units
V       2n   the name, UTF-16LE
```

and forwards, past a variable-length run, a marker at a constant
distance from the elevation:

```text
M       8B   05 00 00 00 48 02 00 00
M+55    f64  elevation, feet
M+208   f64  the same elevation again
```

Two details matter and both were found the hard way.

**The gap between the name and the marker is not fixed.** It is 347
bytes for fourteen of the fifteen levels and **363** for `Basement 2`,
whose block carries one extra parameter entry. A fixed
`name_end + 402` offset — the first thing that worked — silently read
`0.0` for `Basement 2`, which is a plausible elevation and would have
shipped a wrong storey. The marker is therefore searched forward from
the end of the name within `ELEVATION_MARKER_SEARCH_BYTES` (2048), and
the second copy at `+153` must agree bit-for-bit with the first.

**The owner slot at `V-0x47` is the discriminator, not the name.** The
same name string appears in several blocks — `Level 3 - Wall Layouts 1`
appears nine times in `Partitions/46` alone — and three of those pairs
are *wrong*: at `+21322916` the block named `Level 3 - Wall Layouts 1`
carries 46.0 ft, and at `+21323710` `Level 4 - Wall Layouts 2` carries
61.0. Every one of the wrong pairs is owned by an `OST_Levels` record
that is a **container member** (31636, 31637, 33628, 33629, 21990, …) —
that is, by exactly the records the #211 instance test already excludes.
Filtering the accepted blocks to the fifteen standalone Level ids
removes all of them and leaves fifteen blocks, one per level, in the
whole file.

Nothing here is a search over values. The scan walks maximal runs of
`0xff`, tests each run's start against the framing above, requires the
owner to be declared in `Global/ElemTable`, and requires the two
elevation copies to agree. A run of exactly 56 is required, which the
owner's own encoding guarantees can only start where the framing says:
an `ElementId` is a `u32`, so its high four bytes are zero and the run
cannot begin earlier.

## 4. Agreement with Revit, end to end

Fifteen accepted blocks, fifteen Level records, one block each, no
level owning two, no block owned by a non-Level. Scored against the
`IfcBuildingStorey` set of the reference export:

| ElementId | recovered name | recovered elevation (ft) | export `Name` | export `Elevation` | |
|---:|---|---:|---|---:|:-:|
| 20273 | `Basement 2` | −40 | `Basement 2` | −40. | ✓ |
| 20272 | `Basement 1` | −20 | `Basement 1` | −20. | ✓ |
| 20268 | `Level 1` | 0 | `Level 1` | 0. | ✓ |
| 20275 | `Mez 1-2` | 15 | `Mez 1-2` | 15. | ✓ |
| 20274 | `Level 3 - Wall Layouts 1` | 31 | `Level 3 - Wall Layouts 1` | 31. | ✓ |
| 20276 | `Level 4 - Wall Layouts 2` | 46 | `Level 4 - Wall Layouts 2` | 46. | ✓ |
| 20277 | `Level 4 - Wall Layouts 3` | 61 | `Level 4 - Wall Layouts 3` | 61. | ✓ |
| 20308 | `Level 6` | 76 | `Level 6` | 76. | ✓ |
| 20307 | `Level 7` | 91 | `Level 7` | 91. | ✓ |
| 20306 | `Level 8` | 106 | `Level 8` | 106. | ✓ |
| 20305 | `Level 9` | 121 | `Level 9` | 121. | ✓ |
| 20304 | `Level 10` | 136 | `Level 10` | 136. | ✓ |
| 20303 | `Level 11` | 151 | `Level 11` | 151. | ✓ |
| 20302 | `Level 12` | 166 | `Level 12` | 166. | ✓ |
| 65128 | `Level 13` | 185.5 | `Level 13` | 185.5 | ✓ |

**15 / 15**, names and elevations, no wrong pair, no missing storey, no
extra storey. Note that the recovered elevation order and the ElementId
order do not agree — 65128 is the top level, 20273 is the bottom, and
20302…20308 run *downwards* from `Level 12` to `Level 6`. A rank join on
ids would have been as wrong as the rank join on names #213 rejected.
The pairing survives because the file states it.

## 5. What the wider storey set buys

Containment logic is untouched: an exact elevation match, plates by
their record top face (Revit hangs a floor below the level that hosts
it), everything else by its base, fail-closed on a miss and on an
ambiguity. Only the storey set is wider.

| | 11 storeys (#213) | 15 storeys (RE-24) |
|---|---:|---:|
| `IFCCOLUMN` bound | 256 / 256 | 256 / 256 |
| `IFCDOOR` bound | 132 / 132 | 132 / 132 |
| `IFCWALL` bound | 355 / 360 | **359 / 360** |
| `IFCSLAB` bound | 41 / 80 | **44 / 80** |
| `IFCSHADINGDEVICE` bound | 10 / 20 | 10 / 20 |
| `IFCWINDOW` bound | 0 / 6 | 0 / 6 |
| `IFCSPACE` bound | 0 / 18 | 0 / 18 |
| **total** | **794 / 872** | **801 / 872** |

Four of the five previously unbound walls bind at −40 ft; the fifth
(56.4167 ft) is not a storey elevation in the export either, and stays
unbound. Three plates bind.

Two expectations from #219 did **not** hold, and the numbers say so
rather than the prose:

- **The 6 windows still bind to nothing.** This was never an
  elevation-set problem. A window record's base is its *sill height*
  (80.73, 95.73, … ft) — RE-21 measured 0 of 6 window base elevations
  landing on a storey, and adding four more storeys does not change
  that. Binding a window will require reading its host level, not a
  wider elevation set.
- **Most of the 49 unbound plates stay unbound** — 46 of them (36
  `IFCSLAB` and all 10 unbound `IFCSHADINGDEVICE`); only three bind. RE-22 measured why: those plates
  sit 0.1667 ft (2 in) below their level, at the structural-slab /
  architectural-topping interface. That is a thickness question, not an
  elevation-set one, and it is #219's real content.

## 6. Cross-witness surface

`levels` moves from `decoder_baseline` to `known` at tolerance 0 on both
manifests — `diagnostics.exported.storey_count` is 15 and both reference
exports carry 15 `IfcBuildingStorey`, so the count agrees exactly.

A count is a weak claim, so the pair set is gated too, as OctetProof
1.1.0's second additive field class (`storeys`, spec §20.2). One
complication is real and worth recording: **the two sides declare
different length units.** Revit's export of this imperial project
declares `FOOT` as an `IfcConversionBasedUnit` over `METRE` with ratio
0.3048 and writes `Elevation` as `-40.`; rvt-rs's writer declares
`METRE` and writes `-12.192000`. Both are faithful. Comparing the raw
numbers would report a 3.28× disagreement on every storey.

So each witness resolves its *own* file's `LENGTHUNIT` before emitting
and renders the elevation in feet at 1e-6 as a fixed six-decimal string:

| witness | reads | unit resolution |
|---|---|---|
| `rvt-rs` | its own emitted STEP | walks `IfcProject` → `IfcUnitAssignment` itself |
| IfcOpenShell 0.8.5 | Revit's `.ifc` | `ifcopenshell.util.unit.calculate_unit_scale` |
| IFClite 7.1.1 | Revit's `.ifc` | `ifc_lite_core::extract_length_unit_scale` |

The three payloads are byte-identical on both artifacts. The elevation
travels as a string, not a JSON number, for the same reason the
canonicalizer is restricted to integers and strings (§7.3): a Rust
witness and a Python witness must not be asked to print the same `f64`
identically. Unicode normalization is deliberately not applied to the
name — three runtimes with three Unicode tables would launder a real
difference into agreement, and the strict direction is the correct one
for a verification gate.

Claimed surface: `magnetar-2024-core-interior-slim` 11 → 13 fields,
`magnetar-2024-core-interior` 4 → 6. Both verdicts `PASS`.

## 7. What is claimed, and what is not

**Claimed.**

- On Revit 2024, an `OST_Levels` partition record with no container at
  `+0x32` and placement kind `0xffffef7f` at `+0x42` is a Revit `Level`
  element. Measured: 15 of 15 against the export's storey count.
- The block framed by an owning `ElementId` at `value-0x47`, a 56-byte
  `0xff` run, three zero bytes and a `u32` UTF-16 length carries that
  element's display name; an `f64` in feet sits 55 bytes past the
  8-byte marker `05 00 00 00 48 02 00 00` that follows it, repeated at
  `+153`. Measured: 15 of 15 `(name, elevation)` pairs exact.

**Not claimed.**

- Anything about another release. The framing is proven on one Revit
  2024 file; `PARTITION_LEVEL_SUPPORTED_REVIT_VERSIONS` is `[2024]` and
  every other release gets an empty vector.
- Anything about the two words inside the elevation marker. `5` and
  `584` are recorded, not decoded; `584` is a declared `ElementId` on
  this file, which is suggestive of a parameter definition and is not
  asserted.
- Anything about `+0x46`, `+0x4a`, `+0x4c` or the `u16` at `+0x56` in
  the record. Recorded, not interpreted, exactly as `+0x46` and `+0x4a`
  are on a bbox-bearing record.
- **A Level ElementId map for Floors and Rooms.** RE-20 / #86 remains a
  research wall and is not reopened here. Recovering a *Level's own* id
  from its own record is not the same thing as recovering a Floor's or a
  Room's reference to a Level, which is what
  `level-elementid-storey-bind` needs and still does not have. The
  reference-list slots at `+0x88` that RE-23 left unattributed are the
  obvious next place to look (#228), and are not looked at here.
- Anything about a partial recovery. If any Level record does not own
  exactly one accepted block, or two levels share an elevation, the
  whole recovery is discarded and the #213 column-derived path runs
  instead. A fourteen-storey building is not a defensible reading of a
  fifteen-storey file.

## 8. Reproduction

```bash
cargo build --profile ci --example probe_level_records
./target/ci/examples/probe_level_records \
  "$RVT_PROJECT_CORPUS_DIR/2024_Core_Interior.rvt" > /tmp/re24.json

# 75 records, 15 level elements, 15 name blocks, 15 recovered levels
python3 - <<'PY'
import json
d = json.load(open('/tmp/re24.json'))
print(d['level_category_records'], d['level_elements'],
      len(d['name_blocks']), len(d['levels']))
for level in d['levels']:
    print(level['element_id'], level['name'], level['elevation_feet'])
PY

# the reference side
grep IFCBUILDINGSTOREY "$RVT_PROJECT_CORPUS_DIR/../IFC Exports/2024_Core_Interior_slim.ifc"
```
