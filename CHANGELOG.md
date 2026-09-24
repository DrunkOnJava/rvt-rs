# Changelog

All notable changes will be documented here. This project follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
[semver](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- **Large projects open about twice as fast.** The partition scans each
  searched every partition once per element category, once per Level and
  once per type they looked for. They now make one pass for all of them.
  `rvt-ifc` on Core Interior takes 2.75 s instead of 5.3 s, and on the 95 MB
  Snowdon Towers sample 10.1 s instead of 17.9 s. In the browser viewer,
  where WebAssembly searches without SIMD, Snowdon Towers opens in 24
  seconds instead of 56. Output is byte-identical.
- **The partition string scan is about four times faster.** It reads
  every inflated partition byte by byte for length-prefixed UTF-16 strings
  (project units, Level names). It now tests 64 offsets at a time for the
  bytes every such string must have, and checks only the offsets that pass.
  On Snowdon Towers the scan takes 0.22 s instead of 0.85 s, and it finds
  the same 644,367 strings.

### Added

- **Wall sweeps are named as Revit names them (RE-69):** "Wall
  Sweep:Base_Simple:2074039" from the one type object the sweep's record
  names, or the ElementId alone for a sweep of a wall type's own
  structure. On Snowdon Towers all 258 wall sweeps carry Revit's `Name`
  and `ObjectType`.
- **Elements whose record names no Level reach their storey (RE-68).**
  - A multistory stair takes the Level its base sits at, and its railings
    follow it.
  - The parts of a nested light fixture follow the fixture that holds
    them.
  - Slab edges, light fixtures, wall sweeps, generic models, hardscape and
    stairs take the Level their base elevation gives, within a fail-closed
    band.
  - On Snowdon Towers, the elements on no storey fall from 102 to 5, and
    none is placed on a wrong storey.
- **Model text is named as Revit names it (RE-67)**, for example "Model
  Text:10" Trebuchet MS:1448731" rather than "GenericModel-1448731". Its
  type is the one text type its record names, a type object that carries a
  font. On Snowdon Towers all 7 model texts carry Revit's `Name` and
  `ObjectType`.
- **Railings whose type has no element data are named as Revit names
  them (RE-66).** Their type's name is read from its type object, ending
  at a fixed field frame. On Snowdon Towers all 131 railings carry
  Revit's `Name` and `ObjectType`, up from 101.
- **Filter the viewer's scene tree.** A box above the tree, reached with
  /, keeps only the elements whose name, type or ElementId contains the
  text, opens the storeys and categories that hold them and counts them.
  Enter moves to the first match, and Escape clears it. Picking an element
  the filter hides clears the filter.
- **Stair parts read as their number in the scene tree** ("Run 1",
  "Stringer 2") under their stair.
- **Stairs and their parts are named as Revit names them (RE-65).**
  - A stair is "Assembled Stair:Stair:620883", and its parts are
    "Assembled Stair:Stair:620883 Run 1", "… Landing 1" and "… Stringer
    2", numbered in ElementId order among the stair's parts.
  - `ObjectType` gives each type's family and name, such as
    "Non-Monolithic Run:1/4" Tread 1" Nosing 1/4" Riser". The family is
    read from a flag in the type's own object: Assembled or Cast-In-Place
    Stair, Monolithic or Non-Monolithic Run, Stringer or Carriage.
  - A stair type lists its component types. That list settles a part
    whose record names two types or two stairs, and joins one more
    carriage to its stair.
  - On Snowdon Towers, all 26 stairs, 43 runs, 17 landings and 170
    stringers and carriages carry Revit's `Name` and `ObjectType`.

- **Curtain panels that are walls, slab edges and ramps are named as Revit
  names them (RE-64).**
  - A curtain grid cell can hold a basic wall. Its record keeps the panel
    category but names a wall type. Such a panel now takes the one wall
    type its record names, when that type has compound layers, and is
    named "Basic Wall:Type:ElementId" (#370).
  - Slab edges and ramps take their only system family: "Slab
    Edge:_Stair Landing Plate Stringer:629902" rather than
    "SlabEdge-629902".
  - On Snowdon Towers, 18 of 18 panel walls and 59 of 59 slab edges carry
    Revit's `Name` and `ObjectType`, and both ramps its `ObjectType`.
- **The viewer's scene tree groups each storey by category and leads each
  element with its type name.** A storey opens to its categories, each
  with a count, and a category to its elements, sorted by family and type.
  - An element row reads "8" Interior Partition 3 Hour" over "Basic Wall ·
    20796" rather than the whole `Family:Type:ElementId` name cut short.
  - A curtain wall's panels and mullions, and a stair's flights, sit
    under their whole. Doors and windows are listed in their own category;
    the info panel still names their host wall.
  - Selecting a category lights up its elements. Picking an element in the
    3-D view opens its category. Right and Left open and close rows, Home
    and End go to the first and last, and the tree is one tab stop.
  - The project row shows its file name rather than a full path.
- **The Categories panel counts each category's elements**, and no longer
  lists the project and storeys, which have nothing to show or hide.

- **Walls, floors, ceilings, roofs and railings are named as Revit names
  them (RE-63).** `Name` is `Family:Type:ElementId`, such as "Basic
  Wall:8" Interior Partition 3 Hour:20796" rather than "Wall-20796", and
  `ObjectType` is `Family:Type`.
  - The system family is derived by RE-61's rule and recorded as
    `FamilyNameSource` `system_family_by_category`.
  - Core Interior's 460 such elements, and every wall, curtain wall,
    ceiling and roof on Snowdon Towers, carry Revit's own name.
- **Curtain-wall parts whose record names no wall join the curtain wall
  that contains them (RE-62).** On Snowdon Towers, the mullions of the
  "Solar Panels" curtain walls name only one another. Each now takes the
  one curtain wall whose record box contains it, which is Revit's parent
  for all 89 of them. Parts with Revit's parent rise from 1,967 to 2,057.
- **Highlight a layer's material from the element panel.** Each layer
  row of a selected wall, floor, roof or ceiling is a toggle: it lights
  up every element using that material, directly or as one of its layers.
  The panel's material counts now include use in layers.
- **Layer sets are named `Family:Type`, as Revit names them (RE-61).** A
  wall's set is now "Basic Wall:Exterior - Brick on Mtl. Stud" rather than
  the type name alone.
  - Revit stores no system family name. A type with compound layers is a
    Basic Wall, Floor, Compound Ceiling, Basic Roof or Pad by its category.
  - The names equal Revit's on all 1,210 Snowdon Towers elements, on
    all of Core Interior's and RE1's, and on RE1's own four layer sets.
- **Storeys for elements whose record names no Level (RE-60, #369).**
  - A railing now takes the storey of the stair or ramp its record names.
    That is Revit's storey on 73 of 74 Snowdon Towers railings.
  - Light fixtures, generic models, slab edges and wall sweeps take the
    Level their work planes carry, but only where that is also the
    highest Level at or below their base. That is Revit's storey on 161 of
    161.
  - Snowdon's elements on no storey fall from 294 to 102, and 5,805 are
    in Revit's storey (5,612 before).
- **Elements naming two Levels are contained in their base constraint
  (RE-59, #365).** A wall, column, stair or curtain wall whose element
  record names two Levels, its base and top constraint, now goes in the
  storey of its base: the higher of the two at or below its base, else
  the lower. That is Revit's own storey on every such record measured:
  650 of 650 on Snowdon Towers, 482 of 482 on Core Interior and 7 of 7 on
  RE1.

  | file | in Revit's storey, before → after | on no storey, before → after |
  |---|---:|---:|
  | Core Interior | 853 → 854 | 1 → 0 |
  | Snowdon Towers | 3,975 → 5,612 | 1,931 → 294 |
  | RE1 | 27 → 34 | 7 → 0 |
  | the Autodesk tutorial house | no oracle | 230 → 1 |

  - On Core Interior, all 970 building elements now bind to a storey,
    each Revit's.
  - `tools/re/storeys_vs_ifc.py` compares every element's storey with
    Revit's.
- **Material names, and layer sets in the IFC export (RE-58).** A
  material's name is now read from its own data, by ElementId:
  - 219 of Snowdon Towers' 220 materials and 78 of Core Interior's 86;
  - every material of RE1 Architecture, Projeto1, teste_export_2025 and
    the Autodesk tutorial house, saved in 2024 and in 2025.

  Every one that Revit's own export styles by name has Revit's colour for
  it, 174 of 174.
  - The IFC export writes the layers of walls, floors, roofs and ceilings
    as an `IfcMaterialLayerSetUsage` over an `IfcMaterialLayerSet`, each
    layer with its material and thickness.
  - Against Revit's own exports, the layer names are Revit's, in Revit's
    order, on 3,321 of Snowdon's 3,322 layers and on all of Core
    Interior's and RE1's.
  - The sets sit on Revit's bodies within 0.001 ft on 1,196 of 1,207
    Snowdon elements, 88 of 88 on Core Interior and 15 of 15 on RE1. The
    11 Snowdon exceptions are soffit walls whose stud layer Revit cuts
    back.
  - RE1's 2025 export writes layer sets of its own, and rvt-rs writes the
    same four, with the same thicknesses in the same order.
  - Selecting a wall, floor, roof or ceiling in the viewer lists its
    layers, exterior or top first, each with its material and thickness.
  - `tools/re/layer_sets_vs_ifc.py` scores the layer sets, and
    `examples/probe_re58_material_names.rs` lists a file's material names.
- **Floors, roofs and ceilings are drawn as their layers (RE-57).** In the
  glTF export and the viewer, a Revit 2024 or 2025 floor, building pad, flat
  roof or ceiling is now its type's layers, stacked top first in their
  materials' colours, wherever they add up to its recorded height.
  - Against Revit's own IFC4 exports, every layer's bottom and top matches
    to 1.4e-6 ft on 236 of Snowdon Towers' slabs and all 68 of Core
    Interior's floors, and all 427 compared colours are Revit's.
  - `tools/re/slab_layers_vs_ifc.py` scores them.
- **Shed roofs are drawn along their slope (RE-56).** Each sketch line of a
  footprint roof carries its edge's slope angle and whether it defines the
  roof's slope. A Revit 2024 roof with exactly one defining edge now rises
  from that edge at its slope, with its type's thickness square to the
  slope, in IFC (an `IfcFacetedBrep`), glTF and the viewer.
  - It is drawn so only when the rise and thickness reproduce the roof's
    recorded height. On the Autodesk tutorial house the one sloped roof,
    at 1:12, matches to 1e-9 ft.
  - Hip and gable roofs, and roofs sloped some other way, stay level plates.
  - Roof types' layers are now read too: behind the type's name, as on
    2025 floors, whose "hyphen" framing was a type named "-".
- **Revit 2025 walls get their centreline bodies and layers (RE-55).** A
  2025 wall stores its location line, orientation and type layers as 2024
  does: a house saved in 2024 and 2025 reads the same line, flip and layers
  on all 50 of its walls, and its 45 exported walls draw identically from
  both. Against Revit's own 2025 exports, RE1 Architecture's and Projeto1's
  walls stay exact, and 2 of teste_export_2025's 3 angled walls, drawn as
  boxes up to 38 ft off before, now have Revit's faces and ends. Beams and
  sketch lines keep their 2024-only reading.
- **Wall bodies from their centreline (RE-54).** The location line a
  wall's data stores is its centreline, whatever its location-line
  setting: Revit's own body lies half the type's thickness either side of
  it on 964 of Snowdon Towers' 1,054 walls. A Revit 2024 wall is now its
  type's thickness either side of that line, where the record box does not
  already give it.
  - Angled walls, which were drawn as their whole axis-aligned box, now
    have Revit's faces on 75 of Snowdon's 102 (none before) and Revit's
    ends on the 13 whose box a rectangle of their thickness closes.
  - Walls whose box is wider than their type (such as CMU retaining walls)
    take the type's thickness. Walls Revit cut back, and leaning or
    profiled walls, keep their box.
  - More walls are drawn in their layers: 967 on Snowdon (883 before).
  - Core Interior's walls, all axis-parallel and already right, are
    unchanged.
  - `tools/re/wall_bodies_vs_ifc.py` scores an `rvt-gltf` GLB's walls
    against Revit's IFC4 bodies.
- **Walls are drawn as their layers, each in its material's colour
  (RE-53).** A wall type's data carries its compound structure: each
  layer's width, material, deck and function, exterior first. A material's
  data carries its shading colour and transparency, and a wall's data
  carries the flip that puts its exterior on the right or the left of its
  location line. The glTF export and the viewer now draw a Revit 2024
  wall as one band per layer in its material's colour, all selecting as
  the one wall.
  - Snowdon Towers: 883 of the 1,054 walls whose layers are read are drawn
    this way. Against Revit's own IFC4 per-layer solids, every layer centre
    is within 0.001 ft on 851 of 862 walls. The other 11 are one soffit
    type whose stud layer Revit cuts back.
  - All 1,368 compared layer colours are Revit's own surface style colours.
  - A wall whose layers do not add up to its body's thickness (165 on
    Snowdon, mostly soffit wraps and profiled walls) is drawn whole.
  - Floors, roofs and ceilings: their layers are read but not drawn. Revit
    2025 walls are not drawn in layers, because their location lines are
    not read. (RE-55 and RE-57 draw them, and RE-58 writes the layers into
    the IFC export.)
  - The viewer's File status reports the walls whose layers are read under
    Materials, and the export diagnostics carry the count as
    `exported.layered_element_count`.
  - `examples/probe_re53_wall_layers.rs` counts the walls drawn in layers.
    `tools/re/wall_layers_vs_ifc.py` scores an `rvt-gltf` GLB against
    Revit's IFC4 export.
- **Stair flights are drawn as their treads and risers (RE-52).** A straight
  run's data carries its plan sketch: its two sides and a line at each
  riser. The run type it names carries its tread, riser and nosing sizes and
  which parts it has. With the stair's riser height, rvt-rs draws each run
  as its steps, in IFC, glTF and the plan, instead of a box the height of
  the whole stair.
  - Snowdon Towers: 34 of 43 flights are drawn. The 30 steel-pan flights
    equal Revit's own geometry to 1e-5 ft.
  - The 4 with upright risers square the nosing, which Revit shapes with the
    type's nosing profile.
  - Monolithic, riserless and spiral flights keep their box.
  - Each flight also carries its run type's name, equal to the type part of
    Revit's own name on all 49 flights Revit exports.
  - A stair run's body is an `IfcExtrudedAreaSolid` whose `Position` sets
    its plane (`SolidShape::PlacedExtrusion`).
  - `examples/probe_re52_stair_treads.rs` lists each run type and each
    run's outcome.
- **Zoom to an element in the viewer.** Double-click an element in the tree
  or the 3-D view, press F, or use Zoom to element in the info panel to bring
  it into view. The camera keeps its angle and the rest of the model stays
  in view around a small element. Double-clicking a storey frames everything
  on it, and F with nothing selected frames the whole model.
- **The viewer colours elements by category.** Record-backed elements carry
  no material yet, so the 3D view was one grey mass. An element with no
  material of its own now takes its category's colour, in the plan's hues
  from the viewer design system's ramps:
  - amber doors, red columns and lighter red beams and members;
  - translucent blue windows and curtain panels;
  - green furniture and planting, and dark amber stairs and railings;
  - rooms drawn faint, so they no longer hide the building they fill.

  An element's own material still wins. `gltf::category_colour` gives the
  palette.
- **The viewer has a light scheme and a new look.** The browser viewer now
  follows the system's light or dark setting, on a single design system
  (`viewer/src/tokens.css`, `viewer/src/styles.css`): a black header over a
  white sheet, one blue accent for actions, links, focus and selection, pill
  buttons and hairline borders.
  - The 3D canvas, grid, lights, axes and selection highlight take their
    colours from the same tokens and switch with the scheme.
  - Every label is sans serif and sentence case, with tabular figures where
    numbers line up. The stylesheet moved out of `index.html`.
  - The demo thumbnails are flat tiles in the tint of their kind: blue for
    real projects, green for IFC, amber for families, gray for synthetics.
  - No web fonts are loaded: the viewer still makes no network requests.
- **Storeys carry their Revit Level's name, elevation and GlobalId on
  Revit 2024 and 2025 projects alike (RE-51).** RE-24's Level name block
  holds them on every project measured. Its elevation marker is six bytes
  with a per-release `u16` (the other two were the next value's), and the
  confirming copy is found by the 24 bytes before it rather than at a fixed
  distance. Levels in second-prologue frames take their partition record's
  id, and a Level inside another element is not a storey.
  - Every storey written equals one of Revit's export, name, elevation and
    GlobalId: Snowdon Towers 18 of 18 (it had 17 storeys named by
    elevation), each RE1 model 2 of 2, Projeto1 and teste_export_2025 2 of
    2, Core Interior 15 of 15 as before.
  - Elements now bind to named storeys. On Snowdon Towers, 3,379 of 3,380
    are in Revit's storey, and on RE1 Architecture 14 of 14.
  - Snowdon's structural model keeps its box-derived storeys: 8 of its
    Levels carry a flag byte no measured export explains, so its storey set
    is refused.
  - `tests/level_names.rs` checks the RE1 models in CI, and
    `examples/probe_re51_level_names.rs` lists a file's Levels and says why
    a set is refused.
- **Roofs carry their sketched outline, and sketch lines' exact ends close
  more outlines (RE-50).** A roof's sketch lines name the roof, as a floor's
  name the floor, and close into its outline. Each sketch line's own data
  also carries its exact ends, in the line record RE-49 reads for beams.
  Those ends now close outlines the box-based solve of RE-25 leaves
  ambiguous, for floors and roofs alike.
  - Snowdon Towers: 13 of the 20 roofs have an outline (none did), and 11
    of them have the area of Revit's own roof. 15 more slabs have their
    outline (132 of 199, from 117), and 11 of those have Revit's area.
  - No outline that closed before changes: recorded ends are used only
    where the boxes do not close, and only when every line of the sketch is
    level and lies in its own record's box.
  - A roof's slope is not read. A sloped roof stays a level plate as thick
    as its record box, now with its real outline.
  - `examples/probe_re50_roof_profiles.rs` lists which roofs close and why.
- **Beams run along their location lines (RE-49).** A structural-framing
  element's data carries its location line as a bounded line record
  (`04 00 08 01`, two parameters, an origin and a unit direction). With the
  element's record box it gives the beam's solid, a box
  `length × width × depth` along the line.
  - A level beam exports as its section's plan rectangle along the line,
    rotated to the line's plan angle. A sloped beam becomes its section swept
    along its centreline (`IfcFixedReferenceSweptAreaSolid`).
  - Before, every beam was the axis-aligned box around it. For a beam
    rotated in plan that box is 8.4 times Revit's own beam envelope at the
    median, against 1.11 times now.
  - On Snowdon Towers Structural, 923 of the 942 exported beams run along
    their line. Against the meshes of a later edition's VIM export, the
    section is Revit's on 613 of the 839 beams that edition kept unchanged,
    and Revit's less the top a floor join cuts on 179 more.
  - Beam ends are not trimmed at their supports (352 beams run past Revit's
    by a median 0.625 ft), and a beam whose box no solid along the line
    reproduces keeps its box (17, mostly oblique beams with skewed ends).
  - The property set gains `AxisLengthFeet`, `SectionWidthFeet` and
    `SectionDepthFeet`, and `BodySource` reads `partition_beam_axis`.
  - `rvt::partition_beam_axes` exposes the rule, and
    `examples/probe_re49_beam_axes.rs` scores it against a VIM export.
- **Elements, rooms and storeys carry the GlobalId Revit's own exporter
  gives them (RE-48).**
  - `Global/History` lists every editing episode's GUID, and each
    `Global/ElemTable` record holds its element's episode number.
  - Revit's GlobalId is that GUID with the element's ElementId XORed in.
  - Every element rvt-rs exports that Revit's export holds now has Revit's
    GlobalId: 854 of 854 on Core Interior, all 281 on the RE1 models, and
    5,945 of 5,945 on Snowdon Towers, plus Core's 116 rooms and 15 storeys.
  - IFC from rvt-rs and from Revit now diff cleanly against each other, and
    BCF issues and clash results carry over.
  - Revit 2023 and earlier keep generated GlobalIds.
  - `rvt::revit_global_ids` exposes the rule, `IfcModel::global_ids` carries
    it, and `examples/probe_re48_revit_global_ids.rs` compares it with any
    Revit export.
- **Walls, floors, ceilings, roofs and railings carry their type's name
  (#322, RE-44).** A system-family type has no RE-38 name entry, but its
  serialised element data names it. The data opens with
  `ff ff ff ff · u16 · 01 00 00 00` and the type's ElementId, and the first
  framed string after that is the name.
  - The element's type is the one RE-28 type record its reference list
    names. The element-record property set gains `TypeName`.
  - Every name written equals Revit's for the same `Tag`: 460 on Core
    Interior, 1,545 on Snowdon Towers, 16 on RE1 Architecture and 1 on
    Projeto1.
  - On `teste_export_2025` six of seven are equal. The seventh wall's
    frame, data and bounding box all describe the 200 mm type, so that export
    was made after the wall was changed.
  - The system family's own name ("Basic Wall") is not in the file, since
    Revit writes it in the UI language. The IFC `Name` therefore stays
    `Class-ElementId` and no `FamilyName` or `ObjectType` is written.
  - `examples/probe_re44_system_type_names.rs` lists the names for any file,
    and `tests/element_names.rs` checks Core Interior and RE1 against
    Revit's exports.
- **Revit's IFC export overrides are honoured, the element's own and its
  type's (RE-45).** "Export to IFC As", "IFC Predefined Type" and their
  type-level twins are parameter entries in the element's serialised data
  (`i64 BuiltInParameter · u32 n · UTF-16`), read on Revit 2024 and 2025.
  - An element exports as its own override, else its type's (`IfcCoveringType`
    names `IfcCovering`). The predefined type is written when it is an IFC4
    enumerator of that entity.
  - On Core Interior two floors carrying `ifcSlab` / `ROOF` are now
    `IFCSLAB … .ROOF.` as in Revit's export, and every element has Revit's
    entity.
  - On `teste_export_2025` the three walls of a type exported as
    `IfcCoveringType` / `CLADDING` are `IFCCOVERING … .CLADDING.`.
  - `IfcSlab` and `IfcCovering` join `IfcShadingDevice` as honoured targets.
  - `examples/probe_re45_ifc_export_parameters.rs` lists a file's overrides,
    and `tests/ifc_export_overrides.rs` checks Core.
- **Curtain walls export as `IfcCurtainWall` aggregates of their panels and
  mullions (RE-46).** A wall a curtain-wall mullion names in its reference
  list is a curtain wall: exactly Revit's 42 on Snowdon Towers and 1 on RE1.
  - It exports as a bodiless `IfcCurtainWall` `.NOTDEFINED.`, as Revit's
    does, instead of a wall box drawn over its own glazing.
  - Each panel and mullion naming exactly one curtain wall is its
    `IfcRelAggregates` part: 1,798 of Revit's 1,906 on Snowdon and 12 of 13
    on RE1. Late mullions that name no curtain wall, panels Revit writes as
    nested curtain walls, and doors stay standalone.
  - `examples/probe_re46_curtain_walls.rs` lists them, and
    `tests/curtain_walls.rs` checks RE1.
  - A stair or curtain wall that no part names keeps its own body. Snowdon
    stair 1603717, a stair family placed on its own, had been exported with
    no body since RE-39.
  - The export diagnostics count `building_elements_carried_by_parts`, and
    `rvt-ifc` prints it ("6037 with geometry and 68 carried by their
    parts"), so aggregate wholes no longer read as missing geometry.
- **Stairs and flights carry their riser count, riser height and tread
  length (RE-47).** A stair's element data holds its riser height, tread
  depth and number of risers, and a run's data holds its number of risers.
  - They are written as `Pset_StairCommon` / `Pset_StairFlightCommon`, with
    `NumberOfRiser` as `IfcCountMeasure` and `RiserHeight` / `TreadLength`
    as `IfcPositiveLengthMeasure`, as in Revit's export.
  - On Snowdon Towers every stair's values equal Revit's (26 of 26), and
    flights' on 37 of 43. On the other 6, Revit writes the stair's total on
    each flight of a stair it splits in two, and rvt-rs writes each run's own
    count.
  - The IFC model gains `IfcEntity::ElementPropertySet` for an element's
    further property sets and `PropertyValue::PositiveLengthFeet`. The viewer
    panel payload gains `further_property_groups`, which the viewer shows.
  - `examples/probe_re47_stair_dimensions.rs` lists them.
- **Type records are read on Revit 2025.** `partition_type_records` read
  Revit 2024 only. The same records on 2025 carry the 2025 marker and
  prologue constant, and most are second-prologue frames that take their
  partition record's id (RE-35). `decode_at_with_marker` and
  `find_type_records_with_marker` take the release's bbox marker.

### Fixed

- **The element panel labels the ElementId as ElementId**, not GUID, and
  shows Revit's own GlobalId (RE-48) beside it where the file yields one.
- **The IFC export's materials are now the file's own materials (#34).**
  On Revit 2024 and 2025 files, `IfcMaterial` now lists each material the
  file declares whose name is read (RE-58), each with its shading colour.
  It used to list every material-like string found in the file:

  | file | materials written before | now |
  |---|---:|---:|
  | Snowdon Towers | 946 | 219 |
  | RE1 Architecture | 775 | 69 |
  | the Autodesk tutorial house | 686 | 171 |
  | Core Interior | 104 | 78 |

  - Every material Revit's own export writes is among them, except two on
    Snowdon, and each colour both exports write is Revit's.
  - The viewer's Materials row lists these names.
  - Earlier releases keep the string heuristic.
- **A material's colour was read from a misaligned frame when a longer
  run of `ff` bytes preceded its shading frame.** The early read's floats
  are subnormal and its colour a byte-shifted or black one: Core Interior's
  Glass read black instead of Revit's 0000ff at 0.75 transparency. Frames
  with subnormal floats are now rejected; 3 to 6 materials per file change,
  and Glass now equals Revit's style.
- **Orbiting the viewer no longer changes the selection.** An element was
  picked on every mouse press, so each drag that started on an element
  selected it. It is now picked on a click: a press and release within a few
  pixels.

- **The viewer and the plan export draw each element's real body.** The GLB
  the viewer shows (and `rvt-gltf` writes) drew every element as an
  unrotated box of its width, depth and height, and the plan SVG drew
  unrotated rectangles. A rotated wall ran along the wrong axis, a
  sketched slab filled its bounding box, and a sloped or swept solid was
  not drawn at all.
  - Both now draw the body the IFC export gives the element
    (`rvt::ifc::body_geometry`), rotated by the element's rotation: a
    sketched profile with its holes open, a steel section as its section,
    and swept, revolved and brep solids as those solids.
  - An element with no body of its own, such as a stair or curtain wall
    carried by its parts, is no longer drawn as a phantom 1 ft cube.
  - The GLB is now Y-up in metres, as glTF defines it. It was Z-up in feet,
    so every model lay on its side, 3.28 times too large, in the viewer,
    Blender and any other glTF tool. One root node carries the conversion,
    and element nodes stay in the model's own frame.
  - The plan draws slabs, roofs, coverings, plates and spaces beneath the
    walls, columns and doors instead of over them.
  - The Khronos glTF validator reports no errors or warnings on Core
    Interior, RE1 Architecture and Snowdon Towers.
- **The viewer uses one sans-serif face and sentence-case headers.** Panel
  values and the diagnostics JSON no longer switch to a monospace face,
  section headers are bold instead of spaced capitals, and IFC types read
  `IfcWall` rather than the STEP file's `IFCWALL`.
- **Property text with non-ASCII characters is written as ISO 10303-21
  escapes.** `IfcPropertySingleValue` text, such as a type name from a
  Portuguese model ("Genérico - 200 mm"), went into the STEP file as raw
  UTF-8, while every other string used `\X2\…\X0\` escapes. It now uses
  the same encoder. On `teste_export_2025` the export has no raw non-ASCII
  bytes left, and IfcOpenShell reads the names back unchanged.

- **Python wheels install on older and more Linux systems, and on Intel
  Macs.** 0.2.0's Linux wheel was built on the runner's own Ubuntu and came
  out `manylinux_2_35`. On RHEL 8 and 9, Amazon Linux 2023, Debian 11 and
  Ubuntu 20.04, pip fell back to compiling the sdist, which needs Rust.
  - Release wheels now build in the manylinux2014 and musllinux_1_2
    containers, for x86_64 and aarch64, plus Intel macOS: seven wheels
    instead of three.
  - Each wheel is installed in the oldest environment its tag claims (the
    manylinux2014 image with its oldest CPython, Alpine, an Intel macOS runner),
    where it opens a Revit file and exports IFC before anything is published
    (`tools/ci/wheel-smoke.py`).
  - The PR wheel job builds the same way, so a break in the glibc 2.17
    build fails on the PR.
- **The PyPI upload can no longer pick up the CLI archives.** The publish
  jobs downloaded every artifact of the run and uploaded every `*.tar.gz`,
  which includes the release-binaries archives once they finish. On v0.2.0
  the PyPI job downloaded seven seconds before the first one did. They now
  download only the wheels and the sdist, and check that there are seven
  and one.

## [0.2.0] — 2026-09-23

An **inspection-focused alpha** (see
[`docs/release-0.2.0-plan.md`](docs/release-0.2.0-plan.md)). rvt-rs remains a
Revit inspection / reverse-engineering toolkit with experimental export —
**not** a production Revit→IFC converter for arbitrary projects. The first
release with prebuilt CLI archives for Linux, macOS and Windows on the GitHub
Release, and a multi-arch container image on ghcr.io.

### Added

- **Slabs sketched as separate pieces export one element per piece
  (#331).** A plan profile now holds every separate loop of a sketch as its
  own piece, with the voids inside it. The exporter writes each further
  piece as another element with the element's `Tag`, named `…:2`, `…:3`,
  as Revit's own export does.
  - On Snowdon Towers the five such slabs match Revit's pieces. Four match
    piece for piece by area (2 × 1.60, 2 × 1.99, 2 × 0.38 and 9.02 + 9.76
    m²). The fifth has three pieces, which Revit writes as one slab with
    three tessellated bodies.
  - A sketch whose loops cross each other still gives no profile.

- **`rvt-ifc` says what it wrote.** After the IFC is written, it prints the
  number of building elements, how many have geometry and on how many
  storeys; the six largest IFC types; what it left out on purpose, as Revit's
  own export does (2D-only families, non-primary design options); and the
  readiness level, with a pointer to `--diagnostics` for the full report. It
  prints on stderr, so a pipeline reading stdout is unaffected.

- **Families that nest others are named too (RE-42, #324).** A type's
  partition record names its family, and also any family of the same
  category nested in it, so RE-38 left such elements with class-and-id
  names. The family is the one candidate whose own record names every
  other candidate, and `partition_names::resolve_family` now picks it.
  - Every name this adds is Revit's own: RE1 Electrical's 12 lighting
    fixtures, 6 duct fittings on RE1 Mechanical, 117 elements on Snowdon
    Towers (counter tops with sinks, kitchenettes, trees, dining sets), and
    50 on the Snowdon structural sample against the VIM export.
  - Across Core Interior, Snowdon Towers and the RE1 models, 4,397 element
    names now equal Revit's for the same `Tag`, with no name that differs.
  - `tests/element_names.rs` now checks RE1 Electrical, and
    `examples/probe_re42_nested_families.rs` measures the rule.

- **Elements declared by the ElemTable's second id are attributed (RE-41).**
  A 40-byte `Global/ElemTable` record carries two ids, at `+16` and `+36`.
  rvt-rs declared only the first. Where the two differ, the second is the
  id of the element's partition record and the `Tag` Revit's own IFC export
  writes; the first is neither. `elem_table::declared_ids` now declares
  both, and every production path uses it.
  - On Snowdon Towers Architectural the 37 element records that were
    unattributed (16 walls, 4 columns, 17 generic models) now export, all
    of them elements Revit's export holds, and
    `element_record_without_element_id` is absent. Every wall, column and
    generic model of Revit's export is attributed. Snowdon Structural gains
    one floor, which the VIM export also holds.
  - No licensed file in CI has a record whose ids differ, and their
    exports are unchanged. `examples/probe_re41_elem_table_secondary_ids.rs`
    measures it.

- **Design options export as Revit exports them (RE-40, #319).** An element
  record holds its design option at `+0x2a`, and a design option set's name
  entry closes with the ElementId of the set's primary option. The exporter
  now leaves out elements in a set's other options, as Revit's own IFC
  export does by default, and reports them as the skipped item
  `element_record_in_non_primary_design_option`.
  - On Snowdon Towers, the 29 placed elements in a primary option are all in
    Revit's IFC4 export and the 27 in non-primary options are all missing
    from it. Leaving them out removes 18 exported elements that Revit's
    export does not hold, and none that it does.
  - Slab edges (`OST_EdgeSlab`) export as `IfcBuildingElementProxy`: all 59
    on Snowdon are in Revit's export.
  - A set whose name entry is missing or ambiguous keeps all its options.
    `RevitFile::design_options` and `PartitionElementRecord::design_option`
    expose what is read, and `examples/probe_re40_design_options.rs` prints
    it.

- **Roofs export as `IfcRoof` (#323).** All 20 placed roofs on Snowdon Towers
  are `IfcRoof`s in Revit's own IFC4 export. Revit decomposes 7 of them into
  a same-`Tag` `IfcSlab` `.ROOF.` part. No decoded byte says which roofs
  those are, so rvt-rs writes each roof as one `IfcRoof` with its record's
  box. The other test files have no roof records and export unchanged.

- **Stairs export as aggregates of their runs, landings and stringers
  (RE-39, #323).** Each stair is an `IfcStair` with no body of its own. Its
  runs (`IfcStairFlight`), landings (`IfcSlab` `.LANDING.`) and stringers
  (`IfcMember` `.STRINGER.`) name it in their reference lists and are
  related to it by `IfcRelAggregates`, as in Revit's own export. On Snowdon
  Towers:
  - the same 26 stairs aggregate as in Revit's IFC4 export;
  - every one of the 228 parts rvt-rs attaches is in the matching Revit
    aggregate;
  - all 256 new elements are elements Revit exports.
  - Railings stay standalone. Revit aggregates 65 of the 70 that name a
    stair, and no byte read so far tells the other five apart.
  - `IfcEntity::Aggregate` carries the relation, and the STEP writer leaves
    a part out of the storey containment it reaches through its whole.
    `examples/probe_re39_stair_parts.rs` measures the link.

- **Elements carry Revit's own family and type names (RE-38).** Revit 2024
  and 2025 partitions hold a name entry for every loaded family and type:
  `u64 ElementId · u32 length · UTF-16 name · i64 BuiltInCategory`. An
  element's type is the one reference of its own category that carries an
  entry, and the type's partition record names its family the same way.
  - The IFC `Name` of such an element is now `Family:Type:ElementId` and its
    `ObjectType` `Family:Type`, as in Revit's own export, and the
    element-record property set gains `FamilyName` and `TypeName`. Every one
    of the 4,241 names and object types written on Core Interior, Snowdon
    Towers, the RE1 models and `teste_export_2025` is exactly Revit's.
  - On Snowdon every name entry that the VIM export also carries has the
    VIM's name and category (348 of 348).
  - Walls, floors, roofs and other system families keep the
    `Class-ElementId` name: their types are not in these entries.
  - `rvt::partition_names` and `RevitFile::element_names` expose the table.
    `examples/probe_re38_names.rs` reports it for any file, and
    `tests/element_names.rs` checks every name against Revit's export.

- **Lighting fixtures, air terminals, food-service equipment, planting,
  parking, entourage, hardscape, elevators and ramps from element records
  (RE-37).** They export as the entities Revit's own IFC4 export writes for
  them: `IfcLightFixture`, `IfcAirTerminal`, `IfcElectricAppliance`,
  `IfcBuildingElementProxy`, `IfcTransportElement` and `IfcRamp`. Every
  exported element is one Revit's export also holds, and each entity is
  complete:
  - Snowdon Towers: 447 lighting fixtures, 26 food-service items,
    157 site elements, 2 ramps and 2 elevators (local only);
  - RE1 Mechanical: 7 air terminals;
  - RE1 Electrical: 12 lighting fixtures, the first elements it exports.
  - CI tier 2 now also fetches RE1 Electrical, and
    `tests/element_records_2025.rs` checks both models.
  - Electrical fixtures, lighting devices and slab edges are measured but
    not added: Revit's export leaves out some of them, the slab edges
    because they sit in a non-primary design option (#319). Fire alarm and data
    devices and electrical and mechanical equipment are also left out,
    since their IFC4 entity is unmeasured.

- **Structural framing, structural columns, structural foundations and
  generic models from element records (RE-36).** They decode under the same
  instance rule as walls and doors, with their ElementIds from the enclosing
  partition record (RE-35), and export as `IfcBeam` (`.BEAM.`), `IfcColumn`
  (`.COLUMN.`), `IfcFooting` and `IfcBuildingElementProxy` with
  bounding-box bodies.
  - On Autodesk's Snowdon Towers 2024 structural sample (measured locally),
    every exported id its VIM export carries has the record's category
    there: 914 framing members, 74 columns, 90 foundations and 91 generic
    models, none under another category. The 28 framing ids the VIM lacks
    are beam-system members its later edition regenerated.
  - On the architectural sample 260 of 261 generic models are in Revit's own
    IFC4 export; the other sits in a non-primary design option.
  - `examples/probe_re36_vim_oracle.rs` reads a VIM file (`vim-format`, MIT)
    and scores every placed-instance record of a model against it.
  - Rebar, structural connections and beam systems have correct ids but are
    not exported, and braces would export as `IfcBeam`.

- **Frames holding the invalid ElementId take their record's id (RE-43).**
  Some element frames hold `u32::MAX`, the 32-bit form of Revit's invalid
  ElementId −1, at `+0x00`. They carry no id of their own either, and now
  take their enclosing partition record's id like other second-prologue
  frames (RE-35).
  - On Snowdon Towers this recovers stair run 1644693. Every element of
    Revit's own export in the entity types rvt-rs writes is now exported.
  - It also recovers 99 sketch lines, so 18 slabs get their sketched plan
    profile; 16 of them have exactly Revit's area per solid.

- **A sketch of separate pieces no longer yields voids outside the outer
  loop (RE-43).** `plan_profile_from_segments` took every loop but the
  largest as a void, even one outside it, which gave five Snowdon slabs an
  invalid `IfcArbitraryProfileDefWithVoids` with near-zero area. A loop
  outside the outer one is now a piece of its own (#331, above), and a loop
  that crosses another means no profile.

- **Records with no ElementId at `+0x00` take it from their partition record
  (RE-35).** Every `Partitions/*` stream opens with a chain of
  records, one per element: `u64 ElementId · u32 size · u16 prologue constant
  · u16 count`, the body, then a trailer that repeats the size. A record that
  carries its ElementId at `+0x00` *is* the start of its record. A
  second-prologue frame (RE-30) sits inside its element's record, so its id is
  the record's.
  - An interim reference-order inference (RE-34) was replaced before this
    release: on a Revit 2025 project it named a door after its type,
    because partitions are several ascending runs, not one.
  - All 30,432 first-prologue records across nine 2024 and 2025 files start a
    record of their own id, and the chains hold every declared ElementId on
    Core Interior and the RE1 models.
  - The RE1 Architecture, Mechanical and Plumbing models match every entity
    rvt-rs recovers in Revit's own export: all doors, the curtain wall, all
    66 ducts, pipes and fittings, and all 117 pipes and fittings. Snowdon
    Towers Architectural exports 4,795 elements from these categories
    instead of 80; 4,678 are in Revit's export, including every door,
    window, curtain wall, railing, ceiling, furniture item and fixture, and
    1,062 of 1,078 walls.
  - Floors, building pads and ceilings in that layout get their ids too;
    Snowdon exports 176 of Revit's 200 slabs.
  - Records after the chain belong to loaded families' own documents and no
    longer count as unattributed model elements. On Snowdon the
    `element_record_without_element_id` count is 20 (16 walls, 4 columns),
    exactly what rvt-rs still cannot attribute.
  - Core Interior exports byte-identically.
  - `PartitionElementRecord::id_from_enclosing_record` says which kind of
    frame an id came from.
  - `partition_record_chain`, `enclosing_record` and `record_prologue_constant`
    are public. `examples/probe_re35_record_wrapper.rs` reports the chain for
    any file, and `tests/partition_record_chain.rs` holds the first-prologue
    identity on the project corpus.

- **Thirteen more categories from Revit 2024 and 2025 element records
  (RE-33).** Furniture, casework, plumbing fixtures, specialty equipment,
  ceilings, curtain-wall mullions and panels, railings, wall sweeps, ducts,
  duct fittings, pipes and pipe fittings now decode under the same instance
  rule as walls and doors. They export as `IfcFurniture`,
  `IfcSanitaryTerminal`, `IfcBuildingElementProxy`, `IfcCovering`,
  `IfcMember`, `IfcPlate`, `IfcRailing`, `IfcDuctSegment`, `IfcDuctFitting`,
  `IfcPipeSegment` and `IfcPipeFitting`, with bounding-box bodies.
  - Every element exported from a record carrying its ElementId is one
    Revit's own export of the same file also holds: zero false positives on
    the MIT RE1 Architecture, Mechanical and Plumbing models (Revit 2025)
    and on Snowdon Towers (Revit 2024). RE-35 above extends the categories
    to second-prologue records.
  - RE1 Architecture gains all 58 of its elements in these categories.
    Recall on the MEP models and Snowdon is partial, because most of their
    records use the second prologue (RE-30).
  - CI tier 2 now also fetches RE1 Mechanical and Plumbing, and
    `tests/element_records_2025.rs` checks every category against them.
    `examples/probe_re33_category_census.rs` is the census.

- **Revit 2025 projects export walls, slabs and rooms (RE-32).** Element
  records in 2025 files have the 2024 shape, but the 8-byte marker in front
  of the bounding box changes with each release. Its tail is the release's
  `Global/ElemTable` header constant + 40 (`0x05ab` on 2024, `0x05d3` on
  2025), and a prediction for 2023 made from that rule checked out.
  - `partition_element_records::bbox_marker(release)` supplies the marker,
    and 2025 joins the supported releases.
  - Against Revit's own exports of six 2025 files, the decode makes no false
    positives. On the MIT-licensed `Drshelden/IFC-ECS` RE1 architecture model
    it reproduces the export's 7 wall and 2 slab ElementIds and 11 rooms.
  - Doors and windows mostly use the second record prologue (RE-30), so they
    take their ElementIds from their partition records (RE-35 above).
  - Room names, storeys and wall types remain 2024-only.
  - `tools/fetch-corpus.sh` fetches the RE1 models, and
    `tests/element_records_2025.rs` checks them.
- **Release archives and the container image carry build-provenance
  attestations.** The release workflow attests every CLI archive before
  attaching it, and it attests the pushed image digest into ghcr.io
  (`actions/attest`), so anyone can check that a download was built from
  this repository's tagged commit:
  - `gh attestation verify <archive> -R DrunkOnJava/rvt-rs`
  - `gh attestation verify oci://ghcr.io/drunkonjava/rvt-rs:<version> -R DrunkOnJava/rvt-rs`

  PyPI wheels already carry PEP 740 attestations through trusted publishing.

- **A container image of the CLIs.** Tagged releases push
  `ghcr.io/drunkonjava/rvt-rs:<version>` (and `:latest` for a final release)
  for `linux/amd64` and `linux/arm64`: a distroless static base holding the
  release's own static musl binaries, so the image compiles nothing and the
  arm64 build needs no emulation. `docker/stage.sh` checks the archives
  against `SHA256SUMS` before they go in, the release-binaries workflow
  smoke-tests the image on every run (including pull requests that touch
  `docker/`), and `docs/install.md` gains a Docker section. Thanks to
  @Simon-Weij, whose #283 proposed publishing to ghcr.io.
- **An export now says when most of a file's elements could not be read.**
  On Autodesk's Snowdon Towers 2024 samples most wall, door, window, column,
  floor and room records use an element-record layout whose ElementId is
  not located (RE-30), so the fail-closed decode skips them and the IFC
  carries a small fraction of the building while looking complete. The
  export diagnostics now count those records per class as a
  `skipped` item with reason `element_record_without_element_id`, and add a
  warning, first in the list, that the model is incomplete — the one the
  viewer's Warnings row shows inline, and which `rvt-inspect`, `rvt-ifc
  --diagnostics` and Python's `export_diagnostics()` all carry. On the
  architectural sample that is 2,043 records (1,216 walls, 245 doors,
  198 floors, 180 columns, 150 windows, 54 rooms); on `2024_Core_Interior.rvt`
  and every project-count fixture it is zero, which
  `tests/project_count_fixtures.rs` now asserts. The "Suppressed N
  low-confidence schema scan candidates" warning also read the first
  skipped item whatever it was, so on a file with below-confidence elements
  it reported that count instead; it now reads its own item.
- **Every `Global/ElemTable` record now says which element it belongs to
  (#152, RE-31).** The run of `0xFF` bytes the layout detector anchors on is
  an owner ElementId field, and `0xFF` is its unset value. It is a `u32` at
  `+0` on 2023 projects and a `u64` at `+4` on 2024 projects.
  `ElemRecord::owner_id` reads it. On `2024_Core_Interior.rvt` it is set on
  22,368 of 26,425 records, every value is a declared ElementId, and 87 %
  also appear in the element's own partition reference list. By category,
  curtain panels, mullions and grids name their curtain wall, sketch lines
  their sketch, and grouped elements their model group. `rvt-elem-table`
  prints the detected layout instead of guessing it from the first record,
  which went wrong whenever that record had an owner, along with how many
  records name an owner. `--json` adds `layout`, `records_with_owner` and
  `owner_id`. The IFC export does not use the field yet.
- **Element and room schedules for Excel, from the CLI, the viewer and
  Python.** `rvt-schedule model.rvt` writes `model.elements.csv` — one row
  per decoded building element with its Revit ElementId, IFC type, level and
  elevation, material, placement, body size, the host wall of each door and
  window, and the body's provenance — and `--schedule rooms` writes the
  room number, name and level. `--metric` switches lengths to metres,
  `--excel` adds the byte-order mark Excel on Windows needs for non-ASCII
  names, `-o -` prints to stdout. On `2024_Core_Interior.rvt` that is 970
  element rows (360 walls, 132 doors all hosted, 6 windows, 256 columns, 80
  slabs, 116 rooms) and 116 room rows, pinned by
  `tests/rvt_schedule_cli.rs`. The same `rvt::ifc::schedule_csv` module
  backs the viewer's new **Download element / room schedule (CSV)** buttons
  (wasm `scheduleCsv`) and Python's `RevitFile.schedule_csv()`. Only decoded
  values are written — unknowns are empty cells — and rooms carry no area,
  because their bodies are record bounding boxes. Cells come from an
  untrusted file, so a text cell a spreadsheet would read as a formula is
  prefixed with `'` (OWASP CSV injection); the older
  `scene_graph::Schedule::to_csv` gets the same guard.
- **Every file now says who saved it, when, and whether it is workshared,
  and `rvt-info` inventories whole folders.** After its binary header,
  `BasicFileInfo` carries a block of `Key: value` lines on every release
  2016-2026 — `Worksharing`, `Username`, `Central Model Path`,
  `Last Save Path`, `Unique Document GUID`, `Unique Document Increments`,
  `IsSingleUserCloudModel`, `Author`, `ClientAppName` and more — which a
  plain UTF-16LE decode misses: the block starts at an odd byte offset on
  2026 and both magnetar projects, and `IsSingleUserCloudModel` is a narrow
  `"False\0"` packed into wide text. `BasicFileInfo::properties` keeps every
  line verbatim, with typed accessors (`worksharing()`, `username()`,
  `central_model_path()`, `document_increments()`, …), pinned on all 11
  `phi-ag/rvt` releases by `tests/samples.rs`. Project files' `ProjectInformation`
  stream — a one-entry ZIP holding an Atom `project.xml` — is now parsed
  too, giving the save timestamp; the entry's own name embeds the saving
  machine's user folder and is never surfaced. The new `rvt::metadata`
  module combines both into `FileMetadata`, and `read_metadata(path)` reads
  only those two streams from disk. `rvt-info` gains a Document section,
  accepts several files and folders (recursive, `--no-recurse`), and prints
  an inventory — release, worksharing, last saved, size — as a table,
  `-f csv`, `-f json` or `-f jsonl`, marking Revit backup copies and giving
  unreadable files an error row (exit status 1). `--redact` now also scrubs
  the last-saved-by user inside local-copy file names (`<central>_<user>.rvt`).
  Python gains `RevitFile.metadata()` and `rvt.read_metadata(path)`, the
  wasm build `fileMetadata(bytes)`, and the viewer's File status panel
  Saved and Worksharing rows. `PartAtom::entry_title` is the Atom entry's
  own title (the document name); `title` keeps its old last-title-wins
  value for compatibility. Open errors now name the file and say when the
  path is a directory. New schemas: `file-metadata.schema.json`,
  `inventory-row.schema.json`; `summary.schema.json` gains `document`.
- **Rooms come from `OST_Rooms` element records, with their real Revit name,
  number, storey and envelope (#90, RE-29).** Revit rooms are carried by the
  same partition element record every other category uses —
  `BuiltInCategory` `OST_Rooms` (-2000160) — under the unchanged #211
  instance rule. The category was **measured, not assumed**: a brute scan of
  all 23 470 decodable element records on `2024_Core_Interior.rvt`
  histogrammed every `BuiltInCategory` present and scored each one's
  selection against the 116 room ElementIds Revit's own full-project export
  carries in `IfcSpaceType.Tag`; `OST_Rooms` selects **116 and they are
  exactly that set** (FP 0, FN 0, tolerance 0), and no other category
  intersects it at all. The number, the name and the host Level come from the
  room's own parameter block, anchored on the owning ElementId with a
  confirmation copy at `+0x3c`, the Level at `+0x44`, the number's UTF-16LE
  length prefix at `+0x1f5` and the name's 8 bytes past the end of the number
  — the same owner-at-a-fixed-offset framing RE-22 found for the per-instance
  `IFC Export As` override, read forward instead of backward. It accepts
  **116 of the 26 425 ids `Global/ElemTable` declares** and they are exactly
  the rooms; every room is framed twice and no pair disagrees; recovery is
  all-or-nothing per room. Measured against the reference export: **116 of
  116** names equal `IfcSpace.LongName` and **116 of 116** numbers equal
  `IfcSpace.Name`. Storey containment is exact too — all 116 bind through the
  RE-27 Level reference, each of the ten Levels named maps to exactly one
  `IfcBuildingStorey`, and it is the storey that export aggregates the room
  into. Emitted `IFCSPACE` goes **18 → 116**, storey containment **853 → 969
  of 970**, and `entity_counts.IFCSPACE` joins the full-project OctetProof
  claimed surface (13 → 14 fields), where rvt-rs, IfcOpenShell 0.8.5 and
  IFClite 7.1.1 agree exactly.

- **A room's body is its record's bounding box, and that box is the
  reference export's plan envelope and floor-to-ceiling extent (#90,
  RE-29).** The "placeholder 10×10×8 ft bounding box" #90 describes — in
  practice 18 rows named `Room-unnamed` with no body at all — is replaced by
  a measured one. Scored on the emitted IFC against Revit's own swept areas
  in world coordinates: the plan envelope is exact on **116 of 116** (worst
  1.640e-06 ft, the writer's six-decimal metre rounding) and the vertical
  extent is exact on **116 of 116** (worst 1.421e-14 ft), the base sitting on
  the storey elevation and the top 8 ft above it. The RE-26 newest-frame
  tie-break is load-bearing here: 111 of the 116 rooms carry frames that
  disagree about the plan box, and the first frame by `(stream, offset)` is
  exact on only 5 of 116. `partial_element_geometry` disappears from the
  diagnostics: every one of the 970 emitted building elements now carries a
  body.

- **Storey containment from the Level ElementId the element record names
  (#219, RE-27).** A partition element record's counted reference list at
  `+0x88` — the same list RE-23 reads for a door's host wall — carries the
  Revit `Level` that hosts the element as a plain ElementId slot. Accepted
  only when the list names **exactly one** of the recovered Levels: a Revit
  column or wall carries a base *and* a top constraint and both are in the
  list, so all 344 `OST_Columns` records that name a Level name two of them
  and resolve to nothing, keeping the elevation join they already bind
  exactly on. Measured on `2024_Core_Interior.rvt`: where both joins answer
  the same element they agree **537 of 537**, 0 disagreements, so the stated
  join runs first and the inferred one fills in behind it. Containment moves
  **801 → 853 of 872** emitted building elements — all 256 columns, 132
  doors, 6 windows, 100 record-backed plates and 359 of 360 walls. The 46
  plates that sat 0.1667 ft below their level at the structural-slab /
  architectural-topping interface and the 6 windows whose record base is a
  4.73 ft sill height are exactly what a named Level reaches and an inferred
  elevation cannot. `LevelBindResolved` is now `true` on those elements, with
  `LevelElementId` and `LevelBindSource` beside it; `StoreyBindSource` gains
  `record_level_reference`. Reference side **NOT MEASURED** at the time —
  #90 / RE-29 has since scored the 116 rooms against Revit's own
  `IfcRelContainedInSpatialStructure` (116 of 116 on the right storey) and
  taken containment to 969 of 970; the other 853 bindings stay unmeasured.
- **Tagged releases attach prebuilt CLI archives for Linux, macOS and
  Windows.** Until now the only way to run `rvt-info`, `rvt-inspect` or
  `rvt-ifc` was to install Rust and compile. The new reusable
  `release-binaries.yml` builds every `[[bin]]` for
  `x86_64-unknown-linux-musl` and `aarch64-unknown-linux-musl` (static, on
  native x86_64 and arm64 runners), `aarch64-apple-darwin`,
  `x86_64-apple-darwin` (cross-compiled) and `x86_64-pc-windows-msvc`
  (static C runtime), packages each with `README.md`, `LICENSE` and
  `NOTICE`, smoke-tests the four natively runnable archives (`gen-fixture`
  → `rvt-info` → `rvt-inspect` → `rvt-ifc`), and writes `SHA256SUMS`.
  `publish.yml` calls it on `v*` tags to attach everything to the GitHub
  Release, gated only on the source smoke test so a registry failure never
  withholds the binaries; pull requests touching the workflow or the Cargo
  manifests run the whole matrix without uploading, and `workflow_dispatch`
  builds any commit. `docs/install.md` gains a Prebuilt Binaries section
  (checksum verification, macOS quarantine, Windows) and the release
  checklist a post-publish check.

### Changed

- **`IfcSpace.LongName` now carries the room's Revit name (#90, RE-29).**
  The STEP writer wrote the element `Name` into both `IfcSpace.Name` and
  `IfcSpace.LongName`; `LongName` is now the recovered `RoomName` property
  when a room has one, which is the slot Revit's own exporter puts the room
  name in. `Name` keeps the `Room-<ElementId>` identity every other class
  uses, because `IfcSpace` declares no `Tag` and the STEP line would
  otherwise carry no source id at all; the number is a `RoomNumber` property
  beside it. A space with no recovered name is unchanged.

- **The string-derived rooms stand down where element records decode (#90,
  RE-29).** `partition_schema_mvp` kept the `partition_name_candidates` room
  scan as the only room path; it is now superseded by the record-backed
  rooms on any file where they decode, the same trade the plan-loop floors
  take against record-backed slabs. On releases and files with no decodable
  element record — the 2023 Einhoven sample, every Tier-1 synthetic — the
  string scan is still the only room path and nothing changes.

- **`rooms_spaces` moves from `known_gap` to `known` on the full-project
  manifest (#90).** `tests/fixtures/project-counts/2024-core-interior-slim.json`
  now gates `diagnostics.exported.by_ifc_type.IFCSPACE` at **116, tolerance
  0** against the reference export's 116, and
  `2024-core-interior.json`'s row drops to `decoder_baseline` because its
  paired element fixture exports no spaces at all. Both manifests' committed
  `property_sets` decoder baseline moves 854 → 970.

### Removed

- `PartitionSchemaMvp::floors` and `partition_schema_mvp::is_strict_room_name`,
  which only served the plan-loop floors and name-only rooms described
  under Fixed. Floors are in `PartitionSchemaMvp::slabs`.

### Fixed

- **The viewer lays out correctly on HiDPI screens.** The 3D canvas's
  backing store is sized in device pixels, and nothing pinned the element
  to its container. On a 2x display it laid out twice as large, widened the
  grid, pushed the file-status panel off-screen and centred the drop zone in
  the overflow, bottom right. The canvas now fills the viewport exactly. The
  grid tracks cannot grow from their content, the canvas also follows
  viewport resizes that are not window resizes, and the empty-state overlay
  is opaque and scrolls on short screens. A Playwright test at device scale
  factor 2 holds it. The drop zone, workflow and support-profile copy now
  describe what Revit 2024 and 2025 projects export instead of calling
  typed extraction unsolved.

- **Placed instances with no 3D volume are left out, as Revit leaves them
  out.** 2D symbol families placed as equipment or fixtures (floor drains
  drawn in plan, wheelchair circles, clearance zones) have a flat record
  bounding box and no body. Revit's IFC export omits them, and rvt-rs
  exported them as bodiless entities.
  - On Snowdon Towers all 60 such instances were among the elements Revit's
    export lacks, and no element it holds on any measured file is flat.
    Snowdon's elements outside Revit's export fall from 115 to 55.
  - The sidecar reports them as `element_record_without_volume`. They do
    not count toward `unexported_element_records` (#309).

- **Boolean property values are valid STEP.** The writer emitted
  `IFCBOOLEAN(.T)` / `IFCBOOLEAN(.F)`, missing the closing dot of an ISO
  10303-21 enumeration. Every export that carried a boolean property was
  therefore invalid IFC: the element-record property sets (`ProfileResolved`,
  `LevelBindResolved`, `ThicknessResolved`) put 2,400 of them in the Core
  Interior export alone. IfcOpenShell's validator now reports no issue on the
  Einhoven and Core Interior exports. CI validates both in full, because the
  committed fixtures carry no boolean and never exercised the path.

- **The "incomplete model" count covers missing elements, not every
  unreadable frame.** Frames with no ElementId at `+0x00` keep the
  container and placement fields of the instance rule (RE-33), so
  `element_record_without_element_id` and
  `confidence.unexported_element_records` now count only placed instances.
  On the MIT RE1 models the count now equals the elements Revit exported
  and rvt-rs did not, exactly for pipes (60), pipe fittings (48), duct
  fittings (18) and duct and pipe segments (15). On Snowdon Towers it does
  the same for mullions (1,425), columns (118), railings (131), ceilings
  (68) and furniture (344), and falls from 6,019 frames to 4,886.
- `SpecialtyEquipment`, `FurnitureSystem` and `Mass` no longer map to a
  `USERDEFINED` PredefinedType with no `ObjectType`, which IFC4 forbids.

- **Export readiness no longer reads 100% on an incomplete model.** When
  the partition scan finds wall, door, window, column, floor or room records
  with no attributable ElementId (RE-30), the export skips them. The score
  still counted the export as complete, e.g. "geometry · 100%" on Snowdon
  Towers with 43 elements exported and 2,043 left out.
  - `confidence.unexported_element_records` in the diagnostics sidecar
    carries that count, and the score's element terms (elements, typed
    elements, geometry) now count only for the exported share. Metadata and
    units still count in full.
  - Measured: Snowdon Towers Architectural 100% -> 36%, Snowdon Structural
    -> 52%, RE1 Architecture (Revit 2025) -> 95% (its 5 doors and 1 curtain
    wall are left out). Core Interior and Einhoven, which leave nothing out,
    stay at 100%.
  - `rvt-inspect` reports a new `incomplete_model` failure mode, names the
    count in its readiness summary and next steps, and `rvt-ifc` prints a
    warning to stderr. The viewer's status panel shows "Incomplete model"
    and no longer marks decode confidence green while records are missing.
- **Rooms and floors come from element records only; the partition MVP no
  longer invents them.** Its name-only rooms (space-like display strings)
  and plan-loop floors (closed runs of f64 pairs that miss the ArcWall
  centrelines) matched nothing in Revit's own exports:
  - every family file, on every release, gained a room named "Office
    Equipment";
  - the Revit 2025 RE1 Electrical, Mechanical and Plumbing models
    (`Drshelden/IFC-ECS`, MIT) exported 38 slabs and 71 spaces, named after
    space-type strings such as "Banking Activity Area - Office", while
    Revit's exports of them hold no `IfcSlab` or `IfcSpace`;
  - on two Revit 2023 projects with paired Revit exports, the name-only
    path found none of the 9 real rooms of one and invented "Office
    Equipment" in both, and none of the 17 plan loops (mostly triangles)
    comes within 10% of the plan area of any exported slab;
  - Snowdon Towers gained 37 and 15 name-only rooms, including a view name
    ("Enlarged Residential Lobby Plan"), and RE-25 had already found no
    plan loop with the bounds of any Core Interior plate.

  Record-backed walls, doors, windows, columns, slabs and rooms on 2024 and
  2025 are unchanged (Core Interior and RE1 Architecture export exactly as
  before). A 2023 file now exports its ArcWalls without invented floors or
  rooms, and a family file exports no building elements.
  `tests/rooms_floors_from_records_only.rs` checks all 11 family releases.

- **Family files' `Global/ElemTable` is read correctly on every release.**
  It was parsed as 12-byte "implicit" records from `0x30`, which produced
  meaningless ids (0, 4128768, 196608, …) on all 11 family releases.
  Families actually use the project records: 28 bytes through 2023 and
  40 bytes from 2024, starting at `0x1E`. They have no `0xFF` marker and
  end in a trailer, so neither existing check found them. `detect_layout`
  now recognises such a table by its ids, which rise from 1 and equal
  their secondary copy.
  - `parse_records` returns exactly `record_count` records, with real
    ElementIds.
  - An owner field of `0`, which families use for "none", reads as unset.
  - The documented `header_flag = 0x0011` turned out to be record 0's
    owner (ElementId 17).
  - The header value named `element_count` is a per-release constant (1370,
    1411, 1451 and 1481 for 2023–2026), not a count of elements.

  The problem was found by cross-checking phi-ag/rvt's independent notes
  on the 2026 family table, and it is now pinned for all 11 releases by
  `tests/elem_table_corpus.rs`.
- **`source_coverage.decoded_element_fraction` divides by a real count.**
  Its denominator was that header constant, so on Snowdon Towers it
  reported 0.689 (972 / 1411). It now divides by the distinct ElementIds
  `Global/ElemTable` declares, and `rvt-elem-table` labels the header value
  as the constant it is.

- **`rvt-dump` writes exactly the bytes the parsers read.** It inflated
  every stream without stripping Revit's per-page checksum trailers, so a
  multi-page stream either failed outright — `Global/ElemTable` on
  Autodesk's Snowdon Towers sample was reported as "no gzip magic found" —
  or lost every gzip member that straddled a page boundary: that file's
  `Partitions/68` came out 10.5 MB short. Partitions were also joined with
  16-byte `0xFF` separators, so no offset in a dump matched the offsets the
  scanners and the `reports/` write-ups use. Dumps now go through the
  library's page-aware decoders, `Partitions_NN.decomp` is byte-identical to
  `RevitFile::inflated_partition` (with the member count printed), and
  empty, uncompressed and failed streams are reported as such.
- **`Global/ElemTable` no longer reads every ElementId as 0 when a record's
  sentinel field is set.** The layout detector took the record size from
  the spacing between the first two `0xFF` sentinel runs. On Autodesk's
  Snowdon Towers 2024 architectural sample, records 1 and 2 hold `0x10`
  there instead of `0xFF`×8, so the first two runs are 120 bytes apart, not
  40, and all 15,744 rows parsed at that stride read ElementId 0. Because
  element recovery keeps only records whose ElementId is declared in
  ElemTable, no partition element record could match, and the export fell
  back to 64 plan-loop floor guesses (the heuristic's cap) that had no
  ElementId and no geometry. When the measured spacing does not tile the
  stream flush against the declared record count, the detector now tries
  the known 40- and 28-byte strides that do. That file now declares all
  47,233 records (47,232 distinct ElementIds), and its IFC export carries
  5 walls and 1 floor with their Revit ElementIds and bodies in place of
  the guesses. `2024_Core_Interior.rvt`, `Revit_IFC5_Einhoven.rvt`, the
  Snowdon structural sample and every family file detect exactly as
  before. `reports/element-framing/RE-30-snowdon-generalisation.md` records
  what the same two files show beyond the table: most of their element
  records use a second prologue, whose ElementId RE-35 above reads from the
  enclosing partition record.
- **The browser viewer's Export IFC and Export plan SVG buttons work.**
  Both had failed on every click since they shipped (commit 7d54d3f, 2026-04-20)
  — confirmed on the deployed site before this fix — for two stacked
  reasons. The exports run wasm on the main thread, which never
  initialised its own instance (the worker initialises only its own), so
  every call died with `Cannot read properties of undefined (reading
  '__wbindgen_add_to_stack_pointer')`; `viewer/src/main.ts` now
  initialises one instance on first export and retries if that fails.
  Behind that, the STEP writer stamped its header with
  `SystemTime::now()`, which panics on `wasm32-unknown-unknown` ("time not
  implemented on this platform"); the wasm build now reads `Date.now()`.
  The panic hook, previously installed only by the byte-opening bindings,
  is installed by every binding, so a main-thread panic reports its
  message instead of a bare `unreachable`. The viewer suite now downloads
  both exports and checks the STEP and SVG content.
- **The CLIs behave the same way and say what went wrong.** `rvt-gltf` and
  `rvt-sheet` take the model as a positional argument and write
  `model.glb` / `model.svg` next to it unless `-o` says otherwise — the
  form the README already showed but the binaries rejected (they required
  `--src` / `--dst`, which still work). Both refuse to overwrite their
  input. `rvt-ifc`, `rvt-doc` and `rvt-elements` stop printing anyhow's
  `Error:` debug form; every CLI now reports `error: <message>` with the
  file named once and exits 1. `rvt-schema`, `rvt-history`, `rvt-corpus`
  (and `rvt-corpus doctor`), `rvt-elem-table` and `rvt-capabilities` accept
  `--json` as shorthand for `--format json`. Piping any CLI into `head`
  no longer ends in `failed printing to stdout: Broken pipe` and a panic
  backtrace: `rvt::cli::exit_quietly_on_broken_pipe` exits with status
  141, as a shell reports `SIGPIPE`, and leaves every other panic alone.
  Every `--help` ends with worked examples, each one run against the
  sample corpus before it was written down.
- **A storey-less element is no longer written into the first storey
  (#219).** The STEP writer clamped a missing storey index to
  `storey_index.unwrap_or(0)`, so on `2024_Core_Interior.rvt` 71 elements —
  some of them plates at 185 ft — were emitted as contained in
  `IfcBuildingStorey` `Basement 2` at −40 ft, indistinguishable from the 5
  that belong there. Elements with no recovered storey are now contained in
  the `IfcBuilding`, which states what rvt-rs actually knows. On that file
  `Basement 2` drops 76 → 6 contained elements and what stays unbound is
  visibly unplaced instead of silently placed. That set was 19 when this
  landed — 18 name-only `IFCSPACE` rows and one wall — and is **one** wall
  after #90 / RE-29 replaced the spaces with record-backed rooms.

- **Wall bodies are cut back by their joins, and the column profile comes
  from the family type (#215, RE-26).** With #232's double translation gone,
  the world-coordinate gap against Revit's own export was finally readable.
  Read as a world *bounding box* rather than a vertex-mean centroid — the
  centroid is not comparable when one side emits a box and the other a
  62-facet mesh — the column "1.58 ft in Z" of #236 is **0.0000 ft**: all 256
  columns already matched Revit's z extent at both ends. The wall gap was
  real, and it was two things.

  First, **an element can be framed by more than one partition record and the
  newest frame wins.** 171 of 360 walls (and 96 of 132 doors) carry two frames
  that disagree about the plan box, because Revit rewrites an edited element
  into a higher-numbered `Partitions/*` stream and leaves the earlier copy in
  place. The instance selector's tie-break moves from the first
  `(stream, offset)` to the last. On the newest frame the box's thin plan
  extent equals the nominal thickness of the `IfcWallType` Revit's export
  assigns on **360 of 360** walls (201 of 360 on the oldest), its type slot in
  the `+0x88` list equals that `IfcWallType.Tag` on 356 of 360 (183 of 360),
  and `Global/ElemTable`'s `u32` at `+0x1c` — a monotone version counter —
  ranks the same way across all 976 exported instances. No frame on the file
  disagrees about the box's `z`, so nothing can move between storeys.

  Second, **the record box is the untrimmed wall.** Every non-zero delta
  between it and Revit's own `Axis` polyline, across all 720 wall ends, is
  exactly 0.25, 0.3333 or 0.75 ft — half of the 6″, 8″ and 18″ wall
  thicknesses on the file. `element_record_wall_joins` cuts each end back by
  half the thickness of the perpendicular recovered wall whose centreline
  lands on it, spans this wall's centreline and overlaps it in elevation, and
  declines the whole element when candidates disagree. Every input is a
  recorded bounding box; nothing is fitted. Result, measured with IfcOpenShell
  0.8.5 in world coordinates at 1e-3 ft: **336 of 360 walls exact, up from
  27**, worst residual 0.75 → 0.3333 ft, mean 0.2701 → 0.0220 ft, 309
  improved and 16 regressed. The 31 wall ends it gets wrong are all
  over-trims, and they fall inside feature classes that also produce a real
  trim 125 and 22 times — Revit stores that decision per wall pair, and it is
  in neither wall's box nor in 4 KiB past its record. This models Revit's join
  cleanup rather than reading it, which the capability status says.

  Third, **#215 is closed.** The last slot before a column record's own
  ElementId in the `+0x88` list that is itself an `OST_Columns` type-symbol
  record is `5755` (`Column_Sqaure:24" x 24"`) on **256 of 256** — the
  `IfcColumnType.Tag` Revit's export writes for every one of them — and the
  section that symbol carries is now the emitted profile, behind a fail-closed
  check that it agrees with the instance envelope to 1e-6 ft. On this file it
  is the same 2 ft square the envelope already had (worst disagreement
  8.0e-15 ft), so **no vertex moves**; what changes is that `ProfileResolved`
  now means the type said so instead of a box happening to be square. The
  80 columns that still miss have a body Revit *cuts* inset from the full
  prism, which no section can produce.

  `tools/ci/ifc_schema_arity.py` gains wall `20800` as a pinned
  `ObjectPlacement × Position × profile-point` probe — it is cut at both ends,
  so re-emitting the untrimmed box moves the point by 0.1016 m, which the gate
  reports when run against the pre-change export. Entity counts, relations,
  storeys (15) and storey containment (801 of 872) are unchanged and the
  diagnostics sidecar is byte-identical; the two committed `rvt-rs`
  OctetProof observations were regenerated because the payload counts every
  emitted entity type and `IFCPROPERTYSINGLEVALUE` moved 5611 → 8435 with the
  new provenance properties. The four bridge-witness observations and both
  `verdict.json` files are byte-identical, and both verdicts stay `PASS`.
  Report: `reports/element-framing/RE-26-world-coordinate-residuals.md`.

- **Registry coverage declarations — `covers` per adopted reader
  (#229, OctetProof §9.4).** The registry recorded no coverage claim, so
  nothing outside the observations themselves said that rvt-rs, IfcOpenShell
  and IFClite now read `relations` and `storeys` as well as `entity_counts`.
  Each adopted reader in `research/witness-registry.json` now declares
  `covers`, drawn from the §9.4 controlled vocabulary: `entity_counts`,
  `relations`, `storeys` for the three gated readers; `entity_counts` alone
  for dwg-rs, which has produced no observation yet because the
  `rvt-to-dwg` edge is still `pending` — its declaration is the surface it
  would claim, not a reproduced agreement, and the notes say so. The
  authoring witness `autodesk-revit-exporter` declares nothing: coverage is
  a claim about *reading*, and an author emits the artifact rather than an
  observation. `docs/schemas/witness-registry.schema.json` carries the field
  with the vocabulary as an enum and two conditionals — an adopted reader
  must declare it, an author must not — and `tests/witness_registry.rs`
  enforces the same two rules plus the subset rule that makes the
  declaration load-bearing: every committed observation's
  `semantic_surface_covered` must be a subset of its witness's `covers`, so
  a witness cannot widen its own claim without a registry change. No status,
  lineage or `checked` value moved.
- **Slab plan profiles — 80 / 80 exact against Revit's own export
  (#31, RE-25).** RE-22 recovered every slab by ElementId and left the
  profile as the bounding rectangle. It is in the file, but not as a
  polyline: **each boundary line of a Revit sketch is its own partition
  element record**, `BuiltInCategory` `OST_SketchLines` (−2000045), with
  the same 88-byte prologue and the segment's own bounding box. A
  *second* counted reference list follows the `+0x88` list RE-23 read,
  framed identically, and its last slot names the sketched element: the
  3688 sketch-line records on `2024_Core_Interior.rvt` name 126 owners,
  among them all 80 exported `IFCSLAB` and all 20 exported
  `IFCSHADINGDEVICE` ids. The join is by ElementId alone — no geometry,
  no proximity, no containment. Segments are chained by **closure, not
  fitting**: a box degenerate on one axis contributes its endpoints, and
  every other box (a diagonal, or one of the 37 recorded looser than
  their line) is placed only when exactly one pair of still-open
  vertices fits it; an ambiguous segment, a vertex that does not finish
  at degree 2, an unused edge, a zero-extent box or a loop under three
  vertices rejects the whole element. Measured against the reference
  export's `IfcExtrudedAreaSolid` swept areas in the project plan frame
  at tolerance 1e-3 ft per vertex: **80 of 80 exact** across 122 loops
  (80 outer + 42 rectangular voids), worst vertex deviation
  **1.563e-12 ft** — the same doubles, differing only in Revit's own
  floating dust — and 2.842e-14 ft re-measured on the emitted IFC. 42
  perimeter plates emit `IFCARBITRARYPROFILEDEFWITHVOIDS` (a 26-vertex
  ring around one rectangular courtyard void) and 38 emit
  `IFCARBITRARYCLOSEDPROFILEDEF`; before this, all 80 were rectangles,
  accidentally exact on 38 and filling a courtyard on 42. The 20
  exported `IFCSHADINGDEVICE` are rotated plates whose 57 sketch-line
  boxes include one of zero extent and 21 diagonals with no
  axis-parallel anchor: the closure declines all 20 and they keep the
  record box with `ProfileResolved: false`. The pre-RE-25 plan-loop scan
  is a measured dead end — 2317 closed plan-polyline candidates across
  all eight inflated partitions, none with the plan bounds of any
  recovered plate, and no ordered vertex run of the export's polygon
  anywhere in the file at any stride. Building-element counts, the
  diagnostics sidecar and both OctetProof verdicts are unchanged.
- **Revit `Level` records — 15 / 15 exact `(name, elevation)` storeys
  against Revit's own export (#218, RE-24).** #213 derived storey
  elevations from the bounding-box distribution of the recovered column
  records: 11 of the export's 15, with no names, because a rank join of
  12 name candidates onto 11 elevations mislabels. The Levels are in the
  file as elements. A `Level` record carries the same 88-byte element
  prologue with `OST_Levels` (−2000240) at `+0x12`, but no bounding box
  — a datum plane is not a solid — so it ends at `+0x56` where a column
  record's bbox marker would start, which is why the #211 reader rejected
  it. Of the 75 `OST_Levels` records on `2024_Core_Interior.rvt` the #211
  instance test (no container at `+0x32`, placement kind `0xffffef7f` at
  `+0x42`) selects exactly **15**. Each owns exactly one name/elevation
  parameter block, framed the way RE-22 framed the `IFC Export As`
  overrides: the owning ElementId as a `u64` at `value-0x47`, a 56-byte
  `0xff` sentinel run, three zero bytes, the `u32` UTF-16 length, then
  the name; the elevation is an `f64` in feet 55 bytes past an 8-byte
  marker (`05 00 00 00 48 02 00 00`) searched forward from the end of the
  name, repeated 153 bytes later, both copies required to agree. All
  **15 of 15** pairs equal an `IfcBuildingStorey` `Name` / `Elevation` in
  Revit's export exactly — `Basement 2` −40, `Basement 1` −20, `Level 1`
  0, `Mez 1-2` 15, `Level 3 / 4 / 4 - Wall Layouts 1 / 2 / 3` at
  31 / 46 / 61, `Level 6`…`Level 13` at 76 / 91 / 106 / 121 / 136 / 151 /
  166 / 185.5 ft — including the four elevations no column stands on. The
  names are asserted rather than joined: the block is keyed by the
  Level's own ElementId, so the file states the pairing. Recovery is
  all-or-nothing per file — a Level with no accepted block emits nothing,
  and the whole set is discarded unless every Level record owns exactly
  one block and no two levels share an elevation. `levels` moves from
  `decoder_baseline` to `known` at tolerance 0 on **both** manifests.
- **Wider storey set, more bound elements.** Containment is unchanged —
  still an exact elevation match, plates by their record top face — but
  with 15 storeys instead of 11, **801 of 872** building elements land in
  a specific storey, up from 794: all 256 columns, all 132 doors, 359 of
  360 walls (was 355) and 54 of 100 record-backed plates (was 51 —
  `IFCSLAB` 44 of 80, `IFCSHADINGDEVICE` 10 of 20). The 6 windows still
  bind to nothing, because a window record's base is its sill height and
  never a storey elevation; the 46 plates that remain unbound sit
  0.1667 ft below their level at the structural-slab /
  architectural-topping interface (#219) — a thickness question, not an
  elevation-set one. `element_record_storeys` survives as the fallback
  for files with no recoverable Level records.
- **OctetProof `storeys`: a third cross-witness field class (spec 1.1.0,
  §7.2 / §9.4 / §20.2).** A storey set is the sorted set of
  `[name, elevation]` pairs, compared exactly with no tolerance concept.
  It is the first field class in the protocol where the two sides of an
  edge express the same physical quantity in *different declared units*:
  Revit's export of this imperial project declares `FOOT`, rvt-rs writes
  `METRE`, and the raw numbers differ by 3.28. Each witness therefore
  resolves its own file's `LENGTHUNIT` and renders the elevation in feet
  at 1e-6 as a fixed six-decimal string, which keeps the canonical form
  integer/string only (§7.3) — `rvt-ifc --observation` walks
  `IfcProject` → `IfcUnitAssignment` in its own emitted STEP,
  `tools/ci/witness-ifcopenshell.py` uses
  `ifcopenshell.util.unit.calculate_unit_scale`, and
  `tools/ci/witness-ifc-lite` uses
  `ifc_lite_core::extract_length_unit_scale`. The three payloads are
  byte-identical on both artifacts. Manifests gain a `storeys` block
  parallel to `counts` and `relations`; `tools/ci/witness-verdict.py`
  compares it as a separate field class; the observation and verdict
  schemas are updated. Claimed surfaces:
  `magnetar-2024-core-interior-slim` **11 → 13 fields**,
  `magnetar-2024-core-interior` **4 → 6**. Both verdicts stay `PASS`.

- **Door and window host-wall binding — 138 / 138 exact
  `IfcRelFillsElement` pairs against Revit's own export (#222, RE-23).**
  #211 recovered the exact door and window instance sets but left them
  unattached. The host is in the record: past the bounding box, at
  `+0x88`, every partition element record carries a counted reference
  list — a `u32` length then that many `u64` slots — and the slot
  immediately before the record's own ElementId is the host wall. The
  value is accepted only when it is one of the 360 ElementIds the RE-21
  instance rule selects as exported walls, so a wrong read fails closed
  instead of inventing a host. Measured on `2024_Core_Interior.rvt`:
  273 of 273 door and window records give the correct host, and the
  emitted `(host wall, filling element)` pair set equals the reference
  export's `IfcRelVoidsElement` ∘ `IfcRelFillsElement` chain exactly —
  138 pairs, no wrong host, no missing pair, no extra pair. Three
  fixed-shape readings were measured and rejected: `u64 @ +0xac`
  (186 / 273), `u64 @ +0xa4` (81 / 273) and "the last two slots are
  (host, self)" (192 / 273). The list length is not fixed (6 on every
  door record, 18 on every window record, up to 101 elsewhere) and 81
  door records continue past their own id with a trailing type-symbol
  slot.
- **The IFC4 opening chain Revit itself emits.** Each bound door or
  window now ships an `IfcOpeningElement` — bodied with the element's
  own record bounding box, `Tag` repeating the filling element's, as
  the reference export does on all 138 — plus `IfcRelVoidsElement`
  (wall → opening) and `IfcRelFillsElement` (opening → door/window).
  `tools/ci/ifc_schema_arity.py` passes on the fresh export: 18 192
  instances across 40 entity types, `IfcOpeningElement` at its declared
  arity of 9, both relationships at 6. The export's other 63
  `IfcOpeningElement` (42 slab penetrations, 20 shading-device
  penetrations, 1 unfilled wall opening) are not recovered — rvt-rs
  emits no opening that is not filled by a recovered element.
- **OctetProof `relations`: a second cross-witness field class
  (spec 1.1.0, §7.2 / §9.4 / §20).** Entity counts are a weak
  agreement — two witnesses can report the same 138 fill relationships
  while disagreeing about every wall those openings belong to. A
  relation is the sorted multiset of `[host Tag, filling Tag]` pairs,
  compared as an exact set with no tolerance concept; a diff names the
  pairs each side holds alone. All three witnesses emit it, each with
  its own parser — `rvt-ifc --observation` splits its own emitted STEP,
  `tools/ci/witness-ifcopenshell.py` uses `by_type` plus attribute
  access, `tools/ci/witness-ifc-lite` uses `EntityScanner` plus
  `parse_entity` — and the three payloads are byte-identical on the
  full-project artifact. Manifests gain a `relations` block parallel to
  `counts`, `tools/ci/witness-verdict.py` compares it, and
  `docs/schemas/witness-observation.schema.json` /
  `witness-verdict.schema.json` are updated. The
  `magnetar-2024-core-interior-slim` claimed surface goes **10 → 11
  fields**; on the 20 KB element-fixture artifact the same relation is
  `decoder_baseline` and excluded first-class, because that export
  carries no `IfcRelFillsElement` at all. Both verdicts stay `PASS`.

- **Slab instance recovery — 80 / 80 `IFCSLAB` and 20 / 20
  `IFCSHADINGDEVICE` exact ElementId sets on the Core Interior
  full-project export (#212, RE-22).** RE-21 recorded `OST_Floors` as the
  one category where the instance rule fails (99 selected, `TP 79 / FP 20 /
  FN 1`). Both numbers reproduce and neither is a decoder error. The 20
  "false positives" are the export's 20 `IFCSHADINGDEVICE` `Tag` values —
  exported elements under a different entity type, so precision was already
  100 %. The "false negative" is `Pad:Site Pad` ElementId 21975, whose record
  carries `BuiltInCategory` `OST_BuildingPad` (`-2001263`), not `OST_Floors`;
  Revit's own exporter maps a building pad to `IfcSlab` `.FLOOR.`. Adding
  that category recovers the 80th slab.
- **Per-element IFC export-type overrides (`partition_ifc_export_overrides`).**
  The `IfcSlab` / `IfcShadingDevice` split is decided per instance, not per
  type — the reference export carries `IFCSLABTYPE` *and*
  `IFCSHADINGDEVICETYPE` rows with the same `Tag` (`4166`, `71848`). The
  deciding value is in the file: a UTF-16LE string in the element's parameter
  block, framed as a `u32` code-unit count then the value, with the owning
  ElementId as a `u64` at `-0x0dc` and again at `-0x11e`. Requiring both slots
  to agree and the id to be declared in `Global/ElemTable` accepts 31 entries
  on this file naming 21 ids; the 20 that are also standalone placed instances
  are exactly the export's `IFCSHADINGDEVICE` set (the 21st is a container
  member the RE-21 rule already excludes). The scan is general; only
  `IfcShadingDevice` is honoured as a target
  (`ifc::category_map::EXPORT_OVERRIDE_TARGETS`), because it is the only value
  a reference export has demonstrated — an unrecognised value leaves the
  element on its class mapping rather than inventing a type.
- **Slab extrusion thickness (#31, thickness half).** The record's
  bounding-box `z` extent *is* the plate's thickness: it equals the export's
  `IfcExtrudedAreaSolid.Depth` on 79 of 80 slabs at 1e-3 ft and sums to it on
  the 80th (`Floor:Basement Slab` 22756, exported as stacked 0.3333 ft +
  1.1667 ft solids). Record-backed slabs now carry `ThicknessResolved`,
  `ThicknessFeet` and `ThicknessSource`, and
  `floor_slab_extrusion_thickness` is raised per slab so the plan-loop path
  still declares it. The slab *profile* remains the bounding-box rectangle —
  `ProfileResolved` stays `false` and #31's polygon-profile half is open.
  Evidence, rejected candidates and reproduction commands:
  [`reports/element-framing/RE-22-slab-instance-rule.md`](reports/element-framing/RE-22-slab-instance-rule.md).
- **Wall / Door / Window instance recovery — 360 / 132 / 6 exact ElementId
  sets on the Core Interior full-project export (#211, RE-21).** #210 showed
  the Revit 2024 partition element-record header reaches 100 % of the exported
  ElementIds in every architectural category; what was missing was precision.
  Two fields inside the 54 bytes that #210 recorded as sentinel padding supply
  it: a **container reference** at `+0x32` (`u64`, `0xffff_ffff_ffff_ffff` =
  none) and a **placement-kind** word at `+0x42` (`u32`, `0xffffef7f` placed
  instance / `0xffff8000` family-type symbol envelope). A record that carries
  no container reference **and** is marked a placed instance is exactly what
  Revit's exporter emits: 360/360 `IFCWALL`, 132/132 `IFCDOOR`, 6/6
  `IFCWINDOW` and 256/256 `IFCCOLUMN`, matching **ElementId sets** and not
  merely counts, with no false positives and no misses at tolerance 0.
  Each is emitted with a project placement and a bounding-box
  `IfcExtrudedAreaSolid` (an envelope, not a recovered family profile or wall
  location curve) plus an `RvtElementRecordGeometry` property set that says
  so. `building_elements_with_geometry` on that file goes 256 → 754.
  Evidence, histograms and reproduction commands:
  [`reports/element-framing/RE-21-partition-element-record-instance-rule.md`](reports/element-framing/RE-21-partition-element-record-instance-rule.md).
- **Every non-sentinel `+0x32` value is a container ElementId (#216
  explained).** All nine observed on `2024_Core_Interior.rvt` are declared in
  `Global/ElemTable`, are lower than every member id that names them, own a
  contiguous ElementId block spanning several categories at once, and have no
  element record of their own. The 136 `OST_Columns` ids Revit omits split
  with no remainder into 17 type symbols and 119 members of five such
  containers — which is precisely the split #216 described as "family-local"
  plus "exact co-locations".
- **Storey elevations recovered from element-record bounding boxes, and
  elevation-keyed spatial containment (#213).** `src/element_record_storeys.rs`
  turns the distinct base `z` of the partition element records into building
  storeys and binds each recorded element to the storey whose elevation matches
  its base. On `2024_Core_Interior.rvt` the 256 recovered `OST_Columns` records
  stand on exactly **11 distinct base elevations** — 0, 31, 46, 61, 76, 91,
  106, 121, 136, 151, 166 ft — and **all 11 equal an `IfcBuildingStorey.Elevation`
  in Revit's own export of the same file**, with no false positives; the
  export's other four storeys (−40, −20, 15, 185.5 ft) carry no column record
  and are not claimed. `IfcRelContainedInSpatialStructure` goes 1 → 11 and 256
  of 338 building elements land in a specific storey instead of all 338
  defaulting to the first one. Both halves fail closed: the storey set is
  replaced only when the recovered `Level` rows carry no elevation of their own
  (all at one value), and an element whose base matches no storey — or more
  than one — stays unbound. The 2023 ArcWall path, which already recovers real
  elevations, is untouched (verified on the Einhoven sample: 4 storeys,
  4 containment relations, unchanged). Level *names* are deliberately not
  paired with the measured elevations: on this file there are 12 name
  candidates against 11 elevations, and a rank join puts `Level 6` at 91 ft
  where the export puts `Level 7`, so every storey keeps an
  `Elevation N ft` label and the 12 recovered names stay visible in
  `diagnostics.decoded.production_class_counts.Level`. Level ElementId binding
  remains unsolved (#86 / RE-20) and is not what this does.
- **`diagnostics.exported.storey_elevations_feet` and
  `diagnostics.exported.storey_bound_elements`** — the recovered elevations
  alongside the existing `storey_names`, and how many building elements reached
  a specific storey. An all-zero elevation list is the honest signal that only
  Level name strings were recovered.
- **`tools/ci/ifc_schema_arity.py`** — a fail-closed schema-conformance gate
  that re-derives every entity's declared attribute count from the IFC4 EXPRESS
  schema and compares it with what the STEP text actually wrote, for every
  instance in a file, plus a null check on `PredefinedType` for the entity
  types whose every `category_map` row supplies one. Wired into the
  `ifcopenshell-validate` CI job over both committed fixtures and the
  freshly emitted real-project IFC, into `tools/check-local.sh --ifcopenshell`,
  and into `tools/ci/validate-real-ifc.py`. IfcOpenShell's `open()` accepts a
  short record, so a count-only check never saw this class of defect.
- **Storey elevations stay column-derived, and the restriction is now
  measured (#211 + #213).** Wall, door and window records carry base
  elevations too, so the #213 derivation would have widened its evidence base
  silently. Measured against the fifteen `IfcBuildingStorey.Elevation` values
  in Revit's export: `OST_Columns` 11 distinct bases, 11 of them storeys, 0
  not; `OST_Doors` 11 / 11 / 0; `OST_Walls` 13 / 12 (it adds −40 ft) / 1
  (56.4167 ft); `OST_Windows` 6 / **0** / 6 — a window sits at its sill height
  above the level that hosts it, so its record base is never a storey
  elevation. Only `IFCCOLUMN` therefore supplies elevations
  (`STOREY_ELEVATION_SOURCE_TYPES`), which keeps #213's "no false positives"
  claim exactly as recorded, while every record-bodied element still *binds*
  to that set by exact match: `IfcRelContainedInSpatialStructure` stays 11 and
  storey-bound elements go 256 → **743 of 836** (all 256 columns, all 132
  doors, 355 of 360 walls). The 5 walls at −40 / 56.4167 ft and all 6 windows
  stay unbound rather than being placed by proximity. Recovering the export's
  −40 ft storey from wall records costs one false elevation, so it is left
  open rather than taken.
- **Column instance recovery — 256 of 256 `IFCCOLUMN` on the Core Interior
  full-project export (#204).** `src/partition_element_records.rs` decodes the
  Revit 2024 partition *element-record header*, a fixed 88-byte prologue whose
  first field is the record's own `ElementId` (`u64`), whose sixth is the
  element's Revit `BuiltInCategory` (`i64`, negative, at `+0x12`), and which is
  followed at `+0x50` by a fixed marker and at `+0x58` by the element's model
  bounding box as six `f64` feet. 24880 records of this shape exist on
  `2024_Core_Interior.rvt`; every `ElementId` they carry is declared in
  `Global/ElemTable`, and the decoder requires that join, so a stray byte match
  cannot become an element. `partition_schema_mvp::columns_from_partition_category_records`
  keeps the `OST_Columns` (`-2000100`) records, drops those whose bounding box
  is centred on the plan origin (family-local type envelopes, not placed
  instances), and collapses co-located footprint groups to their highest
  `ElementId` — Revit allocates ids monotonically, so the newest of a
  superseded pair is the live element. The result is exactly the 256 columns
  Revit's own exporter emits: no false positives, no misses, gated at
  tolerance 0. Emitted as `IfcColumn` with a project placement and a
  bounding-box `IfcExtrudedAreaSolid` (an envelope, not a recovered family
  profile) plus an `RvtColumnGeometry` property set that says so.
  `building_elements_with_geometry` on that file goes 0 → 256.
- **`columns` is now a scored, cross-witness-agreed category.**
  `tests/fixtures/project-counts/2024-core-interior-slim.json` moves `columns`
  from `known_gap` (decoder 0) to `known` at 256/256, tolerance 0, so the
  OctetProof verdict for `magnetar-2024-core-interior-slim` compares
  `entity_counts.IFCCOLUMN` across all three lineages instead of excluding it —
  the claimed surface widens from four fields to five, and `IFCCOLUMN` is the
  first field in it with a non-zero expectation (the other four are agreements
  about absence). Both committed verdicts still `PASS`; both `rvt-rs.json`
  observations were regenerated, so their `observation_hash_sha256` changed. A
  new `column-instance-recovery` row in `docs/support-matrix.json` records the
  capability as `verified` with its scope stated: one `BuiltInCategory`, one
  release band, one recorded edge, envelope geometry, Level binding still open.
- **Third independent IFC witness — IFClite** (`tools/ci/witness-ifc-lite`):
  a small Rust binary over the crates.io crate `ifc-lite-core`, pinned at
  `=7.1.1` (`LTplus-AG/ifc-lite`, MPL-2.0 — verified against crates.io and the
  GitHub API on 2026-08-30; the registry previously recorded `MIT` and a bare
  repo slug, both corrected). It counts every manifest `source_ifc_type` by
  exact STEP keyword with its own scanner — no IfcOpenShell code — and emits
  an OctetProof §6.2 observation in the same shape as the Python bridge
  witness. Its canonical payload hashes byte-identically to IfcOpenShell's on
  the element fixture. The crate is its own workspace root and is run as a
  separate process, so MPL code is never linked into the Apache-2.0 tree;
  `tests/witness_registry.rs` now fails if the Cargo pin, the binary's
  `WITNESS_VERSION`, and the registry entry drift apart (spec §9.6). The
  Core Interior verdict now lists three lineages —
  `["ifc-lite", "ifcopenshell", "rvt-rs"]` — one source reader and two
  unrelated bridge readers.
- **Second recorded edge: the full-project export** —
  `tests/fixtures/project-counts/2024-core-interior-slim.json` registers
  `IFC Exports/2024_Core_Interior_slim.ifc` (bfdf36ff…, 1665968 bytes, IFC4,
  Autodesk Revit 24.0.20.20 via ODA SDAI 23.12, 19879 entities), which had
  been an artifact with no manifest. Counts measured with IfcOpenShell 0.8.5
  and independently reproduced by IFClite 7.1.1 and the STEP-constructor
  count: 360 `IFCWALL`, 132 `IFCDOOR`, 256 `IFCCOLUMN`, 116 `IFCSPACE`, 80
  `IFCSLAB`, 15 `IFCBUILDINGSTOREY`, 10 `IFCMATERIAL`, 6 `IFCWINDOW`, 1
  `IFCUNITASSIGNMENT`, 0 `IFCROOF` / `IFCBEAM` / `IFCFLOWTERMINAL` /
  `IFCPROPERTYSET` — all three readers agree exactly. rvt-rs recovers 0 / 0 /
  0 / 18 / 64 / 12, so nine categories are recorded as `known_gap` or
  `decoder_baseline` against #30, #31, #32, #33, #34, #35 and the new #204
  (columns, which had no tracking issue), leaving a four-field claimed
  surface. Observations and a `PASS` verdict are committed under
  `research/witness/magnetar-2024-core-interior-slim/` and gated in the
  `ifcopenshell-validate` job; the registry edge moves from
  `recorded-ungated` to `recorded`. No capability status changed.
- **OctetProof 1.0.0 specification** — `docs/octetproof-spec.md` (CC-BY-4.0,
  2026-08-30), the citable Layer-1 artifact of the cross-witness verification
  protocol. Supersedes the received draft, which stays verbatim at
  `docs/octetproof-spec-draft.md`; §19 lists every correction (jDwgParser is
  GPL-3.0, ACadSharp's coverage is undeclared, LGPL is weak copyleft, ifc-lite's
  license needed verification — resolved in note 4 to MPL-2.0 once the exact
  upstream was pinned, the ACIS reader landed in ACadSharp v3.6.51). §6's
  examples are now the real committed observation/verdict files, §7.2 states the
  geometry tolerance as relative 1e-6 with a manifest-stated absolute floor,
  §10.5 adds `REJECTED_INPUT` and `REPLAY_DRIFT`, and §5.3.1 maps the spec's
  registry vocabulary onto `research/witness-registry.json`, marking
  `ci_eligible`, coverage declarations, determinism attestation, and exact
  version pinning as umbrella scope.
- **Observation and verdict JSON Schemas** —
  `docs/schemas/witness-observation.schema.json` and
  `docs/schemas/witness-verdict.schema.json` (JSON Schema 2020-12) make the
  OctetProof §6.2 / §6.3 shapes machine-checkable; the committed files under
  `research/witness/magnetar-2024-core-interior/` validate against them.
- OctetProof witness mode and verdict gate (docs/octetproof-spec.md,
  docs/verification-protocol.md): `rvt-ifc --observation PATH --artifact-id ID`
  emits a canonical, hashed observation of what the decoder wrote (STEP
  constructor histogram, input SHA-256); `tools/ci/witness-ifcopenshell.py
  --observation` does the same for IfcOpenShell reading the Revit-authored
  reference IFC; `tools/ci/witness-verdict.py` compares them fail-closed
  (`PASS` / `DISAGREE` / `INSUFFICIENT_WITNESSES` /
  `INSUFFICIENT_INDEPENDENT_WITNESSES` / `REJECTED_INPUT` / `MANIFEST_ERROR` /
  `REPLAY_DRIFT`), enforcing the §9.3 independence set from
  `research/witness-registry.json` (`lineage`). First committed artifact:
  `research/witness/magnetar-2024-core-interior/` (two observations + a
  `PASS` verdict on eight categories, four excluded as tracked decoder
  gaps); replayed byte-for-byte in the `ifcopenshell-validate` CI job and
  pinned by `tests/witness_verdict.rs`. `sha2` moves from dev- to regular
  dependency for the input hash.
- **Cooperative cancellation + progress** — `rvt::control::{CancelToken,
  WalkerControl, Stage, ProgressEvent}` and additive `*_with_control` entry
  points beside the existing `*_with_limits` ones
  (`walker::scan_candidates_with_control`, `walker::iter_elements_with_control`,
  `partition_scanner::scan_partitions_with_control`); new `Error::Cancelled`
  variant. `rvt-elements --progress` prints each stage to stderr. Output is
  unchanged when no control is attached.
- **Property-based test suite** — `tests/proptest_parsers.rs` (proptest)
  asserts never-panic / in-bounds behaviour for the public byte parsers and
  JSON round-trips for the ES / fixture research record types on stable Rust.
- **Explicit corpus-path strictness** — when `RVT_PROJECT_CORPUS_DIR` is set,
  a missing directory or zero matching tier-two manifests now fails
  `project_count_fixtures` loudly instead of silently dropping coverage.
- **Revit-hosted oracle runner (untested skeleton)** —
  `tools/oracle/runner/pyrevit/` builds the ES-remap-00 seed and runs
  N1–N4 / R1–R2 / C1–C2 / C3a–C4a, writing `es-observation` records. Written
  without Revit; no ES on-disk layout is asserted.
- **Contributor onboarding** — clone-to-first-PR quickstart in
  `CONTRIBUTING.md`, README "For contributors", `good first issue` label,
  opt-in `.githooks/pre-commit` (`cargo fmt --check`), and
  `tests/binary_inventory.rs` pinning the shipped-binary count.
- **Experimental relation domains + capability doctor** — `relations`
  registry (ES / BIM / ElemTable isolation), Tarjan SCC + condensation +
  quarantine stubs with unit tests; `capability::CapabilityManifest`
  honest snapshot (ArcWall 2023 verified, compound/`es.elementid_remap`
  unsupported); `rvt-capabilities` CLI; evidence ledger JSON round-trip
  helpers; `docs/schemas/capability-manifest.schema.json`. Architecture
  only — not wired to production IFC/topology claims.
- **TransmissionData opportunistic extracts** — UTF-16LE probe now harvests
  UUID / path-like / XML node-name triage tokens when present; empty extract
  list explicitly means unknown, not “no links”. Still no linked-model
  resolution or schema rewrite.
- **Compound `0x0821` framing harness** — `compound_framing` marker
  tokenizer + stamp classification + adversarial f64 collision seed;
  docs under `reports/element-framing/RE-compound-0821-harness.md`. Does
  **not** decode compound openings.
- **IFC `Pset_` validation stub** — `ifc::pset_validate` allow-list /
  reserved-unknown classifier + `docs/ifc/pset-mapping-examples.yaml`
  (ES omitted by default).
- **TransmissionData UTF-16 detect stub** — `transmission_data::TransmissionDataProbe`
  classifies empty / UTF-16LE / opaque without inventing a field layout;
  `RevitFile::transmission_data_probe` exposes it. Linked-model resolution
  still unsupported. `rvt-history` clap/docs honesty: DocumentHistory is a
  UTF-16LE `"Revit "` scan, not a full history object model.
- **Preview PNG IEND trim** — `RevitFile::preview_png` truncates at the
  `IEND` chunk CRC when present (drops trailing OLE junk);
  `preview_png_untrimmed` keeps the forensic full tail.
- **Phase 1 research contracts (H-ES5 prep)** — `DocumentIdentity` /
  `ScopedElementRef` / `SourceSpan`, named `EvidenceTier` + evidence /
  edge ledgers, `EsReferenceOccurrence` + fixture transition types,
  `docs/research/unified-research-report.md`, `research/es-remap/`
  scaffold, and `es-observation` / `es-capability` JSON schemas.
  **Does not** claim ES ElementId remapping works; Phase 2 fixture
  generation remains blocked on a Revit-hosted API oracle.
- **Finding 1 / checksum-page framing (#151, Discussion #112)** —
  gated strip of trailing page checksums before inflate on
  `Partitions/*` and `Global/*` streams; Formats/Latest stays ungated
  by default (#162). Wave 1 stream-evidence harness + Wave 2 narrowed
  paged decompress, writer audit, and evidence matrix under
  `docs/recon/` and `docs/re/`.
- **A10 source_coverage fractions** — export diagnostics measure
  `exported_element_fraction` / `geometry_element_fraction` (and
  `decoded_element_fraction` when ElemTable header `element_count` is
  a trusted denominator). Fail closed with `status: unset` / nulls when
  unknown — never invents percentages.
- **Decode confidence + provenance (M3-07 / #150)** — every
  `DecodedElement` carries confidence/provenance; CLI, Python, and
  viewer expose it; default IFC emission hides low-confidence rows
  (threshold documented with the export modes).
- **Production typed / partition MVP path** — `walker::iter_elements`
  prefers fail-closed typed MVP decoders on `Global/Latest`, merges
  version-gated 2023 ArcWall partition recovers, and merges partition
  MVP recovers for Level / Material / Room / Floor plan-loops plus
  2024 ArcWallRectOpening index rows (ElemTable-confirmed related ids;
  never invents typed Door/Window). IFC maps recovered Levels →
  storeys, Floors → boundary-annotated slabs, Rooms → spaces, Material
  display names → `IfcMaterial`.
- **Viewer confidence UI** — File Status shows export readiness /
  confidence, recovered storey names and material samples, honest
  Parameters row (empty until AProperty host joins); scene tree groups
  under `IFCBUILDINGSTOREY` when elevation evidence allows.
- **Corpus + intake** — redistributable project corpus lanes, community
  corpus open→schema→scaffold validation executed
  (`docs/corpus-hunt-2026-04-21.md`: 222/223), corpus intake checklist
  (`docs/corpus-intake.md`), and CI corpus matrix trimming.
- **Inspect / compare tooling** — `rvt-inspect` user-facing status;
  `rvt-ifc-compare` for export QA (M5-05); IFC export `--mode` gates
  (scaffold / typed / geometry / strict) with diagnostics sidecar.
- **Supporting research docs** — RE-19 (no Door/Window discriminator /
  no schema-field Wall on magnetar corpora) and RE-20 (no recoverable
  Level ElementId map; Floors/Rooms stay Unassigned by evidence).
- **Earlier post-0.1.2 foundations still in tree** — ElemTable layout
  detection + `rvt-elem-table` CLI; walker public APIs; scalar-base
  Container decode (synthetic-verified); sector-preserving CFB
  identity roundtrip; always-on stream patch corpus; Python decoded
  API / `rvt-elements`; generic partition scanner.

### Changed

- **`PredefinedType` now follows the authoring witness (#220).** #217 made
  every entity carry its full IFC4 attribute list, which exposed four
  `category_map` rows disagreeing with what Revit's own exporter writes on the
  same project. The rule adopted is the issue's: where a recorded edge shows
  the witness's choice, take it — agreement with the export is what the
  OctetProof verdicts score, and a value the witness demonstrably wrote is not
  an invention. `IfcWall` emits `.NOTDEFINED.` instead of `.STANDARD.` (Revit
  writes it on all 360 walls, including the ones whose body is the plain
  `SweptSolid` that `.STANDARD.` describes); `IfcSpace` emits `.SPACE.`
  instead of `.INTERNAL.` (116 of 116); and `IfcDoor` / `IfcWindow` emit
  `.DOOR.` / `.WINDOW.` where they previously emitted `$` (132 of 132, 6 of
  6). Per-type agreement against the export is now exact for every type rvt-rs
  writes a value for. `Area`, `Space` and `GenericModel` keep `$`: no edge
  records what the witness does with them. No new observation surface class
  was added — the values are attribute-level and the observation payload
  counts entity type names, so neither observation hash moved. Instead
  `tools/ci/ifc_schema_arity.py` gained a `--witness-agreement` mode, run on
  the recorded artifact in CI, that asserts every written value against the
  witness's, and `IfcDoor` / `IfcWindow` joined the set of types whose
  `PredefinedType` may not be null.
- **OctetProof spec 1.1.0 → 1.1.1 — worked examples regenerated, coverage
  row implemented (#224, #229).** Patch-level and non-semantic per §16.1: no
  schema, diff-function, canonicalizer or status-vocabulary change. §6.2 and
  §6.3 still quoted the two-witness, eight-field `entity_counts`-only
  snapshot taken for 1.0.0; they are regenerated from the committed
  `research/witness/magnetar-2024-core-interior-slim/` files at `5da2570`
  (2026-08-30) — three witnesses across three implementation lineages, two
  `schema_version: "1.1.0"` observations carrying `relations` and `storeys`,
  and the complete thirteen-field `PASS` verdict with its independence
  block. Trimming is confined to four arrays and each trim states its full
  length; the verdict is byte-equal to the committed file. §5.3.1's
  `coverage` row moves from "not implemented / umbrella scope" to
  implemented-in-registry, §9.4 gains the reader-only and subset
  clarifications, and §16.3 moves the coverage declaration into the provides
  table, leaving `ci_eligible` as the only registry field still umbrella
  scope. `docs/verification-protocol.md` records the same.
- **Plan-loop floor annotations stand down where element records decode
  (#212, #219).** The 64 `IFCSLAB` boundary annotations the plan-loop scan
  emitted carried no ElementId, no bounding box and therefore no storey.
  Emitting them beside the record-backed slabs would double-count the same
  plates, so `recover_partition_schema_mvp` clears `floors` whenever `slabs`
  is non-empty — exactly one `IFCSLAB` per exported id. The plan-loop path
  remains the only floor path on 2023 files and anywhere no element record
  decodes.
- **Plates bind to a storey by their record top face (#212, #219).** Revit
  hangs a floor below the level that hosts it, so a slab's recorded base is
  never a storey elevation while its top often is: measured over the 100
  record-backed plates on Core Interior, 0 of 40 distinct base elevations is
  a storey elevation and 51 plate tops are — every one of the 51 the storey
  Revit's own export assigns, none wrong. Everything else keeps the #213
  base-face join. `storey_bound_elements` goes 743 → 794 and the
  `StoreyBindSource` property distinguishes `record_base_elevation` from
  `record_top_elevation`.
- **Slab record elevations are *not* admitted as a storey-elevation source
  (#218).** Measured rather than assumed: slab tops would add −40 ft and
  185.5 ft, two of the four storeys #213 misses, at the cost of 13 values
  that are not storey elevations (each 0.1667 ft below a real one, the
  structural-slab / topping interface). `STOREY_ELEVATION_SOURCE_TYPES` stays
  `["IFCCOLUMN"]` and the recovered storey count stays 11.
- **One record per ElementId now keeps the greatest bounding-box `z` extent
  (#212).** 22 of the 100 slab ids are framed more than once with disagreeing
  vertical extents; RE-21's first-by-`(stream, offset)` picked the 2 in
  topping for the 12 `Floor:Floor 1` plates. Against the export's extrusion
  depths, first-by-offset agrees on 67 of 80 and greatest-extent on 79 of 80.
  The change is applied uniformly and perturbs nothing else: 268 walls, 132
  doors and 88 columns also carry multiple records and every one agrees on
  the box.
- **The #204 column heuristic is retired in favour of a direct test (#211).**
  `partition_schema_mvp` no longer drops origin-centred family-local bounding
  boxes and no longer collapses co-located footprint groups to their highest
  `ElementId`; it applies
  `PartitionElementRecord::is_exported_instance` instead. Columns stay at
  256/256, and the same test additionally reproduces the wall, door and window
  id sets — which the heuristic could not (648 / 139 / 8 against 360 / 132 /
  6). `is_family_local` is kept as a documented, strictly weaker diagnostic:
  the `+0x42` symbol set is a superset of it on every category, catching 15
  door, 2 window and 1 wall symbols whose envelope is centred on only one axis.
  `columns_from_records` remains as a back-compat alias for
  `instances_from_records(records, "Column")`.
- **`walls`, `doors` and `windows` are now scored, cross-witness-agreed
  categories.** `tests/fixtures/project-counts/2024-core-interior-slim.json`
  moves all three from `known_gap` (decoder 0) to `known` at tolerance 0, so
  the OctetProof verdict for `magnetar-2024-core-interior-slim` compares
  `entity_counts.IFCWALL` / `IFCDOOR` / `IFCWINDOW` across all three lineages
  instead of excluding them — the claimed surface widens from five fields to
  eight, four of which now assert presence rather than absence. The sibling
  `magnetar-2024-core-interior` manifest records them as `decoder_baseline`
  (its 20 KB reference fixture carries none). Both committed verdicts still
  `PASS`; both `rvt-rs.json` observations were regenerated, so their
  `observation_hash_sha256` changed. `docs/support-matrix.json`'s
  `column-instance-recovery` row is superseded by
  `element-record-instance-recovery`, still `verified`, now covering four
  `BuiltInCategory` values with the scope and the remaining gaps stated.
- **RE-19 is untouched and still reported.** The typed Wall / Door / Window
  recovered here come from the element record's own `BuiltInCategory` field,
  not from the 2024 `ArcWallRectOpening` index and not from a schema-field
  decode. `schema_field_wall_instances` therefore still appears in
  `diagnostics.unsupported_features` — the check now looks for a wall whose
  body did **not** come from an element record, so an element-record wall can
  no longer clear it by accident.
- **Diagnostics: two composite `unsupported_features` split into what is
  actually still missing.** `typed_door_window_discrimination_and_host_binding`
  now fires only when no typed door or window is exported at all; when they
  are exported but carry no host, the narrower
  `door_window_host_wall_binding` fires instead. A new
  `revit_element_parameters_to_ifc_property_sets` fires while every exported
  `IfcPropertySet` is rvt-rs provenance about a recovered body rather than a
  Revit element parameter (#35) — the two manifest rows that previously
  borrowed the door/window feature name now use accurate ones.
- **`levels` moves from 12 name-derived storeys to 11 measured ones on Core
  Interior (#213).** Both project-count manifests move
  `diagnostics.exported.storey_count` from 12 to 11: the previous 12 came from
  partition Level-like name strings that all defaulted to elevation 0.0, so
  exactly one of them matched the export by accident; the 11 that replace them
  are measured elevations that all match. Both `rvt-rs.json` observations were
  regenerated (`IFCBUILDINGSTOREY` 12 → 11, `IFCRELCONTAINEDINSPATIALSTRUCTURE`
  1 → 11, `IFCLOCALPLACEMENT` 352 → 351, `IFCPROPERTYSINGLEVALUE` 1536 → 1792
  for the new `StoreyBindSource` property, `storey_count` 12 → 11), so their
  `observation_hash_sha256` changed; both committed verdicts still `PASS`
  byte-identically, because `entity_counts.IFCBUILDINGSTOREY` is a
  `decoder_baseline` field and is excluded from the claimed surface.
  `unsupported_geometry_missing_level` drops 274 → 18 — only the 18 spaces
  remain unbound.
- **Both committed synthetic IFC fixtures were regenerated** so they carry the
  full attribute lists from #214. `tests/fixtures/synthetic-project.ifc` and
  `tests/fixtures/synthetic-structural.ifc` change entity *text*, not entity
  *counts*.
- **API (breaking, pre-`0.2.0`)** — `elem_table::ElemTableLayout` gains a
  `marker_offset` field (offset of the `FF` marker *within* a record), so
  struct-literal construction needs updating. `start` now means record 0's
  first byte rather than the first `0xFF` run. `rvt-elem-table`'s text
  summary reports the marker's in-record offset instead of sniffing byte 0.
- **crates.io packaging works again** — `stream-evidence` is a path-only
  dev-dependency (Cargo strips it at publish) and `docs/data/*.csv` is no
  longer excluded from the package (`src/class_tag_map.rs` include_str!s
  the tag-drift CSV). `cargo publish -p rvt --dry-run` packages and verifies.
- **Release path proven by dry-run** — `publish.yml`'s viewer smoke now uses
  the same magnetar Einhoven sample as `deploy-viewer.yml` (the old phi-ag
  family sample could never satisfy `projectSampleTest`) and installs wabt so
  the WASM import audit runs; the CLI smoke keeps phi-ag. Three
  `workflow_dispatch` dry-runs: all verification jobs green.
- **wasm-pack builds straight into `viewer/pkg`** via `--out-dir` (no
  `rm`/`mv` shuffle) in every workflow and doc.
- **Formats/Latest page-strip stays disabled**; the walker now records when
  the 64 KiB schema scan cap applies only through #188 (open).
- **Honesty sync** — README, `docs/status.md`, compatibility, and
  supported-profile language emphasize inspection + narrow MVP
  recovers; generic converter claims removed.
- **ADR-004** — desktop distribution wrappers deferred.

### Fixed

- **Every emitted element was placed twice (#232).** The writer put the
  element's own `IfcAxis2Placement3D` in *both* the `IfcLocalPlacement`'s
  `RelativePlacement` and the `IfcExtrudedAreaSolid`'s `Position`. IFC4 has a
  consumer compose `ObjectPlacement × Position`, so the element translation
  was applied twice — on the 2024 Core Interior export that put the first
  column at (48, 220, 159.17) ft where Revit's own export puts it at
  (23.88, 110, 81.58) ft, with world-coordinate errors up to 185 ft across
  834 elements. Viewers that ignore `Position` drew the file correctly, which
  is why it survived every release: two tools could disagree about the same
  bytes with neither of them misreading. The solid's `Position` is now the
  project-level identity placement, and the placement carries the translation
  once — the split Revit's own exporter uses on its `IfcWall` bodies. Measured
  with IfcOpenShell's geometry iterator under `USE_WORLD_COORDS`, matched by
  `Tag` against Revit's export: all 80 slabs now agree to 0.0000 ft on every
  vertex (79 of 80 centroids inside 1e-3 ft), against a 185.33 ft worst-case
  before. The residuals that remain on columns, walls, doors and windows are
  geometry *recovery* fidelity — envelope-vs-panel bodies, base elevations —
  not placement composition, and are tracked on their own issues.
  `tools/ci/ifc_schema_arity.py` now gates the class two ways on every export:
  no swept solid may use the very `IfcAxis2Placement3D` instance its product's
  placement already uses, and five pinned elements — two in each committed
  fixture, a column and a slab of the recorded artifact — must land on a
  pinned coordinate when `ObjectPlacement × Position × profile-point` is
  composed for real. Entity counts, relations and storeys are unchanged; both
  OctetProof observations replay byte-for-byte and neither verdict moved.
  Regenerating `tests/fixtures/synthetic-project.ifc` for this also lands a
  change it should have carried since #222: the fixture was last dumped at
  `f4b88df` (#217), before `08f7858` gave `IfcOpeningElement` the `Tag` of the
  element that fills it, so its one opening still wrote `$` there. A fixture
  is only regenerated behind `DUMP_IFC=1`, which is how it drifted a commit
  behind the writer without any test noticing.
- **Host recovery for doors and windows sat in a dead branch (#222).**
  `ifc::export_content` consulted `recover_door_host` /
  `recover_window_host` only in the `else` arm of the element-record
  geometry check, so a door with a record bounding box — which is every
  door recovered since #211 — skipped it entirely and could never be
  voided into a wall. The lookup is now independent of which geometry
  carrier produced the body.
- **The void/fill chain depended on element emission order (#222).**
  `ifc::step_writer` resolved `host_element_index` against
  `entity_index_to_el_id` *inside* the element loop, so a host emitted
  after its own opening silently dropped the `IfcRelVoidsElement` /
  `IfcRelFillsElement` pair. Both relationships are now emitted after
  the loop, when the index is complete.

- **Every emitted IFC4 instance now carries the full attribute list its entity
  type declares (#214).** The writer emitted building elements with the eight
  `IfcProduct` + `Tag` attributes and stopped, so `IFCCOLUMN`, `IFCSLAB`,
  `IFCWALL`, `IFCBEAM` and friends were one attribute short of the schema's
  nine, `IFCDOOR` / `IFCWINDOW` three short of thirteen, and `IFCSPACE` three
  short of eleven — 338 non-conformant instances on `2024_Core_Interior.rvt`
  (256 + 64 + 18), 21 across the two committed fixtures. Reading
  `PredefinedType` off such an instance raised *"Index 8 is out of range for
  variant of size 8"*. `src/ifc/step_writer.rs` now drives emission from a
  per-type attribute layout, `IfcEntity::BuildingElement` carries a
  `predefined_type` populated from `category_map` (`.COLUMN.` for `Column`,
  `.FLOOR.` for `Floor`, `.STANDARD.` for `Wall`, `.INTERNAL.` for `Room`, …),
  and unknown optionals occupy their slot as `$` rather than being dropped.
  `IFCSPACE` also stops writing the type GUID into slot 8: `IfcSpace` is a
  spatial element with no `Tag`, so that slot is `LongName`.
- **Three further attribute-arity defects the same audit surfaced.**
  `IFCISHAPEPROFILEDEF` wrote 8 attributes against IFC4's 10 (the IFC4-only
  `FlangeEdgeRadius` / `FlangeSlope` were missing), `IFCCLASSIFICATIONREFERENCE`
  5 against 6 (`Sort`), and `IFCMATERIALLAYERSETUSAGE` 4 against 5
  (`ReferenceExtent`). `IfcBuildingStorey.Elevation` was written as a bare
  integer literal (`0`) where ISO-10303-21 requires a REAL; it is now
  `0.000000`. The emitted Core Interior IFC goes from 338 mismatching
  instances to 0 across all 33 entity types.
- **A hostile `PredefinedType` can no longer corrupt a STEP record.** An
  enumeration reference is written bare between dots and has no escape form, so
  a value that is not a legal STEP enumeration name now degrades to `$`
  (`tests/fuzz_regressions.rs` drives adversarial names through the path).

- **Geometry-coverage diagnostics no longer read "solved" from a partial
  export.** `unsupported_features` used to carry `real_file_element_geometry`
  only while *no* exported element had a body, so an export where one category
  gained geometry silently dropped the gap for every other category. It now
  reports `real_file_element_geometry` when nothing has a body and the new
  `partial_element_geometry` when some do and some do not — on Core Interior,
  256 `IFCCOLUMN` with bodies against 82 slabs/spaces without. The slim
  manifest's `rooms_spaces` gap points at the new code accordingly. Documented
  in `docs/export-diagnostics.md`; both `rvt-rs` observations record the new
  code in `unsupported_entities`.
- **`Global/ElemTable` record origin on the 40-byte project variant (#206)** —
  `elem_table::parse_records` walked 26,424 of the 26,425 declared records on
  `2024_Core_Interior.rvt`. `detect_layout` took the first `0xFF` run as
  record 0's first byte, but on that variant the run is a sentinel-valued
  *field inside* the record: each record opens with a zero `u32` and only then
  carries `FF`×8. The four-byte shift ran the walk out of bytes just short of
  the last record. The decompressed stream is 1,057,030 bytes (gzip CRC32 and
  `ISIZE` both verify) = `0x1E` + 26,425 × 40 exactly, and the `u32` ahead of
  every marker is zero on all 26,425 records, so the origin is recovered as
  `len − record_count × stride`; new `ElemTableLayout::marker_offset` keeps
  field extraction anchored to the marker. Id values on the previously-parsed
  records are byte-identical; `ElemRecord::offset`/`raw` shift by −4 and the
  final record is no longer lost. The 28-byte 2023 variant is unchanged — its
  end-anchored origin would fall five bytes ahead of the marker, which the
  `u32`-alignment guard rejects, so it still walks 2,614 of 2,615 with a
  23-byte tail (now pinned exactly rather than asserted as `<=`). Byte
  evidence and record-count semantics:
  `docs/elem-table-record-layout-2026-04-21.md` § "Where the record array
  starts". Committed 270-byte MIT regression fixture under
  `tests/fixtures/elem-table/`. No OctetProof observation changed
  (`rvt-rs.json` stays `b6d9b67c…` on both recorded edges).
- **Corpus gate gap that hid it (#206)** — `elem_table_corpus` read
  `RVT_PROJECT_CORPUS_DIR` but was named by no CI job, so it took its skip
  path on every run. It is now named by the `test` matrix job (family half,
  `RVT_REQUIRE_CORPUS=1`) and by `corpus-tier2` (project half), together with
  the six other corpus-reading targets that were equally ungated
  (`arc_wall_corpus`, `partition_scanner`, `iter_elements_typed`,
  `re15_geometry_invariants`, `re19_door_window_wall_negative`,
  `walker_to_ifc_integration`). `elem_table_corpus` now fails instead of
  skipping when the corpus is configured but a file is missing, and carries
  one always-on committed-bytes test so the target is never wholly vacuous.
- libFuzzer-caught panics in `compression::gzip_header_len`,
  `basic_file_info::extract_path`, and fuzz harness string truncation
  (nightly fuzz matrix unblocked after upload-artifact pin repair).
- Finding 1 strip gate narrowed to exclude Formats/Latest (#162).
- **CI / Deploy viewer baselines** — re-pin project-count
  `material_count` expectations after Finding 1 recoveries (Core
  Interior 102, Einhoven 42); `cargo-deny` path-dep version pins for
  `stream-evidence`; stream-evidence fails closed on explicit stream
  filter / `--all-paged` misses instead of silent first-stream fallback.
- **CI runs every integration test, not a hand-kept list of them.** The
  test matrix ran `--lib --bins` plus a few integration targets by name,
  and the tier-2 job a second name list; 21 of the 37 `tests/*.rs` files
  ran in no job at all — among them the `support_matrix` and
  `witness_registry` honesty gates the docs describe as CI-enforced, the
  JSON schema contracts, `binary_inventory`, `fuzz_regressions`,
  `proptest_parsers` and every CLI test. Both jobs now run
  `cargo test --test '*'` (matrix: every OS, family corpus where fetched;
  tier 2: against the magnetar project corpus), so a new target is gated
  the moment it exists. Running them surfaced one stale assertion:
  `rvt_ifc_diagnostics_cli` pinned the strict-mode rejection to confidence
  `scaffold` while the 2024 sample family has exported at
  `typed_no_geometry` since its typed elements began to export; it now
  asserts any below-geometry level. The first Windows run of the CLI tests
  showed a closed pipe reports `os error 109` ("The pipe has been ended")
  there, which `rvt::cli::exit_quietly_on_broken_pipe` now recognises
  alongside 32 and 232.

### Security

- **CI Actions SHA pins** — remaining floating third-party tags
  (`actions/checkout@v4`, `setup-python@v5/@v6`, `upload-artifact@v7`,
  `maturin-action@v1`, `dtolnay/rust-toolchain@{stable,nightly,master}`)
  pinned to full commit SHAs with version comments across workflows
  (supply-chain follow-up to publish.yml pins).
- Viewer toolchain bumps (vite major line) for Dependabot advisories on
  optimized-deps / transitive esbuild CORS (dev-server class; production
  Pages build still uses `vite build`).
- pyo3 line bumps for advisory GHSA-pph8-gcv7-4qj5 (bindings crate).

## [0.1.2] — 2026-04-19

First tagged release since 0.1.0. Bundles the Python bindings,
document-level IFC4 export, Layer 5a ADocument walker, and the
spatial-hierarchy / classification extensions that land between
`v0.1.0` (initial public release) and the PyPI debut. Changelog
entries previously accumulated under `[Unreleased]` move here
verbatim.

### Changed — IFC exporter now emits the full spatial hierarchy

- **`rvt-ifc` output now includes `IfcSite → IfcBuilding → IfcBuildingStorey`**
  with `IfcLocalPlacement` per container and `IfcRelAggregates`
  binding each level to its parent. Previous output was a valid-but-
  empty `IfcProject`; BlenderBIM and IfcOpenShell-based viewers
  accepted it but couldn't render anything because there was no
  spatial structure for them to attach geometry to. The minimal
  `Default Site / Default Building / Level 1` hierarchy now opens as
  a navigable scene directly. Once the walker surfaces real
  `BasePoint` / `Level` / `Building` records from the Revit file,
  these placeholder names and the zero-elevation storey will be
  replaced with the actual values.
- **`make_guid(index)` deterministic GUID generator** — replaces the
  constant `random_guid_stub()` placeholder. Emits 22-character
  strings in the IFC-GUID alphabet (`0-9A-Za-z_$`), prefix `0rvtrs`
  + base-64 big-endian-encoded entity index. Every entity in one
  export now has a distinct GUID; identical models produce
  byte-identical STEP output (STEP text diffs now work).
- **`IfcClassification` + `IfcClassificationReference` +
  `IfcRelAssociatesClassification` emission.** `RvtDocExporter`
  already extracted OmniClass codes from PartAtom (e.g.
  `23.45.12.34`) into `model.classifications`; the STEP writer now
  actually emits them. Each classification source (OmniClass,
  Uniformat, …) gets one `IfcClassification`; each coded item gets
  an `IfcClassificationReference` linked back to its source; the
  project gets one `IfcRelAssociatesClassification` per reference
  binding the code to the root `IfcProject`. BIM consumers that
  track code/category provenance (Solibri, IfcOpenShell
  classification viewer) can now read those codes directly from
  the exported IFC.
- 7 new unit tests total pinning spatial-hierarchy presence,
  entity counts, GUID alphabet, GUID determinism, per-file GUID
  uniqueness, OmniClass classification emission with items + names
  + edition, and a guard that empty classifications produce no
  classification entities. Existing `ifc_roundtrip` integration
  tests continue to pass across the 11-release corpus.

### Added — Python bindings via pyo3 + maturin

- **`rvt` Python package** — `pip install rvt` produces a single wheel
  per OS/arch that works on every Python ≥ 3.8 (via pyo3 `abi3-py38`).
  Pure-Python `rvt` package wraps the compiled `rvt._rvt` extension
  and ships a PEP 561 `py.typed` marker + hand-maintained
  `__init__.pyi` stubs so mypy, pyright, and IDE autocomplete work
  out of the box.
- **`rvt.RevitFile` class** — Python surface onto `RustRevitFile`.
  Properties: `version`, `original_path`, `build`, `guid`,
  `part_atom_title`. Methods: `stream_names()`,
  `missing_required_streams()`, `schema_summary()`,
  `read_adocument()` (returns a dict with the walker's
  `ADocumentInstance` serialised to native Python types), and
  `write_ifc()` (returns the IFC4 STEP text).
- **`rvt.rvt_to_ifc(path)`** one-shot helper — equivalent to
  `RevitFile(path).write_ifc()` for callers that just want the IFC
  string and never touch the intermediate object.
- **`RevitFile.schema_json()`** — returns the full schema as a JSON
  string (parse with `json.loads` to get a dict equivalent to
  Rust's `SchemaTable`). Zero-copy relative to the decoded schema;
  ~1-2 MB per typical Revit family. `schema_summary()` remains the
  cheap counts-only variant. Two new pytest tests cross-check that
  summary counts match `schema_json()`'s full-parse counts and that
  the `ADocument` class (the walker's target) is always present.
- **`RevitFile.basic_file_info_json()`** — `BasicFileInfo` as JSON
  in one call. Single-call equivalent of the four individual
  getters (`version` / `original_path` / `build` / `guid`) plus
  any future fields. Returns `None` when the stream is unparseable.
- **`RevitFile.part_atom_json()`** — `PartAtom` as JSON in one
  call. Superset of `part_atom_title` — also carries `id`,
  `updated`, `taxonomies`, `categories`, `omniclass`, and `raw_xml`
  (the original XML for lossless downstream reuse). Returns `None`
  when the stream is absent (common on project `.rvt` files).
- Two new pytest tests pin `basic_file_info_json` ↔ individual
  getters agreement, and `part_atom_json` ↔ `part_atom_title`
  agreement plus presence of the structural keys.
- **`RevitFile.read_stream(name)`** — return the raw bytes of an
  OLE stream by name as a Python `bytes` object. Accepts either
  path form (`"/Formats/Latest"` or `"Formats/Latest"`). Raises
  `IOError` for unknown streams. Use `stream_names()` to enumerate
  what's available. Opens up forensic-inspection use cases the
  announcement draft calls out (reading raw bytes without the
  Rust-API dependency). Three new pytest tests pin bytes
  round-trip, path-normalisation equivalence, and
  missing-stream-raises semantics.
- **CI wheel build matrix** (`.github/workflows/ci.yml` `python-wheel`
  job) — `PyO3/maturin-action@v1` builds a release wheel on Ubuntu,
  macOS, and Windows runners, installs it into the runner's Python,
  runs the pytest integration suite (`tests/python/test_rvt.py`), and
  uploads the wheel as a workflow artifact. Any regression in the
  Python surface fails CI across all three OSes.
- **38 pytest integration tests** covering module surface, error
  handling on missing / non-CFB files, happy-path reads against every
  one of the 11 corpus releases (2016–2026), cross-version
  `read_adocument` consistency-band checks, and `write_ifc` output
  shape. Gracefully skips with a clear message when
  `_corpus/rac_basic_sample_family` is absent so local runs work
  without LFS fetches.
- **`docs/python.md`** — full Python API reference (install, quick
  start, tables per method, return shapes, error handling,
  limitations, troubleshooting, contribution notes).
- **`docs/rvt-python-quickstart.ipynb`** — 15-cell Jupyter notebook
  mirror of `docs/python.md` for anyone who prefers an interactive
  walkthrough.
- **`.github/workflows/publish.yml`** — PyPI release workflow. Fires
  on tag push (`v*`) or `workflow_dispatch`. Builds wheels on
  Ubuntu / macOS / Windows via `PyO3/maturin-action@v1`, builds the
  sdist on Ubuntu, downloads every artifact into one `dist/`, and
  publishes via `pypa/gh-action-pypi-publish` using PyPI's Trusted
  Publisher flow (OIDC) — no `PYPI_API_TOKEN` secret stored in the
  repo. Supports `workflow_dispatch` with `test-pypi: true` for
  TestPyPI dry runs. Per-tag releases will cover every Python ≥ 3.8
  on mainstream OSes with one wheel each.

Design principle: expose only the stable high-level surface
(metadata, walker-read ADocument, IFC export). The low-level
byte-pattern / `FieldType` machinery stays in Rust; Python callers
get dicts and strings, no wrapper types to learn. To rebuild the
wheel locally: `maturin build --release --features python`.

### Added — Layer 5: first end-to-end `rvt → ifc` pipeline

- **`rvt::ifc::step_writer::write_step`** — pure-Rust IFC4 STEP
  serializer. Takes an `IfcModel`, produces spec-valid ISO-10303-21
  text with all required framework entities (IfcPerson,
  IfcOrganization, IfcApplication, IfcOwnerHistory, IfcSIUnit×4,
  IfcUnitAssignment, IfcGeometricRepresentationContext, IfcProject).
  No IfcOpenShell dependency. No `unsafe`. 4 new unit tests pinning
  envelope shape, escaping, and required entities.
- **`rvt::ifc::RvtDocExporter`** — concrete `Exporter` that
  populates `IfcModel` from a `RevitFile`. Extracts project name
  from PartAtom (falls back to BasicFileInfo path), builds a
  description string from version + id, pulls OmniClass codes into
  `ClassificationSource::OmniClass`.
- **`rvt-ifc` CLI** — ninth shipped binary. `rvt-ifc input.rfa`
  writes `input.ifc` next to the input. `rvt-ifc -o path input.rfa`
  overrides. `--null` uses the empty-project exporter for
  STEP-writer testing.

First end-user deliverable for Layer 5: `cargo run --release --bin
rvt-ifc -- sample.rfa` produces a ~1 KB IFC4 file that
IfcOpenShell, BlenderBIM, and buildingSMART validators can read.
Geometry and per-element entities are pending walker expansion;
this v1 covers document-level metadata.

### Fixed
- **Windows CFB stream-name path separator.** `RevitFile::stream_names()`
  returned backslash-separated paths on Windows (`Formats\Latest`)
  because `Path::display()` uses host-native separators. Now
  normalises to forward-slashes across all OSes so
  `has_revit_signature()` and equivalent cross-stream comparisons
  work uniformly. This was the root cause of the Windows-only
  integration-test failures on the 2016 sample.
- **MSRV compliance.** Removed a `if let ... && ...` let-chain that
  crept in; let-chains require Rust 1.88+ and the crate's MSRV is
  1.85. Rewrote as nested `if let { if cond { ... } }`.

### Added — Layer 5a walker + rvt-doc CLI

- **`src/walker.rs` module** — first end-to-end schema-directed
  instance reader. Exposes `read_adocument(&mut RevitFile) ->
  Result<Option<ADocumentInstance>>` returning `ADocumentInstance {
  entry_offset, version, fields }` where each field is one of
  `InstanceField::{Pointer, ElementId, RefContainer, Bytes}`.
- **`rvt-doc` CLI** — eighth shipped binary. Dumps ADocument's
  instance fields as human-readable text or machine-readable JSON
  with `--json`. Respects `--redact` for user-path scrubbing.
- **Cross-version detection** — hybrid entry-point finder that
  combines a sequential-id-table heuristic with a scoring-based
  brute-force fallback. **Reliable on Revit 2024–2026**; older
  releases (2016–2023) need further entry-point detection work.
  Observed version bands if/when older releases land:
  2016–17 / 2018 (solo) / 2019–20 / 2021–23 / 2024–26.
- **`RevitFile::missing_required_streams()`** — diagnostic form of
  `has_revit_signature`. Returns the list of required stream names
  not found in the file, so "signature invalid" errors can point
  at the specific missing stream.

### Research progress

- **Q6.3**: refuted Q6.2's "post-history bytes are ADocument"
  hypothesis. The 131-record table at the post-history boundary
  is a multi-table directory, not ADocument's instance.
- **Q6.4**: directory u16 body values are not cross-stream
  references. Two sequential-id tables (Table A + Table B) exist
  in Global/Latest.
- **Q6.5-A/B**: post-Table-B region at 0x0f67 (2024) is where
  ADocument's actual instance data lives. 33× class-tag density
  vs uniform-random baseline.
- **Q6.5-C**: first-pass walker drifts after field 2 because
  Container wire encoding was wrong.
- **Q6.5-D**: Container wire is two-column `[u32 count][12 × 6B
  ids][u32 count][12 × 6B masks]` = 152 bytes for count=12.
- **Q6.5-E**: walker reads 8/13 fields cleanly on Revit 2024.
- **Q6.5-F**: walker reads ADocument on Revit 2024–2026 with
  cross-version-byte-identical output within each version band.
  Older releases (2016–2023) identified the entry-point band but
  still need hardening — tracked as L5B-11.

## [0.1.1] — 2026-04-19

### Added
- **CI-enforced 100% schema-field classification.** New integration
  test `tests/field_type_coverage.rs` opens every file in the 11-version
  `rac_basic_sample_family` corpus, parses the schema, and asserts zero
  fields decode to `FieldType::Unknown`. Fails if any release regresses
  or if the corpus is incomplete — no silent-skip. CI job fetches the
  corpus from [phi-ag/rvt](https://github.com/phi-ag/rvt) at build time
  via `actions/checkout@v4` with LFS (rvt-rs does not redistribute the
  Autodesk-owned sample files; see SECURITY.md).
- `FieldType` enum with 8 variants (`Primitive`, `String`, `Guid`,
  `ElementId`, `ElementIdRef`, `Pointer`, `Vector`, `Container`) —
  classifies **100.00% of all 13,570 schema fields** across the 11-version
  reference corpus (Revit 2016–2026). Zero fields decode to `Unknown`.
  Evidence: `examples/unknown_bytes_deep.rs` against every sample file.
- `ClassEntry.tag`, `.parent`, `.ancestor_tag`, `.declared_field_count`,
  `.was_parent_only` — richer schema metadata with cross-release stability.
- `writer::write_with_patches` + `StreamPatch` / `StreamFraming` types —
  stream-level modifying writer; verified end-to-end round-trip on
  `Formats/Latest`.
- `compression::truncated_gzip_encode` + `truncated_gzip_encode_with_prefix8`
  — inverse of `inflate_at`, producing Revit-compatible gzip bytes.
- `redact` module with `redact_path_str` + `redact_sensitive` —
  shared PII scrubber used by every CLI's `--redact` flag.
- `rvt-analyze` CLI — one-shot forensic analysis. 7 subsystems: identity,
  history, format anchors, schema, schema→data link, content metadata,
  disclosure scan. `--json`, `--section`, `--redact`, `--quiet`,
  `--no-color`.
- `rvt-info --redact` and `rvt-history --redact` — PII propagation to the
  other shipped CLIs.
- `elem_table` + `partitions` modules — Global/ElemTable + Partitions/NN
  header parsers.
- `ifc` module — Layer 5 scaffold: `IfcModel`, `Exporter` trait,
  `NullExporter`, full Revit-class → IFC-entity mapping plan.
- `writer::copy_file` — byte-preserving OLE round-trip (13 streams
  identical, verified).
- 14 new reproducible probes under `examples/` covering every FACT in
  the reconnaissance report.
- `tools/bench.sh` hyperfine benchmark harness + `docs/benchmarks.md`.
- First publicly-available RVT tag-drift table — `docs/data/tag-drift-2016-2026.csv`
  (122 classes × 11 releases) + `tag-drift-heatmap.svg`.
- First publicly-documented Revit format-identifier GUID
  (`3529342d-e51e-11d4-92d8-0000863f27ad`) — stable across every Revit
  release 2016-2026.

### Changed
- Library surface reorganised; `src/lib.rs` has a proper crate-level
  doc with a quickstart example, moat-layer table, and module inventory.
- `FieldType::Primitive` now carries `{kind, size}` instead of
  `{size_hint}`.
- `FieldType::Container` now carries a `kind: u8` field marking the
  element base type (so `Container<u32>` is distinguishable from
  `Container<f64>` / `Container<ref>`). Existing consumers that
  destructure with `..` continue to work.
- `FieldType::decode` is now panic-safe on short inputs: 0/1/2/3-byte
  slices produce either `Unknown` or a typed variant with an empty body
  rather than a bounds-check panic.
- `scan_fields_until_next_class_bounded` respects `declared_field_count`
  — fixes the over-reader that bled from HostObjAttr into Symbol's
  fields.

### Research findings (Phase 4c)

- **Q4**: The u16 "flag" in each tagged-class preamble is an
  **ancestor-class reference**, not a bitmask. 9/9 non-zero values in
  the 2024 sample resolve to named classes in the same schema.
- **Q5**: Decoded the field `type_encoding` byte sequence. 9 category
  discriminators + sub-type variants.
- **Q5.1**: Extended to 84% coverage — wider primitive discriminators
  (`0x01 bool`, `0x02 u16`, `0x05 u32`, `0x06 f32`, `0x07 f64`,
  `0x08 string`, `0x09 GUID`, `0x0b u64`).
- **Q5.2**: Extended to **100.00%** coverage across the 11-version
  corpus. Generalized `{scalar_base} 0x0010 ...` → `Vector<base>` and
  `{scalar_base} 0x0050 ...` → `Container<base>` for every scalar base
  (previously only `0x07 0x10` and `0x0e 0x50` were mapped). Added the
  `0x0d` point/transform base (seen only in composite form), the
  `0x08 0x60 ...` alternate string encoding, the `ElementIdRef { tag,
  sub }` variant (for references that carry a specific referenced-class
  tag — 80+ fields per release use this), the deprecated `0x03` i32-
  alias (2016–2018 only, 5 fields), and robust handling of truncated
  2-byte `{kind}{modifier}` headers (schema-parse boundary artifacts).
- **Q6**: `Global/Latest` is **not** an index + heap. It's a flat
  TLV stream.
- **Q6.1**: Instance data is **schema-directed** (tag-less, protobuf-
  style). Decoding requires schema-first sequential walk from a known
  entry point.
- **Q6.2**: Initial hypothesis — entry point located at offset `0x363`
  in the 2024 sample (right after the document-upgrade-history
  UTF-16LE block). Confidence 0.6. **Refuted by Q6.3.**
- **Q6.3 CORRECTION**: The Q6.2 entry-point hypothesis is refuted by
  rigorous validation against the 11-version corpus. The bytes at the
  post-history boundary are NOT ADocument's 13-field instance — they
  are a multi-table directory / reference-pool with ~131 sequentially
  numbered records per release (stable count across all 11 years,
  unchanged from the 13 that would be expected if this were
  ADocument). Body-size does not correlate with FieldType; body u16
  values do not resolve to schema class tags (0/131 hit). ADocument's
  actual location in `Global/Latest` (or another stream) is not yet
  known — decoding the directory table format is the next open
  research question (Q6.4+). Probes: `examples/adocument_walk.rs`,
  `examples/post_directory.rs`, `examples/directory_class_lookup.rs`.
  See `docs/rvt-moat-break-reconnaissance.md` §Q6.3 for full evidence.
- **Q7**: `Partitions/NN` trailer u32 fields are **not** per-chunk
  offsets. Gzip-magic scan remains correct.

## [0.1.0] — 2026-04-19

Initial public release.

- OLE2/MS-CFB container reader (via `cfb`) — Layer 1.
- Truncated-gzip decompression (via `flate2`) — Layer 2.
- Per-stream framing for `Formats/Latest`, `Global/Latest`,
  `Global/ElemTable`, `Partitions/NN`, `Contents`, `PartitionTable`,
  `RevitPreview4.0` — Layer 3.
- Schema table parser: class names + fields + tags + parent classes
  + declared field counts + cross-release tag-drift map — Layer 4a.
- Phase D moat proof: class tags from `Formats/Latest` occur in
  `Global/Latest` at ~340× uniform-random rate — Layer 4b.
- `FieldType` enum with 7 initial variants (Primitive, ElementId,
  Pointer, Vector, Container, String, Guid). **84% field-type
  classification** on a typical Revit 2024 sample family — Layer 4c.
- Stream-level modifying writer (`write_with_patches`) with
  byte-preserving round-trips verified on all 13 streams — Layer 6.
- Seven shipped CLIs: `rvt-analyze`, `rvt-info`, `rvt-schema`,
  `rvt-history`, `rvt-diff`, `rvt-corpus`, `rvt-dump`.
- Full PII-redaction (`--redact`) across every CLI.
- First publicly-documented Revit format-identifier GUID
  (`3529342d-e51e-11d4-92d8-0000863f27ad`), stable across every
  Revit release 2016–2026.
- First public RVT tag-drift table: 122 classes × 11 releases CSV
  plus SVG heatmap.
