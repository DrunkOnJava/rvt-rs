# RE-29 — the reference list names the joins: walls, and the walls that cut columns

Status: **positive result on columns**, exact on 256 of 256;
**positive result on walls**, exact on 351 of 360, with a narrowed
residual of 9 ends; **#240's id-space hypothesis rejected**, and the
question itself answered next door by RE-28 (#273) while this was in
flight — §6 is a pointer and a correction. Closes #239. Refs #238,
#240, #228. Date: 2026-09-19.
Artifact: `2024_Core_Interior.rvt`
(sha256 `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014`,
Revit 2024, magnetar-io/revit-test-datasets, MIT).
Reference: `IFC Exports/2024_Core_Interior_slim.ifc`
(sha256 `bfdf36ffb0bb768f3409d818403990e64d4c262c6780603be87f8077387ad86d`),
Revit's own full-project export.

RE-26 left three questions on the table and named a suspect for none
of them: 31 over-trimmed wall ends on 24 walls whose join decision
"has no identified carrier"; 80 columns whose exported bodies are cut
by "the cutting element/relation … not the profile"; and a wall type
slot reading `1851` where the export writes `3897`. All three turn out
to be about the same bytes — the counted reference list at `+0x88`
that RE-23 read for a door's host wall and RE-27 read for an element's
Level. RE-28 reached the third one first, from the type record's side;
§6 records what this investigation adds to it and what it got wrong.

**The list names what the element is joined to.** Besides the type and
the Levels, a wall record holds the ElementIds of the walls it joins,
and a column record holds the ElementIds of the walls that cut it.

## 0. The reference side, and why it is readable here

IfcOpenShell is not installable in this checkout, so the reference
geometry is read by `tools/re/ifc_world_aabb.py` — a dependency-free
evaluator for the entity set Revit's IFC4 exporter and rvt-rs's STEP
writer actually emit (`IfcLocalPlacement` chains, `IfcMappedItem`,
`IfcExtrudedAreaSolid` over the indexed-polycurve profiles,
`IfcPolygonalFaceSet`), which raises rather than shrink a box it
cannot evaluate.

It is not trusted on its own word. Pointed at the same reference
export, it reproduces RE-26's published tables exactly:

| RE-26 table | RE-26 (IfcOpenShell 0.8.5) | this tool |
|---|---|---|
| §2.2 column body plan extents | 176 / 38 / 20 / 18 / 4 | **176 / 38 / 20 / 18 / 4** |
| §4.3 walls exact, untrimmed record box | 39 / 360 | **39 / 360** |
| §4.3 walls exact, shipped trim | 336 / 360 | **336 / 360** |
| §4.2 join-trim ends correct | 689 / 720 | **689 / 720** |
| §5 column max / mean residual | 0.5833 / 0.1217 ft | **0.5833 / 0.1217 ft** |

Same numbers, different reader. Every measurement below is the world
axis-aligned bounding box, `max(|Δmin|, |Δmax|)` over the three axes,
in feet, matched by `Tag`; "exact" is ≤ 1e-3 ft on both corners.

## 1. The carrier

Three wall records of the same storey, side by side:

```text
20868  refs1 = [3, 17328, 20307, 20308, 20805, 20806, 20868]
                   ^^^^^  ^^^^^^^^^^^^  ^^^^^^^^^^^^  ^^^^^
                   type      Levels      joined walls  self

20826  refs1 = [3, 17328, 20307, 20308, 20826]
                                         ^^^^^  and nothing else

20800  refs1 = [3, 17328, 20307, 20308, 20382, 20383, 20800, 20801,
                                        ^^^^^^^^^^^^  columns it cuts
                20803, 20804, 20805, 20806, 20808, 20813, 20814,
                20816, 20825, 20830]
```

The list is **ascending**, so slot order carries no meaning beyond the
ids themselves; what it carries is membership. 20868 runs in `x` at
`y = 69.1667` from `x = 85.5` to `100.0` and Revit cuts both of its
ends — by half of 20805 at one end and half of 20806 at the other,
which are exactly the two walls it names. 20826 runs in `x` at
`y = 67.75` from `x = 119.5` to `129.0`, each end landing precisely on
the centreline of an 8" wall that passes through it, and Revit cuts
**neither**. The only thing in the file that distinguishes the two
cases is that 20826's list is empty of walls.

## 2. Walls: requiring the join to be named (#238)

Adding one predicate to RE-26 §4.1's solver — *the candidate must be
named in this record's reference list* — and changing nothing else:

| rule | ends correct | over-trims |
|---|---:|---:|
| RE-26 §4.1, geometry only | 689 / 720 | 31 |
| **+ candidate named in `refs1`** | **703 / 720** | **17** |

Fourteen over-trims removed, **none of the 689 made wrong**. The
removed set is exactly the seven walls that name no other wall at all,
both ends each. Sixteen of the 360 walls name none, and every one of
them is a wall Revit leaves at full length:

```text
20826  20897  25942  55840  59344  59415  60225  60296
61106  61177  61987  62058  62868  62939  63749  80746
```

The relation is symmetric on 466 of the 467 wall pairs it forms — only
`(61968, 62866)` is named in one direction and not the other, and no
end of either wall depends on it.

## 3. Walls: a cut wall stops cutting (#238)

The 17 that remain are all at `x ≈ 137`, and they say what is missing:

```text
20800  runs in x at y = 81, recorded 85.5 … 137.25
       Revit's body            85.8333 … 136.9167   (cut by 20816)

20803  runs in y at x = 137.1667, recorded 81.0 … 87.5
       its low end is on 20800's centreline, and the rule cuts it
       Revit's body            81.0    … 87.25      (not cut there)
```

20800's own high end is cut back to `136.9167` by 20816, and
`137.1667` is past that. **The wall that would do the cutting is no
longer there to do it.** A candidate is therefore tested for reach
against its *own* cut-back run, not its recorded one.

One qualification makes that well-founded rather than circular. Where
the cut a candidate took was imposed by a wall on the very plan line
being resolved, it must not disqualify the candidate — otherwise a
collinear stack disarms itself:

```text
20798  runs in x at y = 58, recorded 48.0 … 100.25, cut to 48.25
       by the x = 48 wall line (20797 and 20817, both 6")
20817  runs in y at x = 48, its high end on 20798's centreline
       Revit's body  51.5 … 57.6667 — 20798 still cuts it
```

So the reach test reduces a candidate by the joins it makes with walls
**not** on the line being resolved. With both predicates:

| rule | ends correct | over-trims |
|---|---:|---:|
| RE-26 §4.1, geometry only | 689 / 720 | 31 |
| + named in `refs1` | 703 / 720 | 17 |
| **+ reach against the cut-back run** | **711 / 720** | **9** |

## 4. What is left: one side of a true L corner

The 9 residual ends are two distinct corners, one repeated on eight
storeys. Both are corners where **neither** wall's line continues
past the meeting point, and Revit cuts exactly one of the pair:

| corner | walls | Revit cuts | Revit leaves |
|---|---|---|---|
| `(137.25, 81)` × 8 storeys | 20800 `x` 8" len 51.75, 20816 `y` 8" len 23.0 | 20800 | 20816 |
| `(102.25, 81)` | 80743 `x` 8" len 54.5, 80747 `y` 6" len 23.33 | 80747 | 80743 |

The file offers no feature that separates them. The survivor is the
thicker wall once and an equal-thickness wall once; the `x` wall once
and the `y` wall once; the higher ElementId once (`20816 > 20800`) and
the lower once (`80743 < 80747`); the shorter run once and the longer
once. Both pairs name each other symmetrically, both carry the same
`+0x42` placement kind, and their prologue flag words
(`0x151`/`0x111`, `0x111`/`0x109`) do not order the two sides the same
way. 17 mutual L corners exist on this file; the **other 8** are not
true corners at all — a collinear wall continues past the meeting
point — and Revit cuts both sides there, which the solver already gets
right.

So the per-pair butt/mitre choice at a true dead-end corner still has
no identified carrier. It is now one named feature class of 9 ends on
9 walls instead of 31 ends on 24 walls, and the solver keeps cutting
both sides rather than guessing which survives — over-trimming one
wall per corner, recorded here and pinned by
`element_record_wall_joins::tests::the_surviving_side_of_a_true_l_corner_is_still_over_trimmed`.

## 5. Columns: the walls that cut them (#239)

RE-26 §2.2 measured that 80 of the 256 columns have bodies inset from
their 2 ft prism by 3", 4" or 7" on one or both plan axes, and that no
section can produce an inset prism. The column record names the walls:

```text
22807  refs1 = [3, 4641, 4870, 5755, 20268, 20274, 22807, 75408, 80743]
                                ^^^^ type symbol (RE-26 §2.1)   ^^^^^
                                                        the wall that cuts it
```

The rule is a straight subtraction of recorded boxes:

```text
exported column body = record prism − ⋃ (record prism of each named wall)
```

| | exact | max | mean |
|---|---:|---:|---:|
| record prism (RE-26) | 176 / 256 | 0.5833 | 0.1217 |
| **prism − named walls** | **256 / 256** | **9.4e-13** | **0.0000** |

Zero false positives, zero misses, at a residual twelve orders of
magnitude below the tolerance. The cutters are the walls' **untrimmed**
record boxes: substituting the join-trimmed runs of §3 scores
190 / 256, so a wall runs its full recorded length into a column it
cuts, and the wall/wall cleanup of §2 does not apply at a wall/column
join.

The 256 split three ways, and the split is what makes the rule safe to
emit as a box:

| group | columns | difference | emitted |
|---|---:|---|---|
| names no wall that overlaps the prism | 107 | — | the record prism |
| named walls cut it, but interior | 69 | a slot / L / cross, not a box | the record prism, whose box already equals Revit's |
| named walls cut it from a face | **80** | **a rectangular box on all 80** | the reduced box |

No column on this file has more than two overlapping cutters; 76 of
the 80 are cut by two walls and 4 by one, and the 69 declines split
65 / 4 the same way.

`element_record_column_cuts::difference_box` requires the surviving
material to fill its own bounding box exactly, by volume, and declines
otherwise. On this file that test passes on every column whose
bounding box shrinks and fails on every column whose bounding box does
not — so declining costs nothing and the emitted solid is never
larger, on any axis, than what Revit kept. Column 20376 is the
canonical decline: a 6" wall passes through the middle of the prism
for 8 of its 14.33 ft, and its bounding box is the full 2 × 2 ft
either way.

The cut plan rectangles reproduce Revit's own body-extent histogram
exactly — 176 × (2.0 × 2.0), 38 × (1.6667 × 1.6667),
20 × (1.75 × 1.6667), 18 × (1.4167 × 1.6667), 4 × (2.0 × 1.6667) ft —
which is the corpus gate
`tests/iter_elements_typed.rs::core_interior_2024_column_join_cut_plan_extents`.

The type section joined in RE-26 §2.1 stays on the element as the
type's own fact (`TypeSectionWidthFeet` / `TypeSectionDepthFeet`), but
it is no longer the shape of the solid on a cut column: the emitted
rectangle is the remainder, and `ProfileSource` says so.

## 6. The 18" Basement type slot: answered next door (#240)

This section is a pointer and a correction. #240 was answered while
this work was in flight, by RE-28 (`RE-28-wall-type-records.md`, #273),
and answered better than the reading below: a wall **type** has a
partition record of its own, in the bbox-less shape RE-24 found for
Levels, with a third placement-kind value `0xffff8080` at `+0x42`.
Eight `OST_Walls` records carry it on this file, and every one of the
360 exported wall instances names exactly one of them in the same
`+0x88` list this report is about. That is a set test, it needs no
tie-break, and it reads the correct `IfcWallType.Tag` on **360 of
360**.

What survives from the investigation here is the diagnosis of *why*
the old reading missed, which RE-28's §"What was proven" also states:

```text
22771  refs1 = [3, 1851, 3897, 20268, 20273, 22771, 22773, 22777]
                   ^^^^  ^^^^
                    |     the IfcWallType.Tag Revit's export writes
                    a lower id that displaces the type from slot 1
```

The list is ascending, so RE-26 §3's `refs1[1]` is an **index
artifact**: on every other wall the type happens to be the lowest slot
after the leading `3`, and on these four a smaller id gets there
first. #240's own framing — "two id spaces for the same type … or a
type-graph indirection `1851 -> 3897`" — is **rejected by
measurement**: both ids are in the same list of the same record, so
there is nothing to follow.

**A correction to a claim this report made before the merge.** An
earlier draft said a brute scan of all 23 470 decodable records finds
no record for `1851`, `3897`, `17328`, `17337` or `17341`, and
concluded that wall types are not framed as records at all. That scan
(`probe_element_record_owner_lookup`) only accepts the **element**
record shape, which requires the bbox marker at `+0x50`; the type
records are bbox-less and it cannot see them. RE-28 finds all of them.
`1851` likewise does have a record — of category `-2009014`, which is
what disqualifies it as a wall type.

Two further measurements from this side are consistent with that and
are recorded rather than used: `1851` is named by 8 `OST_Walls`
records (the four exported basement walls and their four
container-member twins, RE-21 §4) and by 2 `OST_Floors` records, one
of them the exported slab `22756`, where it sits immediately before
that element's own type id `3634`; and its three `Global/ElemTable`
version words are `(0, 0, 0)`, the same as the constant slots `3` and
`113`, while `3897` carries `(5, 5, 5)` and the other three wall types
`(19, 19, 19)`. The string `'1851'` occurs nowhere in Revit's own
export.

## 7. Measured before / after on `2024_Core_Interior.rvt`

World axis-aligned bounding box against Revit's own export, matched by
`Tag`, read from both emitted files with `tools/re/ifc_world_aabb.py`.

| class | n | exact before | exact after | max before | max after | mean before | mean after |
|---|---:|---:|---:|---:|---:|---:|---:|
| `IfcWall` | 360 | 336 | **351** | 0.3333 | 0.3333 | 0.0220 | **0.0081** |
| `IfcColumn` | 256 | 176 | **256** | 0.5833 | **0.0000** | 0.1217 | **0.0000** |
| `IfcSlab` | 80 | 80 | 80 | 0.0000 | 0.0000 | 0.0000 | 0.0000 |
| `IfcShadingDevice` | 20 | 20 | 20 | 0.0000 | 0.0000 | 0.0000 | 0.0000 |

Per element: **15 walls improve, 345 unchanged, 0 regress**;
**80 columns improve, 176 unchanged, 0 regress.** The wall residual
set after the change is `0.0000` × 351, `0.2500` × 1, `0.3333` × 8 —
the §4 corners. Walls cut at one end or both go 329 → 322, the seven
that drop out being the ones that name no join partner.

## 8. Measured before / after on the emitted file

Entity counts, relations, storeys and the diagnostics sidecar are
**unchanged**; the sidecar is byte-identical.

| | before | after |
|---|---:|---:|
| `IFCWALL` / `IFCDOOR` / `IFCWINDOW` / `IFCCOLUMN` / `IFCSLAB` / `IFCSHADINGDEVICE` | 360 / 132 / 6 / 256 / 80 / 20 | unchanged |
| `IFCBUILDINGSTOREY` | 15 | 15 |
| `IFCRELCONTAINEDINSPATIALSTRUCTURE` | 14 | 14 |
| `IFCRELFILLSELEMENT` | 138 | 138 |
| `IFCPROPERTYSINGLEVALUE` | 9231 | 9311 |
| entity **types** | 43 | 43 |
| total instances | 23726 | 23806 |
| diagnostics sidecar | — | byte-identical |

The 80 new property rows are exactly the new provenance —
`JoinCutWallCount`, one per cut column. That single count moves the
OctetProof observation payload hash, so the two committed `rvt-rs`
observations were regenerated, one line different in each:
`d7821c07…` → `e463f885…`. **The four bridge-witness observations and
both `verdict.json` files are byte-identical**; both verdicts stay
`PASS`, element fixture 6 fields / 10 excluded and full project 13
fields / 3 excluded, 0 diffs, all three witnesses replaying.

All three pinned placement probes of `tools/ci/ifc_schema_arity.py`
are unchanged to 0 ft — `IfcColumn 20375` (a column whose cut is
interior, so it keeps its prism), `IfcSlab 20345`, and
`IfcWall 20800` (whose two trims both survive the new predicates).

## 9. What ships

- `element_record_wall_joins::joined_walls` — the other recovered
  walls a record names at `+0x88`, filtered against the recovered wall
  id set so an unresolved slot joins nothing.
- `element_record_wall_joins::join_trims` — the same solver with the
  membership predicate and the reach test of §3.
- `element_record_column_cuts` (new) — `difference_box` and
  `column_cut_boxes`, the fail-closed prism subtraction of §5.
- `partition_schema_mvp::column_and_wall_records` — one partition
  sweep for both categories, because the column body now needs the
  wall boxes.
- `partition_schema_mvp::column_instances_from_records` takes the wall
  records and applies the cut to the emitted placement and extents.
- `ifc::export_content`: `BodySource` becomes
  `partition_element_record_join_cut` on a cut column, `ProfileSource`
  with it, and the property set gains `JoinCutWallCount`.
- `ifc::mod::record_base_elevation_feet` accepts the new body source,
  so storey containment is unchanged at 853 of 872.

## 10. Open

- **The true-L-corner survivor** (§4), 9 ends on 9 walls. Two distinct
  corners; no feature in the record orders the two sides.
- **What `1851` is** (§6). Not a wall type — RE-28 identifies its
  record as category `-2009014` — but what that category *is*, and
  why the four basement walls and two floors all name it, is not
  claimed.
- **Doors and windows** still carry the record envelope where Revit
  exports a panel (#227), unchanged here.
- **The remaining `+0x88` slots** (#228). This report attributes two
  more classes — the joined walls on `OST_Walls` and the cutting walls
  on `OST_Columns` — and leaves the rest recorded, not decoded.

## 11. Reproduction

```bash
cargo build --profile ci --example probe_re29_join_carriers
./target/ci/examples/probe_re29_join_carriers \
  "$RVT_PROJECT_CORPUS_DIR/2024_Core_Interior.rvt" > joins.json

# reference side, no IfcOpenShell required
python3 tools/re/ifc_world_aabb.py \
  "$RVT_PROJECT_CORPUS_DIR/../IFC Exports/2024_Core_Interior_slim.ifc" \
  --type IFCCOLUMN
# and the emitted side, metres to feet
python3 tools/re/ifc_world_aabb.py out.ifc --type IFCCOLUMN \
  --scale 3.280839895013123
```

The corpus gates are
`tests/iter_elements_typed.rs::core_interior_2024_column_join_cut_plan_extents`,
`::core_interior_2024_wall_joins_are_named_in_the_reference_list`,
`::core_interior_2024_column_type_symbol_join` and
`::core_interior_2024_wall_join_trimmed_bodies`; §6 adds no gate of
its own because
`::core_interior_2024_wall_type_record_join` (RE-28) already pins the
four basement walls; the unit gates are
`src/element_record_wall_joins.rs::tests` and
`src/element_record_column_cuts.rs::tests`.
