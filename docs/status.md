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
GlobalId, `IsExternal` (RE-193), curtain mullions' `Span` (RE-226) and, where the boxes of their wall's mullions say which ends Revit's solid keeps, `NetSurfaceArea` (RE-268, RE-400, RE-403, RE-432, RE-433, RE-436, RE-438, RE-439, RE-441), walls'
`ExtendToStructure` (RE-228), railings' `Height` and `Reference` (RE-231, RE-431), stair flights' treads, nosing and run type
and landings' type (RE-92) and the parameters named for a standard property
set's property (RE-194), and a body that is exact where the element's
data is decoded and its bounding box where it is not. It writes that as IFC4,
glTF, plan SVG and CSV, from the CLIs, Python, the zero-upload browser
viewer, and a local, read-only MCP server (`rvt-rs-mcp`, [`docs/mcp.md`](mcp.md))
that answers an agent's questions from the model the IFC export writes. On Revit 2023 projects it reads each element's ElementId, category
and bounding box (RE-81), curtain walls with their parts under them on 2015 to 2023 (RE-366), those parts with their type and family, and the mullions and panels their material (RE-367, RE-377), walls at an angle along their lines (on 2015 to 2020 only where the record box closes on the line's rectangle) and joined by their join lists on 2015 to 2023 (RE-364), or cut back at their joins (RE-120), with their `Height` where nothing is joined to them or cuts them, though not their areas or volumes (RE-364 section 9), beams at their columns
(RE-122) and elsewhere along their location lines on 2015 to 2023, read on Autodesk's samples only (RE-362: none of BIMcollab's 91 beams holds a line, #1514), steel beams as their type's I section on 2017 to 2023 (RE-365), door and
window hosts (RE-85), pipes' types, cylinders and ports on 2015 to 2027 (RE-370), ducts' types, lines, sections and ports and ports' systems on 2015 to 2027 (RE-371), each element's Phase Created on 2015 to 2023 (RE-368), each element's Level parameters on 2015 to 2023 (RE-369), room outlines
(RE-102) with their numbers and names (RE-117, which names every
release's rooms), Levels with the storey each element sits on (RE-107;
beams by the Level at their top, RE-119), and
family instances' family and type names, with their IFC type objects
(RE-109, RE-110), walls', floors' and roofs' types and layers
(RE-111, RE-112), family instances' materials (RE-113), and walls' and
floors' material layer sets (RE-114, RE-115), with each material's shading
colour (RE-116). On Revit 2026 projects, experimentally since 0.5.0, it reads
typed elements, storeys, rooms, doors and windows, type names and layer sets,
measured on one project against Revit's own export (RE-124), and on two held-out projects, counts only. On projects
saved by Revit 2015 to 2022 and by Revit 2027, experimentally since 0.6.0, it
exports their elements, measured against the same sample projects saved by
Revit 2019 (for 2015 to 2018, RE-181), 2023 (for 2019 to 2022, RE-180) and
2026 (for 2027, RE-179) rather than against a Revit export of them, except two
projects against Revit's own export of their release: BIMcollab's 2015
structural example (RE-198) and Vitoria's 2018 `ALD26 Before` (RE-315, RE-319);
2015's `rst_basic` is another model than its 2019 copy, so it is not measured.
On 2025, 2026 and 2027 (RE-341, RE-345) beams are drawn along their axes as on
2024, tread landings (not monolithic ones, which stay boxes) as on 2024, and the
sketch lines' exact ends, arcs and Sketch curve lists are read, so
sketch-driven floors, ceilings and slabs take their outline from their lines,
arcs included, as on 2024. Shaft openings are cut from floors, ceilings and
roofs on Revit 2024 only (RE-345, Griffin 2026-11-05): the only shafts measured
on 2025 cut nothing, so the check against the same project's 2024 save could not
catch a bad cut, and on 2025 to 2027 those stay whole and the export counts the
shafts it did not cut (`shaft_opening_not_cut`); RE-341 had turned the cut on
for 2026 and 2027, which this turns off again.
Stair runs on 2026 and 2027 are drawn as their steps and treads from their
native records as on 2025, with the stair property sets and quantities read
from them. A roof on 2023 to 2027 is drawn from its saved mesh where that
decodes whole (#1191, and #1612 for one drawn as the level block of its sketch
outline), and keeps the record-box solve alone where it does not
(RE-341 section 5): without its slope, so a sloped roof can still be drawn as its
outline at the box's full height, a level block. A roof's saved mesh is scored
against Revit's export on 15 roofs that #1612 and RE-401 draw from it, each within 0.4% of
Revit's volume (Snowdon Towers Architectural's 14, local only, 4 of them Revit's to 1e-6 since a
`GGTag` is walked as a group, and the T/U roof of puzzbobb's Revit 2025 models, RE-389,
RE-401), where the level blocks of 8 of them were 40% or more too large. On Autodesk's samples, which have no Revit export, the mesh of
one roof of the 2026 and 2027 saves is not the 2025 save's body and another keeps
its box, which the same-project gate names (`ROOFS_NOT_THE_2025_BODY`, Griffin
2026-11-05 and 2026-11-09). Such a roof no longer cuts the top
of the walls under it unless its record box is exactly its type's layers and
its outline is read (RE-351): the walls of `rac_basic` that its 2024 (15),
2026 and 2027 (16) saves cut at roof 243274, up to 3.1865 m lower than its
2025 save draws them (Measure run 42106436497), are drawn to their record
boxes as the 2025 save draws them (on the 2026 and 2027 saves all 16 are the
2025 save's bodies, Measure run 42554270370, and on the held-out AA house, Revit
26.4's IFC4X3 export and the one Revit export the rule changes, the walls with
Revit's base and top go from 43 to 56 of 59, counts only, run 42556745265), which is a
smaller error and not Revit's body: Revit's walls attach to the gable's sloped
underside. Only roof 243274 exercises the roof-cover path anywhere, and no
wall is cut by a plate roof, so the branch that keeps a plate roof as a cutter
is unmeasured. No beam, landing, stair run or
sketch outline there is scored against a Revit export: each is held to
the same Autodesk project's earlier save (`tools/re/bodies_across_saves.py`,
which Measure runs and fails on), and that cannot catch an error the earlier
save shares; it compares bodies, and since RE-346 the cuts, ports and the properties
and quantities that change adds, not the others. No 2026 or
2027 held-out model holds a beam, so the beam reader has run only on two
Autodesk samples saved from earlier releases, never on a natively authored
2026 beam. **On 2025 no Revit export checks a beam, tread landing or arc
either** (RE-345): the 28 Revit 2025 models read (puzzbobb's 24 and RE1's four)
hold 0 beams and 0 tread landings, and the 25 with Revit's own 2025 exports
(puzzbobb's 24 and RE1 Architecture) hold 0 shaft openings and 0 floors,
ceilings or roofs whose sketch holds an arc, and their exports are byte for
byte `main`'s. The 2025 saves of `rac_advanced` and
`rst_basic` are held to their 2024 saves: 391 beams and 5 tread landings equal,
and of the 7 elements whose sketch holds an arc, 2 (a floor and a roof of
rac_advanced) are compared as drawn outlines and equal, the ceiling and a
railing being record boxes in both saves and 3 more having no body. None of
the 19 held-out and spent models changes. That check cannot catch an error the
2024 saves share (Measure run 42091457764, head 830a4d94). The floor of
rac_advanced, drawn with chords on the 2025 save before (24 vertices), has the
170 vertices of the 2024, 2026 and 2027 saves. One stair flight is scored against Revit's own 2026 and 2027
exports. Since RE-346 a pipe there is placed by RE-131's connector entries
first, as on 2025 (RE-370 draws the rest from its record), family
instances' ports are placed, walls are cut to their doors' and windows'
outlines, and `IsExternal` (on walls and the curtain walls, members and plates
on them since #1860), railing heights and lengths, stair names, rooms'
height and net volume and the pipe and duct quantities are written, as on 2025,
except a door's or window's Area on 2027 and, on 2026 and 2027, the `ObjectType`
of every class, which Revit's own exports of those releases do not write (418
typed elements of nine classes on the five held-out pairs, counts only, and
Revit writes none on any of them; #1472; Griffin, 2026-11-09, chose to leave it out
from 2026 saves only). A save of an earlier release that Revit 2026 or later
exports has no `ObjectType` in Revit's export either, and rvt-rs writes it there:
nothing in the file says which exporter will read it. On 2027 the occurrence of
a stair, a flight, a slab, a member, furniture, a door, a window, a wall, a
railing and a roof is written without a `PredefinedType` as well: Revit's export
of the two held-out 2027 pairs writes none on 71 elements of those classes (counts
only), where its export of the same projects saved by 2024 to 2026 writes one;
its proxies (`NOTDEFINED`), openings, spaces and every type object keep theirs,
and a class the pairs do not hold (columns, beams, coverings, curtain walls,
plates, MEP terminals ...) is not measured on 2027 (#1692). A typed beam, column,
plate and covering is left out as well, taken from the exporter's source and not
from a measured export: no Revit 2027 export holds one of them, and the 2027
samples hold beams, columns, plates and coverings that rvt-rs writes a value on
(#1716). A curtain wall, a footing, a ramp, a shading device and the MEP
terminals keep their value. The exporter's
public source writes an occurrence's `PredefinedType` only where the element has no type
object when it is created, and only from its tag `v27.3.0.12` (not in `v27.0.1.1` and
earlier), so what decides is the exporter's build and not the save's release: a Revit 2027
install with an older exporter still writes the values, and the file does not say which
exporter read it. The
viewer's element panel, the schedule CSV and the MCP server report what the export
writes, so they say none for those elements as well.
Each
is held to the same Autodesk project's 2025 save, on the MEP samples too
(`tools/re/bodies_across_saves.py`, which Measure fails on a difference), and
not to a Revit export: no held-out model holds a pipe, a duct or a port, and
that cannot catch an error the 2025 save shares. In Measure run 42545885153
(against `main` at 1c088750, unfiltered, the same-project gates passing on 15
samples) no body of the MEP samples' 2026 and 2027 saves differs from the 2025
save's (36 walls on each did on `main`), and their placed ports are the 2025 save's
(6,967 and 7,054, against 1,944 and 1,956 on `main`), and since RE-373 (#1489) the
doors of `rac_basic` 2027 have their Width and Height too. Only 2026 and 2027 exports move, and on the held-out 2026
and 2027 pairs every value added is equal. Sloped
roofs, spiral runs, a wall's profile openings, IFC export
overrides and the opening index stay unread there.
Conduits, conduit fittings and gutters export on every release the exporter
reads and are measured against Revit's own export on `ALD26 Before` only (all
178 it holds are Revit's elements, RE-319); Autodesk's MEP samples and Snowdon
Towers Electrical export them too, counted and not scored, and cable trays do
not export, since no Revit export holds one. On
projects saved by Revit 2011 to 2014, experimentally and outside the 1.0
range (2024 to 2027), it exports their elements too, measured on a local copy
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
named where the file defines them (RE-207), the browser viewer's info panel lists them for a selected element, and `rvt-ifc --revit-property-sets`
writes the named ones in Revit's own property sets (RE-210). With
`--revit-set-placement` it also writes properties where Revit's own exporter
does and IFC4's definitions do not, such as a room's
`Pset_SpaceCommon.FloorCovering`; by default every property is in the set
IFC4 gives (RE-235). The flag also keeps the material Revit associates with
an `IfcSpace`, which IFC4's `IfcRelAssociatesMaterial.AllowedElements` does
not allow, so that output breaks the rule as Revit's own export does; the
default has no material on a space (#1430). It writes IFC4's base quantities by default, as Revit's
built-in IFC4 export setups do, and `--no-base-quantities` leaves them out,
as Revit's exporter does with "Export base quantities" off (RE-242).

It is not a converter for arbitrary models: most parameters, family
geometry (every family instance is drawn from the mesh Revit saved for its
display by default on Revit 2023 to 2027, where the whole mesh decodes, and so,
on Revit 2024 to 2027, are the walls that embed a curtain wall where Revit
raised their base, RE-340; a mesh with a face that does not decode keeps its
box, RE-224, which leaves 1 of Snowdon Towers Electrical's 2,428 elements a box, RE-385, an instance visible in the normal draw and forbidden the targets 10 is drawn, which draws the 16 art-glass pendants of that file, 332 bodies of Snowdon Towers Architectural, 301 of them within 0.01 ft and 2% of Revit's export (the 16 pendants read 1.04 to 1.10 of its volume, the 15 sinks sit 0.0106 ft from its surface), and 54 window trims and muntins of six Revit 2025 models, which match it at volume 1.0000, RE-408, and a mesh is not always Revit's whole body, RE-78), hip roofs over a T or U footprint, roofs sloped by several slope arrows of the configuration of puzzbobb's U, where Revit slopes the two long outer walls along the arms only (RE-307; no other roof of the configuration is measured), and roofs by extrusion outside Revit 2025, the top of
walls whose layers Revit ends at different heights or cuts at a height no join names (#1067), the boundaries of
the rooms whose stored solid still does not close (RE-316 §4) and opening
profiles are not decoded, and rvt-rs does not edit Revit model data.

**Rooms.** Each `IfcSpace` says in `RvtElementRecordGeometry.SpaceOutlineSource` whether its outline was read from the room's stored solid (`stored_solid`), is the record's bounding box (`record_box`), was drawn from a saved mesh or a family's body, is an Area's region among its boundary lines (`area_boundary`, RE-317; `area_boundary_joined` where its outline needed a boundary line carried on to the line it joins, or a connector, RE-330) or the loop of Revit's own plan topology for it (`area_plan_topology`, RE-333; `area_plan_topology_unchecked` on Revit 2025 to 2027 and on the Areas drawn from lines whose partition copies disagree, #1434, whose Areas nothing outside the file checks), or has no body (an Area whose boundary is not proven), and the diagnostics count the boxes (`exported.bounding_box_spaces`) and, by reason, the Areas that have no body (`exported.area_body_refusals`, RE-317; `area_regions_disagree` where the plan topology and the lines' arrangement give different regions, none on a public model in Measure run 42017400629 on `5008247d`). Areas are read on Revit 2015 to 2027 (RE-324, RE-333), the region first the loop of Revit's own plan topology where that is read and passes its checks (RE-333: the 25 Areas of `rac_advanced`'s 2020 save are all within 0.1% in area and 0.01 ft in outline of the three.js IFC2X3 export, none extra and none missing, which nothing of the topology was read off; on `5008247d` the racadv pairs have 116 spaces in both files and none only in either, Measure run 42017400629; the 0.1% and 0.01 ft figures are from `e9124b3a`), else the lines' arrangement with its joins; an Area drawn from the topology says `area_plan_topology` (`area_plan_topology_unchecked` on Revit 2025 to 2027 and on `rac_advanced`'s 2021 save, whose lines have partition copies that disagree and whose 25 Areas are drawn from the topology alone, #1434), so `area_boundary` and `area_boundary_joined` are written only where the topology is not read; those of 2015 to 2023 are written only where their body is proven, counted otherwise in `skipped` as `area_without_proven_body`, and checked against the same project's 2024 save on Autodesk's samples and, for the 16 Areas of `rac_advanced` it draws, against an IFC2X3 export of that project from the three.js repository, by a build of Revit 2021's exporter whose string ends "IU alternativa 21.0.0.383" (publisher not established), showing the 2015 to 2020 state of the project (RE-330): the only export of a 2015 to 2023 Area there is, and not one of Revit's IFC4 exports. Measure scores every space's plan area and outline against Revit's export, and CI gates Core Interior (116 of 116 rooms) and RE1 Architecture (11 of 11). A room whose stored solid the exact walk did not close is read again from the faces its solid lists, up to the solid's own end, and says so: `stored_solid_faces` where the faces' edges close as the walk reads them (9 of the BIM4LCA office's 13 such rooms as built, all at Revit's area to 0.1%) or `stored_solid_approximate` where a column overlaps a wall, an end of one wall edge lies on another, or the outline falls up to 0.02 ft short of the box (4 as built and 11 of the 12 at building permit: 10 of the 15 at Revit's floor area to 0.1%, the 5 others, four rooms, 0.45% to 0.68% under it). On Revit 2024, a room neither reads is read from its solid's edges as the outline of its floor, `stored_solid_edges`, where the solid is a prism of that floor up to 4 ft above it (#90, RE-374: the BIM4LCA office's room 3095831 at building permit, 1864.737 ft² against the 1864.74 of Revit's IFC4 export, and Snowdon Towers' room 830902, 304.059 ft², within 0.001 ft² of Revit's in the model's later VIM edition; Snowdon Towers' room 2181757, whose area Revit takes 4 ft up, keeps its box); the diagnostics count them (`exported.edge_outline_spaces`, `exported.resolved_outline_spaces`, `exported.approximate_outline_spaces`). The scorer's upward-face area reads one of them (1671995) as 323% over and RE-313's 66% room as 66% off; by Revit's floor both are within 0.5% and 0.1%. A room's outline takes off the round columns its stored solid holds (RE-334): all 91 rooms of `rac_advanced`'s 2015 to 2020 saves are within 0.1% in area and 0.01 ft in outline of the IFC2X3 export of that project (before, 42 of 85 were, and 6 kept their box); Revit's public IFC4 export of `rac_basic` draws the one column of its room 101 as a void too (the reader moves that room from 1.710% to 0.628% over Revit's area), and none of the 53 other models measured (the office, Core Interior, RE1, the R2025 models, the held-out files) has such a room. 49 of the 50 `rme` rooms and 4 of Snowdon Towers' 54 keep their box (RE-316, RE-374). Not measured against any Revit export (Griffin, 2026-11-05, "Ship, labelled", then "Same-project gate"): the Areas of Revit 2025 to 2027 and the 108 Area spaces of the recorded Autodesk 2024 Snowdon Towers samples (101 Architectural and 7 Structural written, 96 and 2 with a body; the bodies are from Measure run 42003913780 on `bcb02107`, Architectural's refusals from run 42017400629 on `5008247d`, Structural's total from `e9124b3a`). They are written by default. The Areas of Revit 2025 to 2027 say `SpaceOutlineSource` `area_plan_topology_unchecked`, so that the file shows that nothing outside it checks them (Griffin, 2026-11-05, "Mark 2025 to 2027 in the file"); the Snowdon Towers 2024 samples' Areas say `area_plan_topology`, because a 2024 file shares its release and code path with the office's, whose Areas Revit's IFC4 export checks, and that release is not split. The evidence is that the loop drawn for each equals the area Revit saves in the file for that loop (`PlanCircuit m_area`, to 1e-6) and holds the Area's point and, on 2025 to 2027, the same project's 2024 save gives the same Areas. They stay written only while the Areas of rac_advanced on its 2025, 2026 and 2027 saves equal its 2024 save's, Area by Area, in GlobalId, number, name, storey, outline source, body area, top surface, height and lowest vertex (Griffin, 2026-11-05, "Same-project gate"). Every Measure run of the samples that holds the four saves checks it with `tools/re/areas_across_saves.py` and fails on a difference, a missing or extra Area, other than 25 Areas or 25 bodies on a save, an Area with no storey, or a missing save; a run with fewer than the four saves (a narrower `sample_years`) checks nothing and prints a notice. Measure is not a required check: a failure blocks landing by the landing rule, read before merging. It compares rvt-rs with itself and cannot catch an error the 2024 read shares with the later reads. The seven held-out Revit 2025 to 2027 models write no Area and their exports are byte-identical to main's, so they test nothing here. [docs/rooms.md](rooms.md) says what to trust per release, and RE-313 has the runs.

**What a space is.** Each `IfcSpace` says what it is in `RvtElementRecordGeometry.SpaceKind` (#1437): `room`, `area` (a Revit Area), `mep_space` (a Revit Space) or `family_instance` (any other element, a family instance or a mass in the files measured, that its "Export to IFC As" writes as a space), taken from the Revit class of the record it was read from. No `mep_space` is written today: rvt-rs reads no Revit Space from a file's partition records, which says what rvt-rs writes and not what the files hold. It is the property to read to tell an Area from a room; `SpaceOutlineSource` says only where an outline came from, and its values change with the rules that read outlines, so an importer does not filter on them. The same value is `space_kind` in the MCP server's `list_rooms` and the room schedule's last column. A space whose property set is not `RvtElementRecordGeometry` (in principle a wall or floor read through the schema walker) has no `SpaceKind` and is counted (`exported.spaces_without_kind`, zero on every model measured); `tools/re/space_kinds_vs_ifc.py` checks every export Measure and CI make. On Measure run 42027776156 (head 04d1b101) the 117 models and samples exported held 4,453 `IfcSpace`s, all with a `SpaceKind` (3,733 rooms, 628 Areas, 92 family instances, no `mep_space`), and the scorer passes on all 117: no space without a kind, no `area` outside the Area outline sources and no other kind inside them. See [rooms](rooms.md).

### Saved point/arc ruled surfaces (RE-443)

A zero ruled-profile pointer selects its separately saved point. The reader
admits point/arc patches with a unit envelope and rectangular trim; bodies
holding these patches qualify only by the source record's duct-fitting
category. On the public Autodesk Revit 2023 basic MEP sample, a local run
recovers 105 duct fittings within 0.697 mm and 0.179% volume of native IFC
in the default comparison. None meets the stricter 0.001 ft two-way
threshold with default reference tessellation; all 105 do with fine
reference tessellation. Two sanitary terminals still miss it with fine
tessellation and remain boxes with an explicit refusal. They are negative
controls, not exported saved meshes. Other combinations, envelopes, trims
and categories remain refused. [RE-443](../reports/element-framing/RE-443-collapsed-ruled-profiles.md)
has the bytes, category-override controls, reference hashes and local replay.
The native reference is not shared because redistribution terms are not
established; these accuracy figures are local measurements, separate from
the maintainer's Measure of the body-source changes.

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
Revit 2024 — `Wall`, `Door`, `Window`, `Column`, `Floor` and `BuildingPad`
instances from partition element records. The 2024 rows RE-15 read as an
`ArcWallRectOpening` index are no longer merged: every id they name is a
system element, not a wall opening (RE-376).
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
**A door's and a window's hand flip and facing flip are decoded** (#1357,
RE-343): the decoded Door and Window records carry `m_hand_flipped` and
`m_facing_flipped`, the `m_flippedX` and `m_flippedY` booleans of their
native record, on Revit 2024 to 2027 (1,820 of 1,820 doors and windows of
Autodesk's samples; the booleans are read on 2015 to 2023 too, but those
releases decode no Door or Window records). No door or window is stored as
a reflection, so `m_opening_facing`, the stored transform's Y axis, holds
the hand flip only. For an element set in a plain wall rvt-rs binds, Revit's
own placement is the stored axes reversed where the two booleans differ:
exact on 653 of 653 doors and 626 of 626 windows of the open models and
Snowdon Towers (the two office files are one building, and the case where
the booleans differ is measured on three 2024 buildings, one 2025 door and
one 2026 house), and not on 49 of the 160 doors rvt-rs binds to no plain
wall, which disagree
(`reports/element-framing/RE-343-door-window-flips.md`).
**A door or window set in a plain wall is placed on Revit's axes** (#1447,
RE-347): its `IfcLocalPlacement` X is the stored X, reversed (with Y) where
the two flips differ, Z up, the origin unchanged; the box keeps its place in
the world (a quarter turn swaps its two sides) and the element says so in
`RvtElementRecordGeometry.PlacementAxesSource`. Before, rvt-rs wrote the
default axes on every door and window, which were Revit's on 115 of the 521
doors and 226 of the 540 windows it binds to a plain wall in the open models
(Measure run 42066757105). Now the rule is written for all 521 and 540, and
Revit's axes are the same on all of them, and on the 24 doors and windows of
the R2025 models (Measure run 42106711251, head `c1a2a739`; the BIM4LCA office
files are one building). In the held-out models of 2024 to 2027 it is written
for 41 of 41 doors and 104 of 105 windows (the one other already had a
rotation of its own, which is Revit's) and none differs. Every door's and
window's body, and the opening it fills, is where it was in world
coordinates, as a saved mesh and as a record box, and its GlobalId, type
GlobalId and host wall are unchanged, on Core Interior, RE1 Architecture and
the two office files (`tools/re/placed_bodies_vs_base.py`, runs 42092391898
and 42106731054). A door bound to none, an element with no stored transform
(Revit 2023 and older) and a turned instance keep the default axes, which say
nothing of the direction they face; the origin is still the record box's
centre, more than 0.5 ft from Revit's on all 1,061 walled doors and windows
scored (#1465, RE-357): Revit's is the stored origin plus an offset that is
one value for every element of an `IfcDoorType` or `IfcWindowType` entity
(160 of 160), which revit-ifc keeps in the type's `StyleTransform` from the
instance that created the type and the file does not record; two of its axes
follow the element's own opening and the third does not, so no rule with 0
wrong exists (Measure run 42127619521,
`reports/element-framing/RE-357-door-window-origin.md`). On Revit 2025,
2026 and 2027 the same elements are held to the 2024 save by the same-project
check (`tools/re/bodies_across_saves.py`, run 42106711251): it compares 107,
79 and 65 doors and windows of `rac_advanced` and 17 each of `rac_basic` (of
them 66, 44, 31 and 14, 14, 14 not on the default axes), so two buildings,
each saved by three releases, exercise it; 28 and 42 the 2024 save places by
the rule are left on the default axes in the 2026 and 2027 saves of
`rac_advanced`, not scored
(`reports/element-framing/RE-347-door-window-placement-axes.md`).
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

**The viewer's saved parameters (#35, 2026-11-05)**: selecting an element
lists the same saved parameters in the info panel, instance and type apart,
named ones first and the nameless ones behind a fold that groups them by why
they have no name (#255), each row saying in words where its parameter is
defined. The `savedParameters` binding reads them for the selected element
only, in a worker of its own that ends with the answer, so the open is not
changed and nothing of the file stays resident. Nothing is requested once the
first file is open: both workers are bundled into the page and start from blob
URLs (the page's `worker-src` gained `blob:`, `docs/viewer-privacy-posture.md`),
the decoder's glue is a static import, and the compiled wasm the first open's
worker hands over serves the next open, the reads and the main thread's IFC,
plan and CSV exports, so a page taken offline after the load still reads,
exports and opens the file again. The Parameters row of File Status no longer says the rest are
still to be decoded: it counts the values written to the IFC property sets and
points to the panel. The panel says so when a file is of a release outside
Revit 2015 to 2027, when one of 2015 to 2022 is read only experimentally
(RE-281), when an element has no saved parameters and when a read failed. A
read has taken 0.74 to 1.04 times the open's wasm memory, rising with the file,
so a selection is read on its own only while the wasm memory the page holds is
at most 1 GiB (nothing above the office's 910 MiB is measured) and asks first
above that, saying that clicking may close the tab; after a read has run out of
memory the file is read only when asked. The page keeps each element's answer,
up to 64 and the least recently selected going first, until a file is opened
again. `viewer/tests/parameters-row.spec.ts` holds the panel's entries for an
element to the entries `rvt-elements` prints for the same ElementId, whole, on
Einhoven and on Core Interior. Viewer tests run 42125048710 on head 3146f14e (CI runner, WASM without LTO): all 37 of
Einhoven's tree elements were selected, 28 with parameters (369 entries, 332
instance, 37 type, 8 named) equal to `rvt-elements`' and 7 with none showing
the empty state; a read took a median of 153 ms (most 210 ms) in 17 MiB of wasm
memory, the open's worker 23 MiB. On Core Interior four elements match, each
read 1.5 to 1.9 s in 275 MiB (the open: 326 MiB, 17.3 s). On the BIM4LCA office
(62 MiB), not compared with `rvt-elements`, a selection read on its own: 8
entries in 3.9 s in 943 MiB, the open's worker 910 MiB (66.6 s), and the page
then kept 2 answers, 4,654 characters of JSON. Under a V8 cap of 927 MiB of wasm
memory, between the office's open and its read, a read really runs out of
memory and the file is read only when asked from then on. The page's one
script is 40.27 kB larger (8.65 kB gzipped) than in the build of d1de8adf,
mostly because the glue and the two workers it fetched later are inside it. The release-outside-the-range and
experimental-release states are measured with the worker's answer replaced, as
no such file is in the viewer's job.

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
counts the Revit parameter values written into the export's property sets
(the info panel lists a selected element's saved parameters, below). The scene tree groups each
storey's elements by category, type name first, with curtain-wall and
stair parts under their whole. Elements reach their storey from the
Level their record names (RE-59, RE-60): all 970 of Core Interior's
building elements sit on Revit's storey, and ArcWalls bind by elevation.
The tree, the Schedule panel and the info panel then put each element on the
storey the IFC export writes it on (a part on its whole's, an element in a
room on the room's, an opening on its host's) and list only the elements the
export writes (#1422).
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
#222 solved is a different carrier — the partition element record.
The opening-index rows are not openings at all: every id they name is a
system element (RE-376), and they are no longer merged. AProperty*
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
left out by that change's default and written with `--phase "Working Drawings"`,
which writes 14 of that export's 17 spaces by GlobalId and changes no other
entity (Measure runs 41784415396 and 41784419457). Revit 2015
and earlier write every room, as every element: both phase rules start at
Revit 2016 (#1332). The Python bindings and the viewer follow the default
(`reports/element-framing/RE-326-room-phases.md`).

**RE-445 (2026-11-11)** reads the project's display units from its
`UnitsElem` record: from Revit 2021 its `AUnits.m_formatOptionsMap` gives
every spec its display unit (133 to 150 specs on the corpus), and a family's
document inside the project keeps a `UnitsElem` of its own, named by
`m_famId`. The diagnostics report the units of length, area, volume, angle,
mass and time in `recovered_unit_identifiers` and every spec's in
`project_display_units`; Core Interior and Snowdon Towers read feet and
fractional inches where the old count of unit strings gave metres. A family
the native walk does not read takes its units from its string records where
their maps agree (Autodesk's sample families of 2021 to 2027). Revit 2015 to
2020 keep an array without spec names, which is not read. The IFC is
unchanged: values and unit assignment stay SI
(`reports/element-framing/RE-445-project-display-units.md`).

**#1359 (2026-11-05)** gives the rooms a fallback, a choice by Griffin. An
outside importer reported Autodesk's `rac_basic_sample_project` going from 14
rooms to 0 between 0.13.0 and 0.14.0, and the cause is the RE-326 rule: all 14
rooms are in Working Drawings, the second of three phases, and the last,
Learning Content, holds none (Measure run 41985023986 on the 2016 to 2027 saves:
`room_not_in_export_phase` 14 on each, all in Working Drawings). Where no phase
is named and the project's last phase holds no room, the export now writes the
rooms of the latest phase that holds any, because rooms all in one phase
cannot overlap and the only Revit export of this model holds them. The
diagnostics name the phase and count the rooms (`room_phase_fallback`),
`rvt-ifc` and `rvt-schedule` print a line, and the Python bindings raise a
`RuntimeWarning`. Every other element still follows the last phase, a phase
named (`--phase`, `phase=`), the last included, is exact, and a project with a
room in its last phase is unchanged. It is a choice and not a measurement of
Revit's default: no Revit export made at the last phase of such a project
exists, and a project with no room in its last phase gets earlier rooms that
Revit's default might leave out. Measured (Measure run 42035308270 at `ea0c7449`): the 12
saves of 2016 to 2027 each write 14 `IfcSpace` of kind `room` (0 before) and the
other 99 of 111 exports, held-out ones included, are byte for byte as before; the
default is byte for byte the `--phase "Working Drawings"` export and pairs 14 of
the Revit 2018 export's 17 spaces by GlobalId, and naming the last phase writes
what 0.14.0 wrote (`reports/element-framing/RE-326-room-phases.md` §9.4).

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
(`reports/element-framing/RE-328-beam-rejections.md`). **Since RE-428
(2026-11-11, #1697)** the beam's location line gives such a beam the
`Length`, `Span`, `Slope` and `Roll` it was left without, as it does every
beam that has a line, whatever its body is drawn from; the 39 beams of this
kind in the BIM4LCA files read Revit's four values (the report holds the
counts).

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

**RE-429 (2026-11-11)** writes a wall's `LoadBearing` as Revit's exporter
calculates it: true when the Structural box of its native record is set and
its usage is Bearing, otherwise false, and a floor's from its Structural box
and an unchecked stair's as false, all on Revit 2024 and later; a parameter
named for the property still replaces it. Against Revit's exports: 4,006 of
4,253 walls of the BIM4LCA models, the R2025 models and RE1, 294 of 313 slabs
and all 168 stairs, none different, and the 138 walls of the held-out models
of Revit 2024 to 2027. A stair with a checked box, a wall whose usage is Shear
or Combined, and walls, floors and stairs before Revit 2024 are left without
(the older models show only the false state, so nothing is written for them;
Vitoria's 247 walls are the ones the census counts apart)
(`reports/element-framing/RE-429-structural-load-bearing.md`).

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
(opt-in; the default export is unchanged, hash for hash on 34 of 34 models of Measure run 41884073888, 111 of 111 at the earlier heads) draws an
instance from its family's extrusions where they reproduce its saved mesh.
On Core Interior, RE1 and the BIM4LCA office files the first run (`aaa38de8`, before
the flat-arc rule), which drew arcs as arcs, replaced 335 bodies; of the 251 the body scorer matched, 250 had
Revit's vertices within 0.001 ft of their surface and one did not: Revit 26.2's
IFC2X3 export draws that circle (0.2292 ft) as an `IfcCircleProfileDef`, which
IfcOpenShell cuts into 27 chords, and two half arcs of it come out as 28, so
no vertex coincides (a count mismatch, reproduced and measured in
`reports/element-framing/RE-327-family-extrusions.md` §11). That count
mismatch is the default tessellation's, not a difference of shape: the bar for
a body both files write as true curves is the fine tessellation (RE-331 §8,
the maintainer's decision on #1355). Revit 24.0.5's IFC4 export of the office
writes a circle as half arcs and Revit 26.2's IFC2X3 export as an
`IfcCircleProfileDef`. Every arc is written as the arc, whatever its radius
(#1366). Measured at `34750454` (Measure runs 41884078892 and 41884639625,
RE-331 §9.1): 335 bodies are replaced (Core Interior 37, each office file 129,
each terrain file 11, RE1 Architecture 8, Electrical 8, Mechanical 1, Plumbing
1), where the 0.12 ft rule of #1352 replaced 66 and kept 269; 251 of them are
scored by `mesh_bodies_vs_ifc` (the 42 per office file written as `IfcSpace`
are scored by `space_profiles_vs_ifc`), and all 251 hold "ref on ours", both
directions within 0.001 ft and the volume within 0.01%: 187 are decided by the
fine tessellation, the bar for a body both files write as true curves (the
arcs: 78 in each office file, 11 in each terrain file, 6 in RE1 Electrical and
1 in RE1 Architecture, and 2 circles of 0.12 ft or less), and 64 by
IfcOpenShell's default one; 6 of the 53 exports move against `main`'s (the
office files, the terrain files, RE1 Architecture and Electrical), each only by
the bodies the rule kept, none held-out, and no row that passed on `main`
fails. At `aaa38de8`, before the flat-arc rule, the first arc run had 250 of
the 251 within the default tessellation's bar, the one that did not being a
count mismatch between its default tessellation of Revit 26.2's IFC2X3 circle
profile (27 chords) and of two half arcs of it (28) (run 41805333248, RE-327
§11). The flag reads Revit 2024 to 2027
(measured on 2024 and 2025); an earlier file has no instance transform that
reads, so nothing is replaced (`reports/element-framing/RE-327-family-extrusions.md`
§9).

**RE-350 (2026-11-05)** measures why three categories stay unread, on 59
models with a Revit export and Autodesk's samples (Measure runs 42097940515,
42108271073; `probe_re350_category_stages`, under its earlier name
`probe_re348_category_stages` at `32586847`; #1444, #1445, Discussion #1241).
The files hold them as frames, the same on every save of 2020 to 2027: 346
`Rebar` on `rst_basic` (14 on Snowdon Towers Structural), 16 Structural Beam
Systems on `rst_basic` (15 with a body) and 1 on `rac_advanced`, 3 placed
Furniture Systems on `rac_basic`. No Revit export measured holds a reinforcing
bar (0 of 60), and the only export of a project with a beam system, the
three.js IFC2X3 export of `rac_advanced`, holds neither it nor any of the
project's 38 framing members (23 of them the system's), so neither is added.
Furniture Systems are `IfcSystemFurnitureElement` in the public IFC4 export of
the 2018 `rac_basic` (RE-326 section 5, read locally): the 3 placed Furniture
Systems of that save are all in it (3 of 3), none written that it lacks;
the category meets the rule, and RE-352 below adds it. Revit's exporter has
code for rebar and beam systems (revit-ifc, CLEANROOM source 5), so the
absence of an export is not evidence that Revit never writes them. On
`rac_advanced` and `rac_basic` the sweep reads
1,349 panels, 3,276 mullions and 100 doors, and 44 panels, 144 mullions and 16
doors, on all eight saves of 2020 to 2027, every placed frame written. The 192
panels, 1 door, 4 mullions and 2 panels that RvtIO counts beyond them on its
2023 files are `FamilyInstance` records in the main model that every save
2018 to 2027 holds with no frame, so no stored bounding box; the public IFC4
export of the 2018 `rac_basic` holds none of the 6, and the IFC2X3 export of
`rac_advanced` none of the 193. Their family types are not read from the file
(`reports/element-framing/RE-350-unread-categories-and-rvtio-extras.md`).

**RE-352 (2026-11-05)** exports Furniture Systems (`OST_FurnitureSystems`,
#1444) as `IfcSystemFurnitureElement` with no `PredefinedType`, from their
element records on every release the exporter reads, where `category_map` gave
`FurnitureSystem` the class `IfcFurniture` and nothing produced it. It rests
on one project and 3 elements: Revit's public IFC4 export of Autodesk's 2018
`rac_basic` holds 3 `IfcSystemFurnitureElement`, and the 3 the head writes for
that save are its 3 by `Tag`, none extra and none missing, with record boxes
equal to Revit's bodies to 0.0000 ft (counts and distances; the export has no
licence and nothing is quoted from it; Measure run 42135914864 on `4ca938ad`).
That export writes no property set on its 3, so no `Reference` is written. It
contains all 3 in a space; rvt-rs contains 2 of the 3 in a space on Revit 2024
and later only, as for every furniture class, and the third has no Level and
stays in the building. Type objects are not written, though Revit's export
types all 3. In that run 13 of 111 exports move, every `rac_basic` save of 2015
to 2027, each by 3 `IfcSystemFurnitureElement` and nothing else, 98 are byte
for byte main's, and the same-project gates pass (the three `rac_basic` saves
among their 9 samples hold the category). No other model in Measure places a
Furniture System
(`reports/element-framing/RE-352-furniture-systems.md`).

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

- **v0.21.0** (2026-11-13) is the latest release: crates.io (`rvt` and
  `rvt-rs-mcp`, both published from the tag), the `rvt` wheels and sdist on
  PyPI, GitHub Release archives for Linux, macOS and Windows, and the
  container image, each checked from its published artifacts
  ([`install.md`](install.md)); docs.rs had built `rvt` when they were checked
  (`rvt-rs-mcp` is a binary crate with no documentation there). The GlobalIds
  an export writes hold from one save of a project to the next: openings,
  voids, fills, property sets, quantity sets, material associations,
  containment, aggregation, type relationships, ports and the further pieces of
  a layered floor, a slab or a curtain wall were seeded from the document's
  `Unique Document GUID`, which changes on every save, and are now seeded from
  the GlobalId of the entity they belong to (#1840, #1854). They change once,
  in this release, which a clash or issue-tracking (BCF) workflow sees as those
  entities removed and added again. `rvt-diff --elements` lists what changed
  between two saves, keyed by GlobalId (#1843); `--redact` scrubs the user name
  of a Linux home folder and every path of a message (#1871); and the warning
  for element records of ElementIds the element table does not declare no
  longer calls the IFC incomplete for what a same-save comparison showed to be
  deleted elements (#1838). Against 0.20.0 all 146 measured exports move, and
  `diagnostics.json` is equal on all 121 that publish one; the line comparisons
  of the two GlobalId changes found only GlobalIds changed on the exports they
  kept, and this run compares hashes, not lines. Time and peak memory are 1.00
  times 0.20.0's in the median over 75 files (0.94 to 1.06 times the time).
  What it does not measure: what belongs to no entity still moves between
  saves (the project, site, building and storeys where Revit gives them no
  GlobalId, classifications, materials, MEP systems and some types, #1854);
  `rvt-diff --elements` compares what rvt-rs reads, so two saves are compared
  with one build (#1843); the 79 corner mullions of the 2027 `rac_advanced` get
  no `IsExternal` (#1865); and the reading of #1838 rests on one model compared
  with Revit's export of the same save, as counts. `[0.21.0]` in
  [`CHANGELOG.md`](../CHANGELOG.md) gives what each change measures and what it
  does not.
- **v0.20.0** (2026-11-12): published on every channel, each checked from its
  published artifacts. It is mostly
  hardening against crafted files: such a file can no longer crash the browser
  viewer through a record's length or an overflow in a value it stores, make a
  read take exponential time or time that grows with the square of its element
  count, exhaust the stack through the chains it sets, or write its own text
  into an export's STEP, and a reader that stops at one of the new work budgets
  says so in a `work_budget_reached` item and a warning while the export is
  still written; no real file of the measured corpus reaches one. For real
  files, a wall, floor, ceiling or roof type edited since an earlier save keeps
  its layers on Revit 2024 and later (180 elements of the BIM4LCA files gain
  their type and its layers); the project's display units are read from its
  Project Units; Revit 2026 and 2027 walls, and the curtain parts on them, get
  the `IsExternal` the same project's 2025 save gives (4,643 on Autodesk's
  `rac_advanced` 2026); an element on a Level that is not a building story goes
  to the building story below it, as Revit's export places it, and a file whose
  Levels include no building story (Autodesk's `rst_basic`) is written with no
  storey instead of storeys made up from its elements' heights; and each GLB
  material says whether its colour and alpha are the file's. Against 0.19.0, 49
  of the 146 measured exports move, with no census regression and no body
  regression, and the default export of Autodesk's 29 samples of Revit 2023 to
  2027 takes a median 0.76 times 0.19.0's peak memory and 0.97 times its time.
  What got worse: the stair runs that read their own settings are drawn from
  their sketch where their saved mesh drew them (2.55% larger in volume on the
  five stairs of `rac_advanced` 2024, #1856), and an `IfcRoofType` is written
  on the terrain timber housing where Revit's export writes none. What it does
  not measure: the layer rule rests on one project (the BIM4LCA files), no
  held-out model has a Level that is not a building story, 79 curtain members
  of the 2027 `rac_advanced` still get no `IsExternal` (#1865), and the
  crafted-file bounds are exercised by crafted inputs, not by a real file.
  `[0.20.0]` in [`CHANGELOG.md`](../CHANGELOG.md) gives what each change
  measures and what it does not.
- **v0.19.0** (2026-11-12): published on every channel, each checked from its
  published artifacts. A default export of a Revit 2023 to 2027 project took
  much of the time and memory 0.18.0's saved-mesh default cost back (a median
  0.64 times its peak memory and 0.89 times its time, #1694), drew more
  elements from their saved meshes, which made the IFC of those files larger
  (the BIM4LCA office as built 87.5 to 113.7 MB) and of others smaller, and
  wrote more of what Revit's own export writes for the bodies it draws: the
  quantities of beams, columns, curtain mullions and walls, `LoadBearing` on
  walls, floors and stairs of Revit 2024 and later, and `PredefinedType` on
  furniture, railings, walls, ramps, light fixtures and pipe segments. The
  quantity rules were developed on, and are scored against, the one project
  with a Revit export that holds them (the BIM4LCA office and housing), so the
  beams, columns, mullions and walls of every other model gain them unscored;
  a mesh is not always Revit's (the 24 railings newly drawn read about 1.2
  times Revit's volume). `[0.19.0]` in [`CHANGELOG.md`](../CHANGELOG.md) gives
  what each change measures and what it does not.
- **v0.18.0** (2026-11-11): published on every channel, each checked from its
  published artifacts. A default export of a Revit 2023 to 2027 project drew
  every element that would be a bounding box from the mesh Revit saved for it,
  where all of the mesh decodes (#1191), at 1.1 to 1.9 times the time and 1.4
  to 4.4 times the peak memory of 0.17.0 on Autodesk's samples (0.19.0 takes
  much of that back, above), and a railing that belongs to a stair was
  aggregated under it, a wall with an edited elevation profile had the openings
  Revit's export writes, every type object had a GlobalId of its own, and a
  standalone Revit 2020 family with one revolution exported as an
  `IfcRevolvedAreaSolid` (experimental). Every record box that stays one says
  why (`SavedMeshRefusal`). No roof, beam, slab, footing or stair flight mesh
  was among the bodies scored against Revit's export on 31 models, a mesh was
  not always Revit's (doors 675 of 681, walls 14 of 18), and a stair that
  aggregates its railings has more volume than Revit's where a railing is
  still its record box (RE-303). `[0.18.0]` in
  [`CHANGELOG.md`](../CHANGELOG.md) gives what each change measures and what it
  does not.
- **v0.17.0** (2026-11-09): published on every channel, each checked from its
  published artifacts. It reads more
  of Revit 2015 to 2023 projects: curtain walls are `IfcCurtainWall` with their
  mullions and panels under them, along their own axes, with their type,
  family, material, `Span`, `Length` and `IsExternal`; steel beams of 2017 to
  2023 are drawn as their type's I section; walls write their `Height` where
  their record names nothing joined to or cutting them; and ducts of Revit 2015
  to 2027 take their type, section, line and ports from their native records,
  with a port's `SystemType` and its joins. On Revit 2024 a room whose stored
  solid neither the walk nor the span closes takes its floor's outline from
  the solid's edges, a wall whose floor covers all of its plan but a stub
  inside another wall is cut whole, and a wall whose edited profile closes
  below its record box ends at the profile's highest point with the profile's
  `Height`; each of the last two rests on one wall. A Revit 2026 or 2027 save is
  written without an `ObjectType`, as Revit's own export of those releases
  writes none, and two claims every export made are now made only where they
  hold (#1598). No 2015 to 2023 curtain wall, curtain part, steel beam, duct or
  wall `Height` of Autodesk's samples is scored against a Revit IFC4 export:
  they are held to the same projects' 2024 saves, which cannot catch an error
  those saves share. `[0.17.0]` in [`CHANGELOG.md`](../CHANGELOG.md) gives what each change
  measures and what it does not.
- **v0.16.1** (2026-11-09): the first published release of 0.16: the
  `v0.16.0` tag was made and never published, because its publish run stopped
  at the viewer smoke test (#1569). It reads much more of Revit 2015 to 2023 projects, with
  beams along their location lines, walls joined by their join lists (and a
  wall at an angle along its line where its box closes on it), pipes drawn as
  their cylinders with their types, quantities and ports, `Constraints` Level
  parameters and `Phase Created`, and a Revit 2015 file that follows the export
  phase; and of Revit 2026 and 2027, with beams, stair runs, tread landings,
  sketch outlines, pipes, ports and door and window cuts read as on 2024 and
  2025. Areas of Revit 2025 to 2027 are written (`area_plan_topology_unchecked`:
  checked against no Revit export), every `IfcSpace` says whether it is a room,
  an Area or a family instance (`SpaceKind`), and a project whose last phase
  holds no room writes the rooms of the latest phase that holds any. A door or
  window set in a plain wall is placed on Revit's axes with both flips decoded,
  though its origin is still its record box's centre (RE-357). Exports hold the
  IFC4 EXPRESS rules IfcOpenShell's default validation leaves out and refuse a
  number that is not finite, the viewer's info panel lists an element's saved
  parameters, and release pages list and attest the wheels and the sdist. Most
  of the 2015 to 2023, 2026 and 2027 reads are held to the same Autodesk
  project's 2024 or 2025 save, not scored against a Revit export, which cannot
  catch an error that save shares. Since 0.16.0's notes: a roof drawn as its
  record box leaves the walls under it whole, which is a smaller error and not
  Revit's body (#1471); the two `rac_basic` 2027 doors have their Width and
  Height (#1489); and a sealed registry holds the 1.0 check's fresh held-out
  pairs (#1565). `[0.16.1]` and `[0.16.0]` in
  [`CHANGELOG.md`](../CHANGELOG.md) give what each change measures and what it
  does not.
- **v0.15.0** (2026-11-05): it says, on request, where each decoded element and the values
  its body took were read from, down to the byte, and why each element,
  body, opening or outline was left out (`rvt-ifc --element-provenance`,
  `--export-decisions`): the offsets are confirmed on the Revit 2023 to 2025
  models CI holds and the decisions on 2024 and 2025, not on earlier
  releases. The MCP server asks for both, gains `explain_element` and names
  elements and type objects as the IFC export does. Revit 2008 to 2023
  exports count where their element records stopped (not measured on 2008
  to 2014). Floors whose shape-edited corners lie in one plane are drawn
  along their sloped tops, and the walls on them start from that top (the
  refusals this adds on Snowdon Towers and `rac_advanced` have no Revit IFC4
  export to score them); more Areas are drawn to their boundary lines'
  joins, a rule read off the one IFC2X3 export it is scored against, so
  in-sample; rooms take off the round columns their solids hold; floor voids
  drawn clockwise take Revit's tag; and the opt-in `rvt-ifc
  --family-extrusions` draws family instances from their families' own
  extrusions (Revit 2024 to 2027, measured on 2024 and 2025). `[0.15.0]` in
  [`CHANGELOG.md`](../CHANGELOG.md) gives what each change measures and what
  it does not.
- **v0.14.0** (2026-11-04): it adds `rvt-rs-mcp`, a local, read-only MCP
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
  265 exact to 0.001 ft against Revit's export, in IfcOpenShell's default
  tessellation), and it no longer calls a
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
- `main` has everything v0.21.0 has, and is ahead of it by the fragments
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

## Standalone family revolutions (experimental)

The default exporter reads one visible positive revolution in a standalone
Revit 2020 RFA and writes an analytic `IfcRevolvedAreaSolid` from its saved
line/arc profile, directed axis and start/end angles. Four fixtures are supported:
REV-A, REV-B and REV-BT (line profiles, including partial and tilted sweeps),
and ARC-A (full torus), each confirmed by Revit's own IFC
(RE-391 through [RE-394](../reports/element-framing/RE-394-analytic-family-revolutions.md)).
The cleaned six-file fixture package has an offline replay in
[the runner README](../tools/oracle/runner/revolutions/README.md) and a pinned
archive entry in `research/family-revolutions-measure.tsv`.
The IFC Tag retains the source ElementId; the body
is unclassified and does not bypass mapped-type quality modes.

This is not general family support: project instances, nested/combined forms,
voids, other releases and unmeasured profile families remain outside this
coverage. The existing family-extrusion option is unchanged. Profiles containing
arcs are refused for partial sweeps: ARC-B contradicts Revit's IFC and ARC-BT
has no native IFC verdict. Their face captures remain research leads only. The fixed
acceptance limits are 0.3048 mm sampled distance and 0.01% volume error.
