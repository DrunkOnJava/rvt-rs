# Supported profile

What rvt-rs can be trusted with today, in the terms a BIM user would check.
The evidence behind every line is in [`support-matrix.json`](support-matrix.json)
(`rvt-capabilities --matrix -f text` prints it) and in the measured tables of the
[README](../README.md#what-rvt-rs-reads-from-real-projects). This page
describes `main`; the latest release is v0.11.0, and what has landed since
is in the fragments under [`changelog.d/`](../changelog.d/README.md).

## Four levels of "it works"

A Revit file can succeed at one level and not the next, so rvt-rs keeps them
apart:

1. **Opened.** The OLE container, its streams, metadata, previews and schema
   read: every Revit release from 2016 to 2027, across the family corpus.
   Revit 2015 files open too, on Autodesk's sample projects, where previews
   and PartAtom are not measured.
2. **Elements identified.** Each element is found with its Revit ElementId and
   category, and the set equals the one Revit's own IFC export holds. Revit
   2024 and 2025 project files, and Revit 2023 ones with their bounding boxes
   only (RE-81). Experimentally, Revit 2026 ones, measured so on one project
   (RE-124), and Revit 2015 to 2022 and 2027 ones, which have no measured
   Revit export of their own release (except one 2015 project, BIMcollab's
   structural example, RE-198) and are measured against the same sample projects saved by
   Revit 2019 (for 2015 to 2018), 2023 (for 2019 to 2022) and 2026 (for 2027)
   instead: a 2027 project's ElementIds and classes are its 2026 copy's, and
   on 2015 to 2022 every ElementId shared with the copy keeps its class, with
   the copy's building elements class for class where Autodesk did not edit
   the model between releases (the table below).
3. **Geometry.** An element's body. Exact where the element's data is decoded
   (the table below), otherwise its bounding box, and the export diagnostics
   say which.
4. **Properties.** Names, types, storey, materials, layers and GlobalIds are
   read; most other parameters are not (#35, #155).

## By input

| Input | Opened | Elements identified | Geometry | Properties |
|---|---|---|---|---|
| Revit 2024 and 2025 project (`.rvt`) | yes | yes, measured on Core Interior, the four RE1 models, Snowdon Towers, Projeto1 and `teste_export_2025` (RE1's exports are by Revit 26.2; one plumbing fixture on RE1 Plumbing and 23 elements on Snowdon Towers are not in Revit's export) | see below | see below |
| Revit 2026 project | yes | experimental (since 0.5.0): measured on one 2026 project against Revit's own IFC4X3 export, 113 of its 115 elements and none outside it (RE-124) | the element's bounding box, walls cut back at their joins | Revit's GlobalIds, storeys, rooms with Revit's numbers and names, doors and windows in their hosts, family and type names, layer sets |
| Revit 2023 project | yes | yes, measured on two 2023 projects against Revit's IFC4 exports (RE-81); components nested in doors and windows are left out | the element's bounding box; rooms their outline (RE-102) | ElementId, category, door and window host walls (RE-85), Levels as storeys with each element on the one its record names (RE-107), and family instances' family and type names (RE-109) |
| Revit 2027 project | partial: opens on Autodesk's sample projects; previews and PartAtom not measured | experimental (since 0.6.0): the same ElementIds and classes as the 2026 copies of Autodesk's five sample projects (RE-179); no Revit 2027 export is measured | as on 2026 | as on 2026 |
| Revit 2019 to 2022 project | yes | experimental (since 0.6.0): 2023's records and readers with each release's schema tags; every ElementId shared with the 2023 copies of Autodesk's sample projects keeps its class, and the `rac_basic`, `rst_basic` and 2021 and 2022 `rac_advanced` exports have the 2023 copies' building elements, class for class (RE-180); no Revit export of a 2019 to 2022 file is measured | as on 2023 | as on 2023 |
| Revit 2015 to 2018 project | yes on 2016 to 2018; 2015 partial: opens on Autodesk's sample projects; previews and PartAtom not measured | experimental (since 0.6.0): 2019's records and readers with each release's schema tags; every ElementId shared with the 2019 copies of Autodesk's 2016 to 2018 sample projects keeps its class, and their exports have the 2019 copies' building elements, class for class; on five of the six 2015 samples every shared ElementId keeps its class, and the sixth, `rst_basic`, is another model (RE-181); no Revit export of a 2015 to 2018 file is measured | as on 2019 | as on 2019 |
| Revit 2011 to 2014 project | yes, on local closed-source files only, outside the 1.0 range (2015 to 2027); 2011 to 2013 open since RE-259 | experimental: 2015's records and readers with each release's schema tags; on a local copy of Autodesk's 2013 `rac_advanced` sample every ElementId shared with its 2015 copy keeps its class, and its export has the 2015 copy's building elements, class for class, but for a re-divided curtain wall (RE-259); no Revit export of a file before 2015 is measured | as on 2015 | as on 2015 |
| Revit 2008 to 2010 project | yes, on local closed-source files only, outside the 1.0 range (2015 to 2027) (RE-259) | experimental: 2011's records and readers with each release's schema tags, 2008's envelope 4 bytes shorter (RE-260); checked only for consistency within each file, with no Revit export and no same-model copy | as on 2015 | storeys and named materials |
| Family (`.rfa`) | yes | family metadata and OmniClass only | none | a family's types and type parameters as its `PartAtom` stream lists them: Revit's text at save, not read from the family's own records |
| Template (`.rte`, `.rft`) | the same container code; no template file is in any corpus, so not measured | not measured | none | none |

A file that is corrupt, encrypted, a zero-byte Git LFS placeholder, or not an
OLE/CFB Revit container is refused with an error. A stream larger than
rvt-rs's resource limits (by default 2 GiB for the file, 256 MiB for one
stream and for one inflated member, and 1 GiB inflated per partition) is not
read, so the elements it holds are not exported. `rvt-ifc` and `rvt-gltf` then
name each such stream, write no model and exit with status 3 unless
`--allow-partial`, and the diagnostics list them; `--large-model` raises the
limits for models whose largest partition is several hundred MiB to 1 GiB
(#1075). The readers of native records (types, materials, stairs, phases, saved
parameters and others) follow the same limits and skip only the stream they could
not read. The browser viewer opens a file at the default limits and, when a
stream is not read, reports "Part of the file not read" and offers "Retry
with large-model limits" (streams up to 2 GiB); it stays under wasm32's 4 GiB
memory ceiling, so a very large model can still fail to open there. This is
measured by lowering the limits below the reference files' streams (Core
Interior's 17 MB partition against an 8 MiB limit). No model over the default
limits has been read with `--large-model` yet: the field report behind #1075
(two client models of about 438 MB and 1.13 GB, whose largest partitions are
403 MiB and 1,057 MiB) was measured on a patched 0.10.0; the reporter was
asked on 2026-10-30 to repeat it on 0.11.0 and has not reported back.

A project saved by Revit 2024 to 2027 can keep the schema of the release it
passed through, and with it that release's element-record layout. Such a
file is read as the release its schema belongs to, and the diagnostics say
so ("Saved by Revit X but keeps Revit Y's schema"). A schema that matches no
measured release still reads no element records. The field report behind
#421 found this on 25 of 49 client project files saved by Revit 2024 to 2027,
which 0.10.0 read as empty. None of the reference or sample files keeps an
older schema, so the reading is not yet measured on an affected file; the
reporter was asked on 2026-10-30 to check 0.11.0 on theirs and has not
reported back.

## Geometry on Revit 2024 and 2025 projects

| Element | What you get | How close |
|---|---|---|
| Walls | a body from the wall's centreline and its type's thickness, drawn as its layers in the viewer and glTF, ending as its joins say, along its arc when curved, leaning on its exterior face when tapered, its top cut at the underside of the floor, roof or footing that covers it whole | faces exactly Revit's on 1,030 of Snowdon's 1,078 walls; both ends on 724; all 360 of Core Interior's; both ends on 482 of the BIM4LCA office's 607 as built and 422 of 589 at building permit, a corner's ends not yet read from the join order (RE-222, RE-223); wall tops Revit's on 64 of the 65 the office cuts as built and 70 of 70 at permit, a wall a floor covers in part keeping its full height (RE-267) |
| Floors, building pads | the sketched plan outline, holes included, at the measured thickness | Core Interior 80 of 80 outlines; the BIM4LCA office's slabs with Revit's outline within 0.01 ft, 63 of 63 on each file (Measure run 40927543524) |
| Roofs | the sketched outline; a shed, gable or hip roof, or one sloped by an arrow, as its planes (RE-56, RE-199); a Revit 2025 roof by extrusion as its sketch's section (RE-211) | 11 of 13 outlined Snowdon roofs with Revit's area; on puzzbobb's Revit 2025 models, 19 of 19 roofs with Revit's body (RE-199, RE-211); hips over T and U footprints are level blocks (#884) |
| Columns | the column prism minus what walls cut from it | Core Interior 256 of 256, exact |
| Beams | along their location line with their section; a steel beam whose type stores an I section as that I (RE-103) | 923 of 942 Snowdon structural beams on their line; 377 drawn as their I, Revit's own shape and place on all 271 its VIM export scores; channels and angles are not drawn yet (#94) |
| Steel columns | a structural column whose type stores an I section as that I, turned to the column's own X axis (RE-104); a column whose type is a square hollow section as it, rounded corners and all, from its family's parameters (RE-221) | all 40 of Snowdon's steel columns; Revit's own shape and turn on all 24 its VIM export scores. Revit's own body on all 79 hollow-section columns of the BIM4LCA office as built and 55 at building permit; rectangular and turned tubes and other sections keep their box. Every I-section member carries its I as an `IfcMaterialProfileSet` of its type's material (RE-105) |
| Pipes | a cylinder along its two ends with its outside radius, where the ends and the record box prove one (RE-131) | all 69 pipes on RE1 Mechanical and Plumbing (Revit 2025) and the BIM4LCA office's 12 (Revit 2024), Revit's own radius, ends and length to 0.00001 ft; sloped pipes are not measured, ducts and fittings keep their box |
| Stairs | aggregates of flights, landings and supports, each stair's and flight's `PredefinedType` from its runs and landings; straight flights as their steps, monolithic ones included; riserless flights as their treads, spiral ones as their sector-shaped treads; a monolithic landing as its slab, and a landing that is one tread as that tread | Snowdon Towers 33 of 38 drawn flights equal to Revit's; on the BIM4LCA office (Revit 2024) all 35 flights have Revit's volume, its 34 straight ones and its spiral one, its 21 monolithic landings Revit's top and outline, and its tread landing Revit's body; on puzzbobb's Revit 2025 stair models every flight of models 13 to 16 has Revit's volume (RE-214, RE-217). A monolithic landing's underside is drawn flat where Revit's follows a flight's soffit, 100.4% to 104.7% of Revit's volume, and its `BodySource` says so; monolithic spiral runs and riserless runs with a nosing keep their box (#357) |
| Curtain walls | aggregates of panels and mullions placed by their grids; a mullion or panel turned or tilted off the model's axes as the box along its own axes (RE-106) | every Snowdon panel and mullion relation Revit's; 1,019 turned or tilted mullions and panels with Revit's extents along their axes |
| Doors, windows, openings | the element's box, turned with the instance where it is turned (RE-87); its opening is as deep as the host wall | host is Revit's on 138 of 138 (Core Interior) and 192 of 194 (Snowdon; the other 2 in another wall Revit cuts, RE-85), and on Revit 2024 and 2025 the wall its native record names, Revit's on all 463 of the BIM4LCA office's as built (RE-245); on Revit 2024 and 2025 the opening is the outline the symbol that draws it cuts, through the instance's transform: 138 of 138 on Core Interior and 4 of 4 on RE1 within 0.001 ft of Revit's (RE-215, #227; Measure run 40927543524); a window whose type stores Width, Height and Default Sill Height takes Revit's opening exactly (56 of 68 on Snowdon, RE-93), and a door whose body is its type's Rough Height tall its rough opening (57 of 126, RE-94); any other door whose type stores Width and Height carries them as its overall size (137 of 137 on Core Interior and RE1, RE-187; Measure run 40927543524); on Revit 2024 and 2025 a door's or window's overall size is the outline the symbol that draws it cuts, Revit's on every one with such an outline (138 of 138 on Core Interior, 4 of 5 on RE1, RE-215); a door or window whose symbol cuts no outline takes its own extents as its overall size, Revit's on all 269 windows and 266 of 269 doors of the BIM4LCA office (RE-218); otherwise not Revit's opening profile (#227, RE-83) |
| Rooms | `IfcSpace` with number, name and the outline of the room's stored solid (RE-101) | Revit's own area and outline on all 116 Core Interior and 11 RE1 Architecture rooms (RE-101's run; neither CI nor Measure scores room outlines); 45 of 54 Snowdon Towers rooms at the area its VIM export gives, the other 7 whose solids do not close keeping their box (#90); on Revit 2023, all 9 of Exemplo_data's (RE-102) |
| Areas | `IfcSpace` with number, name and storey; no body | all 30 of Revit's area spaces on the BIM4LCA office, as built and at building permit (RE-191); Revit 2024 only |
| Furniture, fixtures, equipment, MEP devices, site | the element's bounding box, turned with the instance where it is turned (RE-87) | placement and extent only; no family geometry. MEP devices and equipment are measured on RE1 Electrical and Mechanical only (RE-79) |
| Family instances from their saved meshes (doors and windows by default on Revit 2023 to 2027, every other family with `--saved-meshes`) | the mesh Revit saved for the element's display | within 0.01 ft of Revit's export: Core Interior 158 of 158, RE1 (2025) 207 of 214, Snowdon Towers 3,332 of 3,736. Revit's own shape, every vertex within 0.001 ft: Core Interior 138 of 138 doors and windows, the BIM4LCA office 1,294 of 1,709 scored (all its windows; 48 of 269 doors), RE1 148 of 214; a mesh that is not exact is part of the body (RE-78). Since RE-224 a mesh some face of which did not decode keeps its box: on the office 1,294 of the 1,352 bodies then drawn are exact (1,376 of 1,438 at permit) |

## Properties on Revit 2024 and 2025 projects

Read: ElementId, category, `Family:Type:ElementId` name, type name, storey,
material names and layer thicknesses, family instances' materials (RE-82), the IFC type
object of each element's type (RE-110), Revit's GlobalId (other entities get one
derived from the document and their own identity, stable between exports,
#400), stair riser and tread dimensions, each flight's
number of treads and nosing length (RE-92, RE-236), a stair's `TreadLengthAtInnerSide` and, with `--revit-set-placement`, its `Pset_StairFlightCommon` summed or shared from its flights (RE-287), on Revit 2024 its run type as its `Reference`
and each landing's type as its `Reference`, each element's `IsExternal` as Revit's
exporter sets it (a parameter named for its property set, its type's
Function, else its class's rule; on curtain mullions and panels their curtain wall's, RE-193, RE-288; on Revit 2015 to 2023 a wall's, from its type's Function or a parameter named for it, experimentally, RE-283),
each curtain mullion's `Span`, its length along its own axis (RE-226), which is also its `Qto_MemberBaseQuantities.Length` (#1016), and its `NetVolume`, its section times that length, where Revit's export draws it as an extrusion of its section: a mullion whose type has its own rectangular or square section, told by the class of its type's section object in whatever language the model is in (RE-246), on a straight curtain wall whose profile is not edited (RE-243),
each door's and window's `Width`, `Height` and `Area` base quantities, its type's size where the symbol that draws it has the same (#1016),
each curtain wall's type, named as its type's native record names it (RE-230),
each wall's `Qto_WallBaseQuantities.Width`, its type's thickness, also on a wall not drawn from its centreline (#1016), its `Length` where its curve cut back where its body meets the walls joined at its ends is as long as the curve, so either length Revit's export may write is that one (RE-252), and its `Height`, its body's depth, where its native record names no roof or floor joined to it, no element cutting it and no instance's cutout (RE-266), and its `GrossSideArea`, `NetSideArea`, `GrossVolume` and `NetVolume` where no step of the record cuts, embeds or sweeps it and its body is the record's box cut back at its joins, a rectangle, or it is joined to roofs or floors that lie clear of it and its record confirms its length (RE-270), and, on a wall no roof, floor or other element joins or cuts, its `GrossFootprintArea`, its length times its width, where it is more than three times as long as thick (RE-292), each beam's `Qto_BeamBaseQuantities.Length` where it is drawn along its location line (RE-49), its line's length, each curtain wall's `Qto_CurtainWallQuantities.Length`, its curve's (#1016), and the `Qto_OpeningElementBaseQuantities.Depth` of each opening a door or window whose symbol cuts an outline makes in it, that thickness (RE-244); the openings' `Width` and `Height`, which Revit's export swaps, are left out (RE-235 §5),
each pipe's `GrossCrossSectionArea` and `OuterSurfaceArea` from its cylinder, each stair flight's `Length`, the span of its riser lines, and `NetVolume`, its side views' area times its width, and the `Length` of each column whose record holds no `CleanPairGStep` join and, where such a column is drawn as its type's hollow section, its `CrossSectionArea`, `OuterSurfaceArea` and `GrossVolume` (RE-247 §8), which Revit's export draws as an extrusion (RE-247); a joined column's `Length` and a curtain panel's `Width` are left out,
each floor's `Qto_SlabBaseQuantities.GrossArea` where it is drawn as one extrusion, its profile's area less its holes (RE-249), and, where its type has one layer as deep as that extrusion, its `Depth` and `GrossVolume` (#1016), its `NetVolume`, that area times its type's layers' total thickness, where its native record holds no step that cuts it, and its `Width`, that thickness, where the type has more than one layer (RE-295), and each ceiling's `Qto_CoveringBaseQuantities.Width` where its body's depth is its layers' thickness, on a ceiling of several pieces too (RE-250, RE-297), its `GrossArea`, the area of the sketch's outline less its holes, where the ceiling is drawn as one extrusion of it and its native record holds no join that cuts it (RE-297), and the `Width` of each wall Revit's export writes as a covering, its type's thickness (RE-297), each space's `Qto_SpaceBaseQuantities.Height`, its room's, where the room's record holds its solid's faces (RE-255), its `NetVolume`, its outline's area less its holes times the height of its box, where every vertical face of that solid runs from the floor to the ceiling and no face is slanted, on Revit 2024 and 2025 (RE-296), each stair railing's `Qto_RailingBaseQuantities.Length` where its path is one line along its stair's runs (RE-254),
each of these base quantities left out by `rvt-ifc --no-base-quantities`, as Revit's exporter leaves them out with "Export base quantities" off; Revit's built-in IFC4 setups turn it on, so rvt-ifc does by default (RE-242),
the type of each pipe, wall sweep and ramp that neither its stored layout nor its reference list types, the one its own native record names (RE-240),
each wall's `ExtendToStructure`, whether its top is joined to a roof or floor (RE-228; on Revit 2015 to 2023 too, experimentally, RE-284),
each railing's `Height`, its type's top rail height (RE-231),
a slab's `PitchAngle` 0 where its top is known to be level (RE-238),
the project and shared parameters named for a property of one of IFC4's
property sets (`Pset_WallCommon.FireRating`, or `FireExit` alone where no
qualified one gives it, RE-227), from buildingSMART's
definitions of the 278 sets that apply to an entity, written in that set
where IFC4 applies it to the entity written or one of its supertypes, with
the predefined type a scoped set needs (booleans, text, counts and integers,
lengths and thermal transmittance, RE-194, #817), and Revit's "Export to IFC As"
override, applied where it names `IfcShadingDevice`, `IfcSlab`, `IfcCovering`,
`IfcWasteTerminal`, `IfcFireSuppressionTerminal`, `IfcSanitaryTerminal`,
`IfcAirTerminal`, `IfcElectricAppliance` (RE-190), `IfcStair` or `IfcDoor` (#792)
and Revit's exporter acts on it (not on a 2024 file's curtain walls, RE-188, or its wall sweeps, RE-192),
with the values not applied listed in the diagnostics. With `--revit-property-sets`, each element's
named saved parameters in Revit's own property sets, one per parameter group (RE-210; the set
names, grouping and values measured on Autodesk's Snowdon Towers HVAC, Site and Architectural). With
`--revit-set-placement`, also the properties Revit's own exporter writes where IFC4's definitions do not:
a text parameter named `<set>.<property>` for a property the set's IFC4 definition lacks
(`Pset_SpaceCommon.FloorCovering`), and the `Reference` of a set whose entity's name begins with the
element's (`Pset_SpaceHeaterTypeCommon` on a space); without it every property is in the set IFC4 gives
(RE-235). A value Revit's export writes that is known to be wrong is left out either way: a stair's or
flight's `WalkingLineOffset`, half the run's width in feet read as millimetres, a curtain mullion's `Qto_MemberBaseQuantities.CrossSectionArea`, which grows with its length and is never its section's area, and a curtain mullion's or panel's `IsExternal` in the office's exports and its mullion type's, which the exporter that made them sets from its own cache of element types, not from the model (RE-235 §5, RE-269; rvt-rs writes the member's by its curtain wall's value, RE-288). Not read: most instance and type parameters (#35, #155). Elements in a
design-option set's non-primary options are left out, as Revit leaves them out,
and so are elements that do not exist in the project's last phase (RE-184; the order is read on Revit 2015 to 2027, RE-281),
except rooms: Revit skips a room by its own phase parameter, which is not
measured to be its created phase, so a room created in an earlier phase is
still written.

## What approximations look like in the output

- A body that is the element's bounding box is a correctly placed, correctly
  sized box. It is never a guess at a shape.
- A file of a release read experimentally (Revit 2015 to 2022, 2026 and
  2027) says so: the export diagnostics' `input.release_support`, a warning,
  `rvt-inspect`'s `experimental_release` and the viewer's file status (#871).
- Where rvt-rs cannot tell something, it leaves it out rather than guess: an
  element whose storey is unknown sits in the building, not on a storey; a
  missing value is absent, not zero.
- `rvt-ifc --diagnostics out.json` and the viewer's diagnostics download list
  what was exported, approximated and skipped, and why, including every
  stream a resource limit kept from being read (`stream_not_read_resource_limit`). `--mode
  typed-no-geometry`, `geometry` and `strict` refuse a file that does not meet
  that bar instead of writing a thinner IFC.

## Not supported

- Converting an arbitrary Revit model to IFC with Revit-grade fidelity.
- Element records of releases other than 2008 to 2027. Of those, 2008 to 2022, 2026 and 2027 are experimental: 2008 to 2014, outside the 1.0 range, with 2008 to 2010 checked only for consistency within each local file (RE-260) and 2011 to 2014 measured on one local 2013 sample against its 2015 copy (RE-259); 2026 is measured on one project against Revit's export (RE-124), and since 0.6.0 2027 against the 2026 copies of Autodesk's sample projects (RE-179), 2019 to 2022 against their 2023 copies (RE-180), and 2015 to 2018 against their 2019 copies (RE-181), none with a Revit export of its own release except one 2015 project, BIMcollab's structural example (RE-198). On 2023 (since 0.4.0): geometry beyond each element's box, wall joins and beam cuts, layered or angled joins, design options and IFC export overrides.
- Semantic editing of a Revit file. `rvt-write` patches whole streams and
  preserves the rest byte for byte; it does not change model data.
- Door/Window typing from the opening-index rows (RE-19) and Level ElementIds
  from the Formats schema (RE-20): both closed negative. Element records supply
  both instead.

Use `rvt-inspect <file> --json` or the viewer's diagnostics download when a
file falls outside this profile. Those reports can be attached to a GitHub
issue without sharing the model; [`CONTRIBUTING.md`](../CONTRIBUTING.md)
describes how to send a diagnostic instead of a building file.
