# RE-28 — the wall type record: the join is exact on 360 of 360, the compound layers are not there

Status: **positive result on the type join**, exact on 360 of 360;
**measured negative on compound-layer thicknesses**, both in the bytes
and in the only oracle this corpus has. Advances #88 and #34; closes
RE-26 §9's `18" Basement` type-slot question. Date: 2026-09-19.
Artifact: `2024_Core_Interior.rvt`
(sha256 `c805df445d613b408e37337765572021265e3f5dfdc7d1fa53b22ba1600b8014`,
Revit 2024, magnetar-io/revit-test-datasets, MIT).
Reference: `IFC Exports/2024_Core_Interior_slim.ifc`
(sha256 `bfdf36ffb0bb768f3409d818403990e64d4c262c6780603be87f8077387ad86d`),
Revit's own full-project export, read here with the STEP text the file
ships (19879 instances parsed; IfcOpenShell is not required for any
number below).

#88 asks for three things: decode `WallType.CompoundStructure` from
the partition bytes, build one `IfcMaterialLayerSet` per WallType, and
emit `IfcMaterialLayerSetUsage` + `IfcRelAssociatesMaterial` per wall.
This report finds the WallType record, proves the wall → WallType join
exactly, and then reports — as a negative, with numbers — that neither
the bytes near that record nor the reference export carries a layer
thickness. **No layer set is emitted, and none is invented.**

## 1. The wall type record

RE-21 decoded the element record: an 88-byte prologue, the bbox marker
`46 01 ff ff ff ff ab 05` at `+0x50`, a bounding box, then the counted
reference list at `+0x88`. RE-24 decoded the `Level` record: the same
prologue, but the box is absent, so the record carries only
`ff ff ff ff ab 05` at `+0x50` and ends.

A wall *type* has the second shape. Searching every inflated
`Partitions/*` stream for an offset whose leading `u64` is one of the
four `IfcWallType.Tag` values the reference export writes, and whose
`+0x0c` word is the `0x0000059f` every Revit 2024 prologue carries,
finds exactly **four** offsets — one per type, all in `Partitions/46`,
all carrying `BuiltInCategory` `OST_Walls` (−2000011), none carrying a
bounding box:

| `IfcWallType.Tag` | export name | stream offset | `+0x08` flags | `+0x42` |
|---:|---|---:|---:|---|
| 3897 | `Basic Wall:18" Basement` | 5308523 | 0x87 | `0xffff8080` |
| 17328 | `Basic Wall:8" Interior Partition 3 Hour` | 5183320 | 0x7f | `0xffff8080` |
| 17337 | `Basic Wall:6" Interior Partition` | 5184083 | 0x7f | `0xffff8080` |
| 17341 | `Basic Wall:6" Interior Partition 2 Hour` | 5184663 | 0x7f | `0xffff8080` |

`+0x42` is the placement-kind word RE-21 §5 measured at two values.
It takes a **third** one here. RE-21 recorded `0xffffef7f` for a placed
instance and `0xffff8000` for a family/type symbol *envelope* — a
record that still has a box, in family coordinates. `0xffff8080`
belongs to a record with no box at all.

What follows the marker is a counted slot list, framed like the
`+0x88` reference list but starting at `+0x56`:

```text
+0x50  6B   record marker ff ff ff ff ab 05
+0x56  u32  slot count n
+0x5a  n*8  n u64 ElementId slots, ascending, including this id
```

```text
3897   n=3  [3652, 3897, 3898]
17328  n=2  [17328, 17329]
17337  n=2  [17337, 17338]
17341  n=2  [17341, 17342]
```

## 2. The family is a superset of the export's, by construction

Scanning `OST_Walls` for every record of that shape — bbox-less
marker, prologue magic, a declared `ElemTable` id, a slot list holding
the record's own id — finds **eight**:

```text
1710  1711  3897  11356  17328  17337  17339  17341
```

All eight carry `0xffff8080`; no `OST_Walls` record carries that word
with any other shape. The export writes four `IfcWallType` rows, and
they are four of these eight. The other four are wall types the
project defines and never places, which Revit's own exporter does not
write — so the record set is a superset *by construction*, and this
report does not use it as an `IfcWallType` recovery on its own. What
it is used for is §3.

## 3. The join: 360 of 360, and the `18" Basement` question closes

RE-26 §3 read a wall's type from a **position**: `references[1]` of
the newest frame equals the `IfcWallType.Tag` on **356 of 360**. The
four misses are the `18" Basement` walls, whose slot names `1851`
where the export's tag is `3897`; RE-26 §9 left that open as an
"id space [that] is not understood".

It is not an id space. It is a list with two candidates in it and a
positional read that takes the wrong one:

```text
wall 22771  refs1 = [3, 1851, 3897, 20268, 20273, 22771, 22773, 22777]
                        ^^^^  ^^^^
                        not a  a wall type
                        type   record
                        record
```

`1851` is a record of category −2009014 with placement kind
`0xffff8000`; it is not in the eight-id set. `3897` is. Replacing the
positional read with a **set test** — the single slot of the reference
list that is a wall-type record — is exact:

| rule | correct | wrong | none |
|---|---:|---:|---:|
| RE-26 positional `references[1]` | 356 | 4 | 0 |
| **unique type-record slot** | **360** | **0** | **0** |

Every one of the 360 exported wall instance records carries **exactly
one** distinct type-record id in its reference list — never zero,
never two — so the rule needs no tie-break and declines nothing. The
recovered distribution equals the export's own `IfcRelDefinesByType`
distribution exactly:

| type | export walls | recovered |
|---:|---:|---:|
| 17337 `6" Interior Partition` | 129 | 129 |
| 17328 `8" Interior Partition 3 Hour` | 127 | 127 |
| 17341 `6" Interior Partition 2 Hour` | 100 | 100 |
| 3897 `18" Basement` | 4 | 4 |

This is the same shape as RE-26 §2.1's column rule — *the slot that is
a type record of the same category* — with the positional constraint
dropped, because on walls the type slot is not at a fixed index.

## 4. The compound layers are not near the record

The whole point of §1–§3 would be to hang a layer list off the type.
There is no layer list to hang.

Sweeping a **128 KiB span** (±64 KiB) centred on each of the four
exported wall-type records, for every `f64` in `(0, 3]` ft, for every
`f64` equal to one of the file's three nominal wall widths
(0.5, 0.6667, 1.5 ft — the widths RE-26 matched to the record box on
360 of 360 walls), and for every 8-byte-aligned run of two or more
consecutive plausible `f64` summing to one of them:

| type | nominal (ft) | plausible `f64` | `f64` equal to a nominal | runs summing to a nominal |
|---:|---:|---:|---:|---:|
| 3897 | 1.5 | 392 | 5 | **0** |
| 17328 | 0.6667 | 202 | 2 | **0** |
| 17337 | 0.5 | 202 | 2 | **0** |
| 17341 | 0.5 | 204 | 2 | **0** |

Not one run. And the exact-nominal hits are not the type's own width:
every one of the nine is `0.6666666666666666`, tens of kilobytes away
(the nearest is 26301 bytes from record 3897), and the two 6" types
have **no** `0.5` hit anywhere in their span. A wall type's own
thickness is not stored within 64 KiB of its record in any form this
sweep can see, let alone decomposed into layers.

Earlier work looked in the other plausible place and found the same
nothing: `probe_re15_compound_layers` swept ArcWall trailers and
`HostObjAttr` records on the Einhoven corpus for layer runs and H88-1
was falsified there too. Both negatives now stand on record.

## 5. The reference export cannot witness a layer set at all

This is the finding that changes what #88 can be asked to prove on
this corpus.

`2024_Core_Interior_slim.ifc` is a **ReferenceView_V1.2** export
(`FILE_DESCRIPTION ViewDefinition [ReferenceView_V1.2]`,
`ExchangeRequirement [Architecture]`). Counting its material entities:

| entity | count |
|---|---:|
| `IFCMATERIALLAYERSET` | **0** |
| `IFCMATERIALLAYER` | **0** |
| `IFCMATERIALLAYERSETUSAGE` | **0** |
| `IFCMATERIALCONSTITUENTSET` | 150 |
| `IFCMATERIALCONSTITUENT` | 309 |
| `IFCMATERIAL` | 10 |
| `IFCRELASSOCIATESMATERIAL` | 154 |

There is no layer set in the file. #88's acceptance criterion
"layer thicknesses round-trip from Revit-bytes within tolerance
(track per-fixture)" therefore has **no oracle on this artifact**, and
its "at least 5 distinct `IfcMaterialLayerSet`" would be five entities
this file's own author never wrote.

What the 154 associations actually say:

| associated object | relating material | pairs |
|---|---|---:|
| `IFCWALL` | `IFCMATERIAL` | 360 |
| `IFCCOLUMN` | `IFCMATERIAL` | 256 |
| `IFCCOLUMNTYPE` | `IFCMATERIAL` | 175 |
| `IFCDOOR` | `IFCMATERIALCONSTITUENTSET` | 132 |
| `IFCSLAB` | `IFCMATERIAL` | 79 |
| `IFCSHADINGDEVICE` | `IFCMATERIAL` | 20 |
| `IFCDOORTYPE` | `IFCMATERIALCONSTITUENTSET` | 10 |
| `IFCWINDOW` | `IFCMATERIALCONSTITUENTSET` | 6 |
| `IFCSLABTYPE` | `IFCMATERIAL` | 6 |
| `IFCSHADINGDEVICETYPE` | `IFCMATERIAL` | 2 |
| `IFCWINDOWTYPE` | `IFCMATERIALCONSTITUENTSET` | 1 |
| `IFCSLAB` / `IFCSLABTYPE` | `IFCMATERIALCONSTITUENTSET` | 1 each |

**All 360 walls resolve to a single `IfcMaterial`** — `Default Wall`
on 356, `Concrete` on the four `18" Basement` walls — never to a
constituent set, so the export does not even say how many layers a
wall type has. A three-layer wall whose layers share one material
would land in exactly the same row.

The only compound-layer widths anywhere in the file belong to one slab
type, and they are **dangling**: nothing references them.

```text
#3564=IFCSLABTYPE(…,'Floor:Basement Slab',…,'3634',$,.FLOOR.);
#3566=IFCMATERIALCONSTITUENT('Polyvinyl Chloride, Rigid',$,#3565,$,'Materials');
#3567=IFCQUANTITYLENGTH('Width',$,$,0.33333333333333331,$);
#3568=IFCMATERIALCONSTITUENT('Concrete',$,#1604,$,'Materials');
#3569=IFCQUANTITYLENGTH('Width',$,$,1.1666666666666667,$);
#3570=IFCMATERIALCONSTITUENTSET('Floor:Basement Slab',$,(#3566,#3568));
#3571=IFCPHYSICALCOMPLEXQUANTITY('Polyvinyl Chloride, Rigid',$,(#3567),'Layer',$,$);
#3572=IFCPHYSICALCOMPLEXQUANTITY('Concrete',$,(#3569),'Layer',$,$);
#3573=IFCQUANTITYLENGTH('Width',$,$,1.5,$);
```

`#3570` is used (by `#19579`, on the slab and its type); `#3571`,
`#3572` and `#3573` are referenced by nothing in the file. 4" + 14" =
18", which is the same decomposition the count manifest already
records for slab 22756. One slab type, no wall type, and no consumer.

**Consequence for #88.** Its layer-thickness criterion cannot be met
*or refuted* against this corpus, because the reference side does not
carry the quantity. Meeting it needs either a Revit export in a view
definition that writes layer sets (a DesignTransferView / full
`Qto_WallBaseQuantities` export of the same `.rvt`), or an
owner-supplied wall-type schedule. Until one exists, the manifest row
stays `known_gap` with
`unsupported_feature: revit_compound_assemblies_and_walltype_widths`,
and nothing emits a layer.

## 6. Materials: a record family, not yet a recovery

The 102 `IfcMaterial` rvt-rs emits are `partition_name_candidates`
string heuristics against the export's 10 — an over-count the manifest
already records. There is a real material record family underneath.

`BuiltInCategory.OST_Materials` is −2000700 (Autodesk's published
enumeration). Scanning for it with the §1 shape finds **86** records
on this file, every one naming its own ElementId in its slot list,
every one carrying placement kind `0xffff8020`. 86 is a closed,
byte-derived set where 102 is a string filter — but it is **not** a
recovery of the export's 10, for two reasons, and both are reported
rather than worked around:

1. **86 is still not 10.** A Revit project carries every material its
   template defines; Revit's exporter writes only the ones an exported
   element uses. Getting from 86 to 10 needs the element → material
   join, and §6.1 shows that join is incomplete.
2. **The records carry no name.** The RE-24 name block — owner `u64`,
   a 56-byte `0xff` run, a `u32` length, UTF-16LE — resolves 0 of the
   86. Relaxing RE-24's elevation-marker requirement so the shape can
   match other categories produces 165 blocks on this file of which
   most are garbage (`'ڱ'`, `'￿￿ק￿'`), so the relaxed
   shape is rejected: the elevation marker was doing the fail-closed
   work. Measured negative.

What a material name block actually looks like is recorded, not
decoded. Locating the export's own material names as UTF-16LE in the
streams, the owning `OST_Materials` ElementId appears **twice** before
the string, 58 to 60 bytes apart, at a distance from the string that
is *not* fixed:

| name | owner id | owner copies at |
|---|---:|---|
| `Under Slab Fill` | 3763 | −251, −193 |
| `Door - Panel` | 17288 | −251, −193 |
| `SH_Aluminum, Anodized Black` | 17230 | −299, −241 |
| `Wood_Walnut black` | 17229 | −371, −311 |

The same shape carries the wall *type* name: `17328` appears at −366
and −306 before `Interior Partition 3 Hour`. Between the second owner
copy and the `u32` length prefix sits a run of 12-byte entries whose
middle word is a large negative `i32` (`0xfff0b90d`, `0xfff0b90b`,
`0xffee993a`, `0xfff09643`…), i.e. a parameter table, with the name as
the value of one fixed parameter. Decoding that table fail-closed is
the next question and is filed, not guessed.

### 6.1 The one material join the bytes do make

Of the four exported wall types, exactly one carries a material slot:
`3897`'s slot list leads with `3652`, and `3652` is one of the 86
`OST_Materials` records. The four walls of type `3897` are exactly the
four walls Revit's export associates with `Concrete`, and no other
wall on the file gets `Concrete`. That is a one-type agreement, on
four walls, with the material's *name* still unrecovered — so it is
recorded as corroboration for the slot's meaning, not shipped as a
material recovery. The other three wall types carry no material slot
at all while their walls all export as `Default Wall`, so there is no
rule here yet.

## 7. What ships

Library only. **No emitted IFC entity, property or relation changes**,
`tests/fixtures/synthetic-project.ifc` is byte-identical, and every
committed OctetProof observation and verdict is untouched.

- `rvt::partition_type_records` — the §1 record shape, fail-closed at
  every step (undeclared id, wrong prologue magic, non-zero `+0x10`,
  category outside the published band, the element-record marker, a
  zero / absurd / overrunning slot count, and a slot list that does
  not name the record's own id all reject), plus
  `type_definition_ids` and `unique_type_reference` — the §3 rule,
  which returns `None` rather than guessing when a list names two
  different types.
- `examples/probe_re28_walltype_layers.rs` — the probe behind every
  number above, in three modes (`ids`, `category`, `string`), with the
  §4 layer sweep built in so the negative is reproducible.
- The `materials` row of both magnetar count manifests records the 86
  and the `known_gap` reason in the terms of §5 and §6.

## 8. Open

- **Compound-layer thicknesses (#88).** Not near the type record
  (§4), and not witnessable by this corpus's reference export (§5).
  Blocked on either a layer-set-carrying export of the same `.rvt` or
  a wall-type schedule; do not emit a layer set before one exists.
- **Material names (#34, #86).** The block shape is located (§6) and
  the parameter table it sits in is not decoded. That single decode
  would turn 86 records into named materials and would let the §6.1
  join be scored.
- **The remaining slot-list slots.** `17328`'s `17329`, `3897`'s
  `3898`, and `11344` on two of the four unexported types are
  recorded, not attributed. `17329` is a record of category −2000576
  in a chain of tiny bbox-less records; what the chain is, is not
  claimed.
- **Wiring §3 into the export.** The join is proven and available; it
  is not yet a property on the emitted wall, because adding one moves
  the `IFCPROPERTYSINGLEVALUE` count and therefore both committed
  witness observation hashes. That is a deliberate separation, not an
  oversight.

## 9. Reproduction

```bash
cargo build --profile ci --example probe_re28_walltype_layers

# §1, §3, §4 — the four exported wall types and the layer sweep
./target/ci/examples/probe_re28_walltype_layers \
  "$RVT_PROJECT_CORPUS_DIR/2024_Core_Interior.rvt" ids 17328,17337,17341,3897

# §2 — the eight-record wall type family
./target/ci/examples/probe_re28_walltype_layers \
  "$RVT_PROJECT_CORPUS_DIR/2024_Core_Interior.rvt" category -2000011

# §6 — the 86 OST_Materials records
./target/ci/examples/probe_re28_walltype_layers \
  "$RVT_PROJECT_CORPUS_DIR/2024_Core_Interior.rvt" category -2000700

# §6 — a material name block in context
./target/ci/examples/probe_re28_walltype_layers \
  "$RVT_PROJECT_CORPUS_DIR/2024_Core_Interior.rvt" string "Under Slab Fill"
```

The shipped path is `rvt::partition_type_records`; the unit gates are
its own `tests` module, and the corpus gates are
`tests/iter_elements_typed.rs::core_interior_2024_wall_type_record_join`
and `::core_interior_2024_material_record_family`.
