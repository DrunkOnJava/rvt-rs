# Roadmap

Last reviewed: 2026-11-13

This roadmap is the public, contributor-facing view of where rvt-rs is headed.
The support boundary lives in [`docs/status.md`](docs/status.md) and the
machine-readable [support matrix](docs/support-matrix.json); the statuses below
are its ceilings and this page never claims more. The plan itself is the GitHub
milestones and the issues under them; a milestone is closed when its release
ships. [`docs/archive/TODO.md`](docs/archive/TODO.md) is the older task decomposition they grew out of,
and [`docs/maintaining.md`](docs/maintaining.md) says how the tracker is run.

## Product Goal

rvt-rs should become a first-class open-source utility for BIM and AEC users who
need to inspect, validate, and exchange Revit files without installing Revit or
uploading private models to a third-party service.

It is not a production Revit-to-IFC converter yet, and the matrix says so
(`converter-grade-rvt-ifc` is unsupported). It opens every Revit file from 2016
to 2026 and reads its metadata, previews and embedded schema, and Revit 2015
and 2027 files too, measured on Autodesk's sample projects. On Revit 2024 and
2025 project files it reads the building: each element's ElementId, category,
name, type, storey, materials, layers and GlobalId, and a body that is exact
where the element's data is decoded and its bounding box where it is not. It
writes that as IFC4, glTF, plan SVG and CSV from the CLIs and the
zero-upload browser viewer, as IFC4 and CSV from Python, and as IFC4 and glTF
from a local, read-only MCP server. Revit 2023 projects get a narrower version of the
same, and Revit 2015 to 2022, 2026 and 2027 projects are experimental. Every
recovery claim is measured against the oracle its report names: Revit's own
IFC export of the same model wherever one is measured, and for 2015 to 2018,
2019 to 2022 and 2027, which have no measured same-release export (except two
projects, BIMcollab's 2015 example, RE-198, and Vitoria's 2018 `ALD26 Before`,
RE-315), the same
sample projects saved by Revit
2019, 2023 and 2026 (but 2015's `rst_basic`, another model than its 2019 copy).
What is not decoded
(most parameters in IFC, family geometry, openings whose symbol cuts no outline, the room phase rule against a Revit export made with the default setup, RE-326) is listed in [`docs/status.md`](docs/status.md), and
the four levels a file can reach are in the
[supported profile](docs/supported-profile.md).

## Current Position

| Area | Status | Current state | Next decision point |
|---|---|---|---|
| Container, compression, metadata | Verified | MS-CFB container, Revit's truncated-gzip streams and PartAtom open on every release from 2016 to 2027, and a preview is checked on the 2024 family; 2015 project files open on Autodesk's sample projects, where PartAtom and previews are not measured. | Maintain compatibility and bounds checks against hostile input. |
| `Formats/Latest` schema | Verified | The whole schema is read (#410: 4,126 classes on Revit 2024, where 395 were read before). A class's tag is its definition ordinal (#154), and element records name their class by it (RE-76). | Classify the three fields still unclassified (`ClassDefinitionRef::m_ref`, `VarParam::m_pSubst`, `VarSketchDimObj::m_dimValue`: no instance, or only null pointers), only with byte evidence (RE-323). |
| ADocument walker | Partial | Document-level metadata for triage; the root ADocument walk is validated on 2024 to 2026. | Expand confidence across project releases and older files. |
| Typed project elements | Partial | Elements come from their partition element records, typed by `BuiltInCategory` and matched against Revit's own exports. On Revit 2024 walls, doors, windows, columns, floor slabs and building pads match element for element (verified, RE-21); 2025 and 2023 are partial; 2026 is experimental (one local model), and so are 2015 to 2022 and 2027, measured against the same sample projects saved by Revit 2019, 2023 and 2026 (RE-181, RE-180, RE-179), but 2015's `rst_basic`, another model. Schema-field walls and opening-index doors and windows stay unsupported (RE-19). | A Revit export of a 2016 to 2022 or 2027 project to measure against, and a second 2015 one (#421, #878, #879), instance and type parameters in IFC (#35, #155, 0.13.0), held-out models (#408). |
| IFC writer | Partial | IFC4 with storeys, IFC type objects, material layer sets, materials and stable generated GlobalIds. Bodies are exact where the element's data is decoded and the record's bounding box elsewhere, and the diagnostics count each (#409). | The geometry issues open under the 0.13.0 milestone (below). |
| Browser viewer | Partial | Zero-upload WebAssembly viewer over the same decode; its file status shows how much of a model is a stand-in box. | Keep the journey test, which since 0.7.0 opens, inspects, schedules, exports and checks the diagnostics of the six redistributable reference models, green as the viewer grows. |
| Python and CLI | Partial | The `rvt` wheel on PyPI and prebuilt CLI archives on GitHub Releases, sharing the core's honesty bounds. | Stabilize JSON schemas and the one-shot inspect workflow. |
| Write path | Partial | Stream-level patching only. Field-level semantic writes are unsupported and gated by [ADR-002](docs/decisions/ADR-002-semantic-write-api-gate.md). | Stay gated until openability can be proven. |

## Milestones

### Released

- **0.2.0, 2026-09-23: audit-clean alpha.** Repository hygiene, honest public
  status, issue workflow and enforceable quality gates; prebuilt CLIs and the
  Python package. An inspection-focused alpha with experimental export.
- **0.3.0, 2026-09-27: trustworthy Revit 2024/2025 export.** IFC and glTF that
  read like Revit's own export: walls, floors, roofs and ceilings as their
  layers and materials, elements named and placed on Revit's storeys, stable
  generated GlobalIds (#400), declared units (#403), the whole schema (#410),
  and diagnostics that count the stand-in boxes (#409).
- **0.4.0, 2026-09-28.** Revit 2023 projects export typed elements as 2024
  and 2025 do; rooms take their real outline, number and name; doors and windows
  cut the opening their type specifies; turned family instances are drawn
  turned; steel members carry their I section; every typed element is related
  to an IFC type object of its Revit type.
- **0.5.0, 2026-10-18.** Revit 2026 projects export typed elements,
  experimentally; MEP models gain ports, port-to-port connections and systems,
  and pipes are drawn as the cylinders Revit draws; exports carry each
  element's levels, its category, family, type and phase, and more of the
  common property sets; rooms are aggregated into their storeys; Autodesk's
  Snowdon Towers samples export their elements again. The first release with
  outside contributors' pull requests.
- **0.5.1, 2026-10-18.** An export is named by the model's file name rather
  than the path it was saved at, which could carry the author's account and
  folder names (#782), and the README names the release it ships in.
- **0.6.0, 2026-10-19.** The standard property sets are IFC4's: those 0.5
  took from Revit's IFC2X3 exports moved to where IFC4 and Revit's own IFC4
  export put them, checked against buildingSMART's published IFC4 property
  set definitions on every measured export. Exports carry parameters named
  for a standard set's property, `IsExternal` (curtain mullions' and panels'
  by their curtain wall's, RE-288), type Fire
  Ratings, door sizes, Revit 2024 Areas as spaces, and port types, with
  electrical ports' direction (HVAC and piping directions are not read yet,
  #834); Revit 2015 to 2022 and 2027 projects export their elements,
  experimentally.
- **0.6.1, 2026-10-19.** 0.6.0 with an export panic on 32-bit targets fixed
  (#836) and a README whose support table says what is measured, built
  from 0.6.0 because `main` holds a breaking change (#841).
- **0.7.0, 2026-10-24.** Gable, hip and slope-arrow roofs drawn as their
  planes, Revit 2025 roofs with their slopes and roofs by extrusion as their
  sketch's section; Revit 2025's tapered walls; doors' and windows' overall
  sizes and openings from the outline their symbol cuts, on Revit 2024 and
  2025; ports placed, with
  Revit's flow directions, names and descriptions on HVAC and piping ports;
  Revit's materials on family instances, mullions, columns and walls; each
  element's saved parameters in `rvt-elements` and Python; reproducible
  exports; and the stable API 1.0 keeps, with the research readers behind
  `unstable-research`.
- **0.8.0, 2026-10-25.** Revit 2025 stair runs drawn as their treads and
  risers from their native records, spiral, riserless and monolithic runs
  as their steps or treads, and landings as their slabs or their one tread,
  with Revit's predefined types, names and type objects; walls that end
  where Revit's do at T joints (482 of 607 on the BIM4LCA office, from
  412); square hollow-section columns as their section; door and window
  sizes where the symbol cuts no outline; Revit 2024 pipes as cylinders;
  the project's, site's, building's and storeys' stored GlobalIds; Revit's
  property sets (`--revit-property-sets`); and `--anonymize` for
  diagnostics posted publicly.
- **0.9.0, 2026-10-26.** IFC4 base quantities as Revit's export writes
  them, on by default (20,157 equal on the BIM4LCA office as built, none
  that Revit's export leaves out): curtain mullions' `Length` and
  `NetVolume`, doors' and windows' sizes, walls' `Width` and their
  openings' `Depth`, stair flights', pipes' and unjoined columns'; curtain
  mullions' `Span`, walls' `ExtendToStructure`, railings' `Height`, level
  slabs' `PitchAngle`, stair flights' treads and nosing and more
  `Reference` values; types and hosts from native records; and
  `--revit-set-placement`.
- **0.10.0, 2026-10-27.** More of Revit's base quantities (21,065 equal on
  the BIM4LCA office as built, none that Revit's export leaves out): walls'
  `Length` and every wall's `Width`, curtain walls' and beams' `Length`,
  floors' `GrossArea`, `Depth` and `GrossVolume`, ceilings' `Width`,
  hollow-section columns' areas and volume, spaces' `Height` and stair
  railings' `Length`; and system-family elements' type parameters
  (puzzbobb, #1065).
- **0.11.0, 2026-10-30.** A model over the reader's resource limits is
  reported instead of exporting empty, with `--large-model` and a viewer
  retry (#1075); files that kept an older release's schema read as that
  release (puzzbobb, #421), neither measured yet on the reporter's private
  files; elements outside the export phase left out
  (puzzbobb, #328); doors and windows drawn from their saved meshes by
  default; windows' openings from their cut solids; walls' `Height` where
  nothing joins or cuts them; Revit 2008 to 2014 experimental (puzzbobb);
  and the unused stable API removed.
- **0.12.0, 2026-10-31.** More of Revit's base quantities and common property
  values, each where the model's records decide it:
  - on the BIM4LCA office as built, counting the objects rvt-rs writes and
    leaving out what the census counts apart, 29,262 of 39,388 quantities
    equal and, with `--revit-set-placement`, 45,235 of 45,525 property
    values, 1 different each;
  - values Revit's own exporters disagree on left out;
  - native records on Revit 2015 to 2022, saved parameters experimentally,
    and walls' and structural members' flags on Revit 2015 to 2023,
    experimentally;
  - native-record readers kept under the file's own limits (#1075), not yet
    measured on a model over them;
  - doors and windows from their saved meshes in Python and the viewer;
  - the unused public API removed.

  Exports of Revit 2015 to 2023 files take longer (#1237).
- **0.12.1, 2026-11-01.** Exports of Revit 2015 to 2023 files faster than in
  0.12.0, each the same as 0.12.0's but for the version its IFC header
  records; still slower than with 0.11.0 (#1237).
- **0.12.2, 2026-11-03.** More doors drawn from Revit's saved meshes in the
  default export and more bodies with `--saved-meshes`, from a user's
  comparison of 0.6.1 and 0.12.0; some cases it found stay open (#1247).
- **0.13.0, 2026-11-03.** Rooms and spaces: every `IfcSpace` says where its
  outline came from, Revit 2026 and 2027 rooms take their outline, unclosed
  room solids are read from their faces, and bodiless family spaces are left
  out (#1260, #1262); Revit 2018 projects that exported nothing export
  (#1261); joined columns' `Length`, the office's wall hatch widths, roofs
  with several slope arrows, the viewer's real progress, and faster native
  reads (#1203, #810, #885, #1195, #1237).
- **0.14.0, 2026-11-04.** `rvt-rs-mcp`, a local, read-only MCP server (#1324),
  and one export function every surface builds from (#1331); the export phase
  for rooms, chosen from `rvt-ifc`, Python, the viewer and `rvt-schedule`
  (#1194, #1339); Revit 2024 Area bodies and proven Revit 2015 to 2023 Areas
  (#763, #1261); the openings rectangular wall openings cut (#1165); stepped
  wall tops and wall corner order (#1067, #1194); conduits, conduit fittings
  and gutters (#1261); every timed export faster than 0.13.0's and a lower
  memory peak (#1075, #1237).
- **0.15.0, 2026-11-05.** Where each decoded element and the values its body
  took were read from, down to the byte, and why each element, body, opening
  or outline was left out, on request (`rvt-ifc --element-provenance`,
  `--export-decisions`; confirmed on Revit 2023 to 2025 and 2024 to 2025),
  which the MCP server asks for, with `explain_element` (#1325, #1326,
  #1368); the MCP server names elements and type objects as the IFC export
  does (#1327); Revit 2008 to 2023 exports count where their element records
  stopped (#1389); floors with shape-edited corners in one plane drawn along
  their sloped tops, and the walls on them starting from that top (#1384);
  more Areas drawn to their boundary lines' joins, in-sample against the one
  IFC2X3 export (#1261); rooms take off the round columns their solids hold
  (#1354); floor voids tagged as Revit tags them (#1165); and the opt-in
  `rvt-ifc --family-extrusions`, measured on 2024 and 2025 (#156).
- **0.16.1, 2026-11-09.** The first published release of 0.16: the `v0.16.0`
  tag was made and never published, because its publish run stopped at the
  viewer smoke test (#1569). Much more of Revit 2015 to 2023 and of Revit 2026
  and 2027: beams along their location lines, walls joined by their join
  lists, pipes as their cylinders with types, quantities and ports,
  `Constraints` Level parameters, `Phase Created`, and a Revit 2015 file that
  follows the export phase (#1190, #1332); on 2026 and 2027 beams, stair runs,
  tread landings, sketch outlines, pipes, ports and door and window cuts as on
  2024 and 2025 (#1190, #1440); Areas of Revit 2025 to 2027, all 25 of
  `rac_advanced`'s on every save, and `SpaceKind` on every `IfcSpace` (#1261,
  #1434, #1437); the rooms of the latest phase that holds any (#1359); doors
  and windows placed on Revit's axes with both flips decoded (#1357, #1447);
  the openings fire hose cabinets, doors and curtain walls cut in walls
  (#1165); door types' `FireRating`, family types' `Manufacturer` and Furniture
  Systems (#1185, #1444); the IFC4 EXPRESS rules checked and a number that is
  not finite refused (#1430); the viewer's saved parameters (#35); and wheels
  and the sdist attested on the release page (#1466). Since 0.16.0's notes: a
  roof drawn as its record box leaves the walls under it whole (#1471), the two
  `rac_basic` 2027 doors have their Width and Height (#1489), and a sealed
  registry holds the 1.0 check's fresh held-out pairs (#1565). Most of the
  Revit 2015 to 2023 and 2026 and 2027 reads are held to the same project's
  2024 or 2025 save, not scored against a Revit export; the notes say which.
- **0.17.0, 2026-11-09.** More of Revit 2015 to 2023: curtain walls as
  `IfcCurtainWall` with their mullions and panels under them, along their own
  axes, with their type, family, material, `Span`, `Length` and `IsExternal`
  (#1190); steel beams of 2017 to 2023 as their type's I section (#1190); walls'
  `Height` where their record names nothing joined to or cutting them (#1535);
  and ducts of Revit 2015 to 2027 from their native records, with their ports'
  `SystemType` and joins (#96, #834, #869). On Revit 2024 a room whose stored
  solid does not close takes its floor's outline from the solid's edges (#90), a
  wall whose floor covers all of its plan but a stub inside another wall is cut
  whole (#1067), and a wall whose edited profile closes below its record box
  ends at the profile's highest point (#1600). A Revit 2026 or 2027 save is
  written without an `ObjectType`, as Revit's export of those releases writes
  none (#1472), and two claims every export made are made only where they hold
  (#1598). The two wall rules rest on one wall each, and no 2015 to 2023 curtain
  wall, curtain part, steel beam, duct or wall `Height` of Autodesk's samples is
  scored against a Revit IFC4 export; the notes say which.
- **0.18.0, 2026-11-11.** Every element that would be a bounding box is drawn
  from the mesh Revit saved for it, by default on Revit 2023 to 2027, where all
  of the mesh decodes: walls, roofs, railings, curtain mullions and panels,
  columns, doors, windows, furniture, fixtures and equipment (#1191). Roofs that
  were the level block of their sketch outline, railings made of other
  elements' instances, instances forbidden the targets 10, turned family
  instances, and the ellipses, curved ruled surfaces and rings of the saved
  graphics are read now (#1612, #1630, #1635). More of Revit's own export is
  written for the bodies drawn: a wall's `NetVolume` and side areas, a
  mullion's `NetVolume` and `NetSurfaceArea`, a panel's `Width`, a roof's
  `GrossArea` and `GrossVolume`, a sloped floor's `PitchAngle` and the
  properties of an opening, measured on the BIM4LCA office and the housing, and
  a round column no longer carries a square's quantities. A railing that
  belongs to a stair is aggregated under it (#1624), a wall with an edited
  elevation profile has the openings Revit's export writes (#1607), four more
  "Export to IFC As" values are acted on (#1636), every type object has a
  GlobalId of its own (#1667), and a standalone Revit 2020 family with one
  revolution exports as an `IfcRevolvedAreaSolid` (#156, experimental). Every
  record box that stays one says why, on every release (#1645). Reading every
  box's saved graphics costs 1.1 to 1.9 times the time and 1.4 to 4.4 times the
  peak memory of 0.17.0 on Autodesk's samples of Revit 2023 to 2027 (#1694), and
  `--no-saved-meshes` keeps the boxes. No roof, beam, slab, footing or stair
  flight mesh is scored against Revit's export, a mesh is not always Revit's
  (doors 675 of 681, walls 14 of 18), and the quantity rules were developed on
  one project; the notes say which.
- **0.19.0, 2026-11-12.** Takes back much of the time and memory that 0.18.0's
  saved-mesh default cost: a decoded graph's fields take a sixth of the memory
  they did and the saved-graphics pass reads only what meshing needs (#1694), so
  the default export of Autodesk's 28 samples of Revit 2023 to 2027 takes a median
  0.64 times 0.18.0's peak memory (0.29 to 1.18) and 0.89 times its time (0.75 to
  1.27), and against 0.17.0 1.04 to 1.92 times the time and 1.04 to 1.69 times the
  peak memory. More elements are drawn from their saved meshes (faces where a plane
  meets a cylinder, faces with a holed or non-rectangular trim, plane rings that
  visit a vertex twice, four of Snowdon Towers Architectural's six block roofs, the
  curtain mullions and panels that were turned boxes; RE-421, RE-422, RE-426,
  RE-430, RE-434), which makes the IFC of those files larger (the BIM4LCA office as
  built 87.5 to 113.7 MB, Snowdon Towers Architectural 189.1 to 257.8 MB,
  `rac_advanced` +69% to +75%) and of others smaller (Snowdon Towers Electrical
  -12.6%). More of Revit's own export is written for the bodies drawn: a beam's six
  base quantities and its `Length`, `Span`, `Slope` and `Roll` (RE-423, RE-428), a
  column's quantities (RE-424, RE-425), a curtain mullion's (RE-404, RE-418 and
  after), a wall's side areas and heights where it is a prism less its openings
  (#1046), a proxy's `NetVolume`, and `LoadBearing` on walls, floors and stairs of
  Revit 2024 and later (RE-429). Furniture, railings, walls, ramps, light fixtures
  and pipe segments carry Revit's `PredefinedType`, and a Revit 2027 save is written
  without one on the classes whose exporter leaves it off (#1692, #1716); an
  element framed in several partitions is read from its newest frame (RE-444); an
  "Export to IFC As" of `IfcGeographicElement` is acted on (#1772); and a planar
  face of more than 10,000 points or 250 million search steps is refused and keeps
  its box (#1748, #1770). The quantity rules for beams, columns, mullions and walls
  rest on one project with a Revit export (the BIM4LCA office and housing), a mesh
  is not always Revit's (the 24 railings newly drawn read about 1.2 times its
  volume), and `[0.19.0]` in [`CHANGELOG.md`](CHANGELOG.md) lists what each change
  does not measure.
- **0.20.0, 2026-11-12.** Mostly hardening against crafted files: a file can
  no longer crash the browser viewer through a record's length or an overflow in
  a value it stores, make a read take exponential time or time that grows with
  the square of its element count, exhaust the stack through the chains it sets,
  or write its own text into an export's STEP (#1815, #1831, #1833, #1845), and a
  reader that stops at a work budget says so and still writes the export (#1855).
  For real files, a wall, floor, ceiling or roof type edited since an earlier
  save keeps its layers on Revit 2024 and later (#1802, RE-446); the project's
  display units are read from its Project Units (RE-445); Revit 2026 and 2027
  walls and the curtain parts on them carry `IsExternal` (#1860); an element on a
  Level that is not a building story goes to the building story below it, and a
  file with no building story, Autodesk's `rst_basic`, is written with no storey
  (#1842); 105 duct fittings of a Revit 2023 MEP sample are drawn from their saved
  surfaces (RE-443); each GLB material says whether its colour and alpha are the
  file's (#1812); and two exports of one file write the same wall areas and
  volume to the last bit (#1862). Against 0.19.0, 49 of 146 measured exports
  move, with no census or body regression, and the default export of Autodesk's
  samples of Revit 2023 to 2027 takes a median 0.76 times 0.19.0's peak memory.
  The layer rule rests on one project, the stair runs that read their own
  settings are drawn from their sketch where their saved mesh drew them (#1856),
  and `[0.20.0]` in [`CHANGELOG.md`](CHANGELOG.md) lists what each change does
  not measure.
- **0.21.0, 2026-11-13.** The GlobalIds an export writes hold from one save of
  a project to the next: openings, voids, fills, property sets, quantity sets,
  material associations, containment, aggregation, type relationships, ports and
  the further pieces of a layered floor, a slab or a curtain wall were seeded
  from the document's `Unique Document GUID`, which changes on every save, and
  are now seeded from the GlobalId of the entity they belong to (#1840, #1854).
  They change once, in this release, which a clash or issue-tracking (BCF)
  workflow sees as those entities removed and added again. `rvt-diff --elements`
  lists what changed between two saves, keyed by GlobalId (#1843); `--redact`
  scrubs the user name of a Linux home folder and every path of a message
  (#1871); and the warning for element records of ElementIds the element table
  does not declare no longer calls the IFC incomplete for what a same-save
  comparison showed to be deleted elements (#1838). Against 0.20.0 all 146
  measured exports move, and `diagnostics.json` is equal on all 121 that publish
  one; the line comparisons of the two GlobalId changes found only GlobalIds
  changed on the exports they kept. Time and peak memory are 1.00 times 0.20.0's
  in the median. What belongs to no entity still moves between saves (the project, site,
  building and storeys where Revit gives them no GlobalId, classifications,
  materials, MEP systems and some types), 79 corner mullions of Autodesk's
  `rac_advanced` 2027 save get no `IsExternal` (#1865), and `[0.21.0]` in
  [`CHANGELOG.md`](CHANGELOG.md) lists what each change does not measure.
- **0.22.0, 2026-11-13.** It limits what a crafted Revit file can make the MCP
  server, the command-line tools, the Python bindings and the viewer do:
  `rvt-rs-mcp` reads from and writes to one folder and bounds what an answer
  carries (#1883); `rvt-doc --redact`, `rvt-info -f csv`, `rvt-dump`,
  `rvt-ifc`, `rvt-write`, the Python bindings and the viewer each lose a way a
  hostile file reached a terminal, a spreadsheet, a log or a file they should
  not touch (#1884, #1885, #1887, #1919); and a wall profile of thousands of
  regions no longer builds a chain of boolean cuts too deep for a thread to
  walk (#1845). More elements are written the way Revit's export writes them:
  roofs by extrusion of Revit 2024, 2026 and 2027, floors of the Structural
  Foundations category, horizontal bracing, the storeys of beams and braces,
  the corner mullions and the door of a curtain wall on Revit 2027, a lone
  non-ASCII name on Revit 2023, and the doors, windows and walls of Revit 2015
  to 2023 (#1479, #1841, #1842, #1865, #1899, #1912, RE-448, RE-456).
  Against 0.21.0, 72 of the 146 measured exports differ and 37 of the 121
  `diagnostics.json` files; 56 of the 72 are Autodesk's sample projects, which
  no Revit export covers. A foundation slab and a horizontal bracing are
  checked on one reporter's counts, a roof by extrusion is held to the same
  project's 2025 save, 5 corner mullions of the 2027 `rac_advanced` stand in no
  curtain wall (#1865), and `[0.22.0]` in [`CHANGELOG.md`](CHANGELOG.md) lists
  what each change does not measure.
- **0.23.0, 2026-11-14.** It writes more of what Revit's own export writes and
  narrows rules that audits found wider than their evidence. A wall of a
  curtain wall type is an `IfcCurtainWall` on Revit 2023 to 2027 whether or not
  a mullion names it, and a curtain wall that is the host panel of another is
  not one (#1960). A roof, stair, stair flight or building element proxy with
  no predefined type of its own gets Revit's `NOTDEFINED` (#1713), a sloped
  floor gets its `Qto_SlabBaseQuantities.Depth` and `GrossVolume` (#1208), a
  door's or window's `FireRating` is read from its saved parameter (#1507), and
  a column whose saved mesh is a prism of another section than its type's
  rectangle is drawn from the mesh. The doors and windows of Revit 2015 to 2023
  cut the outline their symbol cuts, a material's colour reaches IFC viewers,
  and a file that stores no project GUID gets the project, building and site
  GlobalIds Revit's export gives them, from the GUID in its history header
  (RE-459). Against 0.22.0 all 146 measured exports differ, and 120 of the 121
  `diagnostics.json` files; 79 of the 146 have no Revit export, and on the
  other 67 no count that equals Revit's falls and none that differs rises. A
  horizontal bracing's `BRACE` type is checked on one reporter's counts of a
  structural model (#1841), `NOTDEFINED` on stairs and flights on Revit 2024
  only (#1713), a multistory stair is written once where Revit's export writes
  it once per storey (#1977, RE-463), no Revit export holds a curtain wall
  without a mullion (#1960), and `[0.23.0]` in [`CHANGELOG.md`](CHANGELOG.md)
  lists what each change does not measure.
- **0.24.0, 2026-11-14.** It closes #1713: on the 42 models with a public Revit
  export, every one of the 58,665 elements both exports hold has Revit's
  `PredefinedType`, and a Revit 2024 or later file whose export parameters
  differ between its partitions gives each element its current copy's value.
  A ramp's railings are its parts (#1988), a gutter is an `IfcFlowSegment`
  typed by an `IfcPipeSegmentType` `GUTTER` (#1989), a multistory stair is
  written once on each storey it repeats on (#1977), a ceiling in pieces has
  its piece's `GrossArea` (#1209), and a wall under a roof drawn from its
  slopes is cut along the roof's underside (#1478). Against 0.23.0, 72 of the
  126 measured exports differ, and 29 of the 102 `diagnostics.json` files; 28
  of the 72 are Autodesk's sample projects, which no Revit export covers, and
  on the other 44 no count that equals Revit's falls except those the
  changelog names (the saved-mesh `IfcCovering` row that the gutters leave, and
  the plan-outline and layer counts of the roofs now drawn from their saved
  meshes), and none that differs rises but the layer counts of 4 floors of one
  held-out Revit 2020 file (#1805). `PredefinedType` stays unset on some
  held-out pairs of Revit 2023 and earlier (counts only: 4 of 12 roofs, 1 of 5
  stairs, 1 of 5 stair flights, 12 of 60 furniture and 2 of 10 railings), a
  ceiling's piece `GrossArea` rests on one building, a multistory stair's
  copies are inferred from three groups of one building, and `[0.24.0]` in
  [`CHANGELOG.md`](CHANGELOG.md) lists what each change does not measure.

### 0.25.0: IFC geometry and element data

The geometry and element-data work that continues after 0.24.0, moved from the
0.24.0 milestone when 0.24.0 shipped. Each
slice records a baseline, an improvement and a residual against Revit's own
exports on the reference pack (#407), approximations stay labelled (#409), and
a slice ships in the next release that contains it. Geometry open now: wall
bodies thicker or thinner than their type's layers (#358) and the tops of walls whose layers Revit
ends at different heights (#1067), stair flights
(#357), roofs Revit splits into slab parts (#323), hip roofs over T and U footprints and roof cases
with no oracle (#884, #885), the openings on releases other than 2024 and 2025
(#227), the outlines of stepped rooms (#1589), beams (#94), MEP equipment and routes (#96) and ports and pipes outside
Revit 2024 and 2025 (#869), the sketch-to-solid pipeline (#156), and the Snowdon slab
that is not in Revit's export (#309); and the export time on Revit 2015 to 2023
(#1335). Element data: instance and type parameters with units and precedence
(#35, #155) and the element-record fields that carry them (#223), written to
IFC property sets and shown in the viewer; door and window sizes where the
symbol cuts no outline, measured on one building's families only
(RE-318); and ports' system classification (#834).

### 1.0.0: first-class utility

A dependable open-source Revit data reader: a new user installs it, opens a
model, gets correct elements, geometry and properties, and can reproduce the
measurements. 1.0 supports every Revit release from 2024 to 2027 and ships with
no gaps or limitations: every body is Revit's own shape, every property
Revit's export writes is written, and no release in that range is experimental.
Revit 2015 to 2023 stay readable outside 1.0, 2015 to 2022 experimental and
2023 partial, since no year among them has the three same-release Revit
exports the bar needs (the maintainer, 2026-11-08); a 1.x release adds one
once it has them. The requirements, each with its bar, measured value and
evidence, are in
[`docs/release-1.0-criteria.md`](docs/release-1.0-criteria.md), which is the
release decision; the milestone's completion percentage is not. They include
held-out models measured before fixes (#408), every release with a Revit
export of its own (#421), the surfaces the stability commitment covers (#873),
APIs documented with examples CI runs (#872), and practitioners' reports
(#874). Breadth (more releases, more categories) never lowers measured
correctness.

## Contribution Priorities

Start with [`docs/contribution-map.md`](docs/contribution-map.md). The highest
leverage work is:

1. Openly licensed models with a Revit export of the same file, and a
   held-out check on them (#407, #408).
2. Partition-stream probes that turn byte observations into falsifiable decoder
   hypotheses: one example that states the fact it proves and how to verify it
   against the corpus, and a report with measured counts.
3. Fixture assertions that prevent decoders from claiming false positives.
4. Documentation that keeps user-facing support boundaries honest.
5. Viewer diagnostics that explain what the tool could and could not decode.

## Out of Scope

rvt-rs will not:

- Use Autodesk proprietary SDK internals, leaked documents, or decompiled
  proprietary implementation code. See [`CLEANROOM.md`](CLEANROOM.md).
- Claim production RVT-to-IFC conversion before real project typed elements and
  geometry are corpus-proven.
- Provide a Revit API-compatible surface.
- Edit Revit model data at the field level before the ADR-002 gate opens.
- Resolve cloud-worksharing, licensing, or external linked-model semantics in
  the near-term product.
