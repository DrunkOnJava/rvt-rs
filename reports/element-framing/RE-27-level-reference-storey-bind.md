# RE-27 — the element record names the Level that hosts it

**Issue:** #219 (`Refs` #86, #212, #213, #218)
**Date:** 2026-09-19
**Corpus:** `2024_Core_Interior.rvt`, sha256
`c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014`,
Revit 2024, MIT, magnetar-io/revit-test-datasets.
**Probe:** `examples/probe_re27_level_reference.rs`.
**Result:** storey containment 801 → **853** of 872 emitted building
elements, with **0** of the 537 elements both joins answer disagreeing.
**Reference side: NOT MEASURED.** The paired Revit IFC exports are not
in this checkout, so nothing below is scored against Revit's own
`IfcRelContainedInSpatialStructure`; every number is measured on the
`.rvt` and on rvt-rs's own emitted STEP.

-----

## 1. What #219 actually found, once it was measured

The issue records 82 unbound elements: 64 plan-loop `IFCSLAB` and 18
name-only `IFCSPACE`. That set no longer exists. #212 replaced the
plan-loop annotations with 100 record-backed plates and #218 replaced
the eleven column-derived elevations with the fifteen real Revit
Levels, so the state on `main` at the time of this work is:

| unbound | count | why the elevation join cannot reach it |
|---|---:|---|
| `IFCSLAB` | 36 | top face 0.1667 ft (2 in) below a storey elevation |
| `IFCSHADINGDEVICE` | 10 | same |
| `IFCWINDOW` | 6 | record base is a sill height, 4.73 ft above its level |
| `IFCWALL` | 1 | base 56.4167 ft, mid-storey |
| `IFCSPACE` | 18 | recovered from a partition string; no element record at all |
| **total** | **71** | |

All 71 were nevertheless written into `IfcBuildingStorey` **`Basement
2`** — the writer used `storey_index.unwrap_or(0)`, so the honest
"unbound" state was emitted as a containment claim on a named storey
(§5).

The 46 plates are the group RE-22 §"STOREY_BIND_TOP_FACE_TYPES" already
described as sitting "2 in below their level (the structural-slab /
architectural-topping interface)". Binding them by rounding a top face
up to the next storey would work on this file and is still a guess. The
file does better than that.

## 2. The carrier: a Level ElementId in the counted reference list

RE-21 framed the partition element record; RE-23 read the slot *before*
the record's own id in the counted list at `+0x88` to find a door's host
wall; RE-25 read the last slot of the *second* list to find a sketch
line's owner. The same first list carries one more fact: **a placed
instance names the Revit `Level` that hosts it, as a plain `u64`
ElementId slot.**

The fifteen Level ElementIds come from RE-24's `OST_Levels` records
(`20268`, `20272`–`20277`, `20302`–`20308`, `65128`). Measured over
every placed instance of six categories, scoring the *named* Level
against the elevation the #212 / #213 join uses (base face, or top face
for plates):

| category | instances | no Level named | exactly one | two or more | both joins answer | agree | disagree |
|---|---:|---:|---:|---:|---:|---:|---:|
| `OST_Floors` | 146 | 25 | 121 | 0 | 64 | 64 | **0** |
| `OST_BuildingPad` | 2 | 1 | 1 | 0 | 0 | — | — |
| `OST_Columns` | 463 | 119 | 0 | 344 | 0 | — | — |
| `OST_Walls` | 1487 | 849 | 206 | 432 | 206 | 206 | **0** |
| `OST_Doors` | 320 | 53 | 267 | 0 | 267 | 267 | **0** |
| `OST_Windows` | 7 | 1 | 6 | 0 | 0 | — | — |
| **total** | | | | | **537** | **537** | **0** |

**537 of 537, no disagreement.** Wherever the measured elevation join
already had an answer and the record also named exactly one Level, the
two are the same storey.

Where the elevation join has no answer, the named Level is exactly the
value it was missing. The offsets between the elevation the old join
keys on and the named Level's elevation are a short, flat list:

| category | offset | records |
|---|---:|---:|
| `OST_Floors` | `+0.0000` ft | 64 |
| `OST_Floors` | `−0.1667` ft | 57 |
| `OST_Walls` / `OST_Doors` | `+0.0000` ft | 473 |
| `OST_Windows` | `+4.7300` ft | 6 |
| `OST_BuildingPad` | `−1.5000` ft | 1 |

`−0.1667` ft is the 2 in topping interface; `+4.73` ft is the window
sill height. Both are the *reason* those elements never bound, and both
are now irrelevant, because the storey is named rather than inferred.

## 3. The rule: exactly one, or nothing

```text
level_id = the single recovered Level ElementId in the record's
           counted reference list at +0x88
```

A record that names **no** Level, or **two or more**, resolves to
nothing and keeps whatever the elevation join gives it.

This is not a tuning threshold. A Revit column or wall carries a *base
constraint* and a *top constraint*, and both are in the list: every one
of the 344 `OST_Columns` records that names a Level at all names two,
as do 432 of the 638 `OST_Walls` records. Nothing in the bytes says
which slot is the base — the hit positions are 4 and 5 for columns and
2 and 3 for walls, but a slot index is not a proof — so picking one
would be a guess dressed as a decode. The columns lose nothing by being
declined: all 256 already bind exactly on their base elevation.

The id set the uniqueness test runs against is deliberately the
**broader** one: `partition_level_records::scan_partition_level_ids`
returns every standalone `OST_Levels` record id without requiring the
name/elevation recovery to validate. A Level the full recovery would
drop still counts as a second named Level, so a record that would be
ambiguous stays ambiguous. The *binding* half then uses only the
validated fifteen — an id outside them binds nothing.

## 4. Measured effect on the emitted IFC

`target/ci/rvt-ifc 2024_Core_Interior.rvt --mode geometry`:

| type | level reference | base elevation | top elevation | unbound | total |
|---|---:|---:|---:|---:|---:|
| `IFCCOLUMN` | 0 | 256 | 0 | 0 | 256 |
| `IFCDOOR` | 132 | 0 | 0 | 0 | 132 |
| `IFCSHADINGDEVICE` | 20 | 0 | 0 | 0 | 20 |
| `IFCSLAB` | 80 | 0 | 0 | 0 | 80 |
| `IFCSPACE` | 0 | 0 | 0 | 18 | 18 |
| `IFCWALL` | 134 | 225 | 0 | 1 | 360 |
| `IFCWINDOW` | 6 | 0 | 0 | 0 | 6 |
| **total** | **372** | **481** | **0** | **19** | **872** |

Before, on the same file:

| total | 0 | 747 | 54 | **71** | 872 |
|---|---:|---:|---:|---:|---:|

| | before | after |
|---|---:|---:|
| elements in a specific `IfcBuildingStorey` | 801 | **853** |
| elements on a storey nothing measured | **71** | **0** |
| `IFCRELCONTAINEDINSPATIALSTRUCTURE` | 13 | 14 |
| `IFCPROPERTYSINGLEVALUE` | 8435 | 9231 |
| `IFCPROPERTYSET` / `IFCBUILDINGSTOREY` / `IFCRELFILLSELEMENT` / `IFCMATERIAL` / every `by_ifc_type` | — | unchanged |

Every element bound in both exports keeps the same storey: 801 of 801,
0 moved, 0 lost.

## 5. The unbound container is the building, not storey zero

The writer clamped a missing storey index to `storey_index.unwrap_or(0)`
and wrote the element into the first storey. On this file that put 71
elements into `Basement 2` at −40 ft — including plates at 185 ft — and
a reader had no way to tell them from the 5 that genuinely belong there.

`IfcRelContainedInSpatialStructure.RelatingStructure` is an
`IfcSpatialElement`, so the honest target already exists: elements with
no recovered storey are now contained in the **`IfcBuilding`**. That is
"somewhere in this building, storey unknown", which is what rvt-rs
actually knows. `Basement 2` drops 76 → 6 contained elements.

## 6. What is not claimed

- **The 18 `IFCSPACE` rows stay unbound.** They are recovered from
  partition *strings* (`partition_name_candidates`), not from element
  records, so there is no reference list to read and no bbox to
  measure. They carry no elevation evidence of any kind and are not
  placed by proximity to anything.
- **`Wall 55840` stays unbound.** Its record names no single Level and
  its base (56.4167 ft) is mid-storey.
- **This is not the #86 / RE-20 typed level bind.** `level_bind.rs`
  binds a *typed* `Floor` / `Room` through an `m_level_id` schema
  field; that field is still not recovered and
  `level-elementid-storey-bind` stays `unsupported`. What is recovered
  here is a Level ElementId carried by the **partition element
  record**, for the record-backed categories only.
- **Scope is Revit 2024 partition element records.** On a release or a
  file where `partition_element_records` or `partition_level_records`
  decline, the level-id set is empty, the join resolves nothing, and
  every element keeps the elevation join it already had. Verified on
  the 2023 Einhoven sample: unchanged.
- **The reference export was not consulted.** §1's "why the elevation
  join cannot reach it" column and every count here are measured on the
  `.rvt` and on rvt-rs's own output. Scoring the 853 against Revit's
  own containment needs `IFC Exports/2024_Core_Interior_slim.ifc`,
  which is not in this checkout.

## 7. What ships

- `element_record_level_refs::unique_level_reference` — the rule, plus
  `LEVEL_REFERENCE_FIELD` (`m_levelId`) and
  `LEVEL_ELEMENT_ID_PROPERTY` (`LevelElementId`).
- `partition_level_records::scan_partition_level_ids` — the broad
  Level id set, one sweep, no name/elevation blocks.
- `partition_schema_mvp`: `element_record_decoded` attaches `m_levelId`
  and flips `m_level_bound`; the level id set is recovered once in
  `recover_partition_schema_mvp` and passed to every category.
- `ifc::export_content`: `LevelBindResolved` is now true when a Level
  was named, with `LevelElementId` and `LevelBindSource` beside it.
- `ifc::apply_record_level_reference_storeys`, running before the #213
  elevation join, which now leaves an already-bound element alone.
- `ifc::step_writer`: storey-less elements are contained in the
  `IfcBuilding`.

## 8. Reproduction

```bash
cargo build --profile ci --example probe_re27_level_reference
./target/ci/examples/probe_re27_level_reference \
  "$RVT_PROJECT_CORPUS_DIR/2024_Core_Interior.rvt"
```

Corpus gate:
`tests/iter_elements_typed.rs::core_interior_2024_storey_containment_is_evidence_backed`.
Unit gates: `src/element_record_level_refs.rs::tests` and the three
`src/ifc/mod.rs::tests` named `*_level_reference_*`.
