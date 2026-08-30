# RE-23 — the door/window host wall is the slot before the record's own id

Status: **positive result**, exact on all 138 host bindings.
Closes #222. Date: 2026-08-30.
Artifact: `2024_Core_Interior.rvt`
(sha256 `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014`,
Revit 2024, magnetar-io/revit-test-datasets, MIT).
Reference: `IFC Exports/2024_Core_Interior_slim.ifc`
(sha256 `bfdf36ffb0bb768f3409d818403990e64d4c262c6780603be87f8077387ad86d`),
Revit's own full-project export, whose per-entity `Tag` attribute carries the
source ElementId.

RE-21 recovered the exact door and window **instance** sets (132/132, 6/6) and
recorded that the record body past the bounding box was dumped but not mined.
That body is where the host lives.

## 1. The question, and what the reference answers with

Revit's exporter does not attach a door to a wall directly. It cuts an
`IfcOpeningElement` out of the wall and fills it:

```text
#19916=IFCOPENINGELEMENT(…,'I_Single-Flush:3 x 8:20827:1',…,'20827',.OPENING.);
#19917=IFCRELVOIDSELEMENT(…,#2889,#19916);     wall 20796 ← opening
#19918=IFCRELFILLSELEMENT(…,#19916,#2572);     opening → door 20827
```

Composing the two gives the pair set the decode has to reproduce. Read with
IfcOpenShell 0.8.5:

| in the export | count |
|---|---:|
| `IFCOPENINGELEMENT` | 201 |
| `IFCRELVOIDSELEMENT` | 201 |
| `IFCRELFILLSELEMENT` | **138** |
| filled by an `IfcDoor` | 132 |
| filled by an `IfcWindow` | 6 |
| hosts of a filled opening | `IfcWall`, 138 of 138 |
| distinct host walls | 92 (65 hold one opening, 8 hold two, 19 hold three) |
| unfilled openings | 63 — 42 in an `IfcSlab`, 20 in an `IfcShadingDevice`, 1 in an `IfcWall` |

The 63 unfilled openings are plain penetrations, not door/window hosting, and
are out of scope for #222. The claim is the **138 filled pairs**.

## 2. The carrier: a counted reference list at `+0x88`

The 88-byte prologue and the 48-byte bounding box end at `+0x88`. What follows
is a `u32` length and that many `u64` slots. Door ElementId 25947,
`Partitions/46`:

```text
+0x88  06 00 00 00                    n = 6
+0x8c  03 00 00 00 00 00 00 00        3
+0x94  32 4f 00 00 00 00 00 00        20274
+0x9c  cc 5c 00 00 00 00 00 00        23756   door type symbol (RE-21 §5)
+0xa4  ed 5e 00 00 00 00 00 00        24301   door type symbol (RE-21 §5)
+0xac  90 63 00 00 00 00 00 00        25488   ← host wall
+0xb4  5b 65 00 00 00 00 00 00        25947   ← the record's own id
```

Window ElementId 20898 carries the same framing with `n = 18`:

```text
+0x88  12 00 00 00                    n = 18
+0x8c  3, 17229, 17230, 17259, 17261…17269, 17335, 19275, 20308,
+0x10c 20897   ← host wall
+0x114 20898   ← the record's own id
```

**The rule.** The host is the slot **immediately before the record's own
ElementId** in that list. Shipped as
`PartitionElementRecord::preceding_reference`, and accepted only when the
value is one of the 360 ElementIds the RE-21 instance rule selects as exported
walls — so a wrong read fails closed instead of inventing a host.

## 3. Measurement

138 door/window instances are framed by 273 records on this file. Scoring
every record, not merely the one the per-id selection keeps:

| rule | correct | wrong | no value |
|---|---:|---:|---:|
| **slot before the record's own id** | **273** | **0** | 0 |
| `u64 @ +0xac` (list index 4) | 186 | 87 | 0 |
| `u64 @ +0xa4` (list index 3) | 81 | 192 | 0 |
| last two slots are `(host, self)` | 192 | 81 | 0 |

The three rejected candidates all assume a fixed shape the list does not have.
`+0xac` is right only when `n = 6` **and** the own id sits last; `+0xa4` only
when the own id sits second-to-last; "last two slots" fails on the 81 records
whose list continues past the own id with a trailing door type symbol — the
only four values that ever appear there are `132750`, `132824`, `132825`,
`132875`, all of them `+0x42 == 0xffff8000` symbol envelopes RE-21 already
excludes. The list length itself is not fixed either: 6 on all 267 door
records, 18 on all 6 window records, and up to 101 elsewhere.

Applying the same predecessor read to every category (instances only) shows
what the rule is and is not:

| category | records | predecessor is an exported wall | predecessor is something else |
|---|---:|---:|---:|
| `OST_Doors` | 267 | **267** | 0 |
| `OST_Windows` | 6 | **6** | 0 |
| `OST_Walls` | 638 | 528 | 110 |
| `OST_Columns` | 344 | 0 | 344 |
| `OST_Floors` | 121 | 0 | 121 |

So "the predecessor is a wall" is *not* a door/window discriminator — 528 wall
records have one too. It is a fail-closed check on a value the door and window
records are being asked for, and nothing else. The categories the rule is
applied to are the two RE-21 already identifies by `BuiltInCategory`.

## 4. Agreement with Revit, end to end

`rvt-ifc` emits the same chain — `IfcOpeningElement` (bodied with the
door/window's own record bounding box) + `IfcRelVoidsElement` +
`IfcRelFillsElement` — and the `(host Tag, filling Tag)` pairs are read back
out of the emitted IFC with IfcOpenShell:

| | rvt-rs | Revit export |
|---|---:|---:|
| `IFCRELFILLSELEMENT` pairs | 138 | 138 |
| distinct pairs | 138 | 138 |
| in the export, missing from rvt-rs | **0** | — |
| in rvt-rs, absent from the export | **0** | — |

Exact set equality, no tolerance. `IFCWALL` / `IFCDOOR` / `IFCWINDOW` /
`IFCCOLUMN` / `IFCSLAB` / `IFCSHADINGDEVICE` stay 360 / 132 / 6 / 256 / 80 /
20, and `tools/ci/ifc_schema_arity.py` passes on the fresh export: 18 192
instances across 40 entity types, with `IfcOpeningElement` at its declared
arity of 9 (ending in `PredefinedType`), `IfcRelVoidsElement` and
`IfcRelFillsElement` at 6.

The opening's `Tag` repeats the filling element's, which is what Revit does —
all 138 opening tags in the reference equal the tag of the element that fills
them — and invents nothing, because the opening exists only because that
element does.

## 5. Two writer bugs the binding exposed

Neither was reachable before, because no real-file door had ever carried a
host.

1. **The host recovery sat in a dead branch.** `export_content` only
   consulted `recover_door_host` / `recover_window_host` in the `else` arm of
   the element-record geometry check, so a door with a record bounding box —
   which is every recovered door — skipped it. Moved out of the branch.
2. **Void/fill resolution depended on emission order.** The writer resolved
   `host_element_index` against `entity_index_to_el_id` *inside* the element
   loop, so a host emitted after its opening silently dropped the whole chain.
   The two relationships are now emitted after the loop, when the map is
   complete.

## 6. What is claimed, and what is not

- **Claimed:** on Revit 2024 partition element records, for `OST_Doors` and
  `OST_Windows`, the slot before the record's own ElementId in the `+0x88`
  reference list is the host wall, verified as an exact 138-pair set match
  against Revit's own export.
- **Not claimed:** what the other slots mean. The leading `3` and the
  family/type/level ids between it and the host are recorded, not decoded.
- **Not claimed:** anything about walls, columns or slabs. The predecessor is
  read for every category and *used* only for doors and windows.
- **Not claimed:** the 63 unfilled openings. rvt-rs emits no opening that is
  not filled by a recovered element, so slab and shading-device penetrations
  stay unrecovered.
- **Not claimed:** opening geometry fidelity. The opening's body is the
  door/window's record bounding box placed on the element's own placement, not
  a wall-relative cut derived from the wall's location curve.
- RE-19's negative is untouched: the `ArcWallRectOpening` index bytes still
  carry no Door/Window discriminator and no host claim. This is a different
  carrier, the same way RE-21's was.

## 7. Reproduction

```bash
cargo build --profile ci --example probe_door_window_host_binding
./target/ci/examples/probe_door_window_host_binding \
  "$RVT_PROJECT_CORPUS_DIR/2024_Core_Interior.rvt" > refs.json
```

The shipped path is
`rvt::partition_element_records::decode_reference_list` plus
`rvt::partition_schema_mvp::opening_instances_from_records`; the corpus gate is
`tests/iter_elements_typed.rs::core_interior_2024_door_window_host_wall_binding`,
which re-reads the reference export and compares the pair sets directly, and
the cross-witness gate is `relations.IFCRELFILLSELEMENT` inside the
`magnetar-2024-core-interior-slim` OctetProof surface.
