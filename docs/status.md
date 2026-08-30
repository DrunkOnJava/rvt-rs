# Project Status

Last reviewed: 2026-08-30

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
`Level` (elevation + name), `Material` / `Room` (display-name candidates),
`Floor` (ArcWall-excluded plan loops, only on files where no element
record decodes), and — on Revit 2024 —
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
plan-loop boundary annotations they replace stand down on files where
records decode, so exactly one `IFCSLAB` is emitted per exported id.
IFC export maps recovered Levels → storeys (with their real Revit
names and elevations on Revit 2024, #218), Rooms → spaces, and
Walls / Doors / Windows / Columns / Floors / BuildingPads →
`IfcWall` / `IfcDoor` / `IfcWindow` / `IfcColumn` / `IfcSlab` (or
`IfcShadingDevice` when overridden) with placement and a
bounding-box extrusion (envelope, not a recovered family profile,
wall location curve or floor boundary polygon; base/top Level
ElementId binding still open), and
Material display names → `IfcMaterial`.
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
Containment is unchanged and still an exact elevation match, so the wider
storey set binds more: **801 of 872** building elements now land in a
specific storey (was 794 with 11 storeys) — all 256 columns, all 132
doors, 359 of 360 walls (was 355) and 54 of 100 record-backed plates
(was 51 — `IFCSLAB` 44 of 80, `IFCSHADINGDEVICE` 10 of 20). Plates still
bind by their record **top** face, since Revit hangs a floor below the
level that hosts it. The 6 windows still bind to nothing: a window
record's base is its sill height, never a storey elevation, so the wider
set does not help them. The 46 plates that remain unbound sit 0.1667 ft
below their level at the structural-slab / architectural-topping
interface (#219) — a thickness question, not an elevation-set one.
The #213 column-derived path (`STOREY_ELEVATION_SOURCE_TYPES` =
`IFCCOLUMN`) survives as the fallback for files with no recoverable Level
records; its measurement stands unchanged, including the RE-22 finding
that admitting slab tops as an elevation source would buy −40 ft and
185.5 ft at the cost of 13 false elevations.
Viewer File Status lists
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
slab set (every record-backed slab carries its ElementId) and slab
extrusion thickness is measured from the record bbox; the slab
*profile* is still the bounding-box rectangle rather than the
recovered boundary polygon (#31). Eighty-one per-class
decoder structs remain registered; `MVP_TYPED_CLASSES` are consulted by
`iter_elements`.

## Capability Matrix

| Capability | Status | Evidence | User impact |
|---|---|---|---|
| Open `.rvt`, `.rfa`, `.rte`, `.rft` CFB containers | Full | `reader`, CI matrix | Files can be inspected without Revit. |
| Decode Revit truncated-gzip streams (gated checksum-page strip on Partitions/Global, #151) | Full | `compression`, `checksum_page_framing`, fuzz | Internal streams can be read safely; Formats/Latest stays ungated by default — multipage integrity uncertain (`RVT_FORMATS_MULTIPAGE_UNVERIFIED`). |
| Extract metadata, PartAtom XML, preview PNG | Full | `basic_file_info`, `part_atom`, tests | Users can identify and audit files. |
| Parse `Formats/Latest` schema | Full | 100 percent field classification over 2016-2026 family corpus; multipage integrity diagnostics in inspect/export/viewer | Developers can inspect class and field structure; Formats multipage integrity uncertain while strip stays disabled. |
| Read document-level ADocument data | Partial | Reliable on newer samples; older/project bands need more corpus proof | Good for diagnostics, not complete model extraction. |
| Decode typed elements from real project files | **Partial** | Production `iter_elements`: ArcWall (2023) + partition MVP Levels/Materials/Rooms (+ Floor plan-loops only where no element records decode) + 2024 ArcWallRectOpening (ElemTable-confirmed related ids) + 2024 partition element records for `OST_Walls` / `OST_Doors` / `OST_Windows` / `OST_Columns` / `OST_Floors` / `OST_BuildingPad` (360/132/6/256/80 IFCSLAB + 20 IFCSHADINGDEVICE on Core Interior, exact ElementId sets, cross-witness gated, #204/#211/#212, RE-21/RE-22); HostObjAttr filtered; RE-19 negatives intact: no opening-index Door/Window discriminator, no schema-field Wall on magnetar corpora | Full model conversion is not ready; six categories on one Revit 2024 edge match Revit's exporter exactly, spaces and materials do not. |
| Typed decoder structs | Partial | `elements::all_decoders()` registers **81** decoders; `MVP_TYPED_CLASSES` consulted by `iter_elements`; ArcWall uses a separate partition decoder | Library building blocks plus production MVP/ArcWall path. |
| IFC4 writer | Partial | Synthetic fixtures validate in IfcOpenShell; every emitted instance carries the full IFC4 attribute list its type declares, gated per instance against the EXPRESS schema by `tools/ci/ifc_schema_arity.py` (#214); 2023 Einhoven ArcWall `IfcWall` + partition Level storeys / Floor boundary `IfcSlab` / Room `IfcSpace` / 2024 `IfcWall` + `IfcDoor` + `IfcWindow` + `IfcColumn` + `IfcSlab` + `IfcShadingDevice` with placement + bounding-box extrusion + named Revit Level storeys with measured containment (#218/#213/#212) + measured slab thickness (#212) + the `IfcOpeningElement` / `IfcRelVoidsElement` / `IfcRelFillsElement` chain that voids all 138 doors and windows out of their host wall (#222, exact pair-set match) / Material display names; slab boundary polygon still open; `rvt-ifc --diagnostics` JSON readiness sidecar; `--mode` gates scaffold/typed/geometry/strict | Correct writer path exists, but real-file typed inputs are incomplete / unsolved. |
| Browser viewer | Partial | GitHub Pages deployment, no-network WASM import gate, File Status shows production class counts + storey/material totals, supported-profile matrix | Useful for local inspection; geometry reflects decoded coverage. |
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
