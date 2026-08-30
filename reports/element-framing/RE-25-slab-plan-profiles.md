# RE-25 — slab plan profiles: the sketch is in the file, one element record per boundary line

Status: **positive result**, exact on all 80 exported `IFCSLAB`.
Closes the profile half of #31. Date: 2026-08-30.
Artifact: `2024_Core_Interior.rvt`
(sha256 `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014`,
Revit 2024, magnetar-io/revit-test-datasets, MIT).
Reference: `IFC Exports/2024_Core_Interior_slim.ifc`
(sha256 `bfdf36ffb0bb768f3409d818403990e64d4c262c6780603be87f8077387ad86d`),
Revit's own full-project export, read with IfcOpenShell 0.8.5.

RE-22 recovered all 80 slabs by ElementId and closed the thickness half of
#31, and left one sentence open: *"The profile is still the bbox rectangle,
not the recovered floor boundary polygon: `ProfileResolved` stays false."*
This closes it. The profile is not in the plan-loop scan, and it is not a
polyline anywhere: **each boundary line of a Revit sketch is its own
partition element record**, carrying its own bounding box and naming the
sketched element outright.

## 1. What the reference export actually asks for

The 80 `IfcSlab` bodies carry 81 `IfcExtrudedAreaSolid` between them
(22756 `Floor:Basement Slab` is two stacked solids, RE-22 §5), and every
swept area is an arbitrary profile — no `IfcRectangleProfileDef` anywhere:

| swept area | bodies | outer vertices | inner curves |
|---|---:|---:|---:|
| `IfcArbitraryProfileDefWithVoids` | 42 | 26 | 1, of 4 vertices |
| `IfcArbitraryClosedProfileDef` | 39 | 4 | — |

Reduced to the project plan frame, the 80 slabs use only **four** distinct
polygons:

| shape | slabs | outer | void | plan bounds (ft) |
|---|---:|---|---|---|
| perimeter ring | 42 | 26-gon, sawtoothed on both long sides | rectangle `20,25 – 167,114` | `9,16 – 177,123` |
| floor plate | 36 | rectangle | — | `20,25 – 167,114` |
| inset plate | 2 | rectangle | — | `21.5,26.5 – 165.5,112.5` |

38 of the 80 are therefore plain axis-aligned rectangles, and each of those
rectangles **is** the record bounding box RE-22 already emitted — so the
pre-RE-25 export was accidentally exact on 38 of 80 and wrong on 42, where
it filled a ring that is a band around a courtyard. That distinction is why
the profile could not simply be "the outer loop": emitting a 26-gon without
its void states more solid than the source carries.

## 2. Where the profile is not: the plan-loop scan (negative)

The scanner `partition_schema_mvp::scan_closed_plan_loops` reads runs of
`(x, y)` `f64` pairs and keeps the ones that close. Inventoried over every
inflated `Partitions/*` stream with the shipped constants (4–8 points, span
≥ 5 ft, closure ≤ 0.05 ft):

| stream | inflated bytes | closed candidates | point counts | on a recovered plate's plan box |
|---|---:|---:|---|---:|
| `Partitions/46` | 98 772 093 | 1249 | 4 × 1246, 5 × 1, 6 × 2 | 0 |
| `Partitions/48` | 17 142 729 | 640 | 4 × 640 | 0 |
| `Partitions/51` | 3 315 208 | 365 | 4 × 365 | 0 |
| `Partitions/53` | 32 170 195 | 4 | 5 × 3, 6 × 1 | 0 |
| `Partitions/55` | 13 074 355 | 6 | 4 × 6 | 0 |
| `Partitions/59` | 9 239 919 | 36 | 4 × 36 | 0 |
| `Partitions/61` | 13 440 880 | 15 | 4 × 15 | 0 |
| `Partitions/65` | 443 218 | 2 | 4 × 2 | 0 |
| **total** | | **2317** | 4 × 2310, 5 × 4, 6 × 3 | **0** |

Not one of the 2317 has the plan bounds of any of the 100 record-backed
plates, and the widest point count anywhere is six — against the 26 the
perimeter ring needs. The plan-loop route was never going to reach these
profiles, with or without an ElementId join. Independently: searching all
187 MiB of inflated partitions for the export's polygon as a packed run of
`(x, y)` doubles — in the project frame, in the export's own profile frame,
and at strides of 2, 3, 4, 5, 6 and 8 doubles — finds **no** ordered vertex
run at all. Revit does not store this boundary as a polyline.

## 3. Where it is: one element record per sketch line

`BuiltInCategory` **`OST_SketchLines` = -2000045** (Autodesk's published
constant; the bytes are `53 7b e1 ff ff ff ff ff` at `+0x12`, the same
signed `i64` slot RE-21 documented). Records of this category decode
through the *unmodified* RE-21 header: declared ElementId at `+0x00`,
`0x059f` at `+0x0c`, the fixed bbox marker at `+0x50`, the box at `+0x58`,
the counted reference list at `+0x88`. On Core Interior they are 241 bytes
each and lie contiguously, one run per sketch.

| | |
|---|---:|
| `OST_SketchLines` records found | 3688 |
| distinct sketch-line ElementIds | 3070 |
| streams they live in | `Partitions/46` 3070, `Partitions/51` 570, `Partitions/55` 48 |
| frames of one id that disagree on box or owner | **0** |

The record's box is the **segment**: for the boundary line at
`x = 136.5, y = 115 … 123` the recorded box is exactly
`136.5, 115 – 136.5, 123`, degenerate in `x`.

## 4. The join: the second counted reference list

RE-23 read the *first* counted list at `+0x88` and took the slot before the
record's own id to find a door's host wall. A **second** list is framed
identically right after it — `u32` count, then that many `u64` slots — and
its last slot names the sketched element:

```text
+0x88  u32  n1 = 5
+0x8c  5×u64  20309, 20310, 20312, 20313, 20315   (sibling sketch lines)
+0xb4  u32  n2 = 2
+0xb8  2×u64  20308, 20311                        ← 20311 is the slab
```

Measured over all 3688 records: 6 carry no second list, 1377 carry one of
length 1 and 2305 one of length 2. The last slots name **126** distinct
owners, and

| owners | count |
|---|---:|
| exported `IFCSLAB` ElementIds | **80 / 80** |
| exported `IFCSHADINGDEVICE` ElementIds | **20 / 20** |
| other elements (walls, the superseded container twin 16925, …) | 26 |

Every one of the 100 record-backed plates RE-22 recovers is named by its own
sketch lines. **The join is by ElementId only — no geometry, no proximity,
no containment.** A sketch line belongs to the element the byte names or to
nothing.

## 5. The reconstruction: closure, not fitting

A box is not a segment when the segment is diagonal — `24,115 – 49,123` is
either `(24,123) → (49,115)` or `(24,115) → (49,123)`, and both connect
existing vertices. And 37 of the 1454 plate segments carry a box that is
*looser* than the line: the top boundary of every rectangular plate is
recorded as `20,113 – 167,115` for a line that the export puts at `y = 114`
(and `21.5,111.5 – 165.5,113.5` for `y = 112.5` on the two inset plates).
Both are resolved by the loop, never by the reference export:

1. Every segment degenerate on exactly one axis contributes its endpoints
   outright.
2. Every remaining segment is placed only when exactly **one** pair of
   still-open vertices fits its box — corner pairs (the box's own
   diagonals) first, and only when no corner pair is open, a pair that
   spans the box lengthwise and is degenerate across it.
3. Repeat until nothing is left. A segment that never has exactly one
   candidate, a vertex that does not finish at degree 2, an unused edge, a
   zero-extent box, or a loop shorter than three vertices rejects **the
   whole element**.

On the sawtooth this is forced rather than fitted: `(24,115)` already has
degree 2 from the horizontal `9 → 24` and the vertical at `x = 24`, so the
diagonal over `24 … 49` can only take `(24,123)` and `(49,115)`, which
closes `(49,115)` and forces the next diagonal, and so on. On the loose top
edge no corner pair is open at all — its box corners are `y = 113` and
`y = 115`, which no other segment touches — and the only spanning pair is
the `(20,114)`, `(167,114)` the three tight edges left open.

Collinear vertices are then merged. Revit splits the ring's north-east run
at `x = 167`, so the raw chain has 27 outer segments where the export has
26 edges; merging is what makes the two comparable, and it is also what
makes the emitted profile independent of how finely Revit happened to split
a straight run.

## 6. Profile agreement (#31)

Recovered profile vs the reference export's swept area, both reduced to the
project plan frame, compared as cyclic vertex sequences with orientation
normalised, tolerance 1e-3 ft per vertex:

| slabs | recovered | export | outer vertices | voids | exact | near | miss |
|---:|---|---|---:|---:|---:|---:|---:|
| 42 | outer ring + 1 void | `IfcArbitraryProfileDefWithVoids` | 26 | 1 (4 vertices) | **42** | 0 | 0 |
| 38 | single loop | `IfcArbitraryClosedProfileDef` | 4 | 0 | **38** | 0 | 0 |
| **80** | | | | | **80** | **0** | **0** |

122 loops compared (80 outer + 42 inner). **Worst vertex deviation
1.563e-12 ft** — the recovered vertices are the same doubles as the
export's, differing only in the floating dust of Revit's own transform
(`177.00000000000011` for a nominal 177 ft, which the record bounding box
and the export's profile carry identically). Re-measured on the emitted IFC
after the metre round-trip: 0 mismatches, worst deviation 2.842e-14 ft.

No frame alignment was needed. The 2024 project frame and the export frame
coincide: the recovered vertex `9.0000000000004121` is the export's
`9.000000000000412`.

### 6.1 Segment budget per slab

| slabs | distinct sketch lines | loops closed |
|---:|---:|---|
| 42 | 31 | 27 outer segments → 26 edges after merge, plus a 4-segment void |
| 38 | 4 | 4 |

## 7. The twenty shading devices: declined, with the reason

The same join names all 20 exported `IFCSHADINGDEVICE`, and the
reconstruction refuses every one:

| owner | sketch lines | zero-extent box | axis-parallel box | diagonal box | profile |
|---|---:|---:|---:|---:|---|
| each of the 20 | 57 | 1 | 35 | 21 | **none** |

The first zero-extent box rejects them outright. Even without it, the
reference export writes each as a 29-vertex ring with one void whose
vertices are not axis-parallel — `0 … 182.3333` by `8.6936 … 130.3064` —
so 21 diagonal boxes would have to be resolved against each other with no
axis-parallel anchor to force the first choice. They keep the record box
rectangle and `ProfileResolved: false`. `Floor:…` plates and shading plates
are the same 100 records under one instance rule (RE-22); this is the first
measurement where the two halves separate.

Five further owners are declined the same way: 16925 (the container-member
twin of 20953, already excluded by the RE-21 instance rule), 31636, 31637,
34362 and 55840. Twenty-one non-plate owners *do* close — 12 rectangles and
9 rings — and are recovered but not emitted: nothing in this PR maps them
to an IFC entity.

## 8. What ships

- `partition_element_records`: `OST_SKETCH_LINES`, and
  `PartitionElementRecord::owner_reference` — the last slot of the second
  counted list, decoded by `decode_owner_reference`, fail-closed on a
  missing list or a slot outside the `u32` id range.
- `partition_element_records::scan_category_records_multi`: one inflate per
  stream for several categories. The slab path now reads `OST_Floors`,
  `OST_BuildingPad` and `OST_SketchLines` in **one** sweep instead of two,
  so recovering the profile costs one extra needle search, not another
  190 MiB of inflation.
- `element_record_plan_profiles`: the grouping, the closure solver, the
  collinear merge, and the `m_plan_profile_*` fields.
- `ifc::entities::ProfileDef::ArbitraryWithVoids` and the
  `IFCARBITRARYPROFILEDEFWITHVOIDS` writer path.
- `ifc::export_content`: `ProfileResolved` is now `true` when a profile
  closed, with `ProfileSource`, `ProfileVertexCount` and `ProfileVoidCount`
  beside it. `BodySource` deliberately stays
  `partition_element_record_bbox` — the placement, the plan envelope and
  the extrusion depth are all still read from the record box, and the
  storey join in `ifc::mod` keys on that value.

## 9. Measured before / after on `2024_Core_Interior.rvt`

Building-element counts, diagnostics and both OctetProof verdicts are
**unchanged**; the emitted IFC differs only in the profile chain.

| entity | before | after |
|---|---:|---:|
| `IFCRECTANGLEPROFILEDEF` | 992 | 912 |
| `IFCARBITRARYCLOSEDPROFILEDEF` | 0 | 38 |
| `IFCARBITRARYPROFILEDEFWITHVOIDS` | 0 | 42 |
| `IFCPOLYLINE` | 0 | 122 |
| `IFCCARTESIANPOINT` | 1847 | 3381 |
| `IFCPROPERTYSINGLEVALUE` | 5371 | 5611 |
| total instances | 18209 | 20105 |
| `IFCSLAB` / `IFCSHADINGDEVICE` / `IFCWALL` / `IFCDOOR` / `IFCWINDOW` / `IFCCOLUMN` | 80 / 20 / 360 / 132 / 6 / 256 | unchanged |
| `IFCBUILDINGSTOREY` | 15 | 15 |
| `storey_bound_elements` | 801 | 801 |
| diagnostics sidecar | — | byte-identical to `main` |

(Baseline is `main` at a0044f4, after RE-24 / #230 landed the 15 Level
records.)

`tools/ci/ifc_schema_arity.py` passes on the fresh export: 20 105
instances across 43 entity types, `IfcArbitraryClosedProfileDef` at its
declared arity of 3 and `IfcArbitraryProfileDefWithVoids` at 4,
`IfcPolyline` at 1. Both verdicts stay `PASS` — element fixture 6 fields /
10 excluded, full project 13 fields / 3 excluded, 0 diffs, all three
witnesses replaying against the committed observations. The two committed
`rvt-rs` observations were regenerated because the observation payload
counts *every* emitted entity type; the four bridge-witness observations
and both `verdict.json` files are byte-identical.

## 10. Open

- **The 20 shading devices** keep the box rectangle (§7). Their sketch is
  present and joined; only the closure declines. Resolving 21 mutually
  constraining diagonal boxes needs either a second carrier for the segment
  endpoints or a global search with a uniqueness proof — neither is in this
  PR.
- **Walls, doors, windows, columns** still carry the record box. Wall
  sketches are not `OST_SketchLines`; the 26 non-plate owners here are a
  first hint of what else is sketched, not a claim.
- **`IfcExtrudedAreaSolid.Position`** is emitted as the element's *own*
  `IfcAxis2Placement3D`, which is also the `IfcLocalPlacement`'s relative
  placement, so the element translation is applied twice by a consumer that
  composes both. This predates RE-25 and affects every emitted element on
  every release; the profile is expressed in the same element-local frame
  the rectangle profile always used, so nothing regressed. It needs its own
  issue and its own before/after.
- **#227** (opening geometry from the wall location curve; the 42 slab and
  20 shading-device penetrations) is untouched. The `IfcRelVoidsElement`
  pairs whose filling is absent are still unrecovered — but note that the
  perimeter ring's void is *profile* geometry, not an opening element, so
  the 42 slab penetrations #227 asks for are a different set.

## 11. Reproduction

```bash
cargo build --profile ci --example probe_re25_slab_plan_profiles
./target/ci/examples/probe_re25_slab_plan_profiles \
  "$RVT_PROJECT_CORPUS_DIR/2024_Core_Interior.rvt" > profiles.json
```

The shipped path is
`rvt::element_record_plan_profiles::plan_profiles_from_sketch_line_records`,
reached through
`rvt::partition_schema_mvp::slabs_from_partition_category_records`; the
corpus gate is
`tests/iter_elements_typed.rs::core_interior_2024_slab_plan_profiles` and
the unit gates are `src/element_record_plan_profiles.rs::tests`.
