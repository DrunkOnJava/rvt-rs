# Project Status

Last reviewed: 2026-09-19

This page is the public source of truth for what rvt-rs can do today. It is
intentionally blunt so users can decide quickly whether the tool fits their
workflow.

The current support boundary is tracked in the
[supported MVP input profile](supported-profile.md). The machine-readable
[executable support matrix](support-matrix.json) (schema:
[`support-matrix.schema.json`](schemas/support-matrix.schema.json)) is the
checked-in capability ceiling for audit controls COR-001 / TEST-001 /
DOC-001 — statuses there must stay honest and must not claim
converter-grade typed recovery. Keep that matrix, this page, the README,
and the viewer support matrix aligned.

**Verification protocol:** what "verified" means here is defined in
[`docs/verification-protocol.md`](verification-protocol.md) — cross-witness
agreement across recorded format edges, tracked in
[`research/witness-registry.json`](../research/witness-registry.json) and
enforced by `tests/witness_registry.rs` plus the CI gates it names. Two
RVT → IFC edges via Revit's exporter are recorded and gated today — the 20 KB
element fixture and the full 19879-entity project export — each witnessed by
three independent implementation lineages: rvt-rs on the `.rvt`, IfcOpenShell
and IFClite on the `.ifc`. The full-project verdict's claimed surface is ten
fields wide since #212 (`IFCWALL`, `IFCDOOR`, `IFCWINDOW`, `IFCCOLUMN`,
`IFCSLAB`, `IFCSHADINGDEVICE`, `IFCROOF`, `IFCBEAM`, `IFCFLOWTERMINAL`,
`IFCUNITASSIGNMENT`). The RVT → DWG
edge with dwg-rs is still pending a Revit session.

**OctetProof instance:** the citable protocol specification is
[`docs/octetproof-spec.md`](octetproof-spec.md) (1.0.2, 2026-08-30, CC-BY-4.0);
the draft it supersedes is kept verbatim at `docs/octetproof-spec-draft.md`,
and §19 of the spec lists every correction. Its observation / verdict / replay
shapes are implemented in-repo — `rvt-ifc --observation`,
`tools/ci/witness-verdict.py`, committed artifacts under `research/witness/` —
gated in CI, and published as JSON Schemas
([`witness-observation`](schemas/witness-observation.schema.json),
[`witness-verdict`](schemas/witness-verdict.schema.json)); see the "OctetProof
alignment" section of `docs/verification-protocol.md`. The umbrella repository
is [DrunkOnJava/octetproof](https://github.com/DrunkOnJava/octetproof); the
second (RVT → DWG) edge is still pending a Revit export session.

**Research program pointer:** the governing
[unified research report](research/unified-research-report.md) (rev 1.1)
sequences Phase 0 product readiness → Phase 1 identity/evidence contracts →
Phase 2 ES ElementId remapping oracle (H-ES5). Phase 2 fixture generation
requires a Revit-hosted API oracle and is **not** available on Cloud VMs.
ES remapping is **not** a shipped capability; default IFC omits ES edges.
Coordination mirror: [Discussion #112 notes](disc-112-coordination.md).

## User-Facing Summary

rvt-rs is useful today for inspecting Revit files without Revit, extracting
metadata, reading the embedded schema, auditing stream contents, running a
zero-upload browser viewer, and producing valid IFC/glTF/SVG outputs from the
parts of the model that are actually decoded.

**Generic real-project typed model extraction is not solved yet.** rvt-rs is
not a production RVT-to-IFC converter for arbitrary architectural projects.
Production `walker::iter_elements` prefers typed MVP decoders on
`Global/Latest` (fail closed), merges version-gated 2023 ArcWall partition
recovers, and additionally merges fail-closed partition MVP recovers for
`Level` (elevation + name), `Material` (display-name candidates), and — on
Revit 2024 —
`ArcWallRectOpening` index rows with ElemTable-confirmed related-id
provenance (still not typed `Door`/`Window`) plus `Wall`, `Door`,
`Window`, `Column`, `Floor` and `BuildingPad` instances from partition
element records.
**Walls, doors, windows, columns and slabs now match Revit's own
exporter exactly on a real project**: the Revit 2024 partition element-record
header carries the element's `ElementId` and its `BuiltInCategory`,
followed by a container reference at `+0x32`, a placement-kind word at
`+0x42`, a fixed marker and the element's model bounding box. A record
that is declared in `Global/ElemTable`, carries **no container
reference** and is marked a **placed instance** (not a family/type
symbol envelope) is exactly what Revit's exporter emits: 360 of 360
`IfcWall`, 132 of 132 `IfcDoor`, 6 of 6 `IfcWindow` and 256 of 256
`IfcColumn` in the full Core Interior export — matching **ElementId
sets**, not just counts, with no false positives and no misses, gated
at tolerance 0 (#211, RE-21). The rule is a direct byte test; it
replaced the #204 column heuristic (family-local bbox proxy plus
highest-ElementId-per-footprint collapse) and reproduces the same 256
columns, which also settles #216 — the 136 omitted column ids are 17
type symbols plus 119 members of five container elements (nine such
containers exist on the file across all four categories). RE-19 is
untouched: this is a different carrier (the record's own
`BuiltInCategory`), not an opening-index discriminator and not a
schema-field wall, and both of those stay unsupported.
**`OST_Floors` follows the rule too** — RE-22 (#212) found its 99
selections are *all* exported, 79 as `IfcSlab` and 20 as
`IfcShadingDevice`, so there were never any false positives, and the
80th exported slab is a building pad (`Pad:Site Pad`, ElementId 21975)
carrying `OST_BuildingPad` (−2001263), not `OST_Floors`. The
`IfcSlab` / `IfcShadingDevice` split is a **per-instance** Revit
`IFC Export As` override — the export carries `IFCSLABTYPE` and
`IFCSHADINGDEVICETYPE` rows with the same `Tag` (4166, 71848), so the
same `FloorType` lands on both sides — and it is readable from the
bytes: the UTF-16LE value string sits in the element's parameter block
with the owning ElementId as a `u64` 220 bytes ahead of it and again
at 286, both required to agree and to be declared in
`Global/ElemTable`. Thirty-one entries pass that test on this file,
naming 21 ids, of which the 20 that are also placed instances are
exactly the export's `IFCSHADINGDEVICE` `Tag` set; only
`IfcShadingDevice` is honoured as an override target, an unrecognised
value leaves the element on its class mapping. Composed, the two rules
give **80 of 80 `IfcSlab` and 20 of 20 `IfcShadingDevice`, exact
ElementId sets at tolerance 0**, verified with IfcOpenShell 0.8.5 and
IFClite 7.1.1. The record's bounding-box `z` extent is the slab's real
extrusion thickness — it equals the export's
`IfcExtrudedAreaSolid.Depth` on 79 of 80 slabs and sums to it on the
80th (`Floor:Basement Slab`, exported as two stacked solids of
0.3333 ft and 1.1667 ft) — which closes
`floor_slab_extrusion_thickness` for record-backed slabs (#31); the
plan-loop boundary annotations that preceded them are gone (they matched
no slab in Revit's own exports), so exactly one `IFCSLAB` is emitted per
exported id.
**Every one of the 80 slabs also carries its real plan profile** (#31,
RE-25): each boundary line of a Revit sketch is its own partition
element record — `BuiltInCategory` `OST_SketchLines` (−2000045), the
same 88-byte prologue, its own bounding box — and the last slot of the
*second* counted reference list at `+0x88` names the sketched element.
On Core Interior 3688 such records name 126 owners, among them all 80
exported `IFCSLAB` and all 20 exported `IFCSHADINGDEVICE` ids; the join
is by ElementId alone, with no geometry in it. Segments are chained by
closure rather than fitted: a box degenerate on one axis contributes its
endpoints, every other box is placed only when exactly one pair of
still-open vertices fits it, and anything ambiguous rejects the whole
element. Measured against the reference export's swept areas, **80 of 80
slab profiles are exact** — 122 loops (80 outer plus 42 rectangular
voids), worst vertex deviation 1.563e-12 ft, tolerance 1e-3 ft — so 42
perimeter plates emit `IfcArbitraryProfileDefWithVoids` (a 26-vertex
outer ring around one rectangular courtyard void) and 38 emit
`IfcArbitraryClosedProfileDef`. The 20 shading devices are rotated
plates whose sketch-line boxes are axis-aligned envelopes of diagonal
segments; the closure declines them, and they keep the record box
rectangle with `ProfileResolved: false`. The pre-RE-25 plan-loop scan is
a measured dead end for this: 2317 closed plan-polyline candidates
across all eight inflated partitions, none with the plan bounds of any
recovered plate, and no ordered vertex run of the export's polygon
anywhere in the file at any stride.
IFC export maps recovered Levels → storeys (with their real Revit
names and elevations on Revit 2024, #218), Rooms → spaces, and
Walls / Doors / Windows / Columns / Floors / BuildingPads →
`IfcWall` / `IfcDoor` / `IfcWindow` / `IfcColumn` / `IfcSlab` (or
`IfcShadingDevice` when overridden) with placement and an extrusion.
The extrusion body is the record's bounding box except for the plan
profile of a slab whose sketch closes, which is the sketch itself, and
the plan run of a wall, which is the box cut back by its joins (base/top
Level ElementId binding still open), and
Material display names → `IfcMaterial`.
**Wall bodies now carry the length their joins leave them and the wall's
real thickness** (#215, RE-26). An element can be framed by more than one
partition record, and 171 of 360 walls carry two frames that disagree
about the plan box: Revit rewrites an edited element into a
higher-numbered `Partitions/*` stream and leaves the earlier copy in
place, so the **newest** frame is the current state. On that frame the
box's thin plan extent equals the nominal thickness of the `IfcWallType`
Revit's own export assigns on **360 of 360** walls (201 of 360 on the
oldest frame) and its type slot in the `+0x88` list equals that
`IfcWallType.Tag` on 356 of 360 (183 of 360 on the oldest);
`Global/ElemTable`'s `u32` at `+0x1c` is a monotone version counter that
ranks the same way. No frame on the file disagrees about the box's `z`,
so nothing moves between storeys. What is left after that is the run:
every non-zero delta between the box and Revit's own `Axis` polyline
across all 720 wall ends is exactly 0.25, 0.3333 or 0.75 ft — half of
the 6″, 8″ and 18″ wall thicknesses on the file — so an end is cut back
by half the thickness of the perpendicular recovered wall whose
centreline lands on it, and candidates that disagree decline the whole
element. Measured against Revit's export in world coordinates
(IfcOpenShell 0.8.5, axis-aligned bounding box, tolerance 1e-3 ft):
**336 of 360 walls exact, up from 27**, worst residual 0.75 → 0.3333 ft,
mean 0.2701 → 0.0220 ft, 309 improved and 16 regressed. The 31 wall ends
that rule got wrong were all over-trims, and RE-26 could not separate
them from the 125 and 22 real trims in the same feature classes. #274
(RE-29) found the byte that does — see below — and the residual is now
9 ends. The solver still models Revit's join cleanup rather than reading
the cleanup itself, which is why the capability stays `partial`.
**Every recovered column names its family type** (#215, RE-26): the last
slot before the record's own ElementId in the `+0x88` list that is
itself an `OST_Columns` type-symbol record is `5755`
(`Column_Sqaure:24" x 24"`) on **256 of 256**, which is the
`IfcColumnType.Tag` Revit's export writes for every one of them, and the
section that symbol carries is emitted as the profile. On this file it is
the same 2 ft square the instance envelope already had — the two agree to
8.0e-15 ft — so no vertex moves; what changes is that `ProfileResolved`
now means the type said so. The column residual is **unchanged and was
never what #236 reported**: measured as a world bounding box rather than
a vertex-mean centroid, all 256 columns match Revit's z extent exactly at
both ends, 176 of 256 matched on every corner at RE-26, and the 80 that
did not have a body Revit *cuts* inset from the full prism (62 exported
as `IfcPolygonalFaceSet`), which no section can produce. What does
produce it is #274 (RE-29), below: all **256 of 256** are exact now.
**Doors and windows are bound to their host wall** (#222, RE-23): the
element record's counted reference list at `+0x88` names the host in
the slot immediately before the record's own ElementId, accepted only
when that value is one of the 360 recovered wall instances. Each
recovered opening is emitted the way Revit's own exporter does — an
`IfcOpeningElement` bodied with the door/window bounding box, an
`IfcRelVoidsElement` from the wall and an `IfcRelFillsElement` into the
door or window — and the resulting `(host wall, filling element)` pair
set equals Revit's export exactly: **138 of 138**, no wrong host, no
missing pair, no extra pair, gated as `relations.IFCRELFILLSELEMENT`
inside the OctetProof claimed surface.
**Storeys on Revit 2024 are the Revit `Level` elements themselves, names
and elevations together** (#218, RE-24). A `Level` record carries the
same 88-byte element-record prologue with `OST_Levels` (−2000240) at
`+0x12` but no bounding box — it is a datum plane, so the record ends at
`+0x56` where a column's bbox marker would start. Of the 75 such records
on Core Interior, the #211 instance test (no container at `+0x32`,
placement kind `0xffffef7f` at `+0x42`) selects exactly **15**, which is
the number of `IfcBuildingStorey` Revit's own export writes. Each of the
15 owns exactly one name/elevation parameter block, framed the way RE-22
framed the `IFC Export As` overrides — the owning `ElementId` as a `u64`
at `value-0x47`, a 56-byte `0xff` sentinel run, three zero bytes, the
`u32` UTF-16 length, then the name; the elevation is an `f64` in feet 55
bytes past an 8-byte marker searched forward from the end of the name,
repeated 153 bytes later, both copies required to agree. All **15 of 15**
`(name, elevation)` pairs equal an `IfcBuildingStorey` `Name` /
`Elevation` in Revit's export exactly — `Basement 2` −40, `Basement 1`
−20, `Level 1` 0, `Mez 1-2` 15, `Level 3 / 4 / 4 - Wall Layouts 1 / 2 / 3`
at 31 / 46 / 61, `Level 6`…`Level 13` at 76 / 91 / 106 / 121 / 136 / 151 /
166 / 185.5 ft — including the four elevations #213 could not see because
no column stands on them. The names are *asserted* rather than joined:
the block is keyed by the Level's own `ElementId`, so the file states the
pairing. Recovery is all-or-nothing per file — a Level with no accepted
block emits nothing, and the whole set is discarded unless every Level
record owns exactly one block and no two levels share an elevation.
Both manifests gate `diagnostics.exported.storey_count` at 15,
tolerance 0, with `levels` inside the claimed surface, and both
OctetProof verdicts carry `entity_counts.IFCBUILDINGSTOREY` **and**
`storeys.IFCBUILDINGSTOREY` — the exact `[name, elevation]` set — on
which rvt-rs, IfcOpenShell 0.8.5 and IFClite 7.1.1 agree byte for byte.
**The element record names the Level that hosts it** (#219, RE-27), and
that is now the first join tried. The counted reference list at `+0x88`
— the same list RE-23 reads for a door's host wall — carries the host
`Level` as a plain ElementId slot, and it is accepted only when it names
exactly **one** of the fifteen recovered Levels. That restriction is not
a threshold: a Revit column or wall carries a base *and* a top
constraint and both are in the list, so all 344 `OST_Columns` records
that name a Level name two of them and resolve to nothing, keeping the
elevation join they already bind exactly on. Where both joins answer the
same element the two agree **537 of 537**, with 0 disagreements, which
is why the stated join runs first. Containment therefore moves **801 →
969 of 970**: all 256 columns, all 132 doors, all 6 windows, all 116
rooms, all 100 record-backed plates (`IFCSLAB` 80 of 80,
`IFCSHADINGDEVICE` 20 of 20) and 359 of 360 walls. The 46 plates that
used to sit 0.1667 ft below their level at the structural-slab /
architectural-topping interface, and the 6 windows whose record base is
a sill height 4.73 ft above their level, are exactly the elements a
named Level reaches and an inferred elevation cannot. What stays unbound
is **one** wall, whose record names no single Level; it is contained in
the **`IfcBuilding`** — "in this building, storey unknown" — where the
writer used to drop such elements into whichever storey came first,
which on this file put 71 elements, some of them at 185 ft, into
`Basement 2` at −40 ft. The 18 name-only `IFCSPACE` rows that used to
head the unbound list are gone: #90 / RE-29 replaced them with 116
record-backed rooms that each name their Level outright. The rooms are
the one part of this measured against the reference: each of the ten
Levels they name maps to exactly one `IfcBuildingStorey` of Revit's own
export, and the storey it maps to is the one that export aggregates the
room into, 116 of 116. The other 853 bindings stay **NOT MEASURED**
against Revit's own `IfcRelContainedInSpatialStructure`.
The #213 column-derived path (`STOREY_ELEVATION_SOURCE_TYPES` =
`IFCCOLUMN`) survives as the fallback for files with no recoverable Level
records; its measurement stands unchanged, including the RE-22 finding
that admitting slab tops as an elevation source would buy −40 ft and
185.5 ft at the cost of 13 false elevations.
**The reference list also names the joins** (#238 / #239, RE-29). The
same counted list at `+0x88` that carries the type and the Levels holds
the ElementIds of the walls a wall is joined to, and of the walls that
cut a column. Two predicates on RE-26's solver — the trim candidate must
be named in this record's list, and it is tested for reach against its
*own* cut-back run rather than its recorded one — take wall ends from
**689 to 711 of 720** and wall bodies from **336 to 351 of 360** exact,
with 15 walls improved, 345 unchanged and **0 regressed**. The 9 ends
that remain are a **measured negative, narrowed**: one side of each of
two *true L corners* (one repeated on eight storeys) where neither run
continues past the meeting point and Revit cuts exactly one of the pair.
Nothing in the file orders the two sides — the survivor is the thicker
wall once and an equal-thickness wall once, the higher ElementId once
and the lower once — so the solver keeps cutting both rather than
guessing, and the over-trim is pinned by
`element_record_wall_joins::tests::the_surviving_side_of_a_true_l_corner_is_still_over_trimmed`
instead of being hidden. On columns the same list closes #239 outright:
the exported body is the record prism minus the untrimmed record prisms
of the walls it names, which is **256 of 256** exact at a worst residual
of 9.4e-13 ft (from 176 of 256 at 0.5833 ft). `difference_box` is
fail-closed — it answers only when the surviving material fills its own
bounding box exactly by volume, which is true on all 80 columns whose box
shrinks and false on all 69 interior slots, Ls and crosses — so a decline
never removes material Revit kept. The cut plan rectangles reproduce
Revit's own body-extent histogram exactly. Emitted product counts,
relations, storeys and the diagnostics sidecar are unchanged; the one
moving count is `IFCPROPERTYSINGLEVALUE` 9231 → 9311, the new
`JoinCutWallCount` provenance row per cut column. Substituting the
join-trimmed wall runs for the untrimmed ones scores 190 of 256, so a
wall runs its full recorded length into a column it cuts.
**A wall type is a record of its own, and every wall names exactly one**
(#88, RE-28). A `WallType` has the bbox-less RE-24 shape with a third
placement-kind value at `+0x42`, `0xffff8080`; eight `OST_Walls` records
carry it on Core Interior, of which Revit's export writes four as
`IfcWallType` and never places the other four. Replacing RE-26's
positional `references[1]` read with a *set* test — the single slot of
the reference list that is a wall-type record — is exact on **360 of
360** (against 356 of 360 positionally), needs no tie-break and declines
nothing, and the recovered distribution equals the export's own
`IfcRelDefinesByType` distribution exactly: 129 / 127 / 100 / 4. That
also closes #240 — the `18" Basement` slot reading `1851` where the
export's `IfcWallType.Tag` is `3897` was an **index artifact**, not a
second id space: the list is ascending and both ids are in it, with
`1851` a record of category −2009014 that is not a wall type at all.
**The join is library-side only.** It ships as
`rvt::partition_type_records::unique_type_reference` with the corpus gate
`tests/iter_elements_typed.rs::core_interior_2024_wall_type_record_join`;
it is **not** yet a property on the emitted wall, no emitted entity,
property or relation changes, and both count manifests record that.
**Compound layer thicknesses stay unrecovered, and this corpus cannot
witness them** (#88, RE-28 §4–§5). A 128 KiB sweep centred on each of the
four exported wall-type records finds **0** runs of consecutive `f64`
summing to that type's nominal width, and the two 6″ types have no
`0.5` ft hit anywhere in their span. More decisively, the reference
export is a **`ReferenceView_V1.2`** file carrying **0**
`IfcMaterialLayerSet`, **0** `IfcMaterialLayer` and **0**
`IfcMaterialLayerSetUsage`; all 360 walls associate to a single
`IfcMaterial` rather than a constituent set. #88's layer-thickness
criterion therefore has **no oracle on this artifact** — it can be
neither met nor refuted here — and closing it needs a layer-set-carrying
export of the same `.rvt` or an owner-supplied wall-type schedule. Until
one exists nothing emits a layer. The same work bounds materials: the
real carrier is a record family, **86** `OST_Materials` records in the
same bbox-less shape, a closed byte-derived set against the 102 the
string heuristic yields — but the records carry no recovered name and
the element → material join is incomplete, so nothing is recovered from
them yet and the manifest row stays `known_gap` (#34, #86).
The public viewer's demo gallery leads with the two files this section
measures, staged from the deploy workflow's `magnetar-io/revit-test-datasets`
checkout and pinned by sha256 (#257): `Revit_IFC5_Einhoven.rvt` (2023,
913 KB) and `2024_Core_Interior.rvt` (2024, 33.7 MB). Staging refuses a file
whose bytes do not match the catalog hash, and both cards are gated by
Playwright. Viewer File Status lists
recovered storey names, material name samples, and an honest Parameters
row (empty until AProperty* host joins). The scene tree groups elements
under `IFCBUILDINGSTOREY` nodes (ArcWalls and 2024 element records by
elevation; Floors/Rooms remain Unassigned until Level ElementIds exist
on both sides — bind plumbing is fail-closed and corpus-idle today).
RE-20 (same corpora) found **no** recoverable Level ElementId map:
`Level` is absent from Formats schema; LevelAssociationCell / name /
elevation proximity scans are noise-dominated — Floors/Rooms stay
Unassigned by evidence, not omission. RE-24 (#218) does not reopen
that: it recovers a *Level's own* ElementId from its `OST_Levels`
partition record, which is what lets a name be paired with an
elevation, but it does not give a Floor or a Room a reference to one.
RE-19 found **no** reliable Door vs Window discriminator in the
opening-index bytes and **no** schema-field / 2024 ArcWall envelope
suitable for fail-closed decode; both negatives stand, and the
`schema_field_wall_instances` diagnostic still fires. What #211 and
#222 solved is a different carrier — the partition element record —
and the opening-index rows still carry no Door/Window discriminator and
no host claim of their own. AProperty*
carriers are not present in production `iter_elements` / Global/Latest
candidate scans on these corpora (#35 host joins idle). Floor↔ElemTable id binding is closed for the recovered
slab set (every record-backed slab carries its ElementId), slab
extrusion thickness is measured from the record bbox, and the slab
*profile* is the boundary polygon its `OST_SketchLines` records close
on **80 of 80** exported slabs (#31, RE-25, closed 2026-09-19). What is
still a bounding-box rectangle there is the profile of the 20 rotated
shading plates, which the closure declines and which carry
`ProfileResolved: false`. Eighty-one per-class
decoder structs remain registered; `MVP_TYPED_CLASSES` are consulted by
`iter_elements`.

## Performance

Opening a large project is no longer a minutes-long wait. Every partition
consumer used to open the CFB stream and inflate all its gzip members for
itself — once per `BuiltInCategory`, once per sweep, and again for the
diagnostics pass — which on `2024_Core_Interior.rvt` meant re-inflating
178.9 MiB of `Partitions/*` more than twenty times per export. #266
memoises the inflate on the `RevitFile` handle, screens string candidates
on the raw UTF-16 code units before decoding rather than after, and uses
`memchr::memmem` for the category substring sweep.

Measured on Apple Silicon / macOS with `/usr/bin/time -l`, best of three,
against `main` at `8425a3f` (that is, *after* #267, #268 and #269):

| `rvt-ifc --mode geometry` on `2024_Core_Interior.rvt` | before | after |
|---|---:|---:|
| wall clock | 26.07 s | **1.69 s** |
| peak RSS | 2641.4 MiB | **490.1 MiB** |

`Revit_IFC5_Einhoven.rvt` (913 KB) is the control: 0.44 s / 25.1 MiB to
0.12 s / 20.7 MiB. Under `tools/perf_budget.py --require-category medium`
the three heavy rows go `element_decode` 6116 ms / 1858.0 MiB to 919 ms /
478.5 MiB, `ifc_export` 17880 ms / 2642.0 MiB to 1454 ms / 516.2 MiB and
`viewer_parse_render` 18014 ms / 2642.6 MiB to 1469 ms / 498.8 MiB; every
budget passes afterwards where all three failed on peak RSS before. No
budget was changed to make that true.

**The IFC output is byte-identical.** Both emitted STEP files match the
pre-change binary once the two wall-clock stamps the writer emits are
normalised (`FILE_NAME` and the `IFCOWNERHISTORY` epoch); there is no
`SOURCE_DATE_EPOCH` switch, so those two fields cannot be pinned without
a behaviour change.

In the browser the same change takes the 33.7 MB Core Interior demo from
about 28 s to **about 3 s** of decode, measured live on the deployed
site (6.7 s including the 32 MB download). The viewer's loading card
derives its "about N s" hint from that measurement —
`DECODE_BYTES_PER_SECOND` is 11 MB/s since #271, up from the 1.15 MB/s
the pre-#266 figure implied — and the progress bar stays indeterminate
because the decoder cannot report real progress.

## Capability Matrix

| Capability | Status | Evidence | User impact |
|---|---|---|---|
| Open `.rvt`, `.rfa`, `.rte`, `.rft` CFB containers | Full | `reader`, CI matrix | Files can be inspected without Revit. |
| Decode Revit truncated-gzip streams (gated checksum-page strip on Partitions/Global, #151) | Full | `compression`, `checksum_page_framing`, fuzz, `inflate_capacity_hint_does_not_scale_with_remaining_input` (#256) | Internal streams can be read safely; Formats/Latest stays ungated by default — multipage integrity uncertain (`RVT_FORMATS_MULTIPAGE_UNVERIFIED`). A multi-member stream reserves at most 1 MiB per member before decoding (`INITIAL_INFLATE_CAPACITY_BYTES`) and trims the retained buffer, so a 33.7 MB project decodes inside wasm32's 4 GiB linear memory rather than trapping. |
| CSV element and room schedules | Partial | `ifc::schedule_csv`, `rvt-schedule`, tests | Spreadsheet of decoded walls, doors, windows, columns, slabs and rooms; as complete as the decode of the file. No room area. |
| Extract metadata, PartAtom XML, preview PNG | Full | `basic_file_info`, `part_atom`, `metadata`, tests | Users can identify and audit files: release, worksharing state, central model path, last saved (time and user), save counter. `rvt-info <folder>` inventories a whole tree as a table, CSV, JSON or JSON Lines. |
| Parse `Formats/Latest` schema | Full | 100 percent field classification over 2016-2026 family corpus; multipage integrity diagnostics in inspect/export/viewer | Developers can inspect class and field structure; Formats multipage integrity uncertain while strip stays disabled. |
| Read document-level ADocument data | Partial | Reliable on newer samples; older/project bands need more corpus proof | Good for diagnostics, not complete model extraction. |
| Decode typed elements from real project files | **Partial** | Production `iter_elements`: ArcWall (2023) + partition MVP Levels/Materials + 2024 ArcWallRectOpening (ElemTable-confirmed related ids) + 2024 partition element records for `OST_Walls` / `OST_Doors` / `OST_Windows` / `OST_Columns` / `OST_Floors` / `OST_BuildingPad` (360/132/6/256/80 IFCSLAB + 20 IFCSHADINGDEVICE on Core Interior, exact ElementId sets, cross-witness gated, #204/#211/#212, RE-21/RE-22); wall bodies join-trimmed and columns joined to their family type (#215, RE-26); Revit 2025 element records decode with a per-release marker (walls 7/7, slabs 2/2, rooms 11/11 on the MIT RE1 model, 0 FP on six 2025 files, RE-32); HostObjAttr filtered; RE-19 negatives intact: no opening-index Door/Window discriminator, no schema-field Wall on magnetar corpora | Full model conversion is not ready; six categories on one Revit 2024 edge match Revit's exporter exactly, spaces and materials do not. |
| Typed decoder structs | Partial | `elements::all_decoders()` registers **81** decoders; `MVP_TYPED_CLASSES` consulted by `iter_elements`; ArcWall uses a separate partition decoder | Library building blocks plus production MVP/ArcWall path. |
| IFC4 writer | Partial | Synthetic fixtures validate in IfcOpenShell; every emitted instance carries the full IFC4 attribute list its type declares, gated per instance against the EXPRESS schema by `tools/ci/ifc_schema_arity.py` (#214); the element translation is carried once, by the element's `IfcLocalPlacement`, with the swept solid's `Position` at identity — the same gate composes `ObjectPlacement × Position` for pinned elements so the double-translation of #232 cannot return, and under `--witness-agreement` checks every written `PredefinedType` against the value Revit's own exporter writes for that type (#220: 360 walls `.NOTDEFINED.`, 116 spaces `.SPACE.`, 132 doors `.DOOR.`, 6 windows `.WINDOW.`, 256 columns `.COLUMN.`, 80 slabs `.FLOOR.`); 2023 Einhoven ArcWall `IfcWall` + partition Level storeys / Floor boundary `IfcSlab` / Room `IfcSpace` / 2024 `IfcWall` + `IfcDoor` + `IfcWindow` + `IfcColumn` + `IfcSlab` + `IfcShadingDevice` + record-backed `IfcSpace` rooms with their Revit name, number and storey (#90, RE-29) with placement + bounding-box extrusion (join-trimmed run and thin-axis thickness for walls, family/type section for columns, sketch profile for slabs; #215, RE-25/RE-26) + named Revit Level storeys with measured containment (#218/#213/#212) + measured slab thickness (#212) + the `IfcOpeningElement` / `IfcRelVoidsElement` / `IfcRelFillsElement` chain that voids all 138 doors and windows out of their host wall (#222, exact pair-set match) / Material display names; the plan profile of a wall, door or window is still its record envelope; `rvt-ifc --diagnostics` JSON readiness sidecar; `--mode` gates scaffold/typed/geometry/strict | Correct writer path exists, but real-file typed inputs are incomplete / unsolved. |
| Browser viewer | Partial | GitHub Pages deployment, no-network WASM import gate, File Status shows production class counts + storey/material totals, supported-profile matrix, two hash-verified MIT real projects at the top of the demo gallery with Playwright coverage (#257) | Useful for local inspection; geometry reflects decoded coverage. The public site now opens a real Revit project, not only 20 KB synthetics. |
| Stream-level writer | Partial | Always-on patch corpus (`gen-fixture` project + MIT `empty.rfa`) covers identity, grow, shrink, multi-stream, missing-stream; optional Autodesk corpora add release-matrix + GUID/history checks; corrupt-gzip verification is unit-tested | Useful for controlled stream replacement, not semantic Revit editing. |
| Python package | Partial | CI wheel builds and pytest | Useful for metadata/schema automation. |
| User-facing inspect CLI | Partial | `rvt-inspect` reports file health, decoded coverage, IFC export readiness, warnings, next steps, and stable JSON | Useful for support triage without Revit internals. |
| Community corpus open/scaffold check | Partial (executed) | `docs/corpus-hunt-2026-04-21.md`: 222/223 real files pass open → schema → scaffold IFC | Proves container/schema health on public samples; does **not** prove typed element recovery. |

## Roadmap Position

The near-term project is tracked in GitHub milestones:

- `0.2.0: audit-clean alpha` — **scope complete** (0 open / 19 closed). Quality
  script, honest docs, issue forms, and supply-chain checks landed on `main`.
  Crate version remains `0.1.x` (latest tag `v0.1.2`); **0.2.0 release hold**
  until a version bump / tag is cut (audit GOV-002).
- `0.3.0: real-project wall/floor MVP` - corpus-backed partition scanning and typed element recovery.
- `0.4.0: IFC geometry beta` - trustworthy IFC export modes, diagnostics, and validation.
- `0.5.0: viewer beta` — **tracked issues complete** (0 open / 6 closed). Viewer
  guidance, demo gallery, and browser regression work is on `main`; **0.5.0
  release hold** until a viewer-beta release is cut (audit GOV-002).
- `1.0.0: first-class utility` - documented non-technical workflow with clear support boundaries.

The detailed task backlog lives in [`TODO.md`](../TODO.md) and the matching
GitHub issues.

## Supported MVP Definition

The first broadly useful release should let a non-technical AEC user:

1. Open a supported Revit file locally without uploading it.
2. See a clear status report that says what was decoded, what was skipped, and
   why.
3. Export IFC only when typed elements and geometry meet the supported profile.
4. Receive actionable diagnostics when a file is outside that profile.
5. Follow docs written for BIM users, not Rust developers.

Until those five conditions hold, rvt-rs should present itself as an
open-source Revit inspection and reverse-engineering toolkit, not as a complete
replacement for production Revit export workflows.
