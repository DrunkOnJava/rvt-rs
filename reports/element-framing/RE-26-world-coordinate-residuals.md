# RE-26 — world-coordinate residuals: the column Z gap was a metric artifact, the wall gap was two real ones

Status: **positive result on walls**, exact on 336 of 360; **measured
negative on columns**, with the family/type join closed on 256 of 256.
Closes #215. Date: 2026-08-30.
Artifact: `2024_Core_Interior.rvt`
(sha256 `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014`,
Revit 2024, magnetar-io/revit-test-datasets, MIT).
Reference: `IFC Exports/2024_Core_Interior_slim.ifc`
(sha256 `bfdf36ffb0bb768f3409d818403990e64d4c262c6780603be87f8077387ad86d`),
Revit's own full-project export, read with IfcOpenShell 0.8.5.

#236 fixed the double translation and, for the first time, made the
world-coordinate gap between rvt-rs and Revit's own export small enough
to read. Its follow-up list said *"columns sit 0.125 ft off in plan and
1.58 ft off in Z, walls up to 2.53 ft."* This report takes those two
numbers apart. One of them was not a geometry gap at all.

## 1. The metric, and why #236's numbers read the way they did

#236 scored `max centroid delta`, where the centroid is the **mean of
the emitted mesh vertices**. That is only comparable when the two sides
tessellate the same way, and they do not: rvt-rs emits a box (an
extruded rectangle) while Revit emits a `IfcPolygonalFaceSet` for 62 of
its 256 columns and 105 of its 360 walls. The vertex mean of a box is
its centre; the vertex mean of a mesh is wherever its facets are dense.

Everything below therefore scores the **world axis-aligned bounding
box**, `max(|Δmin|, |Δmax|)` over the three axes, in feet, matched by
`Tag`, from the IfcOpenShell geometry iterator with `use-world-coords`.
It is the same geometry, read the same way; only the summary changes,
and it is invariant to tessellation.

Under that metric, `#236`'s "1.583 ft in Z" for columns is **0.0000 ft**:

| IfcColumn 20375 | rvt-rs | Revit | Δ |
|---|---|---|---|
| world z min (ft) | 76.0000 | 76.0000 | 0.0000 |
| world z max (ft) | 90.3333 | 90.3333 | 0.0000 |
| vertex-mean z (ft) | 83.1667 | 81.5833 | 1.5833 |

All 256 columns agree with Revit's world z extent exactly, at both ends.
No column carries a base-plate offset the record box does not already
have, and the placement is not at the wrong end of the box. Both #236
follow-up hypotheses for the column Z are **rejected by measurement**.

## 2. Columns: the hypotheses, and what the file answers

| # | hypothesis | test | verdict |
|---|---|---|---|
| C1 | the bbox spans a base plate while Revit extrudes between bound levels | record box z vs Revit world z, 256 columns | **rejected** — exact on 256/256, both ends |
| C2 | the placement should sit at the box base rather than its centroid | as above | **rejected** — the 1.5833 ft was the vertex-mean artifact of §1 |
| C3 | the profile is the type symbol's real section, joined via the `+0x88` list | join, then compare sections | **join exact, 256/256**; the section is the one the envelope already had |
| C4 | the plan residual is the section being wrong | record plan extents vs Revit's body plan extents | **rejected** — the envelope is 2 × 2 ft on all 256; Revit's *body* is inset on 80 |

### 2.1 C3 — the family/type join (#215)

RE-21 §5 identified 17 `OST_Columns` records carrying
`+0x42 == 0xffff8000`, the type-symbol envelopes. #215 proposed joining
each instance to its symbol through the counted reference list at
`+0x88`. The rule that works is the same shape RE-23 used for the host
wall — **the last slot before the record's own ElementId that is itself
a type-symbol record of the same category**:

```text
20375  refs1 = [3, 4258, 4546, 5755, 20307, 20308, 20375, 20843, 75407]
                              ^^^^                ^^^^^         ^^^^^
                              type              own id      a symbol,
                                                            after self
```

Two of the nine slots are `OST_Columns` symbols. Taking the one before
the record's own id gives `5755`; taking the last slot would give
`75407`. Measured over all 256 exported columns: **every one has exactly
one symbol slot before its own id, and it is `5755` on all 256** —
`Column_Sqaure:24" x 24"`, which is the `IfcColumnType.Tag` Revit's own
export writes for every one of them. The 12 `IfcColumnType` rows in the
export all carry that Tag.

Symbol `5755`'s own bounding box is the section in family coordinates:

```text
(-1.0, -1.0, -0.0) → (1.0, 1.0, 9.0)      a 2 ft square, 9 ft tall
```

And that is the honest result of the join: **the section it yields is
the rectangle the instance envelope already carried.** Every one of the
256 instance boxes has plan extents 2.0 × 2.0 ft, and the symbol's are
the same to a worst disagreement of **8.0e-15 ft**. Adopting the type
section therefore moves no vertex on this file — the pinned composition
probe for column 20375 in `tools/ci/ifc_schema_arity.py` is unchanged to
1e-10.

That is not nothing. Before this, `ProfileResolved` was `false` for a
column and the property set named the instance envelope, because a box
that happens to be square is not evidence of a square section. Now the
*type* is the authority: the join is exact, the section travels with it,
and the emitted rectangle is the family's, guarded by a fail-closed
check that the section and the envelope agree to 1e-6 ft. A round or
I-shape family would not pass that check and would keep the envelope
with `ProfileResolved: false`, which is the correct answer until the
symbol *body* — not just its extent — is decoded.

### 2.2 C4 — the 80 columns that remain, and why no section fixes them

| Revit body plan extents (ft) | columns |
|---|---:|
| 2.0000 × 2.0000 | 176 |
| 1.6667 × 1.6667 | 38 |
| 1.7500 × 1.6667 | 20 |
| 1.4167 × 1.6667 | 18 |
| 2.0000 × 1.6667 | 4 |

The record envelope is 2 × 2 ft on **all 256**, and so is the type
section. The 80 columns that miss are ones whose exported body is a
**cut** solid — Revit emits 62 of them as `IfcPolygonalFaceSet` — inset
from the full prism by 3", 4" or 7" on one or both plan axes. There is
no section that produces an inset prism; there is a cut, and recovering
it means recovering what cuts it. Filed as a follow-up rather than
guessed at.

## 3. Walls: an element is framed more than once, and the newest frame wins

A single ElementId can be framed by several partition element records.
`select_instance_records` already knew this and kept the record with the
greatest bounding-box `z` extent (#212, RE-22), breaking ties by the
**first** `(stream, offset)`. On this file **171 of 360 walls** carry two
frames that disagree about the plan box, and the tie-break was picking
the wrong one:

```text
wall 20796   type "Basic Wall:8\" Interior Partition 3 Hour"
  Partitions/46  y 80.7500 … 81.2500   thin extent 0.5000   refs1 type 17341
  Partitions/59  y 80.6667 … 81.3333   thin extent 0.6667   refs1 type 17328
```

Both frames are internally consistent — each carries a thickness that
matches the type it names — so they are two *versions* of the same wall,
and Revit keeps the older copy in place when it rewrites an edited
element into a newer partition stream. Three independent measurements
say the higher-numbered stream holds the current one:

| test | oldest frame | newest frame |
|---|---:|---:|
| thin plan extent equals the nominal thickness of the `IfcWallType` Revit's export assigns | 201 / 360 | **360 / 360** |
| `refs1[1]` equals that `IfcWallType.Tag` | 183 / 360 | **356 / 360** |
| world bounding box equals Revit's | 27 / 360 | 39 / 360 |

The four misses in row two are the `18" Basement` walls, whose slot
names `1851` where the export's `IfcWallType.Tag` is `3897`; that is a
separate id-space question, not a frame question, and the thin extent is
still exactly 1.5 ft on all four.

`Global/ElemTable` corroborates the ordering from a different stream
entirely. The `u32` at `+0x1c` of the 40-byte 2024 record is a monotone
version counter, and it ranks exactly with the newest partition holding
a frame — across all 976 exported instances of the five categories:

| newest partition | `ElemTable u32 @ +0x1c` | instances |
|---|---|---:|
| `Partitions/46` | 19, 20, 24, 25, 27, 30 | 344 |
| `Partitions/51` | 33 | 18 |
| `Partitions/55` | 35 | 12 |
| `Partitions/59` | 36 | 480 |

**No frame of any instance on this file disagrees about the box's `z`**
— checked over all 976 — so changing the tie-break cannot move an
element between storeys. `storey_bound_elements` is 801 before and after.

The tie-break is now the greatest `(stream, offset)`. It changes no
column, window, slab or building-pad box (their frames agree), and it
does move door boxes — see §6.

## 4. Walls: the record box is the untrimmed wall, the joins cut it

With the right frame, the record box and Revit's body differ **only
along the wall's run**. Every one of the 360 walls on this file is
axis-parallel, and the box's thin plan extent is the wall's thickness on
all 360. What is left is the two ends:

```text
wall 20796  record box x 47.7500 … 75.0000     Revit Axis x 47.7500 … 74.6667
wall 20797  record box y 57.6667 … 81.0000     Revit Axis y 57.6667 … 80.6667
wall 20798  record box x 48.0000 … 100.2500    Revit Axis x 48.2500 … 100.2500
wall 20799  record box y 58.0000 … 81.3333     Revit Axis y 58.3333 … 81.3333
```

Read them together and the rule is plain. 20796 runs at `y = 81` and its
high end sits on 20799's **centreline** at `x = 75`; Revit cuts it back
to `x = 74.6667`, which is 20799's near face — half of 20799's 8"
thickness. 20797 ends on 20796's centreline at `y = 81` and is cut back
0.3333 ft for the same reason. 20798's high end at `x = 100.25` is
20806's *far face*, not its centreline, so nothing joins there and Revit
leaves it. 20799's high end at `y = 81.3333` is likewise a face, not a
centreline, and is left.

Over all 720 wall ends, every non-zero delta between the record box and
Revit's `Axis` polyline is exactly one of **0.25, 0.3333, 0.75 ft** —
half of the 6", 8" and 18" wall thicknesses on this file — and zero
everywhere else. Nothing else appears.

### 4.1 The solver

For each end of each wall, find every other recovered wall that is
perpendicular, whose centreline coordinate is exactly this end, whose
own run spans this wall's centreline, and whose elevation range
overlaps. No candidate → no trim. One thickness among the candidates →
trim by half of it. Candidates that disagree about their thickness →
**decline the whole element**, which keeps its record box.

Every input is a recorded bounding box. Nothing is fitted, and nothing
outside the recovered wall set takes part.

### 4.2 Where the model and Revit part company

Scored per end against Revit's own `Axis`:

| our thickness | candidate thicknesses | actual trim | ends | rule says | |
|---:|---|---:|---:|---:|---|
| 0.5000 | — | 0.0000 | 202 | 0.0000 | |
| 0.5000 | 0.5000 | 0.2500 | 158 | 0.2500 | |
| 0.6667 | 0.6667 | 0.3333 | 125 | 0.3333 | |
| 0.5000 | 0.6667 | 0.3333 | 83 | 0.3333 | |
| 0.6667 | — | 0.0000 | 68 | 0.0000 | |
| 0.6667 | 0.6667 | **0.0000** | 30 | 0.3333 | **miss** |
| 0.6667 | 0.5000 | 0.2500 | 22 | 0.2500 | |
| 0.5000 | 0.6667, 0.6667 | 0.3333 | 10 | 0.3333 | |
| 0.6667 | 0.5000, 0.5000 | 0.2500 | 8 | 0.2500 | |
| 0.5000 | 0.5000, 0.5000 | 0.2500 | 5 | 0.2500 | |
| 1.5000 | — | 0.0000 | 4 | 0.0000 | |
| 1.5000 | 1.5000 | 0.7500 | 4 | 0.7500 | |
| 0.6667 | 0.5000 | **0.0000** | 1 | 0.2500 | **miss** |

**689 of 720 ends.** The 31 misses are all **over-trims** — Revit let the
wall run on where the model cuts it — and they sit inside feature
classes that also produce a real trim 125 and 22 times respectively, so
no feature available in the records separates them. The reason is
Revit's own: at a join it decides which of the two walls wraps and which
butts, and that decision is stored per wall pair, not in either wall's
box.

It is not in the record either. Searching the 4 KiB that follows each of
the 638 records framing the 360 walls, for Revit's trimmed `Axis`
endpoints as adjacent `f64` and for the trimmed run length as a single
one: the best fixed offset carries an endpoint pair **29 times** and the
run length **12 times**, against 360 walls. There is no carrier; this is
a model of Revit's join, and the report says so.

### 4.3 Extending the rule was tried and is worse

| variant | bbox-exact | max residual | mean residual |
|---|---:|---:|---:|
| record box, oldest frame (`main`) | 27 / 360 | 0.7500 | 0.2701 |
| record box, newest frame | 39 / 360 | 0.7500 | 0.2657 |
| + trim, skipping end-to-end (corner) candidates | 328 / 360 | 0.3333 | 0.0296 |
| **+ trim at every junction (shipped)** | **336 / 360** | **0.3333** | **0.0220** |

## 5. Measured before / after on `2024_Core_Interior.rvt`

World axis-aligned bounding box, `max(|Δmin|, |Δmax|)` in feet, matched
by `Tag`, IfcOpenShell 0.8.5 geometry iterator with `use-world-coords`.
"exact" is ≤ 1e-3 ft on both corners.

| class | n | exact before | exact after | max before | max after | mean before | mean after |
|---|---:|---:|---:|---:|---:|---:|---:|
| `IfcWall` | 360 | 27 | **336** | 0.7500 | **0.3333** | 0.2701 | **0.0220** |
| `IfcColumn` | 256 | 176 | 176 | 0.5833 | 0.5833 | 0.1217 | 0.1217 |
| `IfcSlab` | 80 | 80 | 80 | 0.0000 | 0.0000 | 0.0000 | 0.0000 |
| `IfcShadingDevice` | 20 | 20 | 20 | 0.0000 | 0.0000 | 0.0000 | 0.0000 |
| `IfcDoor` | 132 | 0 | 0 | 3.0855 | 2.9208 | 2.8938 | 2.9190 |
| `IfcWindow` | 6 | 0 | 0 | 4.8228 | 4.8228 | 4.8228 | 4.8228 |
| `IfcOpeningElement` | 138 | 0 | 0 | 4.9213 | 4.9213 | 3.0604 | 3.0846 |

Per wall: **309 improve, 35 are unchanged, 16 regress.** After the
change the wall residual set is `0.0000` × 336, `0.2500` × 1,
`0.3333` × 23. The 16 regressions are the over-trims of §4.2 on walls
that were accidentally exact untrimmed; they are named in the follow-up
issue rather than hidden by a tolerance.

For comparison with #236's table, the centroid metric it used is still
reported by the harness and still moves the other way on meshed classes:
`IfcWall` max centroid delta stays 2.5336 ft because 105 of the 360
Revit wall bodies are `IfcPolygonalFaceSet` whose vertex mean is not
their box centre. That is the artifact §1 describes, not a residual.

## 6. Doors: the frame changed, and no measurement on this file adjudicates it

96 of 132 doors carry two frames that disagree about the plan box, so
§3's tie-break moves them: 24 doors improve, 72 worsen, 36 are
unchanged, and **0 of 132 are exact either way** in both cases. Nothing
here is evidence in either direction, because the door's record box is
not the thing Revit exports: Revit's `IfcDoor` body is the panel and
frame, and its `IfcOpeningElement` is the wall cut. Scored against the
export's own opening bodies the two frames are just as far apart
(mean 3.0604 vs 3.0846 ft, 0 of 138 exact).

What is unchanged is everything the doors are actually gated on. The
host-wall binding of RE-23 reads the same value from either frame — **no
door or window record has frames that disagree about the preceding
reference**, and both give 138 of 138 correct hosts — so the
`IfcRelFillsElement` pair set is untouched.

## 7. What ships

- `partition_element_records`: the counted list at `+0x88` is kept
  verbatim on the record as `references`, and
  `PartitionElementRecord::type_symbol_reference` reads the family/type
  symbol out of it.
- `partition_schema_mvp::select_instance_records`: the frame tie-break
  becomes the newest `(stream, offset)`.
- `partition_schema_mvp::column_instances_from_records`: the #215 join,
  with the fail-closed section/envelope agreement check.
- `element_record_wall_joins`: the wall run reduction and the join-trim
  solver.
- `partition_schema_mvp::wall_instances_from_records`: applies the trim
  to the emitted plan centre and plan extents, and records the
  thickness, the two trims and the body source on the element.
- `ifc::export_content`: `BodySource` becomes
  `partition_element_record_join_trimmed` on a wall that resolved,
  `ProfileResolved` is true when a type section joined, and the property
  set gains `TypeSymbolElementId`, `TypeSectionWidthFeet`,
  `TypeSectionDepthFeet`, `ThicknessResolved`, `ThicknessFeet`,
  `ThicknessSource`, `JoinTrimStartFeet` and `JoinTrimEndFeet`.
- `ifc::mod::record_base_elevation_feet` accepts the new body source, so
  storey containment is unchanged at 801 of 872.
- `tools/ci/ifc_schema_arity.py`: wall `20800` joins the pinned
  composition probes. It is cut at **both** ends, so its first profile
  point is a trimmed corner — re-emitting the untrimmed box moves it by
  0.1016 m, and running the gate against the pre-change export reports
  exactly that.

## 8. Measured before / after on the emitted file

Entity counts, relations, storeys and the diagnostics sidecar are
**unchanged**; the sidecar is byte-identical.

| | before | after |
|---|---:|---:|
| `IFCWALL` / `IFCDOOR` / `IFCWINDOW` / `IFCCOLUMN` / `IFCSLAB` / `IFCSHADINGDEVICE` | 360 / 132 / 6 / 256 / 80 / 20 | unchanged |
| `IFCBUILDINGSTOREY` | 15 | 15 |
| `storey_bound_elements` | 801 | 801 |
| `IFCRELFILLSELEMENT` | 138 | 138 |
| `IFCPROPERTYSINGLEVALUE` | 5611 | 8435 |
| total instances | 20105 | 22929 |
| entity **types** | 43 | 43 |
| diagnostics sidecar | — | byte-identical |

The 2824 new property rows are exactly the new provenance: 360 walls ×
5 plus 256 columns × 4. Because the OctetProof observation payload
counts every emitted entity type, that single row moves the observation
hash, so the two committed `rvt-rs` observations were regenerated —
`70f8df9d…` → `3c210a6f…`, one line different in each. **The four
bridge-witness observations and both `verdict.json` files are
byte-identical**; both verdicts stay `PASS`, element fixture 6 fields /
10 excluded and full project 13 fields / 3 excluded, 0 diffs, all three
witnesses replaying.

`tools/ci/ifc_schema_arity.py --witness-agreement` passes on the fresh
export: 22929 instances across 43 entity types, 992 swept solids with a
`Position` distinct from their product placement, **3** pinned probes
matched.

## 9. Open

- **The 31 over-trimmed wall ends** (§4.2), 16 walls. Revit's per-pair
  join decision is not in the wall's box and not in 4 KiB past its
  record; finding its carrier is the next question.
- **The 80 cut columns** (§2.2). A section cannot produce an inset
  prism; what cuts them has to be recovered.
- **Doors and windows** still carry the record envelope where Revit
  exports a panel — 0 of 132 and 0 of 6 exact — and the opening bodies
  with them (#227).
- **The `18" Basement` type slot** names `1851` where the export's
  `IfcWallType.Tag` is `3897` on all four walls. The thickness is right;
  the id space is not understood.
- **The remaining `+0x88` slots** (#228). This report attributes one
  more of them — the type symbol on `OST_Columns` and `refs1[1]` on
  `OST_Walls` — and leaves the rest recorded, not decoded.

## 10. Reproduction

```bash
cargo build --profile ci --example probe_re26_element_geometry
./target/ci/examples/probe_re26_element_geometry \
  "$RVT_PROJECT_CORPUS_DIR/2024_Core_Interior.rvt" > records.json
```

The shipped paths are
`rvt::partition_element_records::PartitionElementRecord::type_symbol_reference`,
`rvt::element_record_wall_joins::join_trims`,
`rvt::partition_schema_mvp::column_instances_from_records` and
`rvt::partition_schema_mvp::wall_instances_from_records`; the corpus
gates are
`tests/iter_elements_typed.rs::core_interior_2024_column_type_symbol_join`
and `::core_interior_2024_wall_join_trimmed_bodies`, the unit gates are
`src/element_record_wall_joins.rs::tests` and
`src/partition_element_records.rs::tests`, and the emitted-file gate is
the wall `20800` pin in `tools/ci/ifc_schema_arity.py`.
