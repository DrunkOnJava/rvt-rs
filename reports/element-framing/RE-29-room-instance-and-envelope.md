# RE-29 — rooms are element records, and they carry their own name; the boundary polygon is not in the file

**Issue:** #90 (`Refs` #219, #33)
**Date:** 2026-09-19
**Result:** **positive** on the room instance set, the name, the number,
the host storey and the plan/vertical envelope — 116 of 116 on every
one, tolerance 0. **Measured negative** on the boundary polygon.
**Corpus:** `2024_Core_Interior.rvt`, sha256
`c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014`,
Revit 2024, MIT, magnetar-io/revit-test-datasets.
**Reference:** `IFC Exports/2024_Core_Interior_slim.ifc`, sha256
`bfdf36ffb0bb768f3409d818403990e64d4c262c6780603be87f8077387ad86d`,
Revit's own full-project export, read with the repo's own STEP reader.
**Probe:** `examples/probe_re29_room_records.rs`.

-----

## 1. The state #90 describes, and what the reference asks for

v0.2.0 emitted `IFCSPACE` rows from `partition_name_candidates` — a
string scan — with no ElementId, no placement and no body, and #90
recorded them as carrying a "placeholder 10×10×8 ft bounding box". On
`main` before this work the shape was worse than that description: 18
rows, all named `Room-unnamed`, `Representation` `$`, no body at all,
and no storey (RE-27 §6 lists them as the largest remaining unbound
set).

The reference export carries **116** `IfcSpace`. It does not tag them:
`IfcSpace` is an `IfcSpatialStructureElement` and declares no `Tag`
attribute, so the source ElementId travels one hop away, in the
`IfcSpaceType` its `IfcRelDefinesByType` points at:

```text
#186=IFCSPACE(…,'66',$,$,#177,#185,'Stair 2',.ELEMENT.,.SPACE.,$);
#187=IFCSPACETYPE(…,'Stair 2 66:20822',$,$,$,$,'20822',$,.NOTDEFINED.,$);
```

So the reference side is: **ElementId 20822, number `66`, name
`Stair 2`**, aggregated into `IfcBuildingStorey` `Level 6`, with a body
that is an `IfcExtrudedAreaSolid` over an `IfcIndexedPolyCurve`.

| in the export | value |
|---|---|
| `IFCSPACE` / `IFCSPACETYPE` | 116 / 116 |
| distinct room ElementIds | 116 (20822 … 63792) |
| storeys holding a space | 10 of 15 (`Level 3 - Wall Layouts 1` … `Level 12`) |
| spaces per storey | 10, 12, 12, 12, 12, 12, 12, 12, 12, 10 |
| profile | `IfcArbitraryClosedProfileDef` on all 116 |
| outer vertices | 4 ×40, 5 ×16, 8 ×4, 9 ×36, 10 ×1, 11 ×16, 12 ×1, 14 ×1, 22 ×1 |
| extrusion depth | `8.0` ft on all 116 |
| solid base `z` | equals the aggregating storey's `Elevation` on 116 of 116 |

There are only **186 distinct vertices** and **108 distinct
coordinates** across all 116 polygons, because the building is ten
repeats of the same plan. That matters in §4.

## 2. The carrier: `OST_Rooms`, under the RE-21 instance rule, measured not assumed

The category was not looked up and trusted. `probe_re29_room_records`
brute-scans every offset of every inflated `Partitions/*` stream for a
decodable element-record header — the same sweep
`probe_element_record_owner_lookup` uses — histograms every
`BuiltInCategory` it finds, and scores each category's RE-21
instance-rule selection against the export's 116 room ElementIds.

23 470 records decode, across 30 categories. One category intersects
the wanted set, and it is exact:

| category | records | ids | instance rule selects | ∩ wanted | TP | FP | FN |
|---:|---:|---:|---:|---:|---:|---:|---:|
| `-2000160` (`OST_Rooms`) | 399 | 277 | **116** | 116 | **116** | **0** | **0** |
| every other category | 23 071 | — | — | **0** | — | — | — |

`-2000160` is Autodesk's published `BuiltInCategory.OST_Rooms`. The rule
applied is RE-21's unchanged: `u64 @ +0x32 == 0xFFFF_FFFF_FFFF_FFFF`
**and** `u32 @ +0x42 == 0xFFFF_EF7F`. It selects 116 of the 277 ids that
carry the category; the other 161 are container members.

Every one of the 116 is framed more than once — 110 twice, 6 three
times — and the RE-26 tie-break (greatest bbox `z` extent, then the
**newest** `(stream, offset)`) matters enormously here. §3 measures it.

## 3. The record box is the room's plan envelope and its real height

Scored against the reference polygon's plan bounding box and the
solid's world `z` range, tolerance 1e-3 ft:

| record chosen per id | plan bbox exact | worst plan Δ | `z` base exact | `z` top exact |
|---|---:|---:|---:|---:|
| first by `(stream, offset)` | 5 / 116 | 0.166667 ft | 116 | 116 |
| **newest by RE-26** | **116 / 116** | **3.730e-12 ft** | **116** | **116** |

111 of the 116 rooms carry frames that disagree about the plan box and
**none** disagrees about `z`, which is the same shape RE-26 measured for
walls. The older frame is consistently 1 in larger on the long axis —
room 20822 is `y 58.25 … 80.75` in `Partitions/46` and
`y 58.3333 … 80.6667` in `Partitions/59`, and the export's polygon spans
exactly the latter.

The vertical extent is not an envelope either: the record's `z` is
`76.0 … 84.0` where the export places the solid at `z = 76` (the storey
elevation) and extrudes it 8 ft. That holds on all 116 — base and top
both exact.

**So the 10×10×8 ft placeholder is replaced by a measured box, not by a
better guess.** Re-measured end to end on the emitted IFC, after the
metre round-trip and the writer's six-decimal rounding: plan envelope
exact on 116 of 116, worst 1.640e-06 ft; `z` range exact on 116 of 116,
worst 1.421e-14 ft.

### 3.1 How good is a box, honestly

A box is not the polygon. Comparing shoelace areas:

| | |
|---|---:|
| total reference polygon area | 19 582.1 sq ft |
| total record-box area | 20 870.7 sq ft |
| ratio | **1.0658** |
| rooms where box area **equals** polygon area | **76 / 116** |
| rooms where the box over-states | **40 / 116** |
| worst ratio | 2.037 (room 25945, the 22-vertex space) |

76 of the 116 rooms are rectangles (40 of them written with four
vertices, the rest with collinear splits at wall junctions), and for
those the emitted body **is** the room. For the other 40 it is the
convex plan envelope of an L, a T or a ring. `ProfileResolved` stays
`false` and the property set says the body came from the record box,
so nothing reads the rectangle as a modelled boundary.

## 4. The boundary polygon: measured negative

Four independent tests, all negative.

**4.1 The polygon is not stored as an ordered vertex run.** For each of
the 116 reference polygons, every byte offset of every inflated
partition was tested as the start of that polygon's vertex sequence,
cyclically from any of its vertices, at strides of **2, 3, 4 and 6**
doubles per vertex, in **both** windings, quantised to 1e-6 ft:

| stride (doubles/vertex) | winding | longest run found | complete polygons |
|---:|---:|---:|---:|
| 2 | forward | 2 | 0 / 116 |
| 2 | reverse | 2 | 0 / 116 |
| 3 | forward | 2 | 0 / 116 |
| 3 | reverse | 1 | 0 / 116 |
| 4 | either | 1 | 0 / 116 |
| 6 | either | 1 | 0 / 116 |

Against polygons of 4 to 22 vertices. A **translation-invariant** pass
— anchored on an edge *delta* rather than an absolute vertex, so a
polygon stored in a local frame is still found — reaches a longest run
of **3** (76 polygons at 2, 40 at 3) and **0 of 116** complete.

This is not a "the numbers are absent" result: all **108** distinct
reference coordinates and all **186** distinct `(x, y)` vertices *are*
present in the file as adjacent double pairs. They are there because
they are wall faces. What is absent is any place where a room's own
vertices sit in order.

**4.2 Rooms are not sketched.** RE-25's join — the last slot of the
second counted reference list on an `OST_SketchLines` record — names
**126** distinct owners on this file. **0** of them is a room.

**4.3 The record tail holds no geometry.** The 64 doubles that follow
both counted reference lists on every room record are `0.0`, `NaN`
sentinels and small unattributed words. No coordinate.

**4.4 The reference list is too short to be the bounding set.** A room
record's list is `[3, <Level>, <wall>, <wall>, <own id>]` on **all 116**
— length 5 every time — while room 20822 alone has nine boundary
vertices. Two named walls cannot describe them.

Taken together these say what Revit's own model says: a room's boundary
is *computed* from its room-bounding elements, not stored with the room.
Recovering it means solving for the enclosure from the recovered wall
set, which is a different problem from decoding a carrier, and it is not
attempted here. **#90's polygon half stays open**; its placement,
height, identity and naming halves are closed.

## 5. The name and the number: a parameter block, read forward

RE-22 found the `IFC Export As` override by anchoring on a value string
and reading two copies of the owning `ElementId` at fixed *negative*
offsets. A room's number and name are framed the same way and are read
from the other end — anchor on the owner, walk **forward**:

```text
+0x000  u64  owning room ElementId
+0x03c  u64  owning room ElementId        (confirmation; must agree)
+0x044  u64  host Level ElementId
+0x1f5  u32  number length, UTF-16 units
+0x1f9  2*n  room Number, UTF-16LE
+…+ 8   u32  name length, UTF-16 units    (8 bytes past the number's end)
+…+12   2*m  room Name, UTF-16LE
```

The name's offset is *not* fixed, because the number's length is not —
which is why the block is parsed forward from the owner rather than
backward from either string, and why the name-carrier histogram that
found it shows three offsets (−527 / −525 / −523 relative to the name
start) rather than one.

**Precision, measured over the whole id space.** The framing was run
against every one of the **26 425** ElementIds `Global/ElemTable`
declares. It accepts **116** of them, and they are exactly the 116
rooms. **No other declared id accepts it at all.**

**Recall and correctness.** All 116 rooms are framed **twice**, and no
pair disagrees on the number, the name or the Level slot; recovery is
all-or-nothing per room, so a disagreement would drop the room rather
than pick a side. Scored against the reference export:

| | |
|---|---:|
| rooms with an accepted block | **116 / 116** |
| rooms whose blocks agree | **116 / 116** |
| recovered name equals `IfcSpace.LongName` | **116 / 116** |
| recovered number equals `IfcSpace.Name` | **116 / 116** |
| non-room declared ids accepting the framing | **0 / 26 309** |

`Stair 2` / `66`, `Tennant Storage Suite 1` / `68`, `Machine Room 1` /
`69`, `Elevator 1` / `70` — exactly what Revit writes.

## 6. Storey containment (#219)

Two independent carriers agree on all 116.

- **RE-27's rule**, unchanged: the single recovered `Level` ElementId in
  the record's counted list at `+0x88`. Every room record names
  **exactly one**, so all 116 bind and none is declined for ambiguity —
  unlike the columns and walls, which carry a base *and* a top
  constraint.
- **The parameter block's own slot** at `+0x44` names the same Level on
  116 of 116. It is attached only when the reference list did not
  already answer, so RE-27 stays the primary carrier; here it never has
  to.

Independently, the record's base `z` equals the reference storey's
`Elevation` on 116 of 116.

And the join is right, not merely self-consistent: the ten Level
ElementIds named map to exactly one reference storey each, and it is the
storey that export aggregates the room into.

| named Level | reference storey | elevation |
|---:|---|---:|
| 20274 | `Level 3 - Wall Layouts 1` | 31 ft |
| 20276 | `Level 4 - Wall Layouts 2` | 46 ft |
| 20277 | `Level 4 - Wall Layouts 3` | 61 ft |
| 20308 | `Level 6` | 76 ft |
| 20307 | `Level 7` | 91 ft |
| 20306 | `Level 8` | 106 ft |
| 20305 | `Level 9` | 121 ft |
| 20304 | `Level 10` | 136 ft |
| 20303 | `Level 11` | 151 ft |
| 20302 | `Level 12` | 166 ft |

This is the first part of RE-27's storey bind to be scored against
Revit's own containment. The other 853 bindings stay **NOT MEASURED**.

## 7. Measured before / after on `2024_Core_Interior.rvt`

`target/ci/rvt-ifc 2024_Core_Interior.rvt --mode geometry`:

| | before | after |
|---|---:|---:|
| `IFCSPACE` | 18 (name-only, no body, no id) | **116** (exact id set) |
| building elements | 872 | **970** |
| with a recovered body | 854 | **970** |
| `storey_bound_elements` | 853 | **969** |
| elements reaching a storey by record Level reference | 372 | **488** |
| elements on the `IfcBuilding` (storey unknown) | 19 | **1** |
| `IFCPROPERTYSET` / `IFCRELDEFINESBYPROPERTIES` | 854 | 970 |
| `IFCPROPERTYSINGLEVALUE` | 9231 | 10 391 |
| `IFCEXTRUDEDAREASOLID` / `IFCSHAPEREPRESENTATION` | 992 | 1108 |
| `IFCRECTANGLEPROFILEDEF` | 912 | 1028 |
| `IFCCARTESIANPOINT` | 3381 | 3613 |
| total instances | 23 726 | 26 358 |
| `IFCWALL` / `IFCDOOR` / `IFCWINDOW` / `IFCCOLUMN` / `IFCSLAB` / `IFCSHADINGDEVICE` | 360 / 132 / 6 / 256 / 80 / 20 | unchanged |
| `IFCBUILDINGSTOREY` / `IFCRELFILLSELEMENT` / `IFCRELCONTAINEDINSPATIALSTRUCTURE` | 15 / 138 / 14 | unchanged |
| `unsupported_features` | includes `partial_element_geometry` | **gone** |
| geometry warnings | `curve=18, missing_level=19, missing_dimensions=18` | `missing_level=1` |

`partial_element_geometry` disappearing is the honest consequence of the
last geometry-free elements on this file gaining a body: every one of
the 970 emitted building elements now carries one.

Both OctetProof verdicts stay `PASS`, and the full-project claimed
surface goes **13 → 14 fields** with excluded 3 → 2:
`entity_counts.IFCSPACE` joins it, where rvt-rs, IfcOpenShell 0.8.5 and
IFClite 7.1.1 all report 116.

## 8. What ships

- `partition_element_records::OST_ROOMS`.
- `partition_room_parameters` — the block framing, the confirmation
  test, the forward parse, and `resolve_unique`, which drops a room
  whose blocks disagree.
- `partition_schema_mvp::rooms_from_partition_category_records` /
  `room_instances_from_records`, and `recover_partition_schema_mvp`
  standing the string-derived rooms down whenever records decode — the
  same trade the plan-loop floors take against record-backed slabs.
- `ifc::export_content`: `RoomNumber` and `RoomName` join the record
  property set.
- `ifc::step_writer`: `IfcSpace.LongName` is the recovered room name
  when there is one, which is the slot Revit's own exporter uses for it.
  The element `Name` keeps the `Room-<ElementId>` identity every other
  class uses, because `IfcSpace` declares no `Tag` and the STEP line
  would otherwise carry no id at all.

## 9. What is not claimed

- **The boundary polygon** (§4). Not stored, not sketched, not in the
  record tail. Open.
- **`IfcSpace.Name`.** Revit puts the room *number* there; rvt-rs puts
  `Room-<ElementId>`, because that slot is the only place an `IfcSpace`
  can carry its source id in this writer. The number is in the property
  set. Splitting the two needs an `IfcSpace.Name` distinct from the
  element identity, which the entity model does not have today.
- **Area and volume.** Revit computes and stores a room's area; nothing
  here reads it, and nothing emits `IfcQuantityArea`.
- **`IfcRelSpaceBoundary`.** The reference export carries none either.
- **Scope.** Revit 2024 partition element records, one recorded edge.
  On a release or file where `partition_element_records` declines, no
  room record decodes, the string-derived rooms stay in place, and
  nothing changes — verified unchanged on the 2023 Einhoven sample.

## 10. Reproduction

```bash
cargo build --profile ci --example probe_re29_room_records
./target/ci/examples/probe_re29_room_records \
  "$RVT_PROJECT_CORPUS_DIR/2024_Core_Interior.rvt" \
  --ids rooms.txt --verts polygons.txt --names names.txt
```

`rooms.txt` is one reference room ElementId per line, `polygons.txt` is
`id x,y x,y …` per line, `names.txt` is `id<TAB>name`; all three are
extracted from `IFC Exports/2024_Core_Interior_slim.ifc`. With no
`--ids` the probe still prints the category histogram, which is the
measurement §2 rests on.

The shipped path is
`rvt::partition_schema_mvp::rooms_from_partition_category_records` plus
`rvt::partition_room_parameters::scan_room_parameters`; the corpus gate
is
`tests/iter_elements_typed.rs::core_interior_2024_room_instances_names_and_envelopes`,
which re-reads the reference export and scores the id set, the names,
the numbers and the Level bindings directly, and the unit gates are
`src/partition_room_parameters.rs::tests`.
