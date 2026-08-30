# RE-22 — slabs: the instance rule was already exact, the scoring was not

Status: **positive result**, exact on `IFCSLAB` and `IFCSHADINGDEVICE`.
Closes #212, closes the thickness half of #31, moves #219, answers #218
with a negative. Date: 2026-08-30.
Artifact: `2024_Core_Interior.rvt`
(sha256 `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014`,
Revit 2024, magnetar-io/revit-test-datasets, MIT).
Reference: `IFC Exports/2024_Core_Interior_slim.ifc`
(sha256 `bfdf36ffb0bb768f3409d818403990e64d4c262c6780603be87f8077387ad86d`),
Revit's own full-project export, whose per-entity `Tag` attribute carries the
source ElementId.

RE-21 recorded `OST_Floors` as the one category where the partition
element-record instance rule fails: 99 selected against 80 exported,
`TP 79 / FP 20 / FN 1`. Both numbers reproduce exactly. Neither is a decoder
error.

## 1. The twenty "false positives" are exported

Taking the same set difference RE-21 used, the 20 selected-but-not-`IFCSLAB`
ElementIds are

```text
20953, 64160, 64227, 64292, 64358, 64423, 64488, 64553, 64618, 64683,
70366, 71171, 71231, 71291, 71351, 71411, 71471, 71531, 71591, 71651
```

and the export's 20 `IFCSHADINGDEVICE` `Tag` values are the **same twenty
ids, in the same order**. They are not false positives; they are elements
Revit's own exporter emits under a different IFC entity type:

```text
#3468=IFCSHADINGDEVICE(…,'Floor:Arch Topping Slab - 2":20953',$,
                       'Floor:Arch Topping Slab - 2"',…,'20953',.NOTDEFINED.);
```

So the instance rule
(`u64 @ +0x32 == 0xFFFF_FFFF_FFFF_FFFF` **and** `u32 @ +0x42 == 0xFFFF_EF7F`)
selects 99 ElementIds on `OST_Floors` and **all 99 are exported** — 79 as
`IfcSlab`, 20 as `IfcShadingDevice`. Precision was already 100 %. What #212
measured was a type-assignment gap wearing a precision gap's clothes.

## 2. The one "false negative" is a building pad

The single exported slab with no `OST_Floors` record is

```text
#3526=IFCSLAB(…,'Pad:Site Pad:21975',$,'Pad:Site Pad',…,'21975',.FLOOR.);
```

#212 guessed this ("it may be the `Pad:Site Pad`, which is a different Revit
class"). It is. A brute scan of all 23 470 decodable element records finds
ElementId 21975 exactly once, with

```text
category = -2001263   bbox = 20.0 … 167.0 x 25.0 … 114.0 x -43.0 … -41.5 ft
```

`-2001263` is Autodesk's published `BuiltInCategory.OST_BuildingPad`. The
record decodes with the same 88-byte prologue shape, passes the same instance
rule, and Revit's own exporter maps a building pad to `IfcSlab` with
`PredefinedType = .FLOOR.`. Adding the category recovers the 80th slab.

| stage | count |
|---|---:|
| distinct ids carrying `OST_Floors` | 124 |
| instance rule selects on `OST_Floors` | 99 |
| of those, exported by Revit | **99** (79 `IfcSlab` + 20 `IfcShadingDevice`) |
| instance rule selects on `OST_BuildingPad` | 1 |
| of those, exported by Revit | **1** (`IfcSlab`) |
| exported `IFCSLAB` recovered | **80 / 80** |
| exported `IFCSHADINGDEVICE` recovered | **20 / 20** |

## 3. The slab / shading-device split is per instance, and it is readable

The split is not a type property. The reference export carries

```text
#1603 =IFCSLABTYPE          (…,'Floor:Arch Topping Slab - 2"',…,'4166', $,.FLOOR.);
#3433 =IFCSLABTYPE          (…,'Floor:Arch Topping Slab - 2"',…,'4166', $,.ROOF.);
#3482 =IFCSHADINGDEVICETYPE (…,'Floor:Arch Topping Slab - 2"',…,'4166', $,.NOTDEFINED.);
#17950=IFCSLABTYPE          (…,'Floor:Structural Slab',…,'71848',$,.FLOOR.);
#18032=IFCSHADINGDEVICETYPE (…,'Floor:Structural Slab',…,'71848',$,.NOTDEFINED.);
```

— the **same Revit `FloorType` ElementId** (4166, 71848) produces both an
`IfcSlabType` and an `IfcShadingDeviceType`. Only a per-instance decision can
do that: Revit's `IFC Export As` instance parameter.

### 3.1 What was rejected

Two byte fields separate the two sides *within* `OST_Floors` on this file and
neither survives contact with the rest of the corpus:

| candidate | inside `OST_Floors` | why it was rejected |
|---|---|---|
| prologue flags `+0x08` | `0x0119` on all 30 shading records, on none of the 91 slab records | 32 **exported** `IFCWALL` records also carry `0x0119`. The word is a count: its observed values are `0xe9 + 8k` and it tracks the byte at `+0x88` with a per-category offset. The separation is a consequence of all 20 shading plates having one identical footprint, not a marker. |
| record byte `+0x88` | `8` on all 30 shading records, `{5,6,7,9}` on the slab records | 153 exported `IFCWALL` and 107 exported `IFCCOLUMN` records carry `8`. Same objection. |
| bbox shape | all 20 shading plates are exactly `182.3333 x 121.6111 ft`, larger than every slab footprint (`168x107`, `147x89`, `144x86`) | a shape-identity heuristic, not a test — and it is the same objection #212 raised against the #204 footprint key. |

### 3.2 What was found

The parameter value is in the file, as a UTF-16LE string. Searching every
inflated `Partitions/*` stream:

| needle (UTF-16LE) | `Partitions/46` | `Partitions/51` |
|---|---:|---:|
| `IfcExportAs` | 1 | 0 |
| `IfcExportType` | 1 | 0 |
| `IfcShadingDevice` | 42 | 20 |

62 hits = 31 pairs. Each entry is framed

```text
-0x11e  u64  owning ElementId (confirmation slot)
-0x0dc  u64  owning ElementId
-0x004  u32  value length in UTF-16 code units (0x10)
+0x000  32B  "IfcShadingDevice", UTF-16LE
+0x020  u64  parameter-definition ElementId (17368, then 17493)
```

Verified over the whole 3 KB window around every hit: the owning ElementId
appears **only** at those two offsets, never elsewhere, always as a `u64`,
always identical in both slots. Requiring the two slots to agree and the
value to be declared in `Global/ElemTable` accepts 31 entries — the second
string of each pair is rejected (its owner slots hold `0xFFFFFFFF00000000`
and unrelated bytes) and so is nothing else.

The 31 accepted entries name **21** distinct ElementIds: the 20
`IFCSHADINGDEVICE` `Tag` values, plus `16925`, which is a container member of
owner `16229` and therefore already excluded by the RE-21 instance rule — it
is the superseded twin of `20953` and carries the same `182.3333 x 121.6111`
box. Composing the two rules is exact with no remainder.

**The rule.** A partition element record is an exported slab iff it is a
standalone placed instance (RE-21) in `OST_Floors` or `OST_BuildingPad`
**and** no accepted `IFC Export As` override names it. An override naming
`IfcShadingDevice` redirects the entity type instead of suppressing the
element.

The shipped scan is general — it returns whatever `Ifc…` value string the
framing holds — but `src/ifc/category_map.rs::EXPORT_OVERRIDE_TARGETS` lists
only `IfcShadingDevice`, because that is the only value a reference export
has demonstrated. An unrecognised value leaves the element on its class
mapping; it never invents an entity type.

## 4. One record per ElementId: greatest vertical extent

22 of the 100 slab ids are framed more than once and the frames disagree on
`z`. RE-21's "first by `(stream, offset)`" picks `Partitions/46`, which for
the 12 `Floor:Floor 1` plates sees only the 2 in topping. Measured against
the reference export's `IfcExtrudedAreaSolid.Depth`:

| per-id record choice | slabs whose `z` extent equals an export depth |
|---|---:|
| first by `(stream, offset)` | 67 / 80 |
| **greatest `z` extent** | **79 / 80** |

The 80th is `22756` `Floor:Basement Slab`, which the export writes as two
stacked solids of `0.3333 ft` and `1.1667 ft` — summing to exactly the
recorded `1.5 ft`. The change is applied uniformly and perturbs nothing: 268
walls, 132 doors and 88 columns also carry more than one record on this file,
and every one of them agrees on the box, so the wall / door / window / column
id sets and bodies are byte-identical.

## 5. Thickness (#31)

The record's `z` extent **is** the slab's extrusion depth: 79 of 80 exact at
tolerance 1e-3 ft, the 80th equal to the sum of the export's two stacked
solids. Record-backed slabs therefore ship `ThicknessResolved = true`,
`ThicknessFeet`, and `ThicknessSource =
partition_element_record_bbox_z_extent`, and
`floor_slab_extrusion_thickness` no longer fires for them — it is now raised
per slab, so the plan-loop path (2023 files) still declares it.

The *profile* is still the bbox rectangle, not the recovered floor boundary
polygon: `ProfileResolved` stays `false`. #31's "polygon profile extrusion"
half is open; its thickness half is closed.

## 6. Storey containment (#219)

The 64 plan-loop `IFCSLAB` annotations carried no ElementId, no record bbox
and therefore no storey. They are retired on files where records decode —
emitting both would double-count the same plates — so the count is exactly
one `IFCSLAB` per exported id.

Record-backed plates bind by their **top** face, not their base. Revit hangs
a floor below the level that hosts it, so:

| join over the 100 record-backed plates | bound | agrees with the export | wrong |
|---|---:|---:|---:|
| base `z` equals a recovered storey elevation | 0 | 0 | 0 |
| **top `z` equals a recovered storey elevation** | **51** | **51** | **0** |

Exact match, no tolerance, no proximity fallback. The 49 that stay unbound
have tops at `x.8333 ft` — the structural-slab / architectural-topping
interface, 2 in below the level — or sit on one of the four storeys the
column-derived elevation set does not contain.

`diagnostics.exported.storey_bound_elements` moves **743 → 794** on Core
Interior. #219's 82 unbound splits differently now: 49 plates (was 64), 18
spaces, 5 walls, 6 windows = 78.

## 7. Storey elevations (#218): measured, and the answer is no

The #213 restriction was re-measured against the reference export's fifteen
`IfcBuildingStorey.Elevation` values over the recovered slab set:

| slab record face | distinct values | equal to a storey elevation | not a storey elevation |
|---|---:|---:|---:|
| base `z` | 40 | **0** | 40 |
| top `z` | 26 | 13 | 13 |

Slab bases are worthless as an elevation source — not one of the 40 is a
storey. Slab tops would add **−40 ft and 185.5 ft**, two of the four storeys
#213 misses, at the price of **13** elevations that are not storeys (each
`0.1667 ft` below a real one). Two for thirteen is not a trade this project
takes, so `ifc::STOREY_ELEVATION_SOURCE_TYPES` stays `["IFCCOLUMN"]` and the
recovered storey count stays 11. #218 keeps both halves open; the numbers
above are the measurement it asked for.

## 8. Measured before / after on `2024_Core_Interior.rvt`

| metric | before | after | reference export |
|---|---:|---:|---:|
| `IFCSLAB` | 64 (plan-loop annotations, no ids) | **80** (exact id set) | 80 |
| `IFCSHADINGDEVICE` | 0 | **20** (exact id set) | 20 |
| `IFCWALL` / `IFCDOOR` / `IFCWINDOW` / `IFCCOLUMN` | 360 / 132 / 6 / 256 | 360 / 132 / 6 / 256 | same |
| building elements | 836 | 872 | — |
| with a recovered body | 754 | 854 | — |
| `storey_bound_elements` | 743 | 794 | — |
| `IFCBUILDINGSTOREY` | 11 | 11 | 15 |
| `IFCPROPERTYSET` | 818 | 854 | 0 |

Exactness was checked on the emitted IFC with IfcOpenShell 0.8.5 (`Tag` sets,
`include_subtypes=False`) and the reference side independently with IFClite
7.1.1: `IfcSlab` 80/80, `IfcShadingDevice` 20/20, `IfcWall` 360/360,
`IfcDoor` 132/132, `IfcWindow` 6/6, `IfcColumn` 256/256 — zero missing, zero
extra in every case. `tools/ci/ifc_schema_arity.py` passes on the fresh
export: 16 674 instances across 37 entity types, `IfcShadingDevice` at its
declared arity of 9 with `PredefinedType` written as `$` rather than an
invented `.NOTDEFINED.`.

Both OctetProof verdicts stay `PASS`. The full-project claimed surface goes
**8 → 10 fields** (`entity_counts.IFCSLAB` and
`entity_counts.IFCSHADINGDEVICE` join), excluded 5 → 4.

## 9. Reproduction

```bash
cargo build --profile ci --example probe_slab_instance_rule
./target/ci/examples/probe_slab_instance_rule \
  "$RVT_PROJECT_CORPUS_DIR/2024_Core_Interior.rvt" > slabs.json

# the OST_BuildingPad record behind the 80th slab
cargo build --profile ci --example probe_element_record_owner_lookup
./target/ci/examples/probe_element_record_owner_lookup \
  "$RVT_PROJECT_CORPUS_DIR/2024_Core_Interior.rvt" 21975
```

The shipped path is
`rvt::partition_schema_mvp::slabs_from_partition_category_records` plus
`rvt::partition_ifc_export_overrides::scan_ifc_export_overrides`; the corpus
gate is `tests/iter_elements_typed.rs::core_interior_2024_slab_instances_and_export_overrides`
and `tests/project_count_fixtures.rs`.
