# Project Status

Last reviewed: 2026-10-30

This page is the public source of truth for what rvt-rs can do today. It is
intentionally blunt so users can decide quickly whether the tool fits their
workflow.

The current support boundary is tracked in the
[supported MVP input profile](supported-profile.md). The machine-readable
[executable support matrix](support-matrix.json) (schema:
[`support-matrix.schema.json`](schemas/support-matrix.schema.json)) is the
checked-in capability ceiling: `tests/support_matrix.rs` keeps its
statuses honest, and none may claim converter-grade typed recovery. Keep that matrix, this page, the README,
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

**Research program pointer:** the
[unified research report](research/unified-research-report.md) (rev 1.1)
sets out the research on remapping ES ElementIds, whose fixtures
need a Revit-hosted API oracle that CI does not have.
ES remapping is **not** a shipped capability; default IFC omits ES edges.
Coordination mirror: [Discussion #112 notes](disc-112-coordination.md).

## User-Facing Summary

rvt-rs opens every Revit file from 2016 to 2027 and reads its metadata,
previews and embedded schema. Revit 2015 files open too, measured on
Autodesk's six 2015 sample projects, where previews and PartAtom are not
measured. On Revit 2024 and 2025 project files it reads
the building: each element's ElementId, category, name, type (written as
an IFC type object of its Revit type, RE-110), storey, materials, layers,
GlobalId, `IsExternal` (RE-193), curtain mullions' `Span` (RE-226) and, where the boxes of their wall's mullions say which ends Revit's solid keeps, `NetSurfaceArea` (RE-268), walls'
`ExtendToStructure` (RE-228), railings' `Height` (RE-231), stair flights' treads, nosing and run type
and landings' type (RE-92) and the parameters named for a standard property
set's property (RE-194), and a body that is exact where the element's
data is decoded and its bounding box where it is not. It writes that as IFC4,
glTF, plan SVG and CSV, from the CLIs, Python, the zero-upload browser
viewer, and a local, read-only MCP server (`rvt-rs-mcp`, [`docs/mcp.md`](mcp.md))
that answers an agent's questions from the model the IFC export writes. On Revit 2023 projects it reads each element's ElementId, category
and bounding box (RE-81), walls cut back at their joins (RE-120), beams at their columns
(RE-122), door and
window hosts (RE-85), room outlines
(RE-102) with their numbers and names (RE-117, which names every
release's rooms), Levels with the storey each element sits on (RE-107;
beams by the Level at their top, RE-119), and
family instances' family and type names, with their IFC type objects
(RE-109, RE-110), walls', floors' and roofs' types and layers
(RE-111, RE-112), family instances' materials (RE-113), and walls' and
floors' material layer sets (RE-114, RE-115), with each material's shading
colour (RE-116). On Revit 2026 projects, experimentally since 0.5.0, it reads
typed elements, storeys, rooms, doors and windows, type names and layer sets,
measured on one project against Revit's own export (RE-124). On projects
saved by Revit 2015 to 2022 and by Revit 2027, experimentally since 0.6.0, it
exports their elements, measured against the same sample projects saved by
Revit 2019 (for 2015 to 2018, RE-181), 2023 (for 2019 to 2022, RE-180) and
2026 (for 2027, RE-179) rather than against a Revit export of them, except two
projects against Revit's own export of their release: BIMcollab's 2015
structural example (RE-198) and Vitoria's 2018 `ALD26 Before` (RE-315, RE-319);
2015's `rst_basic` is another model than its 2019 copy, so it is not measured.
Conduits, conduit fittings and gutters export on every release the exporter
reads and are measured against Revit's own export on `ALD26 Before` only (all
178 it holds are Revit's elements, RE-319); Autodesk's MEP samples and Snowdon
Towers Electrical export them too, counted and not scored, and cable trays do
not export, since no Revit export holds one. On
projects saved by Revit 2011 to 2014, experimentally and outside the 1.0
range (2015 to 2027), it exports their elements too, measured on a local copy
of Autodesk's 2013 `rac_advanced` sample against its 2015 copy, and files of
Revit 2008 to 2013 now open (RE-259); on Revit 2008 to 2010 projects it
exports their elements, storeys and named materials, checked only for
consistency within each file (RE-260). Those files are closed source and
measured locally only. Every
other recovery claim is measured against the oracle its report names, Revit's
own IFC export of the same file wherever one exists; the measured tables are in the
[evidence page](evidence.md#what-rvt-rs-reads-from-real-projects) and the four
levels a file can reach (opened, elements identified, geometry, properties)
are in the [supported profile](supported-profile.md). On Revit 2023 to 2027
files, and experimentally on 2015 to 2022 ones (RE-281), `rvt-elements` and Python also list each element's saved parameters,
named where the file defines them (RE-207), and `rvt-ifc --revit-property-sets`
writes the named ones in Revit's own property sets (RE-210). With
`--revit-set-placement` it also writes properties where Revit's own exporter
does and IFC4's definitions do not, such as a room's
`Pset_SpaceCommon.FloorCovering`; by default every property is in the set
IFC4 gives (RE-235). It writes IFC4's base quantities by default, as Revit's
built-in IFC4 export setups do, and `--no-base-quantities` leaves them out,
as Revit's exporter does with "Export base quantities" off (RE-242).

It is not a converter for arbitrary models: most parameters, family
geometry (doors and windows are drawn from the mesh Revit saved for their
display by default, and any other family instance with the experimental
`--saved-meshes` option, RE-78), hip roofs over a T or U footprint, roofs sloped by several slope arrows of the configuration of puzzbobb's U, where Revit slopes the two long outer walls along the arms only (RE-307; no other roof of the configuration is measured), and roofs by extrusion outside Revit 2025, the top of
walls whose layers Revit ends at different heights or cuts at a height no join names (#1067), the boundaries of
the rooms whose stored solid still does not close (RE-316 §4) and opening
profiles are not decoded, and rvt-rs does not edit Revit model data.

**Rooms.** Each `IfcSpace` says in `RvtElementRecordGeometry.SpaceOutlineSource` whether its outline was read from the room's stored solid (`stored_solid`), is the record's bounding box (`record_box`), was drawn from a saved mesh or a family's body, is an Area's region among its boundary lines (`area_boundary`, RE-317; `area_boundary_joined` where its outline needed a boundary line carried on to the line it joins, or a connector, RE-330), or has no body (an Area whose boundary is not proven), and the diagnostics count the boxes (`exported.bounding_box_spaces`) and, by reason, the Areas that have no body (`exported.area_body_refusals`, RE-317). Areas are read on Revit 2015 to 2024 (RE-324); those of 2015 to 2023 are written only where their body is proven, counted otherwise in `skipped` as `area_without_proven_body`, and checked against the same project's 2024 save on Autodesk's samples and, for the 16 Areas of `rac_advanced` it draws, against an IFC2X3 export of that project from the three.js repository, by a build of Revit 2021's exporter whose string ends "IU alternativa 21.0.0.383" (publisher not established), showing the 2015 to 2020 state of the project (RE-330): the only export of a 2015 to 2023 Area there is, and not one of Revit's IFC4 exports. Measure scores every space's plan area and outline against Revit's export, and CI gates Core Interior (116 of 116 rooms) and RE1 Architecture (11 of 11). A room whose stored solid the exact walk did not close is read again from the faces its solid lists, up to the solid's own end, and says so: `stored_solid_faces` where the faces' edges close as the walk reads them (9 of the BIM4LCA office's 13 such rooms as built, all at Revit's area to 0.1%) or `stored_solid_approximate` where a column overlaps a wall, an end of one wall edge lies on another, or the outline falls up to 0.02 ft short of the box (4 as built and 11 of the 12 at building permit: 10 of the 15 at Revit's floor area to 0.1%, the 5 others, four rooms, 0.45% to 0.68% under it); the diagnostics count them (`exported.resolved_outline_spaces`, `exported.approximate_outline_spaces`). The scorer's upward-face area reads one of them (1671995) as 323% over and RE-313's 66% room as 66% off; by Revit's floor both are within 0.5% and 0.1%. The 6 `rac_advanced` rooms with round columns, 49 of the 50 `rme` rooms, Snowdon Towers' 7 and office room 3095831 at permit keep their box (RE-316). [docs/rooms.md](rooms.md) says what to trust per release, and RE-313 has the runs.

## Finding history

The paragraphs below record, finding by finding, how that came to be. They
are history: where an early paragraph says something is not decoded and a
later one says it is, the later one holds.

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
value leaves the element on its class mapping. Since RE-45 the override is
read as what it is, the `IFC_EXPORT_ELEMENT_AS` parameter entry of the
element's serialised data, on Revit 2024 and 2025, together with its IFC
predefined type and the pair a type sets for its elements: Core floors 20912
and 70325 carry `ifcSlab` with `ROOF` and are Revit's two `IFCSLAB … .ROOF.`
rows, and the three `teste_export_2025` walls whose type carries
`IfcCoveringType` with `CLADDING` are its three `IFCCOVERING … .CLADDING.`
rows (`reports/element-framing/RE-45-ifc-export-parameters.md`). A curtain wall
in a Revit 2024 file stays `IfcCurtainWall` whatever its value names, as
Revit 2024's exporter writes it: the 124 BIM4LCA office curtain walls set to
`IfcShadingDevice` are `IfcCurtainWall` in Revit's export (RE-188). Nor does a
wall sweep in a 2024 file: the office's 9 sweeps set to `IfcCovering` are
proxies in Revit's export, measured on that project only (RE-192). The office's
fixtures and appliances follow their types' values: 78 waste terminals, 51
electric appliances, 40 fire suppression terminals, 15 wash-hand basins and
an air terminal on the as-built file, each Revit's class and predefined type
(RE-190), and two generic models and a curtain panel follow their own
`IfcStair` and `IfcDoor` (#792). From Revit 2024 an entry belongs to the
data object that holds it
(RE-153), not to the last element-data header before it. Values the
export does not apply are counted in the diagnostics' warnings. Composed, the two rules
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
`IfcArbitraryClosedProfileDef`. The 20 shading devices are plates
whose sketch-line boxes are axis-aligned envelopes of diagonal
segments, which the box closure declines. Since RE-95 they close from
their lines' recorded ends (RE-50), once the one zero-length sketch
line each carries is left out, and all 20 equal Revit's outline
(`reports/element-framing/RE-95-zero-length-sketch-lines.md`). The pre-RE-25 plan-loop scan is
a measured dead end for this: 2317 closed plan-polyline candidates
across all eight inflated partitions, none with the plan bounds of any
recovered plate, and no ordered vertex run of the export's polygon
anywhere in the file at any stride.
IFC export maps recovered Levels → storeys (with their real Revit
names and elevations on Revit 2024, #218; since RE-118 only the Levels
that are building stories, as Revit's export does, which gives Snowdon
Towers' structural model its 12 storeys), Rooms → spaces, and
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
named Level reaches and an inferred elevation cannot. What stayed unbound
was **one** wall, whose record names no single Level (RE-59 now binds it
through its base constraint); such an element is contained in the
**`IfcBuilding`** — "in this building, storey unknown" — where the
writer used to drop such elements into whichever storey came first,
which on this file put 71 elements, some of them at 185 ft, into
`Basement 2` at −40 ft. The 18 name-only `IFCSPACE` rows that used to
head the unbound list are gone: #90 / RE-29 replaced them with 116
record-backed rooms that each name their Level outright. The rooms are
the one part of this measured against the reference: each of the ten
Levels they name maps to exactly one `IfcBuildingStorey` of Revit's own
export, and the storey it maps to is the one that export aggregates the
room into, 116 of 116. On the BIM4LCA office, the 30 Areas carry no record
box. They are read from their partition chain records and aggregated into
their Levels' storeys as Revit's export aggregates them, 30 of 30 on each
file (RE-191), and drawn as the region their area boundary lines enclose
(RE-317). The other 853 bindings stay **NOT MEASURED**
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
RE-127 (2026-09-28) reads which one from the lists of walls each wall is
joined to: the wall whose entry for the other is a run of consecutive
values is cut. Wall ends are now **720 of 720** and wall bodies **360 of
360** exact.
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
`rvt::research::partition_type_records::unique_type_reference` with the corpus gate
`tests/iter_elements_typed.rs::core_interior_2024_wall_type_record_join`;
it is **not** yet a property on the emitted wall, no emitted entity,
property or relation changes, and both count manifests record that.
**RE-28 left compound layer thicknesses unrecovered, and found this
corpus cannot witness them** (#88, RE-28 §4–§5; RE-53 and RE-58 below
supersede this). A 128 KiB sweep centred on each of the
four exported wall-type records finds **0** runs of consecutive `f64`
summing to that type's nominal width, and the two 6″ types have no
`0.5` ft hit anywhere in their span. More decisively, the reference
export is a **`ReferenceView_V1.2`** file carrying **0**
`IfcMaterialLayerSet`, **0** `IfcMaterialLayer` and **0**
`IfcMaterialLayerSetUsage`; all 360 walls associate to a single
`IfcMaterial` rather than a constituent set. #88's layer-thickness
criterion therefore has **no oracle on this artifact** — it can be
neither met nor refuted here — and closing it needs a layer-set-carrying
export of the same `.rvt` or an owner-supplied wall-type schedule. The
same work bounds materials: the
real carrier is a record family, **86** `OST_Materials` records in the
same bbox-less shape, a closed byte-derived set against the 102 the
string heuristic yields. RE-28 recovered no name from them; RE-58 names
78 of the 86. The element → material join is still incomplete, and the
manifest row stays `known_gap` (#34, #86).
**RE-53 (2026-09-23) found the layers** in the type's own element data,
not near its record, which is where RE-28 swept. On Core Interior each of
the four types is one layer by category. On Snowdon Towers, 42 of 42
types give Revit's constituent sequence, and the glTF export draws 2024
walls as their layers in their materials' colours
(`reports/element-framing/RE-53-wall-layers.md`). RE-58 writes them into
the IFC export.
**RE-54 (2026-09-23)** builds a 2024 wall's body from its stored location
line, which is its centreline, at its type's thickness, where the record
box does not already give it. Core Interior's walls are unchanged.
Snowdon's angled walls, which were their whole axis-aligned box, have
Revit's faces on 75 of 102 (`reports/element-framing/RE-54-wall-bodies.md`).
**RE-55 (2026-09-23)** reads the same wall line, orientation and layers on
Revit 2025: a house saved in 2024 and 2025 reads the same on all 50 of its
walls, and 2025 walls now get centreline bodies and layers
(`reports/element-framing/RE-55-wall-lines-2025.md`).
**RE-56 (2026-09-23)** reads a roof edge's slope and draws a 2024 shed roof
along it when its rise and thickness reproduce its recorded height
(`reports/element-framing/RE-56-roof-slopes.md`).
**RE-57 (2026-09-23)** draws floors, roofs and ceilings in the glTF export
and the viewer as their type's layers, stacked top first, where they add up
to the element's recorded height; every layer matches Revit's on Snowdon's
236 and Core Interior's 68 compared slabs
(`reports/element-framing/RE-57-slab-layers.md`).
**RE-58 (2026-09-23)** reads each material's name from its own data, by
ElementId, and the IFC export writes the layers of walls, floors, roofs
and ceilings as an `IfcMaterialLayerSetUsage` over an
`IfcMaterialLayerSet`, each layer with its material and thickness.
- Materials named: 219 of Snowdon's 220 and 78 of Core Interior's 86, and
  every one of RE1's, Projeto1's, `teste_export_2025`'s and the tutorial
  house's.
- Against Revit's own exports, the layer names are Revit's, in Revit's
  order, on 3,321 of Snowdon's 3,322 layers and all of Core Interior's
  and RE1's.
- The sets sit on Revit's bodies within 0.001 ft on 1,196 of 1,207
  Snowdon elements, 403 of 403 on Core Interior (88 before RE-91) and 15
  of 15 on RE1, whose
  2025 export writes the same four sets
  (`reports/element-framing/RE-58-material-layer-sets.md`).

Since #34, the IFC's material list on Revit 2024 and 2025 is those named
material records, each with its colour, rather than every material-like
string in the file: Snowdon writes 219 materials instead of 946, and Core
Interior 78 instead of 104. Every material Revit's own export writes is
among them, except two on Snowdon.

Core Interior's reference export is a Reference View file with no layer
set in it. There the layers are scored against the faces Revit's body
styles with each material. Its walls' single layers take their category's
material, and since RE-91 they carry the one the document's object styles
give walls, "Default Wall", as Revit's export does.

**RE-200 to RE-206 (2026-10-21)** read family instances', mullions',
curtain panels', off-centre columns' and walls' materials by name from the
native records and the family's own embedded document, not from byte
offsets (#355):
- a type whose map holds no material: the instance's own material
  parameters (RE-200);
- a mullion's or panel's type: `m_renderStyleId` (RE-201);
- an off-centre column: one layer of the first joined wall that overlaps
  it, on its side of the wall (RE-202, RE-205);
- rendered content (RPC, planting) is `<Unnamed>` (RE-202);
- a type's geometry map and a material's name: `m_geomTag2MaterialId` and
  `Material.m_name` (RE-203);
- `<Unnamed>` only where the family shows a visible form with no material
  (RE-204, RE-205);
- a layer taking its category's material, where the object styles are
  unset or not read (every 2025 file): the category's graphics style
  (RE-206).

Of the elements rvt-rs gives a material, all 5,134 on Snowdon Towers have
Revit's set (3,457 of 4,042 before), 71 of 71 on RE1 Architecture and 49 of
49 on RE1 Electrical. On the 22 R2025 models, 195 have Revit's set (24
before) and none another. Core Interior is unchanged. Elements whose
material is not read at all still get none: 407 on Snowdon (members,
railings, stair flights), 12 slabs on Core Interior and a railing on RE1
Architecture (`reports/element-framing/RE-200-…` to `RE-206-…`).

**RE-59 (2026-09-23)** contains an element whose record names two Levels,
its base and top constraint, in the base constraint: the higher of the two
at or below the record's base, else the lower. That is the storey Revit's
own export gives it on 650 of 650 such Snowdon records, 482 of 482 on Core
Interior and 7 of 7 on RE1. Core Interior now binds all 970 building
elements, each in Revit's storey. On Snowdon, 5,612 elements are in
Revit's storey (3,975 before) and 294 are on none (1,931 before). The
tutorial house, where 230 were on none (#365), leaves one railing
(`reports/element-framing/RE-59-base-constraint-levels.md`).
**RE-60 (2026-09-24)** places elements whose record names no Level:
- a railing takes the storey of the stair or ramp it names, which is
  Revit's storey on 73 of 74 Snowdon railings;
- any other element takes the Level its work planes carry, but only where
  that is also the highest Level at or below its base. That is Revit's
  storey on 161 of 161.

Snowdon now has 5,805 elements in Revit's storey and 102 on none
(`reports/element-framing/RE-60-hosted-levels.md`).
**RE-61 (2026-09-24)** names each layer set `Family:Type` as Revit does,
for example "Basic Wall:Exterior - Brick on Mtl. Stud". Revit stores no
system family name; a type with compound layers is a Basic Wall, Floor,
Compound Ceiling, Basic Roof or Pad by its category. The names equal
Revit's on all 1,210 Snowdon elements and on every Core Interior and
RE1 element
(`reports/element-framing/RE-61-system-family-names.md`).
**RE-62 (2026-09-24)** gives a curtain-wall part whose record names no
wall the curtain wall whose record box contains it. That is Revit's parent
for all 89 such mullions on Snowdon
(`reports/element-framing/RE-62-curtain-part-containment.md`).
**RE-63 (2026-09-24)** names walls, curtain walls, floors, ceilings, roofs
and railings `Family:Type:ElementId`, with `ObjectType` `Family:Type`, as
Revit's export names them. The system family is derived by RE-61's rule.
On Core Interior all 460 such elements carry Revit's name, and on Snowdon
every wall, curtain wall, ceiling and roof does
(`reports/element-framing/RE-63-system-family-element-names.md`).
**RE-64 (2026-09-24)** names three more kinds the same way. A curtain
panel that holds a basic wall takes the one layered wall type its record
names, and slab edges and ramps take their only system family. On Snowdon,
18 of 18 panel walls and 59 of 59 slab edges carry Revit's name, and both
ramps its `ObjectType`
(`reports/element-framing/RE-64-panel-walls-slab-edges-ramps.md`).
**RE-65 (2026-09-24)** names stairs `Family:Stair:ElementId` and their
runs, landings and supports after them ("Assembled Stair:Stair:620883 Run
1"), with `ObjectType` from each type's name and a flag in its own object
(Assembled or Cast-In-Place Stair, Monolithic or Non-Monolithic Run,
Stringer or Carriage). On Snowdon all 256 stair elements carry Revit's name
and `ObjectType`. On Revit 2025 the same names come from the stairs' native
records (RE-65 §6): on puzzbobb's five R2025 stair models every stair,
flight and stringer has Revit's name and `ObjectType` and a type object
with Revit's GlobalId, where none had either
(`reports/element-framing/RE-65-stair-names.md`).
**RE-66 (2026-09-24)** reads a railing type's name from its type object
when the type has no element data. On Snowdon all 131 railings now carry
Revit's name, up from 101
(`reports/element-framing/RE-66-railing-type-names.md`).
**RE-67 (2026-09-24)** names model text "Model Text:<type>:<ElementId>":
its list names a text type, a type object that frames a font after its
name. On Snowdon all 7 model texts carry Revit's name
(`reports/element-framing/RE-67-model-text-names.md`).
**RE-68 (2026-09-24)** places the Snowdon elements RE-59 and RE-60 left on
no storey. A multistory stair takes the Level its base sits at, and its
railings follow it. The parts of a nested light fixture follow the fixture.
Slab edges, light fixtures, wall sweeps, generic models, hardscape and
stairs take the Level their base elevation gives, within a fail-closed
band. 97 of the 102 are now on Revit's storey and none is on a wrong one
(`reports/element-framing/RE-68-remaining-storeys.md`). Its addendum
(§6, #369) places the 4 plumbing fixtures left over: they take the Level
their Level objects carry, as Revit does, whatever their base elevation
gives. On Snowdon 5,949 elements are on Revit's storey, 1 on no storey.
**RE-69 (2026-09-24)** names wall sweeps "Wall Sweep:<type>:<ElementId>"
from the one type object their record names, and names a sweep of a wall
type's own structure by its ElementId, as Revit does. On Snowdon all 258
wall sweeps carry Revit's name
(`reports/element-framing/RE-69-wall-sweep-names.md`).
**RE-70 (2026-09-24)** reads which wall runs through a butt join from the
walls' own join lists: the wall that names the other runs on to its far
face, and the other stops at its near face. They never contradict a joint
Revit's export draws cleanly on Snowdon Towers (212 of 212 decided ends),
nor the record box at 1,110 axis-parallel ends on five 2024 and 2025
files. Between single-layer walls the body now ends there, which gives 8
more Snowdon walls both of Revit's ends and makes none worse. Layered
joins, which Revit cleans layer by layer, keep their ends, but every
decided end, layered or not, names its partner wall and whether it runs
through, in the IFC and in the viewer's element panel
(`reports/element-framing/RE-70-wall-join-lists.md`).
**RE-71 (2026-09-24)** draws layered walls' corners layer by layer: the
layers nest like concentric Ls, the through wall's layers reaching the
outer edges of the other wall's and the stopping wall's their inner edges.
Where both walls show the same layers in the same order and share their
base and top, the wall's body is that staircase. On Snowdon Towers 516 of
976 axis-parallel walls have both of Revit's ends (346 before) and 3,925 of
5,634 layer ends match Revit's body (3,078 before). A few such joints
Revit draws as clean butts, which nothing read predicts: 16 layer ends on
Snowdon get worse. One RE1 wall does too, but RE1's export writes each wall
as one rectangle and cannot show layered ends
(`reports/element-framing/RE-71-layered-wall-joins.md`).
**RE-72 (2026-09-24)** reads the curtain grid a panel lies on, whose own
data names its curtain wall, and exports panels that hold a basic wall as
`IfcCurtainWall` with their body, as Revit does. On Snowdon all 2,075
curtain parts Revit aggregates have Revit's whole and all 60 curtain walls
match Revit's (`reports/element-framing/RE-72-curtain-grids.md`).
**RE-73 (2026-09-24)** draws T joints: a wall that ends part way along
another stops against it layer by layer, each layer passing the other
wall's layers of lower priority and stopping at the first of equal or
higher priority, and a single-layer wall stops at the face. On Snowdon
Towers 280 of 313 measured T ends are right in every layer, 675 of 976
axis-parallel walls have both of Revit's ends (516 before) and 4,290 of
5,634 layer ends match Revit's body (3,925 before). Revit draws some T
joints as a clean stop at the face, which nothing read predicts, and 13
Snowdon walls lose an end they had. Against RE1's export, which writes each
wall as one rectangle, 3 of its 7 do
(`reports/element-framing/RE-73-wall-tee-joins.md`).
**RE-74 (2026-09-24)** ends walls that meet at an angle on slanted lines:
RE-70's, RE-71's and RE-73's joins apply at 45 to 135 degrees, each layer
ending where its two edges meet the other wall's line. On Snowdon Towers
34 of 102 angled walls have both of Revit's ends (18 before) and 689 of 976
axis-parallel walls (675). Every angled L join drawn is right along every
layer edge, and 18 of 22 angled T joints; one wall gets worse
(`reports/element-framing/RE-74-angled-wall-joins.md`).
**RE-75 (2026-09-25)** draws curved walls along their arc. The record RE-49
reads as a line is an arc where the `u64` before it is 1, with its angles,
axes, radius and centre. On Snowdon Towers the 24 curved walls Revit draws
with an arc axis have Revit's centre and radius, and each is now the ring
sector its arc and thickness make: faces Revit's on 23 (0 before), and a
median footprint overlap of 0.95 (0.03 before). Their ends are not yet
trimmed at joins (`reports/element-framing/RE-75-curved-walls.md`).
**RE-86 (2026-09-27)** draws tapered walls. A wall type stores three face
angles after a fixed frame (`u32 2`, eight zero bytes, `f64` 0.7, a count
of 3), and a wall whose orientation word is 2 is tapered by them. On
Snowdon Towers every such wall Revit tapers leans its exterior face by the
type's 10 degrees and keeps its interior face vertical, and is its type's
thickness about its line at the top. Each is now that cross-section along
its line: 19 of the 20 within 0.001 ft of Revit's body across the wall at
five heights (none within 0.1 ft before), and 24 walls better in the GLB,
none worse (`reports/element-framing/RE-86-tapered-walls.md`).
**RE-213 (2026-10-24)** reads the same face angles on Revit 2025, which
frames them with twelve zero bytes after the `u32 2` instead of eight. On
Snowdon Towers saved by Revit 2025, 19 of the 20 walls Revit tapers are within
0.001 ft of its body across the wall (none within 0.1 ft before), and 24 walls
are better in the GLB, none worse
(`reports/element-framing/RE-213-tapered-walls-2025.md`).
**RE-87 (2026-09-28)** reads a family instance's transform: twelve `f64` in
its data, a 3 x 3 rotation stored row by row and its origin. On Snowdon
Towers, of the 1,956 upright transforms in Revit's export, 1,951 point X
along Revit's placement X, against it or square to it (the 5 others are
walls); RE1 Architecture's 49 all do. A family instance turned off the
model's axes is now the rectangle at its angle whose box is its record box:
all 181 turned Snowdon instances are closer to Revit's body along their own
axes (median 1.47 ft off before, 0.48 ft now), and the openings of the 27
turned doors and windows are Revit's width along them
(`reports/element-framing/RE-87-instance-transforms.md`).
**RE-88 (2026-09-28)** gives walls whose body is not a layer set their
layers' materials as Revit's export does: one layer its `IfcMaterial`,
several an `IfcMaterialConstituentSet`, exterior first, a material's k-th
occurrence named " (k)". That is Revit's association on every Snowdon wall
rvt-rs writes a layer set for (143 of 143, 823 of 824); 95 of the 111
walls that had no material now have Revit's, none a different one,
including walls whose data does not place them
(`reports/element-framing/RE-88-wall-materials-without-layer-sets.md`).
**RE-89 (2026-09-28)** cuts a window's opening in a tapered wall as Revit
does: a vertical box the host type's thickness deep, centred on the
window's own origin (RE-87). All 20 such windows on Snowdon Towers now have
Revit's opening across the wall (none before)
(`reports/element-framing/RE-89-tapered-wall-window-openings.md`).
**RE-91 (2026-09-28)** reads each category's material from the document's
object styles (`i64 BuiltInCategory · ff×8 · u32 1 · ff×8 · u64
material`). Where the Walls entry is set, a wall layer taking its category's
material takes it: Core Interior's 356 such walls carry "Default Wall", as
Revit's export does, and its layer sets go from 88 to 403
(`reports/element-framing/RE-91-category-material.md`).
**RE-92 (2026-09-28)** reads a stair run's StairsRun record (seven
`f64`, a `u32` top riser index, then centre mark, begin with riser and
end with riser flags). "End with Riser" is 0 on exactly the three
Snowdon runs with one riser line more than risers, which are now drawn
ending with a tread. Monolithic runs are drawn as one cast body. 38 of 43
Snowdon flights are drawn as their steps (34 before), and the 4 added
equal Revit's geometry but for one nosing edge
(`reports/element-framing/RE-92-stair-run-ends.md`).
**RE-93 (2026-09-28)** reads a type's length parameters from its value
block (`f64 · ff×8 · i64 parameter`). Revit's export cuts a window's
opening its type's Width by Height, centred on its origin, from 3.0 ft
above it, which is the type's Default Sill Height where the type stores
one. 56 of Snowdon's 68 windows now take that opening, each within
0.001 ft of Revit's (none before), and their `OverallWidth` /
`OverallHeight` (`reports/element-framing/RE-93-window-openings.md`).
**RE-94 (2026-09-28)** cuts a door at its type's Rough Width by Rough
Height where its body is Rough Height tall, which on Snowdon is exactly
the 57 doors Revit cuts that way; 54 now match Revit's opening within
0.001 ft (none before), and all 57 carry Revit's `OverallWidth` /
`OverallHeight` (`reports/element-framing/RE-94-door-openings.md`).
**RE-96 (2026-09-28)** reads a sketch line's arc, stored in RE-75's arc
layout without the marker a curved wall's has, and keeps an arc outline
only where it spans its element's record box. On Snowdon Towers 149 of
199 slabs carry an outline (132 before), and 87 match Revit's area
within 0.1% (73 before)
(`reports/element-framing/RE-96-sketch-arcs.md`). Its addendum reads a
sketch with a quarter arc from its curves, where RE-25's box solve had
closed it with the arc's chord: the BIM4LCA office's roof 1461758 then has
Revit's outline, its two 9.6 ft corner arcs included (#824).
**RE-97 (2026-09-28)** takes a sketch's lines from its Sketch element's
curve list, which names the sketched element, where a line's own owner
reference names another element after an edit. 176 of Snowdon's 199
slabs and all 20 roofs carry an outline, and 102 slabs match Revit's
area within 0.1%
(`reports/element-framing/RE-97-sketch-curve-lists.md`).
**RE-98 (2026-09-28)** gives ceilings their sketch's outline, as roofs
have: 66 of Snowdon's 68 ceilings and all 6 of RE1 Architecture's match
Revit's outline within 0.1% of its area (50 and 5 before)
(`reports/element-framing/RE-98-ceiling-outlines.md`).
**RE-99 (2026-09-28)** cuts shaft openings from the floors, roofs and
ceilings within their height, where the shaft lies strictly inside the
outline. Their sketches' crossing diagonals (the X Revit draws in plan)
are left out. On Snowdon 7 elements take a shaft void, all closer to
Revit's area (`reports/element-framing/RE-99-shaft-openings.md`).
**RE-100 (2026-09-28)** reads a full-circle sketch line (tag `04 00 08 00`,
the arc layout with both angles 0), which gives round shaft 903725 its
outline; 154 of Snowdon's slabs now match Revit's area within 0.1%
(`reports/element-framing/RE-100-sketch-circles.md`).

**RE-207 (2026-10-22)** lists each element's saved parameters in
`rvt-elements` and Python's `decoded_elements()`, on Revit 2023 to 2027
files (#35; 2015 to 2022 since RE-281, experimentally). They are read from the element's native record: its typed
parameter sets, the family slots of its own `FamilyParams` that a
definition resolves, and the native fields read as built-in parameters,
each named by the file's own definition (family, shared, project and
global parameters). A family instance's type adds the type parameters the
instance does not hold. On the BIM4LCA office every parameter named
`<set>.<property>` holds the value Revit's export writes for that property
on the element, 7,606 of 7,606 as built and 7,592 of 7,592 at building
permit, and on the R2025 models a window's `FireRating` is its type's `Fire
Rating`, 18 of 18 (`reports/element-framing/RE-207-element-parameters.md`).
Built-in parameters have no name (the built-in catalog is not shipped,
#255), a family's built-in slots such as a window type's Width are not
resolved, and nothing new is written to IFC.

**RE-256 (2026-10-27)** gives a wall, floor, ceiling and roof its type's
saved parameters after its own in the same lists, through the member its
native record names its type in (`m_WallAttributesId`,
`m_floorAttributesId`, `m_ceilingAttributesId`, `m_roofAttributesId`). On
Snowdon Towers every such value Revit's export writes on the element's type
object is Revit's: type name, assembly code, type mark and walls' fire
rating, 4,297 of the 4,314 Revit writes (the 17 left are stair landings,
which name no type this way)
(`reports/element-framing/RE-256-system-family-type-parameters.md`).

**RE-210 (2026-10-22)** adds `rvt-ifc --revit-property-sets` (#35). It
writes each element's named saved parameters (RE-207) as Revit's exporter
does with "Export Revit property sets", following revit-ifc's
`Release_25.x.x` branch: one set per parameter group, named for the group,
the type's on its type object (or on its elements with `(Type)` added where
it has none), the IFC group left out, one parameter per name in a group,
and each value by its storage and spec, air flows, pressures, powers,
voltages and currents in SI units. Against Revit 2025's export of
Autodesk's Snowdon Towers HVAC with "Export Revit property sets" (measured
locally, not committed), all 28,906 properties written (counted per
element) are in the set of the same name, on the element or type object of
the same Tag, with Revit's value; Revit writes 79,907 more, nearly all
built-in parameters. On Snowdon Architectural, 58,346 of 58,733 are: 369
more are in sets Revit names with `(Type)` on 13 of its type objects, as
the order of its export has it, and 18 are curtain panels' type category,
written before. Against the exports with Revit's property sets of
puzzbobb's 19 Revit 2025 models, which Measure scores in CI (run
41636859789), 2,502 of the 2,646 properties written are Revit's: 126 differ,
as Revit's export writes a window type's sill height and its nested
families' lengths at 1/304.8² of their value, and 18, a stringer type's, are
in no set Revit writes there. On the BIM4LCA office as
built, `FireExit` in `Identity Data` equals Revit's `Pset_DoorCommon` and
`Pset_WindowCommon.FireExit` on 498 of 498 doors and windows
(`reports/element-framing/RE-210-revit-property-sets.md`). Built-in
parameters, enumerations and other specs are left out, and the sets keep
rvt-rs's own GlobalIds.

**RE-215 (2026-10-24)** gives each door and window on Revit 2024 and 2025
whose symbol cuts an outline the `OverallWidth` and `OverallHeight` Revit's
export writes: the extents of the outline the symbol that draws it cuts in
its host, read from that symbol's `m_pCutLoops`. The instance names that
symbol in its `InstanceInfo.m_symbolId`, and it is its own where the
instance's parameters size the family. Equal to Revit's on 138 of 138 doors
and windows on Core Interior (its six windows had `$`), on RE1
Architecture's 4 doors with one and on the R2025 models
(`reports/element-framing/RE-215-symbol-cut-loops.md`). On the BIM4LCA
office, 187 of its 269 doors cut one and have Revit's sizes. **RE-218
(2026-10-25)** sizes a door or window whose symbol cuts no loop by its record
box, along its own X axis and through its height, where its element record
names a wall or a curtain wall: all 269 of the office's windows and 71 more
of its doors now have Revit's `OverallWidth` and `OverallHeight`, and so
does RE1 Architecture's curtain-wall door. Its addendum sizes one whose
record names no wall along its own Y axis, as revit-ifc measures the
height of a family no wall hosts: 266 of the office's 269 doors are then
Revit's, and two are 0.1137 ft wider than Revit's
(`reports/element-framing/RE-218-box-sizes.md`, #810). **RE-318
(2026-11-03)** takes the width of such a door or window from the lines of no
category in its symbol's saved graphics, the edges of the boxes that bound
its forms, hidden forms included, ahead of the record box's: the two wide
doors are wall hatches whose 90 degree type hides the 180 degree leaf Revit's
width includes, and the office's doors and windows then all have Revit's
`OverallWidth` (268 of 268 doors Revit sizes, 269 of 269 windows as built; 268
and 265 at building permit). It fires only where the record box sized the
door or window before: of the other measured models only RE1 Architecture's
curtain-wall door, whose width is unchanged. One building's families, and
what the boxes are is inferred from Revit's widths
(`reports/element-framing/RE-318-symbol-wire-width.md`).

**RE-216 (2026-10-24)** gives a family instance whose element data sits in
two partitions, with transforms that disagree, the copy its current record is
in instead of no transform. That opens Core Interior's last 8 doors as Revit
does: all 132 of its door openings are within 0.001 ft of Revit's
(`reports/element-framing/RE-216-current-instance-copies.md`). Its
addendum does the same for a sketch's lines, its curve list and its slab's
box: the BIM4LCA office's permit-stage slabs 1384047 (a box before) and
1474132 take Revit's outline, 61 of 63 slabs then. Walls keep dropping
disagreeing copies, as their current copy misses 7 of Core Interior's
walls by 0.333 ft (RE-216 §8).

**RE-217 (2026-10-24)** draws a Revit 2025 stair run as its treads and
risers. RE-52's and RE-92's byte anchors are absent on 2025, so its sketch,
riser height and ends are read by member name from its native records
(`StairsRun`, one `StairsTreadInfo` per tread, its `StairsRunType`), with
puzzbobb's `m_endWithRiser` (#357), and its run type from its element
record, as on 2024, which is Revit's on all 75 runs measured. On
puzzbobb's R2025 stair models 13 and 14 each flight now has Revit's volume
and every vertex of Revit's body lies on rvt-rs's, and so do the riserless
run (15), drawn as its separate treads, and the monolithic one (16); the
monolithic spiral run (17) keeps its box
(`reports/element-framing/RE-217-stair-runs-2025.md`). Its addendum draws a
monolithic run with upright risers and no nosing as RE-92 draws one with
slanted risers: model 16's flight then has Revit's volume. A second
addendum cuts a monolithic run's underside where Revit does: at its
`m_extendBelowBase` below its base where it starts at its stair's base, not
at all where it starts on a landing. On the BIM4LCA office 34 of 35 flights
then have Revit's volume, with every vertex of Revit's on rvt-rs's. The
office's spiral run is drawn as its 18 treads, each an extrusion of two
arcs about the spiral's centre and two lines, as Revit writes it, from
the centre, radii and angles its treads' native records hold: Revit's
volume and height (RE-217 §11). A third addendum gives each flight the
predefined type its treads' `m_runType` names, `STRAIGHT` or `SPIRAL`:
Revit's on all 35 office flights of each file and on the 5 R2025 stair
models, where none had one (#323). A fourth gives each stair the
predefined type its runs and landings make, by revit-ifc's rule: Revit's
on all 16 stairs of each office file, where 2 had it, and on the 5 R2025
stair models, and ten more office stair type objects then take Revit's
GlobalId (RE-217 §12).

**RE-214 (2026-10-24)** draws a monolithic stair landing on Revit 2024 as
its slab instead of a box the height of its storey. Its outline is the
lines at its top in its own data, joined straight across where its flights
meet it; its top is `StairsLanding.m_baseElevation`; its thickness is its
landing type's `m_thickness`, the type its own element record names. On
the BIM4LCA office all 21 monolithic landings on each file are slabs with
Revit's top and outline, and the office's slab outlines within 0.01 ft of
Revit's go from 40 to 60 of 63. Revit's underside also follows a flight's
soffit where the flight meets the landing, which is drawn flat: the slabs
are 100.4% to 104.7% of Revit's volume
(`reports/element-framing/RE-214-landing-slabs.md`). Its addendum draws the
office's one landing that is not monolithic, beside its spiral run, as the
tread it is: its native outline loop, arc and all, one run tread thick,
Revit's body to 0.0000022 ft (RE-214 §7).

**RE-220 (2026-10-25)** gives the project, site and building the
GlobalIds the file stores, which Revit's export writes: under the project
information's IfcGUID parameters on BIMcollab's Revit 2015 example, and as
a triple related as revit-ifc's fake ElementIds -16, -15 and -14 relate
them on the BIM4LCA office. All three are Revit's on those three files,
where none were, and the 2015 example's five storeys take their stored
values, Revit's. A file that stores none keeps generated ones: the GUID
Revit derives them from is not found
(`reports/element-framing/RE-220-stored-global-ids.md`).

**RE-221 (2026-10-25)** draws a column whose type is a square hollow
section as that section, rounded corners and all, where it was a solid
box. The section's outside, wall and corner radius are the family's own
parameters `b`, `Ht`, `t` and `kr`, named by their `ParamElemFamily`
records; the profile is written as Revit writes it, an
`IfcArbitraryProfileDefWithVoids` over `IfcIndexedPolyCurve`s. On the
BIM4LCA office all 79 such columns as built and 55 at building permit are
Revit's body, vertices within 0.001 ft and volume within 0.0001. A
rectangular or turned hollow section, or one whose family names its
parameters otherwise, keeps its box
(`reports/element-framing/RE-221-hollow-section-columns.md`).

**RE-222 (2026-10-25)** reads a wall's T joints from its native record: where
another wall meets a wall's end with its middle, the body stops half that
wall's thickness short, as Revit's does on 315 of the BIM4LCA office's 396
such ends against 289 for the join lists. A wall the partition data gives a
line but no orientation (13 of the office's 17 US-1 exterior walls) takes the
native record's word and flip, which equal the partition data's on every wall
that has both, and is drawn from its centreline and in its layers. On the
office as built 428 of 607 walls now end where Revit's do (412 before), and
22 more walls carry Revit's layer set and layer bands; at building permit
377 of 589 (374) and 32 more. Core Interior, RE1 and the R2025 models are
unchanged. A corner rule from the join order and the native line for walls
whose copies disagree were measured and are not used
(`reports/element-framing/RE-222-wall-native-joins.md`).

**RE-223 (2026-10-25)** reads a wall type's core from its native
`CompoundStructure` (`m_numShellLayersExt`, `m_numShellLayersInt`). At a T
joint a layered wall's core now runs through the other wall's layers outside
its core and stops at that core, as Revit's does, where RE-73's layer
priorities stopped it at the face of partition walls whose layers are all
marked structure. On the BIM4LCA office as built 482 of 607 walls end where
Revit's do (428 before), and 422 of 589 at building permit (377); Core
Interior, RE1 and the R2025 models are unchanged
(`reports/element-framing/RE-223-wall-tee-cores.md`).

**RE-322 (2026-11-03)** reads a wall's end at a corner of three or more walls
from the order its native record lists the walls of the join in. Once the
walls that do not overlap it in elevation are left out (the list also names
the walls on other storeys that end at the same point), an earlier wall wins:
a wall stops at the near face of the perpendicular walls listed before it,
meets flush a wall that continues it when that is listed before the next
perpendicular wall, and otherwise runs through the perpendicular walls listed
after it, to faces taken from where each wall's own line ends. It is read
only where it is right on every measured end (a butt join, straight walls at
right angles or straight on, nothing but walls cutting the wall, single-layer
walls or a layered wall stopping against single-layer walls), and names the
wall it stops at or runs through only where one perpendicular wall alone
decides the end. On the office as
built 510 of 607 walls end where Revit's do (482 before), and 430 of 589 at
building permit (422); two single-layer walls that meet at a mitre end on the
line through its corners (R2025 `18_WallJoins`: 9 of 9, from 7). Core
Interior and RE1 are byte for byte as before. Layered corners of two walls,
a layered wall at layered walls and every corner a column cuts were measured
and are not read
(`reports/element-framing/RE-322-wall-corner-order.md`).

The public viewer's demo gallery leads with the two files this section
measures, staged from the deploy workflow's `magnetar-io/revit-test-datasets`
checkout and pinned by sha256 (#257): `Revit_IFC5_Einhoven.rvt` (2023,
913 KB) and `2024_Core_Interior.rvt` (2024, 33.7 MB). Staging refuses a file
whose bytes do not match the catalog hash, and both cards are gated by
Playwright. Viewer File Status lists
recovered storey names, material name samples, and a Parameters row that
counts the Revit parameter values written into the export's property sets. The scene tree groups each
storey's elements by category, type name first, with curtain-wall and
stair parts under their whole. Elements reach their storey from the
Level their record names (RE-59, RE-60): all 970 of Core Interior's
building elements sit on Revit's storey, and ArcWalls bind by elevation.
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
on **80 of 80** exported slabs (#31, RE-25, closed 2026-09-19), and
since RE-95 the 20 shading plates carry theirs too, each equal to
Revit's. Eighty-one classes
remain registered, each through `SimpleDecoder`; `MVP_TYPED_CLASSES` are
consulted by `iter_elements`.

**RE-184 (2026-10-27)** leaves out the elements that do not exist in the
export phase, as Revit's export leaves them out (#328). A project's
`AllProjectPhases.m_phaseIds` lists its phases in order, and with no phase
chosen Revit's export shows the last: an element is exported when it was
created in that phase or before it and not demolished in it or before it,
by its `m_createdPhaseId` and `m_demolishedPhaseId`. Measured against
Revit 2025's exports of two models made for it (a wall created in the last
of three phases is exported, a wall demolished in it is not). On Snowdon
Towers the phases run Legends, Existing, New Construction, so #328's six
slabs, created and demolished in Legends, are left out, and nothing else
is. An export setting that picks another phase is read since RE-326
(`rvt-ifc --phase`), not a view's. Rooms are left out by a rule of their own
(RE-326).

**RE-326 (2026-11-04)** leaves out a room whose own phase is not the export
phase, as Revit's exporter does (revit-ifc `IsRoomInInvalidPhase`). A room's
phase is the `m_phaseId` field of its `RoomElem` record, on Revit 2015 to
2027, and not its created phase, which is unset on every room. The export
phase is the project's last, or the one `rvt-ifc --phase <name|id>` names, and
the status rule of RE-184 for the other elements reads it from the same place.
A room whose phase is unset, unread or not one of the project's is written,
and so is an Area or an MEP space. The diagnostics count the rooms left out in
`skipped` as `room_not_in_export_phase`, one item for each phase they sit in.
The 621 rooms of the 12 models that have a Revit export (Core Interior, RE1
Architecture, the BIM4LCA office as built and at permit and its two terrain
variants, six R2025 room models) are all in the last phase and all in
Revit's export, so none of them changes: in the unfiltered Measure (run
41784044966) 12 of 111 exports move, the `rac_basic_sample_project` saves of
Revit 2016 to 2027, and 99 are byte for byte as before, the held-out models
included. The rule is unmeasured against a
Revit export made with the default setup, because no project in the corpus
has a room outside its last phase and such an export: Autodesk's
`rac_basic_sample_project` has 14 rooms in the second of its three phases, on
all 13 releases, and a public Revit 2018 export of it holds all 14. They are
left out by default and written with `--phase "Working Drawings"`, which
writes 14 of that export's 17 spaces by GlobalId and changes no other entity
(Measure runs 41784415396 and 41784419457). Revit 2015
and earlier write every room, as every element: both phase rules start at
Revit 2016 (#1332). The Python bindings and the viewer follow the default
(`reports/element-framing/RE-326-room-phases.md`).

**RE-328 (2026-11-04)** finds which test of `beam_body` keeps a beam as its
record box when its element data holds one location line (#1247), and draws
one class of them. Every beam of Autodesk's 2024 samples with one line that
the check declines fails the closure of the section solve, bar 2 of Snowdon's
whose line leaves the box by up to 100 ft. 34 of the 52 are level, have no
Cross-Section Rotation and have a record box longer than their line, by at
most a foot; RE-49 took 15 of Snowdon's for beams with oblique end cuts, and
the saved mesh says they are rectangles. Those 34 (17 Snowdon, 9 rst_basic,
8 rac_advanced) are now drawn as the record box's extents along the line,
`BodySource` `partition_beam_box_along_axis`. Its direction and its section
come from the line and the box, and the section across the line is the beam's
own saved mesh's (Revit's tessellation, not an IFC export; there is no Revit
IFC of a 2024 structural model) on all 25 whose mesh decodes whole. **Its
length is the box's, not the beam's**: the mesh's ends are the body's on 16 of
the 25 and end inside it on 9, by more than 0.01 ft and at most 1 ft (the
record box's extent along the line runs past the mesh on 21 of the 25, by up
to 1 ft), so it can run past the beam at its ends. The line must be level
and within 25 degrees of a model axis, the only angles measured. The 9
rst_basic beams have no decodable mesh and sit on a model axis, where the
body is their record box. It gets no
`Qto_BeamBaseQuantities.Length` (so no `Span`, `Slope` or `Roll`) and no I
section; `AxisLength` is the line's length and `BodyLength` the body's. 16
sloped, rolled beams of rst_basic and the 2 Snowdon beams keep their box. The
beams that were drawn along their line are unchanged, and the BIM4LCA office
is byte for byte the same. The probe for a private file is
`probe_re328_beam_rejections`
(`reports/element-framing/RE-328-beam-rejections.md`).

**RE-281 (2026-10-30)** reads the project's phases and each element's saved
parameters on Revit 2015 to 2022, experimentally, and the phase order on
Revit 2023, which `native_element::decode_base` refused with every 2015 to
2023 record because it took `Identifier` for the 8-byte `m_id64` of Revit
2024: before 2024 it is a 4-byte `m_id`. On Autodesk's 33 sample projects of
2015 to 2023 every file lists its phases (two or three) and no element is
left out of the last; on BIMcollab's Revit 2015 example 8 are, none of them
written. So no export moves. Parameters, read by name, hold the values of the same
project's Revit 2023 copy (932,645 equal, 243 different, over 23 projects).
Every other native reader (types, stairs, symbol cuts, mullion hosts, ports,
saved meshes) stays on 2023 to 2027, and a record that needs the
extensible-storage catalogue before Revit 2021 or is one of 2015 to 2017's
`VarSketch` records is refused. BIMcollab's 16 footings name no Level: their
Element base, `-1001352` and `-1002062` are all unset, as are the 15
columns', which Revit still puts on a storey (#879;
`reports/element-framing/RE-281-native-admission.md`).

**RE-283 (2026-10-30)** reads a wall's type Function and the project and
shared parameters named for `IsExternal` or for a standard property from the
saved parameters on Revit 2015 to 2023, experimentally, so a wall's
`IsExternal` and any parameter named for a property are written there as on
2024 and 2025 (RE-193, RE-194). The Function is equal to the same project's
later copy on 1,982 of 1,983 walls (24 pairs of 2015 to 2022 against 2023; the
other is an edit between the saves). Against Revit's exports: BIMcollab's 114
walls' `IsExternal` and `LoadBearing`, which two parameters on the walls
give, and modelo_bim's 4 walls' `IsExternal`, which the Function gives, are
all Revit's value where they were missing. 36 of 94 exports of the unfiltered
run move, all files of Revit 2015 to 2023
(`reports/element-framing/RE-283-wall-function-and-named-parameters-pre2024.md`).

**RE-284 (2026-10-30)** writes each wall's `ExtendToStructure` on Revit 2015 to
2023, experimentally, from the same flag of its native record that RE-228
reads on 2024 and 2025: a `JoinToRoofGStep` with `m_baseOrTop` 1. Against the
same project saved by a later release it is equal on all 1,983 walls both
files hold (24 pairs of 2015 to 2022 against 2023) and on the 249 walls of the
2023 samples against their 2024 copies. Revit's own exports agree on the 114
walls of BIMcollab's Revit 2015 example and the 4 of modelo_bim (a Revit 2023
file), all false: the value true is measured against an export on 2024 and
2025 only. 36 of 94 exports of the unfiltered run move, all of files of
Revit 2015 to 2023, and every file of 2024 to 2027 is identical
(`reports/element-framing/RE-284-wall-top-join-pre2024.md`).

**RE-285 (2026-10-30)** writes `LoadBearing`, true, on every structural column
and structural framing element, as Revit's exporter gives it by category,
unless the element's own parameter named for it says otherwise; on Revit 2015
to 2023, experimentally, it also writes their `IsExternal`, not external, the
calculation's last step. Against Revit's exports: BIMcollab's Revit 2015
example 91 of 91 beams and 15 of 15 columns for each property, modelo_bim (a
Revit 2023 file) 8 of 8 beams and columns for `IsExternal` and 9 of 9 for
`LoadBearing`, and the BIM4LCA office as built all 289 columns, none
different. A parameter named for it on the element's type is not read. 58 of
94 exports of the unfiltered run move: every file with structural columns or
framing (`reports/element-framing/RE-285-structural-load-bearing.md`).

**RE-327 (2026-11-04)** rebuilds a family's extrusions from their own records:
the sketch's `CurveElem` lines and arcs (in the family's coordinates), chained
into loops, placed in the `SketchPlane`'s frame and swept between the
extrusion's two `Double` parameters. Against the mesh Revit saved for each
form, the 24 extrusions of the family corpus (12 releases) match within
0.001 ft and so do all 6,172 extrusions of Autodesk's 58 sample projects
(Revit 2015 to 2027) whose saved mesh decodes whole (by the report's §4
criterion: volume within 1%, bounding box, vertices and surface samples
within 0.01 ft): 6,093 with the sketch
plane's normal as its matrix gives it, and 79, a down-facing plane that is
not flipped and whose extrusion reads `m_alwaysRefPlaneNorm` false, with the
normal reversed (a rule found on those 79). Ellipses (137 forms), two open
sketches (26), sweeps, revolutions, blends and voids' cuts are not drawn. The 2014 family of Discussion #112 is outside
the native releases, so its 85 of 86 is not reproduced. The 79 are
reversed by default, a rule found on them and not held out; every instance in
an export is checked against its own mesh. `rvt-ifc --family-extrusions`
(opt-in; the default export is unchanged, hash for hash on 111 of 111 models) draws an
instance from its family's extrusions where they reproduce its saved mesh.
On Core Interior, RE1 and the BIM4LCA office files the first run, which drew
arcs as arcs, replaced 335 bodies; of the 251 the body scorer matched, 250 had
Revit's vertices within 0.001 ft of their surface and one did not: Revit 26.2's
IFC2X3 export draws that circle (0.2292 ft) as an `IfcCircleProfileDef`, which
IfcOpenShell cuts into 27 chords, and two half arcs of it come out as 28, so
no vertex coincides (a count mismatch, reproduced and measured in
`reports/element-framing/RE-327-family-extrusions.md` §11). Revit 24.0.5's
IFC4 export of the office writes a circle as half arcs. IfcOpenShell's default
tessellation of the two forms of a circle has the same chords up to 0.164 ft
and at 0.3281 ft and one chord more or fewer at 0.2292, 0.4921, 0.9843 and
1.9685 ft; over 0.137 ft a chord's own deviation passes the 0.001 ft bar, so a
difference in the count or in where the circle starts misses it, for a partial
arc as for a whole circle. No one form meets both exporters, and nothing in the
`.rvt` says which will read it. An arc of 0.12 ft in radius or less is written
as the arc, and an instance with an arc over 0.12 ft in radius keeps its mesh in
every release, which meets the bar against both. The shipped rule replaces 66
of the 335 (Core Interior 37, each office file 9, RE1 Architecture 7,
Electrical 2, Mechanical 1, Plumbing 1) and keeps 269 for an arc (the office
files 120 each, the terrain files 11 each, RE1 Architecture 1, Electrical 6);
every combined row of `mesh_bodies_vs_ifc` equals the `--saved-meshes`
baseline's, and the 2 replaced bodies with an arc, circles of 0.12 ft or less
in RE1's IFC2X3 export, each hold the bar and coincide with Revit's at 1e-5 ft.
No partial arc and no arc of the IFC4 dialect is among the 66: the arcs the rule
draws are thinly measured. True arcs for the larger ones come back if a finer
tessellation of the scorers changes the bar (#1355, a decision for Griffin). The flag reads Revit 2024 to 2027
(measured on 2024 and 2025); an earlier file has no instance transform that
reads, so nothing is replaced (`reports/element-framing/RE-327-family-extrusions.md`
§9).

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
normalised (`FILE_NAME` and the `IFCOWNERHISTORY` epoch). This comparison
was made before `rvt-ifc` honoured `SOURCE_DATE_EPOCH`; with it set, those
two fields are now pinned and two exports are byte-identical, as
[`tests/reproducible_outputs.rs`](../tests/reproducible_outputs.rs) checks.

In the browser the same change takes the 33.7 MB Core Interior demo from
about 28 s to **about 3 s** of decode, measured live on the deployed
site (6.7 s including the 32 MB download). The viewer's loading card
opened with an "about N s" hint from that measurement —
`DECODE_BYTES_PER_SECOND` is 11 MB/s since #271, up from the 1.15 MB/s
the pre-#266 figure implied. It is a laptop's figure, and since #1195 the
card shows the decoder's own progress as soon as it reports (see
[`viewer-api.md`](viewer-api.md#decode-progress)).

## Capability Matrix

The capability matrix is [`support-matrix.json`](support-matrix.json): every
capability, its status ceiling, its evidence and what it means for a user,
checked by `tests/support_matrix.rs`. `rvt-capabilities --matrix -f text`
prints it from any installed build.

## Roadmap Position

- **v0.14.0** (2026-11-04) is the latest release: crates.io (`rvt` and,
  new, `rvt-rs-mcp`), the `rvt` wheels on PyPI, GitHub Release archives for
  Linux, macOS and Windows, and the container image, each checked from its
  published artifacts ([`install.md`](install.md)); docs.rs had queued its
  build when they were checked. It adds `rvt-rs-mcp`, a local, read-only MCP
  server, and one export function every surface builds from; rooms follow the
  export phase, which `rvt-ifc`, Python, the viewer and `rvt-schedule` can
  name; Revit 2024 Areas get a body and proven Revit 2015 to 2023 Areas are
  written; the openings rectangular wall openings cut are written; walls a
  floor covers in part are drawn stepped; conduits, conduit fittings and
  gutters are exported; and every one of the 68 exports timed is faster than
  0.13.0's. `[0.14.0]` in [`CHANGELOG.md`](../CHANGELOG.md) gives what each
  change measures and what it does not.
- **v0.13.0** (2026-11-03): every `IfcSpace` says where its outline came
  from; rooms on Revit 2026 and 2027 take their outline, rooms whose stored
  solid the walk did not close are read from its faces, and family instances
  Revit's export leaves out are left out of the spaces. A Revit 2018 project
  that exported nothing exports 1,136 of Revit's 1,321 elements and its 73
  rooms, and Revit 2016 to 2023 exports leave out elements outside the export
  phase and in non-primary design options. `[0.13.0]` in
  [`CHANGELOG.md`](../CHANGELOG.md) gives what each change measures and what
  it does not.
- **v0.12.2** (2026-11-03): in the default export it draws more doors from the meshes Revit
  saved for them (the BIM4LCA office as built 269, against 48 with 0.12.1,
  265 exact to 0.001 ft against Revit's export), and it no longer calls a
  Revit 2008 to 2014 file an unsupported version in the viewer (#1247,
  #1266). With `--saved-meshes` it draws more bodies, not all of them
  Revit's (RE1 Plumbing's new pipe fittings are 1.44 to 1.47 of Revit's
  volume; RE-311 found that figure the scorer's, taken about the file's
  origin, and the tees 1.14 about their own centre). Its door-type change measures on no file here. Not every case
  #1247 reported is fixed; `[0.12.2]` in [`CHANGELOG.md`](../CHANGELOG.md)
  says which are not.
- **v0.12.1** (2026-11-01) makes exports of Revit 2015 to
  2023 files faster than 0.12.0 made them (2015 to 2022 projects take 0.84
  to 0.97 of the time, 2023's 0.89 to 0.96), each the same as 0.12.0's but
  for the version its IFC header records (#1237). They still take 1.6 to 2.9
  times as long as with 0.11.0 (2023's 1.3 to 1.9 times).
- **v0.12.0** (2026-10-31) carries everything under `[0.12.0]` in
  [`CHANGELOG.md`](../CHANGELOG.md):
  - more of the base quantities and common property values Revit's IFC4
    export writes, each where the model's records decide it, and none where
    Revit's own exporters disagree. On the BIM4LCA office as built, counting
    the objects rvt-rs writes and leaving out what the census counts apart,
    29,262 of 39,388 quantities and, with `--revit-set-placement`, 45,235 of
    45,525 property values are equal, 1 different each;
  - native records read on Revit 2015 to 2022, saved parameters
    experimentally;
  - walls' and structural members' flags on Revit 2015 to 2023,
    experimentally;
  - native-record readers kept under the file's own limits (#1075, not yet
    measured on a model over them);
  - Python and the viewer drawing doors and windows from their saved
    meshes;
  - the public API nothing used removed.

  Exports of Revit 2015 to 2023 files take 1.6 to 3.9 times as long as
  with 0.11.0 (#1237). Its "Upgrading from 0.11" section lists what a 0.11
  user has to change.
- **v0.11.0** (2026-10-30) is the release before. A model over the reader's
  resource limits is reported, and `rvt-ifc` and `rvt-gltf` exit with
  status 3 instead of writing an empty model, with `--large-model` and a
  viewer retry (#1075). Files that kept an older release's schema are read
  as that release (#421). Neither fix is measured yet on the reporter's
  private files. It also brought:
  - elements outside the export phase left out (#328);
  - doors and windows drawn from their saved meshes by default in `rvt-ifc`;
  - Revit 2008 to 2014 exported experimentally.
- `main` has everything v0.14.0 has, and is ahead of it by the fragments
  under [`changelog.d/`](../changelog.d/README.md).

Open work is tracked as GitHub issues; [`docs/archive/TODO.md`](archive/TODO.md) is
the older task backlog.

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
