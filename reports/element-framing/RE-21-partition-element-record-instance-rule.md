# RE-21 — the partition element-record instance rule

Status: **positive result**, exact on four categories.
Closes #211, explains #216. Date: 2026-08-30.
Artifact: `2024_Core_Interior.rvt`
(sha256 `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014`,
Revit 2024, magnetar-io/revit-test-datasets, MIT).
Reference: `IFC Exports/2024_Core_Interior_slim.ifc`
(sha256 `bfdf36ffb0bb768f3409d818403990e64d4c262c6780603be87f8077387ad86d`),
Revit's own full-project export, whose per-entity `Tag` attribute carries the
source ElementId.

This is **not** a re-opening of RE-19. RE-19's negatives stand untouched: the
2024 `ArcWallRectOpening` index bytes still carry no Door/Window
discriminator, and there is still no schema-field `Wall` envelope suitable for
fail-closed decode. The carrier here is a different one — the partition
element record's own `BuiltInCategory` field, which names the category
outright (#210 / `src/partition_element_records.rs`).

## 1. The question

#210 established that the Revit 2024 partition element-record header reaches
**100 % of the exported ElementIds** in every architectural category. What it
did not have was precision: the #204 instance filter (drop origin-centred
family-local bboxes, keep the highest ElementId per project-coordinate
footprint origin) is exact for columns and over-counts everywhere else.

| category | distinct ids carrying the category | exported | #204 filter selects | FP | FN |
|---|---:|---:|---:|---:|---:|
| `OST_Walls` (-2000011) | 1210 | 360 | 648 | 288 | 0 |
| `OST_Doors` (-2000023) | 200 | 132 | 139 | 7 | 0 |
| `OST_Windows` (-2000014) | 9 | 6 | 8 | 2 | 0 |
| `OST_Columns` (-2000100) | 392 | 256 | 256 | 0 | 0 |

## 2. Method

`examples/probe_element_record_instance_rule.rs` dumps, for every decodable
category record, the full 88-byte prologue, 256 bytes of record body, and the
`Global/ElemTable` row for the same ElementId. The exported ElementId sets
come from the reference IFC's `Tag` attribute. The record set is then split
into *exported* and *not exported* and the prologue fields are histogrammed on
each side.

## 3. The discriminating evidence

Two prologue fields separate the two sides cleanly. Both were inside the 54
bytes that #210 recorded as "0xff sentinel padding + three unattributed
words".

Histogram of prologue bytes `+0x1a..+0x50` (all five categories, deduplicated):

| side | distinct patterns | shape |
|---|---:|---|
| exported | 1 per category | `ff`×40 then `7fefffff` `<w46>` `<w4a>` `ffffffff` |
| not exported, container members | 9 owner values | `ff`×24 then **`<owner u64>`** then `ff`×8 then `7fefffff` … |
| not exported, type symbols | 2 per category | `ff`×40 then **`0080ffff`** `<w46>` `<w4a>` `ffffffff` |

So:

```text
+0x32  u64  container ElementId, 0xffff_ffff_ffff_ffff = none
+0x42  u32  placement kind: 0xffffef7f placed instance
                            0xffff8000 family/type symbol envelope
```

**The rule.** A record is a standalone placed instance — the thing Revit's own
exporter emits as a building element — iff

```text
u64 @ +0x32 == 0xFFFF_FFFF_FFFF_FFFF   AND   u32 @ +0x42 == 0xFFFF_EF7F
```

Neither half is sufficient alone; both together are exact:

| category | rule A (`+0x32` unset) | rule B (`+0x42` placed) | **A ∧ B** | exported |
|---|---:|---:|---:|---:|
| `OST_Walls` | 361 (1 FP) | 1209 (849 FP) | **360** exact | 360 |
| `OST_Doors` | 147 (15 FP) | 185 (53 FP) | **132** exact | 132 |
| `OST_Windows` | 8 (2 FP) | 7 (1 FP) | **6** exact | 6 |
| `OST_Columns` | 273 (17 FP) | 375 (119 FP) | **256** exact | 256 |

"Exact" means the selected ElementId **set** equals the export's `Tag` set —
not merely the count. Verified again end to end on the emitted IFC: 360/360
`IFCWALL`, 132/132 `IFCDOOR`, 6/6 `IFCWINDOW`, 256/256 `IFCCOLUMN`, zero
missing, zero extra.

## 4. What `+0x32` is

Every non-sentinel value observed is:

- an ElementId **declared in `Global/ElemTable`** (all nine, checked);
- **lower than every member id that names it**;
- the owner of a **contiguous ElementId block that spans several categories at
  once**;
- **without an element record of its own** — a brute scan of all 23 470
  decodable records on this file finds none of the nine.

| owner | columns | walls | doors | windows | member id range |
|---:|---:|---:|---:|---:|---|
| 16229 | 23 | 66 | 14 | 1 | 16347 – 16995 |
| 21984 | 24 | 8 | – | – | 22008 – 22077 |
| 23117 | 24 | 57 | 11 | – | 23134 – 23733 |
| 26863 | 24 | 63 | 14 | – | 26880 – 28001 |
| 26908 | – | 27 | – | – | 26993 – 27045 |
| 33696 | 24 | 70 | 14 | – | 33748 – 34467 |
| 81029 | – | 277 | – | – | 81041 – 87363 |
| 87754 | – | 4 | – | – | 87758 – 87764 |
| 108205 | – | 277 | – | – | 108216 – 114538 |

A geometry-less container element that owns a mixed-category, contiguous
ElementId block allocated immediately after it, whose members Revit's exporter
skips, is consistent with a Revit **group / assembly type definition** — the
first of #216's four candidate explanations. That naming is *not* claimed;
what is claimed is the byte-level behaviour above, which is what the rule
tests.

## 5. What `+0x42` is

It takes exactly two values across all 2458 category records on this file:
`0xffffef7f` (2423) and `0xffff8000` (35). The `0xffff8000` set is a **strict
superset** of the origin-centred `is_family_local` bbox proxy on every
category:

| category | `is_family_local` | `+0x42 == 0xffff8000` | proxy misses |
|---|---:|---:|---|
| `OST_Columns` | 17 | 17 | — |
| `OST_Walls` | 0 | 1 | 130286 |
| `OST_Doors` | 0 | 15 | 17331, 17333, 19269–19274, 23756, 24300, 24301, 132750, 132824, 132825, 132875 |
| `OST_Windows` | 0 | 2 | 17335, 19275 |
| `OST_Floors` | 0 | 0 | — |

The misses are symbols whose envelope is centred on **one** axis only — e.g.
id 17331 spans `x −1.749…1.749` but `y −0.332…3.251`. The bbox proxy cannot
see them; the word does.

## 6. #216 falls out

The 136 `OST_Columns` ElementIds Revit's exporter omits split with no
remainder:

- **17 type symbols** (`+0x42 == 0xffff8000`) — ids 5755, 19250, 22374, 24299,
  28008, 53894, 75407–75417, exactly the 17 #216 lists as family-local;
- **119 container members** (`+0x32` set) — the five 23–24-element blocks
  16347–16369, 22045–22068, 23134–23157, 26880–26903, 33748–33771 that #216
  identified as "exact co-locations of an exported column", each owned by the
  id allocated just before its block.

The #204 highest-ElementId-per-footprint heuristic is therefore **retired**:
`partition_schema_mvp` now applies the direct test. It reproduces the same
256/256 columns and additionally the exact wall, door and window id sets. The
`is_family_local` helper is kept as a documented, strictly weaker diagnostic.

## 7. Where the rule does **not** hold

`OST_Floors` (#212) stays a `known_gap`, measured not hand-waved:

| stage | count |
|---|---:|
| distinct ids carrying `OST_Floors` | 124 |
| exported `IFCSLAB` | 80 |
| of those, present as a record | 79 (one exported slab has no `OST_Floors` record at all) |
| rule A ∧ B selects | 99 (TP 79 / FP 20 / FN 1) |

`+0x42` is `0xffffef7f` on **all** 146 floor records — there are no slab type
symbols on this file — so the rule degenerates to `+0x32` alone there. The
remaining 20 false positives are the Arch Topping / Structural Slab identity
problem #212 describes, which needs a slab-appropriate identity key, not a
looser tolerance. Floors are not moved.

## 8. Reproduction

```bash
cargo build --profile ci --example probe_element_record_instance_rule
./target/ci/examples/probe_element_record_instance_rule \
  "$RVT_PROJECT_CORPUS_DIR/2024_Core_Interior.rvt" > records.json

cargo build --profile ci --example probe_element_record_owner_lookup
./target/ci/examples/probe_element_record_owner_lookup \
  "$RVT_PROJECT_CORPUS_DIR/2024_Core_Interior.rvt" \
  16229,21984,23117,26863,26908,33696,81029,87754,108205
```

The shipped path is
`rvt::partition_element_records::PartitionElementRecord::is_exported_instance`
plus `rvt::partition_schema_mvp::instances_from_records`; the corpus gate is
`tests/iter_elements_typed.rs::core_interior_2024_rect_openings_not_fake_doors`
and `tests/project_count_fixtures.rs`.
