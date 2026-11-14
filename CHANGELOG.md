# Changelog

All notable changes will be documented here. This project follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
[semver](https://semver.org/spec/v2.0.0.html). Changes on `main` since the
last release are also in [`changelog.d/`](changelog.d/README.md), as
fragments a release moves into this file.

## [Unreleased]

## [0.23.0] — 2026-11-14

0.23.0 writes more of what Revit's own export writes, and narrows rules that audits found wider than their evidence. A wall of a curtain wall type is an `IfcCurtainWall` on Revit 2023 to 2027 whether or not a mullion names it, and a curtain wall that is the host panel of another is not one. A roof, stair, stair flight or building element proxy with no predefined type of its own gets Revit's `NOTDEFINED`, a sloped floor gets its `Qto_SlabBaseQuantities.Depth` and `GrossVolume`, a door's or window's `FireRating` is read from its saved parameter, and a column whose saved mesh is a prism of another section than its type's rectangle is drawn from the mesh. The doors and windows of Revit 2015 to 2023 cut the outline their symbol cuts and carry its `OverallWidth` and `OverallHeight`. A material's colour reaches IFC viewers. A file that stores no project GUID gets the project, building and site GlobalIds Revit's export gives them, from the GUID in its history header. `rvt-diff --elements` counts a change in the rule that read a value as a label change, not as a changed element.

Not measured, and said in each entry:
- A horizontal bracing's `IfcMemberType` carrying `BRACE` on Revit 2027 (#1841) is checked on a reporter's counts of a structural model alone: no model Measure holds with a Revit export has a horizontal bracing.
- `NOTDEFINED` on stairs and flights is measured on Revit 2024 only, the one release whose export has such stairs (#1713).
- A multistory stair is written once, where Revit's export writes it once per storey: Snowdon Towers' three lack 53 entities (#1977, RE-463).
- No Revit export holds a curtain wall without a mullion, so what Revit writes for `rme_basic`'s two rests on the source of its exporter (#1960).

Measured against 0.22.0 on every model Measure holds (run [43555120928](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43555120928), `main` at 21d310f0; the release adds only its notes, witness files, version strings, fixtures and the publish workflow's viewer smoke to them, and #1991, a probe, a scorer and a report, landed on `main` after it):

- All 146 exports differ, and 120 of the 121 `diagnostics.json` files (the
  blank Revit 2025 model's is equal). 79 of the 146 have no Revit export:
  Autodesk's 59 sample projects (34 of 2015 to 2023, 25 of 2024 to 2027), 19
  families and Einhoven, so no scorer says which of what moved is Revit's: each
  entry above was measured in its own pull request. The other 67 are scored
  against Revit's own exports: Core Interior, the four RE1 models, BIMcollab's
  example, the 12 BIM4LCA models, the 24 Revit 2025 models, the six
  `rac_advanced` saves of 2015 to 2020, 18 held-out files and the spent Vitoria
  "Before". On them no count of values or quantities that equals Revit's falls
  and none that differs rises, what rvt-rs writes that Revit's export does not
  rises on four files (the last two lines below), BIMcollab's example moves in
  no scorer line, and every other line that moves is one of these:
  - The project's, building's and site's GlobalId is Revit's on 50 models (0 of
    1 each before), and the storeys' on 11 files of 2017 to 2022 (69 storeys,
    from 6) (RE-459). On the spent Vitoria "Before", 1,297 of the 1,314
    elements Revit's export holds are Revit's (from none), its 73 rooms pair
    with Revit's by GlobalId (from none) and 414 of 1,005 type objects have
    Revit's GlobalId (from none).
  - `NOTDEFINED` (#1713): the roofs that agree with Revit's `PredefinedType` go
    from 8 to 39 of 39 on 30 models and the proxies from 34 to 456 of 464 on 22
    (the 8 left are the one proxy in each of the eight BIM4LCA files at permit
    and as built that Revit writes `USERDEFINED` on). The stairs of the two
    timber housing files go from 70 to 76 of 76 and the flights of the two
    timber offices from 58 to 62 of 62.
  - A door's or window's `FireRating` (#1507) is Revit's on 120 values (from
    none) on 14 models: the eight held-out files of 2024 to 2027 and six of the
    Revit 2025 models.
  - A wall's quantities with each opening at its real height:
    `Qto_WallBaseQuantities` equal to Revit's on all 10 walls of each of four
    held-out files of 2024 to 2027 (2 before) and all 9 of each of four
    held-out offices (3 before), on 4 of 5 walls of six Revit 2025 models (2
    before), and on 1 or 2 more walls of four BIM4LCA files.
  - The openings of Revit 2015 to 2023 doors and windows (RE-450): the
    opening's `Qto_OpeningElementBaseQuantities.Depth` is Revit's on 224 of 224
    openings of four held-out files (from none); on the spent Vitoria "Before"
    68 of 68 doors and 64 of 70 windows have Revit's `OverallWidth` and
    `OverallHeight` (from none; the other 6 windows get none) and the boxes of
    54 doors and 3 windows are within 0.001 ft of Revit's (within 0.26 ft for 5
    and within 1 ft for 52 before).
  - The mullions' `Pset_MemberCommon.Span` and `Qto_MemberBaseQuantities.Length`
    are Revit's on 6 more mullions of each concrete office and 8 more of each
    timber office (#1193); the sloped floors' `Qto_SlabBaseQuantities.Depth`
    and `GrossVolume` on 8 more floors of each office (#1208); and 4 more of
    the timber housing's columns are drawn from the mesh Revit saved, which
    matches Revit's on all 8 scored (#1710).
  - A wall profile region that a pocket equals is no longer cut into the wall's
    body: `wall_profile_cut_in_body` goes from 9 to 8 as built and from 10 to 8
    at permit on the timber offices (#1947), and one more wall of the timber
    office as built is drawn in layers (593, of which 592 are within 0.001 ft
    of Revit's, where 591 of 592 were).
  - Where Revit's setup wrote no base quantities, rvt-rs writes more of them,
    counted apart (RE-242): 24 more on Core Interior (3,859 to 3,883), 107 more
    on each `rac_advanced` save of 2015 to 2020 (3,469 to 3,576) and 37 more on
    one held-out file (127 to 164). On another held-out file, whose export holds
    no quantities on the objects rvt-rs writes, rvt-rs writes 389 more opening
    depths, which the scorer counts as quantities Revit does not write (522 to
    911), not apart.
  - One measured negative: the spent Vitoria "Before" gains one opening that
    nothing fills and that Revit's export does not hold (Revit's 5 unfilled
    openings in that file are other doors'), and a held-out Vitoria and a
    held-out Gdynia each gain one opening that Revit's export does not hold,
    with 2 more values that Revit does not write on each of the three (#1165).
    It is a door's cut in a second wall: reading the door cuts of 2015 to 2023
    (RE-450) takes the door cuts the spent file leaves out because the door's
    box was not read from 9 to 2, 6 of the 7 now read are left out as a wall
    parallel to the door's and 1 is written. No Revit export of 2015 to 2023
    holds such an opening, so the rule is to be limited to 2024 to 2027 (RE-353).
- What differs in `diagnostics.json` is counts: the wording of the storey
  warning (120 files), the skipped items (62), the parameter values (32), the
  exported entities (21), the bodies' sources (the saved mesh in 11 files, the
  partition record's box in 10), the walls and curtain walls exported (10), and
  the door openings a wall's cuts leave out because the door's box is not read
  (10).
- Time and peak memory, base and head on one runner (timing run
  [43558339212](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43558339212),
  `main` at 92af055c against 0.22.0, three rounds each): the 16 reference
  models and BIM4LCA files take a median 1.02 times the time (0.995 to 1.04) and
  1.00 times the peak memory (0.99 to 1.00); the 29 samples of 2023 to 2027,
  1.00 times the time (0.99 to 1.05) and 1.00 times the peak memory (0.99 to
  1.06). Five files are more than 3% slower: the two BIM4LCA offices as built
  (concrete 1.04, timber 1.04) and three samples (1.03 to 1.05). No peak is more
  than 2% higher but two Snowdon files', HVAC (1.06) and Plumbing (1.05), whose
  peak is one of two values from round to round and the higher one on every head
  round.

### Upgrading from 0.22

- **GlobalIds move once on files that store none.** The project's, the building's and the site's GlobalId of a Revit 2017 to 2027 file that stores none, and the storeys' of a file of 2017 to 2022, are Revit's now (#878, RE-459); a tool that tracks them sees the old ones removed and the new ones added. The type of a hosted door or window whose host frame or facing was not read no longer carries a GlobalId hashed with a guessed flip (#1947).
- **New entities and values.** Every coloured material has an `IfcMaterialDefinitionRepresentation` naming its surface style; roofs, stairs, flights and proxies carry `PredefinedType` `NOTDEFINED` where they carried none; the two single-cell curtain walls of `rme_basic` and `rme_advanced` (573660 and 573664) are `IfcCurtainWall` with their panels where they were `IfcWall`; the openings of doors and windows of Revit 2015 to 2023 have their symbol's outline, not their record box's.
- **`rvt-diff --elements` JSON** lists the elements whose only difference is a label (`relabelled`, each with its `labels`); a script that counted every difference as a change reads fewer.

### Added

- **A sloped floor drawn along the plane its shape-edited corners lie in gets
  its `Qto_SlabBaseQuantities.Depth` and `GrossVolume` on Revit 2024 (#1208, RE-298,
  RE-384).** Other releases keep neither until a file of theirs is
  measured. Revit's export writes the volume of the slab there, the type's
  layers' total thickness times the cosine of the slope for the `Depth` and
  that times the floor's `GrossArea` for the `GrossVolume`, on all 8 sloped
  floors of the BIM4LCA office as built, where rvt-rs wrote
  neither. A level floor of several layers keeps neither: Revit 2024's
  exporter writes its last layer's thickness and Revit 26.4's another value
  (RE-298).

### Changed

- **`rvt-diff --elements` compares resolved values, not the rules that read
  them (#1864).** An element whose `LevelBindSource` alone differs, or whose
  `BodySource` differs while its body is the same, is counted unchanged and
  listed apart as a label change (`relabelled` in the JSON, each with its
  `labels`). Two saves of a project whose beams bind the same Level by two
  rules no longer show those beams as changed. A body that differs is still a
  change, with its `BodySource` beside it.

- **The support matrix rates two capabilities for the proof they have, and
  one evidence line says what runs where (#1933).**
  `cfb-container-open` was `verified` on one reader, rvt-rs's own over the
  `cfb` crate, where `docs/verification-protocol.md` asks two independent
  witnesses agreeing across a recorded edge, gated in CI: it is `partial`,
  named for `.rvt` and `.rfa` (`.rte` and `.rft` are the same code and in no
  corpus), and its note says an encrypted file opens at the outer container
  with inner streams that read as nonsense, as `docs/compatibility.md` does.
  `record-elevation-storey-bind` was `verified` on an 11 of 11 elevation
  match measured once, before #218 made it the fallback that runs only where
  no Level records are recoverable, with no CI comparison on its path: it is
  `partial`. The evidence of `standalone-family-revolutions-2020` now says
  its export and refusal checks run only where two environment variables
  point at the pinned revolutions package, which CI sets for neither. No
  export changes.

- **CI reads every corpus file's container with a second reader, olefile, and
  holds rvt-rs's to it (#1938).** `tools/ci/olefile_streams.py` lists each
  `.rvt` and `.rfa` of the family corpus, the project corpus and
  `tests/fixtures/families/empty.rfa` with olefile, runs `rvt-dump --raw` on
  the same file and fails on any difference in the stream names, in each
  stream's stored size, or in the stored bytes. It prints how many files and
  streams it compared, and a directory with no Revit file or a run that
  compared no stream fails. `cfb-container-open` stays `partial`: a stream-list
  agreement checks the container and is not yet one of the two independent
  witnesses `docs/verification-protocol.md` asks for. No export changes.

- **Four rules that wrote values on releases their reports never measured now
  stop at the release they are measured on, and the reports of four more say
  what holds them (RE-461, #1945).** A hollow-section column's
  `CrossSectionArea`, `OuterSurfaceArea`, `GrossVolume` and `NetVolume`
  (RE-247) and a wall sweep's and a ramp's type from its own record (RE-240)
  are written on Revit 2024 only, the one release they are measured on. The
  wall that embeds a curtain wall is drawn from its saved mesh on 2024 to 2027
  (its records of 2023 name no embed step), and a shaft's cut takes only
  outlines drawn from sketch lines, as RE-99 measured. A beam's storey at its
  record box's top (RE-119), a pipe from its own record (RE-370), a door's
  origin from its stored transform (RE-361) and a railing's type from its
  payload (RE-66) keep their range: each is held to the project's 2024 save on
  the releases the corpora have a file of, and the doc comments and the report
  say which. In an unfiltered Measure of 146
  exports (run 43525326226) none of the exports and none of the 121
  diagnostics moves.

### Fixed

- **A curtain mullion turned off the model's axes is measured along its own
  axes down to a nearer-degenerate turn (#1193).** On the BIM4LCA office 6
  mullions of each concrete file and 8 of each timber file, shorter than
  their own section and turned some 45 degrees, get their `Span` and
  `Qto_MemberBaseQuantities.Length` from the box along their own axes, within
  0.0003 ft of Revit's; the 6 and 4 nearer 45 degrees keep RE-226's rule.

- **The wall top and base passes tell a roof by its Revit record, not by the
  IFC entity it is exported as (#1480).** A wall is cut at a roof only where
  the roof is a level plate (RE-351); a floor exported as `IfcRoof` now keeps
  its cut, and a roof exported as `IfcSlab` is held to the plate rule.

- **A door's or window's `FireRating` is read from its saved parameter
  captioned Fire Rating on Revit 2024 to 2027 (#1507).** Where its type's family
  type table gives no Fire Rating, the element's own value of that parameter,
  else its type's, is written as the `FireRating` of the standard sets that
  define one. puzzbobb's Revit 2025 window types hold their rating that way.

- **A column drawn as its type's rectangle is drawn from its saved mesh where
  that mesh is a vertical prism of another section (#1710).** The BIM4LCA
  housing's chamfered concrete columns and the thin-walled struts of Autodesk's
  `rme` samples were solid rectangles; their bodies and base quantities now come
  from the mesh Revit saved. A column whose mesh section is its rectangle keeps
  the rectangle, Revit's own form for a plain section.

- **A roof, stair, stair flight or building element proxy with no "IFC
  Predefined Type" of its own or its type's is written `NOTDEFINED`, as
  Revit's export writes it (#1713),** on the releases measured, where it was
  written with none: roofs on Revit 2024 to 2026, stairs and flights on 2024,
  proxies on 2024 to 2027. Furniture already took this default (RE-419).

- **On Revit 2027, a horizontal bracing's type object is an `IfcMemberType` with
  the `PredefinedType` `BRACE` (#1841),** as Revit's export writes it, where it
  was `NOTDEFINED`. The member's occurrence keeps no `PredefinedType`, as Revit
  2027's export leaves it off members.

- **A door's or window's type takes no guessed flip in its GlobalId (#1947).**
  Revit hashes a door's type GlobalId with the door's flip (RE-167), and
  RE-167 measures that a door with no host is not flipped. rvt-rs also took a
  hosted door whose host frame or facing was not read as not flipped. Such a
  door's type now gets the GlobalId the writer derives, as any type rvt-rs
  cannot match does. Found by the batch 9 export audit.

- **A face-hosted cut's opening keeps its place among its host's openings
  when an earlier one is not written (#1947).** Revit's export names each
  opening a door or window cuts in a host after the host and its place among
  that host's cutting instances, the highest ElementId first. rvt-rs counted
  only the openings it wrote, so a cut declined before one (no transform read,
  not an upright prism, a host drawn otherwise) shifted the name and GlobalId
  of every opening after it. Each cut now takes its number before any is
  declined. Found by the batch 9 export audit.

- **A wall profile region that a pocket equals stays that pocket (#1947).**
  A region of a wall's edited elevation profile that one of its pockets equals
  is written as that pocket's opening, with the wall left whole (RE-380). The
  pass that carries a region on to the wall's end (#1473) carried such a region
  too, after which it no longer equalled the pocket, so it was cut into the
  wall's body: the one case the profile rules say must not happen. A region a
  pocket equals is no longer carried. Found by the batch 9 export audit.

- **A room bound by its parameter block says so, every refused wall end is
  counted, and a material layer's name is its `Name` (#1947).** Found by the
  batch 9 export audit.
  - A room bound to the Level its own parameter block names (RE-29), where its
    record names none, carried `LevelBindSource`
    `partition_element_record_reference_list`; it now carries
    `partition_room_parameter_block`.
  - A wall with one end refused with face evidence and the other without
    counted only the first: `exported.wall_end_reads_unmarked` now counts the
    second, as `docs/export-diagnostics.md` says every refusal is counted.
  - `IFCMATERIALLAYER` wrote a layer's name in its sixth attribute
    (`Category`). IFC4 puts `Name` fourth, and the name now goes there. No
    layer is named today, so no export changes from it.

- **A column or beam drawn as its type's I or hollow section binds to a storey
  by its base elevation, as one drawn as its record box does (#1947).** The
  section replaces the record box's footprint and must fit it, and the element
  stays at the box's base. `record_base_elevation_feet` did not list the two
  section body sources, so such an element whose record names no single Level
  lost the base-elevation storey binding its record box had given it.
  Found by the batch 9 export audit.

- **A curved wall's chords stay within 0.0005 ft of its arc whatever its
  radius (#1947).** Each arc of a curved wall's outline was capped at 512
  chords, so a radius past about 100 ft over a half turn gave chords straying
  further inside the arc than the 0.0005 ft its doc and RE-75 promise
  (0.0047 ft at 1,000 ft). The cap is now 8,192 chords, which holds the
  tolerance up to a radius of about 27,000 ft. An arc that would need more is
  not drawn as an arc, and the wall keeps the body it would otherwise have.
  Found by the batch 9 export audit.

- **A wall's base is not cut at the top of a roof drawn as its record box
  (#1947).** The base pass (#1384) took any roof's top as a level a wall's base
  stops at. For a roof drawn as its record box (a roof by extrusion, or a slope
  its reader does not read) that is the box's top, not the roof's, which RE-351
  section 4 names as a flaw the top pass already avoids. The base pass now
  takes a roof only where the top pass does: a level plate, its box exactly its
  type's layers and its outline read. Found by the batch 9 export audit.

- **A wall of a curtain wall type is an `IfcCurtainWall` on Revit 2024 to 2027
  although no mullion or grid names it, and a curtain wall that is the host
  panel of another is not one on any release (#1960).** The 2024 reader made a
  wall a curtain wall only where a mullion named it or a grid was found, so a
  single-cell curtain wall with no mullion (walls 573660 and 573664 of
  Autodesk's `rme_basic` and `rme_advanced`) came out as an `IfcWall`, where
  the 2023 reader, which goes by the wall's type, writes an `IfcCurtainWall`
  with its panel. Revit's own exporter (revit-ifc, `CurtainSystemExporter`)
  makes a wall a curtain wall by the curtain grid it has, with no test of
  mullions or cells, so the 2024 reader goes by the type too. A curtain-type
  wall that is the host panel of another curtain wall's grid (its record holds
  a `HostPanelInfo`; Snowdon Towers has 23) is written by Revit only as parts
  of its owner, and neither reader makes it a curtain wall. No Revit export in
  the corpora holds a mullion-free curtain wall: that case rests on the
  exporter's source (RE-462).

- **A Revit 2017 to 2027 file that stores none gets the project, building
  and site GlobalIds Revit's export gives them, and a Revit 2017 to 2022
  file the storeys', from the project's own GUID; a history with an entry
  closed by another byte than `0x28` is read (#878, #1929, RE-459).**
  Revit's exporter makes the project's, the building's and the site's
  GlobalIds from one GUID with a fake ElementId (-16, -15, -14) XORed into
  its last 32 bits, and a storey's from it with the Level's ElementId before
  Revit 2023. RE-220 could not find that GUID, and files that stored none
  kept generated ids. It is in the file: the header of `Global/History`
  holds it four times, at `0x16`, `0x26`, `0x36` and `0x56`, and it is not
  an entry of the history.
  On the 67 models of Measure that have a Revit export, the four copies are
  equal on all of them (268 of 268), and the project, building and one site
  are Revit's on 63 of the 64 files of 2017 to 2027 whose export is of their
  own project; the storeys are Revit's on 69 of 69 on 11 files of 2017 to
  2022, and on none of 229 from 2023, where each Level's own episode gives
  them as before (Core Interior, 15 of 15). Where Revit's export is of
  another project than the file (two `rac_advanced` saves of 2015 and 2016,
  whose header holds another GUID than the saves that export came from)
  nothing is measured, and BIMcollab's Revit 2015 example stores its ids as
  strings, which win as they did. One held-out Revit 2018 project follows no
  rule found for its project-level ids: neither the header GUID, nor a
  stored string, nor an episode gives them, and they are not claimed. The
  rule is read only on Revit 2017 and later, because no file of 2015 or 2016
  can test it.
  Alongside it, `EpisodeHistory::parse` read none of the 1,314 elements of
  the Revit 2018 Vitoria "Before" model because four of its 1,127 entries
  are closed by `0x1e`, not `0x28`: it reads them now, and the elements of
  that model Revit's export holds are Revit's on 1,297 of 1,314 (from none),
  and 1,420 of 1,441 on the held-out Revit 2018 project, with the 73 rooms
  of each.
  Every project, building, site and storey GlobalId of a file the rule
  applies to changes once, in the release after 0.22.0: 50 of the 146
  exports of an unfiltered Measure get Revit's project, building and site
  where they had none, and 11 files get their storeys, with the diagnostics
  of all 121 equal and, on 22 exports kept from both builds, no line changed
  but the GlobalIds and the clock
  (`reports/element-framing/RE-459-episode-global-ids-2015-2023.md`).

- **`rvt-ifc-compare`'s notes on walls, doors, windows, slabs and openings
  say what rvt-rs now does.** They still cited closed issues (#23, #219,
  #238) and RE-23's host slot: walls are 360 of 360 world-exact on Core
  Interior since RE-127, doors and windows take the wall their reference
  list names (RE-85), and slabs and windows are storey-bound (RE-27). The
  open gap the door and opening notes name is #227's, the opening cut on
  releases other than 2024 and 2025. The table in `docs/ifc-compare.md`
  says the same.

- **The support matrix, the status pages and the viewer promise what their
  proof shows.** An audit of every capability in `docs/support-matrix.json`
  and of `README.md`, `docs/compatibility.md`, `ROADMAP.md`, `docs/status.md`
  and the viewer's support text against the tests, reports and code each names
  corrected over 70 statements, and changed no status. Among them: the viewer's
  legend gave 1.0's range as 2015 to 2027 (it is 2024 to 2027) and listed phase
  filtering as not ready (the phase field has it); the roadmap had the Python
  bindings writing glTF and plan SVG (they write IFC4 and CSV); notes still
  said railings, horizontal bracing, roofs by extrusion outside 2025, Areas,
  Floors and Rooms' storeys, connector SystemType and 2008 to 2014 element
  records were not handled, which later work does handle; and functions,
  tests and counts are named as the code and reports now have them. Four
  status questions the audit raised are on #1933.

- **The viewer's schedule names a layered element's material, the diagnostics
  count what `--revit-property-sets` adds, and a few messages say what the
  export does.** Found by the batch 9 export audit; no IFC value changes.
  - The viewer's schedule read only an element's single material, so a wall
    of layers showed none where the elements CSV names its layer set. Both
    now read one rule: profile set, then layer set, then single material,
    then constituents.
  - `rvt-ifc --revit-property-sets` added its sets after the export's counts
    were taken, so `exported.total_entities` missed them; the counts are taken
    again after it, as after every other step that changes the model.
  - With no Revit Level recovered, the warning said the IFC used a fallback
    storey. It has none, and its elements are contained in the building
    (RE-124); the warning says so.
  - The element panel printed a length that rounds to zero as `-0.000 ft`, as
    its placement rows never do.
  - A glTF export drops a material index past the model's materials, which
    would have made the file invalid, as the IFC writer already drops it.

- **Revit 2023's ArcWall pattern no longer writes a curtain wall's panels and
  grid lines as walls (RE-440).** On `Revit_IFC5_Einhoven` its 24 matches were
  15 curtain panels, 7 curtain grid lines and 2 records with no ElementId:
  they were written as 24 walls with a placeholder thickness, hid 15 of the
  curtain wall's panels and made three storeys from the grid's rows. A match
  is now kept only where its native record is a wall. Einhoven exports its
  one wall, its curtain wall as an `IfcCurtainWall` of 24 panels and its two
  windows on its one Level, `Level 1`, and four of its warnings are gone. A
  Revit 2023 wall whose type is a curtain wall type is an `IfcCurtainWall`
  named by that type, aggregating its panels; 2023 `rac_basic`'s 9 curtain
  walls take the name its 2024 and 2025 saves give them. A curtain wall whose
  panels carry its body is no longer reported as missing one.

- **A material's colour reaches IFC viewers.** rvt-rs wrote each coloured
  material's surface style as an `IfcStyledItem` with no `Item`, which IFC4
  allows only inside an `IfcStyledRepresentation` of an
  `IfcMaterialDefinitionRepresentation`; none was written, so no viewer could
  find the colour, and elements showed in its default grey. Each coloured
  material now has that representation, naming the material, so a viewer
  colours every element associated with it. The glTF export already carried
  the colours. Found by the batch 9 export audit.

- **A wall's quantities from its saved mesh set aside each opening at its
  real height.** Where a door's or window's opening starts above its
  placement (RE-93), the box `wall_mesh_quantities` sets aside for it was as
  tall as its start plus its top, so the faces of the wall above a window up
  to that height were taken for the opening's. The box is now the opening the
  writer cuts: from its start, as high as it is. Since #1741; found by the
  batch 9 export audit.

- **Revit 2015 to 2023 doors and windows cut the outline their symbol cuts
  (RE-450).** On those releases a door's or window's opening was its record
  box, frame and trim included, and its `OverallWidth` and `OverallHeight`
  were not written; the outline its symbol cuts and its own transform were
  read on 2024 to 2027 only. Both are now read on 2015 to 2023 too. On every
  save of Autodesk's `rac_basic` and `rac_advanced` from 2015 to 2023 each
  opening is the same project's 2024 save's in size, and in place but for 24
  `rac_advanced` windows 0.0075 m off it on the 2015 to 2020 saves, and its
  door or window has the 2024 save's `OverallWidth` and `OverallHeight`.

- **The export's storey warning counts each way an element reached its
  storey, and no longer describes a count of zero.** It said "N building
  element(s) reached their storey through the Level ElementId their own
  element record names, the rest through a measured elevation match" even
  where N was every element, as on all 970 of Core Interior's, and it
  counted rooms as building elements. It now says "all N element(s)" where
  one join placed them all, and otherwise names how many reached their
  storey by the Level ElementId, by a measured elevation match, by another
  rule and by none, leaving out the parts of an aggregate.

### Research

- **The beam-end probe cuts a beam at the section the export draws for each
  column its list names, and on the BIM4LCA models that is the column's box
  (RE-455, #1839).** A fourth kind beside `column`, `wall` and `beam` holds
  the beam at the column's own section (a rectangle, a circle or a closed
  outline, turned by the element's rotation and placed at its location),
  along the beam's centre line, where the column kind uses the record box. On
  the ten BIM4LCA models it reads the section of 1,307 of 1,441 columns and
  gives the column kind's bounds on every beam it cuts: Revit's length within
  1 mm on the one beam Revit shortens and it reaches, and a cut where Revit
  leaves the beam on 2 beams, as the box does. The columns there are boxes, so
  that is a check that the kind does no harm, not a test of it; the test is a
  model whose boxes run past Revit's beams, such as the reporter's. Nothing
  ships.

## [0.22.0] — 2026-11-13

0.22.0 limits what a crafted Revit file can make the MCP server, the command-line tools, the Python bindings and the viewer do, and writes more elements the way Revit's own export writes them. The MCP server reads from and writes to one folder and bounds what an answer carries (#1883). `rvt-doc --redact`, `rvt-info -f csv`, `rvt-dump`, `rvt-ifc`, `rvt-write`, the Python bindings and the viewer each lose a way a hostile file reached a terminal, a spreadsheet, a log or a file they should not touch (#1884, #1885, #1887, #1919), and a wall profile of thousands of regions no longer builds a chain of boolean cuts too deep for a thread to walk (#1845). On the element side, roofs by extrusion of Revit 2024, 2026 and 2027, floors of the Structural Foundations category, horizontal bracing, the storeys of beams and braces, the corner mullions and the door of a curtain wall on Revit 2027, a lone non-ASCII name on Revit 2023, and the doors, windows and walls of Revit 2015 to 2023 are read as Revit's export writes them (#1479, #1841, #1842, #1865, #1899, #1912, RE-448, RE-456). puzzbobb wrote the last two: doors and windows of Revit 2015 to 2023 are set in the wall their own record names, and those releases' walls get the openings their rectangular openings cut.

Not measured, and said in each entry:
- A foundation slab and a horizontal bracing (#1841) are checked on a reporter's counts of a Revit 2027 structural model alone: no model Measure holds with a Revit export has either, and the bracing member's `PredefinedType` is not read.
- A roof by extrusion of Revit 2024, 2026 and 2027 is held to the same project's 2025 save, which cannot catch an error both saves share; no Revit export of one exists here (#1479).
- On the 2027 save of `rac_advanced` 5 corner mullions, whose wall nothing in the file names, stand in no curtain wall (#1865).
- The peak memory of concurrent `rvt-rs-mcp` opens, a crafted file with a very long name, and the memory `rvt-dump` saves by releasing each partition it has written are not measured (#1883, #1885).

Measured against 0.21.0 on every model Measure holds (run [43491038743](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43491038743), `main` at 2b815423; the release adds only its notes, witness files, version strings and fixtures to them):

- 72 of the 146 exports differ and 74 are identical, and 37 of the 121
  `diagnostics.json` files differ and 84 are equal. Of the 72, 56 are Autodesk's
  sample projects (34 of 2015 to 2023, 22 of 2024 to 2027), which no Revit
  export covers, so no scorer says which of what moved is Revit's: each entry
  above was measured in its own pull request. The other 16 are scored against
  Revit's own exports: the four BIM4LCA offices, the six `rac_advanced` saves
  of 2015 to 2020, BIMcollab's example, four held-out files and the spent
  Vitoria "Before". No scorer line moves on any other model, and every line that
  moves is one of these:
  - The 268 doors that Revit's export aggregates into a curtain wall on the four
    offices (66, 68, 67 and 67) and 0.21.0 did not are aggregated now, and two of
    the four members that each timber office left out (RE-452, RE-453, RE-449).
  - The beams of the two timber offices are on Revit's storey: 67 rows on another
    storey in each become 4, all ramps, with no beam left (RE-454).
  - On the spent Vitoria "Before", 3 more doors are set in the wall Revit's
    export puts them in, 133 of 133 where 130 were (RE-448).
  - The wall openings of Revit 2015 to 2023 (RE-456) add one opening to each
    `rac_advanced` save of 2015 to 2020, 6 to one held-out file and 18 to
    BIMcollab's example. Each carries the `Reference` values every opening
    carries and Revit's export does not write on them, so the values rvt-rs
    writes that Revit's export does not rise by 2 on each of those saves
    (342 to 344), by 12 on the held-out file (8 to 20) and by 36 on BIMcollab's
    (522 to 558, IFC2X3, counted apart).
  - The check of bodies across saves passes on the 2026 and 2027 saves of
    `rac_basic`, which failed it at 0.21.0 on one roof each: the roof by
    extrusion is drawn as the 2025 save draws it (RE-458).
  - The schema check of 23 samples counts what the entries add: one opening on
    each `rac_advanced` save of 2015 to 2023 and two on each `rac_basic`
    (RE-456), and a foundation floor as `IfcSlab` on the 2024 to 2027
    `rac_basic` and four on Snowdon Towers Structural (RE-457).
- What differs in `diagnostics.json` is counts: the parameter values (28 files),
  the exported entities (23), the openings a wall's cuts leave out, by reason
  (24, 10 and 1 files), the storey-bound elements and the beams skipped (5
  each), the `IfcFooting` and `IfcSlab` counts (5) and the roofs' body sources
  (3).

### Upgrading from 0.21

- **`rvt-rs-mcp` reads and writes inside one folder.** It is `--root <folder>`, else `RVT_MCP_ROOT`, else the folder the server starts in unless that is `/`, the home folder or one above it (then each tool that touches a file refuses and names `--root`). A setup that opens files outside the folder its client starts the server in passes `--root`, or `--unconfined` for the old reach. `list_rooms` pages as `list_elements` does and answers `total`, `offset`, `limit` and `returned`; a `limit` outside 1 to 1000 is a tool error; a name or value from a file is cut at 1,000 characters; an answer over 4 MiB is refused with a message that says how to ask for less.
- **`rvt-ifc`, `rvt-gltf`, `rvt-sheet` and `rvt-schedule` refuse to write over their input**, however it is named, and `rvt-info -f csv` leads a cell that starts with `=`, `+`, `-` or `@` with `'`. `rvt-dump` has `--max-stream-bytes`, `--max-inflate-bytes` and `--large-model`, lists a stream a limit left unread, and exits 3 when it did (`--allow-partial` makes it 0). `rvt-write --dry-run` checks the patches, and a patch that frames a stream otherwise than it is stored is refused.
- **The Python bindings** raise `RuntimeError` for a second call on one `RevitFile` from another thread while one runs (open one handle for each thread or hold a lock), `RuntimeWarning` for an element whose saved mesh decodes in part, and `ImportError` for a failed load of the extension.
- **Exports change** where the entries below say: a roof by extrusion of Revit 2024, 2026 and 2027, a floor of the Structural Foundations category (`IfcSlab`, `BASESLAB`) and a horizontal bracing (`IfcMember`), the storey of a beam or brace, curtain mullions, doors and windows of curtain walls, the host wall and the openings of doors, windows and walls of Revit 2015 to 2023, and the type of two furred ceilings of Revit 2023.
- **Library:** new `rvt::redact::redact_json_strings`, `rvt::cli::csv_text`, `rvt::writer::validate_patches`, `rvt::ifc::gltf::write_glb_to` and a public `RevitFile::release_inflated_partitions`; `rvt::writer::write_with_patches` returns `Error::WriteRefused` for a patch that frames a stream otherwise than it is stored, before it writes anything.

### Changed

- **`rvt-rs-mcp` reads models from, and writes exports to, one folder that its
  setup picks; a path that leaves it is refused (#1883).** The names, values
  and messages in a Revit file reach the client's model unmarked, so a crafted
  file could steer it into asking the server to open a file outside the
  project, or to overwrite one; until now the server read any file the account
  could read and wrote anywhere but inside `--output-dir`. The folder is
  `--root <folder>`, else the environment variable `RVT_MCP_ROOT` (for a
  client that passes only an environment), else the folder the server was
  started in, unless that is the top of a file system, the home folder or a
  folder that holds it (an MCP client chooses a server's working directory, and
  some start theirs from `/`): then the server starts and each tool that
  touches a file refuses with a message that names `--root`. A relative path
  is taken from the folder; a path is compared with it once `..` and links are
  resolved, and one whose text alone puts it outside is refused before any file
  is looked at, so the refusal does not say whether the file exists. The tool
  descriptions and the refusals say where the folder is and how to move it.
  `--root /` and `--unconfined` lift the limit, and `--output-dir` still sends
  the exports to a folder of their own. **Upgrade:** a setup that opens files
  outside the folder its client starts the server in now passes `--root` (or
  sets `RVT_MCP_ROOT`), or `--unconfined` for the old reach (`docs/mcp.md`,
  "The root folder"). Not defended: another process that rewrites the links
  inside the folder while a call runs.

### Fixed

- **A roof by extrusion is a solid on Revit 2024, 2026 and 2027 as on 2025,
  held to the same project's 2025 save (#1479, #1864, RE-458).** The release
  list of the B-spline reader held Revit 2025 alone, so such a roof was its
  saved mesh on 2024, 2026 and 2027 (its record box before #1191). Autodesk's
  `rac_basic` has one, roof 243274: its 2024, 2026 and 2027 saves now draw it
  as the 2025 save does (35.424 m3, where its mesh was 34.974 m3 on 2026 and
  2027), and are the only 3 of 146 exports of an
  unfiltered Measure that change; every body of the project's 2026 and 2027
  saves is the 2025 save's. No Revit export of such a roof of those releases
  exists here, so the check is the 2025 save. `rac_advanced`'s roof 201262,
  which #1479 took for a second one, is not a roof by extrusion and is
  unchanged.

- **A floor of the Structural Foundations category is an `IfcSlab` with
  `BASESLAB`, and a horizontal bracing an `IfcMember` (#1841, RE-457).**
  `StructuralFoundation` was mapped to `IfcFooting` and `StructuralFraming`
  to `IfcBeam` whole. On a reporter's Revit 2027 structural model, Revit's
  export writes the 59 foundation slabs as `IfcSlab` (`BASESLAB` on their
  types) and the one framing instance with stored structural usage 8
  (Horizontal Bracing) as `IfcMember`, where the 914 with usage 3 and 6 are
  `IfcBeam`. A floor of the category, which its record names as the class
  `Floor`, is now written as the slab, and a framing instance of usage 8 as
  the member; family-instance and wall footings and every other usage are as
  before. No model Measure holds with a Revit export has a foundation or a
  bracing, so the change is checked on the reporter's counts after the
  release; the member's `PredefinedType` is not read yet.

- **A beam or brace is in the storey of its Reference Level, which its record's
  second reference list holds (#1842, #1864, RE-454).** The storey was bound from
  the record's first reference list alone, so on each BIM4LCA timber office file
  63 beams were in a storey other than Revit's export puts them in: 47 that name
  two Levels in the first list, where the lower one was taken and Revit's storey
  is the highest, and 16 whose first list names a Level below the one Revit
  uses. A framing record now takes its Level from the second list where that
  names any, else the first, the one Level it names or the highest of several,
  as the `Reference Level` property already did (RE-174, RE-175). On both office
  files the 63 are in Revit's storey now and no other element changes storey.
  The Level is the same on the 2024 and the 2025 to 2027 saves of Autodesk's
  `rac_advanced` and `rst_basic`, where the first list's changes for 10
  `rst_basic` beams; 455 beams of Snowdon Towers Structural that were on no
  storey are on one; and a Revit 2015 to 2023 save takes its 2024 save's Level
  on more beams than before and on none fewer. Other classes, and records that
  name three or more Levels, are as before.

- **A curtain part that names two curtain walls and no grid takes the wall the
  element data of its list names, so 74 of the 79 corner mullions of Autodesk's
  `rac_advanced` on Revit 2027 are aggregated again and all 79 carry
  `IsExternal` (#1865, RE-449).** The 2025 and 2026 saves name one curtain
  wall in a corner mullion's reference list, and RE-46's rule aggregates it
  there; the 2027 save names both walls of the corner, which no arm of the
  match took, so the 79 stood alone with no whole and no `IsExternal`. Neither
  the order of the list nor the part's record box tells the two walls apart,
  but the element data of another id of the list (the part's curtain grid)
  names exactly one of them, at offsets that move between grid kinds and
  releases and are RE-72's on neither save, and that wall is the one that held
  the part on 2025 for 75 of 75 parts that have such an id on each save. The
  rule reads that data without fixed offsets, and it applies only to a part
  with no grid whose list names two or more curtain walls. A part for which no
  one wall comes out, the 5 mullions whose lists hold no such id, stands alone
  and takes the `IsExternal` its walls share where they agree (all 34 walls of
  the file are true). Measured on `rac_advanced`'s 2025 and 2026 saves against
  the 2027 one: the 34 curtain walls hold 4,458 parts, from 4,384, 29 of them
  as many as on 2025; `IsExternal` lost between the saves goes from 79 to
  none, and the 2026 to 2027 pair differs in 425 fields, from 573. The rule
  reaches every file with such a part, not the 2027 sample alone: 14 other
  exports of the 146 on Measure's unfiltered run change, each by `IsExternal`
  on curtain mullions the same way (9 to 79 in the 2015 to 2023 saves of
  `rac_advanced`, 2 to 4 in the four BIM4LCA offices), values that are not
  measured and not equal: Revit's export writes them as an exporter artefact,
  the census counts them apart (RE-269) and it does not move on any model.
  Two members of the BIM4LCA timber office are aggregated into the curtain
  walls Revit's own export names (`IfcMember` same whole 7,872 to 7,874). The `rac_advanced` saves of 2015 to 2022 now
  write the same 19,979 parameter values and those of 2024 to 2027 the same
  21,035; the 2023 save was six short until RE-451 (#1899). Not changed: the 5
  mullions, whose wall nothing in the file names (the curtain door 216844 is
  the next entry's); no Revit export of any of these saves exists, so the 2027
  save is held to the same project's earlier saves.

- **A door or window with no curtain grid found whose reference list names
  exactly one curtain wall is a part of that wall when the element data of an id
  in its list names that wall too, as a mullion's or a panel's grid does
  (#1865, RE-452).** Doors and windows took a curtain wall only through the
  curtain grid their record names, whose offsets (RE-72) are not found on
  Revit 2027, so the curtain door 216844 of Autodesk's `rac_advanced` was
  aggregated into the curtain wall 162023 on the 2025 and 2026 saves and stood
  alone, contained in the storey, on the 2027 save, and lost the `IsExternal`
  its wall gives; it is a part of 162023 on 2027 now. The data fallback RE-449
  added for mullions never reached openings. Naming the wall alone is not enough
  for an opening: it put one door of a Revit 2018 model in a curtain wall where
  Revit's export leaves it in an ordinary one. Against Revit's export of the four
  BIM4LCA offices, 184 of the 268 doors it aggregates into a curtain wall and
  rvt-rs did not are now in the same wall, none in another, and 21 in each
  office were still left out (RE-453, below, takes them); 21 of 146 exports
  change (the offices, the `rac_advanced`, `rac_basic`, `rme_advanced` and
  `rme_basic` samples of 2024 to 2027, and Snowdon Towers Architectural), none
  of Revit 2015 to 2023.

- **The Python bindings warn when an element's saved mesh decodes only in
  part, release the GIL for the Rust work, and say what they return and why
  they fail (#1884).** A saved mesh that decodes in part leaves its element
  with the record's box, counted in the diagnostics; the docs promised a
  `RuntimeWarning` for it and none was raised, and it is now (the note
  `rvt-ifc` prints for the step, when an element keeps its box). `write_ifc`,
  `rvt_to_ifc`, `rvt_to_ifc_diagnostics`, `schedule_csv`,
  `export_diagnostics_json`, `export_diagnostics`, `decoded_elements`,
  `element_counts` and opening a file held the GIL for the whole decode and
  now release it, so another Python thread runs meanwhile. **A handle is used
  by one thread at a time:** a second call on one `RevitFile` from another
  thread while one runs now raises `RuntimeError`, where it used to wait for
  the GIL and then run, so a program that shares a handle between threads opens
  one for each thread or holds a lock. The stub's `TypedField` lists the eleven kinds the
  binding returns and the `float` values of one of them, where it listed four
  kinds. A failed load of the extension (a missing symbol, a wrong ABI) is the
  `ImportError` it is, where `rvt/__init__.py` caught it and it surfaced as
  `module 'rvt' has no attribute 'RevitFile'`. The name of an unread stream is
  quoted in the sentence that reports it, in `rvt-ifc`, `rvt-gltf`,
  `rvt-rs-mcp`, the diagnostics and Python alike, so a stream named with a
  newline cannot forge a line in a log. No export moves.

- **`rvt-dump` takes the resource-limit options and goes on past a stream it
  cannot read, `rvt-info --extract-preview` refuses a preview that is not a
  whole PNG, and `gen-fixture` refuses a name with a path in it (#1885).**
  `rvt-dump` opened every file under the default limits with no option to
  change them, and a stream over the 256 MiB default stopped the whole dump
  with an error. It now has `--max-stream-bytes`, `--max-inflate-bytes` and
  `--large-model` as `rvt-gltf` and `rvt-ifc` do; a stream over the limit is
  listed as not read, the rest of the file is dumped, and the streams a limit
  left unread or cut short are listed on stderr with the way to read them, with
  exit status 3 (`--allow-partial` makes it 0). It also drops each inflated
  partition from the reader's memo once it has been written, where the memo
  held every one (up to 16 GiB) to the end of the dump; the memory this
  saves is not measured. `rvt-info --extract-preview` wrote what the stream
  held from the PNG signature on, and said "preview PNG written", when a
  preview cut short had no IEND chunk and no viewer opens it; it now writes
  nothing, exits 1 and says the preview is cut short (`rvt-dump --raw` writes
  the stream as it is). `gen-fixture NAME` with no `--output` wrote
  `tests/fixtures/synthetic-NAME.rvt`, so a NAME with a `/` or `\` in it
  wrote elsewhere; it is refused, and `--output` names another place.
  `RevitFile::release_inflated_partitions` is now public.

- **`rvt-ifc` refuses to write over its input, and `rvt-gltf`, `rvt-sheet`
  and `rvt-schedule` refuse however the input is named (#1885).**
  `rvt-ifc model.rvt -o model.rvt` replaced the Revit file with the IFC, with
  no check. The other three compared the output path with the input path as
  typed, so `./model.rvt`, a symbolic link to it and (on Unix) a hard link got
  through. All four now ask whether the two are one file before they export,
  and say `refusing to overwrite the input`. `rvt-ifc --observation` also
  wrote its IFC straight to the output's name, so a killed export left a
  truncated `.ifc` there; it is written beside the output and renamed when
  whole, as the export without `--observation` has been.

- **`rvt-write --dry-run` checks what the write would refuse, and the write
  refuses a patch that frames a stream otherwise than it is stored (#1885).**
  The dry run its help describes was unreachable without `--patches`, and with
  one it exited 0 without looking at the streams. It now opens the source,
  reads the manifest if one is given and applies the writer's own refusals
  (`rvt::writer::validate_patches`, new): a stream the file does not have, a
  framing the stream is not stored in, and a patch that would span a stored
  page. A manifest alone chose a stream's framing on disk and the read-back
  used the same framing, so a `Formats/Latest` written as `Global/Latest` is
  framed passed its own verification; a patch framed `RawGzipFromZero` or
  `CustomPrefix8` is now refused unless the stream is stored that way
  (`Verbatim` is the caller's own framing and is not checked); the library's
  `rvt::writer::write_with_patches` returns `Error::WriteRefused` for it before
  it writes anything, where it wrote the file and verified it with the same
  wrong framing. A
  `new_decompressed_base64` that decodes to no bytes (`"="`, `""`) is refused
  where it emptied the stream; an empty `patches` array is a plain copy and
  exits 0, where it exited 1 with `0 stream(s) failed verification`; `--src`
  and `--dst` naming one file, spelled any way, leave a file with no patch as it
  is instead of rebuilding its container, and a copy that fails removes its
  temporary file.

- **The viewer bounds what a large or crafted file can make the page hold
  (#1887).** A second reader of `viewer/src/main.ts` found no injection and no
  network path for file data; what it found was growth a file controls, and
  each part is now bounded. The scene tree builds each group's rows (a
  storey's categories, a category's elements, an element's parts) 500 at a
  time: a model of up to 150,000 rows, which holds the largest Measure has
  (the BIM4LCA office, 91,723 entities), is built whole when it opens as it was
  before, and past that a group builds its first block when it is opened and a
  "Show more" row at its end builds the next. The filter looks at every element
  and lists the first 2,000 matches, and says when there are more; a click on
  an element in the 3-D view builds the row it needs. The phase field lists at
  most 200 phases (the first 199 and the last, the default) and says how many
  the file names; the page keeps at most 2,000 of the decoder's progress
  reports and of what the loading card said; and the saved parameters it
  remembers are bounded to 8 Mi characters of JSON as well as 64 elements, the
  answer just read always staying. The address (`?parametersRemembered=`) could
  set the cache's size without a bound; it can now only lower it. No export
  moves.

- **On Revit 2023 an element-data name of one non-ASCII character is no longer
  taken as a name, so the two furred ceilings of Autodesk's `rac_advanced` find
  their type (#1899, RE-451).** A list of one ElementId read as a name is one
  character, and the elements before the two ceilings (247616 and 247676, `퉻`
  and `퉼`) were candidates for their type beside its own, 242823 `Furred
  Ceiling`: neither was the more specific, so the 2023 save wrote the ceilings
  as `Ceiling-247617` and `Ceiling-247677` with no `Pset_CoveringCommon` and no
  `Pset_EnvironmentalImpactIndicators`, six parameter values short of every save
  from 2015 to 2022. Revit 2015 to 2022 already drop every name of one
  character (RE-180); 2023 keeps a one-letter ASCII name, such as `rac_basic`'s
  `p`, as before, because dropping it would give five walls a type their 2024
  copies do not confirm. A probe over the 40 models of Autodesk's 2015 to 2023
  samples and Einhoven finds the choice of a system type changes for these two
  elements only.

- **A door or window with no curtain grid found whose reference list names one
  wall in all, and that wall a curtain wall, is a part of that wall (#1912,
  RE-453).** RE-452 aggregated such an opening only when the element data of an
  id in its list named the wall too, which left out 84 of the 268 doors that
  Revit's export aggregates into a curtain wall on the four BIM4LCA offices (21
  in each): the element data of their lists never names the wall. Every door of
  the offices and of Autodesk's Snowdon Towers Architectural that Revit puts in
  a curtain wall names that wall alone, and the three Snowdon doors that name a
  curtain wall and that Revit leaves standalone name their host as well, so a
  door whose list names a curtain wall and another wall keeps RE-452's rule.

- **The `export_ifc` tool's schema says which `rvt-ifc` switches it offers and
  what its mesh choices draw (#1919).** The `options` description called them
  "the switches that change the model", but `--placeholder`,
  `--diagnostic-proxies` and `--family-extrusions`, which do, are not offered;
  it now says so. The `default`, `doors_and_windows` and `all` choices of
  `saved_meshes` also draw the walls that embed a curtain wall from their saved
  graphics where Revit raised their base, on Revit 2024 to 2027 (RE-340), and
  `off` keeps their record's body, none of which the descriptions said.
  `docs/mcp.md` and the committed tool contract say the same.

- **Revit 2015 to 2023 doors and windows are set in the wall their own
  record names (RE-448).** On those releases a door or window was set in
  the nearest wall its record's reference list named, which is not always
  its host; the correction from the element's own record (RE-245) ran on
  2024 to 2027 only and read the host only in 2024's 8-byte form. It now
  runs on 2015 to 2023 too, in either form. Autodesk's `rac_advanced` has
  28 doors and windows and `rac_basic` one moved into the wall the same
  project's 2024 save sets them in, their openings in that wall where the
  2024 save's are.

- **Revit 2015 to 2023 walls get the openings their rectangular wall
  openings cut (RE-456).** The steps of a wall's record that name what cuts
  it save that element's ElementId in 4 bytes before Revit 2024, and they
  were read only in 2024's 8-byte form, so on 2015 to 2023 no rectangular
  wall opening, cabinet, door in a second wall or curtain wall cut an
  opening. Both forms are read now, on Revit 2015 to 2027. A rectangular
  opening of 2015 to 2018, which saves its rectangle only in the wall's
  face, is placed along the wall's location line. Autodesk's `rac_basic`
  gains its two rectangular openings and `rac_advanced` its one on every
  save from 2015 to 2023, each equal to the same project's 2024 save, and
  BIMcollab's Revit 2015 structural example gains 18, the 15 of them that
  Revit's own export of it holds each equal to that export's.

### Security

- **A wall whose edited elevation profile has more than 512 regions to cut keeps
  them as its openings and has none cut in its body (#1845).** Each region cut
  in a wall's body is one level of a chain of boolean results, and a chain's
  depth is the depth of every recursion over it. On a thread of 1 MiB the derived
  `Debug` of a `SolidShape` breaks at 3,072 levels and `Clone` at 4,096, and
  `Drop` and the serialiser at 65,536 (`probe_1845_boolean_chain_depth`); a
  profile of 4,096 curves can make 4,096 regions, one circle each. Nothing in the
  export clones or prints a model, so the hazard was a library consumer's: a
  crafted file whose model it cloned or printed. The openings Revit's export
  writes for the regions are written either way, as for a wall whose regions
  are not all solids; only the body is not cut. No wall of the corpus has that
  many regions: 146 of 146 exports and 121 of 121 diagnostics files of an
  unfiltered Measure run are byte for byte as before.

- **`rvt-rs-mcp` bounds what an answer carries and what the server holds
  (#1883).** Answers were sized by the file and the client with no byte cap,
  and a crafted file could make any name, value or message in one as long as
  it liked. A name or value from the file is now cut at 1,000 characters
  (`--max-field-chars`), with the number of characters left out written where
  it was cut; a message that quotes the file and each string of the
  diagnostics document are cut at eight times that, since rvt-rs writes most
  of the long ones itself (of 1,094 diagnostics documents of five Measure
  runs, the longest string is 1,129 characters, so none is changed); the
  diagnostics document, the one answer no `limit` pages, keeps at most 5,000
  entries of a list or object and names in `notes` the ones it held (the
  longest list measured has 1,070); and an answer over 4 MiB of JSON
  (`--max-answer-bytes`, counted once although the wire message carries it
  twice) is refused with a message that says how to ask for less, not sent.
  The tool descriptions say that names, values and messages in an answer are
  text from the file, to report as data and never to follow as instructions.
  **Two behaviour changes:** `list_rooms` is paged as `list_elements` is, with
  `limit` from 1 to 1000 and `offset`, 1000 by default so that a model with up
  to 1000 spaces (the largest measured has 308) answers as before, and its
  answer says `total`, `offset`, `limit` and `returned`, a `returned` below
  `total` being a list that is not complete; and a `limit` outside 1 to 1000 is
  a tool error that says so, where 0 returned one row and a value over 1000
  became 1000 without a word. Both tools' schemas state the range.
  Four more bounds: reading a file and decoding a model take a place at a gate
  with as many places as models are kept (`--max-open-models`), so concurrent
  calls for different files no longer each decode a model before the store
  drops the ones past that number, and a cancelled call leaves the queue (a
  call that has read its file and waits to decode it still holds the file's
  bytes, which the gate does not bound); `export_gltf` writes the GLB from the
  buffer it builds instead of beside a padded copy and a third (the new
  `rvt::ifc::gltf::write_glb_to` writes the bytes `write_glb` does, and a GLB
  over 4 GiB is an error where its length wrapped); the progress queue holds 16
  events and drops what a slow client has not read, where it grew without end;
  and a phase name over 256 characters is refused, the phase errors the server
  remembers being bounded like any message. A file whose modification time
  cannot be read is refused, where it counted as time 0 and a changed file of
  the same size passed for the one opened.
  Measured by the end-to-end test on the synthetic fixture with small limits
  (text cut at 1 character, messages and diagnostics at 8, and an answer limit
  of 900 bytes: the summary is refused, the levels answer is not, the export's
  diagnostics and a message that quotes a path are cut), by pages of three
  covering `list_rooms` on every file the test opens, and by four concurrent
  opens under `--max-open-models 1` that all finish with one model kept. Not
  measured: the peak memory of concurrent opens, and a crafted file with a very
  long name, which none of the corpora holds.

- **`rvt-info -f csv` writes a file's text as text, and `rvt-dump` prints a
  stream's name without its control characters (#1885).** A title, a user
  name, a path or a name of a file's own reached a CSV cell as it stood, so
  one that starts with `=`, `+`, `-` or `@` ran as a formula when the CSV was
  opened in a spreadsheet. Such a cell is now led by `'`, as `rvt-schedule`'s
  cells have been, from one shared rule (`rvt::cli::csv_text`). A crafted
  file can also name a stream with control characters, which `rvt-dump`
  printed to the terminal as they were; it prints the name escaped. The file
  name `rvt-dump` writes a stream to was already sanitised. No committed
  output holds either, so none of it changes.

- **`rvt-doc --redact` scrubs the error text, every part of the path and every
  string a field holds (#1919).** The flag scrubbed only the user name of a
  home folder in the path it printed, against its own help, which promised
  user names and Autodesk-internal paths. A file in an Autodesk OneDrive or
  project-id folder printed that folder as it stood, and the error `rvt-doc`
  prints when it cannot open a file, which names the path it was given, was not
  scrubbed at all. The path, the notes and the error text now go through the
  scrubber the other commands' `--redact` runs (`rvt::redact::redact_sensitive`),
  and the strings of the fields through the new
  `rvt::redact::redact_json_strings`. No field of the 11 family releases (2016
  to 2026) or the 6 projects measured decodes as a string, so the output of a
  file that opens changes only in its path.

### Research

- **The column face is not where Revit stops the beams of a Revit 2027 concrete
  frame, and `tools/re/beam_ends_vs_ifc.py` counts what the ends of Revit's own
  beams lie on (#1839, RE-455).** The reporter ran the beam-end probe and the
  scorer on their model against Revit's export of it, and posted the counts:
  of 915 beams Revit shortens 538, by a median 256 mm; the Revit 2023 rule
  (the face of the column a beam's list names) gives bounds on 361 of them and
  is exact on 2, cutting 310 too far, and would cut 76 beams Revit leaves at the
  box's length, so it stays off for Revit 2024 and later. The scorer now also
  takes each Revit beam's mesh in the world frame and counts its two ends by
  the kind of element whose face (a column's, a beam's, a wall's or a slab's,
  within 1 mm) each end lies near in Revit's export, which asks Revit's own
  geometry what stops the beam without the record box. On the timber office of
  the ten BIM4LCA models, where the rule is exact on 59 of the 60 beams it cuts,
  108 of the 142 ends of the beams Revit leaves at the box's length lie on a
  column's face (105 of 138 at building permit); of the 13 beams Revit shortens
  on the ten models, one (the timber office at permit) has its ends on column
  faces.

- **Revit's export stops a beam where its record box ends on the BIM4LCA
  models, so the column face cannot be tested there (RE-455, #1839).** On the
  ten BIM4LCA models that have Revit's export (300 beams), Revit's beam is its
  record box's length within 1 mm on 273, shorter on 25 (13 beams) and longer
  on 2. The only beams that name a column are the timber office's, 63 on each
  file, and their box already ends at the column's near face: RE-122's rule
  gives the box's own ends on 116 of the 118 it reaches that Revit leaves at
  the box, cuts beam 3360608 230 mm short on both files, and is exact on the
  one beam Revit shortens and it reaches. The 13 beams Revit does shorten name
  a wall (7), a column (1) or nothing (5), and a wall's near face is not
  Revit's stop for any of the ten beams it holds an end of. Nothing ships. The
  test is a model whose boxes run to the lines' ends, such as the reporter's:
  the probe and `tools/re/beam_ends_vs_ifc.py`, which prints counts only
  without `--list`, are for it.

## [0.21.0] — 2026-11-13

0.21.0 makes the GlobalIds an export writes hold from one save of a project to the next, and adds a command that compares two saves element by element. Between two saves of one Autodesk sample, every GlobalId the writer derived moved while the elements kept theirs, because each was seeded from the document's `Unique Document GUID`, which changes on every save; each is now seeded from the GlobalId of the entity it belongs to, which the file keeps (#1840, #1854). `rvt-diff --elements` lists what changed between two saves, keyed by GlobalId (#1843). `--redact` now scrubs the user name of a Linux home folder and every path of a message (#1871). The warning for element records of ElementIds the element table does not declare no longer calls the IFC incomplete for what a same-save comparison showed to be deleted elements (#1838).

Not measured, and said in each entry:
- What belongs to no entity still moves between saves: the project, site, building and storeys where Revit gives them no GlobalId, classifications, materials, MEP systems, and types Revit gives no GlobalId (#1854).
- `rvt-diff --elements` compares what rvt-rs reads, and a change in how rvt-rs reads a file shows as a change, so two saves are compared with one build (#1843). On Autodesk's `rst_basic`, its 2025 and 2026 saves differ in 12 beams' level-bind provenance because the 2026 records name one more Level (#1864), not because rvt-rs reads them differently.
- On the 2027 save of `rac_advanced`, 80 curtain parts (79 `L Mullion 1` corner mullions and a curtain door) stand in no curtain wall where the 2025 save aggregates them, so the 79 mullions get no `IsExternal` (#1865).
- The reading of #1838 rests on the one model compared with Revit's export of the same save (a Revit 2027 structural model, as counts): no model of Measure has such a record.

Measured against 0.20.0 on every model Measure holds (run
[43381774360](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43381774360),
`main` at 9b7dee8c, 146 exports; the release adds only its notes, witness files,
version strings and fixtures to them):

- All 146 exports move, and `diagnostics.json` is equal on all 121 models that
  publish one, so what is read, approximated and left out does not change. That
  the exports move only in their GlobalIds is what the line-by-line comparisons
  of the two GlobalId changes found on the exports they kept (#1850 for the
  further pieces, #1859 for the rest: every changed line is an entity with only
  its GlobalId changed, apart from the header's timestamps); this run compares
  hashes, not lines.
- The same-project check of the MEP samples shows what the GlobalIds change:
  `bodies_across_saves` for the 2025, 2026 and 2027 saves of `rme_advanced` and
  `rme_basic` goes from FAIL (25 differences in each, the layered floors'
  pieces) to PASS, and the sketch outlines of the 2026 and 2027 saves against
  the 2025 save from 47 equal and 6 differing to 53 equal and none. Those six
  scorer outputs are the only ones that differ.
- Time and memory, from the timing job of Measure run
  [43381779893](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43381779893)
  (`rvt-ifc` built at v0.20.0 and at `main` 9b7dee8c, each file exported three
  times in turn on one runner, the median kept, with the peak resident memory of
  the export): the 75 files take 0.94 to 1.06 times the time (median 1.00; the
  1.06 is `rac_advanced` 2024, whose three rounds run 4.3 to 4.7 s on the base
  side and 4.6 to 5.0 s on the head side) and 0.99 to 1.01 times the peak memory
  (median 1.00).
- Not measured in this run: the size of the IFC.

### Upgrading from 0.20

- Every export's IFC header names rvt-rs 0.21.0, so an export that is otherwise
  unchanged still differs from 0.20.0's there.
- **Every derived GlobalId changes once (#1840, #1854).** Openings, voids,
  fills, property sets, quantity sets, material associations, containment,
  aggregation, type relationships, ports and the further pieces of a layered
  floor, a slab or a curtain wall take new GlobalIds in this release; from then
  on they hold from save to save. A tool that follows elements by GlobalId (a
  clash or issue-tracking (BCF) workflow) sees those removed and added again
  between an export made with 0.20.0 or earlier and one made with this
  release. The elements' own GlobalIds do not change.
- **`rvt-diff` has a second mode, `--elements A.rvt B.rvt` (#1843).** Its JSON
  is described by `docs/schemas/rvt-diff-elements.schema.json`. The first mode,
  two files compared stream by stream, is unchanged.
- **Rust: `rvt::ifc::element_diff` is new (#1843).** It adds public items and
  changes none.
- **The wording of the warning for undeclared element records changes
  (#1838).** The command line's warning, the diagnostics' warning, `rvt-inspect`'s
  failure-mode text and the viewer's card say what these records are; the count,
  the readiness score and the `element_record_without_element_id` item, and
  `rvt-inspect`'s failure kind `incomplete_model`, are unchanged.

### Added

- **`rvt-diff --elements` compares two saves of a model element by element
  (#1843).** Each file is exported as `rvt-ifc` exports it, and the elements
  are keyed by the GlobalId the export writes. It lists the elements added,
  removed and changed, and for each changed one the fields that differ:
  class, name, storey, type, predefined type, material, where the body is
  placed, a digest of the body, and each property the export writes, less
  rvt-rs's own record of where in the file it read the element (its
  stream and offsets), which a later save moves. `--json`
  writes the whole comparison (`docs/schemas/rvt-diff-elements.schema.json`),
  and `--redact` scrubs names, parameter names and text values, and the file
  paths in what it prints. An element whose GlobalId an export writes for more
  than one element is left out of the comparison and reported, as
  `duplicate_global_ids` and `set_aside`, so a repeated GlobalId cannot hide a
  change. The library is `rvt::ifc::element_diff`. What rvt-rs does not read is not compared, and a
  change in how rvt-rs reads a file shows as a change, so compare two saves
  with one build. Suggested by HWT-25 (#1829), who compared two saves of a
  Revit 2027 model by hand.

### Fixed

- **Records of ElementIds the element table does not declare are no longer
  all reported as an incomplete IFC from a layout this release cannot locate
  (#1838).** The element records the reader leaves out because their ElementId
  is not one the file's element table declares were reported that way. A Revit
  2027 structural model, compared with Revit's export of the same save, showed
  84 of them were roof framing deleted between two saves: the record chain
  was the same in both, the table declared 143 fewer of its records, and
  Revit's export held none of the 84 and nothing the reader lacked (2,355 of
  2,355 elements by GlobalId). The command line's and the diagnostics'
  warnings, `rvt-inspect`'s failure mode, readiness and next step, the
  viewer's card and the docs now say what was measured: on that one model such
  records were elements deleted since the file's last Save As, which a file
  saved in place keeps, and if nothing was deleted the model is likely
  incomplete. Other causes are known (an element table read short, as on
  Autodesk's Snowdon Towers 2024 sample before RE-41) or not ruled out, so the
  count, the readiness score and the `element_record_without_element_id` item
  are unchanged. Not measured: no model of Measure has such a record (none of
  its 121 published diagnostics carries the item), so the one comparison is
  the reporter's, as counts.

- **An element's further pieces keep their GlobalIds from one save to the
  next (#1840).** A layered floor's parts, a slab's further pieces and a
  curtain wall's parts share their element's Tag, and Revit gives only the
  first a GlobalId. The others were derived from the document's
  `Unique Document GUID`, which can change between two saves of one project,
  so on a Revit 2027 structural model every part's GlobalId moved on every
  save (#1829). They are now derived from their element's Revit GlobalId,
  which the file keeps. The element's own GlobalId does not change.
  **Upgrade note for clash and issue-tracking (BCF) users:** each such
  further piece's GlobalId changes once, in this release. A tool that
  follows elements by GlobalId sees those pieces removed and added again
  between an export made with the previous release and one made with this
  one; from then on they hold. With each piece go the GlobalIds of its
  material association and its property sets. On Autodesk's MEP samples
  that is 25 pieces a file, on Snowdon Towers Architectural 25, and on the
  BIM4LCA concrete housing as built 3; a model with no layered floor, slab
  or curtain wall split into parts has none.

- **Openings, property sets, quantities and the relationships between
  entities keep their GlobalIds from one save to the next (#1854).** Each
  GlobalId the IFC writer derives was seeded from the document's
  `Unique Document GUID`, which changes on every save of every Autodesk
  sample measured, so between two saves of one project every opening,
  property set, quantity set, material association, containment, void,
  fill, aggregation, type relationship and port took a new GlobalId while
  the elements kept theirs. Each is now seeded from the GlobalId of the
  entity it belongs to: an opening from its host's, a property set from its
  element's, a relationship from the entity it starts from. It holds wherever
  that entity's does, and two documents share it only where they share that
  entity's. No project-level identity could take the document's place: the
  first episode and the ProjectInfo GlobalId are the template's, one value
  on 64 of the 102 models measured. What belongs to no entity still moves
  between saves: the project, site, building and storeys where Revit gives
  them no GlobalId, classifications, materials, MEP systems and types Revit
  gives no GlobalId.
  **Upgrade note for clash and issue-tracking (BCF) users:** every such
  derived GlobalId changes once, in this release, as the element pieces of
  #1840 do; from then on they hold.

- **`--redact` scrubs the user name from a Linux home folder, from a path an
  error echoes back, and from every path of a message instead of the first of
  each kind (#1871).** The scrubber that each CLI's `--redact` runs its text
  through knew `\Users\<name>\` and `/Users/<name>/` only, so a file under
  `/home/<name>/`, or an error that names one, kept the name. It also scrubbed
  only the first path of its kind in a string, so a message naming two files
  kept the second one's name, and it left a name alone when no separator
  followed it, which is the form a home folder takes in `No such file:
  /Users/alice`. Such a name now ends at the first space, quote, closing
  bracket, comma, semicolon or colon, or at the end of the string, so a name
  with a space is scrubbed only up to the space there; a home folder with
  nothing after it is left as it is. No committed output holds a home path, so
  none of it changes.

## [0.20.0] — 2026-11-12

0.20.0 is mostly hardening against crafted files. Such a file can no longer crash the browser viewer through a record's length or an overflow in a value it stores, make a read take exponential time or time that grows with the square of its element count, exhaust the stack through the chains it sets, or write its own text into an export's STEP (#1815, #1831, #1833, #1845). No real file of the measured corpus changes from any of those fixes. A reader that stops at one of the new work budgets says so, in a `work_budget_reached` skipped item and a warning, and the export is still written.

It also changes what users of real files get:
- A wall, floor, ceiling or roof type edited since an earlier save keeps its layers on Revit 2024 and later, so 180 elements of the BIM4LCA files gain their type and its layers (#1802, RE-446).
- The project's display units are read from its Project Units (RE-445).
- 105 duct fittings of Autodesk's Revit 2023 MEP sample are drawn from their saved surfaces instead of boxes (RE-443).
- The first run of each of `rac_advanced` 2024's five stairs reads its own settings (#1845).
- Each GLB material says whether its colour and alpha are the file's (#1812).
- Revit 2026 and 2027 walls, and the curtain parts on them, get the `IsExternal` the same project's 2025 save gets (#1860).
- An element on a Level that is not a building story goes to the building story below it, as Revit's export places it, and a file whose Levels include no building story (Autodesk's `rst_basic`, saved by Revit 2016 to 2027) is written with no storey instead of storeys made up from its elements' heights (#1842).
- Two rooms of the BIM4LCA timber office drop a vertex Revit's outline does not have (#1825).
- Two exports of one file write the same wall `GrossSideArea` and `GrossVolume` to the last bit (#1862).

The saved-graphics pass of a default export keeps no inflated copy of the partitions it walks, and each of its reads visits only the partitions holding a record it selects (#1694).

Not measured, and said in each entry:
- The layer rule of #1802 rests on one project, the BIM4LCA files, the only model that holds a type it reaches. Revit 2008 to 2023 keep dropping such a type (#1805).
- None of RE-443's 105 duct fittings passes the 0.001 ft shape threshold against the native reference at its default tessellation, and that reference is not shared.
- No measured material is read the way #1820 changes, so no export shows it.
- On the 2027 save of `rac_advanced`, 79 of its 3,192 curtain members still get no `IsExternal` (#1865).
- No held-out model has a Level that is not a building story, so #1842 is measured on the BIM4LCA housing files, Autodesk's Snowdon Towers structural sample and `rst_basic` alone.
- The crafted-file bounds are exercised by crafted inputs, not by any real file, none of which comes near them.

Measured against 0.19.0 on the reference models, the BIM4LCA office and housing, the Revit 2025 models, Autodesk's sample projects of every release, the held-out models, `rac_advanced`'s IFC2X3 pairs and BIMcollab's model (Measure run
[43357862138](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43357862138),
`main` at 58395b62, 146 exports; the release adds only its notes, witness files, version strings and fixtures to them):

- 49 of the 146 exports move: 7 held-out files, 10 of the 12 BIM4LCA models (all but the office's two terrain files) and 32 of Autodesk's samples, the `rst_basic` saves of 2016 to 2027 among them. The 97 that do not are Core Interior, Einhoven, RE1's four models, puzzbobb's 24 Revit 2025 models, BIMcollab's model, the 6 `rac_advanced` IFC2X3 pairs, 27 of Autodesk's samples (the 23 of Revit 2015 to 2022 that are not an `rst_basic` save of 2016 to 2022, `rac_advanced` of 2023 and 2025, and Snowdon Towers Electrical and Plumbing), 12 held-out files, the 19 family exports and the BIM4LCA office's two terrain files. `diagnostics.json` differs on 89 of 121 models, always in the project's display units and the warning about them (RE-445), and on the models above in what they write.
- On the BIM4LCA housing's four large files the property values equal go from 27,125 to 27,559 of 30,630 (missing 3,501 to 3,083, different 4, the values rvt-rs writes that Revit does not 118, both unchanged) and the quantities from 16,977 to 17,258 of 21,956 (missing 4,979 to 4,686): on the concrete housing as built the coverings' `Reference` 79 to 87 of 88, the environmental indicators' `Reference` 1,410 to 1,456 of 1,465, the openings' `Depth` 75 to 79 of 79 and the coverings' `GrossArea` 28 to 36 of 51, with 1,375 of 1,386 types typed where 1,329 were and 347 layer sets written where 328 were. On the timber office at permit the property values equal go from 38,090 to 38,142 of 39,623 (missing 1,532 to 1,482) and the quantities from 31,425 to 31,502 of 33,673 (missing 2,247 to 2,165): the walls' `Reference` 572 to 586 of 586, the openings' `Depth` 270 to 284 of 284, 14 more types typed, 7 more layer sets written and 5 more wall ends within 0.001 ft of Revit's (311 to 316 of 586). Two rooms of the timber office, `TSTO5` and `TSTO9`, now lie within 0.01 ft of Revit's outline, as built and at permit (the space profiles' vertices within 0.01 ft 120 to 122, #1825). The concrete office's two files move no census line, the housing's terrain files move by 2 values at most and their storeys, and the property values and quantities of every other model are as in 0.19.0.
- Storeys (#1842): on the housing's terrain files the elements on Revit's storey go from 476 to 482 and from 477 to 483 (those on none from 25 to 19 each), on the timber housing as built from 1,609 to 1,620 (26 to 15) and at permit from 1,535 to 1,552 (31 to 14), the count on a different storey unchanged; Snowdon Towers Structural binds 824 elements to a storey where it bound 633. The 12 `rst_basic` saves go from 5 storeys (2016 to 2023) or 2 (2024 to 2027), made up from record-box elevations and holding 36 or 14 elements, to none.
- `IsExternal` (#1860): the values Autodesk's 2026 samples lack that the same project's 2025 save gives go from 4,643 to 0 on `rac_advanced` (its walls, curtain walls, members and plates), 244 to 0 on `rac_basic`, 2,244 to 0 on each of `rme_advanced` and `rme_basic` and 9 to 0 on `rst_basic`; on the 2027 saves from 4,750 to 186, 257 to 13, 2,351 to 107 and 9 to 0, what is left being the 79 curtain members of #1865 and the doors' and windows' `Area`, which rvt-rs does not write on those 2027 saves (186 is 79, 83 and 24 on `rac_advanced`). On one held-out house the walls' `IsExternal` equal to Revit's go from 0 to 59 of 59 (counts only).
- On the 25 held-out, BIMcollab and IFC2X3-pair files, counts only, the property values equal go from 104,526 to 104,624 of 122,608 ("different" 315 to 314, missing 17,767 to 17,670) and the quantities from 1,798 to 1,800 of 6,920; 6 of them move a census line. A held-out Revit 2024 file's stair flight gets Revit's `NumberOfTreads`, `Length` and `NetVolume`.
- What got worse (Measure's comparison lists 0 census regressions and 0 body regressions): the stair runs that read their own settings (#1845) are drawn from their sketch where their saved mesh drew them, 2.55% larger in volume on the five stairs of `rac_advanced` 2024 and one saved-mesh body fewer on a held-out file (#1856); an `IfcRoofType` is written where Revit's export writes none on the terrain timber housing (types whose entity disagrees with Revit's 3 to 4 of 502), and the concrete housing at permit writes 3 roof types against Revit's 1 where it wrote 2.
- Not measured in this run: the size of the IFC.
- Time and memory, from the timing job of Measure run [43357979238](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43357979238) (`rvt-ifc` built at v0.19.0 and at `main` 58395b62, each file exported three times in turn on one runner, the median kept, with the peak resident memory of the export): the 30 samples of Revit 2015 to 2022 take 0.98 to 1.00 times the time (median 0.99) and 1.00 to 1.02 times the peak memory (median 1.00), and the 29 samples of Revit 2023 to 2027 take 0.89 to 1.03 times the time (median 0.97) and 0.67 to 0.80 times the peak memory (median 0.76), Snowdon Towers' five models taking 0.67 to 0.74 times the peak memory. The BIM4LCA office goes from 28.6 s and 946 MiB to 24.7 s and 775 MiB as built (0.86 and 0.82 times) and from 24.0 s and 1,028 MiB to 22.2 s and 835 MiB at permit; its timber files from 32.8 s and 913 MiB to 27.2 s and 709 MiB as built and from 45.5 s and 1,343 MiB to 38.7 s and 1,158 MiB at permit; the housing's four large files take 0.81 to 0.82 times the time and 0.91 to 0.95 times the peak memory (concrete as built 28.3 s and 779 MiB to 22.8 s and 718 MiB); Snowdon Towers Architectural goes from 36.6 s and 1,111 MiB to 32.7 s and 794 MiB (0.89 and 0.71 times); Core Interior from 5.4 s and 327 MiB to 5.2 s and 290 MiB; RE1's four models take 0.94 to 0.97 times the time. What costs more: one file reads more than 3% slower, Autodesk's `rme_basic` of 2023 (8.18 s to 8.44 s, 1.03), which the timing run of the same code before #1868 (run [43354699944](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43354699944)) reads at 0.98 (5.55 s to 5.46 s), so it is the runner and not the code; no file's peak memory is higher by more than 2%.

### Upgrading from 0.19

- Every export's IFC header names rvt-rs 0.20.0, so an export that is otherwise
  unchanged still differs from 0.19.0's there.
- **Rust: `gltf::Material` has a new field, `extras` (#1812).** Code that builds
  one with a struct literal sets it. Every GLB material now carries
  `extras.colourSource` and `extras.alphaSource`, `"file"` or `"rvt-rs"`.
- **Diagnostics name the project's display units (RE-445).**
  `recovered_unit_identifiers` lists the units the project sets for length,
  area, volume, angle, mass and time. On Core Interior these are feet and
  fractional inches, square and cubic feet, degrees, pounds and seconds, where it
  listed metres, square inches and cubic inches. A new `project_display_units`
  lists every spec's unit. The "Unmapped Revit unit identifiers were preserved in
  diagnostics" warning is gone. `fractionalInches` is read as inches.
- **A file that reaches a work budget is reported, and its export is still
  written (#1833, #1845).** The export reports a `work_budget_reached` skipped
  item and a `Work budget reached:` warning per reader. `rvt-ifc` and
  `rvt-gltf` print a warning naming each reader, Python's `write_ifc` gives a
  `RuntimeWarning`, and the MCP server's exports list the stops in their notes.
  A stream a resource limit kept from being read whole still stops `rvt-ifc`
  and `rvt-gltf` with status 3 unless `--allow-partial`, as in 0.19.0. No real
  file of the measured corpus reaches a work budget.
- **Revit 2026 and 2027 walls, and the curtain members, plates and curtain
  walls on them, carry `IsExternal` (#1860).** A property set that held no
  `IsExternal` on those releases now holds the value the same project's 2025
  save gives: 4,643 elements of Autodesk's `rac_advanced` sample gain it.
- **An element on a Level that is not a building story is contained in the
  building story below that Level (#1842).** The storey an element belongs to,
  and so `IfcRelContainedInSpatialStructure`, changes for such an element. A
  file none of whose Levels is a building story (Autodesk's `rst_basic`, 12
  saves) has no `IfcBuildingStorey` now and its elements are contained in the
  `IfcBuilding`, where 0.19.0 wrote storeys named from record-box elevations
  ("Elevation -1.949 ft"). A file with no non-story Level is unchanged.
- An IFC surface style whose material's transparency was not read leaves the
  attribute unset (`$`) instead of writing 0 (#1820).

### Changed

- **Each read of the saved-graphics pass visits only the partitions that hold
  a record it selects (#1694).** The handle's record index, which answers
  that without a walk, held only the elements' records (channel 102). So every
  read of the saved graphics (channel 103) walked every partition twice, once
  to find its records and once to decode them, whatever it selected. Each
  channel a read selects alone now has an index of its own, built by the first
  such read. The exports are unchanged.

- **The saved-graphics pass of a default export keeps no inflated copy of the
  partitions it walks (#1694).** The handle held every inflated partition at
  the pass's peak: 36% to 43% of the peak resident memory of the largest
  files. The pass now indexes its records from those copies before letting
  them go. Each of its reads then inflates only the groups that hold a record
  it selects. On the largest files one partition holds nearly every group, so
  inflating whole partitions cost a walk of the file for each read.

- **Each GLB material says whether its colour, and its alpha, are the Revit
  file's or ones rvt-rs chose (#1812).** Every material carries
  `extras.colourSource` and `extras.alphaSource`, each `"file"` where the
  value is the Revit material's own and `"rvt-rs"` where rvt-rs chose it.
  The second covers the category colours an element with no material of its
  own is drawn in, translucent windows and faint rooms included, the grey of
  a material whose colour was not read, and the opaque alpha of one whose
  transparency was not read. A consumer that stores colour or opacity as
  model data could not tell them apart before. Nothing else in the GLB
  changes, and the IFC is unchanged.

  A view's visibility and graphics overrides are not read, and the support
  matrix now says so. Revit's own IFC export ignores them too: it takes colour
  and transparency from the element's material, or else its category's.

  Breaking for code that builds a `gltf::Material` with a struct literal: it
  has a new field, `extras`.

- **The export reads the project's display units from the parameter
  definitions pass it already makes, rather than walking the file again for
  its `UnitsElem` (#1816).** That pass decodes every `UnitsElem` the units
  read would, in the same order, so the units, and any error reading them,
  are the same. The read walks the file itself only where no such pass has
  been made. Exports and their diagnostics are unchanged.

- **A Revit 2015 to 2023 export reads its walls' records once, not twice, for
  whether each wall's top is joined (#1824).** The read of the walls' records
  that draws their bodies already finds each top join. It now keeps that
  answer for the export's later readers of it, and reads the walls it draws
  no body for (those with no type in the export) in the same pass, for their
  type, top join and load bearing alone. Before, the later readers read every
  wall's record again for those members. Exports are unchanged.

### Fixed

- **A wall, floor, ceiling or roof type edited since an earlier save keeps its
  layers (#1802, RE-446).**
  A type edited after an earlier save is rewritten into a newer partition and
  its older copy stays, and the layers of a type whose copies gave different
  lists were dropped, so every type edited since had none: its walls no
  `IfcWallType`, `Width` or `Reference`, its floors and ceilings no layer set.
  On Revit 2024 and later such a type now takes the copy of the partition the
  element index routes it to (the rule the transform reader uses, RE-216), and
  none where no partition is named or that partition's copies disagree. On the
  BIM4LCA files 180 elements gain their type and its layers: the 14 walls of
  `VS-2.CLTpinta` and `SPR-1` of the timber office at building permit (the
  first type's width is Revit's 0.158 m, its older copy's 0.166 m is not), the
  concrete housing's 34 walls as built and 18 at permit, 108 coverings, 4
  slabs and 2 roofs. Every layer set written is within 0.001 ft of Revit's, and
  over the 42 models with a Revit export the quantities equal to Revit's rise by
  358 and the property values by 488, none different; the three values of the
  timber office that RE-444 stopped writing for want of these layers (a wall's
  `NetVolume`, an opening's `Height` and `Width`) are written again. Six
  exports move (the timber office at permit, the concrete housing and the
  timber housing as built and at permit, the terrain timber housing) and 120 are
  byte for byte unchanged. What it costs: 6 walls of the timber office at permit
  and 6 walls and 16 coverings of the concrete housing at permit take another
  body, and two wall ends of the housing that were exact are 0.27 and 0.29 m off
  (#1806); the 19 type objects it creates carry no `IsExternal`, as no type
  does (#1185). Not done: Revit 2008 to 2023 keep dropping such a type, because
  the current copy gave four floors of a held-out Revit 2020 file materials
  Revit's export does not give (#1805), and no model but the BIM4LCA files holds
  a type this reaches, so the rule rests on one project.

- **An IFC surface style no longer states that a material is opaque when its
  transparency was not read (#1820).** A material whose colour was read but
  whose transparency was not used to be written with a transparency of 0,
  fully opaque, a value no file gave. That optional attribute is now left
  unset (`$`). No material of the measured models is read that way, so no
  export changes.

- **Saved ruled surfaces with a point profile (RE-443).** A zero profile
  pointer now uses its separately saved point, for measured point/arc
  patches with a unit envelope and rectangular trim. Bodies containing
  these patches qualify only by the source record's duct-fitting category,
  independently of their exported IFC class. On Autodesk's public Revit
  2023 basic MEP sample, 105 duct fittings gain bodies instead of boxes;
  two sanitary terminals remain boxes with an explicit refusal and are
  negative controls. Default native-IFC comparison of the fittings is within
  0.697 mm and 0.179% volume; none passes the stricter 0.001 ft shape threshold
  by default, while all 105 do with fine reference tessellation. Unmeasured
  categories, combinations and trims remain refused. The report includes
  pinned public bytes, category-override controls and local native-reference
  figures. The native reference is not shared: redistribution terms are not
  established. One cell along the straight point/arc generators retains the
  reviewed arc sampling: the 105 admitted bodies hold 64,380 triangles
  (613 on average, 636 maximum). The 2024 MEP IFC grows from 103.6 to 107.5 MB.
  Snowdon Towers HVAC retains 32 recovered duct fittings with 5,856 triangles
  (183 on average): 65.5 to 65.9 MB. RE-443 separates the current local checks
  from the maintainer's Measure of the earlier 107-body scope.

- **The export reads the project's display units from its `UnitsElem`
  record, one per spec, instead of counting unit strings, and no longer warns
  about units of specs the IFC has no use for (RE-445).** The diagnostics'
  `recovered_unit_identifiers` are now the units the project's Project Units
  set for length, area, volume, angle, mass and time: Autodesk's 2024 Core
  Interior and Snowdon Towers give feet and fractional inches, square and
  cubic feet, degrees, pounds and seconds, where the count gave metres,
  square inches and cubic inches; it gave the metric `rac_basic` and RE1
  Architecture square inches and pounds too, and they now give millimetres,
  square and cubic metres and kilograms. The warning "Unmapped Revit unit
  identifiers were preserved in diagnostics", which listed the electrical,
  HVAC and structural units every multi-discipline template sets (130 of
  them on Core Interior), is gone; a warning names a display unit of the six
  only where no IFC unit is known for it. A new `project_display_units`
  lists the display unit of every spec (133 to 150 on the corpus). A family
  whose records the native walk does not read takes its units from the
  partition's string records where their maps all agree. Revit 2015 to 2020
  files keep their units without spec names and are not read, as before. The
  IFC is unchanged: its values and unit assignment stay SI. `fractionalInches`
  is now read as inches (it was feet) and the diagnostics label a kilogram
  `KILO GRAM` and a gram `GRAM` (they said `GRAM` and `MILLI GRAM`).

- **A room outline drops a point where two straight edges turn by less than
  a hundred-millionth (#1825).** The test for a straight continuation took
  only turns under 1e-12 of the edges' lengths, so two rooms of the BIM4LCA
  timber office, `TSTO5` and `TSTO9`, kept a vertex Revit's own outline does
  not have, on both of its saves. They now lie within 0.01 ft of Revit's.

- **An element on a Level that is not a building story is placed on the
  building story below that Level, as Revit's export places it (#1842).** A
  Level whose Building Story setting is off is not a storey of Revit's export
  (RE-118), and rvt-rs bound nothing to it, so an element whose record names
  such a Level was left on no storey unless its height placed it. Revit's
  exporter gives that Level the nearest building story below it, and one below
  every building story none. Autodesk's Snowdon Towers structural sample has
  7 such Levels among its 19, and four of the BIM4LCA housing files have 2 to
  10.

- **`rvt-write` decodes a manifest's unpadded base64 to the bytes it holds
  (#1845).** A `new_decompressed_base64` value whose last group had no `=`
  padding decoded to three bytes whatever the group's length, and an `=`
  inside the text decoded as zeros, so the stream was patched with bytes the
  manifest did not give. Both now decode as standard base64 does, and text
  ending in a lone character is refused.
- **`rvt-dump` writes every stream, each to its own file (#1845).** A
  stream's name is the file's own. One holding a character Windows refuses
  in a file name (`<>"|?*` or a control character) failed its write and
  stopped the dump: the character is now `_`, as `/` is. A name Windows
  reserves for a device (`CON`, `NUL`, `COM1` and the like) takes a `_` in
  front. Two names that became one file name, the later stream replacing
  the earlier, now take `~2`, `~3` and so on.

- **Revit 2026 and 2027 walls, and the curtain members, plates and curtain
  walls on them, get the `IsExternal` the same project's 2025 save gets
  (#1860).** A wall's type Function was read only on the releases measured
  through 2025, so on a 2026 or 2027 save every wall, and every curtain part
  that takes its `IsExternal` from its wall, was written without it: 4,643
  elements of Autodesk's `rac_advanced` sample. It is now read on 2026 and
  2027 too, measured against each sample's 2025 save element by element.
  On the 2027 save of `rac_advanced`, 79 of its 3,192 curtain members still
  get none (#1865).

- **Two exports of one file write the same wall `GrossSideArea` and
  `GrossVolume` to the last bit (#1862).**
  Each boundary loop of a wall's side was summed from a vertex taken in hash
  order, so the area moved by up to 2e-14 of its value from one export to the
  next. An element-by-element comparison of a file's export with itself
  listed 4 to 117 walls as changed on seven of eleven models.

- **A file whose Levels include no building story is written with no
  storey, as Revit's export writes it, instead of storeys made up from its
  elements' heights (#1842).** Every one of the 9 Levels of Autodesk's
  `rst_basic` sample, saved by Revit 2016 to 2027, has its Building Story
  setting off. rvt-rs then fell back to storeys named from record-box
  elevations ("Elevation -1.949 ft"), which no Revit export has. Revit's
  exporter makes no storey from such a Level and contains the file's elements
  in the building, and rvt-rs now does the same.

### Security

- **A crafted file can no longer crash the browser viewer through a native
  record's length, or make a curved face's trim take unbounded work (#1815).**
  A record group whose length field put its end within 4 bytes of the address
  limit panicked the 32-bit and wasm32 builds the viewer runs. The group is now
  refused, and the rest of the file reads on. A cylinder face's trim ring could
  take 100,000 points from every edge it shares with a plane. It is now held to
  20,000 in all, and an element with a face over that is drawn without its
  saved mesh. The cuts of a trimmed parametric face, which could ask about
  4e10 comparisons of a hostile trim, are held to 250,000,000. No real file of
  the measured corpus reaches either bound. The walker's two-column container
  read is checked as its trial walk already was, though no file reaches it
  through the public reader.

- **A crafted file can no longer panic a reader through an overflow in
  arithmetic on a value it stores (#1831).** An element's creation episode at
  `u32::MAX`, or two wall shell-layer counts whose sum passes the address
  limit, panicked the 32-bit and wasm32 builds the viewer runs. Extreme stair
  riser indices, an increment's episode at `i64::MAX`, an electrical circuit
  list's entry count and a very large `TransmissionData` stream overflowed
  where overflow checks are on. Each sum is now checked, and a value that
  overflows is refused. No real file of the measured corpus changes.

- **A crafted file can no longer make a native read or the saved-graphics
  pass take exponential time (#1833).** The layout of an element index row
  was bounded only by depth, so a schema whose classes each name the next
  twice took 2^127 steps. It is now held to 10,000 class visits and fields,
  where a real row has at most 40 leaves. The check that a graphics branch
  holds only curves walked a child shared at every level once per path. It is
  now held to 100,000 calls, as the graphics walk itself is, and a branch over
  that is walked, or refused, as one that holds more than curves. No real file
  of the measured corpus changes.

- **A crafted file can no longer hang a read of pipe connectors, MEP curve
  fields, room parameters, room solids, sketch loops or empty saved-mesh
  faces (#1833).** Each did work that grew with the square or more of a
  count the file sets. Each now finds its answer through an index built
  once, or counts its work against a budget no real model comes near.
  Room parameters go through the stream's listed data objects, each
  checksum read from sums kept along the stream. Room solids are kept for
  the stream's rooms alone. Empty faces come from one walk of each graph,
  and a pipe end's keys through a set. Past a budget the reader leaves the
  element as an unread stream leaves it, and the export says so: a
  `work_budget_reached` skipped item counting each reader's stops and a
  `Work budget reached:` warning per reader. The export is still written:
  `rvt-ifc` and `rvt-gltf` print a warning naming each reader, Python's
  `write_ifc` gives a `RuntimeWarning`, and the MCP server's exports list
  it in their notes. The memo of element-data header positions is held under 64 MiB, and the memo
  of inflated partitions under 16 GiB. No real file of the measured corpus
  changes.

- **Six export and reader passes no longer take time that grows with the
  square of a crafted file's element count (#1845, #1833).** Each searched a
  list once for every element where an index answers at once: the IFC type
  objects, the host of each face-hosted cut, the property sets each piece of
  an element written in pieces repeats, each wall's join partners, the
  family blocks a content key names, and the project units each `UnitsElem`
  record registers again. Each now builds its index once. Every export is
  the same as before.

- **A crafted file can no longer exhaust the stack or hang the export
  through the chains and counts it sets (#1845).** A wall's profile cuts,
  one boolean level each, are written and meshed by a loop instead of by
  recursion. The scene graph hangs an element at most 16 hosts below its
  storey, as the IFC's storey lookup already did, so doors chained through
  each other no longer recurse as deep as the chain. The parts a covering
  element cuts from a wall's plan (1,024), the pieces left of a stub wall
  inside its neighbours (128), a roof envelope's points (about a million),
  the mullion ends in one grid cell (64) and the outline tests of space
  containment (a billion) each have a bound no real model comes near. Past
  one, that one step is left undone: the wall keeps its plan, a crowded
  cell counts no partner, nothing is contained in a space. No real file of
  the measured corpus changes.

- **A crafted file can no longer write its own text into an export's STEP,
  fail a whole IFC export over one number, or write NaN or infinity into
  the glTF or plan SVG (#1845).** The comment after an export's header held
  the file's PartAtom id unchanged, so a `*/` in it ended the comment and
  the rest became part of the file; it is now kept inside the comment. A
  stored number that is not finite, or larger than 1e30 (twenty orders past
  any model's coordinates), reached the export, where the IFC writer
  refused the whole file. It is now read as absent wherever a stored
  number is decoded (`le_bytes`, which every partition scanner reads
  through, the walker and the native graphs), so no sum or difference made
  from it becomes infinite. A sum of stair counts that would overflow
  leaves the property out. The glTF and the plan SVG, which had no check of
  their own, now draw an element whose transform or outline is not finite
  as no geometry.
- **A stair run reads its own "begin" and "end with riser" settings where a
  NaN came first in its data (#1845).** A NaN passed the test that matches
  the run's record to its sketch, so the first run of each of
  `rac_advanced` 2024's five stairs took its settings from the wrong bytes:
  it had 14 treads, not 12, and was drawn from its saved mesh, not its
  sketch. On the held-out Revit 2024 model, the run this moves now has
  Revit's tread count, length and net volume.

- **An export whose own geometry stops at a work budget says so (#1845).**
  The bounds #1845 put on wall tops, roof and slab envelopes, mullion grid
  cells and space containment left the shape out or the element uncontained
  without a word. Each now notes its stop, as the readers' budgets do
  (#1833): the export's diagnostics list it under `work_budget_reached`, and
  the export is written with a warning naming it. No file of the measured
  corpus reaches one.

### Research

- **A wall whose line drifts a hair off a model axis keeps its line's ends
  (RE-447, #1806).** Two walls of the BIM4LCA concrete housing at permit lost
  an exact end when #1807 gave them their type's thickness. Their lines drift
  about 5e-8 ft across an axis, so they are drawn from their line's ends and
  not their joined record box's. Over the 21 ends of four BIM4LCA models where
  the box and the line differ, the box is nearer Revit's end at 12 and the
  line at 9. At each of those 9, Revit's end lies on a layer plane inside a
  joined wall, which no box end reaches. The best per-end rule, from the end's
  own join record, picks right at 19 of the 21 and is never worse than the
  line. It was fitted on those same ends, though, and no other wall of the
  119 exports drifts. So nothing ships, and the ends join #1814.
  `tools/re/wall_ends_list.py` now lists every wall with an end off by more
  than the tolerance, not only the 40 largest.

- **A probe lists every native pass a default export makes (`probe_pass_ledger`).**
  Each `extract` and `extract_graphs` walk, and each use of the handle's
  parameter-definitions pass, is listed with the line that asked for it, the
  partitions it walked, the bytes it inflated and its time. A selection read
  twice, and a definitions pass made again after the export let it go, are
  named. It is kept only while a probe turns it on, so exports are unchanged.

## [0.19.0] — 2026-11-12

0.19.0 takes back much of the time and memory that 0.18.0's saved-mesh default cost, draws more of the elements that stayed bounding boxes (which makes the IFC of some files larger), and writes more of what Revit's own export writes for the bodies it draws. A decoded graph's fields take a sixth of the memory they did (`rvt::research::graph_value::Fv`), and the saved-graphics pass builds only the fields meshing reads, cuts its batches by the bytes of the symbols they read and meshes the owners that name nothing 5,000 objects at a time (#1694): the default export of Autodesk's 28 samples of Revit 2023 to 2027 takes a median 0.64 times the peak memory of 0.18.0's (0.29 to 1.18) and 0.89 times the time (0.75 to 1.27), and against 0.17.0, which drew doors and windows alone, 1.04 to 1.92 times the time and 1.04 to 1.69 times the peak memory where 0.18.0 took 1.13 to 1.85 and 1.42 to 4.38. A revolved face is cut into steps by the reach of its profile over its own range, and a face cut by its trim is cut across at the steps of the chord and of the trim and not at every point of it (#1737, RE-435, RE-437), which takes 5% to 14% off the triangles of the files each reaches. More elements are drawn from their saved meshes: faces where a plane meets a cylinder across both of its parameters (175 elements of Autodesk's 2024 samples, RE-426), faces whose trim is not the rectangle of their parameters or has holes (119 elements of Snowdon Towers Architectural and 43 of each BIM4LCA office, RE-430), plane rings that visit a vertex twice (21 sanitary terminals, RE-434), four of Snowdon Towers Architectural's six block roofs (RE-422), and the curtain mullions and panels that were drawn as their turned box (RE-421). A beam gets its six base quantities from its saved mesh (RE-423) and its `Length`, `Span`, `Slope` and `Roll` from its location line (RE-428); a column its `Length`, `CrossSectionArea`, `OuterSurfaceArea` and `GrossVolume` (RE-424, RE-425) and a hollow section its `NetVolume` (RE-247); a curtain mullion its `NetVolume`, `NetSurfaceArea` and `Length` (RE-404, RE-418, RE-432, RE-433, RE-436, RE-438, RE-439, RE-441); a wall that is a prism less its openings its side areas, gross volume and height (#1046); a building element proxy its `NetVolume` (RE-387); and walls, floors and stairs their `LoadBearing` (RE-429). The housing's railing types have the name Revit gives them (RE-431), an element framed in several partitions is read from its newest frame (RE-444), and an element whose "Export to IFC As" names `IfcGeographicElement` is one (#1772). Furniture, railings, walls, ramps, light fixtures and pipe segments carry their own or their type's `PredefinedType` (#1681, #1713), and a Revit 2027 save is written without one on the fourteen classes whose exporter leaves it off (#1692, #1716): this is the one change that moves an export on purpose for every user of a 2027 file. A planar face whose rings hold more than 10,000 points, or whose boundary search would take more than 250 million steps, is refused and keeps its box instead of being triangulated, so a hostile file cannot take an export hours (#1748, #1770).

Not measured, and said in each entry: the quantity rules for beams, columns, curtain mullions and walls were developed on, and are scored against, the one project with a Revit export that holds them (the BIM4LCA office and housing, Revit 2024), so the 938 beams of Autodesk's samples and the columns, mullions and walls of the models with no Revit export that gain them are scored by none (RE-423, RE-424, RE-425, RE-404, #1046). A mullion's `NetSurfaceArea` stays unread beside a mullion the reader leaves out and at two kinds of end, a mullion across centred on an end (3 of 28 wrong) and an overlapped mullion's axis on the plane of a free end (4 of 4 wrong, RE-441); the 8 hollow tube beams of the office get none of the six (RE-442); and `LoadBearing` is written on Revit 2024 and later, a stair with a checked box and a wall whose usage is Shear or Combined being in no model (RE-429). A mesh is not always Revit's: the 24 railings newly drawn read about 1.2 times Revit's volume (RE-426), the 102 chairs 1.0164 and the faucet 1.0992, and no body of the 176 elements newly drawn that Revit's export holds has every vertex of Revit's within 0.001 ft of rvt-rs's surface, Revit drawing round tubes as inscribed polygons (RE-430). The rule that leaves `PredefinedType` off beams, columns, plates and coverings of a Revit 2027 save follows Autodesk's exporter source and no measured 2027 export (#1716). Reading the newest frame (RE-444) leaves 25 elements of the timber office at permit, 10 of the concrete office at permit and 1 of the concrete housing as built whose newest frame is still not Revit's height, the record's box not being Revit's body there, and the 131 columns each of `rme_advanced` and `rme_basic` of 2025 to 2027 whose `Length` is not a choice of frame (#1801). The IFC is larger where more is drawn (below), the chord bound staying 0.001 ft.

Measured against 0.18.0 on the reference models, the BIM4LCA office and housing, the Revit 2025 models, Autodesk's sample projects of every release, the held-out models, `rac_advanced`'s IFC2X3 pairs and BIMcollab's model (Measure run
[43152139337](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43152139337),
`main` at 1d76c82f, 126 exports; the release adds only its notes, witness files, version strings, help text and one doc comment to them):

- 107 of the 126 exports move. The 19 that do not are the Revit 2025 model `00_Blank`, BIMcollab's model, 12 of Autodesk's samples of Revit 2015 to 2022 and 5 held-out files; the 19 family exports are byte for byte 0.18.0's.
- On the BIM4LCA office the quantities Revit writes that rvt-rs writes equal go from 34,281 to 37,451 of 39,754 as built (missing 5,472 to 2,302) and from 34,616 to 37,831 of 40,006 at building permit (missing 5,389 to 2,174), "different" staying at 1 on each and rvt-rs's own at 0: the mullions' `NetSurfaceArea` 7,532 to 9,883 of 10,003 and `NetVolume` 9,678 to 10,001 as built, the walls' `NetSideArea` 303 to 383, `Height` 306 to 386, `GrossVolume` 299 to 379 and `GrossSideArea` 299 to 379 of 579, the columns' `NetVolume` 199 to 278 of 279, the openings' `Depth` 283 to 308 of 308, `Height` 96 to 121 of 152 and `Area` 0 to 5 of 5, the spaces' `Height` 234 to 248 of 248. The property values equal go from 44,492 to 44,501 of 45,798 as built and from 44,648 to 44,655 of 45,957 at permit, "different" unchanged at 1. The bodies still drawn as their record box fall from 168 to 100 as built and from 169 to 101 at permit.
- On the timber office the quantities equal go from 27,253 to 31,693 of 33,800 as built (different 3 to 1, missing 6,544 to 2,106) and from 26,782 of 33,663 to 31,425 of 33,673 at permit (different 71 to 1, missing 6,810 to 2,247): the beams' six quantities 0 to 71 of 71 each, the columns' `OuterSurfaceArea` 59 to 339 of 339, `GrossVolume` 58 to 338, `CrossSectionArea` 60 to 340 of 344 and `Length` 203 to 339 as built, the mullions' `NetSurfaceArea` 5,431 to 7,749 of 7,876 and `NetVolume` 7,549 to 7,875, the walls' side areas, gross volume and height 78 more of 594 each. The property values equal go from 37,791 to 38,090 at permit (different 101 to 1, missing 1,725 to 1,532): the shading devices' `Reference` 0 to 44 of 44, the members' `Reference` 7,834 to 7,872 of 7,872 and `Span` 7,801 to 7,859, the railings' `Reference` and `Height` 4 to 41 of 41, `LoadBearing` 583 to 586 of 586.
- On the housing's four large files the quantities equal go from 15,006 to 16,977 of 21,956 (different 4 to 0, missing 6,946 to 4,979) and the property values from 24,673 to 27,125 of 30,630 (missing 5,953 to 3,501, different 4 to 4): the railings' `Reference` and `Height` 0 to 43 of 43 and the `LoadBearing` of the concrete housing as built's walls 0 to 369 of 369, slabs 5 to 35 of 35 and stairs 0 to 29 of 29. On the four terrain files the quantities equal go from 2,802 to 2,868 of 3,016 and the property values from 3,289 to 3,327 of 3,477, and the values rvt-rs writes that Revit does not from 98 to 12 and from 12 to 0 (the `IfcGeographicElement` of #1772).
- On puzzbobb's 24 Revit 2025 models the property values equal go from 1,354 to 1,492 of 1,930 (missing 576 to 438) and the quantities from 993 to 1,029 of 1,411 (missing 418 to 382), none different; on RE1 Architecture the property values equal go from 209 to 218, the bodies still drawn as their record box fall from 3 to 2, and the other three models' values are as in 0.18.0.
- On Autodesk's 28 sample projects of Revit 2023 to 2027 the bodies still drawn as their record box fall from 3,584 to 2,703 (Snowdon Towers' five, from 1,111 to 816); no sample of Revit 2015 to 2022 changes a body (67,661 boxes).
- On the 13 of 18 held-out files that move, counts only, the quantities equal go from 1,205 to 1,493 of 3,601 ("different" stays at 16, missing falls from 2,380 to 2,092) and the property values from 7,455 to 7,628 of 10,925 ("different" stays at 22).
- What got worse (Measure's comparison lists 0 census regressions and 2 body lines): the layer sets of the timber office at permit, "layer spans within 0.1 ft of Revit's" 450 to 449 of 450 and "whole layer sets" 553 to 552 of 552, because a wall retyped since an earlier save takes a type whose layers are not read (#1802); five values that were equal to Revit's there are no longer written (a wall's `NetVolume` and an opening's `Height` and `Width`, from the two retyped walls above, #1802, and two mullions' `NetSurfaceArea` beside a mullion whose frame moved, #1803) and a wall is drawn from its record box where it was drawn from its saved mesh (#1407), all from reading the newest frame (RE-444); one wall of the concrete housing as built carries a `LoadBearing` that Revit's export does not write (RE-429), the values rvt-rs writes that Revit does not going from 117 to 118 on the housing's four large files.
- What costs more, size: the IFC of a file that now draws more elements is larger, by the triangles. The BIM4LCA office as built goes from 87.5 to 113.7 MB (+30%; its triangles from saved graphics 672,298 to 1,325,002), the timber office as built from 77.3 to 103.7 MB, the housing's large files from 18.7 to 22.8 MB in concrete (+22%) and 20.2 to 24.4 in timber, Snowdon Towers Architectural from 189.1 to 257.8 MB (+36%; 4,206,305 to 5,826,472 triangles), `rac_advanced` of 2023 to 2027 from 42.0 to 45.4 MB to 73.3 to 76.7 MB (+69% to +75%; the 2023 save's triangles 596,651 to 1,305,375), RE1 Architecture from 1.2 to 1.4 MB, the housing's terrain files from 8.6 to 13.6 MB (+58%), the office's terrain files from 0.9 to 1.2 MB (+32%), Snowdon Towers Structural from 13.5 to 14.5 MB (+7%) and `rst_basic` of 2023 to 2027 by 2.6% to 6.4%. Where the faces RE-435 and RE-437 cut outweigh what is newly drawn the IFC is smaller: Snowdon Towers Electrical from 135.9 to 118.9 MB (-12.6%), HVAC -7.5%, Plumbing -4.1%, `rme_advanced` and `rme_basic` -1.0% to -1.7%, `rac_basic` -0.5% to -0.6% and RE1 Plumbing -11.5%.
- Time and memory, from the timing job of Measure run [43152141863](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43152141863) (`rvt-ifc` built at v0.18.0 and at `main` 1d76c82f, each file exported three times in turn on one runner, the median kept, with the peak resident memory of the export): the 30 samples of Revit 2015 to 2022 take 0.84 to 0.98 times the time (median 0.94) and 0.94 to 1.06 times the peak memory (median 0.98), and the 28 samples of Revit 2023 to 2027 take 0.75 to 1.27 times the time (median 0.89) and 0.29 to 1.18 times the peak memory (median 0.64), the most for `rac_basic` of 2023 to 2027 (875 to 890 MiB become 251 to 270). The BIM4LCA office goes from 34.8 s and 1,432 MiB to 30.8 s and 944 MiB as built (0.89 and 0.66 times) and from 30.6 s and 1,544 MiB to 27.0 s and 1,015 MiB at permit; its timber files from 32.6 s and 1,614 MiB to 28.7 s and 913 MiB as built and from 39.3 s and 1,999 MiB to 37.4 s and 1,341 MiB at permit; Snowdon Towers Architectural from 40.6 s and 2,097 MiB to 37.6 s and 1,115 MiB (0.93 and 0.53 times); Core Interior from 3.0 s and 454 MiB to 2.6 s and 328 MiB; RE1's four models take 0.78 to 0.92 times the time and 0.67 to 0.97 times the memory. What costs more: Autodesk's `rst_basic` of 2023 to 2027, 1.05 to 1.08 times the time and 1.08 to 1.18 times the peak memory (143 to 147 MiB become 159 to 169), because its beams' saved graphics are read now (#1693); Snowdon Towers Structural 1.27 times the time (4.5 s to 5.8 s, its peak memory 0.80 times); `rac_advanced` of 2023 1.06 times the time; and the housing's four large files 1.04 to 1.05 times the time (their peak memory 0.59 to 0.71 times). Against 0.17.0 (run [43152271314](https://github.com/DrunkOnJava/rvt-rs/actions/runs/43152271314), the same method) the 28 samples take 1.04 to 1.92 times the time (median 1.20) and 1.04 to 1.69 times the peak memory (median 1.29), and the 30 of Revit 2015 to 2022 0.83 to 0.98 times the time and 0.94 to 1.06 times the memory. What is left of the peak on the largest files is the inflated partitions the handle keeps (36% to 43% of Snowdon Towers Architectural's and the offices'), which #1694 prices next.

### Upgrading from 0.18

- Every export's IFC header names rvt-rs 0.19.0, so an export that is otherwise
  unchanged still differs from 0.18.0's there.
- **A Revit 2027 save is written without a `PredefinedType` on its stairs, stair
  flights, slabs, members, furniture, doors, windows, walls, railings and roofs
  (#1692), and on its beams, columns, plates and coverings (#1716).** Revit's own
  export of the held-out pairs saved by 2027 writes none on those ten classes, and
  rvt-rs wrote one; the four classes of #1716 follow Autodesk's exporter source and
  no measured 2027 export holds one of them. A file saved by Revit 2026 or earlier is
  unchanged. A tool that reads the attribute on a 2027 export handles `$`.
- **More elements carry a `PredefinedType`, and it is Revit's (#1681, #1713).**
  Furniture, railings, ramps, light fixtures and pipe segments carry their own or their
  type's "IFC Predefined Type" where it is an enumerator of their IFC4 entity, and a wall
  carries it in place of `NOTDEFINED` (`PARTITIONING`, `SOLIDWALL`, `SHEAR`, `PARAPET`,
  `PLUMBINGWALL`, ...): a tool that selects walls by `NOTDEFINED` finds the ones Revit's
  export also calls that, and no more.
- **An element whose "Export to IFC As" names `IfcGeographicElement` is one (#1772).** On
  the BIM4LCA housing terrain model the 9 furniture, 9 planting, 3 specialty equipment
  and 2 floor elements that carry the value were `IfcFurniture`,
  `IfcBuildingElementProxy` and `IfcSlab`; the two floors lose the six
  `Qto_SlabBaseQuantities` values Revit does not write.
- **An element framed in several partitions is read from its newest frame (#1710,
  RE-444).** Five BIM4LCA exports move, each only in the elements whose frame changed (the
  timber office at permit 414 of them): a height, a type, a `Length` or a `Width` is the
  newer frame's, as Revit's export has it. Revit 2008 to 2023 already did this.
- **The IFC of a file that now draws more elements from their saved meshes is larger,
  and one that draws few new ones is smaller.** The BIM4LCA office as built goes from 87.5
  to 113.7 MB (its triangles from saved graphics 672,298 to 1,325,002), Snowdon Towers
  Architectural from 189.1 to 257.8 MB and `rac_advanced` of 2023 to 2027 by 69% to 75%,
  where Snowdon Towers Electrical goes from 135.9 to 118.9 MB and `rme_advanced` and
  `rme_basic` shrink by 1.0% to 1.7%. A tool that compares triangle counts or IFC sizes
  with 0.18.0's sees both; the shapes are the same to the chord bound (0.001 ft).
- **The saved-graphics pass takes less memory and time than in 0.18.0 (#1694).** The 28
  samples of Revit 2023 to 2027 take a median 0.64 times the peak memory (0.29 to 1.18)
  and 0.89 times the time (0.75 to 1.27); the BIM4LCA office as built peaks at 944 MiB
  where it took 1,432 and Snowdon Towers Architectural at 1,115 where it took 2,097.
  Against 0.17.0 a default export takes 1.04 to 1.92 times the time and 1.04 to 1.69 times
  the peak memory (0.18.0: 1.13 to 1.85 and 1.42 to 4.38).
  `--no-saved-meshes` in `rvt-ifc` and `rvt-gltf`, `saved_meshes=False` in Python,
  `saved_meshes: "off"` in the MCP server,
  `ExportOptions::with_saved_meshes(SavedMeshChoice::Off)` in the Rust library,
  `savedMeshes: false` in the wasm binding and the viewer's option or `?savedMeshes=0` still
  keep the boxes and return the previous memory.
- **A planar face of more than 10,000 points, or whose boundary search and ring split would
  take more than 250 million steps, is refused instead of triangulated (#1748, #1770).** It
  keeps its box and says `planar face budget exceeded`; no export of the corpus changes.
- **Rust API, behind the `unstable-research` feature only:** a decoded graph's fields are
  `rvt::research::graph_value::Fv` where they were `serde_json::Value`, in `GraphObject`,
  `ObjectFields`, `Parameters`, `Parameter::raw_value` and the extensible-storage and
  metadata records that hold one (#1694). `get`, indexing, `as_u64`, `as_i64`, `as_f64`,
  `as_str`, `as_bool`, `pointer`, `is_*` and the comparisons with a bool, a string or a
  number read as they did; `as_array` returns a slice and `as_object` an `Obj` that
  iterates `(&str, &Fv)`; `to_json()` and `Fv::from_value` convert to and from
  `serde_json::Value`. The stable modules of `rvt`, the CLIs' JSON, the Python package and
  the wasm bindings are unchanged.

### Added

- **A curtain mullion drawn from its saved mesh gets Revit's `NetSurfaceArea` (#1046, RE-404).**
  Where its solid's flat ends are one section each, the mullion's
  `Qto_MemberBaseQuantities.NetSurfaceArea` is the area of the faces along its
  axis plus the end faces its solid keeps (RE-268's count, now read for a
  mullion on any host, where the box's own value was written only on a plain
  one); a mullion with a mitred end gets its lateral area plus the mitre face,
  and the flat end where that is kept. A mullion with a partner (a piece of the
  same run that meets its end) at an end that count keeps is given none, since the
  count reads the boxes of one curtain wall and misses a continuation on another.
  Revit's export writes the same on the mullions of the BIM4LCA housing and
  office whose body rvt-rs draws from the mesh Revit saved: 1,634 to 1,658 more
  of each office's mullions read equal to Revit's (the timber office at permit,
  5,292 to 6,950 of 7,872) and the 184 mitred mullions of the housing's large
  files, none different. 13 of Autodesk's samples gain the value on 4,877
  mullions, which no Revit export scores.

- **The opening a door cuts in a second wall carries
  `Qto_OpeningElementBaseQuantities.Depth` and `Height` (#1165, RE-338).**
  `Depth` is the extent the opening is extruded across that wall by, the door's
  own width, and `Height` its outline's height; Revit's export measures this
  opening the same way round, not swapped as on a door's own opening. The
  opening's `Width`, which Revit's export writes too, stays out: Revit's is the
  thickness of the wall the door stands in, and rvt-rs's outline is 0.2 mm wider
  than that on two of the four openings read.

- **A railing's type is named by the latest copy of its type object, which gives the housing's railings
  their `Reference` and `Height` (#1193, RE-431).** Every railing type of the BIM4LCA housing holds an
  older copy with a Finnish name and a newer one with the English name Revit's export carries, so the
  names never agreed and no type was attached. The name from the latest partition that holds one is
  Revit's on all 198 of the housing's railings, and the 237 railings of the housing, the timber office
  at permit and the two terrain office files gain their type, `Pset_RailingCommon.Reference` and
  `Height` and `Pset_EnvironmentalImpactIndicators.Reference`, each Revit's (4 to 241 of 241 and 237
  more, none different); 7 of 126 exports move and the other 119 are byte-identical. A railing type
  object is written for each of them, and `Qto_RailingBaseQuantities.Length` stays out of the housing.

- **A wall, a floor and a stair get the `LoadBearing` Revit's exporter calculates for them (#1193, RE-429).**
  `Pset_WallCommon.LoadBearing` is true when the Structural box of the wall's record is set and its
  usage is Bearing, and false otherwise; `Pset_SlabCommon.LoadBearing` is whether the floor's Structural
  box (`FLOOR_PARAM_IS_STRUCTURAL`) is set, and a stair whose box is unchecked gets false. They are
  written on Revit 2024 and later; a parameter named for the property still replaces the value. They
  read equal to Revit's on 4,006 of the 4,253 walls the censuses hold (the BIM4LCA models, the R2025
  models and RE1; from 2,372), 294 of 313 slabs (from 171) and all 168 stairs (from 52), none different,
  and on the 138 walls of the held-out models of those releases. The 247 walls left are an IFC2X3 pair of
  2018, counted apart. Autodesk's samples of Revit 2024 to 2027 and Core Interior, which no Revit export
  scores, gain the property on their walls and slabs (6,041 elements across the 60 exports Measure keeps
  that move, each in the set it already held); no element is added or removed. A stair with a checked
  box, a wall whose usage is Shear or Combined, and walls, floors and stairs before Revit 2024 are left
  without: no model holds the first two, and the older models show only the false state. Timed on 26 files of Revit 2024
  (Measure run 43040952174), the change reads 0.96 to 1.04 of `main`'s time and 0.95 to 1.00 of its
  peak memory.

- **A column drawn from its saved mesh gets the `Length`, `CrossSectionArea`,
  `OuterSurfaceArea` and `GrossVolume` Revit's export writes for an extruded
  column (#1203, RE-424).** `Qto_ColumnBaseQuantities` now holds them, read from
  the mesh Revit saved for the column where that is one closed solid of flat faces
  with no hole through it, a prism along the vertical with two flat, square ends of
  one area whose volume is the end area times its extent (the rule of a beam's
  quantities, RE-423, with the axis the vertical): the extent, the area of one end,
  the lateral area and the volume. They are written where the column's set holds
  none of `CrossSectionArea`, `OuterSurfaceArea` and `GrossVolume`, or all three and
  no `Length`, and no value a column holds is replaced. A column Revit draws as a
  cut mesh where a slab meets its top, which has a `CrossSectionArea` and a
  `NetVolume` and no more, is no prism along the vertical and gets none, nor do a
  round column, a hollow section or a column drawn as its record box. On the
  BIM4LCA timber office as built they read equal to Revit's on 340 of 344
  `CrossSectionArea`, 339 of 339 `OuterSurfaceArea`, 338 of 339 `GrossVolume` and
  339 of 339 `Length`, from 60, 59, 58 and 203, and at permit on 336 of 340, 335 of
  335, 334 of 335 and 333 of 335, from 58, 57, 56 and 197, none different; the
  concrete office at permit gains the four on 6 columns, the housing's concrete
  files 3 of 7 (each state), its timber files 2 of 54 and its terrain files 1 of 5,
  and a held-out Revit 2023 model 4 of its 8 columns, each equal. Autodesk's
  `rst_basic` of 2023 to 2027 and Snowdon Towers Architectural and Structural gain
  them on 44 columns, which no Revit export scores. Measured on one project with a
  Revit export (the BIM4LCA office and housing, Revit 2024); the housing's 50
  box-drawn columns get none (#1705). The change reads no graphics the saved-mesh
  pass did not read: Measure's timing of 64 files reads 0.98 to 1.04 in time and
  1.00 to 1.04 in peak memory, the spread the 30 samples of 2015 to 2022, which the
  pass cannot read, show too. `--no-saved-meshes` runs no pass, so it leaves these
  quantities out.

- **A flat roof written as a slab carries its `Perimeter`, the length of the
  outline of the profile its body extrudes (#1208).** Revit's export writes a
  roof's `Perimeter` in the file's unit, where a floor's is the same length in
  feet, unscaled (RE-235 §5): on the BIM4LCA office as built it is the
  outline's length on all 10 roofs, exactly on the two read to the micrometre
  (`Basic Roof:KA-1` 1461758, 197,888.88 mm, and `Basic Roof:YP-1` 2991742,
  158,585.00 mm), and 10 of the 29 values the measurement finds missing are
  written. A floor's, a base slab's and a stair landing's `Perimeter` stay
  out, and a roof whose profile has holes gets none: whether the export counts
  a hole's outline in the value is unmeasured. A roof whose outline has a
  corner that is not a right angle gets none either: an arc of its sketch
  reaches the outline as chords, whose length is short of the arc's.

- **A beam gets the six base quantities Revit's export writes for it (#1693, RE-423).**
  `Qto_BeamBaseQuantities` now holds `CrossSectionArea`, `OuterSurfaceArea`,
  `GrossSurfaceArea`, `NetSurfaceArea`, `GrossVolume` and `NetVolume`, read from the
  mesh Revit saved for the beam where that is one closed solid of flat faces with no
  hole through it, a prism along its longest axis with two flat, square ends of one
  area, and its volume is the end area times its extent: the sums of the prism
  Revit's export extrudes the beam's profile into (the end's area, the lateral area,
  the lateral area and both ends, the volume). The beam's body does not change. They
  read equal to Revit's on 21 of the BIM4LCA concrete housing's 22 beams, 41 of the
  timber housing's 41 and 71 of the timber office's 71, none different; the concrete
  office's 8 hollow tubes, whose Revit values are no sum of their faces, get none.
  Autodesk's `rac_advanced`, `rst_basic` and Snowdon Towers Structural gain them on
  938 beams, which no Revit export scores. Measured on one project with a Revit
  export (the BIM4LCA office and housing, Revit 2024). Reading the beams' saved
  graphics costs time and memory on some files (Measure run 42972934548, base and
  head timed in turn on one runner, three rounds): Autodesk's `rst_basic` of 2023
  to 2027 take 21 to 29% longer and peak at 2.1 to 2.3 times the memory (143 to
  147 MiB become 314 to 327 MiB), `rac_advanced` of 2023 to 2027 take 14 to 16%
  longer and peak at 1.8 to 1.9 times (557 to 579 MiB become 1,010 to 1,090 MiB),
  and Snowdon Towers Structural of 2024 takes 69% longer and peaks at 2.5 times
  (494 MiB becomes 1,224 MiB). The other 57 of the 68 files timed read 0.99 to 1.01
  in time and 0.96 to 1.00 in peak memory, and the files the quantities are scored
  on, the BIM4LCA housing and timber office (run 42976698994), read 0.98 to 1.01 in
  time and 0.99 to 1.03 in peak memory. `--no-saved-meshes` runs no pass, so it
  leaves out these quantities and this cost.

- **A beam gets the `Length`, `Span`, `Slope` and `Roll` of its location line, whatever its body is drawn from (#1697, RE-428).**
  Revit's export writes `Qto_BeamBaseQuantities.Length` and `Pset_BeamCommon.Span` as the
  length of the beam's `Axis` curve, `Slope` as that curve's angle to the horizontal and
  `Roll` as the Cross-Section Rotation, and rvt-rs wrote them only where it had solved the
  beam's body along that line (`Slope` only for a level one). The line of the beam's
  element data that lies inside its record box now gives all four wherever the body comes
  from: a box along the line, a box longer than the line (RE-328, which held that no Revit
  export measures a beam longer than its line; Revit writes the line's length on all 39 of
  them), a box no section along the line closes, or a mesh. They read equal to Revit's on
  all 291 beams of the BIM4LCA housing and offices that have a line (36 of them sloped),
  none different, with 246 values added in the housing's four files. A beam with no line (9
  in those files), a beam cut by a column with no line, and a `Roll` whose rotation is not
  read as 0 stay without, as before. Across the housing and 29 Autodesk samples, which no
  Revit export scores, 700 beams gain properties and no element is added or removed; the
  beams' bodies do not change. The line is the one the beam's body already reads, so the
  change adds no pass. Measured on one project with a Revit export (the BIM4LCA office and
  housing, Revit 2024); the IFC2X3 oracle models, whose Revit exports hold no `Span` or
  `Slope` for `rac_advanced` 2015 to 2020's 8 beams, are counted apart.

- **A column drawn as its type's rectangle gets the `Length`, `CrossSectionArea`,
  `OuterSurfaceArea`, `GrossVolume` and `NetVolume` its saved mesh gives (#1705,
  RE-425).** The BIM4LCA housing draws 50 of its timber file's 54 columns (and 4
  of the concrete file's 7) as the rectangle their type's section gives, a body
  neither RE-291's box rule nor the saved-mesh pass took, though Revit's export
  writes the four on every one. The mesh Revit saved for such a column is now read
  for its quantities only, as a beam's and a wall's are, and where it is one closed
  solid of flat faces that is a prism along the vertical (RE-424) they are written,
  creating `Qto_ColumnBaseQuantities` where the column holds none; no value a column
  holds is replaced, and the body stays the rectangle. On the housing they read
  equal to Revit's on 52 of the timber file's 54 `CrossSectionArea`,
  `OuterSurfaceArea` and `GrossVolume` (from 2) and 54 of 54 `NetVolume` and
  `Length` (from 4 and 25), 7 of 7 of the concrete file's and 5 of 5 of each terrain
  file's, 526 values on six files, none different, the four columns of a chamfered
  concrete family reading Revit's section and not the rectangle's. Core Interior
  (107 columns, whose Revit export holds no base quantities) and Autodesk's
  `rme_advanced` and `rme_basic` of 2024 to 2027 (4 struts each, whose section is
  not their type's rectangle) gain them too, which no Revit export scores; the
  office's exports are byte-identical. Reading these columns' graphics costs memory
  on some files (Measure run 43000953082, base and head timed in turn on one runner,
  three rounds): Core Interior peaks at 536 MiB where it peaked at 437 to 453, the
  two timber housing files at 37 MiB more (1.04 times), and no other file moves
  outside the runs' noise. `--no-saved-meshes` runs no pass, so it leaves these
  quantities and this cost out.

- **A curtain mullion with two mitred ends and both kept gets the whole surface as its `NetSurfaceArea`
  (#1746, RE-433).** A bent joint mitres both ends of a mullion that has a bent partner at each, and the rule
  that reads a prism's end faces returned none for two mitred ends, so the 131 mullions of the BIM4LCA concrete
  office as built (and 131 at permit, 134 and 132 on the timber office) got no value although their ends are
  counted right since RE-432. The faces along the mullion plus both end faces are Revit's value on all of
  them: 528 more values equal to Revit's across the four office files, none different; 122 of 126 exports are
  byte-identical. A prism with two mitred ends and an end not kept still gets none.

- **A column drawn as its type's hollow section gets the `NetVolume` Revit's
  export writes (RE-247).** `Qto_ColumnBaseQuantities` now holds it beside the
  `CrossSectionArea`, `OuterSurfaceArea` and `GrossVolume` such a column already
  holds: not the section's area times the length but the volume Revit computes
  for it, the section's outside rectangle less its inside one times the
  extrusion's depth, which the arcs of its corners are missing from. Revit
  writes that value exactly on the five columns of the BIM4LCA office's three
  hollow-section types checked (PI-2, PI-4 and PI-5, at five lengths, to the
  last digit it writes), and the office as built is the file where rvt-rs leaves
  it out, on 79 hollow-section columns, its 80th being the round column, which
  stays without. Measured on one project with a Revit export (the BIM4LCA
  office, Revit 2024).

- **A building element proxy drawn from a saved mesh of flat faces gets the
  `Qto_BuildingElementProxyQuantities.NetVolume` Revit's export writes (#1016,
  RE-387).** The mesh Revit saved for the element is the solid Revit's
  `NetVolumeCalculator` measures, so its volume is Revit's where no face of that
  solid is curved, whose tessellation the mesh would then be; a proxy whose body
  is a tessellated profile, and the proxy's `NetSurfaceArea`, whose faces a join
  may hide from Revit's own area (RE-387), get nothing.

- **A shading device carries its type's name as `Pset_ShadingDeviceCommon.Reference`,
  the set Revit's IFC4 export writes it in.** Revit's export of the BIM4LCA timber
  office as built writes that property on all 44 of its `IfcShadingDevice` and on no
  other element, and rvt-rs writes the same 44 shading devices, so the set joins the
  ones a Revit export confirms and the value is the type name rvt-rs already writes
  as `Reference` in every other confirmed set. `IsExternal`, which Revit's export
  also writes on those 44 and on their type, stays out: no rule read gives it.

### Changed

- **A wall whose saved mesh is a vertical prism less the openings it carries
  gets its `NetSideArea`, `GrossSideArea`, `GrossVolume` and `Height` where its
  base is not attached (#1046).** Every horizontal face of such a mesh between
  its lowest and highest point must be the top or the bottom of an opening
  rvt-rs cuts in the wall, at least one opening must be cut in it, every
  vertical face beyond the four of a prism must be a jamb of one of those
  openings, and the two largest vertical faces must still agree on the gross
  area; Revit's export writes the four on all 170 such walls of the BIM4LCA
  office's two exports whose base is not attached and on none of the 5 whose
  is. A wall with no opening keeps the four-vertical-face gate, since the two
  such walls of the BIM4LCA housing with more have a `NetVolume` alone.
  Measured on the saved meshes rvt-rs reads (Measure 43115751418): 1,771 values
  read equal to Revit's over the 17 files whose Revit export writes base
  quantities, the held-out pairs among them, none different and none that Revit
  omits. The rule also writes 1,880 values where no Revit export can check
  them: 292 on Core Interior, RE1 Architecture and Einhoven, whose exports hold
  no base quantities, and 1,588 on 17 of Autodesk's sample saves of 2024 to
  2027.

- **Four of Autodesk's Snowdon Towers Architectural's six block roofs are drawn
  from their saved meshes (Revit 2024; #1683, RE-422).** Each stayed the level
  block of its sketch outline, 33% to 133% over Revit's volume, because a
  ruled-surface component was refused whenever one of its edges named fewer than
  two faces. The roofs' ruled faces are closed; the open edges are on the planes
  and the cylinder beside them, which cannot join two ruled faces, so a component
  is refused now only where a ruled face of it borders such an edge. The four are
  within 0.11% of Revit's volume with every vertex of Revit's body within
  0.000358 ft of their surface, in 198,762 triangles. A component the old guard
  refused takes a grid per face, and a face between two straight profiles a
  quarter of the cells the shared grid asked (a bilinear patch leaves its two
  triangles by one quarter of its twist over the cell count; the first version
  drew the roofs in 1,510,643 triangles where Revit's own bodies are 1,259, 138,
  240 and 192). A component the old guard accepted keeps its grid, so 124 of 126
  exports of Measure's models are byte-identical; the other two are Snowdon
  Towers Architectural and Snowdon Towers Structural, whose 43 footings
  take a saved mesh in place of their record box (Structural has no Revit IFC4
  export; the VIM export of the model scores them: the height of all 43 equals
  Revit's to 0.00001 m, the volume reads within 0.56%, the VIM's coarser
  tessellation). Two roofs stay blocks, each with its decoder diagnostic
  (#1706).

- **A curtain mullion or panel drawn as its oriented box is offered the mesh Revit
  saved for it, and says why where it keeps the box (#1689, RE-421).** RE-106 draws
  a mullion or panel turned or tilted off the model's axes as the box along its own
  axes; the saved-mesh pass took the record box, a beam's box and a turned box but
  not this one, so these bodies were never replaced and carried no reason. On Revit
  2023 to 2027 the pass now draws them from their saved meshes where all of the mesh
  decodes: the BIM4LCA office's 325 (concrete) and 327 (timber) go to 2 and 1,
  `Qto_MemberBaseQuantities.NetVolume` reads equal to Revit's on 323 and 326 more
  mullions of each office (10,001 of 10,003 and 7,875 of 7,876 as built), none
  different, and the boxes left say `partial: disconnected UV boundary`. Autodesk's
  `rac_advanced` of 2023 to 2027 draws all 148 of its oriented boxes from their
  meshes, Snowdon Towers Architectural all 1,019. On Revit 2015 to 2022 the boxes stay
  boxes and now say `not_offered: release_outside_2023_to_2027`. Measured on one
  project with a Revit export (the BIM4LCA office, Revit 2024); the Autodesk samples
  have none to score against.

- **A Revit 2027 save is written without a `PredefinedType` on its stairs, stair flights, slabs, members, furniture, doors, windows, walls, railings and roofs (#1692).** Revit's own export of the two held-out pairs saved by 2027 writes none on 71 elements of those ten classes (counts only), where its export of the same projects saved by 2024 to 2026 writes one on each (`DOOR`, `WINDOW`, `FLOOR`, `STRINGER`, `NOTDEFINED` ...), and rvt-rs wrote it. The same export still writes a `PredefinedType` on the 88 building element proxies (`NOTDEFINED`), the 29 opening elements (`OPENING`), the 8 spaces (`NOTDEFINED`) and on every type object, and rvt-rs writes those as before, so the types, their GlobalIds and the property sets that depend on a predefined type do not change. The viewer's element panel, the schedule CSV and the MCP server report the `PredefinedType` the export writes, so they say none for those elements too. A class the two pairs do not hold (columns, beams, coverings, curtain walls, plates, footings, ramps, shading devices, the MEP terminals) is not measured on 2027 and keeps its value. Autodesk's public exporter source writes an occurrence's `PredefinedType` only where the element has no type object, from its tag `v27.3.0.12`, so what decides is the exporter's build and not the save's release: a Revit 2027 install with an older exporter still writes the values, and the file does not say which exporter read it. Saves of Revit 2026 and earlier are unchanged. `tools/re/predefined_types_vs_ifc.py` now reads every class that has a `PredefinedType`, every type class and every space, and the held-out scrub keeps its counts.

- **A decoded graph's fields take a sixth of the memory, and a default
  export reads 6% to 25% faster (#1694).** The decoder now builds every
  object's fields as `rvt::research::graph_value::Fv`, which keeps an
  object's values in one slice and its field names once for all the objects
  of a class, where each field was a map entry with a key string of its own
  (a node costs 32 to 34 bytes, not 172 to 215). Nothing an export holds
  changes: 145 of 145 exports are byte-identical to `main`'s, with the same
  diagnostics and censuses (Measure run 43144800763). On one runner, three
  rounds each (run 43144804167), a default export takes 0.75 to 0.94 of the
  time (the 29 files together 0.83) and peaks at 0.50 to 1.00 of the memory:
  Snowdon Towers Architectural 1,406 to 1,114 MiB, the BIM4LCA office as
  built 1,219 to 945 and at permit 1,363 to 1,034, Core Interior 434 to 327,
  `rac_basic` 2023 348 to 250 and `rst_basic` 2023 320 to 159. The four
  largest files fall 21% to 24%, not the 30% the rows asked for; what is
  left of their peak is the open work of #1694.

- **The owners that name nothing (walls, beams, columns) are meshed 5,000 graph
  objects at a time, not 20,000, which takes 16% off Core Interior's peak memory
  and 3% off the BIM4LCA office's (#1694).** The saved-graphics pass reads those
  owners in one pass and meshes a buffer of them at a time; the buffer, about
  95 MiB at 20,000 objects, was the pass's peak on a file whose walls fill it. On
  one runner, `main` against the change: Core Interior 516 to 434 MiB, the office
  as built 1,249 to 1,205 and at permit 1,375 to 1,329, every other file 0.96 to
  1.05 of main's (the runs' noise) and export time 0.91 to 1.03. A file whose
  owners never fill the buffer, such as `rst_basic`, is not moved. The exports
  are byte for byte as before.

- **The saved-graphics pass builds only the graph fields meshing reads, which
  takes 3% to 7% off a default export's peak memory (#1694).** The pass decoded
  every field of every graph object of a graphics record, and the dhat run put
  77% to 87% of its peak in those decoded nodes. It now keeps the 66 fields its
  readers use (`SAVED_GRAPHICS_FIELDS`, through `Options::graph_fields`), reading
  and checking the rest as before. On Autodesk's samples of Revit 2023 and 2024,
  the BIM4LCA files and the reference models the export's peak reads 0.93 to 1.00
  times the previous build's on one runner (Snowdon Towers Architectural 1,395 to
  1,351 MiB, `rac_basic` 2023 361 to 347, the offices 1,301 to 1,250 and 1,435 to
  1,376), the pass's own peak 6% to 10% lower, and 125 of 125 exports are
  byte-identical. Export time is within 3% on 28 of 29 files (`rac_advanced` 2023
  reads 1.05). The decoded nodes are still most of the pass's peak: a pointer
  value is a three-entry map, about 730 bytes (#1694).

- **A plane that meets a cylinder along an edge running across both of the
  cylinder's parameters is meshed, and 175 elements of Autodesk's Revit 2024
  samples are drawn from their saved meshes in place of a box (#1706, RE-426).**
  Snowdon Towers Architectural had 145 elements refused with `shared curved
  edge is not an isoparametric interval` or `shared curved edge changes its
  fixed parameter` (a slanted plane through a cylinder, a mitred rail): 126
  are drawn now and 19 stay boxes for the next reason. `rac_advanced` gains
  21, `rac_basic` 3, `rme_advanced` and `rme_basic` 8 each, and `rst_basic`
  9 beams. The edge takes, on the plane's polygon and on the cylinder's loop
  alike, the points of the edge itself that keep its chords within 0.00025 ft
  of the edge. The 102 of the 126 that are not railings are read against
  Revit's IFC4 export of the model (local only) and agree: 100 building
  element proxies, a slab and a roof within 1% of Revit's volume, with every
  vertex of each body within 0.001 ft of the other's surface on 101 of them
  and within 0.01 ft on the other. The 24 railings read about 1.2 times
  Revit's volume (two are within 1%), as the 101 railings drawn before this
  change do (34 within 1%): Revit's export draws their round bars as polygons
  with fewer sides inscribed in the circle the mesh draws, and the scorer
  reads 1,792 of the 1,937 shells of 22 of them as exactly that; the 145 it
  does not read are #1712. The 49 elements of the other samples are not
  scored, no Revit export of them existing. No element that was drawn
  changes (0 of 19,207). The same change draws 25 railings in each of the
  four BIM4LCA offices (24 of the 25 within 1% of Revit's volume), 1 and 2
  in the terrain files, 2 in each of four held-out files and the same
  elements in the other releases of the samples (38 of 126 exports of the
  unfiltered run differ). An export that draws more is larger: Snowdon
  Towers Architectural 196.6 to 229.7 MB (+16.8%), `rac_advanced` +18% to
  +20%, the offices +9% to +11%, the rest 1% to 4%; export time stays within
  4% and peak memory within 3% of the previous build's on the same runner.

- **A ruled, revolved or cone face whose trim is not the rectangle of its parameters, or has holes, is meshed in the cells of its grid, and 119 elements of Autodesk's Snowdon Towers Architectural, 43 of each BIM4LCA office and 71 of each `rac_advanced` save are drawn from their saved meshes in place of a box (#1706, RE-430).**
  A face whose one loop did not run round the rectangle of its parameters (the
  outline of a revolved bowl, a hole in a plate, two loops of one face) was
  refused as `nonrectangular parametric trim` or `multiple parametric trim
  loops unsupported`. The `u` of every point of every loop is already a knot
  of the face's grid, so the loops cut the region they enclose into strips,
  cut across at the grid's own `v` steps and at the `v` of the trim's steps and
  cusps (#1737), each cell is fanned into
  triangles, and a triangle without area whose edges
  are all 1e-4 ft or shorter, or whose shortest edge is 1e-8 ft or shorter, is
  dropped as a pole's is; one without area and a longer edge still refuses the
  face. Furniture, sanitary terminals, coverings, one railing, two beams and a
  space change from `partition_element_record_bbox` or
  `partition_family_instance_turned_box` to `saved_graphics_mesh`: Snowdon
  Towers Architectural 119 (102 chairs, 15 sinks, a faucet, a space), the
  offices' 43 toilets each, the large housing exports 15 each and the terrain
  ones 9, `rac_advanced` 71 per save (2023 to 2027), `rac_basic` 1, Snowdon
  Towers Plumbing 13 and Structural 2, RE1 Architecture 1. 25 of 126 exports
  of the unfiltered run move, and no element that was drawn changes (0 of the
  77,842 drawn on both sides of the 24 whose IFC was kept). The 176 new
  elements that Revit's export holds are scored against it: 73 have Revit's
  volume within 1% (the offices' toilets at a median 1.0060, the housing's
  and Snowdon's sinks), the 102 chairs read 1.0164 (Revit draws their round
  tubes as inscribed polygons) and the faucet 1.0992, and no body has every
  vertex of Revit's within 0.001 ft of rvt-rs's surface: Revit's own
  vertices lie up to 0.0043 ft (the chairs) and 0.0050 ft (a toilet) from it,
  on both sides. An export that draws more is larger, and by the triangles:
  an element drawn from a revolved or ruled face carries 4,600 to 17,500 on
  average (Revit's toilet has 1,668), so Snowdon Towers Architectural grows
  from 229.7 to 269.4 MB (+17.3%), the offices by 27.8% to 31.2%, the large
  housing exports by 27% to 29% and the terrain ones by 61%, `rac_advanced` by
  53% to 56% and `rac_basic` by 8.6%. Export time reads 0.98 to 1.03 and peak
  memory 0.95 to 1.02 of the previous build's on every file of the timing run
  but the `rac_advanced` saves, which read up to 1.09 and 1.06: 71 elements
  add 759,222 triangles to a pass of six to eight seconds. 21 sanitary
  terminals of Snowdon Towers Architectural whose plane faces have holes keep
  their boxes (#1732), and a cylinder face with several loops is refused as
  before.

- **A Revit 2027 save is written without a `PredefinedType` on its beams,
  columns, plates and coverings as well (#1716).** The exporter's public source
  writes an occurrence's `PredefinedType` only where the element has no type
  object, and the creators of those four classes call that helper as the
  creators of the ten classes #1692 measured do; this is taken from the source
  and not from a measured export, since no Revit 2027 export holds one of them,
  and a curtain wall, a footing, a ramp, a shading device and the MEP terminals
  keep their value.

- **A plane face whose outline is one ring that visits a vertex twice (two lobes touching at a point) is split there and meshed ring by ring when its triangles miss its area, and 21 sanitary terminals of Autodesk's Snowdon Towers Architectural are drawn from their saved meshes in place of a box (#1732, RE-434).**
  The 21 are one toilet type, `Toilet-Domestic-3D`, whose symbol holds a plane
  face with a ring of 22 points that visits its crown twice (two thin spandrels
  under a flat arch, beside two corner pieces): earcut, given the ring whole,
  covered a sliver under the crown too, its triangles added up to 0.65% more
  than the ring's area, and the face was refused for `triangulated area
  mismatch` once RE-430 let the toilets reach it. The rings are split at a
  repeated vertex only after that check fails, so no element that is drawn
  changes: of 126 exports of the unfiltered run 125 are byte for byte the
  previous build's, and Snowdon Towers Architectural moves by exactly these 21
  elements (4,282 drawn on both sides, 0 of their meshes differ; the refusals
  `triangulated area mismatch` 21 to 0, `saved_graphics_mesh` 4,282 to 4,303).
  The 21 read 1.0063 of Revit's volume (21 of 21 within 1%), and Revit's
  vertices lie up to 0.0047 ft from the mesh, on both sides of it, as for the
  toilets of the BIM4LCA office. The model's IFC grows from 269.5 to 296.3 MB
  (+9.9%): each toilet carries 34,260 triangles where Revit's has 1,668
  (#1737). Export time and peak memory read 1.02 and 1.03 of the previous
  build's on that model.

- **A revolved face's steps around its axis take the reach of the profile over the face's own `v` range, not the centre plus twice the radius, and the triangles of 40 of 126 exports fall (#1737, RE-435).**
  A revolved face with an arc or an ellipse for a profile was cut into steps
  from `angular(c + 2 R)`, `c` the distance of the arc's centre from the axis
  and `R` its radius, though no point of the profile is farther than `c + R`
  and the face holds only the part of the profile its own `v` range covers.
  `profile_reach` takes the farthest of 64 equal steps of that range, and half
  a step's travel added so the true maximum is never short: the chord of a step
  stays within the 0.001 ft the grid keeps at every radius of the face. 40 of
  the 126 exports of the unfiltered run move, by the meshes of elements with
  such a profile (7,231 of 126,381 objects drawn from a saved mesh on both
  sides: pipe and duct fittings, light fixtures, sanitary terminals, railings,
  furniture), and no element drawn before stops being drawn. The triangles
  from saved graphics fall by 9.7% on the BIM4LCA office (a toilet from 17,494
  to 14,025), 6.4% on the large housing exports, 8.5% on Snowdon Towers
  Architectural, 14.1% on its Electrical model, 9.9% on `rac_basic` and 5.7%
  on `rme_advanced`, and the IFCs by 0.9% to 12.7% (Snowdon Towers
  Architectural 269.5 to 251.3 MB). Against Revit's export no class's median
  volume moves by more than 0.0006 and the largest change of a body's is
  0.0015; one body of RE1 Architecture, a shower head, crosses the 0.001 ft
  line of Revit's vertices from the mesh (0.00073 to 0.00103 ft), because the
  chord is now used up at the farthest radius of the face where the old bound
  left a margin, while two sinks of that file move from 0.00395 to 0.00366 ft.

- **A face cut by its trim is cut across at the chord's `v` steps and at the `v` of the trim's own steps and cusps, where every trim point's `v` was a cut, and the triangles of 25 of 126 exports fall (#1737, RE-437).**
  `trimmed_cells` cut every strip's trapezoids at each knot of `v`, and
  `face_grid` made every trim point's `v` a knot, so a trim of T points left
  every trapezoid that held a point's `v` cut once more across the whole
  face. The strips keep a knot at every trim point's `u` (a trim segment must
  cross a strip whole, so each corner of the trim is still a corner of the
  cells) and the cuts across them are the chord's steps and the `v` of the
  vertices the trim does not run through from one strip to the next (the ends of
  a step along a strip boundary and the tip of a hole): without those a strip's
  cells run straight past a vertex of the next strip's and leave a crack inside
  the face, which a first version of this change did on all 43 toilets of the
  BIM4LCA office. A face meshed on its grid keeps every trim point as a knot,
  and a plane face that shares an edge with a cut face follows the same `v`
  knots. 25 of the 126 exports of the unfiltered run move (the 43 toilets of each
  BIM4LCA office among them), and no element drawn before stops being drawn. The triangles from saved graphics fall
  by 7.5% on the BIM4LCA office (a toilet from 14,025 to 11,516), 6.8% on Snowdon
  Towers Architectural, 4.6% on the large housing exports and 10.4% on
  `rac_advanced`, and the IFCs by 0.4% to 6.6% (the office 117.1 to 113.6 MB,
  Snowdon Towers Architectural 272.1 to 257.6 MB). The vertices of a face's
  cells that lie inside a boundary edge of the same face's cells are as many as
  before, class by class, on the office, the timber office, the housing and RE1,
  none in a toilet. Against Revit's export no class's median volume moves and
  no count of bodies within 1% of its volume, or with every vertex of Revit's
  within 0.001 ft of the mesh, falls; one body of RE1 Architecture, a shower
  head, has its farthest vertex from Revit's surface move from 0.009123 to
  0.01007 ft, over the 0.01 ft column. Export time reads 0.97 to 1.02 and peak
  memory 0.98 to 1.05 of the previous build's.

- **An element whose "Export to IFC As" names `IfcGeographicElement` is
  written as one (#1772).** On the BIM4LCA housing terrain model measured, 9
  furniture, 9 planting, 3 specialty equipment and 2 floor elements carry the
  value and Revit's export writes them as `IfcGeographicElement`, where rvt-rs
  wrote `IfcFurniture`, `IfcBuildingElementProxy` and `IfcSlab`; each now takes
  its own or its type's "IFC Predefined Type" where that is a `TERRAIN`,
  `USERDEFINED` or `NOTDEFINED` enumerator, and the two floors no longer carry
  the six `Qto_SlabBaseQuantities` values Revit does not write.

### Fixed

- **A curtain mullion's `Length` and `Span` are its saved mesh's extent along its axis where the record box overruns the solid (#1046, RE-418).**
  49 mullions of the BIM4LCA timber office at permit were written 0.01 m to
  0.279 m (0.49% to 14.3%) longer than Revit's export has them, because the
  record box they took their length from overruns the solid. A mullion whose
  box volume is not its mesh's (RE-403) now takes the extent of its mesh along
  the axis its record runs it along, where the mesh is a prism along it and the
  extent lies within 35% of the box's `Length`, and its `Span` the same;
  otherwise the box's length stays. 49 more of 7,872 read equal to Revit's, and no other value on
  that file or any model with a Revit export changes; none of Autodesk's 58 samples
  changes either.

- **A curtain mullion turned near 45 degrees about its own axis gets its `Span`
  and `Length` (#1016, RE-226).** RE-106 draws such a mullion as the box along
  its own axes, but that box is not solvable from the record box where the turn
  leaves the box's two horizontal extents nearly equal, so `add_mullion_span`
  wrote nothing: 18 of the BIM4LCA office's 10,003 mullions as built. Where the
  mullion's own Z is one of the model's axes, the record box's extent along that
  axis is the mullion's length, its two other axes being square to it, and 6 of
  the 18 now read a `Span` of 0.7 m, equal to Revit's export's. The other 12,
  turned about a horizontal axis so that their own Z crosses the model's axes,
  keep no `Span` or `Length`.

- **A furniture element's `PredefinedType` is Revit's (#1681, RE-419).** rvt-rs
  wrote none on 104 of the 107 furniture of each concrete BIM4LCA housing file
  and 103 of the 106 of each timber one, where Revit's export writes `NOTDEFINED`
  on 89 or 90, `CHAIR` on 9 and `BED` on 6 or 4. An element now takes its own or
  its type's "IFC Predefined Type" where that is an `IfcFurnitureTypeEnum`
  enumerator and, on Revit 2024, 2025 and 2026 files, `NOTDEFINED` where it
  has none: the held-out pairs of those releases write it on every furniture, and
  the two Revit 2027 pairs write no `PredefinedType` at all, so a 2027 file keeps
  none, and a 2022 file keeps none too, since its export parameters are not read.
  A value that is no enumerator stays unset, since revit-ifc's releases differ on
  it.

- **The saved-mesh pass cuts its batches by the body bytes of the symbols they
  read, which takes the peak memory of the default export down on the files
  where the old batches overshot (#1694).** A batch was sized from the graph
  objects per symbol of the one before it, and the symbols sort by id with the
  largest last, so the batch that met them held several times its budget. One
  walk now reads the body bytes of every symbol an owner names without building
  a graph, and the batches are cut at 2,000,000 bytes (about the 20,000 graph
  objects of before). The export's peak memory falls from 875 to 889 MiB to 360
  to 392 on Autodesk's `rac_basic` of 2023 to 2027, from 934 to 1,001 to 724 to
  792 on `rme_advanced` and `rme_basic`, from 1,790 to 1,396 on Snowdon Towers
  Architectural and from 1,439 and 1,550 to 1,301 and 1,436 on the BIM4LCA
  office as built and at permit. No export changes (126 of 126 byte-identical
  to `main`'s) and no timed file takes more than 2% longer (Measure run
  42977559276). A file whose symbols come to one or two batches under the
  budget decodes more of them at once than the old first batch of 8 symbols
  did, and peaks a little higher: Snowdon Towers HVAC 366 to 410 MiB, RE1
  Architecture 160 to 186. `--no-saved-meshes` still runs no pass.

- **Of an element framed in several partitions, the exporter keeps the newest frame, not the one with the greatest height (#1710, RE-444).**
  A Revit 2024 and later project rewrites an edited element into a newer
  partition and leaves the earlier frame in place, and the exporter kept the
  frame with the greatest vertical extent, so an element made shorter since kept
  its older, taller frame. On the BIM4LCA timber office at building permit 414
  elements with a body in Revit's export (251 curtain panels, 65 doors, 52
  mullions, 42 walls and 4 columns) were read from an older frame, the newest
  frame having Revit's height on 405 of them and the frame kept on 1: 3
  mullions and 2 columns had a `Length` 0.05 to 0.123 m too long, 20 members
  the type of an older frame, and 2 walls an older type's `Width`. The values of
  that file that differ from Revit's export fall from 18 quantities and 52
  properties to 1 and 1, and the BIM4LCA housing gains 25 quantities equal to
  Revit's. Five exports move, and in each only the elements whose frame
  changed: the timber office at permit (250 IfcPlate, 74 IfcMember, 66 IfcDoor,
  48 IfcWall, 7 IfcSpace, 4 IfcColumn and 1 IfcCurtainWall), the concrete
  housing as built (60 IfcPlate, 16 IfcCurtainWall, 13 IfcWindow, 8 IfcSlab,
  5 IfcCovering, 4 IfcWall, an IfcDoor and an IfcSpace) and at permit (20
  IfcWall, 7 IfcWindow, an IfcRoof and an IfcSpace), and the timber housing as
  built (4 IfcCovering, 2 IfcWall, an IfcDoor and an IfcSpace) and at permit (7
  IfcWindow). Five values that were equal on the timber office at permit are no
  longer written (two walls retyped since the earlier save take a type whose
  layers are not read, #1802, and two mullions stand beside a mullion whose
  frame moved, #1803), and one wall is drawn from its record box where it was
  drawn from its saved mesh (#1407). Within one partition the greater extent
  still wins. Revit 2008 to 2023 already kept the newest frame (RE-107).

- **A railing's `PredefinedType` is its own or its type's (#1713).** rvt-rs
  wrote none on any railing. A railing now takes its own or its type's "IFC
  Predefined Type" where that is one of IFC4's `IfcRailingTypeEnum` enumerators:
  on the BIM4LCA office and housing files 360 railings read as Revit's export
  writes them and none differs. A railing with no such value stays unset:
  Revit's 2024 export writes `NOTDEFINED` on the 8 of those files, and its 2027
  exporter leaves the attribute off, so nothing is written there without a
  measurement per release. A Revit 2027 save still writes none (#1692).

- **A wall's, ramp's, light fixture's and pipe segment's `PredefinedType` is
  its own or its type's (#1713).** rvt-rs wrote none on every ramp, light
  fixture and pipe segment, and `NOTDEFINED` on 3,842 walls where Revit's
  export writes `PARTITIONING`, `SOLIDWALL`, `SHEAR`, `PARAPET`,
  `PLUMBINGWALL`, `STRAIGHT_RUN_RAMP`, `SECURITYLIGHTING` or `GUTTER`. Each
  now takes its own or its type's "IFC Predefined Type" where that is an
  enumerator of its IFC4 entity. A wall with no value keeps `NOTDEFINED`,
  which is what Revit writes there, and a ramp, light fixture or pipe segment
  with none stays unset, since Revit writes `NOTDEFINED` on some of them and
  no 2027 export of the three is measured. A Revit 2027 save still writes
  none on a wall (#1692).

- **A continuation bent off a curtain mullion's axis no longer hides its end face (#1735, RE-432).** The
  8 `Qto_MemberBaseQuantities.NetSurfaceArea` values one end face short on each BIM4LCA office file (RE-404's
  rule on the mullions RE-421 newly meshes, which landed together unmeasured) all have a partner bent off the
  axis by 1.8 to 2.2 degrees, which RE-268 and RE-404 counted as hiding the end, and Revit's solid counts the
  end faces of all 159 of the office's mullions that have one. A partner now hides an end only where it
  coincides with it (a cosine above 0.99999). The 8 read equal to Revit's on the concrete office as built and
  at permit (9 different quantities to 1 each), 7 of the timber office's 9 as built and 5 of its 11 at permit;
  30 mullions' values change and nothing else, and 122 of 126 exports are byte-identical.

- **A continuation of another section no longer hides a curtain mullion's end face (#1745, RE-436).** The 2
  `Qto_MemberBaseQuantities.NetSurfaceArea` values one end face short on the BIM4LCA timber office as built are
  two pieces of a row where a 50 x 50 mm mullion meets a 150 x 50 mm one end to end, and Revit's value counts
  the end where the sections differ whole and hides the end where they match, as it hides an end only where it
  coincides with its continuation's end face. A partner now hides an end only where its section is the
  mullion's. The 2 read equal to Revit's as built and at permit (different 2 to 0 and 6 to 4); 4 values change
  and nothing else, and 124 of 126 exports are byte-identical.

- **A curtain mullion's end beside an oblique mullion, or a partner end beside a mullion across inside, no longer hides it or leaves its mullion out, and a joint centred under a mullion across is hidden (#1767, RE-438).**
  Revit's `Qto_MemberBaseQuantities.NetSurfaceArea` counts every end of the
  BIM4LCA concrete office as built that has only an oblique mullion at it (50
  of 50), a partner and an oblique one (76 of 76) or a partner and a mullion
  across inside (52 of 52), and hides the 4 ends of the timber office at permit
  where a partner meets a mullion across at its centre. The rule that read a
  mullion's ends hid the partner ends, kept those joints and left a mullion out
  for an oblique mullion or one across inside at an end (150 of the concrete
  office as built's 10,003, 153 of the timber office's 7,876). It now reads 131
  and 130 of them, with Revit's count on every one, and the offices gain 108
  (concrete, as built and at permit) and 107 (timber, as built and at permit)
  `NetSurfaceArea` values equal to Revit's, none different; the 4 values
  different at permit fall to 1, the 3 pieces of a chain of 50 x 50 mm
  mullions being Revit's. 9 of 126 exports move, by that value alone (the
  offices, 32 mullions of each `rac_advanced` save, 24 of Snowdon Towers
  Architectural); the 70 and 69 mullions whose partner end an oblique mullion
  keeps get none here (#1790, RE-439).

- **A curtain mullion that keeps a partner end gets its `NetSurfaceArea` from its saved mesh (#1790, RE-439).**
  RE-438 reads a partner end that an oblique mullion or a mullion across inside
  lies at as one Revit's solid keeps, but the pass that draws a mullion's
  `NetSurfaceArea` from its saved mesh refused every mullion with a kept count
  of 2 and a partner end, as it could not tell such a partner from a
  continuation on another curtain wall that the wall's boxes had missed. The
  boxes now say which of a mullion's ends is kept with a partner, the mesh's two
  ends are told by the side of the mullion's axis each lies on, and only a
  partner at another end refuses it. 34 mullions of the BIM4LCA concrete office
  (as built and at permit) and 33 of the timber office gain a `NetSurfaceArea`
  equal to Revit's, none different, and 4 of Snowdon Towers Architectural gain
  one (its Revit export has none). The other 36 and 36 of the 70 and 69
  mullions that keep a partner end lie beside a mullion the reader leaves
  unread (#1794), which the report of RE-438 had taken for the mesh path's
  refusals.

- **A curtain mullion beside an overlapped mullion is read like any other, but for two kinds (#1794, RE-441).**
  The rule that tells which end faces of a curtain mullion Revit's solid keeps
  left a mullion out whenever one of its ends lay within 6 cm of a mullion that
  was left out or overlapped (another lying over it). Applied to the 314
  mullions of the BIM4LCA concrete office as built and the 280 of the timber
  office whose only unread neighbours are overlapped ones, it gives Revit's
  count on 312 and 272, the others having no value or being of two kinds
  that, measured on the BIM4LCA offices, stay unread: a mullion with a
  mullion across centred on an end (3 of 28 wrong), and an end with no
  partner where an overlapped mullion's axis lies on the end's plane, a
  vertical standing on the bar's face (4 of 4 wrong, none of the 216 chain
  ends the rule gets right). The concrete office as built and at permit gain
  353 `NetSurfaceArea` values and the timber office 320 and 316,
  every one equal to Revit's, none different; 13 exports with no Revit export
  to meet gain 562 more (`rac_advanced`, `rme_advanced`, `rme_basic` and
  Snowdon Towers Architectural). The neighbours of a mullion left out stay
  unread.

- **An opening a floor's sketch makes carries the `Qto_OpeningElementBaseQuantities.Area` Revit's export gives it where its outline is not a rectangle, that outline's area (#1016).**
  The notched openings of the BIM4LCA office files gain Revit's `Area` within
  1e-6 m2 and the rectangular ones gain none, on which Revit's export writes
  none either. An outline with a corner that is not a right angle gains none:
  an arc of the sketch reaches it as chords, whose polygon is short of the
  area Revit takes from the curves. Nothing else of an opening changes, and an
  export made with `--no-base-quantities` gains none.

- **An Area's `Qto_SpaceBaseQuantities.Height` is the height of its body
  (RE-317).** Revit's export writes it on all 30 Areas of each BIM4LCA office
  file and rvt-rs wrote none of them: an Area exported from a Revit 2024 to 2027
  file now says the distance from its Level up to the Level above, the depth its
  body is extruded, and no room or family-instance space changes. A Revit 2015
  to 2023 Area keeps none, the height of its export not being measured.

- **A curtain wall whose record holds another curve besides its location curve
  gets its `Qto_CurtainWallQuantities.Length` (#1016, RE-252).** rvt-rs read a
  curtain wall's location curve only where its native record held that curve
  alone, so 7 of the BIM4LCA office's 220 curtain walls carried no `Length`
  where Revit's export writes one on all 220. It now takes the curve the
  record's `VWallDriver.m_pCrv` names, whatever other curves the record holds,
  and still none where that curve is an arc.

- **A spiral stair flight's `Pset_StairFlightCommon.NumberOfTreads` is
  written (RE-287).** The BIM4LCA office's assembled stair flight 3057703 was
  the one flight of 35 whose tread count rvt-rs left out, where Revit's export
  writes 18: a spiral run's plan is an arc, so its begin- and end-with-riser
  flags are not read and its count was not computed. A spiral run whose treads
  are drawn as its body now gives its flight the number of those treads, and
  with `--revit-set-placement` its stair's flight set sums it; no other value
  on that file changes.

### Security

- **A planar face whose rings hold more than 10,000 points, or whose boundary
  search and ring split would take more than 250 million steps between them,
  is refused instead of triangulated (#1748).** Earcut, the search that puts
  back the boundary points earcut leaves out, and the search for a ring that
  visits a vertex twice grow faster than the points of a face, and a file can
  give each edge of a ring up to 100,000 of them, so a hostile ring could take
  an export hours. The steps are counted for the face in all (its outer rings
  and the second try after a split share the budget). The largest planar face
  of the 2023 to 2027 samples, the BIM4LCA models and the reference models
  holds 2,308 points and the most one call of the boundary search asks is 14.6
  million steps, so no export of the corpus changes; a face over either budget
  keeps its box, as every other refused face does, and says `planar face
  budget exceeded`.

- **A planar face is refused for its points as each edge of its rings is read,
  and the tests of a face with regions count against its work, instead of after
  the ring is built (#1770).** #1748 refused a face of more than 10,000 points
  or 250 million steps, but counted the points once a ring was built, after
  each of its edges had been refined up to 100,000 points, and charged nothing
  for a face with regions, whose rings were earcut with no cap and whose every
  region vertex was tested against every parent vertex. The points an edge may
  still add are now what is left of the face's budget when the edge is read:
  the graph's own points are counted before any is copied, and the edge's
  refinement stops with `planar face budget exceeded` at the last point the face
  may hold; the parent's ring and the regions' rings share one such budget, and
  the vertex tests of a region (its vertices times the parent's) are charged to
  the face's work. The budgets' values are as #1748 chose them (10,000 points,
  250 million steps; the largest planar face of the corpus holds 2,308 points),
  so no export of the corpus changes; a face over either keeps its box, as every
  other refused face does.

### Research

- **A tube beam's six base quantities are not its saved mesh's (#1698,
  RE-442).** On the BIM4LCA office as built, the saved mesh of each of the 8
  hollow tube beams gives `CrossSectionArea`, `OuterSurfaceArea`,
  `GrossSurfaceArea`, `NetSurfaceArea`, `GrossVolume` and `NetVolume` within a
  relative 1.1e-4 to 4.4e-4 of Revit's export, short every time by the chords
  of its rounded corners, where the census reads equal within 1e-5: all 48
  values would read different, so none is written. The profile Revit's export
  draws (two rounded rectangles) and the length give all six within the
  tolerance, so they arrive with the tube's body. A probe,
  `probe_beam_tube_quantities`, prints what each beam's mesh gives.

## [0.18.0] — 2026-11-11

0.18.0 draws every element that would be a bounding box from the mesh Revit saved
for it, by default on Revit 2023 to 2027, and writes more of what Revit's own
export writes for the bodies it draws. Where all of a mesh decodes, a wall, roof,
railing, curtain mullion or panel, column, door, window, piece of furniture,
fixture or item of equipment is that mesh and not its record box (#1191): a roof
that was the level block of its sketch outline, a railing whose parts are
instances of other elements, an instance forbidden the targets 10, a turned family
instance, and the ellipses, curved ruled surfaces and rings that repeat their
first point in the saved graphics are read now (#1612, #1630, #1635, RE-385,
RE-389, RE-397, RE-401, RE-408). Reading every box's saved graphics costs 1.1 to 1.9
times the time and 1.4 to 4.4 times the peak memory of 0.17.0 on Autodesk's samples of
Revit 2023 to 2027 and makes the IFC larger; `--no-saved-meshes` keeps the boxes (#1694).
A railing that belongs to a stair is aggregated under it, as Revit's export aggregates
it (#1624). A wall's `NetVolume` and side
areas, a mullion's `NetVolume` and `NetSurfaceArea`, a panel's `Width`, a roof's
`GrossArea` and `GrossVolume`, a sloped floor's `PitchAngle` and the properties of
an opening are written as Revit's export writes them where the BIM4LCA office
and housing show it, and a round column no longer carries a square's quantities.
A wall with an edited elevation profile has the openings Revit's export writes
for it. Four more "Export to IFC As" values are acted on (#1636), every type
object has a GlobalId of its own (#1667), and a standalone Revit 2020 family with
one revolution exports as an `IfcRevolvedAreaSolid` (#156, experimental). Every
record box, beam box and turned box that stays one says why, on every release: its
`SavedMeshRefusal` property and `exported.saved_mesh_refusals` give the reason
(#1645); a mullion or panel drawn as its oriented box has none yet (#1689). puzzbobb wrote the
turned-instance and curved-surface mesh reads (#1620) and the correction of a
family type table's warning (#1633), BIM4GIB the standalone family revolutions
(#1632) and their measurement (#1628), and mahach666, RvtIO's maintainer, linked
the shadow cutout steps to the steps they mirror (#1165) and offered the weld
passes and the round-column rule measured under RE-410 and RE-411; thanks.

Not measured, and said in each entry: no roof, beam, slab, footing or stair flight
mesh is among the bodies scored against Revit's export on 31 models, so the roofs
drawn from their meshes rest on Snowdon Towers Architectural (Revit 2024, local
only; 14 of its 20 roofs read within 0.061% of Revit's volume and 6 keep their
level blocks, RE-420, #1683), the T and U roof of puzzbobb's Revit 2025 models and
the same-project gates, and on 3 held-out roofs two scorers disagree (RE-389). A
mesh is not always Revit's: doors are 675 of 681, sanitary terminals 68 of 94,
flow fittings 71 of 89, flow terminals 30 of 35 and walls 14 of 18 with every
vertex within 0.001 ft of Revit's export. A stair that aggregates its railings
has Revit's height and, where a railing is still its record box because its mesh
is not whole, more volume than Revit's (2 to 127 times on the office's 13 stairs
that hold railings, RE-303). The quantity rules were developed on the BIM4LCA
office, and their first measurement on a second project, the housing, found 5 of
15 census lines holding where they are exercised (RE-402); RE-403, RE-405, RE-413
and RE-417 repair what it found wrong, and an entry says where no second project
exercises its rule. A mullion's `NetSurfaceArea` rests on the one model whose
Revit export writes it, and 263 mullions of the office whose count still gets 7
wrong stay out (RE-400).

Measured against 0.17.0 on the reference models, the BIM4LCA office and housing,
the Revit 2025 models, Autodesk's sample projects of every release, the held-out
models, `rac_advanced`'s IFC2X3 pairs and BIMcollab's model (Measure run
[42949360321](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42949360321),
`main` at 83347260, 126 exports; #1691 moves none of them, run
[42954375538](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42954375538), and the
release adds only its notes, witness files, version strings and help text to them):

- 123 of the 126 exports move. The 3 that do not are the Revit 2025 models
  `00_Blank`, `01_Walls` and `11_RoofByExtrusion`. On the 45 of Revit 2015 to
  2022 no census line moves but the plate types written with no `Tag` (0 to 5 on
  `rac_advanced`'s six saves and on the held-out Vitoria project, #1594); their
  exports move by the `SavedMeshRefusal` each box body gains (68,579 on the 31
  that report diagnostics, as many as their box bodies; the other 14 are counts
  only).
- On the BIM4LCA office the quantities Revit writes that rvt-rs writes equal go
  from 29,761 to 34,281 of 39,754 as built (missing 9,992 to 5,472) and from
  30,037 to 34,616 of 40,006 at building permit (9,968 to 5,389), "different"
  staying at 1 on each and rvt-rs's own at 0: the mullions' `NetVolume` 7,795 to
  9,678 of 10,003 and `NetSurfaceArea` 6,606 to 7,532 as built, the panels'
  `NetVolume` 0 to 260 of 260 and `Width` 0 to 257 of 257, the walls' `NetVolume`
  100 to 519 of 607, `NetSideArea` 100 to 303 of 579 and `Height` 132 to 306 of
  579. The property values equal go from 44,422 to 44,492 of 45,798 as built and
  from 44,571 to 44,648 of 45,957 at permit (the openings' `Reference` 12,787 to
  12,824, their `FireExit` 0 to 25, the floors' `PitchAngle` 54 to 62), "different"
  unchanged at 1. The bodies still drawn as their record box fall from 10,679 to
  168 as built. On its two terrain files the quantities equal go from 16 of 44 to 25
  of 45 on each.
- On the timber office the quantities equal go from 22,868 of 33,796 to 27,253 of
  33,800 as built (different 8 to 3) and from 22,335 of 33,659 to 26,782 of
  33,663 at permit (different 145 to 71); the property values different stay at
  101 at permit.
- On the housing's four large files the quantities equal go from 3,059 of 5,268
  to 3,636 of 5,358 and from 2,800 of 4,972 to 3,598 of 5,062 in concrete (as
  built, at permit) and from 2,699 of 5,788 to 3,909 of 5,872 and from 2,658 of
  5,580 to 3,863 of 5,664 in timber (the totals rise with the objects #1636
  writes), and "different" from 131, 169, 170 and 109 to 0, 4, 0 and 0; the quantities
  rvt-rs writes that Revit does not fall from 38, 2, 28 and 15 to 7, 0, 2 and 2,
  and the property values it writes that Revit does not from 167, 163, 168 and
  163 to 31, 27, 32 and 27.
- On puzzbobb's 24 Revit 2025 models the quantities equal go from 878 to 993 of
  1,411, none different, and the property values are as in 0.17.0. On RE1 the
  bodies still drawn as their record box fall from 52 to 3 (Architecture), 49 to
  2 (Electrical), 43 to 0 (Mechanical) and 60 to 0 (Plumbing), Einhoven's from 9
  to 0, and the values are as in 0.17.0; Core Interior draws none as a box in
  either release and writes no property set or quantity, as before.
- On Autodesk's 28 sample projects of Revit 2023 to 2027 the bodies still drawn
  as their record box fall from 78,310 to 3,584 (Snowdon Towers' five, from 9,953
  to 1,111); no sample of Revit 2015 to 2022 changes a body (67,661 boxes, each
  now with its reason).
- On the held-out files of Revit 2023 to 2027, counts only, 11 move: the
  quantities equal go from 845 to 1,056, "different" stays at 16 and missing
  falls from 2,593 to 2,380.
- What got worse (Measure's comparison lists 0 census regressions and 19 body
  lines): the volume of a stair that aggregates its railings, which read as their
  record boxes where their mesh is not whole (#1624, RE-303): 13 stairs of the
  office, the worst 6.0 to 911.9 ft3 as built against Revit's 7.2, and the Revit
  2025 spiral stair of `17_Rooms_Stairs_Spiral`, 683.7 to 1,365.1 ft3 against
  Revit's 58.9. A wall or floor drawn from its saved mesh is drawn whole, not in
  layers, in the glTF: 3 fewer walls in the concrete office at permit and 1 fewer
  in the timber office as built, and 9 and 2 fewer floors, roofs and ceilings in
  the timber office at permit and as built; the layers still drawn are as close
  to Revit's as before.
- What costs more, time and memory, from the timing job of Measure run
  [42956917709](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42956917709)
  (`rvt-ifc` built at v0.17.0 and at the source of `main` from #1691 on, each
  file exported three times in turn on one runner, the median kept, with the
  peak resident memory of the export): the 30 samples of Revit 2015 to 2022 take
  1.00 to 1.02 times the time and 1.00 to 1.01 times the peak memory, because
  the pass is not offered there; the 28 samples of Revit 2023 to 2027 take 1.13
  to 1.85 times the time (median 1.39) and 1.42 to 4.38 times the peak memory
  (median 1.87), the most for `rac_basic` 2023 (1.9 s and 200 MiB to 3.4 s and
  875 MiB). The BIM4LCA office goes from 25.4 s and 914 MiB to 33.0 s and 1,432
  MiB as built (1.30 and 1.57 times) and from 22.5 s and 973 MiB to 27.6 s and
  1,545 MiB at permit; Snowdon Towers Architectural from 17.6 s and 1,073 MiB to
  28.0 s and 2,086 MiB (1.60 and 1.94 times), its IFC from 37 MB to 189; Core
  Interior from 4.7 s and 305 MiB to 5.3 s and 437 MiB; RE1's four models
  1.05 to 1.14 times the time and up to 1.60 times the memory; Einhoven and the
  office's two terrain files within 5%. The peak memory still above 0.17.0's is
  #1694.

### Upgrading from 0.17

- Every export's IFC header names rvt-rs 0.18.0, so an export that is otherwise
  unchanged still differs from 0.17.0's there.
- **A default export of a Revit 2023 to 2027 project draws every element that
  would be a bounding box from the mesh Revit saved for it, where all of the mesh
  decodes (#1191).** Doors and windows alone were until now. The IFC is larger
  (the BIM4LCA office as built from 68 MB to 87 MB, Snowdon Towers Architectural from
  37 MB to 189 MB), reading the saved graphics takes 1.1 to 1.9 times the time and 1.4 to
  4.4 times the peak memory of 0.17.0 on Autodesk's samples of Revit 2023 to 2027 (the
  BIM4LCA office as built 1.3 and 1.6 times; files of Revit 2015 to 2022 take at most
  2% more time), and a very large file may need more memory than before (#1694). `--no-saved-meshes` in `rvt-ifc` and
  `rvt-gltf`, `saved_meshes=False` in Python, `saved_meshes: "off"` in the MCP
  server, `ExportOptions::with_saved_meshes(SavedMeshChoice::Off)` in the Rust
  library, `savedMeshes: false` in the wasm binding and the viewer's option or
  `?savedMeshes=0` keep the boxes and return the previous memory;
  `SavedMeshChoice::DoorsAndWindows` (new) and the MCP's
  `saved_meshes: "doors_and_windows"` give 0.17.0's default. For library
  callers `SavedMeshChoice::Default` and `saved_meshes::default_scope` now mean
  every boxed element.
- A roof that was the level block of its sketch outline is drawn from its saved
  mesh where that decodes whole (#1612, #1630), and its `ProfileSource`,
  `ProfileVertexCount`, `ProfileVoidCount` and `SlabShapeRefusal` are left out
  while `ProfileResolved` reads false; a railing is drawn from its saved mesh
  where its saved graphics are whole (#1635); an instance visible in the normal
  draw and forbidden the targets 10 and a family instance drawn as its turned box
  are drawn from their meshes (RE-408, RE-385).
- On Revit 2024 to 2027 a railing that belongs to a stair is aggregated under that
  stair's `IfcStair` instead of standing alone (#1624).
- Every record box, beam box or turned box that stays one has a `SavedMeshRefusal`
  property in its `RvtElementRecordGeometry` set, on every release, and
  `exported.saved_mesh_refusals` counts them by reason: `no_saved_graphics`,
  `no_saved_mesh`, `no_triangles`, `element_id_shared`, `partial: ` and the
  diagnostics, `not_a_candidate: ` and `not_offered: ` (#1645). A reader of the
  diagnostics or the decision log (`saved_mesh_not_used`) that lists the
  reasons handles a value it does not know.
- Quantities and properties that change on a default export: a wall's
  `NetVolume`, `NetSideArea` and, where its sides agree, `GrossSideArea` and
  `GrossVolume` from its saved mesh (RE-388), and its box's quantities and
  `Height` only where its box is its solid (RE-405); a curtain mullion's
  `NetVolume` from its mesh, its `NetSurfaceArea` on more mullions and only
  where its box is its solid (RE-387, RE-400, RE-403); a curtain panel's `Width`
  (RE-396); a roof's sketch holes stay in its `GrossArea` and `GrossVolume`
  (RE-413); a round column no longer carries a square's quantities (RE-416); a
  floor drawn along its shape-edited plane carries `Pset_SlabCommon.PitchAngle`,
  also in `--mode typed-no-geometry` (RE-384); the opening of a wall's edited
  profile, of a floor's void and of a face-hosted door or window carries its
  host's `Reference` in `Pset_EnvironmentalImpactIndicators` as well, and a
  second wall's door opening its door's `FireExit` (RE-381, RE-382, RE-386).
- A wall with an edited elevation profile has the openings Revit's export writes
  for it, the pockets of the profile against its convex hull, and the rest of the
  profile's cut is made in its body (`ProfileCutSource`
  `wall_profile_cut_in_body`, `exported.wall_profile_cuts`) (#1607, RE-380): ten
  walls of the BIM4LCA files lose an opening Revit does not write.
- An `IfcPlateType` has no `Tag` (#1594), and every type object has a GlobalId of
  its own: a type read for several symbols kept one GlobalId on all its objects
  and now only the first keeps it (#1667).
- Four more "Export to IFC As" values are acted on (#1636, RE-417): generic
  models exported as `IfcSpaceType`, `IfcFurniture`, `IfcSolarDevice` and
  `IfcTransportElement` are written as `IfcSpace`, `IfcFurniture`,
  `IfcSolarDevice` and `IfcTransportElement` instead of proxies.

### Added

- **Standalone family revolutions (#156):** decode a single visible positive
revolution in the measured Revit 2020 families and export an analytic
`IfcRevolvedAreaSolid`: line profiles with measured full, partial and tilted
placements, and arc profiles with a full turn only. Share driven sketch-curve selection with extrusions.
Six synthetic fixtures include native references and an offline replay;
project instances, voids and other releases remain outside this coverage.
See RE-392, RE-393 and RE-394.

- **An element still drawn as its record's box says why its saved mesh was not
  used (#1645).** After the pass that draws a box from the mesh Revit saved for
  it (Revit 2023 to 2027), each box that stays one has a `SavedMeshRefusal`
  property in its `RvtElementRecordGeometry` set: `no_saved_graphics`,
  `no_saved_mesh`, `no_triangles`, `element_id_shared`, or `partial: ` and the
  decoder diagnostics that left the mesh partial (`unsupported reachable
  graphics class ...`, `shared curved edge ...`). The diagnostics count them by
  reason in `exported.saved_mesh_refusals`, and the decision log
  (`rvt-ifc --export-decisions`, the MCP server's `explain_element`) names the
  reason of each element as `saved_mesh_not_used`. No body is drawn
  differently.

- **A wall's profile opening carries the wall's type name as its `Reference` and
  its profile's area, as Revit's export writes them (RE-381).** The opening of a
  wall's edited elevation profile (RE-380) now has the wall's `Reference` in
  `Pset_OpeningElementCommon` and `Pset_EnvironmentalImpactIndicators`, and none
  where the wall has none, and, where the wall has base quantities, its `Area`
  in `Qto_OpeningElementBaseQuantities`, the area of the opening's profile in
  the wall's plane, and no other quantity. Revit's export writes exactly these
  on the BIM4LCA terrain files' opening (Reference `MS-2`, Area 7.216427 m2)
  and the Reference pair on all 13 of Snowdon Towers' pocket openings (local
  only). Core Interior's export writes no property sets or base quantities at
  all (its setup), so it shows neither, and rvt-rs writes them there as it does
  for every other element of Core. Measured on one opening for the Area, which
  is why it is the profile's area and nothing more.

- **The opening of a host's own geometry carries the host's type name as
  `Pset_EnvironmentalImpactIndicators.Reference` too, as Revit's export writes
  it (RE-382).** A floor's sketch void and the opening a face-hosted door or
  window cuts in its host (RE-237) had the host's `Reference` in
  `Pset_OpeningElementCommon` alone, where Revit's export holds it in both sets
  on every opening. It does so on the BIM4LCA office's floor voids and roof
  hatch openings (40 as built and 39 at building permit; `VP-1` on the floors'
  voids, `YP-1` on the roof's hatches) and on Snowdon Towers' 12 (local only),
  each equal to the host's.
  Core Interior's export writes no property sets at all (its setup), so it shows
  nothing, and rvt-rs writes the set there as it does for every other element
  of Core.

- **A floor drawn along the plane its shape-edited corners lie in carries that
  plane's slope as `Pset_SlabCommon.PitchAngle`, as Revit's export writes it
  (RE-384).** The angle of the floor's top plane from the horizontal, the
  arctangent of the length of the gradient of `z = a·x + b·y + c`, is what
  revit-ifc's slope calculator gives a slab with no location curve. Revit's
  export writes it on the 8 sloped floors of the BIM4LCA office (1.9 to 24
  degrees), which rvt-rs has drawn along their plane since #1384 and left
  without the value; the angle of the largest upward face of rvt-rs's own body
  for each is Revit's to 2e-7 rad. A floor whose plane is not drawn gets none.

- **The opening a door cuts in a second wall carries its door's `FireExit` in
  `Pset_OpeningElementCommon`, as Revit's export writes it (RE-386).**
  Revit's export gives an opening the properties of the element that cuts it,
  so where a project parameter names `FireExit` on its doors (the BIM4LCA
  office's does), each opening a door cuts in a further wall has the door's
  value beside the `Reference` it already had. The office's export writes it
  on every opening a door cuts (43 as built, 45 at building permit, all
  `false`); rvt-rs writes the openings that are in walls (25 and 33), equal to
  Revit's. An element with no such parameter, a cabinet or a curtain wall,
  gets none, as in Revit's export.

- **A body drawn from its saved mesh of flat faces gives its `NetVolume`, as
  Revit's export writes it (RE-387).** Revit's export takes a curtain mullion's,
  a curtain panel's and a column's `NetVolume` from the volume of its solid,
  and the mesh Revit saved for it is that solid, so where rvt-rs draws the
  body from the mesh (the default since #1191), every face of it is flat and it
  closes, the volume of the mesh is the quantity. On the BIM4LCA office it is
  Revit's on all 9,678 mullions and 260 panels drawn so as built (9,706 and 269
  at building permit), 1,883 and 260 of which had none. A body with a curved
  face, whose mesh is a tessellation, and an entity not measured get none; a
  column's `GrossVolume` stays out, since Revit omits it on 18 columns at
  permit that nothing tells from the 6 it writes it on; and `NetSurfaceArea`
  stays out: Revit leaves out the end faces of a mullion that its joins hide.

- **A wall's `NetVolume`, `NetSideArea` and, where its two sides agree,
  `GrossSideArea` and `GrossVolume` come from the mesh Revit saved for it, as
  Revit's export writes them (RE-388).** Revit's export takes them from the
  wall's solid, joined to the walls beside it and cut by what it hosts, and the
  saved mesh is that solid: its volume is `NetVolume`, the area of its largest
  vertical face less its holes `NetSideArea`, the area inside the outer loop of
  that face `GrossSideArea`, and the wall type's width times it `GrossVolume`.
  A wall gets its volume where every face of its mesh is flat, the faces close
  into one piece wound one way, and the quantity is not already written by a
  rule of its own (RE-270, RE-293). The areas and the gross volume are given
  only of a wall with no more than four vertical faces, its sides and ends,
  because Revit's export writes only a volume of some walls of a more complex
  plan (26 of the office's 581 with quantities, each with five or more) and
  nothing in the mesh tells them from the rest; and a gross area only where
  the wall's two sides face opposite ways and have the same outer-loop area,
  since Revit's choice between two sides that differ is not read. A wall with a
  curved face, a mesh that does not close, or two pieces gets none.

- **A curtain panel drawn from its saved mesh as a vertical box carries its
  `Width`, as Revit's export writes it (RE-396).** Revit's export gives a curtain
  panel `Qto_PlateBaseQuantities.Width` the longer of its horizontal sides, and
  gives none to a panel whose body is not a box. Where rvt-rs draws a panel from
  the mesh Revit saved for it (the default since #1191) and that mesh is a
  closed box of flat faces with its smallest side horizontal, the longer of the
  two horizontal extents is the quantity: Revit's on all 257 panels of the
  BIM4LCA office as built (266 at building permit), none of which had one, and
  the 3 panels whose mesh is not a box, which Revit gives none, get none.

- **A curtain mullion's `NetSurfaceArea` is written on 926 more mullions of the
  BIM4LCA office as built, 924 at permit (RE-400, #1046).** RE-268 left out a
  mullion with another lying over one of its ends, and every neighbour of a
  mullion shorter than its own section, because its reading was wrong on 7 of
  the 1,189 it left out. The reading is Revit's on the 159 mullions with another
  lying over an end and on the 767 that meet only a short mullion, none
  different, on 40 and 106 of the office's 220 curtain walls; the 263 whose
  count still gets 7 wrong stay out. The rule rests on the office, the one model
  whose Revit export writes the quantity.

- **Saved meshes replace a family instance's turned box, and decode
  ellipses, curved ruled surfaces and rings that repeat their first point
  (RE-385).** A family instance drawn as its record box turned to its angle
  (RE-87) is now replaced by its saved mesh, as a record box is. The
  saved-graphics reader reads a `GEllipse` as a surface profile, a
  `RuledSurf` between arcs and ellipses as well as lines, and a trim ring
  that repeats its first point (a conduit's end cap). On Snowdon Towers
  Electrical (Revit 2024 and 2025) 18 of the 2,428 elements stay boxes,
  against 300; 16 of them are art-glass pendant drops whose nested instances
  are forbidden the targets 10, which stay refused. Architectural draws
  2,508 bodies from saved meshes, against 2,377. Every mesh these changes
  add fits inside the box it replaces, on the 13 models probed. The default
  export takes up to 17% longer where it draws more meshes (Snowdon Towers
  Electrical 2024, 11.7 s to 13.7 s).

### Changed

- **A wall's shadow cutout step is read as the step it mirrors (#1165, RE-398).**
  A `WallShadowCutoutGStep` names the wall whose record holds the cut
  (`m_otherWallId`) and that wall's step (`m_ggId` is its `m_id`), of the same
  cutter. All 124 of the BIM4LCA office's, `rac_advanced`'s (2024 to 2027) and
  Snowdon Towers Architectural's do: 12 mirror a curtain wall's embed and 112 a
  door's, window's, other family's or opening's cutout. The export reads each
  as that step, so a cut it left out as `shadow_cutout_step_not_read` is
  counted under the reason of the step it mirrors (the office as built:
  `curtain_wall_within_half_foot_of_wall_end` 41 to 42 and
  `door_opening_box_not_read` 1 to 2). No export changes: the opening in a
  shadow's wall is not written until the mirrored step's own rule writes it.
  The link between a shadow step and the step it mirrors was reported by mahach666
  (RvtIO, discussion #1241).

- **Every element that would be a bounding box in a Revit 2023 to 2027 file is
  drawn from the mesh Revit saved for it by default, where all of the mesh
  decodes, in `rvt-ifc`, `rvt-gltf`, Python, the MCP server and the viewer;
  `--no-saved-meshes` keeps the boxes (#1191).** Doors and windows alone were
  until now, and furniture, fixtures, equipment, MEP devices and the rest took
  `--saved-meshes`. The BIM4LCA office as built draws 11,094 of its 11,247 boxed
  bodies from their meshes, 672,298 triangles against 257,108, and its IFC goes
  from 68 MB to 87 MB. **Cost:** reading every box's saved graphics takes 1.1 to 1.9
  times the time and 1.4 to 4.4 times the peak memory of 0.17.0 on Autodesk's
  samples of Revit 2023 to 2027 (Measure run 42956917709, the median of three
  exports on one runner per file): the office as built goes from 25.4 s and 914
  MiB to 33.0 s and 1,432 MiB, Snowdon Towers Architectural from 17.6 s and
  1,073 MiB to 28.0 s and 2,086 MiB (its IFC 189 MB against 37), Core Interior
  from 4.7 s and 305 MiB to 5.3 s and 437 MiB, and RE1 Electrical from 1.9 s and
  165 MiB to 2.1 s and 264 MiB; files of Revit 2015 to 2022 are not offered the
  pass and take at most 2% more time (#1694). In the
  viewer's Playwright measurement (Deploy viewer run 42957158734 on `main` at
  979ec428, one open each, the release WASM) the wasm memory with the meshes on
  is the boxes' on Einhoven (17 MiB), RE1 Mechanical (68) and Plumbing (54), 75
  MiB more on Core Interior (308 to 383), 21 MiB more on RE1 Architecture (96 to
  118) and 52 MiB more on RE1 Electrical (161 to 213), and the wasm open call
  takes at most 0.40 s longer (RE1 Architecture; Core Interior's took 1.8 s
  less). **Not every
  mesh is Revit's:** of the bodies scored on 31 models (Measure run
  42612357300) 17 classes have every vertex of Revit's export within 0.001 ft
  of the mesh, but doors are 675 of 681, sanitary terminals 68 of 94, flow
  fittings 71 of 89, flow terminals 30 of 35 and walls 14 of 18. No roof, beam,
  slab, footing or stair flight mesh is among the bodies scored on those 31
  models: on Autodesk's samples the mesh of one roof of the 2026 and 2027 saves
  is not the 2025 save's body (the same-project gate names it; RE-389), and
  `rac_advanced`'s eight beams are held to its 2024 save. A mesh with a face
  that does not decode still keeps its box. **A very large file may need
  more memory than before:** it depends on how many boxed elements the file
  holds, and none over 1.4 GiB has been measured. The opt-out in each interface
  returns the previous memory: `--no-saved-meshes` in `rvt-ifc` and `rvt-gltf`,
  `saved_meshes=False` in Python, `saved_meshes: "off"` in the MCP server,
  `ExportOptions::with_saved_meshes(SavedMeshChoice::Off)` in the Rust library,
  `savedMeshes: false` in the wasm binding, and the viewer's option or
  `?savedMeshes=0`. `SavedMeshChoice::DoorsAndWindows` (new) and the MCP's
  `saved_meshes: "doors_and_windows"` give the previous default. **For library
  callers** `SavedMeshChoice::Default` and `saved_meshes::default_scope` now mean
  every boxed element, not doors and windows alone.

- **An `IfcPlateType` is written with no `Tag` (#1594).** Revit's export gives a
  curtain panel's type object the `Tag` of one of its plates, the first of its
  group in an order of the curtain grid that no record holds (RE-367 section
  4a; on Vitoria's `ALD26 Before` the 67 plates make 5 types and each `Tag` is
  the first plate of its group). rvt-rs wrote the panel type's own ElementId,
  which is not Revit's `Tag` on any of them, so the 269 plates of the BIM4LCA
  office at building permit (260 as built) each disagreed. The type's own
  ElementId is still its `Type Id` property and the MCP server's `element_id`
  for it; members and furniture, whose `Tag`s agree, are unchanged.

- **A roof drawn as the level block of its sketch outline is drawn from its saved
  mesh on Revit 2023 to 2027 too, where the mesh decodes whole (#1612, RE-389).**
  The block is the outline extruded over the record box's full height, which holds
  the roof's reference surface and not its slope or thickness; the mesh is what
  Revit drew. Against Revit's own exports, Snowdon Towers Architectural's 10 roofs
  that move (local only) are within 0.4% of Revit's volume and 0.01 ft of its box,
  where 7 of them were 40% to 125% too large as blocks, and the roof of the T and
  U footprints of puzzbobb's Revit 2025 models goes from 37,357 ft3 to Revit's
  1,845.6. The same project's saves now agree: rac_advanced's three 15 degree
  gables are meshes on its 2025 save as on 2026 and 2027. A roof whose mesh does
  not decode whole keeps its block, and a resolved outline stays the body of every
  other class. The outline's own properties (`ProfileSource`, `ProfileVertexCount`,
  `ProfileVoidCount`) and the `SlabShapeRefusal` of a body replaced by its mesh are
  left out, and `ProfileResolved` reads false. **Not measured:** the Autodesk
  samples' roofs have no Revit export, and on 3 held-out roofs two scorers
  disagree (RE-389 section 4). `--no-saved-meshes` keeps the blocks.

- **A railing that belongs to a stair is aggregated under it, as Revit's export
  aggregates it (#1624).** Revit's own IFC4 exports put each railing hosted on a
  stair among the parts of its `IfcStair`, beside its runs and stringers: 76 of
  Snowdon Towers' 142 railings (Revit 2024, local only) and all 10 of the R2025
  stair models' 13 to 17 (Revit 2025). rvt-rs wrote them standalone, so the body
  of the stair, which has none of its own and is the sum of its parts, was short
  by its railings: 2.74 ft of height and 0.72 ft3 of volume on R2025 model 13. A
  railing joins the stair its record names (`BaseRailing.m_stairsId`, the field
  RE-254 reads a stair railing's length from) on Revit 2024 to 2027, where a run,
  landing or stringer already makes that stair a whole; a stair no part names
  keeps its own body. A railing whose record names a component of the stair
  (`m_stairsComponentId`, a run) stays a plain element, as in Revit's export
  (RE-390): one of the BIM4LCA office's 26 stair railings, Tag 2598405, which
  Revit's export puts on Level_01 while its stair is in the Basement. The
  stair's height now matches Revit's, but its volume as the sum of its parts is
  larger than Revit's, 2 to 127 times on the office's 13 stairs that hold
  railings, because a railing's body is still its record box where its saved mesh is not
  whole (RE-303); a viewer drawing the stair's parts shows the same railing boxes
  as before.

- **A `GGTag` in a body's saved graphics is walked as a group of one point
  (#1630, RE-401).** It is a point with a test point for selecting its group, no
  surface, and it kept 4 of Snowdon Towers Architectural's roofs the level block of
  their sketch outline. They are drawn from their saved meshes now, each Revit's
  volume to 1.5e-7 with every vertex of Revit's body within 0.000006 ft of its
  own, as are 2 slabs of the same project (within 0.009% of Revit's volume) and 1
  roof of each of `rac_advanced`'s five saves, 2023 to 2027, which have no Revit
  export. Snowdon's Revit export is local only, so Measure holds the roofs by
  the same-project gates alone. No other export moves on any of the 118 models
  Measure reads. `--no-saved-meshes` keeps the blocks.

- **A railing is drawn from its saved mesh where its saved graphics are whole
  (#1635, RE-397).** A railing's saved graphics name its parts as instances of
  other elements. The walk skipped the instances flagged hidden in the normal
  draw and forbidden the targets 53246, which hold up to 59% of a railing's
  body, and refused a flip control, which is a grip with no surface. Both are
  read now. On the BIM4LCA office 15 of the 41 railings are drawn from their
  saved meshes (their bounding boxes within 0.01 ft of Revit's export's), as are
  2 on each of the four Revit 2025 stair models and 1 on RE1 Architecture; the
  four stairs, which aggregate their railings, go from 2.6 to 3.0 times Revit's
  volume to Revit's (the stair volume RE-390 disclosed). Two of the office's railings, the `KAI-3`, are 19.5% over
  Revit's volume, all of it in their posts: the saved graphics hold a round
  post, and Revit's export draws it as a quadrilateral inscribed in that circle,
  the difference of the hexagon it draws on the 30 mm rails of the BIM4LCA
  housing, so the volume reads over where every vertex is within 0.01 ft of
  Revit's surface. 224 bodies on 19 of
  Autodesk's samples change from a record box in the same way, with no Revit
  export to score them. 26 railings of the office keep their boxes: the 22 stair
  railings, and 4 whose faces the parametric reader refuses.

- **Four more "Export to IFC As" values are acted on (#1636, RE-417).** On each
  of the four large files of the BIM4LCA housing, 42 `Turning Free Space`
  generic models carry `IfcSpaceType`, 3 generic models `IfcFurniture`, 1 roof
  `IfcSolarDevice` and 1 piece of specialty equipment `IfcTransportElement`, and
  Revit's export writes them as such where rvt-rs wrote proxies and a roof. They
  are written as `IfcSpace` (`INTERNAL`, drawn from their saved meshes),
  `IfcFurniture` (`USERDEFINED`), `IfcSolarDevice` (`SOLARPANEL`) and
  `IfcTransportElement` (`ELEVATOR`), each value limited to the class it was
  seen on, and a value that names an entity's type object no longer counts as
  one the export does not apply. 122 of the 126 models measured are
  byte-identical; the values rvt-rs writes that Revit omits fall from 163 to 168
  to 27 to 32 on each housing file.

- **Every box the saved-mesh pass leaves says why, also one it did not look
  at (#1645).** An element drawn as a record box, a beam's box or a turned box
  that the pass did not take as a candidate (a stair or curtain wall whose
  parts draw it, an element with no location or no ElementId, a class outside
  the scope a caller asked for) now has `not_a_candidate: ` and the reason as its
  `SavedMeshRefusal`, so that `exported.saved_mesh_refusals` counts every such
  box and not only the candidates whose saved graphics were read. No body is
  drawn differently.

- **Every record box, beam box and turned box says why, on every release (#1645).**
  An element drawn as a record box, a beam's box or a turned box that no saved-mesh pass was offered (a
  Revit 2008 to 2022 file, whose saved graphics are not read; a caller that
  turned the pass off; a file whose release could not be read; a pass that
  stopped on an error) now has `not_offered: ` and the reason
  (`release_outside_2023_to_2027`, `saved_meshes_off`, `release_not_read`,
  `pass_failed`) as its `SavedMeshRefusal`, so that
  `exported.saved_mesh_refusals` counts every such box on every release. A mullion
  or panel turned or tilted off the model's axes and drawn as its oriented box
  (RE-106) is not offered the pass and has no reason yet (#1689). No body is
  drawn differently, and the files of Revit 2023 to 2027 do not change.

- **An instance visible in the normal draw and forbidden the targets 10 is drawn
  (#1645, RE-408).** The saved-graphics walk refused any `GInstance` whose
  `m_forbiddenTarget.m_targets` was not 0, which kept the appliances, chairs, sinks
  and pendant lamps that nest one as record boxes. The pair of visibility 0 and
  targets 10 is drawn now, and every other mask stays refused. On Autodesk's
  Snowdon Towers Architectural 332 bodies come from their saved meshes, all in
  Revit's own export, 301 of them within 0.01 ft of it both ways and within 2% of its
  volume (the 16 art-glass pendants read 1.04 to 1.10 of its volume with every
  vertex within 0.0093 ft, the 15 kitchen sinks 0.0106 ft from its surface), and 16
  more on Snowdon Towers Electrical, which has no export to score them. Six
  of puzzbobb's Revit 2025 models gain nine window trims and muntins each (54
  bodies), all in Revit's export at volume 1.0000 with every vertex within 2.03e-6 ft
  of it both ways. The housing's showers meet the next refusal and stay boxes.

### Fixed

- **A round column no longer carries a square's quantities (#1203, RE-416).**
  24 round columns of Autodesk's `rac_advanced` sample (450 mm, 6 in each save
  of Revit 2024 to 2027) hold a type whose one length is `b`, which the column
  quantity pass read as a square section, so each was written with the
  `CrossSectionArea`, `OuterSurfaceArea`, `GrossVolume` and `NetVolume` of the
  square that bounds it, 4/pi of the cylinder's. A column drawn from its saved
  mesh now keeps those four only where its box's volume is within 2% of the
  mesh's, and keeps its `Length`. No Revit export holds those saves to score the
  24 against; every model that has one is byte-identical.

- **A wall with an edited elevation profile carries the openings Revit's export
  writes for it, the pockets of the profile against its convex hull, and the
  rest of its profile's cut is made in its body (#1607, RE-380).** RE-151 read
  the profile as the box less one opening for each run of its lines that leaves
  its bounding rectangle. That matched Revit's export on Core Interior's wall
  55840 and wrote an opening Revit does not write on ten more walls: RE-358's
  eight walls of the BIM4LCA office, in both office files, and both of its
  terrain files' walls, one of which, 1387731, has an opening in Revit's export
  that is the first three lines of its top closed by a chord. Revit writes an
  opening only where the profile has a pocket against its convex hull, none on a
  straight sloped top, and tags it with the first line of the pocket's stretch
  (12 of the 15 pockets measured; the last line on the 3 of one Snowdon Towers
  wall, which nothing read explains). A pocket is closed by the hull's edge, or
  through the profile rectangle's corner where the edge cuts it off, as on Core
  Interior's wall 55840 (301.65 ft2); a pocket whose edge ends otherwise is not
  written. A region of RE-151 that a pocket equals is no longer an opening of
  its own: the pocket is written in its place and the wall keeps its body. Any
  other region is taken out of the wall's body as one boolean difference
  (`ProfileCutSource` `wall_profile_cut_in_body`, counted in
  `exported.wall_profile_cuts`), so the shape is the one it had and the openings
  are Revit's. A wall the cut cannot be made for keeps its regions as its
  openings, as before.

- **Every type object has a GlobalId of its own (#1667).** A type read for
  several symbols can leave several type objects of one entity and Tag that all
  take the writer's fallback GlobalId. The BIM4LCA office at permit (timber)
  wrote one mullion type as 7 `IfcMemberType` with one GlobalId, and their
  property sets and `IfcRelDefinesByType` repeated it too: 24 `IfcRoot.UR1`
  findings in IfcOpenShell's validation, one for each repeat. The first of them
  keeps the fallback and each later one takes a GlobalId that carries its place
  in the list.

- **The saved-mesh pass no longer holds every wall's graph at once (#1690).** It
  read every owner's graph first and held all of them until the last batch was
  meshed, and since RE-388 the owners include every wall, whose graph is about
  1.6 MiB (#1429): the default export of a Revit 2023 to 2027 file took 1.4 to 4.4
  times the peak memory of 0.17.0 (Core Interior 305 to 647 MiB, Snowdon Towers
  Architectural 1,072 to 3,964). The owners that name nothing, walls among them,
  are now let go after that first read and read again in one further pass that
  meshes a buffer of about 20,000 graph objects at a time. No body changes: all
  126 exports of Measure's models are byte-identical to what they were. The peak
  falls to 437 MiB on Core Interior, 1,432 on the BIM4LCA office as built and
  2,086 on Snowdon Towers Architectural, at 1% to 3% more time; it is still 1.4,
  1.6 and 1.9 times 0.17.0's, and 4.4 times on the small samples of Revit 2023 to
  2027 (`rac_basic` 2023: 200 to 875 MiB), which this did not move (#1694).

- **`rvt-ifc --mode typed-no-geometry` keeps the `PitchAngle` of a floor drawn
  along its shape-edited plane (RE-384).** The geometry modes write it
  (`atan` of the plane's gradient), and a level slab keeps its 0 in the mode
  without geometry because the rule is read before the mode strips the body;
  the sloped floor's was read after, so the mode left it out. Revit's export
  writes the angle whatever the body.

- **A curtain mullion's box gives its `NetVolume` and `NetSurfaceArea` only
  where Revit's solid is that box (RE-403, #1046).** A mullion whose box volume
  differs from its saved mesh's, as a mullion with a 45 degree end does, takes
  the mesh's volume, which is Revit's, and loses its `NetSurfaceArea`; an
  overlapped mullion is left out where a mullion with a box that is not its
  solid (a section that most boxes of its type do not have) lies over one of its
  ends. RE-400 had written 24 wrong values on the BIM4LCA timber office at
  building permit.

- **A wall's box gives its quantities and its `Height` only where Revit's solid
  is that box (RE-405, #1636).** A wall whose saved mesh is whole and has
  another volume than its box (a wall with an angled end, a box longer than the
  wall) loses the quantities taken from the box and takes the mesh's
  `NetVolume` and, of a wall of no more than four vertical faces, its areas. A
  wall whose type has several layers is given none of its box's quantities and
  no `GrossFootprintArea`, which Revit's export omits on it, and takes the
  mesh's where that is whole. Its `Height` is written where the mesh's vertical
  span confirms the extrusion's height, and dropped where it does not, as on a
  wall whose lower part is cut away. On the BIM4LCA housing every wall `Height`
  that differed from Revit's is gone and several hundred are written.

- **A roof's sketch holes stay in its `GrossArea` and `GrossVolume` (RE-413,
  #1636).** A flat roof Revit's export writes as a slab, drawn from a sketch with
  holes, gets its outline's area for `Qto_SlabBaseQuantities.GrossArea` and that
  area times its depth for `GrossVolume`, as Revit's export does: it writes the
  holes as openings and takes them off its `NetVolume` alone. A floor's holes are
  still taken off its `GrossArea`, which is Revit's on the 8 floors with holes of
  the BIM4LCA housing as built. The change moves one roof, in each of the four
  large saves of that project, whose two quantities were 0.9% to 2.9% under
  Revit's and are equal now; of 126 models no other export moves, so no second
  project exercises the rule.

- **No false RE-354 warning for a family's type table (RE-395).** A family
  type table's Fire Rating and Manufacturer entries in a `Family` element's
  block are the rows of the family's own type table, not a type's value,
  and rows that differ are not a disagreement. They are now left out, and the export
  no longer warns that a type text parameter is left out on Core Interior,
  Snowdon Towers Architectural (2024 and 2025) or Golden Nugget Architektur,
  where every such warning was a `Family`. No IFC changes on the 32 projects
  measured.

### Research

- **Rectangular family revolutions (#156, RE-391):** measure three authored Revit 2020 fixtures:
the saved axis marker, profile placement and angle parameters rebuild all
three, and all three pass the strict native IFC4 Design Transfer comparison.
A controlled export-settings check resolves the initial coarse full-ring
reference without changing the rebuild or tolerance. Adds a probe and reproducible
authoring/scoring tools, without changing production export behaviour.

- **The mesh body scorer reads a round section's volume and the faces Revit's
  export writes twice (#1658, RE-397).** `mesh_bodies_vs_ifc.py` read a volume
  ratio outside 1% as a difference where none is: Revit's export draws a round
  section as a polygon inscribed in the circle a saved mesh draws with more
  segments (a ratio of 1.1989 for a hexagon on the BIM4LCA housing's 30 mm rails,
  1.60 for the quadrilateral on the office's `KAI-3` posts, 0.9643 where rvt-rs's
  circle has the fewer segments), and it writes some bodies with every face
  twice in the same winding, so that their volume is twice the body's (the roofs
  and ramp flights that read exactly 0.5000). A body outside 1% is now read
  shell by shell (`volume_readings.py`): a shell is `explained` only where
  Revit's end faces are polygons inscribed in rvt-rs's circle (to 0.001 ft) and
  its volume ratio is their area ratio, and `doubled` only where its volume is
  rvt-rs's once Revit's repeated triangles count once and every vertex of each
  lies within 0.01 ft of the other's surface; every other difference stays
  `other`. The class table is unchanged; a `shell readings:` line and `reading`
  rows follow it. On the BIM4LCA files, all 12 and 16 outlying railings of the
  housing, the office's `KAI-3` and `KAI-5` railings and every roof and ramp
  flight at 0.5000 or 0.5010 are explained; the housing's ramp flight, the
  doors with a shell Revit's export lacks and the walls stay `other`.

- **Revit's body keeps a profile wall's top line on to the wall's end, measured
  at its stub ends (RE-383, #1473).** On the BIM4LCA office's two files the
  highest point of Revit's body at the end of the 10 stubs of 9 walls is the
  top edge of the wall's sketch loop carried on to the end, to 0.0001 ft, and
  rvt-rs's is Revit's to 0.0000 ft (RE-358's rule); on 4 stub ends of 3 walls of
  Snowdon Towers Architectural, checked against Revit's own export locally, the
  same. No byte of the file states the rule: its inputs are the sketch's lines
  and the record's box. The report names what no file with a Revit export can
  test (a stretch longer than the wall is thick, an arc, an angled wall).

- **Nothing in Revit's own bodies or in the saved graphics tells which walls and
  mullions Revit leaves quantities off (RE-399, #1046).** On the BIM4LCA
  office's two files Revit writes a `NetVolume` alone on 28 of 607 walls as
  built (24 of 589 at permit), and the body Revit draws a wall with does not
  separate them from the walls with the full set: 304 of the 331 walls with a
  face-set body have it, and so do 275 of the 276 extrusions. The saved graphics
  of the 10,003 curtain mullions flag no primitive hidden or filtered, so they
  say nothing of the end faces Revit's `NetSurfaceArea` leaves out. For columns
  Revit's body and its quantity set agree on every column, which is RE-291
  read from Revit's side. The report names what stays unmeasured; nothing
  rvt-rs writes changes.

- **The first measurement of the shipped rules on a second project, the
  BIM4LCA housing (RE-402, #1636).** Of the 15 census lines the pre-registration
  reads, 5 hold on the housing where they are exercised, 9 do not and 1 is not
  exercised; on the concrete office the rules were developed on 13 hold and 2
  do not. Every wrong mullion value on the housing is one type, 0.005429 ft²
  above Revit's whatever its ends. RE-400 writes 24 wrong `NetSurfaceArea`
  values on the timber office at permit, a file it had not been measured on;
  its repair follows. The roof and duct scorers misread a project that Revit's
  export rotates and are repaired. Nothing rvt-rs writes changes.

- **A cylinder face trimmed by several loops, measured again: still negative for
  the furniture (#1645, RE-407).** RE-305's mesher, rebased onto `main`, draws the
  30 `Stay_Wall_Mounted_` furniture of the BIM4LCA office at 1.11 times Revit's
  volume with vertices up to 0.0118 ft from Revit's surface, as in RE-305, and the
  shell reading of #1658 (inscribed round sections, faces written twice) explains
  none of them; the eight sauna doors of the housing it also draws read 1.0004 and
  within 0.0053 ft. It is not shipped. The report records the loops of the 175
  faces with several on Autodesk's 2024 samples and what was ruled out.

- **A door's saved mesh, tried for its `OverallHeight` (#1636, RE-409).** Where
  no cut loop gives a door's size, the height of its saved mesh is Revit's
  `OverallHeight` on the 65 doors of the BIM4LCA timber office at permit whose
  written height is 15 or 45 mm over it, and a rule that takes it makes those 65
  equal. It also turns the one door of RE1 Architecture's export that it reaches,
  equal before, 1.9 mm short, and the housing exercises it on no door, so nothing
  ships. `tools/re/door_window_mesh_extents_vs_ifc.py` reads, for each door and
  window, the size written and the mesh's extents against Revit's.

- **RvtIO's three weld passes, tried on the edges of 3,759 rooms' solids (#90,
  RE-410).** Welding the ends of a room solid's edges at 0.000523 ft, RvtIO's
  second pass, closes the faces that can be placed of 3,748 of 3,759 solids on 100
  models, against 1,829 with no weld, and of 6 of the 7 Snowdon Towers rooms #90
  names; its third, 1/32 in, settles 8 more, all on the BIM4LCA office, and none of
  the seven. Room 2181762 and two of the office's terrain rooms close at no
  tolerance. Nothing is shipped: the reader of RE-374 refuses its rooms for the
  prism height, an unread face and floor lines that do not close, not for a weld.
  RvtIO's maintainer offered the passes on discussion #1241.

- **RvtIO's round-column rule, run beside RE-353's on 6,672 columns (#1203,
  RE-411).** On the saved meshes of 100 models RvtIO's cylinder test takes 1,902
  columns as round and RE-353's caption rule 6, with 4 in common: the BIM4LCA
  office's one round column on each of its four files. Of the 1,898 only the
  cylinder test takes, 1,048 have a type with German captions (`Durchmesser`,
  `Radius`), 680 a type whose one length is `b`, which RE-291 reads as a square
  section, and 170 a type whose lengths are not read (Revit 2023). 24 of the 680,
  in Autodesk's rac_advanced sample, carry the areas and volumes of that square,
  4/pi of the circle's: RE-416 repairs them. RvtIO's maintainer offered the rule
  on discussion #1241.

- **A flagged `GInstance` with a rotation matrix draws none of the 32 showers it
  was meant for (#1645, RE-414).** The traversal refuses a `GInstance` whose
  `m_bHasScale` is set, which kept the BIM4LCA housing's 22 showers and the
  office's 10 as boxes. Walking one whose matrix is a rotation (111 of the 112 in
  those files are the identity, one a quarter turn; no other model of the Measure
  set holds any) reaches the showers' nested symbols, which hold faces of three
  kinds the mesher refuses (multiple cylinder trim loops, nonrectangular parametric
  trims, HermiteSurf), so every shower stays a box and no body moves. The flag was
  not what blocked them; the rule is not shipped.

- **A sloped roof's volume in Revit's export is its saved mesh's times the cosine
  of its slope (#1656, RE-415).** The 0.2% by which the BIM4LCA housing's roof
  1627602 read over Revit's is not a face or a shell: its surface and Revit's coincide
  to 7e-5 ft, and Revit's export writes the roof type's layers (24 and 10 mm) along the
  vertical where the saved mesh has them perpendicular to the slope (34.0000 mm),
  so its body is thinner by the cosine of the slope, 3.6436 degrees. The same holds
  on the 7 sloped roofs of Snowdon Towers Architectural (0.99939, the cosine of 2
  degrees) and the 3 flat ones match to 1e-6. Nothing in the reader changes.

- **Why Snowdon Towers Architectural's roofs stay level blocks (#1630, RE-420).**
  Six of its 20 roofs keep their block, each with exactly one decoder diagnostic
  and no mesh (four `malformed ruled-surface edge is connected to face #`, a kind
  no other class shows; one `shared curved edge changes its fixed parameter`; one
  `multiple cylinder trim loops unsupported`), 33% to 133% over Revit's volume,
  and the other fourteen are drawn from their saved meshes and read within
  0.061% of it (the cosine of a 2.0 degree slope on seven, RE-415). The six
  decoders are #1683. Nothing in the reader changes.

## [0.17.0] — 2026-11-09

0.17.0 reads more of Revit 2015 to 2023 and corrects what the export claims. On
Revit 2015 to 2023 projects curtain walls are `IfcCurtainWall` with their
mullions and panels under them, along their own axes, with their type, family and
material, `Span`, `Length` and `IsExternal`; steel beams of 2017 to 2023 are
drawn as their type's I section; walls write their `Height` where their record
names nothing joined to or cutting them; and ducts of Revit 2015 to 2027 take
their type, section, line and ports from their own native record, with a port's
`SystemType` and its joins. On Revit 2024 a room whose stored solid neither the
walk nor the span closes takes its floor's outline from the solid's edges, a
wall whose floor covers all of its plan but a stub inside another wall is cut
whole, and a wall whose edited profile closes below its record box ends at the
profile's highest point with the profile's `Height`. A Revit 2026 or 2027 save is
written without an `ObjectType`, as Revit's own export of those releases writes
none. Two claims the export made on every file are now made only where they hold:
the missing host join of the 2024 opening index, whose rows were never
openings, and the compound layers that host elements were said to lack.
puzzbobb wrote the Revit 2015 to 2023 curtain walls, parts and beam sections, the
duct and port reads (#1518, #1519, #1520, #1524) and the correction of those two
claims (#1598); thanks.

Not measured, and said in each entry: no curtain wall, curtain part, steel beam,
duct or wall `Height` of Revit 2015 to 2023 on Autodesk's samples is scored
against a Revit IFC4 export. They are held to the same projects' 2024 saves,
which cannot catch an error those saves share, and the curtain walls and parts
are also scored against Revit's IFC2X3 exports of other projects, counted apart.
The wall stub rule rests on one wall of the BIM4LCA office (3004978) and the
profile-top rule on one wall of its terrain files (1387731), and each is
labelled and counted apart (`TopCoverStub`, `ProfileTopSource`). The Revit 2026
and 2027 `ObjectType` is Revit's on the 418 typed elements of nine classes of the
five held-out pairs saved by those releases (counts only). A panel's material from
the Curtain Panels category rests on one project, Vitoria's, in two saves; on
Revit 2024 and later no model has a category that holds one, so which Revit's
exporter prefers there is not measured.

Measured against 0.16.1 on the reference models, the BIM4LCA office, the
Revit 2025 models, Autodesk's sample projects of every release, the held-out
models, `rac_advanced`'s IFC2X3 pairs and BIMcollab's model (Measure run
[42632958286](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42632958286),
`main` at 52282ea1, 118 exports; the release adds only its notes, witness files
and version strings to it):

- 77 of the 118 exports move. The 41 that do not are Core Interior, Einhoven,
  RE1 Architecture and Electrical, all 24 Revit 2025 models, six held-out
  models, and Autodesk's `rst_advanced` 2015, `rac_basic` 2024 and 2025,
  `rst_basic` 2024 and 2025 and Snowdon Towers' electrical and structural
  models.
- On the BIM4LCA office the property values and quantities are as in 0.16.1,
  as built and at permit: equal, different and missing are unchanged. The port
  `SystemType` equal to Revit's goes from 0 to 24 of 67 on each file (heating),
  its wall 3004978 as built has Revit's top (606 to 607 of 607 walls with
  Revit's base and top), and the one room that kept its box at permit takes its
  floor's outline from its solid's edges, at Revit's area. On its two terrain
  files the wall `Height` equal to Revit's goes from 1 to 2 of 2, the quantities
  equal from 15 to 16 on each and "different" from 1 to 0 and from 2 to 1 (the
  remaining one is a space's `NetVolume`).
- On RE1 Mechanical the bodies still drawn as their record box fall from 68 to
  43: its 25 ducts are their sections along their lines. On RE1 Plumbing the
  ports joined to a segment's port have Revit's Z axis on 118 of 118, from 108.
  On the 2026 and 2027 MEP samples the ports compared with the 2025 save go
  from 6,967 to 7,276 and from 7,054 to 7,364 with none different, and the
  properties compared from 5,140 to 5,158 and from 5,145 to 5,163, none
  different.
- On Revit 2018's IFC2X3 exports of the two Vitoria projects, counts only, the
  equal property values go from 2,737 to 3,494 on `ALD26 Before` and from 3,291
  to 4,048 on the held-out `ALD26 Project`, and the values rvt-rs writes that
  those exports do not from 65 to 55 and from 62 to 52. On `ALD26 Before` 5 of
  its 9 curtain walls are `IfcCurtainWall` (none before), its 206 members and 67
  plates have Revit's `Name` and `ObjectType`, and 442 elements have the same
  materials as Revit's, from 375. An IFC2X3 export is counted apart and removes
  no value.
- On `rac_advanced` 2015 to 2020 against the three.js IFC2X3 export, counts only,
  the equal property values go from 2,503 to 15,026 on each save and the missing
  from 13,605 to 1,078; "different" goes from 0 to 11, the mullions' `Span`
  (about 7.5 mm, counted apart), which were missing before.
- On the held-out models, counts only, Revit 2021's Warmond goes from 188 to 279
  equal quantities (the walls' `Height` on 2015 to 2023, #1535; 5 different as
  before), Revit 2023's `modelo_bim` from 7 to 8, and the Revit 2022 office
  pair's property values from 105 to 107.
- Time and peak memory of `rvt-ifc` against 0.16.1 (Measure run
  [42636959116](https://github.com/DrunkOnJava/rvt-rs/actions/runs/42636959116),
  both builds of a file in turn on one runner, three rounds): on the reference
  models and the Revit 2024 to 2027 sample projects (35 files) the time is 0.94
  to 1.05 times 0.16.1's, but for Einhoven, a 0.1 s file (106 to 123 ms), and the
  peak memory 0.98 to 1.07 times (the largest rise is Snowdon Towers' HVAC model,
  215 to 230 MiB). On the 33 sample projects of Revit 2015 to 2023, whose curtain
  wall, beam, duct and port reads the release widens, the time is 1.00 to 1.65
  times (`rac_advanced` 1.19 to 1.42, the 2015 and 2019 MEP samples 1.54 to 1.65,
  the 2015 `rme_advanced` from 1.96 s to 3.08 s) and the peak memory 1.00 to 1.12
  times (the 2015 `rme_advanced` from 329 to 370 MiB).
- No property value or quantity goes from equal to different or missing on any
  model, and "different" rises only by those 11. The IFC4 rules check finds
  nothing on any of the 93 exports that keep it, and the same-project gates pass
  (15 samples held to their earlier saves, and the Areas).

### Upgrading from 0.16

- Every export's IFC header names rvt-rs 0.17.0, so an export that is otherwise
  unchanged still differs from 0.16.1's there.
- A default export of a Revit 2015 to 2023 project can change: a curtain wall is
  an `IfcCurtainWall` with its mullions and panels aggregated under it and a
  door in none of them, its parts take their type, family and, mullions and
  panels, their material, a steel beam of 2017 to 2023 is its type's I section,
  a wall writes `Qto_WallBaseQuantities.Height` where its record names nothing
  joined to or cutting it, and a duct takes its type, section, line and ports.
- A default export of a Revit 2024 project can change: a room that kept its box
  takes the outline of its floor from its solid's edges
  (`SpaceOutlineSource` `stored_solid_edges`, `exported.edge_outline_spaces`),
  and a wall gains `TopCoverStub` or `ProfileTopSource` with a different top and
  `Height` (the BIM4LCA office's wall 3004978 and its terrain files' wall
  1387731 are the two measured).
- A Revit 2026 or 2027 export has no `ObjectType` on any element, where it had
  `Family:Type`; a `USERDEFINED` `PredefinedType` is then left unset. Revit 2025
  and earlier are unchanged.
- Ducts and ports of Revit 2015 to 2027: a port has the `SystemType` of its
  connector's system and an Exhaust Air port is `SOURCEANDSINK`; a join is
  written only where each of its connectors keeps the other, and the joins left
  out are counted in `exported.port_joins_left_out`.
- The export's claims: `iter_elements` no longer yields the
  `ArcWallRectOpening` rows of a Revit 2024 file (they were never written to the
  IFC; Core Interior's `production_class_counts` loses `ArcWallRectOpening` 3,777
  and `source_coverage.exported_element_fraction` goes from 0.20 to 0.92);
  `unsupported_features` no longer lists
  `opening_index_to_ifc_openingelement_host_join`, the export decisions no longer
  name `opening_index_without_host_geometry`, and
  `revit_compound_assemblies_and_walltype_widths` is listed only for a file with
  a host element written without its layers.
- Rust API (the research and IFC model types): `PartitionSchemaMvp::rect_openings`
  is removed (`unstable-research`); `native_connectors::port_flow_direction`
  takes the connector's system classification as a fourth argument and
  `ConnectorPorts::segments` maps to a `Calculation` (direction and system);
  `native_connectors::InstancePort` and `ifc::entities::IfcEntity::Port` have a
  `system_type` field; `ifc::entities::PropertyValue` has a `Ratio` variant, so
  an exhaustive `match` outside the crate needs an arm for it; `ifc::IfcModel`
  and `ifc::ExportedModelDiagnostics` have a `port_joins_left_out` field, which a
  struct literal outside the crate needs.
- New keys in `diagnostics.json` (`exported.wall_top_stubs_cut`,
  `exported.wall_profile_top_sources`, `exported.edge_outline_spaces`,
  `exported.port_joins_left_out`) and new markers in
  `RvtElementRecordGeometry` (`TopCoverStub`, `ProfileTopSource`); code that
  reads either handles a key or marker it does not know.

### Added

- **Revit 2015 to 2023 curtain mullions and panels have their type and family, and
  the mullions their material (#1190, RE-367).** On `rac_advanced` all 3,276
  mullions and 1,349 panels take the type and family of the 2024 save, and every
  mullion type the material its 2024 save's `m_renderStyleId` holds. Against
  Revit's own IFC2X3 exports (counted apart) the parts have Revit's `Name`,
  `ObjectType` and `Reference`: all 1,349 plates and 3,276 members of
  `rac_advanced` 2015 to 2020, the 67 plates and 206 members of Vitoria's `ALD26
  Before` and of the held-out `ALD26 Project`. A plate's type object carries the
  ElementId of a plate in Revit's export, not the type's, which is the exporter's
  convention. A panel takes the material of the Curtain Panels category's Object
  Styles where the category holds one, as Revit's export gives the 67 plates of
  `ALD26 Before` (RE-377), and its type's `m_renderStyleId` otherwise.

- **Revit 2015 to 2023 curtain walls are `IfcCurtainWall`, with their mullions and
  panels under them, along their own axes, with `Span`, `Length` and `IsExternal`
  (#1190, RE-366).** Over the 31 saves of 2015 to 2023 that have a 2024 save of
  the same project, the 511 walls this turns into curtain walls are all the 2024
  save's, and no mullion or plate lies under a wall that is not one; on
  `rac_advanced` all 34 curtain walls and, on 2023, all 4,661 of their parts'
  bodies are the 2024 save's. A door is set in no curtain wall. Against Revit's own
  exports, every one an IFC2X3 export (Measure run 42581222440, counts for the
  held-out model): `rac_advanced` 2015 to 2020 has 34 of 34 curtain walls and
  `Span` 3,241 equal, 11 different (about 7.5 mm, counted apart) and 24 missing of
  3,276; Vitoria's `ALD26 Before` has 5 of 9 curtain walls, the other 4 stay
  walls, and `Span` 206 of 206, and the held-out `ALD26 Project` 5 of 8 and 206 of
  206. **No curtain wall of these releases is scored against a Revit IFC4
  export**, and a later save cannot catch an error it shares.

- **Revit 2017 to 2023 steel beams are drawn as their type's I section (#1190,
  RE-365).** On `rst_basic` 2019 to 2023 all 323 beams drawn along their lines are
  now their 2024 save's, from 312. Revit 2015 and 2016 hold their sections under
  ids of the document's own, which are not read. **No beam of these releases is
  scored against a Revit export here**; each is held to the same Autodesk project's
  later save, which cannot catch an error that save shares.

- **Ducts have their type, section, line and ports from their own native record on Revit 2015 to 2027, ports their `SystemType`, and Exhaust Air ports their flow (#1190, #96, #834, #869, RE-371).**
  Every duct of the releases the native record path reads is typed (`Family`,
  `Type`, type `Reference`) and drawn as its section along its one line, at
  any slope, with its `Length`, its taps' ports and, where it is centred, its
  `Constraints` elevations, `Slope` and justification; a port has the
  `SystemType` its connector's system's classification names (`VENTILATION`,
  `EXHAUST`, `HEATING`), and an Exhaust Air port is `SOURCEANDSINK`. A port
  joined to another is placed with the axis Revit's export gives it however
  the two flow. On Revit 2015 to 2022 the family instances' ports are read
  and joined too; a join is written only where each of its connectors keeps
  the other, and the 10 joins of five duct taps that the records of the `rme`
  samples of 2015 and 2019 name in conflict are left out and counted in
  `exported.port_joins_left_out`. On Snowdon Towers HVAC (2025, local only) the record gives
  Revit's own type name, `Length`, section, body and `Constraints` values on
  1,053 of 1,053 ducts, and `SystemType` on every port whose system is
  named; on RE1 Mechanical the 25 ducts' type, length and body. **Flex ducts
  are not read, and no duct of 2015 to 2023, 2026 or 2027 is scored against
  a Revit export.**

- **Revit 2015 to 2023 walls have their `Height` where their record names
  nothing joined to them or cutting them, as on 2024 to 2027 (#1535, RE-364
  section 9).** That flag comes from the step classes of the wall's native
  record, which on Autodesk's samples of 2015 to 2023 read as on the same
  projects' 2024 saves: 2,894 of 2,896 walls hold the same steps, and all
  2,896 the same flags. Held to the 2024 save wall by wall, 781 `Height`s are
  equal and none differs; 7 are on walls the 2024 save does not have, and 10
  that the 2024 save writes are not written (curtain-gridded walls 573660 and
  573664 of `rme_basic` and `rme_advanced` 2015 and 2019, and two walls of
  `rac_basic` 2023 for which that save writes no quantity at all; neither
  cause is decoded). No Revit export of these samples exists, so an error the
  2024 save shares is not caught. Against Revit's own exports, `modelo_bim`
  (Revit 2023, held out) is 1 equal and none different, and Warmond (Revit
  2021, held out, its export passed through another tool) 91 equal and 45
  different. All 45 are walls Revit's export writes as several objects under
  one Tag, which the quantity scorer now counts. Side areas, footprint and volumes
  are still not written on these releases: on 25 walls whose record allows
  them, the 2024 save draws the wall from its centreline and writes none, and
  the record box's volume differs from that body's on all 203 of their old
  saves, by 0.003 to 3.55 m3; nothing these records give tells those walls
  apart.

- **A room whose stored solid neither the walk nor the span closes takes its
  floor's outline from the solid's edges, on Revit 2024 (#90, RE-374).** A
  room solid lists its faces and, before them, its edges. Each edge names
  its two faces and gives each face's (u, v) at two of its points, so a wall
  face that is not a rectangle gives its real end, which its uv box (all the
  walk and the span read) does not. The floor's edges, each drawn as the
  wall it meets, give the outline (`SpaceOutlineSource` `stored_solid_edges`,
  counted in `exported.edge_outline_spaces`).

  The BIM4LCA office's room 3095831 at building permit, which kept its box,
  is now 1864.737 ft² against the 1864.74 of Revit's IFC4 export, and
  Snowdon Towers' room 830902 304.059 ft², within 0.001 ft² of Revit's area
  for it (`Vim.Room.Area` in the model's later VIM edition, local only). On
  every room of Autodesk's samples,
  the office and the Revit 2025 models that the walk or the span already
  reads and this rule accepts, the edges give the same outline, to 0.01 ft²
  in area and 0.001 ft on its extent.

  It is read only where every edge's two planes give it the same ends and
  the solid holds no point between its floor and 4 ft above it except on a
  vertical edge, so that a raised floor, a step, a pocket or a low bulkhead
  refuses it. Snowdon Towers' room 2181757 shows why: its floor is 5172.857
  ft², its section 4 ft above the floor is 5177.493, and Revit's area for it
  (`Vim.Room.Area` in the model's later VIM edition) is within 0.001 ft² of
  that section and 4.6 ft² over the floor. It keeps its box.

### Changed

- **A wall whose floor covers all of its plan but a stub inside another wall is
  cut whole, as Revit cuts it (#1067, RE-375).** The BIM4LCA office's wall
  3004978 as built, whose roof leaves a 0.0853 ft stub of its end inside the
  wall it abuts, kept its full height: 9.24 ft over Revit's top and 14.94 ft³
  of body over. It is now cut at the roof's underside over its whole plan, as
  Revit's body is. It carries `TopCoverStub` (`inside_another_wall`) and is
  counted in `exported.wall_top_stubs_cut`. The rule rests on that one wall:
  what a floor leaves of a wall must be under 0.05 ft² and wholly inside the
  plans of other walls, and of the 225 pieces of Revit's wall plans outside a
  floor that RE-375 reads in five buildings the stub is the only one that is,
  while Revit keeps the wall's own height over most others.

- **A Revit 2026 or 2027 save is written without an `ObjectType` on any element
  (#1472).** Revit's own export of those releases writes none, where rvt-rs wrote
  `Family:Type`: of the five held-out pairs saved by 2026 and 2027, 418 typed
  elements of nine classes differed and Revit's export writes no `ObjectType`
  on any of them (counts only), and its export of the same projects saved by
  2025 writes `Family:Type` on every typed element, as rvt-rs does for 2025 and
  earlier. A `USERDEFINED` `PredefinedType` states its type in `ObjectType`, so where
  there is none the `PredefinedType` is left unset, as for any element whose
  `ObjectType` was not read (#1430). Revit 2025 and earlier are unchanged.

- **`tools/ci/seal-holdout-pair.sh` keeps one unopened copy of each sealed pair
  in a private repository, which the final 1.0 check will fall back to where a
  contributor's link no longer serves the file whose hash the row holds
  (#1565).** It uploads both files as `<sha256>.rvt` and `<sha256>.ifc`,
  checks each asset's digest against the hash it is named by, and only then
  deletes the local copies and adds the row, so no pair is listed without a
  copy. It refuses to upload unless that repository is private: the files
  embed Autodesk's template families, which no contributor's licence covers,
  so the copies are never made public or redistributed.
  `tools/ci/check_sealed_holdout.py` now also fails on any committed file
  that is not Markdown and names that repository. The final check is still
  to come, and its pull request decides which credential it reads the copies
  with.

### Fixed

- **A curtain mullion's or panel's material is its type's own, not what a type map
  gives (RE-367).** On Autodesk's `rac_advanced`, `rme_advanced` and `rme_basic` of
  2024 every mullion and panel was `Wood - Laminated`, glazed panels included. They
  now take `m_renderStyleId`, as `Glass`, `Stone - Granite` or
  `Metal - Aluminium, Black-Anodized`. The 20,989 parts scored against Revit's own
  exports are unchanged, and none of them is one that changes.

- **A curtain panel of Revit 2015 to 2023 takes the material of its category's
  Object Styles where the category holds one, and its type's otherwise (#1593,
  RE-377).** Revit's export gives the 67 plates of a Revit 2018 project the
  material of the Curtain Panels category's two graphics styles, not the panel
  type's `m_renderStyleId`, which is another material on all 67; on every other
  model with plates the category holds none and Revit gives the type's. The panels
  of those releases had no material since RE-367; they have Revit's on that
  project and, where the category holds none, the type's as on 2024 and later.
  Revit 2024 and later are unchanged: no model of those releases has a Curtain
  Panels category that holds a material, so which Revit's exporter prefers there is
  not measured.

- **A wall whose edited elevation profile closes below its record box ends at the
  profile's highest point, and its `Height` is the profile's (#1600, RE-378).**
  The BIM4LCA office's terrain files hold a wall (1387731) whose profile closes
  3.6765 ft below its box over its whole 173.9 ft: Revit's body ends at the
  profile's highest point, 196.5223 ft, where rvt-rs drew the box's 200.1988 ft,
  because its openings are cut from the profile's own bounding rectangle and left
  the strip up to the box solid, and it wrote the box's 5.520605 m as the wall's
  `Qto_WallBaseQuantities.Height` where Revit writes the profile's 4.4 m. The wall's
  body is now drawn to the profile's highest point, and it carries `ProfileTopSource`
  (`wall_top_at_profile_top`), counted in `exported.wall_profile_top_sources`. The
  rule rests on that one wall, so it is as narrow as the wall allows: the record
  names nothing joined to or cutting the wall, the profile spans the whole of it
  and starts at its base, and its highest point lies below the box. A wall that
  goes on past its profile keeps the box's top, as `rac_basic`'s wall 493612 does,
  whose box holds the profile's line carried on. Revit 2024 only.

- **`unsupported_features` lists
  `revit_compound_assemblies_and_walltype_widths` only for a file with a
  host element written without its layers.** It was listed on every file,
  whatever it held, though each wall, floor, roof and ceiling whose type's
  compound structure is read has been written with its layers, materials and
  widths as an `IfcMaterialLayerSetUsage` since RE-58 (Revit's on 88 of 88
  sets on Core Interior and 15 of 15 on RE1 Architecture). It now goes where
  every wall, slab other than a stair landing, roof and ceiling has a layer
  set: on RE1 Architecture, Electrical, Mechanical and Plumbing, and
  Autodesk's `rst_basic` 2024 and 2027, of 21 exports measured. Core
  Interior (41 walls and 12 floors without) and Einhoven (26 walls) keep it.
  The IFC is unchanged.

- **The 2024 rows read as an `ArcWallRectOpening` index are no longer
  elements, and the export no longer claims their host join is missing
  (RE-376).** Every id those rows name is a system element: on Core
  Interior all 4,917 name `ConceptualConstructionType`,
  `ConceptualSurfaceType`, `GStyleElem`, `FillPatternElem`,
  `LinePatternElem` or `MaterialElem` records, and none names one of the
  192 elements that cut its walls. The other five 2024 projects measured,
  whose walls hold 165 `SWallRectOpening`s and 4 `ArcWallRectOpening`s,
  give no row. `iter_elements` no longer yields them (Core Interior's
  `production_class_counts` loses `ArcWallRectOpening` 3,777 and
  `source_coverage.exported_element_fraction` goes 0.20 to 0.92), and
  `unsupported_features` no longer lists
  `opening_index_to_ifc_openingelement_host_join`, which it did on every
  file: wall openings are written from the walls' own records (RE-321,
  RE-338). The `opening_index_without_host_geometry` gap, which nothing
  filled, is gone. The IFC is unchanged.

### Research

- **Revit keeps a wall's own height over the part of its plan outside the
  floors that cut it on 191 of 225 pieces of five buildings, and cuts it on
  5; nothing that rests on more than one wall separates them (#1067,
  RE-375).** `tools/re/wall_outside_floor_vs_ifc.py` reads Revit's own bodies
  for those pieces. The stub of the BIM4LCA office's wall 3004978 as built,
  which RE-320 leaves at its full height, is the one piece of them under
  0.05 ft² and inside other walls: Revit's roof leaves the same stub
  uncovered, the wall's Axis starts at the roof's edge, and Revit cuts the
  stub with the rest of the wall. Three readings fit it, and each holds one
  wall.

- **The wall-end rule's unexercised paths are stated, with what each export
  reaches (#1418).** RE-337 now has a section for the paths no model
  exercised before RE-348.
  - A joined wall drawn as a solid is counted, as `joined_wall_not_read`.
    Depending on the order of the record's list, it lands in
    `exported.wall_end_refusals` (and the decisions log) or in
    `exported.wall_end_reads_unmarked`.
  - A wall in the way that the rule does not read is still found by
    nothing, and the end would be drawn through it.
  - `base_cut_lost_on_extended_plan` and `finish_wall_ends` are reachable
    and have no instance.

  On the 111 exports of Measure run 42567326561 (`main` at `83417fde`):
  - 15 of the 93 openly licensed exports are Revit 2024 saves, the only
    release the rule reads;
  - the rule draws, keeps or counts an end on 3 of them: the BIM4LCA
    office's two files and Snowdon Towers Architectural;
  - `joined_wall_not_read` and `base_cut_lost_on_extended_plan` appear on
    none;
  - all 8 walls whose end is drawn have their base cut, so
    `finish_wall_ends` makes nothing.

  No synthetic fixture can reach the pass: it reads Revit 2024 element
  records that `gen-fixture` does not write. `docs/export-diagnostics.md`
  now says which joined walls `joined_wall_not_read` covers, and that the
  decisions log leaves out the unmarked reads.

- **Revit 2027's `rac_basic` names another Level for 21 of its 144 mullions
  than its 2025 and 2026 saves do, and nothing else of them differs (#1493,
  RE-379).** The records' boxes, their host walls (whose own records name the
  same Levels on all three saves) and the rest of their reference lists are
  equal, so the difference is in the files, which the 2027 save sets to the
  host's Level where the others name Level 1. The exports hold every member
  in the same storey on all three saves, and `LevelElementId` is the one Level or
  storey value that differs, as the records do. The other difference #1493 listed, a curtain
  wall of `rac_advanced` whose `Qto_CurtainWallQuantities.Length` the 2027
  save lacked, does not reproduce on `main`: 34 of 34 curtain walls have it,
  equal, on the 2025, 2026 and 2027 saves. No export changes.

## [0.16.1] — 2026-11-09

0.16.1 is the first published release of 0.16. The `v0.16.0` tag was made
but never published. Its publish run passed the tag's signature, branch and
`CI passed` checks and stopped at the viewer load smoke test, which runs
`rvt-elements` and found no such program on its `PATH`. Nothing reached
crates.io, PyPI, the release page or ghcr.io, and the page's `upload` and
`page` jobs run for the first time on `v0.16.1` (#1569). A tag cannot be
moved, so 0.16.1 replaces it.

Everything in
[0.16.0](https://github.com/DrunkOnJava/rvt-rs/blob/v0.16.1/CHANGELOG.md#0160--2026-11-08)
ships in 0.16.1, with the changes listed here, and where the two differ these
entries hold. The doors of `rac_basic`'s 2027 save now write their Width and
Height (#1489). The measurement in 0.16.0's notes is of `main` at 9c66e9bd,
before these changes: the two entries under Fixed give their own, and the
others change no export. A user of 0.15 should read
[Upgrading from 0.15](https://github.com/DrunkOnJava/rvt-rs/blob/v0.16.1/CHANGELOG.md#upgrading-from-015)
with 0.16.1 where it says 0.16.0: an export's IFC header names rvt-rs 0.16.1,
and the release page's files are checked with
`gh attestation verify <file> -R DrunkOnJava/rvt-rs --source-ref refs/tags/v0.16.1`.

### Added

- **Fresh held-out pairs for the 1.0 check are sealed in a registry that no
  committed workflow or script may name (#1565).** W5 counts a held-out pair
  only when its first measurement meets W4, W6 and W7, so a pair that arrives
  first is sealed until the final 1.0 check. `tools/ci/seal-holdout-pair.sh`
  downloads a pair, hashes and counts it, deletes it and adds its row to
  `research/sealed-holdout-models.tsv`. It refuses, from the first 64 KiB of
  each file alone, a project that is not an OLE2 file and an IFC that is not
  that release's own IFC4 or IFC4X3 export, and it refuses a row the check
  would fail, among them a link that names no fixed version (a file at a
  commit, a release asset or a Zenodo record), since the final check fetches
  the files again. `tools/ci/check_sealed_holdout.py`, a step of the PII guard job,
  fails on a malformed row, on any other tracked file holding a sealed URL or
  hash, and on any committed file that is not Markdown and names the registry.
  It cannot see a path built at run time or a run dispatched from an
  unreviewed branch. Its failure-first tests run beside it, and each of 16
  mutations of the check fails them. The registry is empty; the final check's
  Measure input is added when W4, W6 and W7 pass on the reference models.

### Fixed

- **A roof drawn as its record box no longer cuts the top of the walls under it,
  unless the box is exactly the roof's layers and its outline is read (#1471,
  RE-351).** Autodesk's `rac_basic` draws 16 walls up to their record boxes on
  its 2025 save and cut at 5.7265 m on its 2026 and 2027 saves (15 of them on
  2024), up to 3.1865 m lower, from the same record box, base and top Levels,
  joins and top-join element on all 13 of its saves (Measure run 42106436497 at
  b2b5716e). The element is roof 243274, a roof by extrusion on every save: the
  2025 reader draws it as a solid, which the wall-top pass leaves out, and the
  other readers, which read no roof by extrusion, draw it as its record box,
  which the pass took for a roof with a level underside though the box's
  underside is its lowest point. The pass now takes a roof only where
  `attach_slab_layers` gave it its layers, which it does where they add up to
  the box's height (the proof that the underside is level), and where its plan
  outline is read (an L-shaped roof drawn over its box's rectangle would cover a
  wall it does not reach); a slab or footing is cut at as before. **This is a
  smaller error, not Revit's body:** Revit's walls attach to the roof's sloped
  underside, and the box-height prism restored here is still not it (the second
  read of #1476 estimates about 5.1 m3 over across the 16 against about 13.5 m3
  under with the flat cut; not reproduced). Wall 506386 loses the five values
  the roof's box let it carry, `Height` and its net and gross volume and side
  area, and keeps `Length` and `Width` as the 2025 save does (the 2025 and 2026
  exports of Measure run 42063046730). Only roof 243274 exercises the roof-cover
  path anywhere, and no wall is cut by a plate roof, so the branch that keeps a
  plate roof as a cutter is unmeasured. In the unfiltered Measure run
  42554270370 (head 584c8a89 against `main` at fc29acbf, 118 exports) four
  exports move: `rac_basic` 2024, 2026 and 2027, and the held-out AA house
  (Revit 2026, IFC4X3, counts only), whose walls with Revit's base and top go
  from 43 to 56 of 59, the other 3 on Revit's storey (Measure run 42556745265,
  once the held-out counts keep wall heights). The other 114, the BIM4LCA
  office's among them, are byte for byte `main`'s. On the 2026 and
  2027 saves the 16 walls are the 2025 save's bodies, and only roof 243274,
  still a record box, differs among 441 bodies. So the same-project check
  pins no wall any more (#1471, the exception of 2026-11-05 closed), and
  held-out models now keep counts of the walls at Revit's base and top.

- **A type's value block ends where the next native record starts, so the
  doors of `rac_basic`'s 2027 save have their Width and Height (#1489,
  RE-373).** Three family instances' own Width, Height, Rough Width and Rough
  Height are stored right after a door symbol's record, with no value block
  mark between. The length scans gave them to the symbol, which then held two
  of each and wrote neither, so two doors lost their `Qto_DoorBaseQuantities`
  Width and Height. Each block now also ends at the first native record after
  its mark, from the handle's record index. Since records do not nest, that
  drops only entries outside the owner's record. In a Measure run of 111
  exports (42558908204, without the racadv and BIMcollab models) 2 move. One
  is that save, now 418 compared values like its 2026 save. The other is the
  held-out AA house (Revit 26.4, IFC4X3, counts only), whose door Width,
  Height and Area each go from 11 to 13 of 13 equal. No value goes from equal to different or missing anywhere,
  and export time is 0.98 to 1.01 times as before. The family type table's
  text form (`FireRating`, `Manufacturer`) keeps its own bounds.

### Research

- **Revit 2015 to 2020 angled walls keep their record box: Revit stops seven
  of the eight walls that a line body makes worse about 0.28 ft inside their
  lines' ends (#1535, RE-364).** On the spent Vitoria `ALD26 Before` (Revit
  2018, IFC2X3), drawing an angled wall's faces from its line takes 33 of 48 to
  Revit's faces within 0.001 ft, from 1. Of those walls, 8 are worse than
  their box at their ends alone. A scorer that signs each end's difference
  shows that seven run 0.275 to 0.279 ft past Revit's end with the line, and
  0.163 to 0.186 ft with the box, at one or both ends. That fits a join trim
  or a rule for their one wall type, and neither is tested. No angled wall
  has a join reach on these releases, so they stay boxes. No export changes.

## [0.16.0] — 2026-11-08

Tagged as `v0.16.0` and never published: its publish run stopped at the
viewer load smoke test, and 0.16.1 ships everything below (#1569).

0.16.0 reads much more of Revit 2015 to 2023 and of Revit 2026 and 2027, and
says what each space is. On Revit 2015 to 2023 projects beams are drawn along
their location lines, walls are joined by their join lists and a wall at an
angle can follow its line, pipes are drawn as their cylinders with their
types, quantities and ports, elements have their `Constraints` Level
parameters and `Phase Created`, and a Revit 2015 file follows the export phase.
On Revit 2026 and 2027 beams, stair runs, tread landings, sketch outlines,
pipes, ports and door and window cuts are read as on 2024 and 2025, with the
2025 properties but the `IsExternal` of walls, curtain walls, members and
plates; on Revit 2025 beams, tread landings and sketch arcs are read
as on 2024. Areas of Revit 2025 to 2027 are written, all 25 of
`rac_advanced`'s on every save from Revit's own plan topology, every
`IfcSpace` says whether it is a room, an Area or a family instance
(`SpaceKind`), and a project whose last phase holds no room writes the rooms of
the latest phase that holds any. A door or window set in a plain wall is
placed on Revit's axes and a door set in no wall at its stored origin, and
both flips are decoded. The openings fire hose cabinets, doors and curtain
walls cut in walls, door types' `FireRating`, family types' `Manufacturer`,
the quantities of columns a slab cuts, and Furniture Systems as
`IfcSystemFurnitureElement` are written as Revit's export writes them. Exports
now hold the IFC4 EXPRESS rules IfcOpenShell's default validation leaves out,
`rvt-ifc`, Python and the viewer refuse an export holding a number that is not
finite, and the diagnostics, the elements CSV,
the viewer's panels and the MCP server count and place exactly what the IFC
writes. The viewer's info panel lists an element's saved parameters, read in
the tab with no request once the file is open. Release pages now list and
attest the wheels and the sdist with the archives. puzzbobb wrote the Revit
2015 to 2023 beams, walls, phases, Levels and pipes and the 2026 and 2027
reads (#1426, #1454, #1509, #1517, #1521, #1522, #1523); STE1200's
saved-graphics findings (discussion #112) and mahach666's RvtIO findings
(discussion #1241) are measured in the Research entries; and inevitable-co-ai's
reports led to the `rac_advanced` Areas (#1261), the door flips (#1357) and the
room phase fallback (#1359). Thanks to all four.

Not measured, and said in each entry: no Level parameter or `Phase Created` of
Revit 2015 to 2023, and no beam, pipe, landing or sketch outline of Revit 2026
or 2027, is scored against a Revit export, and of the 2026 and 2027 stair runs
only one flight on each of two held-out models. The Revit 2015 to 2023 wall
lines, pipes and beams are scored on held-out projects only, as counts (below).
Every one of these readers is held to the same Autodesk project's 2024 or 2025
save instead, which cannot catch an error that save shares. On 2025 to 2027
that check compares each element's class, GlobalId, storey, placement and body,
and on 2026 and 2027 also the cuts, ports, door and window sizes and types and
the properties RE-346 adds; on 2015 to 2023 it compares the values each entry
names. Among the files Measure fetches outside the held-out pairs, the 2015 to
2023 beam rule reaches Autodesk's samples only: BIMcollab's 91 beams hold no
line it reads (#1514). Two doors of `rac_basic`'s 2027 save still write no
Width and Height quantities (#1489). The Areas of Revit 2025 to 2027
(`area_plan_topology_unchecked`) and of the Snowdon Towers 2024 samples are
checked against no Revit export, and the only external check of
`rac_advanced`'s Areas is an IFC2X3 export of unestablished origin. Furniture
Systems rest on one project and 3 elements, the wall end order on 4 walls of
one project, and the cut-opening and raised-host thresholds on the BIM4LCA
office pair alone. The placement origin of a door or window set in a wall is
still its record box's centre, more than 0.5 ft from Revit's on almost all
(RE-357). The release workflow's signature, branch and `CI passed` checks and
the page's `upload` and `page` jobs run for the first time on this tag.

Measured against 0.15.0 on the reference models, the BIM4LCA office, the
Revit 2025 models, Autodesk's sample projects of every release, the held-out
models, `rac_advanced`'s IFC2X3 pairs and BIMcollab's model (Measure run
42555357168, `main` at 9c66e9bd, 118 exports; the release adds only its notes,
witness files and version strings to it):

- 97 of the 118 exports move. The 21 that do not are 18 of the 24 Revit 2025
  models, Snowdon Towers' electrical and HVAC models and one held-out Revit
  2025 model.
- On the BIM4LCA office the property values equal to Revit's go from 44,231 to
  44,428 as built and from 44,394 to 44,577 at permit, and the quantities from
  29,562 to 29,761 and from 29,849 to 30,037; "different" stays 1 on each, and
  the values rvt-rs writes that Revit's export does not fall from 2 to 1 and
  from 7 to 6.
- On RE1 (IFC2X3) the equal property values go from 127 to 138 on Electrical,
  from 182 to 185 on Mechanical and from 313 to 318 on Plumbing, whose values
  Revit's export does not write fall from 2 to 0 (the water closet tank,
  #1496); Architecture is unchanged.
- On the held-out models, counts only: the Revit 2026 and 2027 pairs go from
  209 to 289 equal property values each and the 2026 and 2027 office pairs from
  186 to 230, and their equal quantities from 21 to 89, 17 to 74, 14 to 74 and
  10 to 60; the AA house goes from 378 to 427 and from 135 to 200. On three
  Revit 2020 and 2021 projects the angled walls whose faces lie within 0.001 ft
  of Revit's go from 0 to 262 of 474, and those whose ends do from 0 to 124.
  On one Revit 2021 project's IFC2X3 export 3 of its 10 beams' `Span` and
  `Slope` are equal, from none, and none is different.
- On Revit 2018's IFC2X3 exports of the two Vitoria projects, counts only, the
  pipe quantities equal go from 0 to 149 of 149 and from 0 to 140 of 140, and
  the values rvt-rs writes that those exports do not from 18 to 62 and from 20
  to 65: the pipes' `InvertElevation`, which they do not hold. An IFC2X3 export
  is counted apart and removes no value. On BIMcollab's Revit 2015 model the 3
  air terminals its export leaves out are gone (#1332), and its 241 elements
  pair with the export's 241.
- No property value or quantity goes from equal to different or missing on any
  model, and "different" is unchanged everywhere. The IFC4 rules check finds
  nothing on any of the 118 exports, and the same-project gates pass (15
  samples held to their earlier saves, and the Areas).
- Export time is 0.82 to 1.36 times 0.15.0's on the 68 timed files, 1.015 in
  total (243.9 s to 247.5 s). The 2026 and 2027 MEP samples take 1.32 to 1.36
  times as long, the cost of reading their pipes, ports and cuts (RE-346), and
  `rac_advanced`'s 2026 and 2027 saves 1.12. Peak memory is 0.54 to 1.29 times
  0.15.0's. It rises most on RE1 Architecture (93 to 120 MiB, 1.29), on
  `rac_basic`'s 2024 to 2027 saves (1.11 to 1.20; 222 to 266 MiB on 2024) and
  by 1.08 to 1.09 on `rac_advanced`'s 2016 to 2022 saves and `rme_basic`'s 2019
  save, and the highest peak is Snowdon Towers Architectural's 1,073 MiB. Each
  file's two sides were timed on one runner.

### Upgrading from 0.15

- Every export's IFC header names rvt-rs 0.16.0, so an export that is
  otherwise unchanged still differs from 0.15.0's there.
- A default export of a Revit 2015 to 2023 project can change: beams are
  drawn along their lines with `Span`, `Slope`, `Roll` and `Length`, walls are
  cut back at the joins their join lists name and a wall at an angle may take
  a line body, pipes take their type, cylinder, `Qto_PipeSegmentBaseQuantities`
  and ports, and elements gain `Constraints` Level parameters and
  `Phasing.Phase Created`. On Revit 2015 an element not in the export phase,
  and a room not in it, is left out, and `--phase` is accepted.
- A default export of a Revit 2025, 2026 or 2027 project can change: beams,
  tread landings and sketch arcs (2025 to 2027), and stair runs, pipes placed
  by their connector entries, family instances' ports, wall cuts at doors and
  windows, `IsExternal` on the classes RE-193's last step reads (not on walls,
  curtain walls, members or plates), railing, stair, room and pipe and duct
  values (2026 and 2027), as on 2024 and 2025. A door's or window's Area is not
  written on 2027, nor a stair's, flight's or stringer's `ObjectType` on 2026
  and 2027, where Revit's own exports disagree. Shaft openings are cut on Revit
  2024 only: on 2025 to 2027 floors, ceilings and roofs stay whole and the
  diagnostics count each shaft as the `skipped` item `shaft_opening_not_cut`.
- Areas of Revit 2025 to 2027 are written as `IfcSpace` and say
  `SpaceOutlineSource` `area_plan_topology_unchecked`; an Area drawn from
  Revit's plan topology on 2015 to 2024 says `area_plan_topology`, and
  `rac_advanced`'s 16 Areas that said `area_boundary` or
  `area_boundary_joined` now say it. Read the new
  `RvtElementRecordGeometry.SpaceKind` (`room`, `area`, `mep_space`,
  `family_instance`) to tell spaces apart, never the outline source's values.
  The room schedule (`rvt-schedule --schedule rooms`, Python's `schedule_csv`,
  the viewer) gains a last column, `space_kind`, and the MCP server's
  `list_rooms` a `space_kind` and its summary `space_kinds`.
- A project whose last phase holds no placed room now writes the rooms of the
  latest phase that holds any (`rac_basic_sample_project`: 14 rooms where
  0.15.0 wrote none). The diagnostics name it in `room_phase_fallback`, the CLIs
  print a line and the Python bindings raise a `RuntimeWarning`; name a phase
  (`--phase`, `phase=`) to keep 0.15.0's result.
- A door or window set in a plain wall is placed on the stored axes, reversed
  where its two flips differ (`PlacementAxesSource`), with its body where it
  was: `rotation_deg` in the elements CSV and the viewer's schedule, the info
  panel's Rotation and the MCP server's `rotation_degrees` are now set off +X,
  and on a quarter turn a box body's `width_ft` and `depth_ft` swap. A door
  set in no wall is placed at its stored origin (`PlacementOriginSource`
  `stored_origin_no_opening`), so its `location_feet` moves. The decoded Door
  and Window records carry `m_hand_flipped` and `m_facing_flipped`, and Door
  records `m_opening_placement_origin_x`, `_y` and `_z`.
- A floor's void can take a different `Tag` (RE-355), and with it its
  opening's GlobalId and the 3 ids that follow it (one void on each BIM4LCA
  office file, 4 on Snowdon Towers).
- New values Revit's export writes: the openings of fire hose cabinets, of
  doors in a second wall and of untrimmed curtain walls; door types'
  `FireRating` and family types' `Manufacturer`; a cut column's
  `CrossSectionArea` and `NetVolume` and a round column's `OuterSurfaceArea`.
  Furniture Systems are `IfcSystemFurnitureElement`, not `IfcFurniture`, and a
  plumbing fixture whose type draws nothing visible is left out
  (`element_record_family_type_draws_nothing`).
- Walls: a wall that embeds a curtain wall is drawn from its saved mesh where
  Revit raised its base, a profile-edited wall's end and base and its cut-out
  past the profile follow Revit's, and a wall holding its previous location
  line is read. New markers say so (`BodyEndSource`, `BodyEndRefusal`,
  `BodyBaseSource` and `BaseCoverRefusal` values, `ProfileStubSource`,
  `ProfileStubRefusal`).
- IFC4 rules: a sloped beam's sweep is under `AdvancedSweptSolid`, a
  `USERDEFINED` type carries its name as `ElementType`, and an `IfcSpace` is
  associated with no material; `--revit-set-placement` (the MCP server's
  `revit_set_placement`) keeps the material Revit's export gives it, so the
  elements CSV and the viewer list none for a space.
- `exported.building_elements` and `by_ifc_type` count only the elements the
  IFC writes: add the new `exported.body_pieces_folded` to get 0.15.0's
  figures (47 `IFCCOVERING` on each BIM4LCA office file).
  `building_elements_with_geometry`, `body_sources`, the geometry-gap items and
  `decoded.parameter_value_count` leave the folded pieces out too.
- The elements CSV, the viewer's schedule, info panel and scene tree, and the
  MCP server put each element on the storey the IFC writes it on (a stair's or
  curtain wall's part on its whole's) and give a folded piece no row; an
  element with folded pieces has empty size cells (47 fewer rows on each
  office file). The MCP summary gains `elements.folded_into_bodies`, and the
  tool contract in `docs/mcp-tools.json` moves with it.
- New diagnostics fields: `room_phase_fallback`, `exported.space_kinds`,
  `spaces_without_kind`, `body_pieces_folded`, `wall_cut_openings_left_out`,
  `wall_end_refusals`, `wall_end_sources`, `wall_base_sources`,
  `wall_ends_order_dependent`, `wall_end_reads_unmarked`,
  `wall_profile_sources` and `wall_profile_refusals`, and a warning counting
  chain records whose entry block does not read on a file with an Area.
- An export holding a NaN or an infinity now fails with an error naming the
  entity (`rvt-ifc`, Python's `write_ifc`, the viewer's IFC download), and
  `step_writer::write_step_to` returns an `InvalidData` error.
- The viewer's page script is about 40 kB larger (8.65 kB gzipped), its
  `worker-src` gains `blob:`, and selecting an element reads its saved
  parameters in a worker of its own; above 1 GiB of wasm memory the panel
  asks first.
- The release page's `SHA256SUMS` and attestation cover the wheels and the
  sdist as well as the archives:
  `gh attestation verify <file> -R DrunkOnJava/rvt-rs --source-ref refs/tags/v0.16.0`.

### Added

- **The openings a fire hose cabinet and a door cut in a wall, and a curtain
  wall's where nothing trims it, are written, and every step left out is
  counted (#1165, RE-338).** A wall's native record lists the elements that
  cut it, in the order Revit's export numbers the wall's openings, and Revit's
  export writes an `IfcOpeningElement` for each that nothing fills. RE-321
  wrote the rectangular wall openings and measured these three kinds without
  writing them. A cabinet's opening is the box of the solid its symbol cuts,
  placed by its instance, a recess where it is shallower than the wall; a
  door's `WallCutoutGStep` in a wall other than its own (`m_bCutSomething` 0)
  is the door's own opening box where the wall runs across the door's wall;
  a curtain wall's is its record box, half a foot past the wall's faces,
  where the box lies half a foot inside the wall's body at both ends, no other
  wall touches it and every copy of its record agrees. On the BIM4LCA office
  as built Revit's export holds 39 cabinet, 39 door and 70 curtain-wall
  openings of these kinds and rvt-rs writes 39, 25 and 27 of them (38, 33 and
  13 of 38, 43 and 72 at permit), every one with Revit's box within 0.01 ft,
  GlobalId, name, type and both `Reference`s; none is wrong. Unfilled
  openings written go from 75 to 169 as built and from 69 to 156 at permit,
  and the census gains 182 and 198 equal property values and quantities as
  built (head f54b619f, Measure run 41959375917). A door's opening 10 ft
  across a wall that runs along its own wall, and a curtain wall that reaches
  its wall's end, has another wall beside it or has record copies that
  disagree, are not written: `exported.wall_cut_openings_left_out` counts each
  step left out by reason, and the counts close. Three openings as built and
  four at permit (one of them RE-321's) are written that Revit's export of
  the same file lacks and its export of the sibling file holds: the
  maintainer decided on 2026-11-04 to write them and count them apart, by a
  rule with no ids that the opening and property scorers apply. Four of the
  six have Revit's box in the sibling export to 0.000 ft; the cabinet 3388647
  and the door 1728679, whose cutters moved between the saves, are 0.655 and
  0.130 ft off it. Unlike the 2026-11-03 opening, the host walls' records are
  not byte for byte the sibling file's (RE-338 section 6). The thresholds were fitted on the
  office pair alone: Core Interior and RE1 Architecture are unchanged, the 19
  held-out and spent models' exports are byte for byte `main`'s (Measure run
  41954664033) and the Autodesk samples' are but `rac_basic` 2024 and 2025,
  which each gain one opening.

- **A door type's `FireRating` and a family type's `Manufacturer` are written, read from the family type table's form (#1185, RE-354).** A type's text parameters are stored in a second form beside the one RE-77 reads: `u32 n · UTF-16 × n · (u32 0 | ff ff ff ff · u16) · f64 · i64 -1 · i64 BuiltInParameter`, the text first, in the type's own value block. The BIM4LCA office's six door types that Revit's export gives a `FireRating` (`EI30`, `EI60`, `-`) hold it so under the built-in id -1001206, and the six family types it gives a `Manufacturer` hold it so under -1010108, which RE-289 had not found because it looked for RE-77's form alone. A table entry is a type's only where no value block mark of any id lies between the type's mark and the entry (33 of the office's 88 `Manufacturer` entries lie more than 100 KB from their block's mark, up to 3.2 MB), and a type whose copies disagree, across partitions or with RE-77's form, gets neither and is counted in the diagnostics. The type object gets each in its common set, or in `Pset_ManufacturerTypeInformation` where IFC4 applies it (the type of an `IfcElement`), and an element takes its type's Fire Rating where its own `Pset_DoorCommon.FireRating` is empty (the two doors of the `-` type). Unfiltered Measure run 42135050819, head e07349c0 against `main`: on the office, as built and at permit, `FireRating` goes 47 to 55 of 55 and `Manufacturer` 0 to 6 of 6 equal; on the held-out AA house (Revit 26.4's IFC4X3 export) 0 to 15 of 15 and 0 to 16 of 16; on Snowdon Towers Architectural, against its local Revit IFC4 export, the door `FireRating` 16 to 128 of 128; on RE1 (IFC2X3) `Manufacturer` gains 11, 3 and 5 equal on Electrical, Mechanical and Plumbing. None is different, and the values written that Revit's export does not write are `main`'s except one on RE1 Plumbing: the type of the water closet tank 442378, which rvt-rs writes and Revit's export does not hold (#1496; RE1's export is IFC2X3, counted apart). Core Interior, whose export writes no property set, gains the door `FireRating` on 4 type objects and 50 doors, and the four `rac_basic` samples a `Manufacturer` on 23 type objects each, unscored. The other models' IFC is `main`'s byte for byte. A door type's `ThermalTransmittance` (3.7021, the `f64 · ff × 8 · i64 -1005430` entry of its block) is found and left out: reading it was equal on the office but wrote a value on a door type of each of six R2025 models whose exports write none (Measure run 42122005108). A window's `Pset_WindowCommon.FireRating` stays missing: R2025's `0.5h` are table entries under family-parameter ids, not -1001206 (#1507).

- **Revit 2015 to 2023 projects draw beams along their location lines, with
  `Span`, `Slope`, `Roll` and `Length` (#1190, RE-362).** A beam that a named
  column trims keeps its cut box (RE-122), and a steel beam's I section is not
  read on these releases. **No Revit export scores any beam this rule
  draws** among the models Measure fetches without a held-out pair (Measure run
  42134881703): BIMcollab's 91 beams stay record boxes (none holds a line the reader
  finds, #1514, so its reach is Autodesk's samples), and the three.js export
  of `rac_advanced` holds no `IfcBeam`. Each beam is held to the same Autodesk
  project's later save, which cannot catch an error that save shares. One
  held-out Revit 2021 project's IFC2X3 export (counts only) scores 3 of its 10
  beams' `Pset_BeamCommon.Span` and `Slope` equal, none different and 7
  missing (Measure run 42548058739, head e853e731, the change as it landed;
  its 3 `Roll` values, which IFC2X3 does not define, are counted apart).

- **Revit 2015 to 2023 elements have their `Constraints` Level parameters (#1190, RE-369).**
  `Level`, `Base Constraint`, `Base Level`, `Top Constraint`, `Top Level` and
  `Reference Level` are the Levels each element's own native record names. On
  Autodesk's samples every one equals the same project's 2024 save, 85,121 of
  85,121 across 30 saves. **None of these releases' Level parameters is scored
  against a Revit export here**; a later save cannot catch an error it shares.

- **Revit 2015 to 2023 elements have their `Phasing.Phase Created` (#1190, RE-368).**
  It is the phase each element's own `m_createdPhaseId` names. On Autodesk's
  samples every element both saves give a phase has the 2024 save's phase,
  84,045 of 84,045 across 30 saves. Each of those samples holds its elements in
  one phase, so the figure shows the field is read, not that it tells two phases
  apart. **No Phase Created of these releases is scored against a Revit export
  here**; a later save cannot catch an error it shares.

- **Pipes have their type, cylinder and ports from their own native record on Revit 2015 to 2027 (#1190, #869, RE-370).**
  On 2015 to 2023 every pipe now gets its type (`Family`, `Type`, type
  `Reference`), the cylinder it is drawn as, its
  `Qto_PipeSegmentBaseQuantities`, a vertical pipe's invert, and ports placed
  at its ends with their flow direction; on 2026 and 2027, which RE-131's
  connector entries do not read, every pipe is drawn as its cylinder with its
  ports placed; on 2024 and 2025 so is every pipe those entries do not place.
  The record's line and diameter give Revit's own export on 93 of 93 pipes of
  RE1 Plumbing, RE1 Mechanical and the BIM4LCA office. On Autodesk's twelve
  MEP sample saves of 2015 to 2027, 93,987 pipe values equal the 2024 save's
  and 3, all of one pipe edited between the saves, differ. **No pipe of 2015
  to 2023, 2026 or 2027 is scored against a Revit IFC4 export.** Revit 2018's
  IFC2X3 exports of the two held-out Vitoria projects (counts only) hold 149 of
  149 and 140 of 140 `Qto_PipeSegmentBaseQuantities.Length` equal, none
  different, and no `Pset_PipeSegmentOccurrence.InvertElevation`, so the values
  rvt-rs writes that they do not go from 18 to 62 and from 20 to 65; an IFC2X3
  export is counted apart and removes no value (Measure run 42555357168).

- **Revit 2015 to 2023 walls are joined by their join lists, and a wall at an angle can
  follow its line (#1190, RE-364).** The join lists' 32-bit layout is read, and every
  wall of `rac_advanced` names on 2015 to 2023 the partners its 2024 save names. On 2021
  to 2023 a wall at an angle is drawn along its line; on 2015 to 2020, where nothing
  marks a slanted wall, only if its record box closes on the line's rectangle. A wall
  along a model axis keeps its record box cut back at its joins, and a wall's `Length`,
  `Height`, areas and volumes are not written on these releases. On Autodesk's samples
  24 walls in 12 exports take a line body, on models with no Revit export to score it,
  and on held-out projects with one, 268 walls, 259 of them on one project. Against Revit's exports of 2018 to 2021 projects
  the axis-parallel walls and their quantities stay as they were, and layer sets gain
  (three held-out models, counts only).

- **Revit 2026 and 2027 projects draw beams along their axes, stair runs
  and tread landings as their steps and treads, and floors, ceilings and
  slabs from their sketch lines (#1190, RE-341).** Beams are drawn as on
  2024, runs from their native records as on 2025 with the stair property
  sets and quantities read from them, tread landings as on 2024 (a
  monolithic landing stays its box), and the sketch lines' exact ends,
  curves and Sketch curve lists are read as on 2024 and 2025. A roof there
  keeps the record-box solve as on `main`, without its slope, so a sloped
  roof can still be drawn as a level block. **No beam, landing, stair run or
  sketch outline of a 2026 or 2027 file is scored against a Revit export**;
  each is held to the same Autodesk project's earlier save instead
  (`tools/re/bodies_across_saves.py`, which Measure runs and fails on), and
  that cannot catch an error the earlier save shares; it compared bodies alone
  when this landed, and since RE-346 (below) also compares cuts, ports, door
  and window sizes and types and the properties RE-346 adds. No 2026 or 2027 held-out model holds a
  beam, so the beam reader has run only on two Autodesk samples saved from
  earlier releases. Against Revit's own export only one stair flight is
  scored, on a held-out 2026 model and a held-out 2027 one (property values
  equal 209 to 224 of 352 and of 354, quantities 21 to 23 of 322 and 17 to
  19 of 367; Measure run 41988011289, head 1d5bbf03). In run 42023546789
  (head d3635bbb) 103 of 111 exports are unchanged and 8 change, all 2026
  or 2027; the check reads PASS on all six samples, and the exports'
  `rules:` findings are 0. The one outline the 2024 and 2025 saves
  disagreed on, a floor, was the 2025 save drawing its arcs as chords (24
  vertices against 170); the 2025 save reads them since RE-345 (#1440).
  The shaft voids this entry first cut from floors and ceilings on 2026 and
  2027 are no longer cut (RE-345).

- **Revit 2026 and 2027 projects place a pipe by RE-131's connector entries
  first, as 2025 projects do (RE-370 already drew every pipe as its cylinder
  from its record), place family instances' ports, cut walls to their doors' and windows' outlines, and write
  `IsExternal` on the classes RE-193's last step reads, railing heights and lengths, stair names, rooms' height and net
  volume and the pipe and duct quantities, as 2025 projects do (#1190,
  RE-346).** Ten readers read the 2025 records these releases write; their
  release lists gain 2026 and 2027. **None of it is scored against a Revit
  export of a 2026 or 2027 Autodesk sample**; it is held to the same sample
  saved by Revit 2025 (`tools/re/bodies_across_saves.py`, which Measure now
  fails on a difference on `rme_advanced` and `rme_basic` too), and that cannot
  catch an error the 2025 save shares. Revit's own exports of the held-out 2026
  and 2027 projects disagreed in two places, so a door's or window's Area is
  not written on 2027, where Revit's export gives it a value
  that is not its Width times its Height, and a stair's, its flights' and its
  stringers' `ObjectType` is not written on 2026 and 2027, where Revit's export
  writes no `Family:Type` there (the `ObjectType` of every other class differs on
  those releases as well, #1472); their Width, Height and the stairs' names and
  types are written and equal. `IsExternal` on walls, curtain walls, members and
  plates, which the 2025 save writes, is still not written on 2026 and 2027, and
  two doors of `rac_basic`'s 2027 save write no `Qto_DoorBaseQuantities` Width
  and Height that its 2025 and 2026 saves write (#1489); the check counts both
  as values only the 2025 save writes, not as failures. No
  held-out model holds a pipe, a duct or a port. In Measure run 42545885153
  (this change against `main` at 1c088750, unfiltered), on the 2026 and 2027
  saves of `rme_advanced` and `rme_basic` every body is the 2025 save's (6,605
  and 6,650; 36 walls differed on `main`, uncut by their doors and windows),
  every pipe (488 and 491) is the 2025 save's cylinder, as it already was since
  RE-370, and the placed ports are the 2025 save's, 6,967 and 7,054 of them
  against 1,944 and 1,956 on `main`. `rac_advanced`'s 2027 save compares its 83 doors and
  24 windows and leaves none out. Only 2026 and 2027 exports move (15 of 118),
  and on the held-out 2026 and 2027 pairs every value added is equal, with
  "different" and the values Revit does not write unchanged.

- **Areas of Revit 2025 to 2027 are written as `IfcSpace`, as on 2024.**
  Their partition chain records are the 2024 ones, but a file saved by Revit
  2025 to 2027 may put a block of 22-byte entries before a record's body,
  counted by the `u16` after its class tag, and the reader now skips it. On
  Autodesk's `rac_advanced` the 2025, 2026 and 2027 saves write the 25 Areas
  of the 2024 save, each with the same GlobalId, number, name, storey and
  body; the 2025 to 2027 saves are checked against the 2024 save only, and a
  Snowdon Towers 2025 file awaits a recorded source (#1194, RE-333). On the recorded Autodesk 2024 Snowdon Towers samples rvt-rs writes 101 Area spaces (96 with a body) on Architectural and 7 (2 with a body) on Structural (the bodies are from Measure run 42003913780 on `bcb02107`, Architectural's refusals from run 42017400629 on `5008247d`, Structural's total from `e9124b3a`),
  where main writes none; these are counts, and no Revit export of those two
  is available. The diagnostics gain a warning that counts the declared chain
  records whose entry block does not read, on a file that holds an Area or an
  area boundary line (it appears on those two Snowdon Towers samples, 14,881
  and 2,932 records, and on no other of the 58 samples). Read by puzzbobb's
  commit (#1371), which keeps their authorship. **Not measured against any
  Revit export** (Griffin, 2026-11-05, "Ship, labelled"): the
  Areas of Revit 2025 to 2027 and the 108 Area spaces of the Snowdon Towers
  2024 samples are written by default; the Areas of Revit 2025 to 2027 say `SpaceOutlineSource` `area_plan_topology_unchecked` and the Snowdon Towers 2024 samples' `area_plan_topology`, because a 2024 file shares its release and code path with the office's (Griffin, 2026-11-05, "Mark 2025 to 2027 in the file"); the evidence is that the loop drawn for each equals
  the area Revit saves in the file for it and holds the Area's point and, on
  2025 to 2027, that the same project's 2024 save gives the same Areas. They stay written only while Autodesk's `rac_advanced` gives, on its 2025, 2026 and 2027 saves, the Areas of its 2024 save, Area by Area in GlobalId, number, name, storey, outline source, body area, top surface, height and lowest vertex (Griffin, 2026-11-05, "Same-project gate"). Every Measure run of the samples that holds the four saves checks it and fails on a difference, other than 25 Areas or 25 bodies on a save, an Area with no storey or a missing save; a run with fewer than the four saves checks nothing and prints a notice, and a failure blocks landing by the landing rule. The seven held-out Revit 2025 to 2027 models write no Area, so they test nothing here.

- **A column one slab cuts across its top gets the cut mesh's `CrossSectionArea` and `NetVolume`, and a round column its `OuterSurfaceArea` (#1203, RE-353).** Where Revit's export finds no extrusion for a column it writes only these two quantities, from the element's computed area and volume; on the BIM4LCA office at building permit the 18 columns drawn as a cut mesh have exactly those, the volume the mesh's and the area half the mesh's surface. A box-drawn column whose type is a plain rectangle, whose `CutPairGStep` names one slab that starts inside it and reaches its top and that a clean pair joins to the same slab, is its box less the slab's plan region (outline less voids) clipped to its section, from the slab's base up: `NetVolume` is that volume and `CrossSectionArea` half the surface of the cut box. At permit 17 of the 18 are written equal (`CrossSectionArea` 320 to 337 and `NetVolume` 265 to 282 of 345) and none different; the 18th, whose cut pairs name a wall that only touches it, is left out, and `Length`, `OuterSurfaceArea` and `GrossVolume` stay unwritten on the 17, as Revit omits them. A round column no pair joins, whose type's named lengths are `Diameter`, equal to both sides of its box, and at most the family's `Offset Base` and `Offset Top` each exactly 0, gets its `OuterSurfaceArea`, the circumference times its height, which is Revit's exactly: one column on each of the two office files, the same column. Its `CrossSectionArea`, `GrossVolume` and `NetVolume` are not the circle's (Revit's extrusion area is 0.99934 of it, its volume parameter 0.99978) and stay out. Unfiltered Measure run 42130190386, head 35cfd89f against `main`: the office gains 34 and 1 equal quantities at permit and 1 as built (17 columns stay, as at 223968aa before the limits below), different stays 1, quantities Revit does not write stay 0, and the census of every other model, the counts of the 18 held-out models and the IFC of Core Interior, Einhoven and RE1 Architecture are `main`'s; the `same-project gates` check passes. The IFC of Snowdon Towers Architectural (Revit 2024) gains the round column's `OuterSurfaceArea` on its 2 round columns (136 bytes, run 42135731462); Revit's export of it holds no base quantities, so the value is counted apart, and Revit's own extrusion of each column gives it to 2.4e-7 m². The rules read Revit 2024 files only, take a slab only where it is one exported extrusion with an outline that was read, a flat top as the wall pass knows it and a clean pair joining it to the column, leave out a column RE-178 trimmed, and write `CrossSectionArea` only where every edge of the slab inside the section runs parallel to a side of it (the half-surface rule is measured on meshes whose every face is, and `NetVolume`, exact geometry, is written either way).

- **All 25 Areas of Autodesk's `rac_advanced` sample are written with their
  region on its 2015 to 2020 and 2022 to 2027 saves, from Revit's own plan
  topology; 16 were on 2015 to 2024 (RE-330) and none on 2025 to 2027; the
  2021 save, which holds two states of 16 lines, is a separate entry
  (#1261).** For each area scheme on each Level Revit keeps the loops its area
  boundary lines make, each with its area, and each Area names its loop. The
  loop is rebuilt from the lines, each moved by its area rule's offset (to a
  wall's face, 0.497 ft on that sample) and met where the moved lines cross,
  and is used only where its area is the one Revit saves for it and the Area's
  point is inside. It comes first; where it is not read the region is the lines' arrangement with RE-330's joins, and where it is read and fails a check of its own the Area has no body, and a group of lines
  that cannot be read as a whole (a design option, a stray height, a copy that
  names another Level or scheme or lies at another height, a missing line)
  gives no region of either kind; copies that conflict in place give the
  topology's region alone (the 2021 entry). It draws the 9 Areas
  the arrangement could not. An arc is written as an arc, an
  `IfcIndexedPolyCurve` with three-point arcs as Revit's export writes one (a
  span over a half turn as two arcs), and the outline is checked as it is
  written, its area with the arcs as arcs against the circuit's; an arc that
  cannot be read finitely refuses the Area (`area_arc_not_finite`). A point
  where a third line meets a straight outline stays a vertex, as in RE-317.
  An Area drawn from the plan topology says `SpaceOutlineSource` `area_plan_topology` (`area_plan_topology_unchecked` on Revit 2025 to 2027 and on the 2021 save, below), a new value that is also in the MCP tool descriptions
  of `space_outline_source`; on `rac_advanced` the 6 that main marks
  `area_boundary` and the 10 it marks `area_boundary_joined` now say it. An
  Area whose loop fails a check has no body, as before, and says why in the
  diagnostics: `area_not_in_one_plan_topology`, `area_circuit_not_found`,
  `area_circuit_side_not_read`, `area_circuit_area_differs`,
  `area_point_outside_circuit` or `area_arc_not_finite`. Measured on `e9124b3a`
  (Measure runs 41894715743 and 41894722135): against the IFC2X3 export of
  `rac_advanced` from the three.js repository, which nothing of this reader
  was read off, the 25 Areas of the 2020 save are all written, within 0.1% in
  area and 0.01 ft in outline, none extra and none missing, the arc included;
  the 16 that the lines' joins draw are equal to their topology loops on all
  nine saves where main draws them. The export is IFC2X3 by a build of Revit
  2021's exporter whose string ends "IU alternativa 21.0.0.383" (publisher not
  established), counted apart and never used to remove a value. On the
  BIM4LCA office (2024) the 30 Areas have the region, vertices and area they
  had; the IFC differs from main's only in their `SpaceOutlineSource`, in the
  starting vertex of 3 outlines and the order of the holes of 6 (the same
  rings), and in a timestamp. The rule is puzzbobb's (#1371), whose two
  commits keep their authorship; the arcs, the vertex rule and the value
  source are the maintainer's. No Revit IFC4 export of these Areas exists: the
  IFC2X3 export is the only external check, and it is out of sample for this
  rule because the topology's loops and the figures they are checked against
  were not read off it, and only its counts are kept (its terms were never checked); on `5008247d` (Measure run 42017400629) the 2020 save's pairs have 116 spaces in both files, none only in Revit's and none only in rvt-rs's. Where the plan topology and the lines' arrangement both give a
  region they must be the same one, else the Area has no body and says
  `area_regions_disagree`; an Area drawn from the topology has `BodySource`
  `native_area_plan_topology` and a value source `area_plan_topology` that
  names the records it was read from. The counts of what is written are from Measure run 42017400629 on `5008247d`; the vertex-level and equality figures above are from the archived head `e9124b3a` of #1371 (runs 41894715743 and 41894722135) and were not re-read on `bcb02107`.

- **Measure says what moved in an export's meaning, and Jev says whether the
  pull request states it (#1334).** A Measure run with a base now diffs the
  two sides' MCP dumps for each openly licensed model (Core Interior,
  Einhoven, RE1, the BIM4LCA office, the Revit 2025 models) with
  `tools/re/semantic_diff.py`: elements added, removed and changed with the
  value before and after, byte offsets that moved for unrelated reasons
  dropped, order ignored, values compared to the dump's six decimals, and
  identical changes collapsed by their whole (path, before, after) so a bulk
  change is one line and a different value is never hidden in it. The diffs
  are the `semantic-diff` artifact; a model that is not openly licensed has no
  dump and the compare job fails if one holds one. The new Semantic check
  workflow (`workflow_run` on Measure, run from main in the Actions
  environment `jev` that holds the key) scores only a Measure run dispatched
  in this repository, gives Jev `jev-1.13.0` the diff in chunks of at most
  16,000 characters beside the pull request's own `## Summary` and `##
  Testing` (found through the API from the measured commit, HTML element tags
  and invisible characters removed, quoted as untrusted data, all of it shown in
  the check's summary) and asks, blind (the outputs are labelled A and B at random per
  chunk, with no model name, pull request number or direction), whether each
  difference is stated specifically, follows from a specific change, is
  covered only by a general description, is unstated, or is only rounding. A
  general description counts as unstated, so "improves how walls are
  exported" covers no change. It writes the check run `semantic diff
  (advisory)` on the commit Measure measured: pass (success), warn (neutral),
  fail (failure: a move the Summary and Testing do not state specifically) or
  NOT MEASURED (no key, a pull request from a fork, no pull request for the
  commit even for an empty diff, a check written on a commit Measure did not
  measure, no Summary where the diff has findings, HTTP 402, a time-out, an
  answer that does not add up), with each finding and its probability. The
  measured commit and the base are written by the compare job from the plan
  job's refs, not by the measured code, and a run whose base is not on main
  writes no check. Only
  a run of every open model writes a check, and a pass needs all 34 compared
  in full: a model whose dump is missing, a diff that failed, a dropped group,
  sampled property sets that one dump alone holds too often (none held by
  both while either holds any, or more than a tenth of them) or an unscored
  chunk makes it NOT MEASURED at best, and a run with a
  `models` filter writes none, so a narrower run's pass cannot stand beside a
  fuller run's fail. It is not a required check and posts no pull request
  comment. A pass means the dumps agree, not that the exports are identical:
  the dump samples five elements per entity. `docs/maintaining.md` says how
  an agent fixes a fail (change the code, or state the change specifically
  in the Summary) and answers a warn, and gives the command for a complete
  run. Failure-first tests of the diff and of the verdict logic (0.35
  against 0.3500001, a reorder with moved offsets, a bulk collapse that hides
  a different value, a gap that reads as a pass, an answer that does not add
  up, a chunk over the cap in non-ASCII text) run in the PII guard job.

- **A door's and a window's hand flip and facing flip are decoded
  (#1357, RE-343).** The decoded Door and Window records carry
  `m_hand_flipped` and `m_facing_flipped`, the `m_flippedX` and `m_flippedY`
  booleans of the instance's current native `FamilyInstance` record, on Revit
  2024 to 2027, and the typed Door view's `flip_hand` and `flip_facing` read
  them (they were null on every door; 1,588 of 1,588 doors of the 2024 to
  2027 samples carry them, equal to the booleans, run 42037272808 at
  18954f33). No door or window is stored as a reflection: the first orthonormal block of its element data has determinant
  1 on all 1,439 doors and windows scored against Revit's export (Measure run
  42014420180 at f65fc702, and Snowdon Towers scored on a Mac) and on all
  1,820 of the Autodesk samples of 2024 to 2027 (run 42024111444 at
  e2f8accc), so the transform holds neither flip. `m_opening_facing` is that
  transform's Y axis: a half turn from the unflipped one where
  `m_hand_flipped` is set, and unchanged by `m_facing_flipped`. For a door or
  window set in a plain wall rvt-rs binds, Revit's own placement is the
  stored axes reversed where the two flips differ, so the direction it faces
  (the placement's +Y, where IFC4 has the leaf open for a family whose swing
  lies on its +Y side) is `m_opening_facing` reversed there. That is exact on
  all 653 such doors and 626 such windows of Core Interior, RE1
  Architecture, the BIM4LCA office as built and at permit, the R2025 models
  and Snowdon Towers, and on 41 doors and 105 windows of held-out models
  (counts only). The two office files are one building and the R2025 models
  one layout six times: counted once, 456 doors and 346 windows. The case
  where the flips differ (287 doors and 286 windows, 186 and 146 counted
  once) is measured on three 2024 buildings, one 2025 door, one 2026 house
  and no 2027 element. The 160 doors rvt-rs binds to no plain wall follow the
  rule on 111 and not on 49, and are not claimed. Within one symbol, a left or
  right `OperationType` of Revit's export differs between two doors exactly
  when the flips differ, on 161 of 162 pairs as built, 161 of 161 at permit
  and 108 of 108 on Snowdon Towers. The fields are carried, equal to the
  booleans, by 1,820 of the 1,820 doors and windows of the Autodesk samples
  of 2024 to 2027 and by none of 2015 to 2023, whose doors and windows are not
  decoded as Door and Window records (run 42024111444). On 2024 and 2025 the
  flips are read in the pass that already reads the doors' and windows'
  symbols and add none; on 2026 and 2027 they add one. Against `main` the IFC
  is byte for byte the same on all 111 models of the unfiltered run
  (42024095189) and, over 64 files, no peak memory moves by more than 1 MiB
  and export time by more than 1.2% on the files of 2023 to 2025 or 1.6% on
  those of 2026 and 2027 (run 42024099050), where the same measurement varies
  by 7% on files the change does not touch.

- **The 25 Areas of Autodesk's `rac_advanced` 2021 save are written, from
  Revit's plan topology alone, and say `area_plan_topology_unchecked`
  (#1434).** The 2021 save holds both states of the model: 16 area boundary
  lines have two copies in two partitions that differ in box (0.0246 ft),
  each naming the same Level and scheme at the same height. A group of
  lines with such a conflict is no longer unreadable: the lines'
  arrangement is not read (which copy is current is not known from the
  partitions), and the Area is drawn from Revit's plan topology, which reads
  the current copy of each line and is checked, as everywhere, against the
  area Revit saves for the loop and the Area's point; an Area may name a line
  whose copies conflict in place. A copy that names another Level or scheme
  or lies at another height, a design option, a stray height, a missing line
  and a line off the Level still make the group unreadable. The 2021 save's
  25 regions equal the 2022 save's in vertex count, holes, arcs and area
  (25 of 25), and differ from the 2020 save's on exactly the 7 Areas where
  the 2020 and 2022 saves differ (0.23 to 2.77 ft2), so the current copy is
  the later state; the only Revit export of the project shows the earlier one,
  which equals 18 of the 25 outlines and does not show the other 7, so the
  Areas say `area_plan_topology_unchecked` and not `area_plan_topology`
  (Griffin's decision for this save; the 2022 to 2024 saves keep
  `area_plan_topology` by his release rule). A same-project gate holds the 2021 save to
  the 2022 save, Area by Area, in GlobalId, number, name, storey, outline
  source, body area, top surface, height and lowest vertex. Of the models of
  a Measure run of 117 exports, one moves, this one; the held-out models are
  byte for byte as before, so none of them exercises the change, and no
  public file has a design-option area line, so that refusal is kept by
  construction and shown only by the helpers' tests. Report:
  `reports/element-framing/RE-333-area-plan-topology.md`, section 10.

- **Every `IfcSpace` says what it is: `RvtElementRecordGeometry.SpaceKind` is
  `room`, `area`, `mep_space` or `family_instance` (#1437).** An importer
  could not tell an Area from a room by asking what it is: the only marker in
  a default export was `SpaceOutlineSource`, which says where an outline came
  from, and a filter on its values stops catching Areas when a release reads
  them by another rule (the plan topology of #1428 gives them values of their
  own). `SpaceKind` is taken from the Revit class of the record the space was
  read from (a room, an Area, a Revit Space (MEP), or any other element whose
  own "Export to IFC As" names `IfcSpace`, #763, as `family_instance`), so it
  holds whatever the outline source is, a body-less Area included, and in an
  export with no geometry too. `SpaceOutlineSource` is unchanged and keeps
  meaning only where an outline came from, which is what decides whether to
  trust a space's area; read `SpaceKind` to tell the kinds apart and never
  filter on the outline source's values (`docs/rooms.md` now leads with which
  property answers which question). No `mep_space` is written today: rvt-rs
  reads no Revit Space from a file's partition records, which says what it
  writes and not what the files hold. The same value is `space_kind` on each
  room of the MCP server's `list_rooms` and in a last column, `space_kind`, of
  the room schedule (`rvt-schedule --schedule rooms`, Python's `schedule_csv`,
  the viewer's room schedule); the MCP summary gains `space_kinds`, and its
  `room_count`, which counts every space, is described as that (the tool
  contract in `docs/mcp-tools.*` moves with it). A space whose property set is
  not `RvtElementRecordGeometry` (in principle a wall or floor read through the
  schema walker) is written without the property, never with a guess, and
  counted: the diagnostics gain `exported.space_kinds` and
  `exported.spaces_without_kind`, and `rvt-ifc` says so in its summary when it
  is not zero. `RvtElementRecordGeometry` is rvt-rs's own set, so the property
  is in no `Pset_` count, no `decoded.parameter_value_count` and no
  property-set census. The library builder `ifc::build_ifc_model` writes the
  property sets its caller gives it and adds no kind. A new check,
  `tools/re/space_kinds_vs_ifc.py`, runs in Measure on every model and sample
  and in the IfcOpenShell job of CI. It fails on a space with no kind, with an
  outline source it does not know, or with a kind that disagrees with its
  outline source, and its run total fails on a model whose result saw fewer
  spaces than its export counted or whose geometry export lost the outline
  source; CI shows it failing on copies of Core Interior's export with the
  property renamed, every room called an area, and every room's outline
  source made an Area's, and runs it on an export with no geometry. On
  Measure run 42027776156 (head 04d1b101) the 117 models and Autodesk
  samples exported held 4,453 `IfcSpace`s, every one with a `SpaceKind`:
  3,733 rooms, 628 Areas and 92 family instances, and no `mep_space`. The
  BIM4LCA office holds 308 at both stages (234 rooms, 30 Areas, 44 family
  instances). The check passes on all 117: no `area` outside the Area outline
  sources and no other kind inside them, on the 66 models and samples that hold
  spaces and among them the 18 held-out ones and the six `rac_advanced` saves
  scored against the IFC2X3 export (counts only).

- **Revit 2025 projects draw beams along their axes and tread landings as
  their treads, and read a sketch line's arcs (#1440, RE-345).** Beams are
  drawn as on 2024, 2026 and 2027, tread landings as on 2024 (a monolithic
  landing stays its box), and a floor, ceiling or roof whose sketch holds
  an arc takes the arc where the 2025 save drew a chord: rac_advanced's
  floor has the 170 vertices of its 2024 save, not 24, and its roof and
  ceiling equal the 2024 save's. **No Revit export checks any of it on
  2025**: the 25 models with Revit's own 2025 exports (puzzbobb's 24 and
  RE1 Architecture) hold no beam, no tread landing, no shaft opening and no
  floor, ceiling or roof with an arc, and their exports are byte for byte
  unchanged. Each is held to the same Autodesk project's 2024 save instead
  (`tools/re/bodies_across_saves.py`, which Measure runs and fails on): 391
  beams and 5 tread landings of rac_advanced and rst_basic are equal per
  element, and of the 7 elements whose sketch holds an arc, 2 (a floor and
  a roof) are compared as drawn outlines and equal, the other 5 being
  record boxes or having no body. That cannot catch an error the 2024 save
  shares, and it compares bodies, not quantities or property sets; none of
  the 19 held-out and spent models changes (Measure run 42091457764, head 830a4d94).
- **Shaft openings are cut from floors, ceilings and roofs on Revit 2024
  only, and no longer on 2026 and 2027 (#1440, RE-345).** #1426 had turned
  the cut on for 2026 and 2027. The only shafts measured on a 2024 or 2025
  Autodesk sample, two in rac_advanced, cut nothing, so the check against
  the same project's earlier save cannot catch a bad cut, and no file of
  2025 to 2027 is known to exercise one. On those releases the floors,
  ceilings and roofs stay whole, and the export's diagnostics count the
  shaft openings it did not cut as the `skipped` item `shaft_opening_not_cut`.
  No export changes by it: in Measure run 42091457764 (head 830a4d94) of
  111 exports, 108 are byte for byte `main`'s, where the cut is still on
  for 2026 and 2027 (the 3 that move are the Autodesk 2025 samples), among
  them all 10 Autodesk samples of 2026 and 2027 and 5 held-out models of
  those years; the only 2026 and 2027 shafts counted, 2 in each
  rac_advanced, cut nothing.

- **Furniture Systems are exported as `IfcSystemFurnitureElement` (#1444,
  RE-352).** The element-record sweep reads `OST_FurnitureSystems` under the
  unchanged instance rule on every release the exporter reads, and
  `category_map` writes the class Revit's own IFC4 export writes for it, with
  no `PredefinedType`, where it mapped `FurnitureSystem` to `IfcFurniture`
  before. The evidence is one project and 3 elements: Revit's public IFC4
  export of Autodesk's 2018 `rac_basic` sample holds 3
  `IfcSystemFurnitureElement`; the 3 written for that save are its 3 by `Tag`,
  none extra and none missing, with record boxes equal to Revit's bodies to
  0.0000 ft (counts and distances from Measure run 42135914864; the export has
  no licence and nothing is quoted from it). It writes no property set on them,
  so no `Reference` is written, and it contains all 3 in a space: rvt-rs
  contains 2 of the 3 in a space on Revit 2024 and later only, as for every
  furniture class, and one of the 3 has no Level and stays in the building.
  Revit's export types all 3; rvt-rs types none. In that run 13 of 111 exports
  move, every `rac_basic` save of 2015 to 2027, each by 3
  `IfcSystemFurnitureElement` and nothing else; 98 are byte for byte main's,
  and the same-project gates pass. No other model in Measure places a
  Furniture System, so there is no held-out check. Not exercised: nested
  components of the class's own category and the design-option and phase
  filters.

- **The viewer's info panel lists an element's saved parameters (#35, RE-207).**
  Selecting an element shows, under "Saved parameters", the same entries
  `rvt-elements` and Python's `decoded_elements()` list for it: instance
  parameters, then the type parameters it takes from its type, each with its
  name, where it is defined, its value as saved and its unit where the entry
  has one (`3 ft · 0.9144 m`, `Yes`, a material's or family type's name beside
  its ElementId). A parameter with no name in the file is listed by its id
  behind a fold that groups such parameters by why they have none (#255), and
  what the read could not list (family slots no definition resolves, an
  instance parameter whose value is its family's default) is counted in a line.
  The panel says so when the file is of a release outside Revit 2015 to 2027,
  when it is of one of 2015 to 2022, which are read experimentally (RE-281),
  when the element has no saved parameters, when the element has no ElementId
  in the export and when a read failed, with "Try again"; it shows and
  announces that it is reading while it does, and moves focus to the result of
  a read a button started. It is a new binding, `savedParameters`
  (`docs/viewer-api.md`), and it does not read at the open: the export, and so
  every IFC, GLB and diagnostics output, is unchanged. An element is read when
  it is selected, in a worker of its own that reads the `File` the page
  already holds and ends with its answer, so nothing of the file stays
  resident beyond what the page held before, the open's worker is not held up
  by a read, and a read that fails leaves the rest of the panel where it was.
  **Nothing is requested once the first file is open**: both workers are
  bundled into the page and start from blob URLs (the page's `worker-src` gains
  `blob:`, and nothing else in its Content-Security-Policy moves,
  `docs/viewer-privacy-posture.md`), the decoder's glue is a static import and the
  compiled wasm the first open's worker hands over serves the next open, the
  reads and the main thread's IFC, plan and CSV exports, so selecting an
  element, exporting and opening the file again fetch nothing and a page taken
  offline after it has loaded does all three. The first page load is heavier:
  the page's one script grew from 959.29 kB to 999.56 kB (262.82 kB to 271.47 kB
  gzipped) between the build of d1de8adf and this head, 40.27 kB and 8.65 kB
  gzipped, of which the glue and the two workers (12.91, 12.43 and 9.82 kB, a
  third larger as base64) that used to be fetched later as files are most, and
  this round's panel code the rest; the site holds one script, one stylesheet
  and the wasm. A read has taken 0.74 to 1.04
  times the open's wasm memory, rising with the file, and nothing above 910 MiB
  has been measured, so a selection is read on its own only while the wasm
  memory the page holds is at most 1 GiB; above that the panel asks first,
  saying that clicking may close the tab, and after a read has run out of
  memory, or its worker has gone without a word, that file is read only when
  asked. The page keeps each element's answer, up to 64 elements with the least
  recently selected going first, until a file is opened again, so an element
  selected a second time is not read again. Measured in the viewer tests (run
  42125048710, head 3146f14e, CI runner, WASM without LTO): on Einhoven every
  one of the 37 tree elements was selected and the panel held, for the 28 with
  parameters, exactly the 369 entries `rvt-elements` prints (332 instance, 37
  type, 8 named), and showed the empty state for the 7 with none; a read took a
  median of 153 ms (most 210 ms) in an instance of 17 MiB, against 23 MiB for the
  open's worker. On Core Interior four elements of four categories match
  `rvt-elements` too; a read took 1.5 to 1.9 s in 275 MiB, against 326 MiB and
  17.3 s for the open. On the BIM4LCA office (62 MiB) a selection read on its
  own: 8 entries in 3.9 s in 943 MiB, against 910 MiB and 66.6 s for the open,
  and the page then kept 2 answers, 4,654 characters of JSON. A read takes 0.74
  to 1.04 times the open's wasm memory while it runs, which rises with the file.
  Under a V8 cap of 927 MiB of wasm memory, between the office's open (910) and
  its read (943), the open succeeds and the read really runs out of memory: the
  panel says so and reads that file only when asked from then on. That office is
  not compared with `rvt-elements`.
- **File Status's Parameters row stops saying the rest are still to be
  decoded (#35).** It said so while `rvt-elements` and Python already listed
  every element's saved parameters (RE-207). It now counts the values written
  to the IFC property sets and points to the info panel. The panel's "Not
  recovered" note for an element with no property set now names "IFC property
  sets", not "Revit parameters", which it lists itself.

### Changed

- **Every number on the README and the supported profile now names the model
  it was measured on, and CI fails one that does not (#1192).** A new step of
  the PII guard job, `tools/ci/measured_numbers.py`, reads both pages and
  fails a number with no model of the new "Model index" table in
  `docs/reference-models.md` within 300 characters of it in its own clause
  (nothing inside parentheses ends a clause), a number on a `local only` or
  `not shared` model whose clause does not say so beside that model's name,
  and an index row that does not say who can fetch its model
  (`redistributable`, `fetched by Measure`, `local only` or `not shared`).
  Every numeral counts, in a heading as in a cell, except what its spelling
  marks as an identifier (an issue, a report, a hash, a date, a Revit
  release, a Measure run id, a tool's version), and a number that is not a
  measurement carries `<!-- w10: reason -->`, which fails without a reason or
  with nothing to excuse. On the pages as they were at `f009976e`, 51 clauses
  failed the first form of the check (39 with no model, 12 on a local-only
  model that did not say so); each now names its model, and the Snowdon
  Towers, 255ribeiro, flowbim, pre-2015 and client-model figures say "local
  only" or "not shared". The one figure that traced to no committed report,
  Snowdon Towers' 1,030 of 1,078 walls with faces exactly Revit's, is from a
  local run recorded in #446 on 2026-09-27 and is labelled as not re-run
  since. The saved-mesh figures that had gone stale (the BIM4LCA office's
  1,294 of 1,709 and RE1's 207 and 148 of 214, from October) are now Measure
  run 41850769925's at `e024f03c`: the office 1,554 of 1,650 as built and
  1,638 of 1,738 at building permit, with 265 of 269 doors and all 269
  windows exact as built, RE1 148 of 228 exact and 228 of 228 within 0.01 ft
  of Revit's box, and Core Interior 138 of 138 (RE-78's 158 included 20 shading
  devices, which run 41850769925 does not score). The README and the
  profile say how to read the figures, `docs/reference-models.md` gives the
  one Measure dispatch that re-runs them on the models anyone can fetch, with
  what it does not cover, and `docs/maintaining.md` says how a new number
  keeps to the rule and that a pull request which moves a figure edits the
  page. Measured by `--refusals` (a copy of the pages with a bare number, a
  number far from its model, a number in a heading, an unflagged local-only
  model, a local-only word beside another model's name, a marker without a
  reason, a marker with nothing to excuse and an index row with no
  fetchability, each refused), by the pure judgement's failure-first tests
  (twenty-two mutations of the script each fail them) and by the check
  passing on the pages. Not measured: that the re-run command reproduces the
  pages' figures, which nobody has compared, and whether each figure belongs
  to the model in its clause.

- **The parameter definitions pass no longer builds the values of the
  `Family` and `FamilySymbol` records it reads two fields of (#1335).** The
  pass decoded each of those records in full and kept the record's `m_id` and
  its `m_categoryId` or `m_familyId`, which was most of the pass on a Revit 2015
  to 2023 project. It now reads them as before, with the same checks and the
  same refusals, and builds only the fields it keeps
  (`native_parameters::decode_graph_projected`, `Options::graph_fields`). No
  export changes. The figures are in the pull request.

- **A pass over a file's records makes the plan of each class once, not once
  for each record (#1335).** Decoding a record works out, for each class it
  meets, the chain of classes and the key of each field. A pass over a Revit
  2015 to 2023 project's records did that again for every record, which was
  3.1% of the time of the 33 samples' exports. The passes now keep the plans
  between records (`native_parameters::PlanCache`), in a cache that holds at
  most a million units of plan cost and empties itself past that, so that a
  schema with huge classes cannot make it grow without bound. The graphs are
  the same. No export changes. The figures are in the pull request.

- **The native readers' shared preamble is read in less time (#1335).** Before
  its first pass over a file's records every native reader reads the schema,
  the history, the element index and the extensible storage catalog. The
  element index no longer searches its layout for the leaves of each of its
  thousands of rows, the schema's checksums no longer format a string for each
  byte, and the catalog reads the document it takes its schemas from without
  building the values of the fields it ignores. No export changes. The figures
  are in the pull request.

- **The read of the wall records no longer builds the values of the fields it
  ignores (#1335).** The read that takes each wall's type and whether its top
  is joined to a roof or floor decoded every `SWall` and `ArcWall` graph in
  full to read two fields of it. It reads and checks the same bytes, with the
  same refusals, and builds only the fields it keeps. No export changes. The
  figures are in the pull request.

- **A project whose last phase holds no room writes the rooms of the latest
  phase that holds any (#1359).** Autodesk's `rac_basic_sample_project` has its
  14 rooms in Working Drawings, the second of three phases, and none in its
  last, Learning Content, so the room-phase rule of RE-326 (0.14.0) left out
  all 14 and an importer's room count went from 14 to 0. Where no phase is
  named and the last phase holds no room, `rvt-ifc`, the Python bindings, the
  MCP server and the viewer now write the rooms of the latest phase that holds
  any, because rooms all in one phase cannot overlap and the only Revit export
  of this model holds them. It is a choice by Griffin (2026-11-05), not a
  measurement of Revit's default: no Revit export made at the last phase of
  such a project exists, and a project with no room in its last phase gets
  earlier rooms that Revit's default might leave out.
  - The diagnostics name the phase and count its rooms in the new
    `room_phase_fallback` (`docs/schemas/export-diagnostics.schema.json`), absent
    in every other export. `rvt-ifc`, `rvt-gltf`, `rvt-schedule` and `rvt-sheet` print a line, and the
    Python bindings raise a `RuntimeWarning` naming the phase and saying to pass
    `phase=` (`decoded_elements()` and `element_counts()`, which take no phase,
    say to export with it). `RevitFile::room_phase_fallback` gives the phase.
  - Every other element still follows the last phase. A phase named
    (`rvt-ifc --phase`, `phase=`, the MCP server's `phase`), the last included,
    is exact and writes none of these rooms. A project with a room in its last
    phase, and a project whose rooms are all in it, are unchanged, byte for
    byte. Only placed rooms count towards the last phase holding a room and towards
    the latest phase that holds any: an Area or an unplaced record does not, and
    a placed room whose phase is unset, unread or not one of the project's is
    written as before and does not either.
  - Measured (Measure run 42035308270 at `ea0c7449`, 111 exports): the 2016
    to 2027 saves of `rac_basic_sample_project` each write 14 `IfcSpace` of kind
    `room` (0 before), and the other 99 exports, held-out ones included, are byte
    for byte as before. The default is byte for byte the `--phase "Working
    Drawings"` export and pairs 14 of the Revit 2018 export's 17 spaces by
    GlobalId; `--phase` with the last phase writes what 0.14.0 wrote on all ten
    saves of 2018 to 2027 (RE-326 §9.4). One project exercises the fallback, and
    the Python warnings are checked by `tests/python/test_room_phase_fallback.py`
    in the full CI, which has not run on this head; the viewer's label has no
    browser test.

- **The provenance table names the records behind a wall's layers, an I-section
  beam, an Area's Levels, a tread landing and a spiral run, and the `current`
  copy rule is read in CI (#1368).** With `--element-provenance`:
  - a wall drawn at its type's thickness or in its layers lists the type's
    layers (`wall_type_layers`), a beam or column drawn as its type's I section
    lists the type's six section entries (`type_i_section`, before listed for
    a column only), and an Area lists the block of every Level its base and
    height compare (`area_levels`, one part for each Level);
  - a landing drawn as its runs' tread lists the landing type's record and the
    record of each distinct run type of its stair's runs, and a spiral run
    lists its run type's record, as a straight run already did.
  - The export itself is unchanged: only the provenance table gains sources.
  - The `current` copy rule is on one of the 92 models Measure run 42111951132
    read, the BIM4LCA office at building permit (the sketch lines of 3 of its
    floors, the Sketch curve list of one), and on none of the readers
    `instance_transform`, `sketch_line_data` or `roof_line_data`. CI's tier-two
    job now also fetches that permit file (`research/bim4lca-models.tsv`), and
    `tests/value_sources_offsets.rs` reads its sources at their bytes and fails
    if the office stops taking the rule.
  - No element has `selected_frame_box`, a body whose profile no profile reader
    noted, on any of the 92 models (the same run), so the text that called it a
    missing source is now the stated fallback it is. Of the four values #1368
    listed, one still has no source, with its reason in
    `docs/export-diagnostics.md`: a rectangular column's two sides, which only
    decide whether its box quantities are written. Other inputs of the bodies
    have none either (a wall's top, base and end rules, its orientation and
    flip, a tapered wall's face angle, a beam's rotation, the partner lists of
    joins and cut-loop sizes); #1368 did not list them and they are filed
    apart.
  - Measured on this head (ee449c3b): the 31 exports of Measure run 42118484766
    (no `ifc_flags`; Core Interior, Einhoven, the four RE1 models and
    Autodesk's samples of 2024 to 2027) are byte for byte main's
    (`ifc.sha256`), so the default export is unchanged, and its
    `same-project gates` job passed. Run 42118489328 (the 92 models with
    `--element-provenance`) counts the new sources: `wall_type_layers` on
    1,973 elements of 29 models, `area_levels` on 483 Areas of 17,
    `type_i_section` on 557 elements of 5 (none on a CI gate model),
    `native_run_type` on 84 of 17 and `native_landing_type` on 82 of 8;
    `current` stays on the one permit office (the 2021 `rac_advanced`, after
    #1450, has none: its 25 Areas read `area_plan_topology`) and
    `selected_frame_box` on none. The Corpus health job, which runs
    `tests/value_sources_offsets.rs` on the permit office and the new readers,
    passed on bd3466c8.

- **A wall that embeds a curtain wall is drawn from the mesh Revit saved for
  it where Revit raised its base (#1385, RE-340).** RE-335 found that Revit
  draws such a wall less the curtain wall's prism and less whatever cuts its
  end stubs, and that no record read says which walls keep the stubs. The
  wall's own saved graphics do: on the BIM4LCA office, as built and at
  building permit, the lowest vertex of the saved mesh of each of the 113
  walls that embed a curtain wall is Revit's lowest point to within 0.001 ft.
  Fourteen stand 8.2021 or 9.514 ft above their base, the curtain wall's
  height, and 12 of those 14 meshes are Revit's body to 0.02 ft³ (the other
  two, one wall at each date, are 1.0% under Revit's IFC tessellation of the
  wall and equal to Revit's own `NetVolume` and `GrossSideArea` of it).
  `rvt-ifc`, `rvt-gltf`, the Python package and the viewer now draw a wall
  that embeds a curtain wall from its saved mesh when the mesh starts at least
  0.5 ft above the wall's base and decodes whole, on Revit 2024 to 2027, and
  leave the wall as it was otherwise: a wall that keeps its stubs (2103576 as
  built, 2177158) is unchanged, and so is a wall with a door, window or other
  opening hosted in it (the void its own curtain wall cuts does not count), a
  Tag shared with another entity or a base cut at a floor (#1384).
  - The base of all 14 walls Revit raises, 6 as built and 8 at permit, is now
    Revit's with the curtain walls' openings (#1165) written (Measure run
    41973316830 at 3ad0762f): walls whose base and top are Revit's
    rise from 598 to 604 of 607 as built and from 577 to 585 of 589 at permit,
    and no other wall's row moves.
  - The 0.5 ft threshold was fitted and checked on those same two files (113
    host states, 10 distinct raised walls); the held-out models and
    Autodesk's samples gave no host to score.
  - The glTF draws the six hosts as built, which have two or more layers, in
    one material, not in their layers; the hosts of one layer keep their
    layer's colour (#1405). The IFC keeps every host's layer set.
  - `--no-saved-meshes` keeps the walls' records. A file with no curtain wall
    reads no wall record for this, and the exports of Core Interior and RE1
    Architecture are byte-identical.

- **A door or window set in a plain wall is placed on Revit's axes, and its
  `IfcLocalPlacement` says which way it faces (#1447, RE-347).** rvt-rs
  wrote the default axes (X along the model's +X, Z up) on every door and
  window, which were Revit's on 115 of the 521 doors and 226 of the 540
  windows it binds to a plain wall in Core Interior, RE1 Architecture and
  the BIM4LCA office as built and at permit (Measure run 42066757105), and
  it writes no `OperationType`, so the placement's +Y was all the file said
  of the direction. Now the placement's X and Y are the stored axes,
  reversed where `m_hand_flipped` differs from `m_facing_flipped` (RE-343),
  Z up and the origin unchanged, and
  `RvtElementRecordGeometry.PlacementAxesSource` says so. Revit's axes are
  the same on all 1,061 doors and windows so placed in those models, on all
  24 doors and windows of the R2025 models and on all 145 of the 2024 to
  2027 held-out models' (counts only), and every door's and window's body
  and opening is where it was in world coordinates, as saved meshes and as
  record boxes, with its GlobalId, type GlobalId and host unchanged (a
  quarter turn swaps the two sides of its box; Measure runs 42106711251 and
  42106731054). So `rotation_deg` of the elements CSV and the viewer's
  schedule, the Rotation row of the viewer's info panel and
  `rotation_degrees` of the MCP server's `get_element` are now set for every
  such door and window not on +X (0, 90, 180 or -90), and on one whose body
  is its box `width_ft` and `depth_ft` (the panel's Width and Depth) swap on
  a quarter turn: width is now along the placement's X, the door's width,
  where it was the model's X extent. Core Interior's export gains 138
  `PlacementAxesSource` values and 98 directions. A door bound to no plain
  wall, an element with no stored transform (Revit 2023 and older) and an
  instance turned off the model's axes keep the default axes. On Revit 2025,
  2026 and 2027 the same elements are held to the 2024 save of the same
  project (`tools/re/bodies_across_saves.py`): 124, 96 and 82 doors and
  windows of two Autodesk sample projects, 80, 58 and 45 of them off the
  default axes, are the same, and 28 and 42 the 2024 save places stay on the
  default axes in the 2026 and 2027 saves of `rac_advanced` (counted, not
  scored). The origin is still the record box's centre, more than 0.5 ft
  from Revit's on almost all (#1465). Not measured: Snowdon Towers against
  Revit's export, and the memory of the added read of the stored transforms
  (its time is below what a run resolves).

- **A release tag that is not signed, not on `main` or a hotfix branch, out
  of step with a version file, or on a commit whose `CI passed` is red or
  that CI never ran on now fails in seconds, before any wheel builds, and a
  release that failed part way can be re-run (#1457).** A `preflight` job in
  `publish.yml`, which every other job needs, refuses a tag that is
  lightweight or whose signature GitHub does not report as verified, whose
  commit is on neither `main` nor a `hotfix/*` branch, or that disagrees
  with any of `rvt`, `rvt-py`, `rvt-rs-mcp` and `rvt-wasm`'s versions, the
  `=X.Y.Z` pins on `rvt`, `viewer/package.json`, `CITATION.cff`, both lock
  files or the `CHANGELOG.md` section (`tools/ci/release_preflight.py`; the
  same version check runs on every pull request), and waits for `CI passed`
  on the tagged commit through the new `wait-for-check` action, which Deploy
  viewer waits through too. A commit whose every file CI's push filter skips
  is refused as a release tag, since a docs-only commit after a red one
  would otherwise ship the red commit's tree. The crates.io steps skip a
  version crates.io already has, PyPI skips files it has from the second
  attempt on, releases run one at a time so two tags cannot race for the
  container image's `latest`, and the docs.rs preview uses CI's pinned
  nightly and banner check where it floated on `nightly`. The Intel macOS
  CLI is now run on the arm64 runner under Rosetta before it ships, and
  `SHA256SUMS` is attested with the archives, so verifying that one file
  checks every archive (the wheels and the sdist join them in #1466).
  Measured by the pure judgements' failure-first tests
  (each comparison mutated in a scratch copy fails them), by running the
  check read-only against v0.15.0 and v0.14.0 (it passes) and against
  missing, mismatched and off-branch cases (it refuses each), by `wait.sh`
  on real commits, by `publish.yml` dispatch run 42072957401 (the version
  check refusing a scratch branch in 14 s, with the other jobs skipped) and
  by Release binaries run 42072884597 at head 0fbd4e38 (the Intel build's
  smoke under Rosetta, which the image already had). Not yet measured: a
  tag push, which is the only run of the signature, branch and CI checks and
  of the crates.io state check on a hosted runner.

- **The wheels and the sdist on a release page are now in `SHA256SUMS` and the
  build-provenance attestation, so `gh attestation verify <file> -R
  DrunkOnJava/rvt-rs --source-ref refs/tags/<tag>` works for each of them and
  the one attested `SHA256SUMS` lists every file on the page but itself and the
  provenance bundle (#1466).** Up to v0.15.0 both covered the CLI archives
  only, though the page carried seven wheels and the sdist (the files PyPI
  gets). `release-binaries.yml`'s `checksums` job now hashes every file the run
  holds for the page (the archives, and for a release exactly seven wheels and
  one sdist, the `wheel-*` and `sdist` artifacts of the publish run), a new
  `attest` job checks the page against `SHA256SUMS` (every line has its file
  with that hash, every file a line) before it attests `dist/*`, and `upload`
  attaches the signed page as one artifact, holding `contents: write` and no
  longer the OIDC token. A final `page` job fails a release whose page is not on
  it (fifteen files), so a skipped `upload` goes red. A pull request or weekly
  run builds no wheels, and its `SHA256SUMS` lists the archives alone. A
  `workflow_dispatch` input, `python`, builds one wheel and the sdist so the
  same jobs can be run before a tag (RELEASE.md): nothing is attached or
  pushed, but the attestation it signs is real, which is why the documented
  `gh attestation verify` commands name the tag's ref. `RELEASE.md` also says
  how to recover a release whose page chain failed. `tools/ci/check-workflows.py`
  fails when the checksums, attest, upload and page jobs stop describing the
  same files, when a tag run could go green with no page (`upload` needing more
  than `attest`, `attest` or `upload` not running on `inputs.upload`, a job
  between them, a job-level `if` above them: a skipped job skips every job that
  needs it, however far down, which skipped `image`, `attest` and `upload` in
  the first design's pull request run 42134335447), and when the wheel and
  archive counts leave publish.yml's `build-wheels` matrix and the `build`
  matrix behind. Measured by Release binaries run 42137536209 (a dispatch with
  `python`, head 872f6b6f): the downloaded `release-page` artifact passes
  `sha256sum -c SHA256SUMS`, `gh attestation verify -R DrunkOnJava/rvt-rs`
  passes on its wheel, sdist, `SHA256SUMS` and a Linux and the Windows archive,
  the one attestation's eight subjects are the seven `SHA256SUMS` lines and
  `SHA256SUMS` itself, `docker/stage.sh` passes in the run's image job and on
  the page directory and on the two Linux archives with `SHA256SUMS` alone
  (the lines of absent files skipped), and the image was neither pushed nor
  attested; by the pull request run 42137542924 at the same head (no wheels:
  `SHA256SUMS` lists the five archives, the image job's `stage.sh` and smoke
  test pass); and by `--source-ref refs/tags/v0.15.0` passing on v0.15.0's
  macOS arm64 archive and `--source-ref refs/tags/v9.9.9` refusing it. The
  exact wheel count and the `page` comparison were run on stand-in files and
  against v0.15.0's 15 release assets with `gh release view`, not in CI. Not yet
  measured: a tag push, the only run of the `upload` and `page` jobs, of the
  seven-wheel count (the rehearsal builds one) and of the image push after its
  two-step download.

- **A door set in no wall is placed at its stored origin (#1494, RE-361).**
  Where Revit's export gives a door no opening, its `IfcLocalPlacement` origin
  is the origin of its stored transform (RE-357); rvt-rs now writes it there
  for a door it binds to no host wall, whose stored transform is read and
  upright. The door's body, a saved mesh or the record box, stays where it was
  in the world, and no GlobalId, host or opening moves. What does move is the
  location those doors report: `location_feet`, so their location in the
  elements CSV, the scene graph and the MCP server's views goes from the
  record box's centre to Revit's own placement origin, and the viewer's
  schedule with them. The doors carry
  `RvtElementRecordGeometry.PlacementOriginSource`, `stored_origin_no_opening`
  (the MCP server lists it among their trust markers), and the decoded Door
  record gains `m_opening_placement_origin_x`, `_y` and `_z`. On the 153 such doors
  of the office files and RE1 Architecture, every origin is within 0.001 ft of
  Revit's, and no door with an opening gets one. Their axes are not changed: no rule
  with 0 wrong was found.

- **The provenance table lists a source for a wall's top, base and end rules, a door's placement, a wall's orientation and flip and a tapered wall's face angle, and the offsets test fails on a written `*Source` value that has neither a source nor a stated gap (#1498).** With `--element-provenance`:
  - an element lists a noted source for any reader its own `*Source` property values name, beside its `BodySource` and `ProfileSource`: a wall with a `BodyHeightSource`, `BodyBaseSource` or `BodyEndSource` lists its own native record (`native_wall`, which holds the join lists and cut pairs, `m_realJoined` among them) and, for the top and base, the frame of each cover it names (`wall_top_covers`, `wall_base_cover`) and, for the end, the native records of the walls its lists name (`native_wall_joined`); a door or window with a `PlacementAxesSource` lists its stored transform (`instance_transform`), its own native record (`native_opening`, the two flips) and its host wall's record (`opening_host_wall`, the wall's line); a wall drawn from its centreline lists its orientation and flip (`wall_orientation`, `agreeing`: the scan compares every copy) and the native record, and a tapered wall its type's face angle (`wall_type_face_angle`, `agreeing`);
  - `tests/value_sources_offsets.rs` has a row (`SOURCE_VALUES`) for every written `*Source` value of the models of the CI gate and fails on one with none: a row is sourced (and the test requires the readers it names on every element that carries the value), covered by another table, unread, or a declared gap with its reason. 20 values are declared gaps, in four kinds listed in `docs/export-diagnostics.md`: 12 bindings to a Level or a storey, 2 family names chosen by a table, 1 column height (`partition_geometry_node_after_joins`) and 5 schema-walker values that none of the 92 models of Measure run 42136996423 writes;
  - the export itself is unchanged: only the provenance table gains sources. A door's cut-loop size, a beam's cross-section rotation and a section's dimensions have no `*Source` property to key a row on and keep their stated reasons.

### Fixed

- **The MCP server's levels and element lists are the ones the IFC export
  writes (#1327).** The scorer of #1327 found that a part of a stair or a
  curtain wall had a level of its own in the server's rows, or none, where
  the IFC puts it on its whole's storey, so the levels' element counts
  differed from the export's (5 Revit 2025 models, 4 held-out, and
  the BIM4LCA office, 599 rows of each of its two files, 542 of them
  curtain-wall members at "Roof" where the IFC says Level_05), and that the
  office's lists held 47 `IfcCovering` rows the export does not write, the
  further pieces of an element of separate loops that it writes as items of
  the owner's body. Both are the writer's own decisions, now
  `rvt::ifc::step_writer::placements` (folded, a door's or window's opening,
  in a space, part of an aggregate, on a storey, in the building) and
  `storey_of`, which the writer places every element with and the server
  reads: an element's `level` and the levels' `element_count` are the storey
  the export contains it in, through its space, its whole or its host, and a
  folded piece is neither a row nor in `total` or `by_ifc_type`. The
  summary's `elements.folded_into_bodies` counts the folded pieces by IFC
  entity (the diagnostics' `exported` counts held them until #1424). The tool
  output schemas change (`folded_into_bodies`), and so does the contract in
  `docs/mcp-tools.json`. The diagnostics' `storey_bound_elements`, the rooms
  schedule (`room_rows`) and `every_storey_holds_an_element` read
  `placements` too, so a part counts as bound to its whole's storey in all
  of them, and `folded_into_bodies` is always in the summary, empty where
  none is folded. Measured with the scorer of #1404 and `base=none`, on code
  head d0836833 (this change merged with main 6f4bc534, which has #1406):
  run 41965720746 passes 32 of 32 models, the 6 reference models, the 24
  Revit 2025 models and the BIM4LCA office as built and at permit. On main's
  server before, the office failed the counts, element membership, element
  storeys and levels checks (47 coverings, and 599 of 12,974 and of 13,020
  rows, 542 of them curtain-wall members and the rest stair flights and
  landings, 8 and 7 of 10 levels) and the 5 Revit 2025 stair models failed
  element storeys and levels (runs 41949018446 and 41949021964). The IFC
  text `rvt-ifc` writes is the same on 34 of 34 models, the reference
  models, the four office files and the 24 Revit 2025 models, in run
  41965717668 (head d0836833 against main 6f4bc534, `scorers=none`). In the
  diagnostics of that run only `exported.storey_bound_elements` moves, on 7
  of the 34 models: the office as built from 12,961 to 12,971 and at permit
  from 13,008 to 13,018, the Revit 2025 stair models 13 to 15 from 24 to 27
  and 16 and 17 from 24 to 25, the elements bound to a storey through their
  whole. The held-out models and Autodesk's samples were not run.

- **Revit 2015 files follow the export phase (#1332, RE-372).** An element that does
  not exist in the export phase and a room whose phase is not the export phase are
  left out, as on 2016 and later, and `--phase` is accepted on 2015. On one Revit 2015
  project with an export (counts only), the 3 air terminals Revit's export leaves out
  are gone and its 241 elements stay. Design options stay 2016 and later.

- **A wall with an edited elevation profile, cut back by its join from inside
  the wall beneath it, is drawn to that wall's face (#1396, RE-337).** The
  Basement walls of the BIM4LCA office that stand on the sloped footings of
  RE-336 were cut back, by the join of their record, to the face of a Level_01
  wall that overlaps them by 0.66 ft of 9.5; Revit's body at the base runs on
  to the face of the Basement wall beneath, which the record box's end lies
  inside. A wall with an edited profile that is a plain rectangle along a model
  axis, drawn from its record box, with a join trim, now has its end drawn to
  the faces of the walls its record joins there, where the cut of the wall's
  base still resolves over the longer plan (RE-336, #1411). A wall that keeps
  an end the rule would have drawn carries `BodyEndRefusal`, counted in
  `exported.wall_end_refusals` and logged as `wall_end_extension_refused`. A
  rule that did not ask the record's joins drew a Snowdon Towers soffit wall
  0.1016 m longer than Revit's. This corrects the 0.15.0 entry for #1384
  (RE-336), which says that 2 walls as built (4 at permit) are 0.123 ft
  (0.044 ft) off Revit's lowest point because their plans end short of Revit's:
  walls 2057668 and 2057670 have it as built and at permit, and 2058499 and
  2058501 with #1409 and #1411. How the end steps up the height is in the entry
  for #1410 (RE-348). The measured effect is in
  `reports/element-framing/RE-337-wall-ends-at-the-base-wall.md` and
  `reports/element-framing/RE-348-wall-end-join-order.md`.

- **A floor's void is tagged by the direction its first line is stored in, as
  Revit tags it, on 133 of 133 voids (RE-355, #1408).** Of the sketch lines of a
  void's loop, the one with the lowest ElementId runs along the loop's
  counter-clockwise walk or against it; against it that line is the tag, along it
  the line after it. That is Revit's tag on the BIM4LCA office as built (36 of
  36) and at building permit (35 of 35) and on Core Interior (62 of 62),
  including the void 3147725 that RE-339 left wrong, and on 12 of 12 voids of
  Snowdon Towers architectural (11 of 12 before, scored locally against Revit's
  export). Where the direction is not read (a curve in the loop, two lines on one
  edge, no stored ends) the tag is the one RE-339's rule gave. A reading from the
  order the first two lines were drawn in is wrong on 20 of Core Interior's 62 and
  is not used.
  - This moves tags in users' exports, and a moved tag moves GlobalIds: the
    opening's id is keyed on the tag, and the ids of its `IfcRelVoidsElement`,
    its `Pset_OpeningElementCommon` and its defining relation follow it, so 4 ids
    move on each of the 6 voids that moved. The id is not redesigned.
  - Measured on Revit 2024 only. Measure runs 42122776008 (6d3f6c73 against
    `main`) and 42131336084 (e77ef0dd, every scorer, 111 exports): 3147725 moves
    on both office files, 1 void on Snowdon architectural (now Revit's) and 3 on
    Snowdon structural, with no export to check them; the other 107 exports, the
    24 Revit 2025 models, the samples of 2025 to 2027 and the 19 held-out and
    spent models among them, are byte for byte `main`'s. No Revit 2025 export
    holds a floor void to check. Of the 19 held-out and spent models the probe
    read 11, each with no loop to read, and could not read 8 (Revit 2018 to
    2022): the rule is exercised on no held-out model.
  - The exporter reads a subset of what the probe reads and keeps RE-339's tag
    where it declines; why Revit does this is unread, and the rule for a first
    line stored along the walk rests on 9 loops of the office (RE-355 sections 3
    and 4).

- **A profile-edited wall's end follows, at each height, the first wall its
  record joins there that stands (#1410, #1409, RE-348).** Revit steps the
  ends of the BIM4LCA office's Basement walls 2057668, 2057670, 2058499 and
  2058501 up the wall's height, to the face of the Basement wall beneath, then
  of a wall between it and Level_01, then of Level_01's. Which of the walls
  that stand at a height it follows is the order in which the wall's record
  lists the walls joined at that end (`JoinEndGStep.m_realJoined`; the order of
  `m_geomJoined`, the last wall that stands and the least ElementId each give a
  different end on at least three of the four walls). **That order rests on 4
  walls in one project.** The end is now drawn as the wall's own plan to its
  top and one column from its base for each distance the end reaches beyond the
  next, each to the height the walls it follows stand to, from the plans of
  those walls: a plain rectangle, a layered profile, or a wall whose top a
  floor cut in steps, read by its pieces up to its own top. The labelled
  approximation of #1396 (`BodyEndSource`
  `wall_end_to_base_wall_upper_step_omitted`, which omitted the steps above the
  wall beneath) is retired; a wall carries `BodyEndSource`
  `wall_end_stepped_to_joined_walls` or keeps the end its record gives with a
  counted reason. A kept end carries `BodyEndRefusal` (`joined_wall_not_read`,
  `joined_wall_top_not_read`, `joined_wall_does_not_meet_the_end`,
  `joined_walls_do_not_reach_the_wall_height`,
  `joined_wall_face_inside_the_wall`, `joined_wall_faces_overhang_the_lower_end`,
  `base_cut_lost_on_extended_plan`, `base_not_cut_over_the_extended_plan`,
  `base_solid_not_made_over_the_extended_plan`, `wall_stands_in_the_added_end`)
  only where the faces of the joined walls read before the refusal show it
  short of or past them: a wall that already has Revit's end carries nothing,
  and the reason is counted in `exported.wall_end_reads_unmarked`, which also
  holds `end_join_curve_not_at_the_box_end` and `end_join_list_not_read`. The
  diagnostics count the walls that take the rule and the ends that depend on the
  order (`exported.wall_end_sources`, `wall_base_sources`,
  `wall_ends_order_dependent`); only those ends test the order, and on the office
  they are the 4 as-built ends, 3 orderings chosen after about four alternatives
  had been tried. The rule is on its own release list (Revit 2024). Only the two
  BIM4LCA office files take it (4 walls each); no wall of the other 109 exports
  of the unfiltered run does.
  On the office (Measure 42101944733 at
  `006f5f98`) the four walls have Revit's end at every height and no point of
  their plan above Revit's top; 2058499 and 2058501 at building permit go from
  0.044 ft above Revit's lowest point to Revit's, and the wall beneath them
  (1347201, a layered wall whose top a floor cut in steps) is read. **Their
  volume is not Revit's to 0.01 ft3**: it is 0.209 ft3 over (0.207 at permit)
  on all four, which is the far end's sloped top that the profile line does
  not reach (#1473), not the ends. Walls 2056705 and 2056770 at permit keep
  their ends with `joined_wall_face_inside_the_wall`: Revit cuts them back
  above 182.087 ft and the rule only adds. No other wall's body in either
  office file changes, and every other export of the unfiltered run (Measure
  42111477633 at `e5ff94f4`) is byte for byte `main`'s but Autodesk's Snowdon
  Towers Architectural, whose two refusal markers the policy above removes where
  the wall already has Revit's end. The measured effect is in
  `reports/element-framing/RE-348-wall-end-join-order.md`.

- **A wall whose base join names a floor or footing that covers its plan only
  in part is cut at that element's top over the part it covers (#1411,
  RE-349).** Revit takes a wall's base up to the top of the element its base
  join names where the two share plan, and leaves it at the wall's own base
  where they do not. The BIM4LCA office's Basement walls 2058499 and 2058501
  as built stand past the edge of their footing (2058504, 0.24 ft of their
  plan lies outside it, 0.55 ft once their ends reach Revit's), so they kept
  their base, and RE-337 (#1398) kept their ends 0.31 ft short of Revit's
  because the longer plan would only have added base. They are now the convex
  pieces of their plan inside the footing's outline, cut along its sloped top,
  and the rest of the plan from the wall's own base (`BodyBaseSource`
  `wall_base_sloped_at_floor_covering_in_part`; `wall_base_cut_at_floor_covering_in_part`
  for a level top), a wall whose covered part is not read keeps its base and says
  so (`BaseCoverRefusal` `partly_covered_plan_not_read`). Measured on the
  office as built against Revit's export (Measure 42099826920 at `c413f382`):
  the two walls keep Revit's lowest point (174.2126 ft), take Revit's end
  (y 75.4183 ft), and their volume goes from 4.497 ft3 over Revit's (35.302
  against 30.805) to 0.147 ft3 under (30.658), a net of the upper step that
  RE-337's end then omitted and the far end's sloped top (#1473), the same as
  wall 2057670's to 3e-5 ft3. No other wall of the office as built changes (607
  bodies compared), and the office at building permit is byte for byte what
  `main` writes. A wall whose base join names an element that covers the whole plan is decided by
  those alone, as before, and one that covers part counts only where none
  covers the whole; a cut that resolves and whose solid cannot be made keeps the
  prism and says so (`BaseCoverRefusal` `base_cut_solid_not_made`). No other
  model takes the rule (Measure 42111477633, 111 exports, and 42119702779, 68
  exports: only the two office files differ from `main` there). The level top of a partial cover, a
  partially covered wall with no end extension, a footing with an opening under
  a wall and a wall whose ring is clockwise or concave are exercised by no
  measured wall. The measured effect is in
  `reports/element-framing/RE-349-wall-base-covered-in-part.md`.

- **The elements CSV lists each element on the storey the IFC writes it on,
  and only the elements it writes (#1417).** `rvt-schedule`, Python's
  `schedule_csv()` and the viewer's "Download element schedule (CSV)" took a
  row's `level` from the element's own storey and listed the further pieces
  of an element of separate loops, which the export writes as items of the
  owner's body and not as elements. They now read the writer's own decision,
  `rvt::ifc::step_writer::placements` and `storey_of`, as the MCP server and
  the rooms schedule do (#1412): a part of a stair (a run, landing or
  stringer) or of a curtain wall is on the storey of its whole, an element in
  a room on the storey of the room, an opening on the storey of its host, and
  a folded piece has no row. The `level` and `level_elevation_*` cells are
  empty where the export puts the element in the building only, or where no
  storey is reached through its room, whole or host. An element with folded
  pieces is written with a body of several items, which one width, depth and
  height cannot state, so its size cells are empty: a ceiling of separate
  loops is now one row with empty size cells, where it used to have a row
  for each loop (47 fewer rows on each office file). Rows are still in
  level-elevation order, so a row whose level moves moves in the file. The
  columns do not change. The user guide says what the level column is.
  Measured with `probes/examples/probe_1417_elements_csv_levels.rs` in
  Measure run 41983667731 (code head 38ddc579, `base=none`,
  `scorers=none`), where all 8 models pass the probe's six checks (header,
  cells, count, level-elevation order, every row's ElementId, IFC type and
  level against the placements, and the empty size cells of an element with
  folded pieces) and the model `rvt-ifc` writes has the same written elements
  and storeys as the schedule's model: on the BIM4LCA office as built the
  schedule goes from 13,021 rows to 12,974 and at building permit from 13,067
  to 13,020, the 47 rows left out in each being `IfcCovering` pieces the
  export folds into another element's body, and 599 rows of each change level
  (542 `IfcMember`, 35 `IfcStairFlight` and 22 `IfcSlab`, all parts of a
  whole): 542 from one storey to another and 57 from none to a storey. The 47
  pieces belong to 19 ceilings as built and 20 at permit, none without an
  ElementId, and the width, depth and height of the row of each of them are
  empty. On the Revit 2025 stair models r2025-13, r2025-14 and r2025-15 the
  27 rows stay 27 and 3 take a level (2 `IfcMember`, 1 `IfcStairFlight`), and
  on r2025-16 and r2025-17 the 25 rows stay 25 and 1 `IfcStairFlight` takes
  one; no row moves from a storey to none on any of them, and none of them
  has an element with folded pieces. 2024 Core Interior keeps its 970 rows,
  none moves and none has folded pieces. The office's 12,974 and 13,020 rows
  are the element rows the MCP server's consistency check counted on those
  files in run 41977947391. The IFC text `rvt-ifc` writes is the same on 34
  of 34 models, in run 41983679071 (head 38ddc579 against main 40b86b80,
  `scorers=none`, nordic and r2025 on). No row moved for an element in a
  space or an opening on these models, so those two paths are not exercised
  by this measurement. The held-out models and Autodesk's samples were not
  run, and `rvt-ifc --saved-meshes`, `--family-extrusions` and the viewer's
  saved-mesh path were not measured for folding. The
  diagnostics' count of folded pieces followed in #1424.

- **A click in the viewer picks what is drawn (#1422).** A raycast does not look
  at `visible`, so with a category hidden a click on what is drawn behind it
  selected the hidden element instead, and a category hidden before the meshes
  arrived stayed drawn. A new browser test,
  `viewer/tests/folded-pieces.spec.ts`, opens the BIM4LCA office as built, hides
  every category but one, and clicks until a click lands on a piece folded into
  another element's body, read from the raycast's own hit: the panel, GlobalId
  and tree row are the owner's, it states no size, and the category and schedule
  counts are the elements the export writes, and a stair flight shows the storey
  of its stair. It runs in the viewer's browser
  tests on a hash-checked copy of the model, and says NOT MEASURED if the host
  of that copy cannot be reached. Before this, nothing in a browser exercised a
  folded piece.

- **The viewer's Schedule panel, element info panel and scene tree show each
  element on the storey the IFC writes it in, and list only the elements it
  writes (#1422).** `build_schedule`, `element_info_panel` and
  `build_scene_graph` took an element's level from its own storey and listed
  the further pieces of an element of separate loops, which the export writes
  as items of the owner's body and not as elements. They now read the
  writer's own decision, `rvt::ifc::step_writer::placements` and `storey_of`,
  as the MCP server, the rooms schedule and the elements CSV do (#1412,
  #1417): a part of a stair (a run, landing or stringer) or of a curtain wall
  is on the storey of its whole, an element in a room on the storey of the
  room, an opening on the storey of its host, and a folded piece has no
  schedule row and no tree node. An element the export contains in the
  building only is in the schedule with an empty storey and in the tree's
  `Unassigned` storey. An element with folded pieces has no size rows in its
  panel, as its CSV row has no size cells: its body is several items, which
  one width, depth and height cannot state, and the panel does not call the
  size missing. A folded piece is still drawn in the 3-D view, so a click can
  pick it; the click selects the element whose body it is part of (the panel
  names it in the new `entity_index`), with its tree row, its GlobalId, and a
  highlight and a zoom over the element and all its pieces, and a category's
  highlight and zoom cover the pieces of the elements in it. The category
  list counts the elements the tree lists. A part of several wholes sits
  under the first, as the export has it. A door or window is nested under its
  host wall in the scene graph only where both are on the same storey, an
  element whose host is not a node of its own is a node of its storey, and so
  is one of two elements that host each other. The JSON the bindings return
  keeps its shape, with `entity_index` added to the panel.
  `docs/viewer-api.md` says what the storey is. Measured with
  `probes/examples/probe_1422_viewer_panel_levels.rs` in Measure run
  41986216576 (code head a8785bb9, `base=none`, `scorers=none`, nordic and
  r2025 on), where all 8 models pass the probe's 10 checks (the schedule's
  rows and the tree's nodes are exactly the written elements, each on the
  storey `storey_of` gives it, in level-elevation order; every sampled
  panel has that storey, names the written element in `entity_index` and,
  for an element with folded pieces, states no size and does not list
  `extents` as missing; a folded piece has the panel of its owner): on the
  BIM4LCA office as built the schedule goes from 13,021 rows to 12,974 and
  at building permit from 13,067 to 13,020, the 47 rows left out in each
  being `IfcCovering` pieces, and 599 rows of each change storey (542
  `IfcMember`, 35 `IfcStairFlight` and 22 `IfcSlab`, all parts of a whole):
  542 from one storey to another and 57 from none to a storey, and the tree
  and the panels move the same 599; the tree's `Unassigned` storey goes from
  60 elements to 3 as built and from 59 to 2 at permit. The panels of all 19
  elements with folded pieces as built and all 20 at permit have their size
  rows left out. On the Revit 2025 stair models r2025-13, r2025-14 and
  r2025-15 the 27 rows stay 27 and 3 take a storey, and on r2025-16 and
  r2025-17 the 25 rows stay 25 and 1 takes one, each from none to a storey;
  Einhoven's 37 elements move none and lose none, and neither it nor the
  Revit 2025 models has an element with folded pieces, a null result that
  does not exercise the fold path or the size rows. The IFC text `rvt-ifc`
  writes is the same on 8 of 8 of those models, in run 41986242000 (head
  a8785bb9 against main b510ff3c, `scorers=none`). These figures cover the
  Rust side: the three functions' output and the panel's `entity_index` and
  extents. The probe did not count elements that host each other, so that
  path is not known to be exercised. Not measured: the viewer's TypeScript
  (the click selecting the written element, the highlight and zoom over its
  pieces, the category counts, the part under the first whole) in a real
  browser, since no browser test exercises the fold path; the held-out
  models and Autodesk's samples; 2024 Core Interior and RE1; and the
  viewer's saved-mesh model beyond what the probe builds
  (`ExportOptions::default()`). The probe's checks derive from `placements`,
  so they prove the viewer's views follow the writer's decision and not that
  the decision is right (the MCP scorer reading the written IFC does, run
  41977947391). The diagnostics' count of folded pieces is #1424's: before it
  the Schedule panel's total read 47 below the diagnostics' on the office.

- **The diagnostics count the elements the IFC writes, and say how many
  pieces the writer folded (#1424).** `exported.building_elements` and
  `exported.by_ifc_type` counted the further pieces of an element of
  separate loops that the writer folds into the body of the element that
  owns them (#823, `Placement::Folded`), which the IFC does not write as
  elements: Measure run 41977947391 (head 2fc83c47, before this change)
  counted 158 `IFCCOVERING` on the BIM4LCA office as built and 122 at
  permit, where the IFC and Revit's export hold 111 and 75, and 13,021 and
  13,067 building elements where the MCP server lists 12,974 and 13,020.
  They now count what `step_writer::placements` says is written, and the
  new `exported.body_pieces_folded`, by STEP entity type, counts what was
  folded: the figure of `building_elements` and `by_ifc_type` from before is
  the new one plus it. A folded piece clones its owner's properties and
  storey, so it also left `building_elements_with_geometry`, `body_sources`,
  `bounding_box_bodies`, `bounding_box_spaces`, the refusal and outline
  counts of `exported`, the decisions of `rvt-ifc --export-decisions` that
  those are totalled against, the `skipped` geometry-gap items
  (`unsupported_geometry_*`) and `decoded.parameter_value_count`, which
  counted the `Pset_` values of the sets cloned onto a piece, none of which
  the writer emits; they cannot be recovered from `body_pieces_folded`.
  Otherwise a piece would count its owner's source, refusals and gaps twice,
  and `body_sources` would no longer add up to `building_elements`.
  `storey_bound_elements` already left it out. The ratios made from these
  counts (`source_coverage.exported_element_fraction` and
  `geometry_element_fraction`) and the element terms of `confidence.score`
  move with them. The IFC is unchanged. The descriptions of the MCP tools
  `open_model`, `get_summary` and `export_gltf`, and the output schemas'
  `folded_into_bodies` and `building_elements`, change, so the contract in
  `docs/mcp-tools.json` does: `export_gltf` reports the IFC's counts, and
  now says in `notes` how many pieces its glb draws beyond them.
  `tools/re/mcp_answers_vs_ifc.py` checks the field against the summary's,
  `parameter_value_count` against the `Pset_` values in the IFC, and, with
  `--require-folded-field` on Measure's head side, that the field is there.
  **Upgrading:** a reader of `exported.building_elements` or `by_ifc_type`
  sees fewer on a file with folded pieces (47 fewer `IFCCOVERING` on each
  BIM4LCA office file; none of the other 33 models whose diagnostics run
  41989882391 kept has one), and `body_sources`, the geometry-gap items and
  `parameter_value_count` change likewise; add `exported.body_pieces_folded`
  to `building_elements` and `by_ifc_type` to get their figures from before.
  Measured in Measure run 41989882391 (code head 0b1bc044, `base=main`
  a4044dc8, `nordic`, `r2025` and `holdout` on): `ifc.sha256` is identical
  on all 53 models, and the diagnostics of the 35 models that keep them move
  on the two office files only. As built `building_elements` goes from
  13,021 to 12,974, `building_elements_with_geometry` from 12,787 to
  12,740, `by_ifc_type.IFCCOVERING` from 158 to 111 and
  `decoded.parameter_value_count` from 54,952 to 54,764; at permit from
  13,067 to 13,020, from 12,833 to 12,786, from 122 to 75 and from 55,145
  to 54,957; `body_pieces_folded` is `{"IFCCOVERING": 47}` on both and
  empty on the others, and the `skipped` items do not move on either (no
  piece was in a gap). `parameter_value_count` falls by 188 on each file,
  4 for each of the 47 pieces. On the head side every consistency check of
  the MCP scorer reads 0 differ on all 53 models, with
  `--require-folded-field` on and the new check that
  `parameter_value_count` equals the `Pset_` values in the IFC passing on
  every model; on the base side that check differs on the two office files
  only (the IFC holds 54,764 and 54,957 values, main's diagnostics count
  54,952 and 55,145), so that equality had not been checked before. Not
  measured: the held-out models' diagnostics (their results keep counts
  only), Autodesk's samples, and the `export_gltf` note on a model with
  folded pieces.

- **The export's peak memory is back to what it was before the walls that
  embed a curtain wall were drawn from their saved meshes (#1429).** #1401
  (RE-340) reads the graphics of those walls in the saved-graphics pass, and
  the pass kept every owner's decoded graph until it returned, so the 42 hosts
  of the BIM4LCA office as built (41 at permit; about 1.6 MiB each, drawn from
  their mesh or not) stayed under every later batch of door and window
  symbols: 66 MiB of live heap, and 76 MiB of resident memory at the pass's
  peak (917 to 993 MiB as built, 979 to 1,057 MiB at building permit). The
  pass now frees an owner's graph with its batch, unless another owner names
  it (a shared symbol that names one already freed has it read again), and
  meshes the owners that name no symbol of their own in a batch of their own
  before it reads any symbol. The batches of symbols are the ones they were,
  except that where an owner names only other owners the later ones can differ
  in size; the elements are the ones they were. `rvt-ifc`'s peak on the office
  is 914 MiB as built and 971 MiB at permit (the parent of #1401: 916 and 979
  MiB), and Snowdon Towers architectural's 1,062 MiB, against 1,108 MiB on
  `main` before this change. A file with no such wall is not changed: Core
  Interior, the RE1 models and
  Einhoven are at 1.00 of the parent's peak. The time does not move: 0.98 to
  1.02 of `main`'s across the 68 files timed. The IFC does not move: the 111
  files an unfiltered Measure exports (34 models, 19 held-out and spent
  models, 58 Autodesk samples) are byte-identical to `main`'s.
  `probes/examples/probe_1429_export_peak.rs` profiles the heap at the
  export's peak by call site.

- **Exports hold the EXPRESS rules of the IFC4 schema that
  IfcOpenShell's default validation leaves out; Revit's way of breaking one
  is behind `--revit-set-placement` (#1430).** Measure's `rules:` check
  (#1106) found three kinds of rule finding in what `rvt-ifc` wrote on main,
  in run 41988011289:
  `IfcShapeRepresentation.CorrectItemsForType` (an
  `IfcFixedReferenceSweptAreaSolid`, the body of a sloped beam, under
  `SweptSolid`, which IFC4 reserves for extruded and revolved solids: 14
  findings on `rst_basic` 2024, 2 on `rac_advanced` 2024, 10 on Snowdon
  structural 2024), `CorrectPredefinedType` (an `IfcAirTerminalType`,
  `IfcElectricApplianceType` or `IfcSpaceType` with `PredefinedType`
  `USERDEFINED` and no `ElementType`, 1 each on the BIM4LCA office as built
  and at permit) and `IfcRelAssociatesMaterial.AllowedElements` (a material
  associated with an `IfcSpace`, which that rule does not take: 1 on each
  office file and each terrain file). A sweep along a directrix is now
  written under `AdvancedSweptSolid`. A body of several solids takes the one
  type IFC4 accepts for all its items (`AdvancedSweptSolid`, `SolidModel` or
  `SurfaceOrSolidModel`), where it used to take `CSG` for any mix, which
  holds boolean results alone. A mix with a boolean result is written as the
  union of its items under `CSG`, a triangulated item of it as the faceted
  brep of the same triangles: IFC4 takes a tessellated boolean operand only
  where it is closed (`IfcBooleanResult.FirstOperandClosed` and
  `SecondOperandClosed`), and the face set records `Closed` as `$`. On
  hand-written STEP files, IfcOpenShell 0.9.0.post1 finds that rule on the
  union with the face set as either operand and none on the same union with
  the face set marked closed or written as the faceted brep, and finds
  `CorrectItemsForType` on extruded and tessellated items under `CSG` and
  none under `SurfaceOrSolidModel`. No current producer builds such a mix, so
  no current output changes. A type object whose `PredefinedType` is
  `USERDEFINED` now carries its own name as `ElementType` (an empty label
  where the type has no name), and keeps the `PredefinedType` and the GlobalId
  Revit's export has. An occurrence with `PredefinedType` `USERDEFINED` and no
  `ObjectType` read is written without a `PredefinedType`, which would break
  the same rule, and no measured model has one. An `IfcSpace` is associated
  with no material, and `rvt-ifc --revit-set-placement` (the MCP server's
  `revit_set_placement`) keeps the one Revit's export associates with each.
  Revit's own export of the office breaks the second and third rules the
  same way (measured on the as-built file with the same check: `USERDEFINED`
  types with `ElementType` `$`, and a material on `IfcSpace`), so what to
  write was Griffin's call, decided on 2026-11-05: valid IFC4 by default,
  Revit's behind the flag. The output of `--revit-set-placement` therefore
  breaks `IfcRelAssociatesMaterial.AllowedElements` on the 42 `Turning Free
  Space` instances of each office file and the 2 of each terrain file, as
  Revit's export does; the `ElementType` is set either way, which Revit's
  export leaves empty. The flag is the writer's entry points
  `write_step_with_space_materials` and `write_step_to_with_space_materials`,
  which `rvt-ifc` and the MCP server pick by the option; library, Python and
  wasm callers of `write_step` get the default. Everything that lists an
  element's material follows the default export, as the schedules follow its
  storeys (#1417, #1422): the elements CSV (`rvt-schedule`, Python's
  `schedule_csv()` and the viewer's download) and the viewer's info panel,
  schedule and material highlight list none for an `IfcSpace`, and the
  material's element count leaves spaces out. No schedule has the flag, so
  none lists a space's material. The glTF colours are left alone: a colour on
  a space's mesh is a drawing choice, not an association. Revit's export of a
  beam is an `IfcExtrudedAreaSolid` under `SweptSolid`; the sloped beam's body
  is still a fixed-reference sweep. CI's IfcOpenShell job now runs the same
  rules check, on the default output, on every export it writes (the tier-1
  projects, the synthetic pair, Einhoven, Core Interior with and without
  saved meshes and as the two witness steps write it, and the four RE1
  projects), after the witness steps so that a finding never drops the
  observations a replay fix needs, and fails on any finding. Its check no
  longer ends at the first message holding a `%` (an `IfcSpaceType` named `50
  % Space` made IfcOpenShell's logger raise and hid that finding and the
  ones after it), and Measure keeps the count of rule findings, with the
  entity and rule that name each, for the held-out models, whose `schema.txt`
  it deletes, saying `not read` where the rules check left no count. Measured in Measure run
  41994889680 (code head c4b6e861, default output, `base=main`, `nordic`,
  `r2025`, `holdout` and `samples` on, no `ifc_flags`; the base side
  reproduces the findings above): the `rules:` section reads `findings 0` on
  the 90 models whose export it checks (Core Interior, Einhoven, the four RE1
  projects, the four office files, 24 Revit 2025 models, one spent pair's
  model and 55 of Autodesk's 58 sample projects) and the 18 held-out models
  report `schema rules: findings 0` as counts. The other 3 samples (the 2024
  `rme_advanced`, `rme_basic` and Snowdon architectural) are over the 30 MB
  the schema step reads, so their rules are not measured, as before. Run
  42002548557 (head f3c87e48, the same flags) read by the main session: 104
  of 111 exports are byte-identical and 7 move, the 4 office files and the
  2024 `rac_advanced`, `rst_basic` and Snowdon structural, the 7 that had a
  finding, with 0 rule findings on every model checked, the held-out counts
  at 0 on all 18 and the MCP server's answers equal on 1,258 lines. No other
  output of any model differs between the two sides apart from `schema.txt`
  (run 41994889680, compared file by file with the timings, logs, diagnostics
  and MCP dump apart), which covers the type-object, GlobalId, tagged-space
  and property-set scorers. `element_materials_vs_ifc` pairs no `IfcSpace`,
  so it reads the same on both sides: the spaces' materials leaving the
  default output shows only as the finding going. With `--revit-set-placement`
  (run 42002553349, head f3c87e48, the two office files, `base=main`, the
  `schema`, `psets_vs_ifc` and `element_materials_vs_ifc` steps) the rules
  check reads `findings 1` on each, `AllowedElements` for the 42 spaces, where
  `main` reads `findings 4`, and the property set and material outputs are the
  same at `main` and at the head. CI's gate printed `findings 0` on 13 files
  in 48 s on the pull request's CI run at f3c87e48 (read by the main
  session). The three fixed kinds occur on none of Core Interior, Einhoven
  and the four RE1 projects, which read 0 on `main` in run 41994889680, so
  the gate would have passed those on `main`; the tier-1 and synthetic
  exports were not checked on `main`. That the gate fails on each kind was
  shown on Revit's terrain export (52 findings) and on a hand-written file
  with all three kinds (exit 1). Not yet measured on the head that carries the union,
  consumer, `USERDEFINED` occurrence and gate-position changes: the
  exports' bytes against run 42002548557's head side, the elements CSV, and
  the 2026 and 2027 sample exports that #1426 will give their findings.

- **A wall whose elevation profile was edited keeps its cut-out past the
  profile, to the wall's end (#1473, RE-358).** The sketch of a profile spans
  the wall's location curve, but the wall's body goes on past it at some ends
  (the record lists no joined wall at the ones measured), and the opening that voids the wall (RE-151) stopped at the
  profile's end: the stretch stayed as the wall's box had it while Revit's body
  goes on along the profile. Where a wall goes on past an end of its profile
  and an opening reaches that end with one vertical edge, the
  opening is carried on to the wall's end in two shapes: a sloped line along its
  own slope, and the cut-out of a notch (both neighbours level) straight out,
  told apart from the wall's own top or base by height and not by the order the
  outline lists them in (`ProfileStubSource` `wall_profile_sloped_past_*`,
  `wall_profile_level_past_*` or `wall_profile_sloped_and_level_past_*`). A wall
  that does not read so keeps what it had, with `ProfileStubRefusal`. Read on
  Revit 2024 only, on its own release list, and counted in
  `exported.wall_profile_sources` and `exported.wall_profile_refusals`.
  Measured on the BIM4LCA office: eight Basement walls take it on each file (six
  high ends, two low ends), each Revit's volume to 1e-4 ft3 (the four of #1473
  were 0.209 ft3 over, 0.207 at permit) with no top above Revit's, and two of
  them (#1474) move 0.2 ft3 towards Revit's. Two walls of Autodesk's `rac_basic`
  and one of Snowdon Towers Architectural take it as well; no Revit export
  checks the `rac_basic` walls (the report is
  `reports/element-framing/RE-358-profile-slope-past-the-profile.md`).

- **A wall whose record holds its previous location line beside its
  location curve is read by the curve `VWallDriver.m_pCrv` names (RE-360,
  #1475).** Revit 2027's save of `rac_advanced` stores a second `GLine`,
  owned by `WallRunTimeData.m_prevLocLine`, in 18 of its 180 `SWall`
  records, and `native_walls` left out any wall with a second curve, so the 8
  host walls of 14 doors returned no native line and the doors kept the
  default axes. A record with exactly two curves is now read where one is the
  level `GLine` `m_pCrv` names and the other is a `GLine` only
  `m_prevLocLine` names; every wall read before is read as before, and a
  `CurveLoop.m_curves` profile wall stays unread. Measured on Autodesk's
  sample projects of 2024 to 2027 (no Revit export), Measure run
  42134051521 at `0a0ba7ce` against the same code without it: 26 walls are
  newly read over every `SWall` record (`rac_advanced` 2027 18, `rac_basic`
  6, and 5 of those on the earlier saves, Snowdon Towers Architectural 2), of
  which 20 can reach an export (the 18, and `rac_basic` 709245 and 234869;
  none on Snowdon). `rac_advanced` and `rac_basic` then read the same walls
  on all four saves, the 14 doors are placed by the RE-347 axes rule and
  equal the 2025 save's in axes, origin, vertices and volume, and two
  curtain walls (`rac_basic` 709245 on every save, `rac_advanced` 149877 on
  2027) gain `Qto_CurtainWallQuantities.Length`, a value no Revit export or
  gate checks on these files. No other wall, placement, body or property of
  the five exports that move (`rac_basic` 2024 to 2027, `rac_advanced` 2027)
  changes. The unfiltered run (Measure 42141380072 at `5754c7f3`, 111 models
  and samples) moves six exports: those five and the BIM4LCA office as built,
  where one door gains the rule's axes and they are Revit's (doors placed by
  the rule 193 to 194, none differing from Revit's; its two newly read plain
  walls' lengths are Revit's, and no body, class or quantity of it changes).
  The other 105, Core Interior and the office at permit among them (281 and
  76 walls newly read), are byte for byte main's. No held-out model holds a
  previous location line. The 14 openings the doors cut have their doors'
  axes; their size is still a record box on 2027, as the cut loop of a door's
  symbol is read on 2024 and 2025 only.

- **A plumbing fixture whose type draws nothing visible is left out, as Revit's export
  leaves it out (#1496, RE-363).** It is counted in the diagnostics as
  `element_record_family_type_draws_nothing`. RE1 Plumbing's water closet tank is the
  one such fixture measured. Other categories are written whatever their type draws.

- **An IFC export holding a number that is not finite is refused instead of
  written.** A NaN or an infinity was written as `NaN` or `inf`, which
  ISO 10303-21's REAL cannot express and IfcOpenShell refuses. `rvt-ifc`
  (both of its write paths), the Python `write_ifc` and the viewer's IFC
  download now stop with an error that names the entity, and
  `rvt::ifc::step_writer::write_step_to` returns an `InvalidData` error. The
  text-returning `write_step` functions still write what they are given, and
  say so. No Revit file is known to reach the writer with one: this is a check
  in depth, and it changes no export of a finite model.

### Research

- **The saved-graphics reader reads an `InstanceInfo` with no `m_GRepId`
  (RE-342, #1158, #1190; STE1200, discussion #112).** Revit 2015 to 2022's
  `InstanceInfo` has no `m_GRepId`, which Revit 2023 to 2027 set to 0 in all
  220,660 of the objects measured (44 files); the readers took its absence for
  an unsupported representation, and now take it for the default one when the
  object holds only the three fields those releases declare, on the graphics
  of a file of Revit 2015 to 2022 alone (the next entry narrows this to that
  route; this one first let every route do it). With
  `Options::releases` widened to `NATIVE_RELEASES`, `native_saved_scene`
  draws 5,459 to 5,756 of the 6,241 to 6,433 `FamilyInstance` and
  `ImportInstance` elements of the three sample projects of each release 2015
  to 2022, where it drew 68 to 70. Against the same projects' Revit 2023
  downloads (separate Autodesk files, not re-saves), 41,687 of the 45,349
  elements both files draw whole have the same triangle count, world box and
  total surface area, among them every one of the 11,482 on the 2021 and 2022
  projects; 3,662 do not (the `rac_advanced` saves of 2015 to 2020, the
  `rst_basic` of 2016 to 2018 and the 2015 `rac_basic`, within 1 ft, not traced
  to a cause). That is one reader read against itself on two downloads, not a
  check against Revit. No export changes (92 of 92 IFC hashes equal `main`'s,
  with and without `--saved-meshes`, at `ab9503b1`), and saved graphics stay a
  research reading on 2015 to 2022. STE1200's patch also read deferred history
  episodes, `FacetedTopology9` under normal binding 2 and a set `m_bHasScale`;
  no file Measure fetches has the first two (0 of 92 files, 0 of 18 held-out
  models, 0 of 13 families) or draws a body from the third (on the 8 held-out
  scenes, 1,592 flagged instances in 3 pre-2023 models, the owners drawn whole
  and their triangles are the same with the flag read), so none of the three
  was in this change; the next entry takes them, on routes no export reads. The
  probe `probe_re342_saved_graphics_pre2023` counts what each meets in a file,
  and runs on your own: Measure runs 42015502520,
  42015507053, 42015511974 and 42015516241.

- **The saved-graphics probe reads deferred history episodes,
  `FacetedTopology9` under normal binding 2 and a set `m_bHasScale`
  (RE-342, #1158, #1190; STE1200, discussion #112).** STE1200 ran the probe on
  their Revit 2020 family of imported geometry and posted the counts: 109
  deferred episodes read, 325 `FacetedTopology9` objects under binding 2 whose
  47,968 stored normals are all within 0.1 degree of the winding, a set flag on
  26 identity and 10 rotation instances, and 36 owners drawn whole with 48,520
  triangles. That is a measurement reported on a file CI cannot fetch, and the
  only file that exercises the three. Each is read fail closed, narrower than
  their patch, and on a route no export takes. Deferred `Episode` pointers are
  read only when `Options::deferred_history_episodes` is set, which only the
  probe does; every other reader, the whole native preamble included, refuses
  such a history as before. `FacetedTopology9` under binding 2 is read only in
  the graphics of a file of Revit 2015 to 2022, and only where every stored
  normal follows the winding (`normals_follow_winding`); a mesh that fails is
  refused, never redrawn from the winding. A set `m_bHasScale` is read only on
  an `InstanceInfo` of `m_Trf`, `m_symbolId` and `m_cda` with a rotation
  matrix, so a mirror, a scale, or the flag on Revit 2023 to 2027 stays
  refused. This also narrows #1436: an `InstanceInfo` with no `m_GRepId`,
  which it let every route read as the default representation, is now read on
  the pre-2023 route alone and refused on every other, as before. What the flag means on the older schema is not read: if it marks a
  scale the matrix does not carry, a mesh drawn through it is wrong and nothing
  in the file says so. In one held-out model 57 owners pass the scale check and
  end with no primitive, and what they hold is not known. Saved graphics stay a
  research reading on Revit 2015 to 2022 and no export reads them. On
  the files CI fetches the changes alter no export and draw no body through a
  set flag: the unfiltered Measure run 42067738690 holds 111 of 111 export
  hashes identical to `main`'s (the default export; with `--saved-meshes`, 92
  of 92 at an earlier head, run 42054606416) and the
  same-project gates pass, and the owners drawn whole and their triangles are
  unchanged in the pairs and the 8 held-out scenes (42067745903, 42067762581).
  Their stop histograms do move against #1436, and owners gain a partial
  primitive, with none drawn whole: 29 owners change their stops in 15 of the
  24 pair files (21 leave the scale-check stop, 8 gain a `GText` stop and a
  partial primitive), and in 3 of the 8 held-out scenes 1, 58 and 2 owners do,
  two models gaining a partial primitive and 57 of the 58 in one model going
  from partial to `decoded_saved_graphics` with no primitive (182 to 239). No
  file CI fetches holds a deferred pointer or a `FacetedTopology9` object (0 of
  92 open files and 18 held-out models), so nothing in CI reaches those two
  paths and nothing in CI will catch a regression in them. The winding rule
  keeps 182 of the 185 `FacetedTopology0` objects and 336 of 336
  `FacetedTopology8` under binding 2 and refuses 3, all in one Revit 2024
  sample; its count on a mesh the reader would refuse for a malformed row or
  index is `refuses`, so a run on the contributor's family reads 325 only if
  all 325 of its meshes are well-formed.

- **Where an area boundary line lies against the wall it is picked from, and
  that the file stores it (#1261, RE-332).** The 127 area boundary lines of
  Autodesk's `rac_advanced` were compared with the record boxes of the walls
  their reference lists name: 61 name a basic wall, 50 a curtain wall, 16
  none. A line on an interior wall lies on its centre line, one on an exterior
  or concrete wall half the wall's thickness off it (the outer face), a
  curtain wall's 0.041 ft off its axis, and two lines are drawn on a face for
  one stretch and on the centre line for another. Joined line by line with the
  offsets the file stores for each line (`AreaMeasureCurveData`, RE-333), the
  position is that offset on all 127 (71 lines have one), and the step of the
  two compound lines is stored at the same place on both. A first reading of
  this, that nothing in the file says where such a line steps from face to
  centre, was wrong, and the rule it drew two Areas with is not in rvt-rs:
  the plan topology of RE-333 draws all 25 Areas from the stored offsets.
  `tools/re/wall_lines_vs_offsets.py` joins the two probes' output and fails
  on any line whose position and offsets disagree. Report:
  `reports/element-framing/RE-332-wall-hosted-lines.md`.

- **Rebar and Structural Beam Systems stay unread because no Revit export
  shows them, Furniture Systems are `IfcSystemFurnitureElement` in Revit's
  export, and the 199 elements RvtIO counts beyond rvt-rs on `rac_advanced`
  and `rac_basic` are family instances with no stored geometry that no Revit
  export of the project writes (#1444, #1445, RE-350).** Measure run
  42097940515 (59 models with a Revit export, 37 Autodesk samples of 2020 to
  2027, every probe exit 0) finds the files hold the three categories as
  frames, the same on all eight saves of each sample: 346 `Rebar` on
  `rst_basic` (14 on Snowdon Towers Structural), 16 Structural Beam Systems
  on `rst_basic` (15 with a body) and 1 on `rac_advanced`, 3 placed Furniture
  Systems on `rac_basic`. None of the exports measured holds an
  `IfcReinforcingBar`, and the one export of a project with a beam system,
  the three.js IFC2X3 export of `rac_advanced`, holds neither it nor any of
  the project's 38 framing instances, 23 of which the system's record lists,
  which is when revit-ifc's exporter writes no assembly (cited by class), so
  neither is added. Revit's public IFC4 export of the 2018 `rac_basic` (RE-326)
  writes its 3 Furniture Systems as `IfcSystemFurnitureElement` with no
  `PredefinedType`, and all 3 of the sweep's placed ones are in it (3 of 3),
  none written that the export lacks: the category meets the rule, and
  RE-352 exports it (one project, 3 elements; `category_map` mapped it to
  `IfcFurniture` before). The only `IfcElementAssembly` in a measured export are 3 in
  each of Vitoria's two 2018 files, Revit assemblies with no representation
  (RE-315) that no frame carries. The sweep reads 1,349 curtain panels, 3,276
  mullions and 100 doors on `rac_advanced` and 44, 144 and 16 on `rac_basic`
  on every save, every placed frame written. The 192 panels, the door, the 4
  mullions and the 2 panels RvtIO counts on its 2023 files are
  `FamilyInstance` records in the main model that every save of 2018 to 2027
  holds with no frame of any category (Measure runs 42104379244 and
  42108271073), and the public IFC4 export of the 2018 `rac_basic` holds none
  of the 6, the IFC2X3 export of `rac_advanced` none of the 193; their family
  types are not read from the file. `probe_re350_category_stages` and the
  scorer's `category census` lines are new, and the held-out, BIMcollab and
  racadv models keep only their counts.

- **A door's or window's `IfcLocalPlacement` origin stays at its record box's
  centre: Revit's is not the stored origin, and no rule with 0 wrong that
  rvt-rs can check gives it (#1465, RE-357).** Measure run 42127619521 (Core
  Interior, RE1 Architecture and the BIM4LCA office as built and at permit,
  against Revit's own exports, every job succeeded) finds Revit's origin more
  than 0.5 ft from the stored one on all 1,061 walled doors and windows
  (604 counted once, without the permit file), and its offset one value for
  every element of an `IfcDoorType` or `IfcWindowType` entity (160 of 160 type
  entities) and not for every element of a symbol (46 of 82, in the stored
  axes), which is revit-ifc's per-type `StyleTransform`, taken from the
  instance that creates the type and not recorded in the file. Two axes
  follow the element's own opening (x and z on all 919 walled elements of the
  two office files), the third, across the wall, does not (y on 375 of 385
  doors and 198 of 534 windows). The elements with both flips set agree on all
  three axes (391 of 391), and that is not written as a rule: the flip state
  has no mechanism behind it. Where Revit's export gives a door no opening,
  its origin is the stored one (153 of 153; #1494). The scorer
  `door_placements_vs_ifc.py` now prints the offset by type entity, symbol and
  flip state, and per axis against each export's opening.

- **Core Interior's 20 shading devices were never saved-mesh bodies on
  `main`: RE-78's 158 was the count before RE-95 (#1492).** RE-78 counted 158
  saved-mesh bodies on Core Interior (132 doors, 6 windows, 20 shading
  devices), and `mesh_bodies_vs_ifc` and `rvt-ifc --saved-meshes` count 138.
  Nothing changed on `main`. The 158 was measured on 2026-09-27, when the 20
  plates and the 116 rooms were record boxes; RE-95 (#467) closed the plates'
  sketch outlines and RE-101 (#473) gave the rooms their stored-solid
  outlines on 2026-09-28, both before the saved-mesh option landed on
  2026-09-30, and `saved_meshes::attach` replaces only a body whose profile is
  not resolved, as it did when it landed (73861770). No body that measured
  exact was lost, and RE-78 now says so under its first table. The new
  `tools/re/shading_bodies_vs_ifc.py` scores the plates' bodies by shape
  against Revit's, which neither `mesh_bodies_vs_ifc` (saved meshes only) nor
  `plan_profiles_vs_ifc` (plan outline only) did: on Core Interior all 20
  are within 0.01% of Revit's volume and every vertex within 0.001 ft of its
  body's surface (the largest distance 1.5e-06 ft), Measure run 42133202790.

- **RE-363: what separates the family instances Revit's export leaves out (#1496).** On 52 models with a Revit export (Measure runs 42518898114 and 42521763730; the held-out models as counts only), a family instance other than a door or window whose saved graphics decode to no surface and no diagnostic, with a branch left out as not visible, is never in Revit's export: 5,551 of 31,958 instances, no false negative, and 5,556 when read once per type symbol, also none. RE1 Plumbing's water closet tank is one of them. No export changes: the rule is not written yet.

- **mahach666's findings on the parameter store and on sketch to solid
  (RvtIO, Discussion #1241) are measured where bytes can state them and cited
  as reported, not measured, where only the Revit API did (#155, #156,
  RE-344).** On Autodesk's 58 sample projects, the BIM4LCA office, Core
  Interior, the RE1 and R2025 models and 13 families (Measure run
  42039153578), none of 641,480 positive parameter ids lacks a definition
  (the ElementId of a `ParamElem*` element of the same document); the 4,818
  records with no saved metadata are outside that test, and a negative id
  never has a definition, so that side tests nothing. Of the stored lengths
  that are whole or tenth millimetres, `mm * (1 / 304.8)` reproduces the most
  in the family slots (69% to 93% of the values on which seven conversions
  differ), as RE-310 found on 2024 to 2027, and `mm / 1000 / 0.3048` the most
  in the typed Double sets (58% and 59%). The set of conversions is not
  exhaustive: a seventh, `mm / 10 * (1 / 2.54) * (1 / 12)`, alone reproduces
  7,413 and 6,195 typed values. 17% and 21% of the typed values and 4% to 26%
  of the slots' match none of the seven, and 66% to 76% of those are more than
  3 units in the last place from every one, so a search for a length by its
  bits has to try several and cannot take a miss as absence. That the shares
  are alike on 2015 to 2023 and on 2024 to 2027 shows the stored bits survive
  re-saving, not that Revit's conversion is the same across releases: the
  samples are re-saves of one model. A `RevolutionElem` holds -1001802 and
  -1001803 on 550 form-and-file pairs (at most 207 counted once, files of
  identical form counts taken as one), each value a multiple of the
  14-decimal 3.14159265358979, a truncation of pi seven units in the last
  place below it: 517 pairs are 0 and 2 pi less seven units, and none of the
  555 nonzero values equals a multiple of pi/2 by bits, so a comparison with
  pi or 2 pi by bits fails on all of them. A `BlendElem` holds -1001804 and
  -1001805 on 1,229 pairs (at most 278), and none of 5,006 `SweepElem` (at
  most 1,383) holds any extent id. The `PartAtom` of the 2014 family of
  RE-327 gives each of its 25 shared parameters a GUID, 2,800 in 112 types,
  and the 12 corpus families, which use no shared parameter, hold none. The
  unit factors beyond the specs rvt-rs converts, the angle meaning of the
  revolution ids, the host areas and the volume rules rest on the Revit API
  and on models that are not in their folder; they are reported, not
  measured, and nothing that rests on NDA material is used (`CLEANROOM.md`,
  the maintainer's decision of 2026-11-05). Against the counts in RvtIO's
  table, `main` (5988d839, Measure run 42028541274; `main` has moved since)
  differs by category on `rac_basic` by 5 to 11 building elements (3
  Furniture Systems, a Roof, a Mass, and from 2023 on 4 mullions and 2
  curtain panels), on `rst_basic` by the 346 reinforcing bars and 16 element
  assemblies it does not read (and, on the 2022 file, the 1 space RvtIO
  writes), and on `rac_advanced` by 192 curtain panels, a door, an assembly
  and 2 Masses (and 25 Areas on the 2021 save). The causes of the roof, the
  panels, the door, the mullions and the Masses are not known. Revit's public
  export of `rac_advanced` holds the 1,349 panels and 100 doors rvt-rs reads
  (run 42031090233), but it is an export of the 2020-era state, not of the
  files where RvtIO reads 1,541 panels and 101 doors. `rac_basic`'s 14 rooms
  come back with #1435.

## [0.15.0] — 2026-11-05

0.15.0 makes an export say where its values came from and why it left
anything out, and draws more of what Revit draws. Two opt-in switches say
where each decoded element's record was read from and which copy its body,
profile or outline took its value from (`rvt-ifc --element-provenance`), and
why each element, body, opening or outline was left out
(`rvt-ifc --export-decisions`); the MCP server asks for both, gains
`explain_element`, and now names elements and their type objects as the IFC
export does. The provenance table's offsets are read again and
confirmed on the Revit 2023 to 2025 models CI holds and the decisions log's
on 2024 and 2025; those of Revit 2008 to 2022, the second-prologue frames of
2023, the decisions log's before 2024 and the schema-walk carrier are not
measured, the log keeps at most 5,000 entries a reason, and some values a
body reads (an Area's Levels, a wall's layers, a beam's I section) have no
recorded source. Every export of a Revit 2008 to 2023 project counts where
its element records stopped, so an export that writes nothing says at which
stage. Rooms take off the round columns their stored solid holds, and a
floor's void drawn clockwise takes the tag Revit's export gives it. Areas whose
boundary lines name each other are drawn to their joins: a rule read off the
one public export it is scored against (an IFC2X3 export of `rac_advanced`
of unestablished origin), so it is in-sample, and no Revit IFC4 export of
these Areas exists. A floor whose shape-edited corners lie in one plane is
drawn along that sloped top, and a wall it wholly covers starts from it; the
refusals this adds on Snowdon Towers and `rac_advanced` have no
redistributable Revit IFC4 export to score them. The opt-in
`rvt-ifc --family-extrusions` draws a family instance from its family's own
extrusions, arcs as true arcs, where they reproduce the mesh Revit saved
(Revit 2024 to 2027 files, measured on 2024 and 2025). The viewer's browser
tests are now part of every pull request's required check. The element
record stages follow hemanth1986's reports on #421 and the Area joins
inevitable-co-ai's on #1261, and puzzbobb's Revit 2025 models now check the
roof value sources in CI. Thanks to all three.

Not measured by the runs below, and said in each entry: whether a value
source names the right copy (copies that agree are equal; `current` is
`current_partitions`); the stage counts on Revit 2008 to 2014 and on the four
files of #421 that export nothing; the 2021 to 2024 saves of `rac_advanced`
against any export, and the 9 of its 25 Areas left out; Revit's own area of
the `rac_advanced` rooms, a concave or short column, and the rooms of the
2021 to 2027 saves and of Snowdon Towers; the BIM4LCA office's Basement walls
still off Revit's lowest point (2 as built, 4 at permit); and a family whose
extrusions include a void, an ellipse or an unclosed sketch, which keeps its
mesh.

Measured against 0.14.0 on every reference model, the Revit 2025 models,
Autodesk's sample projects, the held-out models and the families (Measure run
41965523134, this release's head at 1e56972e):

- On the BIM4LCA office the walls with Revit's base and top go from 586 to
  598 of 607 as built and from 565 to 577 of 589 at permit; no wall's top
  moves.
- On the same office the floor voids written with Revit's tag go from 27 to
  35 as built and from 27 to 34 at permit, and the values rvt-rs writes that
  Revit's export does not fall from 10 to 2 as built and from 16 to 9 at
  permit.
- On `rac_advanced`'s saves of 2015 to 2020 and 2022 to 2024, 13 more Areas
  are written with a body (2021's save and 2025 to 2027 are unchanged).
- Each of Autodesk's 33 sample projects of 2015 to 2023 now reports its
  element record stages, every count adding up: 192,811 of 197,463 markers
  give a record, 87,366 elements are handed to the export and the models
  hold 87,315 of them.
- Nothing moves from equal to different, the values Revit's export does not
  write are unchanged, and no value goes from equal to missing; every other
  scorer output, on every other model, is line for line 0.14.0's but for
  the witness files' version.
- Export time is 0.98 to 1.03 times 0.14.0's on all 68 timed files, and
  peak memory 0.92 to 1.04 times (Core Interior 309 to 323 MiB; run
  41944962415, `main` at 77d71cbb, before #1403 and #1405, neither of which
  adds work to a default export).

### Upgrading from 0.14

- Every export's IFC header names rvt-rs 0.15.0, so an export that is
  otherwise unchanged still differs from 0.14.0's there.
- A default export can change where these apply: a room whose stored solid
  holds round columns loses their area, and one with columns against its
  walls that kept its record box takes an outline (Autodesk's `rac_advanced`
  and `rac_basic` samples, and 2 of Snowdon Towers' boxed rooms); a floor whose shape-edited corners lie in one plane is
  drawn along its sloped top (`BodySource` `native_slab_shape_edit`), and a
  wall whose base join names one element covering its whole plan starts from
  that element's top (`BodyBaseSource`, `BaseCoverElementId`; the BIM4LCA
  office's Basement); and an Area of a Revit 2015 to 2024 file whose boundary
  lines name each other is drawn to their joins (`SpaceOutlineSource`
  `area_boundary_joined`). A floor's void whose sketch lines run clockwise
  in the order they were drawn is tagged with the line Revit's export tags
  it with, so its `Tag` and its GlobalId change (8 voids of the BIM4LCA
  office as built, 7 at permit). Partition streams are now taken in number
  order, which moved no Measure model's export.
- New trust markers in `RvtElementRecordGeometry`: `SlabShapeRefusal` on a
  shape-edited floor kept as its record box and `BaseCoverRefusal` on a wall
  that keeps its base, each naming the reason.
- New fields in the diagnostics: `decoded.element_record_stages` on Revit 2008
  to 2023 projects, the `skipped` reason `element_record_frame_stopped`, and
  `exported.slab_shape_refusals` and `exported.wall_base_refusals`. With
  `--element-provenance` or `--export-decisions` they also hold
  `decoded.element_provenance` or `export_decisions`; without them the
  diagnostics carry neither.
- `--family-extrusions` implies `--saved-meshes` and cannot be combined with
  `--no-saved-meshes`.
- The MCP server's element `name` is the `Name` the IFC export writes (an
  `IfcSpace`'s room number), absent only where it would be the
  `<Revit class>-<ElementId>` placeholder of a room with neither number nor
  name; rows and `get_element` gain `long_name`; `element_type` gains
  `ifc_type` and `name`, its `element_id` is the type object's `Tag`, and it
  is absent where the export writes no type object. A client that reads
  `name` as always present or `element_type` as always served must allow for
  both. The tool output schemas (`docs/mcp-tools.json`) say so.
- The MCP server builds the provenance table and the decisions log each time
  it opens a model, so `open_model` takes longer than in 0.14.0 on a large
  model; neither is in `get_diagnostics`.

### Added

- **`rvt::ifc::step_writer::written_names` and `type_objects` (#1327).**
  `written_names(ifc_type, name, type_guid, property_set)` returns the
  `Name` and `LongName` the IFC writer writes for a building element (an
  `IfcSpace`'s room number and room name, or `<room name>:<ElementId>`
  without a number) and whether the name is the exporter's own `<Revit
  class>-<ElementId>` placeholder. `type_objects(model)` returns the type
  objects the writer makes (`TypeObjects::groups`, one per type, entity and
  predefined type of the elements named `Family:Type`, and `rooms`, one
  `IfcSpaceType` per room whose name was read), each with its entity,
  `Name`, `Tag`, property sets and the entities it types. `write_step`
  writes with both, and the diagnostics' count of the type objects'
  parameter values reads `type_objects`, which leaves out the pieces folded
  into another element's body; a reader that wants an element's name or type
  object asks here instead of grouping again.

- **Where each decoded element was read from, down to the byte, on request
  (#1325).** `rvt-ifc --diagnostics out.json --element-provenance`
  (`ExportOptions::with_element_provenance(true)`, Python's
  `export_diagnostics(element_provenance=True)`) adds
  `decoded.element_provenance` to the diagnostics, keyed by ElementId: the
  carrier of the element's record (a partition element record frame that
  starts its partition record, one inside another's record as RE-30 and
  RE-35 describe, a `Global/Latest` schema-walk instance, an ArcWall record,
  an Area chain record, or a row with no frame of its own), its stream
  (`Partitions/46`, `Global/Latest`) and byte offset in that stream inflated
  (the bytes `rvt-dump` writes as `Partitions_NN.decomp`, #291), where its
  ElementId is stored (`record_offset`, with the enclosing partition
  record's id for a second-prologue frame), its class tag and the class that
  names in the file's own schema, its `BuiltInCategory`, and the native
  record the export's native passes indexed for it (its partition, offset
  and class, and, where several partitions hold a copy, which is current:
  RE-145, RE-216). An element whose offset cannot be given is kept without
  one and counted by carrier (`by_carrier`, `unkeyed_elements`, `native`);
  the table lists at most 250,000 elements and counts the rest. It is off by
  default, since it is large: on the BIM4LCA office as built the table is
  6,202,130 bytes of compact JSON against 38,640 bytes of diagnostics
  without it (about 161 times, with the value sources of #1360 and #1368), and
  the CLI's pretty-printed sidecar grows from 52,677 to 11,098,081 bytes and
  Core Interior's from 27,588 to 1,549,080 (Measure run 41890994176); RE1
  Architecture's grew from 13,956 to 85,470 before the value sources.
  Without the switch the diagnostics lack the field and are the bytes they
  were, and the table is never written into the IFC. It is built from the
  walk, the inflated partitions and the native record index the export
  already holds. An `offset` is the frame the exporter selected among the
  element's frames (Revit 2024 and later: the greatest vertical extent, then
  the greater stream and offset; 2008 to 2023: the newest, with no extent
  test). A body reader may take another copy (the sketch-line readers take
  the current one, RE-216), so it and the native record's stream can name
  different partitions; the copy a reader took is in `value_sources` (#1360). `rvt-rs-mcp` asks for it when it opens a model,
  keeps it beside the model, and returns the fields in every element's
  `provenance` (`carrier`, `stream`, `offset`, `record_offset`,
  `enclosing_record_id`, `class_tag`, `class_name`, `native_record`) and the
  file's `partition_count`, `layout_release` and `omitted_entries` (an
  element past the table's cap says `provenance_omitted`);
  `get_diagnostics` does not carry the table. `tests/element_provenance_offsets.rs` reads the bytes at
  every reported offset on Core Interior, the RE1 models and the BIM4LCA
  as-built office and checks the record decodes to the same ElementId, and
  prints the counts by carrier, the size of the table and the time to build
  it (CI's `element-provenance` artifact). Einhoven (Revit 2023) confirmed
  13 of 13 first-prologue frames and 71 of 71 native records; Revit 2008 to
  2022, and the second-prologue path of 2023, are not measured. The
  schema-walk carrier has no element on any of the seven models, so its
  offsets (into `Global/Latest` as the walk inflates it, where `rvt-dump`
  uses its own routine) have not been checked.

- **Why an element, body, opening or outline was left out, one entry per
  element, on request (#1326).** `rvt-ifc --diagnostics out.json
  --export-decisions` (`ExportOptions::with_export_decisions(true)`, Python's
  `export_diagnostics(export_decisions=True)` and `rvt_to_ifc_diagnostics`)
  adds `export_decisions` to the diagnostics. Each fail-closed decision is an
  entry with the element's ElementId, a stage (`element_skipped`,
  `body_not_drawn`, `body_left_as_box`, `opening_not_cut`, `outline_not_read`,
  `storey_not_bound`), a reason code, the class the diagnostics count it under,
  and the stream and byte offset its record was read from where known (a frame
  with no ElementId has its place and no id; an element the walk decoded takes
  its place from its provenance). The `skipped` section is now counted from the
  same decisions and is unchanged: every item of it, the geometry gaps among
  them, has a reason in the log whose `total` is its count, and the decisions
  behind `exported.bounding_box_bodies`, `bounding_box_spaces` and
  `area_body_refusals` are in the log too (`body_is_record_box`,
  `space_outline_is_record_box`, `area_body_refused`). The log keeps at most
  5,000 entries a reason, by ElementId, and counts what it drops per reason and
  in all. It is a switch of its own, not `--element-provenance`: the log is
  bounded where the provenance table is not, and needs no table in the
  diagnostics. Without the switch the diagnostics lack the field and are the
  bytes they were, and neither goes into the IFC. The stage for a value left
  out because Revit's exporters disagree exists and records nothing: those
  omissions are rules for a whole class of element in the reader that would
  write the value, not decisions made per element, and nothing counts them.
  `rvt-rs-mcp` asks for the log when it opens a model, keeps it beside the model
  and has a new tool, `explain_element(model, element)`: the element's
  provenance and trust markers where the export holds it, and every decision
  recorded for it, whether or not it is exported (`capped_reasons` names the
  reasons that dropped entries). `tests/export_decisions_counts.rs` checks, on
  Core Interior, the RE1 models, Einhoven and the BIM4LCA as-built office, that
  every reason of `skipped` has equal counts in the log and that the entries by
  class are its classes, re-reads the bytes at the frame of each entry of the
  seven element-record frame reasons on Revit 2024 and later, and prints the
  counts per reason (CI's `export-decisions` artifact). The offsets of Revit 2015
  to 2023 files and of `Global/Latest` entries are not verified, and a scan that
  finds an element in several partitions names its last frame, which can differ
  from the frame the provenance table names. `--anonymize` scrubs the log's
  phase names as it scrubs `skipped`. Core Interior has nothing left out, so its
  log is empty; the other models name reasons that must have entries.

- **The copy each body reader took its value from, in the provenance table
  (#1360).** With `rvt-ifc --element-provenance` (and the MCP server, which
  asks for the table), an element's entry in `decoded.element_provenance` now
  lists `value_sources`: for each reader that took a value of its body from a
  copy other than the frame the entry's `offset` names, the reader, the rule
  that chose the copy (`selected`, `agreeing`, `current` or `native`), how many
  partitions held a copy, the copy's stream and offset, and whether it is in
  another partition than the frame. A reader notes its choice where it makes it
  (the copy-aware scans of a wall's or beam's location line, of a sketch's
  curves and of a family instance's transform, and the native record readers
  of saved graphics, stair runs and landings), only while the table is being
  built, so the default export does nothing more and its IFC and diagnostics
  are byte for byte what they were. The table's `value_source_counts` counts,
  by reader, the elements whose source is in another partition than their
  frame (for an `agreeing` copy, in none of the partitions that hold one). The
  entry lists a source only where the element's final `BodySource` is drawn
  from that reader. A body that is the record's box but whose profile is read
  from sketch lines or the type's section (floors, columns) names the frame as
  `selected_frame_box`, for the box and the placement only, where no reader
  notes the profile's records (the entries below give floors, columns and
  rooms theirs). The MCP `get_element` and `explain_element` list
  the sources in the element's provenance; `list_elements` does not. This
  replaces the #1325 caveat that the offset may not be the copy a body came
  from. `tests/value_sources_offsets.rs` reads the bytes at each recorded
  offset on Core Interior, the RE1 models, Einhoven and the BIM4LCA as-built
  office, and names every `BodySource` on them as covered or not yet. The
  readers that note no copy yet are listed in `docs/export-diagnostics.md`;
  those elements have no value source, which is not a claim
  that they read the frame. Not measured: that the copy
  taken is the right one (`agreeing` copies are equal; `current` is
  `current_partitions`).

- **Value sources for family instances drawn from their families' own
  extrusions (#1368).** With `--element-provenance` and `--family-extrusions`
  both on, an instance whose body is rebuilt from its family's extrusions lists
  the records of the family's embedded document that it was rebuilt from
  (`family_forms`, `parts`: each visible extrusion, its sketch plane and the
  curves of its sketch, at the place the document's group puts them in their
  partition), beside the `instance_transform` that places them and the
  `selected_frame` they are drawn at. A record the file holds in several
  partitions is named at its copy in the highest partition, the copy the
  rebuild reads; an instance one of whose records has no place (its group's
  bytes are not the partition's own) gets no `family_forms` source, and keeps
  its other sources. The family document is read once, as it is embedded.
  Where the table is not asked for, no place and no record list is collected
  or kept (the document is read as before), and the IFC and the default
  diagnostics are unchanged.

- **Value sources for room outlines, column sections and pipe axes (#1368).**
  Under `--element-provenance` a room whose outline is read from its stored
  solid lists the header of each solid that gave it (`room_solid_faces`,
  `parts`, in the room's own partition), where it had only the frame for its
  box; a column drawn as its type's hollow or I section lists the type's
  parameter entries it took the section from (`type_hollow_section`,
  `type_i_section`: the width, height, wall and corner radius, or the I
  section's six values, each the first copy of an entry every copy of which
  holds the same value); and a pipe drawn from its connector entries lists
  the two entries that are its ends (`pipe_connectors`). The readers still
  without a source are listed in `docs/export-diagnostics.md`. These sources add 1.2% to
  the BIM4LCA office's table (6,131,584 to 6,202,130 bytes of compact JSON) and
  2.0% to Core Interior's (718,828 to 733,326; Measure run 41890994176).
  Nothing is noted unless the table is asked for, so the default export and its
  diagnostics are unchanged.

- **Value sources for profiles, stair runs and landings (#1368).** Under
  `--element-provenance` an element whose body is its record's box but whose
  profile was solved from sketch lines (floors, slabs, ceilings, and roofs
  whose body is the record's box) or taken from its type symbol's box
  (columns) now lists where the profile came from, as well as
  `selected_frame` for the box: `sketch_lines` (`parts`: the frame of each
  sketch line the solve was given, the first of its frames by frame order, or
  the current partition's where they disagree) and `sketch_list` (the
  Sketch's curve list, with the rule that chose its copy), or `type_section`
  (`parts`: the type symbol's frame; rule `agreeing`, or `frame` where the
  symbol's frames give different boxes and the last is taken). A stair run
  drawn from its sketch lists `stair_run_lines`, `stair_run_ends`,
  `stair_run_type` and `stair_run_base_cuts` (a Revit 2025 run, `native_run`
  and `native_run_type`),
  and a landing slab `landing_lines`, `native_landing` and
  `native_landing_type` (the landing type's record gives the thickness). A
  source made of several records lists them in `parts`, at most 64, with
  `parts_left_out` counting the rest. The sources of #1368 add 3.6% to the
  BIM4LCA office's table (5,987,841 to 6,202,130 bytes of compact JSON) and 66%
  to Core Interior's (440,909 to 733,326; Measure run 41890994176). The table's
  `value_source_counts` also counts,
  by reader, the elements whose copy was chosen as the current partition's,
  so a run on any model shows whether the rule is exercised. The readers
  still without a source, and the elements that keep `selected_frame_box`,
  are listed in `docs/export-diagnostics.md` (a room's outline has
  `room_solid_faces`, below). Nothing is noted unless the table is asked
  for, so the default export and its diagnostics are unchanged.

- **Value sources for roofs drawn from their slopes or from an extrusion
  (#1368).** Under `--element-provenance` a roof drawn from its slopes lists
  the slope block of each sketch line the slope was made from
  (`roof_slope_edges`: the defining edges, or the slope arrow and the boundary
  lines it rises from), those lines' own data (`roof_line_data`), its fascia
  depth and rafter cut where they were read (`roof_fascia_depth`,
  `roof_rafter_cut`) and the layers of its type that give its thickness
  (`roof_type_layers`), beside the outline and the box it already listed. A
  roof drawn from an extrusion lists the first frame of each of its sketch
  lines (`sketch_lines`), their data and its type's layers. Each owner map is
  built where the roof reader already resolves its sketch. CI's Corpus health
  now fetches puzzbobb's Revit 2025 models so that
  `tests/value_sources_offsets.rs` reads these sources and every other part
  against the models whose roofs are drawn this way. Nothing is noted unless
  the table is asked for, so the default export and its diagnostics are
  unchanged.

- **Value sources for Areas and for floors drawn along their shape-edited
  top (#1368).** Under `--element-provenance` an Area drawn from its area
  boundary lines lists the frame of each line that bounds its region
  (`area_boundary_lines`: the first placed copy by frame order, every placed
  copy of a line giving the same line) and the element data that gave its
  point, which picks the region among the lines (`area_point`: the Area's
  data object on Revit 2024, its element-data header on 2015 to 2023). A
  floor drawn along the plane of its shape-edited corners (#1384), which had
  no value source, lists its frame, the sketch lines of its outline and the
  current native record whose `SlabShapeEditGStep` gives that plane
  (`native_slab_shape`). Every `BodySource` now has a reader that notes its
  copy; an Area's Levels, which give its base and height, have no source.
  `tests/value_sources_offsets.rs` re-reads each line's frame, the native
  record's ElementId, and each Area's point, which must lie inside the
  outline the export drew. On the BIM4LCA office as built all 30 Areas and
  all 8 such floors have their sources, and the compact diagnostics with the table grow 0.9%
  (6,236,391 to 6,291,246 bytes with the diagnostics; CI's Corpus health on
  `main` at 77d71cbb, run 41944703690, against this change at 779ee16c). The
  Revit 2015 to 2023 path of `area_point` and the lines is not exercised by
  a model in CI. Nothing is noted unless the table is asked for, so the
  default export and its diagnostics are unchanged.

- **The diagnostics of a Revit 2008 to 2023 project count where its element
  records stop (#1389).** `decoded.element_record_stages` counts, per partition
  stream and in total, the markers found, those with a declared ElementId, those
  that gave a record, the placed instances, those with volume, the frames kept,
  the elements handed to the export and the elements the IFC model holds, each
  step exactly what the one before it passed on and left; the elements handed to
  the export are the elements written plus those set aside, each for one cause
  (an ElementId an earlier reader already held, a confidence below the floor, or,
  by Revit class, an element the exporter writes nothing for), and a stream that
  does not inflate is counted. The frames that stopped before they were records,
  and the placed instances with no volume, are also in `skipped` as
  `element_record_frame_stopped`, by stage (frames of every category, not lost
  elements), and, with `--export-decisions`, in the log of decisions with the
  place of their ElementId, 1,000 of each stage at most. When no record is
  written, `stopped_at` names the stage they stopped at, and `rvt-ifc` prints a
  line of its own with it. The rejected-frame reasons of 2024 and later counted
  nothing on these layouts, so a file such as the four on #421 that exports no
  element showed only `low_confidence_schema_scan_candidate`. The counts are
  integers added where the scan and the recovery already decide: no further pass
  over the file, and the default export lists no frame. Every export of a file of
  these releases has the report, even with every count zero, but a placeholder
  export's; `rvt-ifc` prints no line for a family file (`.rfa`, `.rft`), whose
  elements are read from `Global/Latest`. The IFC is unchanged. On Autodesk's 33
  sample projects of 2015 to 2023 (Measure run 41932213967), every count added
  up, per stream and in total, and the IFC was byte-identical to main's; 192,811
  of their 197,463 markers gave a record, 87,366 elements were handed to the
  export and the models hold 87,315 of them. The 51 not written are all
  `Mass` elements, which the exporter writes nothing for (none held by an
  earlier reader, none below the confidence floor; the release's Measure run
  41944962415).
  `tests/element_record_stages.rs` re-reads the markers on Einhoven and checks a
  file with no record and a family; Measure checks the sums on each sample
  project with `tools/re/element_stage_counts.py`.

- **`rvt-ifc --family-extrusions` draws a family instance from its family's
  own extrusions (#156, RE-327).** Off by default, and implies
  `--saved-meshes`: an instance whose body is the mesh Revit saved for it is
  written as `IfcExtrudedAreaSolid`s over `IfcIndexedPolyCurve` profiles,
  placed by the instance's transform, where its family's extrusions reproduce
  that mesh (every vertex within 0.0001 ft of their surfaces, samples of their
  surfaces within 0.0011 ft of the mesh, volumes within the arcs' chord loss
  and 0.01%), checked for each instance on its own; it keeps the mesh
  otherwise, as it does for a family with a void or with an extrusion that
  does not rebuild (an ellipse, an unclosed sketch) and for an instance with
  no transform that reads (other forms a family holds are not read, and the
  check against the instance's own mesh refuses what they make differ). An
  arc in a profile is written as the arc, whatever its radius (#1366), in the
  form Revit 24.0.5's IFC4 export gives it. Revit 26.2's IFC2X3 export writes
  a circle as an `IfcCircleProfileDef` and IfcOpenShell's default
  tessellation of the two forms has a chord more or fewer at some radii (27
  against 28 for a toilet-paper dispenser of 0.2292 ft, 0.001439 ft off); the
  scorers no longer count that as a miss, since a body both files write as
  true curves is judged in a fine tessellation, where the forms coincide
  (RE-327 section 11, RE-331). On Core Interior, RE1 and the BIM4LCA office
  files, measured at `34750454` (runs 41884078892 and 41884639625), 335 bodies
  are replaced; the body scorer scores 251 of them and all 251 hold the bar,
  187 judged in the fine tessellation and 64 in the default one, and 6 of 53
  exports move against `main`'s, none of them held-out. The extrusions are
  smaller than the meshes they replace: the BIM4LCA office's IFC is 77.9 MB
  instead of 82.2 MB, and its terrain files' 0.17 MB instead of 0.92 MB. The
  default export is unchanged (34 of 34 models, run 41884073888). Revit 2024
  to 2027 files, measured on 2024 and 2025: a file
  before 2024 has no instance transform that reads, so nothing is replaced.

### Changed

- **More Areas of Revit 2015 to 2024 files are drawn, from area boundary
  lines joined as their records say, and a line off the plan axes refuses
  only the Areas it touches (#1261, RE-330).** An area boundary line often
  stops short of the line it joins, and two parallel lines can stop at the
  same place a wall's thickness apart. Where two lines name each other, a
  free end is now carried on, by at most 1 ft, to the farthest perpendicular
  line that names it and is named by it, when that line spans the place
  reached; and the free ends of two such parallel lines, at most 1 ft apart
  and running away on opposite sides, are joined by a short connector. An
  Area whose outline needed either says `SpaceOutlineSource`
  `area_boundary_joined` instead of `area_boundary`. A line that is not a
  straight line along an axis, or whose record box is wider than the line,
  used to make every Area of its scheme on its Level unreadable; it is now
  left out with its box and refuses only the Areas whose region the box
  touches. On Autodesk's `rac_advanced`, 16 of the 25 Areas are drawn on
  every save from 2015 to 2024 but 2021's (3 before), 10 of them `area_boundary_joined`; the
  four once taken to be bounded partly by walls are bounded by lines alone.
  What this rests on: the only external check is an IFC2X3 export from the
  three.js repository, by a build of Revit 2021's exporter whose string ends
  "IU alternativa 21.0.0.383" (publisher not established), showing the
  model's 2015 to 2020 state. The 16 Areas drawn on the 2020 save are within
  0.1% in area and 0.01 ft in outline of it and none is written that it
  lacks, but that is in-sample: the joins were read off this same export, and
  that a reference list means a join is inferred from the fit. The 2022 and
  2023 saves are also checked against the 2024 save's reader. No Revit IFC4
  export of these Areas exists, and the export is not used to remove a value
  or to settle a disagreement between exporters. The BIM4LCA office's 30
  Areas are unchanged (the IFC is byte for byte as before); so is the
  held-out BIM Whale project's, which draws no Area body, so it does not
  exercise the joins. Left out: 9 Areas of `rac_advanced`, 1 to 5 (two
  straight axis lines whose record boxes are 0.497 ft wider than the line),
  7 and 8 (a straight line 0.016 degrees off the axis), 10 (an arc) and 6 (a
  join its records do not name). Report:
  `reports/element-framing/RE-330-area-joins.md`.

- **The viewer's browser tests are part of `CI passed`, so a pull request
  cannot merge before they have passed on its head (#1394, #1374).** They ran
  in Deploy viewer's pull-request run, which `CI passed` did not wait for:
  #1370 merged before that run had finished and Deploy viewer went red on
  main. They are now `viewer-test.yml`, called by `ci.yml` as its `viewer`
  job in the full set (a ready pull request, or a push to main), in three
  parallel groups of spec files, on a WASM built without LTO (55 s warm
  against 150 to 225 s). A change that touches only documentation skips
  them, and a skipped job passes. The saved-mesh measurement, which took 12
  minutes of every pull request's run, moves to Deploy viewer (pushes to
  main and dispatch); its one assertion that gates a merge, the toggle on
  Core Interior, stays in the suite. Deploy viewer has no `pull_request`
  trigger; on a push to main it runs the suite and the measurement again on
  the release WASM it publishes, and its deploy job waits for those and for
  `CI passed`. To check a draft's viewer, dispatch `viewer-test.yml` on its
  branch.

### Fixed

- **The MCP server's element names and type objects are the ones the IFC
  export writes (#1327).** The scorer of #1327 found two disagreements on
  every model it ran on. An `IfcSpace`'s `name` in `list_elements` and
  `get_element` was the model's own `Room-<ElementId>`, where the export
  writes the room number as `Name` (or `<room name>:<ElementId>` for a room
  with a name and no number) and the room name as `LongName`; and
  `element_type` was a parallel grouping that lacked the `Other` and
  `Identity Data` sets the writer gives every type object and the
  `IfcSpaceType` it makes for each room, and named a type for elements the
  writer makes none for. Both now come from the writer:
  `rvt::ifc::step_writer::written_names` and `type_objects` are the
  functions it writes the `Name`, `LongName`, type objects and their sets
  with, so the server and the IFC cannot differ on them, and the
  diagnostics' parameter value count of the type objects' sets reads
  `type_objects` too. A row and `get_element` gain `long_name`; `name` is
  absent only for a room with neither a number nor a name whose model name
  is exactly the exporter's `<Revit class>-<ElementId>` placeholder, and the
  placeholder names of other elements are still served as `name` (Einhoven
  23 of 37 rows, RE1 Electrical 3 of 49, the BIM4LCA office as built 52 of
  13,021). No row of the 32 models measured lacks a name, so the absent-name
  path is not measured. `explain_element`'s `name` and
  `other_entities[].name` follow the same rule (they were the model's name).
  `element_type` gains `ifc_type` and `name`, its `element_id` is the type
  object's `Tag` (an `IfcSpaceType`'s is its room's ElementId), and it is
  absent where the writer makes no type object, which it used to be served
  for. The tool output schemas change, and so does the contract in
  `docs/mcp-tools.json`.
  Measured with the scorer of #1404, on head 34ace109 with `base=none`: run
  41954686452 (scorer 30480145) passes 25 of 32 models, and run 41957587403
  (scorer 9b51c1a8, tip 3c9a263e, same code) the same 25. The 6 reference
  models (Einhoven, Core Interior, RE1) pass, where run 41949014871 (the
  1895e771 scorer, main's server) failed all 6, and 19 of the 24 Revit 2025
  models pass (1 before, run 41949021964); the 5 that fail are the stair
  models, on the level of a stair's parts and the levels' counts. The
  BIM4LCA office as built and at permit fails counts, element membership,
  element storeys and levels: as built 158 `IfcCovering` rows against the
  export's 111, 599 of 12,974 rows and 8 of 10 levels' counts; at permit 122
  against 75, 599 of 13,020 rows and 7 of 10 levels. Its names, long names,
  type objects and their sets are equal. Those failures are the levels and
  lists #1412 fixes: with both merged (a throwaway branch, 537d88ba), run
  41957685928 passes 32 of 32 models. The IFC text `rvt-ifc` writes is
  unchanged by the move: in run 41954681945 (`base=main` at 07caf059,
  `scorers=none`) the `ifc.sha256` of 34 of 34 models, the reference models,
  the four office files and the 24 Revit 2025 models, is identical. The
  held-out models and Autodesk's samples were not run.

- **A floor's void is tagged with the sketch line Revit's export tags it with
  where the lines run clockwise (#1165, RE-339).** RE-151 tagged each void
  with the lowest ElementId of the sketch lines on its loop, which is Revit's
  on 27 of the 36 voids of the BIM4LCA office as built and 27 of the 35 at
  building permit, and on all 62 floor and shading-device voids of Core
  Interior. On 8 and 7 of the others Revit's tag is the highest ElementId of
  the loop's lines: taken in ElementId order, which is the order they were
  drawn in, they run clockwise round the loop, and Revit starts such a loop at
  the line drawn last. That is what 8 voids of one exporter show; why is not
  read. The tag is now the highest where at least three straight lines, each
  on an edge of its own, step exactly one edge clockwise all the way round the
  loop in that order (the closing step included), and the lowest everywhere
  else, a loop with a curve included. On the office the voids with Revit's tag
  go from 27 to 35 of 36 as built and from 27 to 34 of 35 at permit, with the
  box, name, type and `Reference` equal on every one, and the voids written
  under another line's Tag go from 9 and 8 to 1 on each file, a loop of 6
  lines with a notch drawn later (#1408). The GlobalId of a void is rvt-rs's
  own, derived from the tag, so the 8 and 7 voids whose tag changes get new
  GlobalIds. The scorer measured is the unfilled-openings one, with the
  property and quantity census, on the two office files (Measure run
  41953415257, head e8ef33d3); Core Interior's and RE1 Architecture's exports
  and the 19 held-out and spent models' are byte for byte `main`'s.

- **A family's embedded document is read in partition-number order (#1368).**
  The family extrusions, the nested instances and the materials of a family
  read the records of the family's document partition by partition. They took
  the partitions in name order, which is not number order where the numbers
  have unequal digit counts (`Partitions/100` before `Partitions/99`), so the
  copy of an extrusion record held in several partitions that won, and the
  order the records were read in, followed the names. They take the partitions
  as `RevitFile::partition_stream_names` does, the newest copy last.

- **A floor whose corners are shape-edited is drawn along its sloped top, and
  the walls that stand on it start from that top (#1384, RE-336).** A floor's
  native `SlabShapeEditGStep` moves its corner vertices; where they lie in one
  plane the floor is the slab between that plane and the same surface one
  thickness lower (`BodySource` `native_slab_shape_edit`), where its record box
  reproduces the plane's highest point. A wall whose base join
  (`JoinToRoofGStep`, base) names one element that covers the whole of its plan
  is cut at that element's top over the plan, as a solid between that top and
  the wall's own (`BodyBaseSource` `wall_base_sloped_at_covering_floor`, or
  `wall_base_cut_at_covering_floor` for a level element), with the element in
  `BaseCoverElementId`. Where this is not proven the element keeps its record
  box or the wall its base, and the reason is a trust marker on the element
  (`SlabShapeRefusal`, `BaseCoverRefusal`): edits of interior or edge vertices
  or creases, corners not in one plane, an outline in pieces, a wall covered
  in part, by two elements or with the top crossing its base or top. An edited floor, roof or ceiling is never taken for a flat one, and a floor whose edit is
  found but not read leaves the walls on it as they were. The refusals are counted in
  `exported.slab_shape_refusals` and `exported.wall_base_refusals` and are
  decisions of the log (`slab_shape_refused`, `wall_base_cut_refused`). The top
  cut (RE-267) no longer takes a floor drawn along a sloped top for a level
  one. On the BIM4LCA office the 8 such floors, as built and at permit, have
  Revit's volume to 0.01%, and 12 of the 14 Basement walls Revit raises as
  built (the 4 on level floors and 8 of the 10 on sloped footings; 12 of 16 at
  permit) have Revit's lowest point (598 of 607 walls with
  Revit's base and top, from 586; 577 of 589 at permit, from 565); the other
  2 (4) are 0.123 ft (0.044 ft) off, from 0.656 ft, because their plans end
  short of Revit's. No wall's top moved, and no other model's export moves
  but for the refusal markers on Autodesk's Snowdon Towers (18) and `rac_advanced`
  (1 a save), which have no redistributable Revit IFC4 export to score them against.
  Such a floor keeps the `Qto_SlabBaseQuantities` `GrossArea` and `NetVolume`
  of its plan profile, Revit's on all 8, and a one-layer wall cut at a
  shape-edited or level floor keeps its layer's colour in the GLB (#1405). Details:
  `reports/element-framing/RE-336-sloped-footings.md`.

- **Partition streams are taken in the order Revit numbers them, not by
  name.** `RevitFile::partition_stream_names` sorted the `Partitions/NN`
  streams as text, which puts `Partitions/9` after `Partitions/46` and
  `Partitions/189` before it. So a reader that keeps the first or last copy
  of an element framed in several partitions could keep the wrong one
  when the partition numbers had different digit counts. Two cases had
  their own text comparison on top: the newest frame of an instance whose
  copies tie on vertical extent, and the first copy of a sketch line or a
  roof's sketch line (Revit 2024 and later).
  - `partition_stream_names` now sorts by partition number, and
    `partition_stream_name` returns the lowest-numbered stream.
  - The copy pickers compare partition numbers
    (`PartitionElementRecord::frame_order`).
  - Revit 2023 and earlier already compared numbers when picking a frame.
  - On every Measure model no export moves (see the pull request).

- **A room's outline takes off the round columns its stored solid holds
  (#1354, RE-334).** The walk that reads a room's outline from its stored
  solid (RE-101) read only planar wall faces, so a vertical cylinder, a
  round column, was ignored: the room kept the column's area, and a room
  with columns against its walls kept its record box. The walk now reads the
  arcs of vertical cylinders that reach the floor first, as chords within
  0.0005 ft, and where that does not close it reads the faces as before, the
  columns ignored; the resolved span (RE-316) reads them too, but a span with
  a cylinder closes undirected or not at all, since an arc's direction is not
  read. A room with more than 2,048 plan segments is left as before.
  - On `rac_advanced`'s saves of 2015 to 2020, against the public IFC2X3
    export of the project, all 91 rooms are within 0.1% in area and 0.01 ft
    in outline (Measure run 41909372124): before, 42 of the 85 rooms drawn
    from their stored solid were, and 6 kept their box. 48 spaces have as
    many voids as Revit's, on all six saves (read outside Measure from run
    41900428551's kept IFC, since deleted under the counts-only rule). A public Revit 2018 IFC4 export of `rac_basic` draws the
    one column of its room 101 as a void too, and the reader moves that room
    from 1.710% to 0.628% over its area (read outside Measure, run
    41909414507). The exports of the 33 other models measured (the BIM4LCA
    office, Core Interior, RE1, the R2025 models) are byte for byte as before
    (run 41909377642), and so are the 20 held-out and BIMcollab models
    (run 41909388707), so none of them has such a room.
  - Measure's `racadv` input scores the 2015 to 2020 saves against that
    export, fetched for the run and never stored or cached; only its summary
    counts are uploaded.
  - Revit's own area for `rac_basic` room 101 (its `GSA Space Areas`,
    781.08 ft²) is the geometry with the column taken off, as the schedule
    area now is; 4.95 ft² of the reader's 0.628% are in the room's outer
    ring, which this does not change and is not explained.
  - On Autodesk's samples (run 41909382808) the six rooms with columns
    against their walls take an outline on all 13 `rac_advanced` releases,
    2015 to 2027, and 2 of Snowdon Towers architectural's 7 boxed rooms do;
    nothing scores the 2021 to 2027 saves or Snowdon's two rooms, whose
    Revit IFC4 export holds no `IfcSpace`.
  - Not measured: Revit's own area for the `rac_advanced` rooms (its export
    carries none), a concave cylinder, a short column, and a prism room's
    `NetVolume` with columns (written for Revit 2024 and 2025 only).

- **A sketch's value-source notes cost nothing without
  `--element-provenance`, and the viewer's stop-in-the-silent-stretch test
  measures its cap (#1374).** The two notes `solve_sketch_plan_profiles`
  kept for the value sources, the set of elements whose outline came from
  their lines' own ends and the set of lines whose copies disagreed, were
  built on every open. Both are now built only while the handle records
  value sources. The viewer test that stops Core Interior's open between
  the last partition stream and the saved graphics no longer hard-codes a
  310 MiB cap: where the wasm memory grows between two reports moved with
  #1370 (16.3 MiB of it now falls inside the saved meshes, with the same
  325.4 MiB peak; the viewer measurement of Deploy viewer run 41870855196
  at f1f80d9e against run 41885589164 at 0ca59127), so the test opens the
  model once at the real ceiling and puts its cap between the memory at the
  first report that reads "Building the model" and the memory at the first
  report after those.

### Research

- **Measure scores what the MCP server answers (#1327).** A scripted client
  with no language model (`tools/re/mcp_client.py`) starts the built `rvt-
  rs-mcp`, opens each model Measure exports and asks a fixed query set: the
  summary, the levels, every element's row, the totals per IFC entity, Revit
  class and level, the rooms with their areas, `get_element` for the five
  lowest ElementIds of each entity, the diagnostics and an `export_ifc`.
  `tools/re/mcp_answers_vs_ifc.py` checks the answers against the same run's
  IFC export, where a difference ends its output with `exit 1`: the elements
  and spaces by GlobalId with their entity, Name, LongName, Tag, predefined
  type and storey; the storeys and the elements each holds; the rooms' Name,
  LongName, storey and plan area; for the sampled elements the property and
  quantity sets, trust markers, type object, translation, host and whether
  the body is none or mapped; the diagnostics against `diagnostics.json`;
  the server's export against `ifc.sha256`. The server and the writer share
  one `IfcModel`, so this finds what either does to it on the way out (a
  wrong projection, an element or set the writer drops or makes, a run that
  does not repeat) and no decode error; the answers are also scored against
  Revit's export with the pairing and tolerances of the class sets, storeys,
  space profiles, property sets and quantities scorers. It is run by name
  (`-f scorers=mcp_answers_vs_ifc`) or with the rest of the suite, builds
  the server only then (a server that does not build leaves it NOT MEASURED
  and fails no other scorer), keeps for the held-out models the `mcp
  answers:` lines, which hold only fixed words, integers and IFC entity
  names, and writes the answers of the openly licensed models as a
  deterministic dump (`mcp-dump.json`, sorted, floats at six decimals, no
  handle or path) for the semantic merge check (#1334).
  On main's server, Measure run 41950741547 (the pull request's, head
  9fbb594c, Core Interior, Einhoven and RE1) and runs 41949014871,
  41949018446, 41949021964 and 41949025326 (head 1895e771, the BIM4LCA
  office, the 24 Revit 2025 models, and the held-out, spent, BIMcollab and
  rac_advanced models as counts) found 57 of 58 models not consistent; only
  a blank Revit 2025 project passed. A space's `Name` was the model's
  `Room-<ElementId>` where the IFC has the room number, and its `LongName`
  was not served; `element_type` lacked the `Other` and `Identity Data` sets
  of the type objects and those made for rooms, and named a type where the
  writer makes none. Parts of a stair or railing had no level where the IFC
  puts them on their whole's storey (5 Revit 2025 models, 4 held-out). The
  office as built (13,021 rows, scored in 44 s) has 599 such rows and 8 of
  10 levels' counts differing, and lists 158 `IfcCovering` rows against the
  export's 111; at permit (13,067 rows, 42 s) 599 rows, 7 of 10 levels, and
  122 coverings against 75. The protocol, the rooms' areas, the sampled
  property and quantity sets, trust markers, the diagnostics and the export
  hash were equal on every model. The two server faults are fixed in this
  release by #1406 (under Fixed); on its head the scorer passes 6 of 6 reference models and 19 of 24 Revit
  2025 models (runs 41951524664, 41951554180). The scorer's later changes
  (this round's report guard, verdicts and sampling) are measured by the
  pull request's own run on its final head.

- **A wall that embeds a curtain wall is cut by it; the body is measured, not
  drawn (#1067, RE-335).** After RE-267 and RE-320 the BIM4LCA office's walls
  whose lowest point is not Revit's are 20 as built and 24 at building permit
  (Measure run 41906631624, `main` at 5026d863). Fourteen of the 20 as built
  stand on footings Revit draws sloped. The other six are raised exactly
  8.2021 ft, and each embeds a curtain wall (`WallEmbedWallGStep`) that is
  8.2021 ft tall from the wall's base and spans 0.90 to 1.0 of its length.
  - All 72 walls embedded in the 57 walls that embed one are curtain walls, so
    the step alone does not separate the six. What does in Revit's body is the
    end stubs: 2103576 as built has the same curtain wall as 2098938 and keeps
    its stubs (Revit's volume 32.08 ft³ against 23.42), where 2098938's are
    gone. No byte signal that tells them apart is read.
  - Nothing is drawn differently, and the diagnostics do not count these walls:
    a rule that raised the base wherever the curtain wall spans the wall would
    be wrong on 2103576 and 2177158.

- **The body scorers measure shapes beside IfcOpenShell's default tessellation
  (#1355, RE-331).** `mesh_bodies_vs_ifc`, `saved_meshes_vs_ifc` and
  `family_extrusion_bodies_vs_ifc` mesh both IFC files with IfcOpenShell's
  default settings, so a circle Revit 24.0.5 writes as two half arcs and one
  Revit 26.2 writes as an `IfcCircleProfileDef` come out as 28 and 27 chords at
  69.85 mm, 0.0014 ft apart, although both are one exact circle. With Measure's
  new `fine_mesh` input (`RVT_SCORER_FINE_MESH=1`) the scorers also mesh both
  files at one equal, fine deflection (1e-5 ft, 0.03 rad) and print labelled
  `fine` rows after their own: the same columns, how many bodies are meshed
  differently, which hold "ref on ours" in the default tessellation, the fine
  one, both or neither, and the bodies that miss in the fine one. Without it
  their output is as it was: the default rows of the two flag runs equal the
  earlier runs of `main`'s scorers, 99 of 99 files in each (41850764234 and
  41850769925 against 41843267888 and 41843276506). In the fine tessellation
  the two forms of a circle from 0.02 to 2 ft in radius coincide within 1e-5
  ft at every start, and RE1 Architecture's toilet-paper dispenser, redrawn the
  way rvt-rs writes it, is 0.000002 ft off Revit's body where the default
  tessellation has it 0.001439 ft off. Measured on the models: none of the
  1,368 doors and windows the default export draws from saved meshes is meshed
  differently, and none of the 66 family-extrusion bodies changes verdict (2
  of them, circles of 0.12 ft or less, are meshed differently and hold in
  both); of 3,816 saved-mesh bodies 295 are curved, 38 (the office's
  waste-terminal meshes, the largest 0.001001 ft against the bar's 0.001) hold
  "ref on ours" only in the default tessellation and 158 hold both directions
  within 0.001 ft only in the fine one. Drawn as arcs at `aaa38de8`
  (the first arc run's head, before the flat-arc rule), 185 of the 269 bodies
  the 0.12 ft rule of #1352 kept as meshes are scored, and all 185 hold in the
  fine tessellation (184 in the default one,
  the dispenser being the exception); the other 84 are office spaces, not
  scored by these scorers. No figure and no bar changes here. The maintainer decided the
  bar (2026-11-04, "Fine where both draw curves"): the fine tessellation
  decides only where both files write true curves, and saved meshes keep the
  default bar, so no saved-mesh verdict moves; #1365 implements it in the
  scorers and #1366 removes that 0.12 ft rule, so the 185 circles over 0.12 ft are
  drawn as true arcs again.
  Report:
  `reports/element-framing/RE-331-scorer-tessellation.md`.

- **The body scorers judge a curve in the fine tessellation (#1365, RE-331
  section 8).** On #1355 the maintainer chose "Fine where both draw curves":
  `mesh_bodies_vs_ifc`, `family_extrusion_bodies_vs_ifc` and
  `saved_meshes_vs_ifc` now take the verdict ("ref on ours", both directions
  within 0.001 ft, the boxes) of a body from a mesh at one equal, fine
  deflection when both files write it as true curves (an extrusion of a circle,
  of an outline with an arc and the like, and no mesh item, read from each
  file's own representation items by `tools/re/true_curves.py`), and from
  IfcOpenShell's default tessellation otherwise. A saved mesh is triangles on
  rvt-rs's side, so it is never in the set: no saved-mesh, door or window
  verdict moves. Only the bodies in the set are meshed finely. The rows say
  which tessellation decided: a line counts the bodies, the class tables gain a
  last column, the census rows end `decided by the fine tessellation`, and the
  `fine` rows of #1355 show those bodies in both tessellations. Without such a
  body the output is what it was. Report:
  `reports/element-framing/RE-331-scorer-tessellation.md` section 8.

- **A family's extrusions rebuild from their own sketch curves (#156, RE-327;
  Discussion #112).** An `ExtrusionElem`'s helper names its sketch; the
  `CurveElem` records that name it each hold one `GLine` or `GArc` in the
  family's coordinates; chained into loops, sorted into outlines and holes,
  placed in their `SketchPlane`'s frame (the columns of `m_3x3`, `m_flipZ`
  reversing the normal) and swept between the extrusion's two `Double`
  parameters (-1001800 and -1001801), they reproduce the mesh Revit saved
  for the form: the 24 extrusions of the family corpus (2 on each of 12
  releases, 2016 to 2027) within 0.001 ft, and all 6,172 extrusions of
  Autodesk's 58 sample projects (Revit 2015 to 2027) whose saved mesh decodes
  whole, by volume (1%), bounding box, vertices and surface (0.01 ft) (run
  41805344172). 6,093 match with the plane's normal as its matrix gives it;
  the other 79 share one signature (a down-facing plane, not flipped, whose
  extrusion reads `m_alwaysRefPlaneNorm` false) and match with the normal
  reversed, which the reader does by default for that signature only and
  refuses where the field is not read. The rule was found on these 79 and is
  not held out; the export checks every instance against its own mesh. One
  up-facing flipped plane that differed is refused, and the refusal of an
  unread `m_flipZ` cost none of the 6,172. `native_document` refuses every
  family file, so the new `native_form_geometry` reads a family file's own
  groups; `VarSketch` is refused on 2015 to 2017 (`VarParam::m_pSubst`,
  RE-323) and the rebuild does not read it. Not reproduced: STE1200's 85 of
  86, a Revit 2014 file outside the native releases. Report:
  `reports/element-framing/RE-327-family-extrusions.md`.

- **Clipping a trimmed loop to its grid cells takes the BIM4LCA office back
  under the schema step's size limit, and the readers stay off `main`
  (#1247, RE-302).** RE-302's readers (ellipse profiles, curved ruled
  surfaces, trimmed parametric faces) draw the office's 43 `WC-istuin:WC-1`
  toilets, which `main` keeps as boxes. With each earcut sliver cut at every
  grid knot they took the office as built from 568,965 to 2,097,501
  triangles (against the `main` of the time) and its IFC from 82.1 to 181.8 MB, over the 120 MB below which
  the schema step validates. With each loop clipped to the grid cells it
  covers and the pieces' corners shared they take it to 1,191,138 triangles
  and 104.6 MB (the permit file to 1,195,576 and 104.6 MB), validated again
  with the 4 rule findings `main` has; a toilet is 14,471 triangles against
  Revit's 1,668. They do not land: the toilets are 1.006 of Revit's volume
  but fail the 0.01 ft bar and "ref on ours" (0 of 43), and the default
  export, which they should not change, moves on one held-out model (14
  more of its windows drawn from saved meshes, all within the #1280 bar).
  Measured against `main` on 19 `--saved-meshes` pairs and 111 default
  export pairs (Measure 41849309128 and 41849313875); the readers stay on
  the draft #1361. Report: `reports/element-framing/RE-302-ellipse-profiles-and-trimmed-parametric-faces.md`
  (§6 and §7).

## [0.14.0] — 2026-11-04

0.14.0 adds `rvt-rs-mcp`, a local, read-only MCP server that answers an
agent's questions about a Revit file from the model the IFC export is built
from, and one export function, `rvt::ifc::run_export`, that `rvt-ifc`, the
Python package, the viewer and the MCP server all build that model with.
Exports follow the project phase as Revit's do (`rvt-ifc --phase`, the
Python package, the viewer and `rvt-schedule --phase` name another), write Revit 2024 Areas with a body and Revit 2015 to 2023 Areas
where their boundary proves one, write the openings that rectangular wall
openings cut, draw a wall a floor covers in part as stepped, end walls at a
corner in the order their record joins them, draw a level beam whose record
box is longer than its line as that box along the line, and export conduits,
conduit fittings and gutters. A large export holds less memory at its peak,
and every one of the 68 exports timed is faster than 0.13.0's. Much of it follows
reports from hemanth1986 (#1075, #1247) and inevitable-co-ai (#1261).
Thanks to both.

Measured against 0.13.0 on every reference model, Autodesk's sample projects,
the held-out models and the family files (Measure run 41803457521, `main` at
48930287 against v0.13.0):
- every one of the 68 files timed exports faster, each timed against 0.13.0
  on one runner: Autodesk's Revit 2015 to 2023 samples in 0.53 to 0.74 of
  0.13.0's time, their 2024 to 2027 samples in 0.74 to 0.92, and the
  reference models and the BIM4LCA offices in 0.75 to 0.91. None peaks
  higher in memory (0.54 to 0.99 of 0.13.0's peak; the office as built 1,690
  to 913 MiB);
- on the BIM4LCA office as built, 30 more unfilled openings are written,
  each within 0.01 ft of Revit's box with its GlobalId, name, sets and depth
  equal, so 58 of the unfilled openings in Revit's export are now written
  (28 before); at building permit 25 more are written, 24 of them in Revit's
  export (52, from 28) and one that Revit's export writes only as built, which the census counts
  apart. A door's or wall's cut in a further host, curtain walls' and
  cabinets' cuts are still not written. All 30 Areas of that one building
  have a body equal to Revit's in area and outline, as built and at permit,
  where none had one. 510 of 607 walls have both ends within 0.001 ft of
  Revit's (482) and 430 of 589 at permit (422). No value moves from equal to
  different. The values Revit's export does not write are unchanged as built
  and go from 14 to 16 at permit, both on that one opening;
- on the held-out Revit 2018 Vitoria project 1,514 elements and spaces pair
  with Revit's (1,336), among them 102 conduits, 74 conduit fittings and 2
  gutters, 1,359 on Revit's storey (1,181), and the values Revit does not
  write stay at 18 (counts only). Conduits, their fittings and gutters are
  drawn as their record box (Revit's bodies fill 0.37 and 0.31 of it for
  conduits and fittings). On eight held-out Revit 2024 to 2027 models 88
  windows go from their box to their saved mesh, every one exact against
  Revit's export (counts only). Six Revit 2025 models draw 18 windows from
  their mesh, each exact, and the two mitred walls of `18_WallJoins` now end
  where Revit's do (9 of 9 walls, from 7);
- Autodesk's `rac_basic_sample_project` saves from 2016 on leave out the 14
  rooms of its Working Drawings phase, the second of three;
  `rvt-ifc --phase "Working Drawings"` writes them.

The run compares `main`, whose export header still named 0.13.0: 46 of the
111 exports are byte for byte 0.13.0's and 65 moved, and the figures above
are the scorers' on those. A run of this release's own head, with #1339's
phase fix, against v0.13.0 (Measure run 41810374671) gives the same scores
line for line; there all 111 exports differ from 0.13.0's, since their
header names 0.14.0.

Not measured by this run, and said in each entry:
- the stepped wall tops, which no default scorer reads (the entry's figures
  are from its pull request's runs, #1304): 24 of 607 walls as built and 18
  of 589 at permit still differ from Revit's top, among them walls whose
  layers Revit ends at different heights, and the GLB's stepped tops are not
  scored;
- the Areas outside that one building: on Revit 2024 the top storey, Levels
  that are not building stories, a "Story Above" that is not the next Level,
  Areas bounded partly by walls, and `Qto_SpaceBaseQuantities.Height`, which
  is not written. No Revit 2015 to 2023 file with Areas has a Revit export:
  3 `rac_advanced` bodies on 8 saves are new, checked against the 2024
  save's, 22 of its 25 Areas stay unwritten on each of those saves, and the
  held-out BIM Whale project's Areas stay undelivered (#1261);
- the room phase rule against a Revit export made with the default setup;
- conduits on Autodesk's MEP samples and Snowdon Towers Electrical, which are
  counted, not scored;
- #1075's 438 MB project.

A level beam drawn along its line takes the box's length, not the beam's.

### Upgrading from 0.13

- Every export's IFC header names rvt-rs 0.14.0, so an export that is
  otherwise unchanged still differs from 0.13.0's there.
- A door's or window's `RvtElementRecordGeometry` set no longer carries the
  19 values the exporter used to hand from one stage to the next
  (`OpeningCentreX` to `OpeningHeight`, `InstanceOriginX`/`Y`, `TypeWidth`,
  `TypeHeight`, `CutWidth` to `CutSolidHeight`, `BoxWidth`, `BoxHeight`,
  `SymbolWireWidth`). Read a door's or window's size from `OverallWidth` and
  `OverallHeight`, and an opening's centre, axis and base from its
  `IfcOpeningElement`'s placement and extrusion. `rvt::ifc::IfcModel` gains a
  public field, `filler_sizes`, so a caller that builds an `IfcModel` as a
  struct literal has to set it.
- On Revit 2016 to 2027 a room whose own phase is not the export phase is
  left out, as Revit's export leaves it out, and counted in the diagnostics as
  `room_not_in_export_phase` (with the new `phase` field). The export phase is
  the project's last; `rvt-ifc --phase <name|id>` and
  `ExportOptions::with_phase` name another. Autodesk's
  `rac_basic_sample_project` saves from 2016 on no longer write their 14
  Working Drawings rooms unless that phase is named.
- New entities in a default export: an `IfcOpeningElement` for each
  rectangular wall opening that cuts a wall (with Revit's GlobalId),
  conduits and conduit fittings as `IfcCableCarrierSegment` and
  `IfcCableCarrierFitting`, gutters as `IfcPipeSegment`, Revit 2024 Areas
  with a body (`SpaceOutlineSource` `area_boundary`) and Revit 2015 to 2023
  Areas whose region and body are proven. An Area left without a body says
  why in `AreaBodyRefusal` or is counted as `area_without_proven_body`.
- New `BodySource` and body values: `partition_beam_box_along_axis` (a level
  beam drawn as its record box along its line, with no
  `Qto_BeamBaseQuantities.Length`), and a wall a floor covers in part carries
  `BodyHeightSource` `wall_top_stepped_at_covering_floor` and
  `TopCoverElementId`, with no `Height`, side areas or volumes; the room
  schedule leaves its Width, Depth and Height empty.
- `walker::read_field_by_type` and `write_field_by_type` read and write a
  `0x02` primitive as a `u8` and a `0x03` primitive as an `i16` (they were a
  `u16` and an `i32`), and a `0x02` vector or container as its bytes.
  `FieldType` gains `PointerArray` and `PointerContainer`, and `Flagged`
  takes byte 2 = 0x04.
- `rvt::ifc::run_export`, `run_export_with_limits` and
  `run_export_with_limits_and_control` build the model `rvt-ifc` writes, from
  `ExportOptions`; `RevitFile::export_phase` reports the phase
  `set_export_phase` resolved. A cancelled `WalkerControl` now ends an
  export with `Error::Cancelled` also while the saved graphics are read, and
  `rvt-ifc` prints its step notes after the decode (stderr only).
- `RevitFile::set_export_phase` may be called at any time: choosing another
  phase drops the handle's memoised walk, so one handle exported at two
  phases gives what two handles opened for them give.
  `RevitFile::clear_export_phase` goes back to the last phase and
  `RevitFile::project_phases` lists a project's phases (`ProjectPhase`). The
  Python export functions take `phase`, the wasm open takes `limits.phase` and
  returns `phases`, and `rvt-schedule` takes `--phase`.
- `rvt::research` (unstable): `native_element_parameters::read_defined_values`
  is replaced by `read_property_values`.
- A plane face of a saved mesh split into regions that tile it is drawn
  whole, so windows with such faces are drawn from their saved mesh in the
  default export (Revit 2023 to 2027) where they kept their box.
- A level beam drawn along its line (`partition_beam_box_along_axis`) gets no
  `Qto_BeamBaseQuantities.Length`, `Span`, `Slope` or `Roll`, and no I
  section.
- `FieldType::Primitive`'s `size` for the descriptors with base byte 2 and 3
  is 1 and 2 (it was 2 and 4), so `rvt-schema --json` output changes; the
  schema-table JSON schema gains the `PointerArray` and `PointerContainer`
  branches. The export diagnostics gain `exported.area_body_refusals` and a
  skipped item's optional `phase`.
- `rvt-ifc --phase`, `RevitFile::set_export_phase` and
  `ExportOptions::with_phase` fail for a phase the file does not have (the
  error lists the file's phases), for a family, and for Revit 2015 and
  earlier, whose phases the export does not apply.
- New public items: `ifc::FillerSize` (the type of `IfcModel::filler_sizes`),
  `ifc::step_writer::element_global_ids` and `ifc::schedule_csv::room_rows`.
- `rvt-rs-mcp` is a crate of its own and needs Rust 1.88 to build; `rvt`
  still needs 1.87.

### Added

- **The export phase in Python and the viewer, as `rvt-ifc --phase` has it
  (#1339, RE-326).** An export leaves out a room whose own phase is not the
  export phase, and the elements that do not exist in it; the export phase is
  the project's last unless told otherwise, and of the tools only `rvt-ifc`
  could tell it.
  Now `RevitFile.write_ifc`, `schedule_csv`, `export_diagnostics` and
  `export_diagnostics_json`, and `rvt_to_ifc` and `rvt_to_ifc_diagnostics`,
  take `phase: str | int | None = None`: a phase's name, matched exactly first,
  or its ElementId. A phase the file lacks raises `ValueError` listing the
  file's phases; one `RevitFile` can export different phases in turn, and a
  call without `phase` is the last phase again. In the browser,
  `limits.phase` of `openRvtBytesWithDiagnosticsModeAndOpenLimits` does the
  same, its result gains `phases` (`[{ id, name? }]`, in order, the last the
  default), and the File status panel lists the file's phases with the last
  as the default and reads the file again with the one chosen. Calls without
  a phase export the last phase as before. `rvt-schedule --phase <name|id>`
  schedules another phase, and Python's `schedule_csv(phase=...)` builds the
  model it builds (the document exporter at the phase set on the handle
  first). `RevitFile::project_phases` lists a project's phases and
  `RevitFile::clear_export_phase` goes back to the default; `set_export_phase`
  may now be called at any time on a handle, since choosing another phase drops
  the memoised production walk, which holds what the phase leaves out, so a
  handle exported at one phase and then another gives what a handle opened for
  that phase gives. The notes a walk makes for the diagnostics (family spaces
  left out for want of a body, Areas left unwritten, doors and windows written
  though their type was not read) now follow that walk, so exporting a handle
  twice at the same phase reports them both times. The stubs
  (`python/rvt/__init__.pyi`), [docs/python.md](docs/python.md) and
  [docs/viewer-api.md](docs/viewer-api.md) follow, and RE1 Architecture
  checks it in Python (`tests/python/test_export_phase.py`) and in the
  browser (`viewer/tests/export-phase.spec.ts`).

- **The openings a rectangular wall opening cuts in a wall are written.** A
  wall's native record names each element that cuts it, in the order Revit
  numbers the wall's openings, and Revit's export writes an opening with
  nothing to fill it for each rectangular wall opening among them. rvt-rs now
  writes it voiding the wall Revit's export voids, as the rectangle the
  opening's record saves, through the wall, tagged with the opening's
  ElementId and named for the wall, with the wall's type name as
  `Pset_OpeningElementCommon.Reference` and
  `Pset_EnvironmentalImpactIndicators.Reference`, the rectangle's width and
  height and the extrusion's depth as `Qto_OpeningElementBaseQuantities`, and
  Revit's own GlobalId: 30 of 30 on the BIM4LCA office as built and 24 of 24
  at building permit, each within 0.003 ft of Revit's box with every one of
  those values equal. The office at permit holds one more such opening than
  its Revit export (the records are the same in both files and Revit writes
  it only as built), which the census counts apart. Fire hose cabinets (39
  of 39 exact, one more than Revit's export in the office as built and two
  at permit) and curtain walls (box exact on 52 of 68) are measured and not
  written, and a door's further cut in a second wall, a door's cut in itself
  and a wall's own profile are not read yet (RE-321, #1165).

- **Areas of Revit 2015 to 2023 are written as `IfcSpace` where their
  boundary lines prove a body.** The 32-bit partition record (RE-315) of an
  Area is the 2024 one with `u32` ElementIds, and a 2015 to 2023 Area is read
  like a 2024 one: number, name, Level, area scheme, the two boundary lines
  it names, its point, and the region those lines enclose around the point,
  extruded to the Level above (RE-317). On Autodesk's `rac_advanced` of 2015
  to 2023 (25 Areas) and `rst_basic` of 2016 to 2023 (1) every Area has the
  number, name, Level, scheme and named lines of the same project's 2024 save;
  the point matches on 2021 to 2023 (24 of 25 on 2015 to 2020) and the line
  boxes on 2022 and 2023 (111 of 127 on 2015 to 2021, whose saves hold an
  earlier state of the sample model: 16 north-edge lines 0.0246 ft off, and the
  2021 save holds both states, which makes its groups of lines unreadable).
  Where the model is the 2024 one (the 2022 and 2023 `rac_advanced`, every
  `rst_basic`) the reader gives the same body or the same refusal, field for
  field. An Area is written only where its region and body are proven. Writing
  every placed Area, as 2024 does, wrote 2 `IfcSpace` that Revit's export
  lacks on the held-out Revit 2021 BIM Whale project (counts only; Revit's 8
  found; the cause is not measured). Under the narrowed rule none of its 10
  Areas has a proven body, so it writes 0 of Revit's 8 and 0 extras, as `main`
  does; that zero is vacuous, and **its Areas stay undelivered (#1261)**, a
  regression check under W5. Only the 3 `rac_advanced` bodies on 8 saves are
  new, checked against the 2024 save's reader output: no 2015 to 2023 file
  with Areas has a Revit export. An Area without a proven body is not written
  and is counted by reason in the export diagnostics (`skipped`,
  `area_without_proven_body`): 22 of `rac_advanced`'s 25 on each of 8 saves
  (18 in groups with a line that is not along an axis, 4 bounded partly by
  walls), all 25 on its 2021 save, and `rst_basic`'s one
  (`next_storey_not_proven`). Not measured: a design option on a 32-bit Area
  (an Area that names one is left out on 2015, which reads no option sets),
  `Pset_SpaceCommon.IsExternal` and the other named parameters of these
  spaces, and Revit 2008 to 2014 (not read)
  ([docs/rooms.md](docs/rooms.md), #1194, #1261, RE-324).

- **Conduits, conduit fittings and gutters are exported from their element
  records, on every release the exporter reads.** `OST_Conduit`,
  `OST_ConduitFitting` and `OST_Gutter` join the recovered categories under
  the unchanged RE-21 instance rule, with the record's box as the body and the
  Level its record names (a gutter's by its elevation). Against Revit's own
  IFC2X3 export of BIM-Speed's Vitoria `ALD26 Before` (Revit 2018), the one
  file with an export open to scoring that holds any of them (its held-out
  sibling pairs the same counts, counts only), all 102 conduits, 74 conduit
  fittings and 2 gutters the rule selects are elements of Revit's by `Tag` and
  none is outside it: 1,314 of its 1,321 elements are now written (1,136
  before), 1,305 of them in Revit's class or IFC4's for its IFC2X3 one
  (conduits as `IfcCableCarrierSegment`, their fittings as
  `IfcCableCarrierFitting`, gutters as `IfcPipeSegment`, the entities Revit's
  own type objects name, with no `PredefinedType`), and 1,251 on Revit's
  storey (1,073 before), with every validation finding at 0 and the
  properties Revit does not write unchanged at 20. The 7 still missing are 4
  panel walls and 3 assemblies. Cable trays and cable tray fittings are not
  recovered, since no file with a Revit export holds one, and neither are the
  conduit centerline categories Revit's export leaves out. Autodesk's MEP
  samples of 2015 to 2027 (19 or 20 conduits and 14 conduit fittings on
  `rme_advanced` and `rme_basic`) and Snowdon Towers Electrical (530 conduits
  and 322 fittings) now export them too, counted and not scored: no Revit
  export of those files exists. See
  [RE-319](reports/element-framing/RE-319-conduits-conduit-fittings-gutters.md)
  (#1261).

- **`rvt-rs-mcp`, a local, read-only MCP server over stdio, answers an
  agent's questions about a Revit file.** It is the new `rvt-mcp` workspace
  crate (its own `rust-version`, 1.88, rmcp's; `rvt` gains no async runtime and
  stays at 1.87) and targets MCP 2026-07-28, the stateless revision, as well
  as the older `initialize` handshake. `open_model` decodes a file once with
  the decode `rvt-ifc` runs and returns an opaque `model_id`; `get_summary`,
  `list_elements`, `get_element` (by ElementId or GlobalId), `list_rooms`,
  `list_levels` and `get_diagnostics` answer from that same model, so an
  answer and the IFC export cannot disagree, and each element carries a
  provenance object (ElementId, GlobalId, category, class, `BodySource`,
  `ProfileSource`, `SpaceOutlineSource`, `AreaBodyRefusal`, and the file's
  release, build, schema class count and ElemTable layout). `export_ifc` and
  `export_gltf` write to a path the caller names, refuse the model itself, and
  return a summary, the SHA-256 and a `resource_link`, with progress and
  cancellation. It opens no socket: its dependency tree has no HTTP crate (CI
  checks it) and its end-to-end test, which drives the built binary over real
  stdio in both lifecycles and compares its exports with `rvt-ifc`'s and
  `rvt-gltf`'s byte for byte, asserts the server holds no network socket. The release
  archives and the container image carry the binary. Two small public
  functions let it name what the export writes:
  `ifc::step_writer::element_global_ids` (the writer's own GlobalIds) and
  `ifc::schedule_csv::room_rows` (the rooms of `rooms_csv`), with no change to
  any export. See [docs/mcp.md](docs/mcp.md) (#1324).

- **`ifc::run_export`, one function that builds the model `rvt-ifc` writes,
  which `rvt-ifc`, `rvt-gltf`, the Python package, the viewer and
  `rvt-rs-mcp` all call (#1331).** The steps were written out five times: the
  exporter, the project phase, the bodies drawn from saved graphics, Revit's
  property sets and their placement, the base-quantity switch and the record of
  streams a resource limit kept unread. `run_export(&mut rf, &ExportOptions)`
  takes the switches `rvt-ifc` has, `--phase` included
  (`ExportOptions::with_phase`), and returns an `ExportRun`: the model, its
  diagnostics, the unread streams and the notes `rvt-ifc` prints (`ExportNote`,
  whose `Display` is the sentence, and whose `...Skipped` variants say why a
  step did not run). `run_export_with_limits` adds the walker limits and
  `run_export_with_limits_and_control` a `WalkerControl`, as the exporters'
  own `..._with_limits` and `..._with_control` entry points do.
  `RevitFile::export_phase` reports the ElementId that `set_export_phase`
  resolved from a phase's name or number (`None` while the last phase is in
  effect).
  The callers keep their defaults and their signatures. `rvt-rs-mcp` gains an
  optional `phase` argument on its tools that take a path: a model is opened
  at a phase, has its own `model_id` (keyed by the phase's ElementId, so a
  name and its number are one model), and exports what `rvt-ifc --phase`
  writes. Two behaviors move, neither expected to change an export:
  `rvt-ifc` prints its step notes after the decode instead of between steps
  (stderr only, the same text), and a cancelled `WalkerControl` now ends an
  export with `Error::Cancelled` also when it stops the saved-graphics step,
  where the viewer's bindings used to turn that into a diagnostics warning and
  carry on (the viewer cannot cancel a decode, it ends its worker, so it is
  not affected). `ExportKind`, `ExportOptions`, `ExportNote`, `ExportRun` and
  `SavedMeshChoice` are new public items.

- **A Revit 2024 Area's `IfcSpace` has a body: the region its area boundary
  lines enclose, from its Level up to the Level above it.** The Area stores no
  boundary; rvt-rs takes the lines of its area scheme on its Level, splits
  them where they meet or cross, leaves out the ones with a free end, and
  draws the loop that holds the Area's point, with the loops directly inside it
  as holes, as Revit's export draws it. On the BIM4LCA office all 30 Areas, as
  built and at building permit, equal Revit's in area (within 0.1%) and in
  outline (within 0.01 ft), where Measure scored none of them before. An Area
  keeps no body, as before, and says why in
  `RvtElementRecordGeometry.AreaBodyRefusal` and `exported.area_body_refusals`,
  wherever its boundary is not proven: a boundary line that is not straight
  along the plan axes, that was not read, or that is in a non-primary design
  option, a point in no single region, two Areas that share a region, or a
  Level that is not a building story. Not
  measured: the top storey (no Level above), Levels that are not building
  stories, a Level whose "Story Above" is not the next Level up (never read),
  Areas bounded partly by walls, and releases other than 2024.
  `SpaceOutlineSource` is `area_boundary` on a drawn Area and `none` on one that
  is not; the room schedule gives it an area. An Area does not hold a room's
  furniture. `Qto_SpaceBaseQuantities.Height`, which Revit's export writes on an
  Area, is not written yet ([docs/rooms.md](docs/rooms.md), #763, RE-317).

### Changed

- **A large export no longer holds several copies of the same bytes at its
  peak (#1075).** The peak of `rvt-ifc` and `rvt-gltf` on the reporter's
  438 MB project (10.3 GB at the export stage) was not measured here, since
  the file is not available; the figures below are from the BIM4LCA office
  as built (Revit 2024, 90,842 entities), whose stages a counting allocator
  sizes in exact heap bytes. Four copies are gone. Building the handle's
  cache of inflated partitions held each partition twice (every gzip
  member's output beside the joined copy) and the stored stream beside its
  stripped copy: members now inflate straight into the one buffer and the
  stored pages strip in place. Every native pass copied each partition it
  read and joined each continuation group into a buffer of its own: a pass
  strips in place, and a group of consecutive members of the cache is
  borrowed from it. The partition's string records were scanned in a copy of
  the whole partition, joined by a separator: each chunk is scanned where it
  lies, after the records that begin in the separator before it. The saved
  graphics of doors and windows decoded every symbol they share before
  meshing the first door: a batch of symbols (about 20,000 graph objects) is
  decoded now, the doors and windows that name it meshed, and the batch
  freed before the next. The heap at the highest point of each stage, in MiB
  (main in run 41778285915, this change in run 41781699897 at d1fc5d3c):
  inflating the 361 MiB partition 901 to 633 (resident 907 to 489), the
  partition strings 838 to 459, the export 848 to 770, the saved graphics
  1,568 to 868, so the highest stage of the whole export from 1,568 to 868.
  The peak resident memory of a whole `rvt-ifc` run, the median of three
  alternating runs of main and of this change on one runner (run
  41785668359 at 64db4426): the office as built 1,680 to 918 MiB, at
  building permit 1,634 to 979, Snowdon Towers architectural 1,351 to 1,073,
  Autodesk's 2024 MEP sample 623 to 519, and Core Interior 324 to 322. Of
  the 68 files measured none peaks higher, 57 run 3 % to 17 % faster, and
  two run slower: the office as built, 28.3 s to 29.2 s (3 %), and Snowdon
  Towers architectural (1 %); the office at permit runs 27.4 s to 26.2 s. All
  111 exports of that unfiltered run (the reference models, the BIM4LCA
  office files, the Revit 2025 models, Autodesk's sample projects and the
  held-out models) are byte-identical to main's, and the batched saved
  graphics are element by element the one-shot read's, with its progress.
  The probes `probe_1075_heap_stages` (exact heap bytes per stage),
  `probe_1075_heap_sites` (the call sites that hold the heap, by dhat) and
  `probe_1075_saved_batches` (the batched saved graphics against the
  one-shot read) reproduce them.

- **A room whose phase is not the export phase is left out, and `rvt-ifc
  --phase` names the export phase (#1194, RE-326).** A room's phase is the
  `m_phaseId` field of its `RoomElem` record on Revit 2015 to 2027, not its
  created phase, which is unset on every room. Revit's exporter leaves out a
  room whose own phase is not the export phase, the project's last unless the
  export setup or a view names another, and rvt-rs now does the same: the
  diagnostics count the rooms in `skipped` as `room_not_in_export_phase`, one
  item for each phase they sit in (`phase`, new in the diagnostics schema), and
  `rvt-ifc --phase <name|id>` chooses the phase, which the RE-184 rule for the
  other elements reads too. A room whose phase is unset, unread or not one of
  the project's is written, and so is an Area or an MEP space. The 621 rooms of
  the 12 models with a Revit export are all in the last phase and in Revit's
  export, so none changes; the rule is unmeasured against a Revit export made
  with the default setup, because no project in the corpus has a room outside
  its last phase and such an export. Autodesk's `rac_basic_sample_project` has
  14 rooms in Working Drawings, the second of three phases, which a public
  Revit 2018 export of it holds: its saves from Revit 2016 on no longer write
  them by default, and write them with `--phase "Working Drawings"`, all 14
  of the 17 `IfcSpace` of that export and no other entity changing (Measure
  runs 41784415396 and 41784419457). Of the 111 exports of the unfiltered
  Measure (run 41784044966), 12 move, those saves from Revit 2016 to 2027, and
  the other 99 are byte for byte as before, the held-out models included. Its Revit 2015 save
  keeps them, because both phase rules start at Revit 2016 (#1332). The Python
  bindings, the viewer and `rvt-schedule` name another phase too (#1339). Report:
  `reports/element-framing/RE-326-room-phases.md`.

- **`Formats/Latest`: 11 of the 14 residual field names are classified, and
  Revit 2027's flagged fields are shown to hold no instance bytes (#1196,
  RE-323).** `FieldType` gains `PointerArray` (`0e 13`: `GEdge`'s three
  face and edge arrays, two external pointers each) and `PointerContainer`
  (`0e 53` and `0e 54`: a counted container of external or owning pointers),
  appended to the non-exhaustive enum, and `Flagged` takes byte 2 = 0x04
  (`VarExpr::m_refCt`, an `i32`). The native graph decoder reads the
  instances of six of these fields in Autodesk's sample projects with no
  layout failure, no record left with a byte unconsumed and every non-null
  pointer resolving to an object of its record: `GEdge`'s three arrays
  (111,985 instances each, 57 of 58 projects, all 13 releases 2015 to 2027),
  `VarExpr::m_refCt` (138,477), `VarSketchConstrObj::m_constrElems` (58,876)
  and `VarSketchObj::m_params` (93,092) on 2018 to 2027. Five more fields
  (`LFormatDefaults`' three, `VarFunction::m_args`,
  `VarSketchDimObj::m_refPts`) have no instance in any file read and are
  classified by their header alone. `ZoningAppInfo::first` was `Unknown`
  only because the schema scan cut a field's type at the end of its class's
  4,096-byte window, two bytes after the field name; the scan now reads it
  from the stream. 2027's 339 flagged fields (`DBView::m_horzDirTemp`,
  edit-mode managers) occupy no bytes: the native decoder skips them, and
  97,000 of the 97,005 records of Autodesk's five 2027 sample projects still
  end exactly at their body's last byte (the other 5 fail on a size
  budget), with 110 of the fields' classes among them. Still unclassified:
  `ClassDefinitionRef::m_ref` (no instance in any file read),
  `VarParam::m_pSubst` (31,705 instances in the 2015 to 2017 sample
  projects, every one a null pointer, which two layouts read alike) and
  `VarSketchDimObj::m_dimValue` (none). The coverage gate also reads each
  schema with the exact grammar, which names these fields by the classes
  that declare them (the scan files `GEdge`'s under `GInfo` and
  `VarSketchDimObj`'s under `VarSketchObj`), and fails on a residual entry
  that no longer decodes `Unknown`. The `schema-table` JSON schema gains two
  `FieldType` branches. Report:
  `reports/element-framing/RE-323-residual-field-encodings.md`.

- **Exports of Revit 2015 to 2023 files take 0.54 to 0.78 of the time they
  took on `main` before this change, and 1.1 to 1.7 times as long as with
  0.11.0 (#1237).** 0.12.0 took
  2.0 to 3.4 times as long as 0.11.0. Four changes, none of which changes an
  export (every Measure model, run 41793618569, 111 exports):
  - the read of the parameters named for a property decodes a record only if
    its body holds the id of a definition the read uses, and reads the types
    only if one of those definitions is a plain `IsExternal`, whose type
    value an element takes. This assumes that a value is saved against its
    definition's id itself; on Autodesk's 33 sample projects of 2015 to 2023
    every record that gave such a parameter held an id. On the 2015
    `rac_advanced`, 6 of the 5,608 records it decoded held one, and no sample
    defines `IsExternal`;
  - the phase read, which takes the created and demolished phase of every
    record in the file, reads a record's Element base fields straight from
    its bytes where the schema has the layout measured on Revit 2015 to 2027,
    and by the graph decoder otherwise. It agreed with the graph decoder on
    all 1,014,473 records of Autodesk's 58 sample projects, 2015 to 2027;
    none fell back (Measure run 41782852434, on a branch whose reader is
    byte-identical to this change's);
  - the graph decoder works out a class's field keys once for each class a
    record uses, copies no field for the items of a container, and keeps no
    field spans it never reads, and a class is found by name through an
    index. The 25 samples of 2024 to 2027 take 0.80 to 0.89 of the time, and
    the reference models and offices 0.75 to 0.87 (run 41793618569);
  - the parameter definitions pass asks once for each class whether it owns a
    definition, not for each record.

  Each comparison was timed on one runner of its own, the median of three
  exports each, so compare ratios, not seconds, between the two: this change
  at its merged head against `main` (Measure run 41793618569) and, at
  6f387fd1, against 0.11.0 (run 41782874568), on Autodesk's sample projects.
  Every `ifc.sha256` and every diagnostics file of the 111 exports of run
  41793618569 is unchanged. The time of this change over `main`'s is 0.62
  for the 6 projects of 2015, 0.66 for the 9 of 2016 to 2018, 0.65 for the 15
  of 2019 to 2022, 0.69 for the 3 of 2023 and 0.86 for the 25 of 2024 to
  2027; over 0.11.0's, 1.49, 1.51, 1.47 and 1.13 for the first four, single
  projects 1.09 to 1.65. Peak memory over `main`'s, per file, is 0.84 to 1.00
  on the 58 samples and 0.93 to 1.04 on the reference models and offices
  (Core Interior 1.04, 309 to 322 MiB). What remains over 0.11.0 is the
  parameter definitions pass, which decodes every `FamilySymbol` record for
  two root fields, the wall records' top joins and the native records'
  preamble (#1335).
- **`rvt::research` (unstable): the named-parameter read changes its entry
  point (#1237).** `native_element_parameters::read_defined_values` is
  replaced by `read_property_values`, which also takes the captions whose
  type values the reader uses. `native_document::Options` gains
  `filtered_parameters_only` (off by default), and `native_parameters` gains
  `Spans` and `decode_object_fields_spanning`.

- **Hand-off values no longer reach the IFC.** Nineteen properties of
  `RvtElementRecordGeometry` were values one stage of the export wrote so a
  later stage could read them back, not facts about the element: the opening
  a window's or door's type and transform give (`OpeningCentreX`,
  `OpeningCentreY`, `OpeningAxisX`, `OpeningAxisY`, `OpeningBaseElevation`,
  `OpeningWidth`, `OpeningHeight`), a turned instance's origin
  (`InstanceOriginX`, `InstanceOriginY`), and the sizes its type and symbol
  give (`TypeWidth`, `TypeHeight`, `CutWidth`, `CutHeight`, `CutSolidWidth`,
  `CutSolidDepth`, `CutSolidHeight`, `BoxWidth`, `BoxHeight`,
  `SymbolWireWidth`). They stay inside the exporter, which works the openings
  and each door's and window's `OverallWidth` and `OverallHeight` out of
  them as before. The trust markers (`BodySource`, `ProfileResolved`,
  `SpaceOutlineSource` and the rest), the identity properties and the
  measured facts stay. Upgrading: nothing user-facing changes value,
  `OverallWidth`, `OverallHeight` and every opening are what they were, and
  only those 19 properties and their `IFCPROPERTYSINGLEVALUE` lines are gone
  from a door's or window's set. An importer that read a door's or window's
  size from one of them reads `OverallWidth` and `OverallHeight` instead; an
  opening's centre, axis and base elevation are in its `IfcOpeningElement`'s
  placement and extrusion. The `*_PROPERTIES` constants of
  `rvt::ifc::export_content` stay, as the names of the hand-offs.
  `rvt::ifc::IfcModel` gains a public field, `filler_sizes` (each door's and
  window's size, serialised when it is not empty), which breaks a caller that
  builds an `IfcModel` by hand.

- **A level beam whose record box is longer than its location line is drawn
  as that box along the line (#1247, RE-328).** `beam_body` declines a
  line whose section does not close; on Autodesk's 2024 samples that is
  every beam it declines with one line, bar 2 Snowdon beams whose line
  leaves the box by up to 100 ft. 34 of the 52 are level, have no
  Cross-Section Rotation and have a box longer than their line by at most a
  foot (17 Snowdon, 9 rst_basic, 8 rac_advanced; RE-49 took 15 of Snowdon's
  for beams with oblique end cuts, and their saved mesh says otherwise).
  They are now the box's extents along the line, `BodySource`
  `partition_beam_box_along_axis`, instead of the axis-aligned record box.
  Read it as an envelope: its direction and section come from the line and
  the box (the section across the line is the beam's own saved mesh's on all
  25 whose mesh decodes whole; the mesh is Revit's tessellation, not an IFC
  export), and **its length is the box's, not the beam's**: the mesh's ends
  are the body's on 16 of the 25 and end inside it on 9, by more than
  0.01 ft and at most 1 ft, so it can run past the beam at its ends (the
  record box does on 21). The line must be level and within 25 degrees of a
  model axis, the only angles measured.
  The 9 without a decodable mesh are on a model axis, where the body is their
  record box. It gets no `Qto_BeamBaseQuantities.Length`, `Span`, `Slope` or
  `Roll`, and no I section; `AxisLength` is the line's length and
  `BodyLength` the body's. The beams already drawn along their line, and
  the BIM4LCA office, are unchanged. 16 sloped, rolled beams and the 2
  Snowdon beams keep their box. New: `beam_check` and `BeamReject` in
  `rvt::research::partition_beam_axes` name the test that declines a line
  (`beam_body` is unchanged), and `probe_re328_beam_rejections` sorts a
  file's beams by it, counts only. Report:
  `reports/element-framing/RE-328-beam-rejections.md`.

### Fixed

- **A wall a floor covers in part is drawn stepped (RE-320, #1067).** Revit
  cuts a wall's top at the underside of the floor, roof or footing its top
  join or a Join Geometry cut pair names, over the part of the wall's plan
  that element covers, and leaves the wall's own top over the rest; rvt-rs
  drew that only where one element covers the whole plan (RE-267). A wall
  such elements cover in part is now one `SweptSolid` body: its own plan up
  to the lowest underside and, over the part of the plan they do not cover,
  convex pieces up to the next underside or the wall's top. On the BIM4LCA
  office that is 49 walls as built and 46 at building permit. Against
  Revit's own export, rvt-rs's top equals Revit's over the whole plan on 583
  of 607 walls as built (544 before) and 571 of 589 at permit (530), and the
  area drawn at a wall's full height where Revit's body is cut falls from
  1,915.8 to 238.2 ft2 as built and from 1,888.1 to 38.1 ft2 at permit; no
  wall that was equal moved, the 64 and 70 walls cut whole, every wall's
  lowest and highest point and the quantity, property set, layer and opening
  scores are unchanged, and Core Interior, RE1 Architecture and the R2025
  models export byte for byte as before. A stepped wall carries
  `BodyHeightSource` `wall_top_stepped_at_covering_floor` and
  `TopCoverElementId`, and gets no `Height`, side areas or volumes, as
  before. The glTF and the plan sheet still draw it in its layers and by its
  stepped plan.
- **An element that only a cut pair names cuts a wall's top only where it
  reaches it (RE-320, #1067).** A Join Geometry cut takes the volume the two
  elements share, which is the wall's top only where the element rises to it.
  Three office walls as built (3484876, 3490093, 3490551) pass through a slab
  whose top is 0.984 ft below theirs, and Revit leaves their tops whole;
  the rule that cut a wall at any element it names would have cut them. The
  64 and 70 walls RE-267 cuts whole are unchanged, and a cut-pair element
  that reaches the wall's top (20 walls as built, 10 at permit) still cuts.
- **Not drawn, and measured.** A wall whose layers Revit ends at different
  heights (the KS walls 1347198 to 1347201, taller in rvt-rs by 102.9, 14.3,
  35.7 and 11.7 ft2 as built), a top Revit cuts at a height no join names,
  and the gaps and ends Revit's walls have where no floor reaches. One wall as
  built (3004978) keeps its full height: 0.0243 ft2 of its plan, all of it
  inside the wall it abuts, is cut by Revit, and no width, area or position
  of that piece separates it from the pieces Revit keeps, 0.032 ft slivers
  included (they are drawn). The room schedule leaves a stepped wall's
  Width, Depth and Height columns empty, since its body is no longer one
  extrusion, and the GLB's stepped tops are not scored.

- **A plane face split into regions that tile it is drawn whole (RE-329,
  #1144, #1247).** Revit saves a face that was split, for paint or material,
  as the face and a region face for each part, and rvt-rs refused any face
  with regions, which kept the whole element's record box. A plane face
  whose regions are plane faces on its own plane, each with one loop and
  none of its own, listed once, is now drawn as the one face it is where the
  regions tile it: each lies inside its outline, shares no area with
  another, and together they cover exactly its area, measured on
  triangulations. Any other face with regions is refused as before. Doors
  and windows are drawn from their saved meshes by default, so this moves
  the default export of the Revit 2025 models 12_Rooms_Door_Windows to
  17_Rooms_Stairs_Spiral, three windows each from their record box to their
  mesh, 18 in all. Against Revit's own export every one is exact (Measure
  run 41790592582: volume ratio 1.0000, Revit's vertices within 1.8e-6 ft
  of the mesh and the mesh's of Revit's), the census no longer lists
  regions as a cause in any file, and no other line of those six files'
  scores moves. No door or window of the BIM4LCA office, Core Interior or
  RE1 is kept as a box for regions (census, run 41786625424), so none
  changes there. The whole Measure (run 41791824472) moves 15 of 111
  exports and no other: the six models above; 42 windows of Autodesk's
  Snowdon Towers architectural sample, from box to mesh (172 of 172 door
  and window bodies drawn, 130 before; no Revit export, so not scored); and
  eight held-out models, 88 windows in all, every one exact against Revit's
  export (counts only, run 41793911979). Not measured: a face with regions
  on any other surface, or with holes, which stay refused; whether regions
  explain #1247's boxed doors and windows is for its reporter's census to
  say.

- **A rectangular wall opening in a wall drawn stepped is written.** RE-321's
  openings (#1305) took only walls drawn as one extrusion, so in a wall
  RE-320 draws stepped (#1304) the opening was left out without a word. A
  stepped wall keeps its whole prism, and the opening voids every piece of
  its body, so it is written there too. An opening whose corners do not lie
  in one plane across the wall, as on a wall whose axis was not read, is
  left out (#1165, RE-321).

- **A wall's end at a corner of three or more walls follows the order its
  record joins them in (RE-322, #1194).** A join's walls are listed in the
  order Revit joined them, and an earlier wall wins: counting only the walls
  that overlap in elevation (the list also names the walls on other storeys
  that end at the same point), a wall stops at the near face of the
  perpendicular walls listed before it, meets flush a wall that continues it
  when that is listed before the next perpendicular wall, and otherwise runs
  through the perpendicular walls listed after it. The faces are those of each
  wall where its own line ends, since walls of different thickness are
  aligned on a face, not a centreline. It is read only where it is right on
  every measured end: a butt join, every wall straight and meeting at a right
  angle or straight on, no element but a wall cutting the wall, the faces that
  decide agreeing, and the wall single-layer or stopping against single-layer
  walls. An end it reads names the wall it stops at or runs through
  (`JoinStartWallElementId`) only where one perpendicular wall alone decides
  it. Against Revit's own export of the BIM4LCA office, 510 of 607 walls
  as built have both ends within 0.001 ft (482 before) and 430 of 589 at
  building permit (422); 30 and 8 ends go from wrong to right and none from
  right to wrong. Two single-layer walls that meet at a mitre (R2025
  `18_WallJoins`, the only one measured) end on the line through the two
  corners: 9 of its 9 walls, from 7. Core Interior and RE1 export byte for
  byte as before. Layered corners of two walls, a layered wall at layered
  walls and every corner a column cuts were measured and are not read
  (`reports/element-framing/RE-322-wall-corner-order.md`).

- **`Formats/Latest`: a `char` field is 1 byte and a `short` field 2, not 2
  and 4 (#1196, RE-325).** `FieldType::decode` gave the descriptors with base
  byte 2 a size of 2 (a `u16`) and base byte 3 a size of 4 (a "deprecated
  i32 alias seen in 2016 to 2018 only"); the native graph decoder had read
  them as 1 and 2 bytes. The schema names the types (every `std::pair<
  char, K >` slot carries base 2 and every `std::pair< short, K >` slot base
  3, in all 13 family files, 2014 and 2016 to 2027), and the instances settle
  the width: the 58 Autodesk sample projects of Revit 2015 to 2027 hold
  402,652 native records with an instance of such a field, 402,293 of them
  decodable without the extensible-storage catalog (the other 359 are
  Revit 2024 records that need it), and all 402,293 are read to their last
  byte at 1 and 2 bytes (683,250 instances of 20 fields). `FieldType`'s
  widths read 65 of them, those whose only instances are empty byte buffers,
  and each field rewritten alone to another width leaves none of its
  non-empty records whole. The byte buffers are files that frame their own
  length: 13,407 PNGs end with their IEND chunk at exactly the declared
  count, and 8,157 RTF streams open with `{\rt`. Base 3 is not confined to
  2016 to 2018: the schema has 24 to 27 `short` fields and three `short[3]`
  arrays in every release 2014 to 2027. `walker::read_field_by_type` and
  `write_field_by_type` read and write a `0x02` primitive as a `u8` and a
  `0x03` primitive as an `i16`, and a `0x02` vector or container (a byte
  blob, which the walker could only skip the count of) as its whole bytes.
  Report: `reports/element-framing/RE-325-scalar-widths.md`.

### Research

- **What keeps a saved mesh from decoding whole, face by face (RE-329,
  #1144, #1247).** `probe_re329_saved_graphics_census` meshes every face of
  an element that keeps its box, not only the first that fails, and prints
  counts: each failing face's surface, profile curve and trim, and each
  element's set of causes, split into doors and windows (drawn by default)
  and everything else (`--saved-meshes`). On the BIM4LCA office as built 146
  of 11,246 candidates keep their box (147 of 11,363 at permit): 43 toilets
  that need ellipse profiles, curved ruled surfaces and trimmed
  revolutions together (RE-302), 30 fittings held by cylinder faces with
  several loops alone (RE-305), 19 sanitary terminals that need
  `HermiteSurf` and `GHermiteSpline` together, and 25 railings and terminals
  held by the graphics walk alone, not by a face; no door or window of the
  office, Core Interior or RE1 is among them (Measure run 41786625424).
  RE-305's 30 fittings, read on today's `main` with the volume taken about
  each body's centre, are still 1.1102 to 1.1105 of Revit's (run 41787545512)
  and, face by face, draw no surface Revit lacks (run 41788644583: 2.3% more
  area, no face off Revit's surface). They fail "Revit's vertices within
  0.001 ft" on 72 of Revit's 1,083 vertices, all nearest surfaces of
  revolution, up to 0.0051 ft: 26 in the pattern of RE-302's open item, a
  junction between two curved faces, and 46 at 0.0011 to 0.0023 ft on three
  other faces, for which no cause is shown. The reader stays off `main`. Not
  claimed: that the excess volume is wholly Revit's faceting of thin rods,
  or that any of the 72 vertices is Revit's error and not the mesh's.

## [0.13.0] — 2026-11-03

0.13.0 makes room and space import dependable on more files, reads Revit
2018 projects that exported nothing before, and shows the browser viewer's
real progress. Most of it follows reports from inevitable-co-ai (#1260,
#1261, #1262), hemanth1986 (#1075, #1247, #421) and puzzbobb (#421), and it
takes puzzbobb's speed-up of the native record reads (#1301). Thanks to all
three.

Measured against 0.12.2 on every reference model, Autodesk's sample projects
and the held-out models (Measure run 41741896192, this release's code
against v0.12.2):
- every `IfcSpace` says where its outline came from
  (`RvtElementRecordGeometry.SpaceOutlineSource`). On the BIM4LCA office 229
  of the 232 rooms paired with Revit's as built and 231 of 234 at building
  permit have Revit's area to 0.1% (220 and 221 with 0.12.2). Of the 12 rooms
  per file 0.12.2 drew as their box, 9 and 10 now have Revit's area; the rest
  are labelled approximate (a column Revit's export may not take off, 0.45%
  to 0.68% under Revit's floor area) or keep their box, and say so;
- rooms on Revit 2026 and 2027 take their outline instead of their box: each
  of the 257 rooms of Autodesk's four sample projects with rooms gives the
  same verdict and area in its 2026 and 2027 saves as in 2025, the rooms
  that keep their box in 2025 included; where an outline sits is not
  measured on those files;
- the BIM4LCA terrain office writes Revit's 9 `IfcSpace`, not 98: the 89
  bicycle bays Revit's export leaves out are left out;
- Revit 2018's `ALD26 Before` writes 1,136 of the 1,321 elements and all 73
  spaces of Revit's export, where 0.12.2 wrote none; its held-out sibling
  writes 1,336 elements and spaces against Revit's 1,520, none that Revit's
  export lacks (counts only). Revit 2016 to 2023 exports now leave out
  elements outside the export phase and in non-primary design options, as
  2024 and later did; on that one 2018 project the two filters together
  remove 198 elements Revit's export lacks, and the counts cannot say which
  removed which;
- on the BIM4LCA office 186 more columns as built and 250 more at building
  permit carry Revit's `Length` (279 of 279 and 321 of 327; the 6 at permit
  without it are not explained), and the two wall hatches have Revit's width
  (268 of the 268 doors Revit sizes). Both are one building's measurement;
- the T-shaped roof of puzzbobb's Revit 2025 `20_T_U_Roof` is Revit's body;
  the U-shaped roof stays its box, and roofs with several slope arrows are
  drawn on Revit 2025 only.

Faster, measured on one runner in one run against `main` before #1301 (which
already had #1237's 4% to 5%): #1301 makes Revit 2015 to 2023 exports of
Autodesk's samples 5% to 19% faster, their 2024 and 2025 samples 5% to 13%,
and the BIM4LCA office 7% to 9%; RE1 Architecture took 6% longer and RE1
Electrical 2% less.

What is not measured yet is said in each entry: among others, whether the
202 MB model behind #1075 now reads whole at the default limits, whether the
ElementIds of the #421 reporter's files are Revit's, the #1247 large-model
door fix on the user's own project, and the door width rule on any building
but the BIM4LCA office.

### Upgrading from 0.12

- Every export's IFC header names rvt-rs 0.13.0, so an export that is
  otherwise unchanged still differs from 0.12.2's there.
- Every `IfcSpace` in a geometry export carries
  `RvtElementRecordGeometry.SpaceOutlineSource` (`stored_solid`,
  `stored_solid_faces`, `stored_solid_approximate`, `record_box`,
  `saved_mesh`, `family_body` or `none`). An importer can use it to tell a
  room drawn as its box from one drawn as its outline. The room schedule
  leaves the area of a boxed or approximate room blank.
- A family instance exported as `IfcSpace` whose family draws no model form
  for its parameters is no longer written, as Revit's export does not write
  it (#1262; measured on Revit 2024, applied to 2023 to 2027). The export
  diagnostics count it as `element_record_family_space_without_body`.
- Revit 2016 to 2023 exports leave out elements outside the export phase and
  in non-primary design options. Revit 2008 to 2023 projects whose element
  frames carry no ElementId of their own export many more elements (measured
  on one 2018 project).
- `rvt-ifc` writes the IFC to `<output>.partial` and renames it when whole,
  so a failed export leaves no truncated file under the output's name (with
  `--observation` it writes the file directly).
- At the default limits a partition may inflate to 16 times its stored size,
  1 GiB to 4 GiB on a 64-bit build, so `--large-model` is needed less often.
- A door or window sized from its symbol's box edges carries
  `RvtElementRecordGeometry.SymbolWireWidth`.
- `rvt::ifc::step_writer::write_step_to` streams the STEP text to any
  writer.
- The wasm binding `openRvtBytesWithDiagnosticsModeAndOpenLimits` takes an
  optional fourth argument, a progress callback.
- The export diagnostics add `exported.bounding_box_spaces`,
  `exported.resolved_outline_spaces`, `exported.approximate_outline_spaces`
  and a warning that counts doors and windows whose type record was not
  read.

### Added

- **The browser viewer shows the decoder's own progress, and says when a
  model is too large for a tab.** The loading card names the stage the
  decoder is in (the stages `rvt-gltf --progress` prints) and fills its bar
  with that stage's count, where it used to sweep and guess "about N s" from
  the file's size. `openRvtBytesWithDiagnosticsModeAndOpenLimits` takes an
  optional fourth argument, a function called with the stage, the count done
  and the count it is out of. `rvt-gltf --progress` now also prints a
  `partition-scan` line for each `Partitions/*` stream as it is first
  inflated, which is most of a project's open, where it printed the stage's
  start and end only. A file whose size says it may need more than a tab's
  4 GiB (at least about ten times its size, the figure of the largest
  reference model; the warning can fire on a file that fits) is warned about
  before the decode and still tried. When the wasm instance runs out of
  memory, the viewer says the model is too large for the browser, how far the
  decode got and where, and that `rvt-ifc` and `rvt-gltf` with `--large-model`
  (streams up to 2 GiB and, on a 64-bit machine, inflating up to 8 GiB, for a
  file of up to 2 GiB), or Python with a larger `max_stream_bytes` and
  `max_inflate_bytes`, can read a larger file, instead of
  `RuntimeError: unreachable`. A stop a long way short of the ceiling says
  "Ran out of memory" and that the file may be damaged, and a panic that is not
  a memory failure keeps its own message (#1195).

- **A column a clean join pairs with carries its `Length`.** On a column
  drawn as its record box whose base quantities RE-291's rule writes,
  `Qto_ColumnBaseQuantities.Length` is now written as the box's height where a
  clean pair joins it: Revit's export writes it on every such column of the
  BIM4LCA office, 186 of 186 as built and 250 of 250 at building permit,
  equal to that height. The 18 columns Revit draws as a cut mesh, with no
  `Length`, are those a cut slab straddles, which the rule leaves out (#1203).

- **Every `IfcSpace`'s plan area and outline is scored against Revit's export,
  and says where its outline came from.** Measure's new
  `space_profiles_vs_ifc` scorer pairs the spaces of rvt-rs's export with
  Revit's and counts those within 0.1% in area and 0.01 ft in outline (outline to outline, and by the mesh's vertices); the
  IfcOpenShell CI job runs it on Core Interior (116 of 116 rooms) and RE1
  Architecture (11 of 11) and fails below those counts. A room's
  `RvtElementRecordGeometry.SpaceOutlineSource` is `stored_solid` where its
  outline was read from the room's stored solid and `record_box` where the
  body is the record's bounding box and its plan area is the box's, not the
  space's (`saved_mesh` with `--saved-meshes`, `family_body` for a family
  instance or Revit Space drawn as more than its box, `none` for an Area,
  which has no body). A family instance written as a space gets the property
  too. `exported.bounding_box_spaces` in the diagnostics and a line in
  `rvt-ifc`'s summary count the spaces drawn as their record's box. What an
  `IfcSpace` holds per release, and what is measured where, is in
  [docs/rooms.md](docs/rooms.md) (#1260, RE-313).

### Changed

- **A partition may inflate to 16 times its stored size at the default
  limits, at least 1 GiB and at most 4 GiB on a 64-bit build.** It was
  1 GiB whatever its size. A 202 MB Revit 2024 model whose 148 MB partition
  inflates past 1 GiB, which 0.6.1 read by cutting the partition short
  without a word, has needed `--large-model` since 0.11.0; at 16 times its
  stored size it should now read whole at the defaults, which is not yet
  measured on that model. A small file still cannot make a reader hold more
  than 1 GiB of one stream, and a 32-bit build and the browser viewer keep
  1 GiB (#1075).

- **`rvt-ifc` writes the IFC as it builds it.** The STEP text was built
  whole in memory and then written. It is now handed to the file every MiB,
  after the file handle and the partitions it keeps inflated are released.
  The bytes are the same as before this change, but for the version the
  header records. The text goes to a file beside the output, renamed
  when whole, so a failed export leaves no truncated file under its name. On
  the BIM4LCA office the text is 68 MB and the handle 362 MiB of inflated
  partitions; the saved-graphics pass, which runs before, still sets the
  peak there (#1075). With `--observation` the text is still built whole,
  since the witness reads it back. `rvt::ifc::step_writer::write_step_to`
  streams to any writer.

- **Saved meshes read cone faces.** A `ConeSurf` face failed with
  `unsupported surface ConeSurf`, which kept the whole element's box
  (RE-224). It is read as a cone of revolution about its apex, `v` the
  distance along the generator. With `--saved-meshes` RE1 Plumbing draws 8
  more pipe transitions, every Revit vertex within 0.001 ft of the mesh and
  all within 0.01 ft of Revit's body (1.08 to 1.10 of its volume: Revit's
  export is a 6- or 8-sided polyhedron). No other body scored against
  Revit's export changes, on RE1, the office or the Revit 2025 models, and no
  default export changes. On Autodesk's 2024 samples, which have no Revit
  export, `--saved-meshes` draws 471 more bodies: counted, not
  scored. The two RE1 Electrical panels #1144 names keep their box for
  trims RE-302 and RE-305 measured (#1144, RE-311).

- **Exports read walls' native records once instead of twice, and skip
  per-record hashes no export uses.** Measured on GitHub's runners against
  0.12.x on every reference model and Autodesk's samples, with output
  byte-identical but for the version its header records: Revit 2015 to 2023 exports take 4% to 5% less time, 2024 3%, 2025
  3%, 2026 and 2027 1% to 2%, the reference and BIM4LCA models 4%. Peak
  memory is unchanged within 1 MiB except on Autodesk's 2015 rme basic
  sample, 361 to 386 MiB, which is not explained (#1237).

- **The export diagnostics count the doors and windows whose type record was
  not read.** Since #1271, a door or window whose type's record a read did
  not reach (a large file over its limits) is written as a door or window
  drawn as its record box, not as its opening alone. Its category comes from
  its own record, but whether it draws geometry was never read. A warning
  now gives their number, since one that is a cased opening in Revit is its
  opening alone in Revit's export (#1247).

- **A room whose stored solid the exact walk did not close is read again
  from the faces its solid lists, and labelled apart.** The walk stopped at a
  record longer than 0x200 bytes between two faces, walls on lines up to
  0.0015 ft apart left a step no face crosses, a column overlapping a wall
  put a loop partly inside the wall's, and a solid ran past the 64 KB window.
  The solid ends at the count of its list of objects and at a gap of 1,024
  bytes, so the faces of the element that follows are not read, and the work
  is bounded (512 faces, 4,096 pieces; crafted solids take under 10 ms).
  `SpaceOutlineSource` is `stored_solid_faces` where the faces' edges close as
  the walk reads them, and `stored_solid_approximate` where a column overlaps
  a wall (its area depends on whether Revit's export takes the column off),
  an end of one wall edge lies on another, or the outline falls up to
  0.02 ft short of the record box; `stored_solid`
  stays the exact walk's. The diagnostics count both
  (`exported.resolved_outline_spaces`, `exported.approximate_outline_spaces`)
  and the room schedule leaves an approximate room's area blank. On the
  BIM4LCA office as built the 13 rooms that kept their box take an outline:
  nine are `stored_solid_faces`, all at Revit's area to 0.1% (their boxes were
  3% to 67% over it), and four `stored_solid_approximate`, 0.45% to 0.68%
  under Revit's floor area. At building permit 11 of 12 take an outline, all
  `stored_solid_approximate`, ten at Revit's floor area to 0.1%; room 3095831
  keeps its box. 6 of the 8 rooms that kept their box do on a held-out Revit
  2021 project, all 6 at Revit's area. Measure's `space_profiles_vs_ifc`
  compares the faces of Revit's mesh that point up, which read a room under a
  stepped or sloped ceiling as a fraction of itself (room 1671995: 1218 of
  5177 ft²; RE-313's 66% room); the new `space_overlaps_vs_ifc` also compares
  the floor, where all 443 office `stored_solid` room scores agree. The six
  `rac_advanced` rooms with round columns, 49 of the 50 `rme` rooms and
  Snowdon Towers' seven keep their box. Core Interior and RE1 Architecture are
  unchanged. A resolved room carries no `NetVolume` (#1260, RE-316).

- **A roof sloped by several slope arrows is drawn as the planes they
  define, where Revit's export is measured to resolve them so.** Each
  arrow's plane rises from the boundary line its tail is on, at the arrow's
  slope, with the top a plane's own thickness over the cosine of its angle
  above the underside, and the planes meet in hips and valleys. On
  puzzbobb's `20_T_U_Roof` (Revit 2025) the T-shaped roof 4149 is now
  Revit's body, its farthest vertex 3.2e-06 ft from Revit's and its volume
  436.7078 ft³ against Revit's 436.7074 (it was the outline over the record
  box, 4,426.4 ft³). It is drawn on Revit 2025 only, for arrows set by
  their heights. The U-shaped roof 5941 stays its record box: Revit slopes
  its two long outer walls along the arms only, and a roof of its
  configuration (a sloped edge met inside by a reflex edge's extension,
  beside a sloped neighbour at a convex corner) is not drawn either, though
  only the U is measured (#885, RE-307).

- **Native record reads are faster: the graph decoder borrows the schema's
  classes instead of copying them for every object, and a values read takes
  only the types its elements name.** On one GitHub runner against `main`
  before it (Measure 41736361020), Revit 2015 to 2023 exports of Autodesk's
  samples take 5% to 19% less time (`rac_advanced` and `rme` 15% to 19%),
  their 2024 and 2025 samples 5% to 13%, and the BIM4LCA office 7% to 9%;
  RE1 Architecture, a one-second export, took 6% longer in that run and RE1
  Electrical 2% less. Peak memory is within 5% of before on every file, the
  largest rises Autodesk's 2015 `rme_basic` sample (361 to 379 MiB in that
  run) and RE1 Electrical (175 to 183 MiB). The IFC and the diagnostics are
  the same as the commit before this change (Measure 41740244025) (#1237,
  #1301; thanks to puzzbobb).

### Fixed

- **`--large-model` now raises the budget of the reads of native records,
  and a door or window is written as its opening alone only where its type's
  record was read.** Each native read took a fixed 256 MiB budget for one
  reassembled group, which `--large-model` did not raise. On a user's 438 MB
  Revit 2024 project a group over it stopped the read of the door types and
  of the saved graphics, even with `--large-model`. That project's 73 doors
  then fell back to the type-parameter scan, which RE-299 showed misses
  maps, and were written as their openings alone (#1247). The default
  budget now follows the file's inflate limit, so `--large-model` raises it,
  and is never more than the partition's own inflated bytes. On Revit 2024
  to 2027, a door or window is written as its opening alone only where its
  type's native record was read with an empty geometry map; a type the read
  did not reach is not taken to draw nothing. Earlier releases keep their
  rule.

- **Rooms on Revit 2026 and 2027 files take their outline, not their box.**
  The reader of a room's stored solid ran on Revit 2008 to 2025 only, so on
  a 2026 or 2027 file every room was its record box, and an L-shaped room's
  box covered its neighbours: Autodesk's `rac_basic` Entry Hall was 72.8 m²
  where its 2025 save gives 30.3 m². Every room of Autodesk's four sample
  projects with rooms (257) now gives the same verdict and area in its 2026
  and 2027 saves as in 2025 (#1260, reported by inevitable-co-ai).

- **A Revit 2008 to 2023 project whose element frames carry no ElementId of
  their own now exports those elements, on their storeys (measured on one
  Revit 2018 project).** Revit 2018's
  `ALD26 Before` exported none of the 1,394 elements and spaces in Revit's own
  export (#1261): most of its frames in `Partitions/0` do not open with a
  declared `u32` ElementId, but sit inside their element's partition record, as
  2024 and later frames do (RE-35). Those records are `u32` ElementId, `u32`
  size, the tag of `ElementHeader`, a count, the body and a 12-byte trailer that
  echoes the size; the 2024 walk's flag test stops at the first record with
  another flag word, after 115 of this file's 12,626. A placed-instance frame
  with no declared id now takes the id of the declared record around it when
  its own reference list names that id, and so does a Level frame that is a
  `Level` with a name block in its stream, which gives the file its 7 storeys.
  The file now writes 1,136 of Revit's 1,321 elements by ElementId, 1,127 of
  them in Revit's class or IFC4's for its IFC2X3 one (9 curtain walls come out
  as walls), none that Revit's export lacks, 1,073 on Revit's storey. A curtain
  panel frame that opens with a count rather than `ff ff ff ff` is left out,
  since 96 of the 102 on that file are the shells of panels Revit does not
  write. The 185 elements still missing are 178 conduits, conduit fittings and
  gutters, whose categories are not yet recovered, 4 panel walls and 3
  assemblies (RE-315). Files whose frames all open with their id export as
  before.
- **Revit 2016 to 2023 exports leave out the elements Revit's own export
  leaves out by phase and by design option.** Only 2024 and later applied the
  export phase (RE-184) and the primary design option (RE-40). A Revit 2018
  project wrote 198 pipes, fittings and other elements that Revit's export does
  not hold, and writes none now; the counts cannot say whether the phase or the
  design options removed them, and the held-out file is not opened to find
  out. The phase is read as RE-281 reads it. Design options are read as on 2024
  with `u32` ids: on the `rst_basic_sample_project` of each of 2016 to 2023 the
  one `Mass` in a non-primary option is found and counted, as the 2024 and 2025
  copies of the model count it, and leaves the production counts of the
  diagnostics without changing the exported IFC. Revit 2008 to 2015 keep every
  element (RE-315).

- **A family instance written as a space is left out where its family draws
  no model form for its parameters.** Revit's export writes a family
  instance that is not a door or a window only when it has a 3D body. On the
  BIM4LCA terrain office, 92 of the 94 "Parking Space" instances whose
  "Export to IFC As" is `IfcSpace` are bicycle bays whose "Stripe Y/N" is 0,
  so no form of their family is drawn, and Revit's export holds only the two
  car bays; `rvt` wrote 89 of the bicycle bays as `IfcSpace` too. The terrain
  office's `IfcSpace` are now the 9 of Revit's export (9 of 98 before), and
  the six reference models, the other office files, the R2025 and the
  held-out models export byte for byte as the commit before this change
  did. Only instances written as
  `IfcSpace` are tested, other categories are not, and the export
  diagnostics count what is left out (#1262).

- **A 28-byte `Global/ElemTable` is recognised by its first id, the one
  `declared_ids` reads as the ElementId.** The layout check tested the
  second id, which on 8 client files saved by Revit 2024 to 2026 with a 2022
  or 2023 schema holds an older, lower number on 1% to 37% of records, so
  their tables were read as 12-byte records with a "fits neither its 28- nor
  its 40-byte layout" warning. On the reporter's build of this change the
  warning is gone on all 8; whether their records' ElementIds are Revit's is
  not yet measured (#421, found by puzzbobb, measured by hemanth1986).

- **A door or window whose symbol cuts no loop takes its `OverallWidth`
  from the lines of no category in that symbol's saved graphics, and the
  BIM4LCA office's two wall hatches now have Revit's (RE-318, #810).**
  Doors 1756089 and 1765267 are `TRAPDOOR` hatches (family `LU-3.VSS`, types
  `_90°` and `_180°`), not doors with a sidelight. Their record box was
  0.1137 ft wider than Revit's 5.8629 ft, and neither their cut loop (none),
  cut solid, stored lengths nor visible surfaces give it: the 90° type hides
  the 180° leaf its width includes. A symbol's saved graphics hold the edges
  of the boxes that bound its forms as lines of category -1, hidden forms
  among them, and their extent along X is Revit's width on all 268 of the
  office's doors Revit sizes and all 269 windows as built (268 and 265 at
  building permit), where the record box missed 2 doors and the surfaces 2.
  The rule fires only where the record box sized the door or window before,
  and a symbol with no such line, a degenerate extent, an instance or
  component reference in its graphics, two readings that disagree, or edges
  reaching past the record box, keeps the width it had. Against `main`
  (Measure 41725344128) the office's `IfcDoor` widths equal Revit's on 268
  of 269, from 266, and three widths
  move on the as-built file (1756089, 1765267 and, by 7 micrometres, 1722215);
  the only other model with such a door, RE1 Architecture's curtain-wall
  door 445975, keeps its width, and 31 of 34 measured models and all 18
  held-out models write the same IFC as the commit before this change. One
  building's families: what the
  boxes are, and why their extent is Revit's width, is inferred from Revit's
  widths, not documented (`reports/element-framing/RE-318-symbol-wire-width.md`).

### Research

- **Why saved pipe fittings measure 1.2 times Revit's volume (RE-311).**
  Revit's export of a pipe elbow is a polyhedron of 6, 8 or 10 sides round
  the pipe and two segments along its bend, inscribed in the surface: 16 to
  26% short of the true torus, which rvt-rs's mesh reaches to within 1 to 2%.
  The closet bends' wrong volumes (0.28 to 2.59, one negative) were the
  scorer's, which took a mesh with cracks between its faces about the file's
  origin; `mesh_bodies_vs_ifc.py` now takes it about the body's centre (the
  three RE1 Mechanical bends: 1.1431 each) and counts the bodies whose Revit
  vertices all lie on the mesh. `fitting_faces_vs_ifc.py` and
  `probe_re311_fitting_faces` set each face of a saved fitting against
  Revit's body (#1144).

- **Measure scores the BIM4LCA terrain office, in concrete and in timber,
  against Revit's exports of them.** `-f nordic=true` now reads the two
  files at building permit and their IFC4 exports by byte range, beside the
  concrete office. `probe_re312_parking_spaces` and
  `tools/re/parking_spaces_vs_ifc.py` list the family instances written as
  `IfcSpace` against Revit's export and the model forms of their families
  (RE-312, #1262).

## [0.12.2] — 2026-11-03

0.12.2 draws more doors from the meshes Revit saved for display in the
default export, and more bodies with `--saved-meshes`. It also changes how
a door's type decides whether the door is written as its opening alone.
Both changes come from a user's comparison of 0.6.1 and 0.12.0 (#1247;
thanks to hemanth1986). Not every case that comparison found is fixed:
- the user's 341 beams lost their meshes on no file measured here, so
  their cause is open;
- toilets, railings and wall-mounted fittings whose saved meshes RE-302,
  RE-303 and RE-305 measured stay boxes, because those meshes are not
  Revit's body;
- the door change measures on no file here: its positive case is the
  user's own project, which has not been measured with it.

Measured against 0.12.1 on every reference model (Measure run 41589642386,
`main` before the version bump), in the default export, which draws doors
and windows from their saved meshes:
- the BIM4LCA office draws 269 doors from their saved meshes, against 48 as
  built and 46 at building permit, 265 of them exact to 0.001 ft against
  Revit's export and all within 0.01 ft;
- six R2025 models (12 to 17) each draw one storefront door from its saved
  mesh, within 0.01 ft of Revit's body and at 1.0002 of its volume, where
  0.12.1 drew a box;
- Core Interior, Einhoven, the RE1 models and the other R2025 models export
  the same IFC as with 0.12.1, but for the version its header records;
- 8 of the 13 held-out models change; they are counted, not scored.

The office as built's IFC grows from 58.6 to 67.8 MB, its saved-mesh
triangles from 66,872 to 257,108. With `--saved-meshes`, light fixtures and
more bodies are drawn too, with each entry's own caveats (below).

The viewer no longer calls a Revit 2008 to 2014 file an unsupported version
(below).

### Upgrading from 0.12.1

- A door or window whose saved mesh now decodes whole is drawn from it
  instead of its box, in `rvt-ifc`, `rvt-gltf`, Python and the viewer.
  `--no-saved-meshes`, `saved_meshes=False` and `?savedMeshes=0` keep the
  boxes.
- A door or window whose type's native map holds entries is written as an
  `IfcDoor` or `IfcWindow` filling its opening, where 0.12.1 may have
  written only its `IfcOpeningElement` (no measured file has such a door).
- `rvt-inspect`'s `unsupported_revit_version` message names 2008 to 2027;
  the `failure_mode.kind` value does not change.
- `rvt-capabilities --matrix` gives the Revit 2023 and 2024 project
  capabilities new names, in its text and JSON; their ids and statuses do
  not change.
- The stable public API does not change. `rvt::research` (unstable) gains
  `native_type_materials::GeometryMap` and `geometry_maps`.

### Changed

- **`rvt-capabilities --matrix` names the Revit 2023 and 2024 project
  capabilities for what they give.** `element-records-revit-2023` was
  "Revit 2023 elements from their partition element records: identity,
  category and box (RE-81)", though 2023 projects also get wall joins, beam
  cuts, room outlines, storeys, hosts, family and type names, materials and
  layer sets; `typed-project-elements`, the Revit 2024 row, was "Decode typed
  elements from real project files". The two names change, and their notes,
  which still said names, types, storeys and joins were not read on 2023, in
  the text and JSON of the built-in matrix; ids, statuses and evidence do not.
  `rvt-doc`'s note on older releases names the reconnaissance document by its
  new path, `docs/format-reconnaissance.md`. The
  README's support-by-release table is now rendered from the matrix, and CI
  fails when the two disagree (`tools/ci/readme-matrix.py`).

### Fixed

- **Saved meshes read cylinder faces whose trim is not a rectangle in their
  UV box.** Such a face was rejected, and with it the whole element, which
  kept its box. The face is now meshed in strips across its angle, each no
  wider than the chord bound. On the BIM4LCA office 155 more doors are drawn
  from their saved meshes, as the default export draws doors: 151 exact to
  0.001 ft against Revit's export, and all within 0.01 ft and 1% of its
  volume. Other files' new meshes with `--saved-meshes` are not all Revit's
  body: RE1 Plumbing draws 11 more pipe fittings, within 0.01 ft of Revit's
  surface but at 1.44 to 1.47 of its volume, and RE1 Architecture a toilet
  within 0.0126 ft. Autodesk's 2024 samples gain 163 and 124 (rme basic and
  advanced), 265 (Snowdon Towers plumbing), 34, 24 and 15 (Snowdon
  architectural and HVAC, rac basic), counted, not scored: no Revit export
  of them is scored (#1247, RE-304).

- **Saved meshes read cylinders whose saved basis is left-handed.** Such a
  face was rejected, and with it the whole element, which kept its box.
  It is the same cylinder with its surface normal turned. On the BIM4LCA
  office 100 more bodies are drawn from their saved meshes (102 at permit),
  among them doors that the default export draws, 66 exact to 0.001 ft
  against Revit's export and 93 within 0.01 ft. Snowdon Towers'
  architectural sample gains 172, counted, not scored: no Revit export of
  it is scored (#1247, RE-301).

- **With `--saved-meshes`, a light fixture's light source is left out of
  its body, instead of keeping the fixture's box.** The light source is saved as a
  filter of its own: a `GImposter` (a rendering asset with no surface)
  with the shape and lines that draw the light, none of it in Revit's
  export. On the BIM4LCA office 43 more bodies are drawn from their saved
  meshes, all exact to 0.001 ft against Revit's export, as are RE1
  Electrical's 2. Snowdon Towers' electrical sample draws 1,324 bodies from
  saved meshes instead of 907 and keeps 139 boxes instead of 556; no Revit
  export of it is scored, so those are counted, not scored (#1247, RE-300).

- **A door or window is no longer written as its opening alone when its
  type draws geometry in no material of its own.** RE-84 takes a type
  whose value block holds no geometry-material map as one that only cuts
  its host; the scan of that block missed a map whose values are all unset
  or name a material whose name it did not read. The type's native map
  now decides: it is empty on every door and window Revit exports as its
  opening alone (RE1's CasedOpening, and Snowdon Towers' 47 in Revit's
  export, which is local only), and those keep their openings. This fix alone
  changes no export of the reference models, the BIM4LCA office, the R2025
  models or Autodesk's samples. The case it was found on, the 73 doors of #1247's
  project written as openings, is the user's own file and has not been
  measured with it (#1247, RE-299).

- **The viewer no longer calls a Revit 2008 to 2014 file an unsupported
  version.** Its file status warned "Unsupported Revit version" for any file
  before Revit 2015, though rvt-rs reads 2008 to 2014 element records
  experimentally since RE-259 and RE-260 (#1076, #1077); it now warns only
  outside 2008 to 2027, as `rvt-inspect` decides. `rvt-inspect`'s
  `unsupported_revit_version` message, and the docs that quote it, name the
  same 2008 to 2027 range.

## [0.12.1] — 2026-11-01

0.12.1 makes exports of Revit 2015 to 2023 files faster than 0.12.0 made them,
each the same as 0.12.0's but for the version its IFC header records (#1237).
Timed on one runner against 0.12.0, on Autodesk's sample projects:
- 2015 to 2022 projects take 0.84 to 0.97 of the time;
- the three 2023 projects take 0.89 to 0.96;
- 2024 to 2027 take as long as before.

They are still slower than with 0.11.0. 2015 to 2022 projects take 1.6 to 2.9
times as long and 2023's 1.3 to 1.9 times, and some 2016 to 2022 files still
peak up to 19% higher in memory. That remaining cost comes with the native
records 0.12.0 reads on those releases, and #1237 tracks it.

### Upgrading from 0.12.0

Nothing changes except the rvt-rs version an IFC header records. Exports'
contents, diagnostics and the stable public API are the same.
`rvt::research` (unstable) gains `native_element_parameters::read_defined_values`.

### Changed

- **Exports of Revit 2015 to 2023 files are faster than in 0.12.0 (#1237).**
  Two changes:
  - the export's reads of the parameters named for a property and of a
    storey's `EntranceLevel` now share one read of the file's parameter
    definitions (#1239);
  - the read of the parameters named for a property leaves out the family
    and material records, which only name ElementId values (#1240).

  Timed on one runner against 0.12.0 (Measure run 41255129905), on Autodesk's
  sample projects:
  - 2015 to 2022 projects take 0.84 to 0.97 of 0.12.0's time, 12.7% less over
    all thirty;
  - the three 2023 projects take 0.89 to 0.96 (Einhoven, a 2023 model, 0.99);
  - 2024 to 2027 take as long as before (0.99 to 1.03);
  - no export's contents change, and no 2015 to 2023 file's peak memory rises
    against 0.12.0.

  Against 0.11.0 (run 41249804617):
  - 2015 to 2022 projects still take 1.6 to 2.9 times as long, 2.36 over all
    thirty;
  - 2023's take 1.3 to 1.9 times as long;
  - some 2016 to 2022 files peak up to 19% higher in memory;
  - 2024 to 2027 exports take 0.97 of 0.11.0's time in total, single files
    0.90 to 1.11.

  The largest part of what remains, from 43% to about half on the two samples
  profiled, is the named-parameter read decoding each element's native record
  (#1237).

## [0.12.0] — 2026-10-31

0.12.0 writes much more of what Revit's own IFC4 export writes, each value
where the model's records show what Revit writes, and leaves a value out
where Revit's own exporters disagree on it.

Base quantities Revit computes from its joined geometry are written where
the records decide them:
- walls' side areas, volumes and `Height`, and the `GrossFootprintArea` of a
  wall nothing joins to or cuts;
- the areas and volumes of box-drawn columns nothing cuts or crosses;
- floors' `NetVolume` and flat roofs' slab quantities where nothing cuts
  them, and ceilings' `GrossArea`;
- spaces' `NetVolume` where the room's solid is a prism of its outline;
- curtain mullions' `NetSurfaceArea` where their wall's mullions say which
  ends Revit keeps.

A floor's and a covering's `Width` are left out: Revit 2024's exporter writes
them, later ones do not.

These property values are written:
- unrotated beams' `Roll`, level beams' `Slope` and the `Span` of beams drawn
  along their location line (a rotated beam gets no `Roll`, a sloped one no
  `Slope`);
- storeys' `Reference` and `EntranceLevel`;
- structural columns' and framing's `LoadBearing`;
- type objects' common sets, without `IsExternal`, which Revit's newer
  exporters leave out;
- a stair's `TreadLengthAtInnerSide`, and with `--revit-set-placement` its
  flights' `Pset_StairFlightCommon`, measured on 14 and 5 stairs;
- curtain members' `IsExternal` by their curtain wall's value. Exterior
  members' values are checked against revit-ifc's source only, since no
  newer exporter's model has an exterior curtain wall.

A pipe's `InvertElevation` is now its height above the model's zero, as
Revit's export writes it.

On the BIM4LCA office as built, counting the objects rvt-rs writes:
- 29,262 of the 39,388 base quantities Revit's export writes are equal
  (Measure run 41213779326);
- with `--revit-set-placement`, 45,235 of the 45,525 property values are
  equal (run 41215480967);
- 1 differs in each.

What the census counts apart is not in those totals:
- values Revit's export gets wrong (10,390 quantities as built: mullions'
  `CrossSectionArea`, door openings' `Width` and `Height`, floors'
  `Perimeter`; RE-235 §5);
- openings rvt-rs does not write;
- the values the project's own property-set file places;
- the curtain members' values the exporter took from its cache.

No quantity is written that Revit's IFC4 exports leave out. rvt-rs writes 10
property values Revit's export does not as built and 14 at permit:
- 9 and 8 on openings and, at permit, 5 on one column, all on elements
  Revit's export does not hold;
- one on a fixture Revit's export writes as another class (#792).

`docs/release-1.0-criteria.md`, W7, has the details.

Native records are read on Revit 2015 to 2022 too: each element's saved
parameters, experimentally, and the phase order, measured against the same
projects saved by Revit 2023. Records that need the extensible-storage
catalogue before Revit 2021, and 2015 to 2017 `VarSketch` records, are
refused. On Revit 2015 to 2023, experimentally, walls' `IsExternal` and
`ExtendToStructure` and structural columns' and framing's `IsExternal` are
written. They are measured against:
- the same projects' later copies;
- modelo_bim's Revit IFC4 export;
- Revit 2015's IFC2X3 export of BIMcollab's example, passed through an
  optimizer and counted apart.

`ExtendToStructure`'s joined value is measured against an export only on
Revit 2024 and 2025.

A model with a partition over 512 MiB no longer loses every value the native
records give. Those readers read under the file's own limits, which
`--large-model` raises, and name a partition over the limit (#1075). That is
not yet measured on a model over the limits.

Python and the browser viewer draw the doors and windows of Revit 2023 to
2027 files from Revit's saved meshes where the whole mesh decodes, as
`rvt-ifc` has since 0.11.0. A wall's top is cut at the floor that covers its
whole plan. A wall a floor covers only in part keeps its full height where
Revit steps its top: 29 office walls as built and 26 at permit.

Exports of Revit 2015 to 2023 files take longer. The time comes with the
native records now read on those releases, and is not yet broken down by
stage (#1237). Timed on one runner against 0.11.0 (Measure run 41228075400),
on Autodesk's sample projects:
- 2015 to 2022 projects take 2.1 to 3.9 times as long, 0.9 s to 5.6 s more
  each (2.8 to 3.4 times over each release's samples together);
- 2023 projects take 1.6 to 2.4 times as long;
- peak memory is up to 19% higher.

Revit 2024 to 2027 exports take as long as before in total per release,
within 4%. Single files take 0.93 to 1.15 times as long, and small ones about
0.1 s more.

The public API that nothing used is removed, and other public types change;
"Upgrading from 0.11" lists each.

AhmadAnzar brought two out-of-date statements in `docs/status.md` and
`ROADMAP.md` up to date (#1214), a first contribution. Thank you.

### Upgrading from 0.11

- Rust code that names a removed public item stops compiling. "Removed"
  below lists each one and what to use instead. `rvt::capability`'s
  relation-domain types and `ifc::NotYetImplemented` are gone.
- `FieldType::ElementIdRef` has no `sub`, `referenced_tag` no longer keeps
  bit 15, and `FieldType::encode` writes the reference's six bytes.
- Code that builds an `IfcModel` with a struct literal adds
  `storey_properties` and `type_entity_sets` (`Default::default()`), and
  `PropertySet::storey_sets_for` takes the storey properties.
- `rvt-capabilities --json` writes `schema_version` 2, without the
  `relation_domains` section. `rvt-schema --json` writes no `sub` on an
  `ElementIdRef` field. The diagnostics' `parameter_value_count` counts the
  values on type objects too.
- Python's `write_ifc()` and `rvt_to_ifc()` draw doors and windows from their
  saved meshes. Pass `saved_meshes=False` for their boxes, as before.
- The viewer draws them from saved meshes too, with a File status option.
  `?savedMeshes=0` starts it off, and `openRvtBytesWithDiagnosticsModeAndOpenLimits`
  takes `savedMeshes`.
- At the default limits the native-record readers stop at 256 MiB, as the
  rest of the reader does, where they read up to 512 MiB before. Pass
  `--large-model` for a larger partition.
- A pipe on a raised storey gets a different `InvertElevation`: its height
  above the model's zero, not above its storey.
- An export no longer writes a floor's or a covering's
  `Qto_*BaseQuantities.Width`, which Revit's exporters disagree on.
- Exports of Revit 2015 to 2023 files take 1.6 to 3.9 times as long, 0.9 s
  to 5.6 s more on Autodesk's sample projects, with up to 19% more peak
  memory (#1237).

### Added

- **Unrotated beams' `Pset_BeamCommon.Roll` (W7, RE-276).** Revit's export
  writes a beam's Cross-Section Rotation, in radians, as its `Roll`. A
  beam's native record holds it in the `m_rotation` of its
  `FamInstCurveDriver`. rvt-rs reads it and writes `Roll` 0 for the beams it
  writes an extrusion length for whose rotation is exactly 0: on the BIM4LCA
  office, 8 of 8 beams as built and at building permit, each Revit's value. A
  rotated beam gets none, as no measured beam is rotated.

- **Level beams' `Pset_BeamCommon.Slope` (W7).** For a beam it exports as an
  extrusion, Revit's export takes the slope from a parameter named `Slope`,
  then from the extrusion's own slope, and otherwise as the angle between the
  beam's location line and the horizontal, 0 for a line in the plan. rvt-rs
  writes 0 on the beams it writes an extrusion length for whose location line
  stays in the plan: on the BIM4LCA office, 8 of 8 beams as built and at
  building permit, each Revit's value. A sloped beam gets none, as the
  extrusion's slope it would take is not measured.

- **Beams' `Pset_BeamCommon.Span` (W7).** Where it draws a beam as an
  extrusion and no parameter gives the span, Revit's export writes the
  extrusion's length as the beam's `Span`, the length its
  `Qto_BeamBaseQuantities.Length` holds. rvt-rs writes it for the beams it
  writes that length for, those drawn along their location line (RE-49): on
  the BIM4LCA office, 8 of 8 beams as built and at building permit, each
  Revit's value.

- **Ceilings' `GrossArea` (#1016, RE-297).** Revit's export writes a
  compound ceiling's `Qto_CoveringBaseQuantities.GrossArea` as the area of
  the profile its sketch gives, less the cut where Revit cuts it, and a cut
  shows in the ceiling's native record as a `CutPairGStep`. A ceiling drawn
  as one extrusion of its sketch whose record holds none carries that area,
  its holes taken off: 57 of the 90 values Revit's export writes on the
  BIM4LCA office as built and 45 of 75 at building permit, none different.
  The area of a ceiling in pieces (Revit's export gives one piece's), of a
  cut one and of the office's roofing coverings, ventilation units drawn as
  boxes, is not written.

- **A box-drawn column's areas and volumes (#1046, RE-291).** A column
  rvt-rs draws as its record's box, whose type is a plain rectangle (its
  length parameters are exactly `b` and `h`, or `b` alone, the box's sides),
  carries the box's `CrossSectionArea`, `OuterSurfaceArea`, `GrossVolume` and
  `NetVolume` when its pair steps are all read and every slab they name lies
  wholly above or below it, with no wall or covering cutting it, whether or
  not a clean pair joins it. On the BIM4LCA office (Revit 2024, the one
  building measured), Revit's export draws a column as a cut mesh, whose
  quantities are not the box's, exactly where a slab its cut pair names lies
  across an end of it; a round column's type holds a `Diameter` and gets
  none. They are Revit's on all 199 such columns of the office as built and
  265 at building permit, none different and none that Revit's export
  leaves out: the office goes from 28,329 to 29,125 of the 39,388 quantities equal
  as built (Measure run 41203821396). A joined column's `Length`, the
  round column and the 18 meshed columns stay unwritten (#1203).

- **Curtain mullions' and panels' `IsExternal` by their curtain wall's value
  (W7, RE-288, #1187).** Revit's `IsElementExternal` gives a family instance
  with no parameter and no type Function its host's value, which exporters
  from IFC_v24.2.20 on give a curtain wall's members; the exporter that made
  the BIM4LCA office's exports caches types instead (RE-269) and is counted
  apart. rvt-rs writes each member's curtain wall's value, and none on the
  member types: 10,262 values on the office as built, 10,299 at permit and
  12 on RE1 Architecture, whose 12 members are equal to its export's (0 of 12
  before). Only those three exports change. The exterior members' values are
  checked against revit-ifc's source only: no model of a newer exporter has
  an exterior curtain wall.

- **Floors' `NetVolume` (#1016, RE-295).** Revit's export writes a floor's
  `Qto_SlabBaseQuantities.NetVolume` as the volume of its solid: its
  `GrossArea` times the total thickness of its type's layers, flat or
  sloped, unless something cuts it. A floor whose native record holds
  neither a `CutPairGStep` nor a `WallCutoutGStep` carries that. On the
  BIM4LCA office that is 31 of the 63 `NetVolume` values Revit's export
  writes, as built and at building permit, none different and none that
  Revit's export leaves out. The volumes of the stair landings Revit draws
  as a mesh and of the roofs written as slabs are not written.

- **Curtain mullions' `NetSurfaceArea` (#1046, RE-268).** Revit's export
  sums the faces of a mullion's Revit solid, so an end face is not counted
  where the mullion meets its own continuation end to end, nor where it stops
  inside a crossing mullion, and is counted where it butts against a crossing
  mullion's side face. rvt-rs's boxes of a curtain wall's mullions are those
  solids, so it reads which ends are kept from where they meet and writes
  `Qto_MemberBaseQuantities.NetSurfaceArea` as the section's perimeter times
  the length plus that many of the section's areas, on the mullions drawn as
  an extrusion of their section (RE-243). On the BIM4LCA office that is 6,606
  of 10,003 mullions as built and 6,636 of 10,031 at building permit, each
  Revit's value. Left out are the mullions with an end that meets a mullion
  lying over another, an oblique one, a crossing one that holds the end off
  its axis and faces, one shorter than its section or one left out for
  these reasons, and every mullion on a curved or edited curtain wall.

- **Saved parameters and the phase order are read on Revit 2015 to 2022
  (RE-281, #1158).** `rvt-elements` and Python's `decoded_elements()` list
  each element's saved parameters by name on those releases, experimentally,
  and `rvt-ifc --revit-property-sets` writes them. Against the same project's
  Revit 2023 copy, 932,645 parameters of Autodesk's sample projects read by
  name and value are equal and 243 differ, over 23 projects, each an edit
  between the saves. `native_element::decode_base` took `Identifier` for
  Revit 2024's 8-byte `m_id64` and refused every Element base of 2015 to
  2023, so the project's phase order was not read on Revit 2023 either; it
  takes the 4-byte `m_id` too, and the order reads on all 33 sample projects
  of 2015 to 2023 with no element left out of the last phase, so no export
  changes. A record that needs the extensible-storage catalogue before Revit
  2021, or a 2015 to 2017 `VarSketch` record, is refused, and every other
  native reader (types, stairs, symbol cuts, mullion hosts, ports, saved
  meshes) stays on 2023 to 2027. BIMcollab's 16 footings name no Level, so
  #879 needs another rule.

- **Flat roofs' slab quantities (#1016, RE-298).** Revit's export writes a
  flat roof as a slab, and gives it `Qto_SlabBaseQuantities`: its `GrossArea`
  (its sketch's profile's area), its `Depth` (the layers' total, however
  many there are) and `GrossVolume` (the area times that depth), and its
  `NetVolume` where its record holds no cutting step. rvt-rs writes them on
  all 10 of the BIM4LCA office's flat roofs, as built and at building
  permit, none different and none that Revit's export leaves out; the roof a
  hatch cuts keeps Revit's smaller `NetVolume` unwritten, and a roof's
  `Width` (its profile's extent) stays unwritten. A level floor of several
  layers gets no `Depth` or `GrossVolume`: the office's export (Revit
  24.0.5) gives it its last layer's thickness, but Revit 26.4's export of
  the held-out Revit 2026 house writes another value on 2 such slabs, so
  rvt-rs leaves it out.

- **Spaces' `NetVolume` (#1016, RE-296).** Revit's export writes a space's
  `Qto_SpaceBaseQuantities.NetVolume` as the volume of the room's closed
  shell. Where the room's stored solid is a prism of its outline, every
  vertical face running from the floor of its record box to the ceiling and
  no face slanted, that volume is the outline's area, its holes taken off,
  times the box's height, and rvt-rs writes it. On the BIM4LCA office that
  is 167 of the 278 values Revit's export writes as built and 178 at
  building permit, and both rooms of each of puzzbobb's six R2025 models
  with rooms, none different and none that Revit's export leaves out. The
  volume of a room whose ceiling steps (54 as built), of the mapped
  accessibility volumes and of the rooms whose outline does not close is not
  written; nor is `GrossVolume`, which Revit's export writes on 88 of the
  167 prisms only and which nothing in the file tells from the 79 it leaves
  out.

- **A stair's `TreadLengthAtInnerSide` and flight set (W7, RE-287, #1186).**
  Revit's export writes on each `IfcStair` a `Pset_StairCommon.
  TreadLengthAtInnerSide` and, by revit-ifc's matching of a set to an entity
  by the start of its name, a `Pset_StairFlightCommon` holding the sums of its
  flights' risers and treads and their common riser height, tread length,
  nosing length and tread lengths at offset and at the inner side. rvt-rs
  writes the first always (the stair's tread depth, as a flight's) and the
  second with `rvt-ifc --revit-set-placement`: 111 values on each of the
  BIM4LCA office's two files (14 stairs; the spiral stair's tread count is
  not read) and 8 on each R2025 stair model, 7 on the spiral one, all equal
  to Revit's export's, none different. No other model's export changes. The
  mechanism in the exporter that fills the container's flight set is not read:
  the rule is measured on 14 and 5 stairs of one to three flights.

- **Storeys' `Pset_BuildingStoreyCommon.EntranceLevel` (W7, RE-275).**
  revit-ifc's definition of the set gives `EntranceLevel` no calculator, so
  Revit's export reads a yes/no parameter named
  `Pset_BuildingStoreyCommon.EntranceLevel`, then `EntranceLevel`, on the
  Level. rvt-rs writes the value the Level's own record holds, where it holds
  one: on the BIM4LCA office, as built and at building permit, 9 storeys (the
  Level_01 yes, 8 more no) and none for Sea Level, which holds no value, each
  Revit's. A file whose Levels declare no such parameter, as the R2025
  models', gets none.

- **Storeys' `Pset_BuildingStoreyCommon.Reference` (W7, RE-275).** Revit's
  export writes the name of a storey's Level type as its `Reference`. A
  Level's native record names its type in `m_attrId`, and the type's record
  holds the name: `Korkomerkintä` on each of the BIM4LCA office's 10 Levels,
  as built and at building permit, and `Level 1` on the R2025 models'. rvt-rs
  writes it on each storey, 20 on the office and 110 on the R2025 models, each
  Revit's value. `IfcModel` gains `storey_properties`, by storey index, and
  `PropertySet::storey_sets_for` takes it.

- **`LoadBearing` on structural columns and framing (#1158, RE-285).**
  Revit's exporter calls a structural column or structural framing element
  load bearing by its category alone. rvt-rs now writes that `LoadBearing`
  in its common property set on every release, unless a parameter named for
  it says otherwise, and on Revit 2015 to 2023, experimentally, the
  element's `IsExternal`, not external, as the calculation's last step gives
  it. Against Revit's exports: BIMcollab's Revit 2015 example 91 of 91 beams
  and 15 of 15 columns for each property, and modelo_bim (a Revit 2023 file,
  Revit 26.2's exporter) 8 of 8 beams and columns for `IsExternal` and 9 of 9
  for `LoadBearing` (the ninth on the type objects RE-286 writes); on the
  BIM4LCA office as built one column with no such parameter gains Revit's
  value. None different, and no new value that Revit's export leaves out.

- **Type objects' common property sets (W7, RE-286, #1185).** Revit's IFC4
  export gives each type object the common set of its entity, with the
  properties its entries give the type itself. rvt-rs now writes them: the
  type's Fire Rating (RE-77), `LoadBearing` for the structural column and
  framing categories, and a stair type's and flight type's dimensions (a
  stair type's `NosingLength`, `NumberOfRiser` and `NumberOfTreads` at 0, in
  `Pset_StairFlightCommon` too with `--revit-set-placement`). These are equal
  to Revit's export on 34 of the 182 values it writes on the BIM4LCA office's
  type objects as built (29 of 154 at permit), on 34 on the R2025 models and
  on modelo_bim's column and beam types, none different and none that Revit's
  export leaves out. No type object gets `IsExternal`: the office's and the
  R2025 models' exporters write it, Revit's newer ones (the held-out models',
  RE1 Architecture's) do not, and nothing in a file says which exporter will
  read it. `IfcModel` gains `type_entity_sets`, by type and type entity,
  which the writer adds to the type object's `HasPropertySets` (code that
  builds an `IfcModel` with a struct literal adds
  `type_entity_sets: Default::default()`), and the diagnostics'
  `parameter_value_count` counts a type object's `Pset_` values. Core
  Interior's `IFCPROPERTYSET` count goes from 6,508 to 6,510.

- **More roof-joined walls' side areas and volumes (#1046, RE-290).** The
  longest vertical face of Revit's solid for a plain wall is as long as the
  centreline cut back where the wall meets the walls joined at its ends,
  which rvt-rs reads wherever both ends agree with the record's join reaches.
  A wall joined to roofs, floors or footings that lie clear of it, whose
  body is its record's box, now carries `GrossSideArea`, `NetSideArea`,
  `GrossVolume` and `NetVolume` when that length is its box's long side, not
  only when the record confirms its `Length`. They are Revit's on all 29 such
  walls of the BIM4LCA office as built and 36 at building permit, none
  different and none that Revit's export leaves out: the office goes from
  28,162 to 28,278 of the 39,388 quantities equal as built (Measure run
  41182651693).

- **A roof-joined wall's side areas and volumes (#1046, RE-270).** A wall
  joined to roofs, floors or footings that rvt-rs draws as plain extrusions
  lying wholly above its top or wholly below its base (after the top cut at
  the floor that covers it, RE-267), with its length confirmed by its record
  (the curve cut back at its joins as long as the curve), and no step in its
  record but those and a clean join, carries the same `GrossSideArea`,
  `NetSideArea`, `GrossVolume` and `NetVolume` as a wall nothing joins to or
  cuts. They are Revit's on all 18 such walls of the BIM4LCA office as built
  and 22 at building permit (2 of each cut at a floor), none different and
  none that Revit's export leaves out: the office goes from 27,948 to 28,020
  of the 39,721 quantities equal as built and from 27,878 to 27,966 at
  permit (Measure run 41120485504). A wall a floor passes through, one cut
  by a pair or a cutout, or one joined to a sloped roof (the R2025 models')
  stays unwritten.

- **A wall's side areas and volumes (#1046, RE-270).** A wall whose native
  record holds no step that cuts, embeds or sweeps it and whose body is its
  record's box cut back at its joins, a rectangle, carries
  `Qto_WallBaseQuantities.GrossSideArea`, `NetSideArea`, `GrossVolume` and
  `NetVolume`, as revit-ifc's `CreateWallBaseQuantities` works them out of
  the wall's solid: the longer side's area, the rectangle times the height,
  and the wall type's width times the side area. They are Revit's on all 49
  of the BIM4LCA office's such walls as built, 48 at building permit and 91
  of puzzbobb's R2025 walls, none different and none that Revit's export
  leaves out; the office as built goes from 27,752 to 27,948 of the 39,721
  quantities equal, at permit from 27,686 to 27,878 and the R2025 models
  from 411 to 775 (Measure run 41108951821). Revit writes the four on
  exactly the walls it writes a `Height` on, and a `NetVolume` on the rest;
  the walls with a top join, openings, cut pairs, embedded walls or sweeps
  stay unwritten.

- **Walls' `Pset_WallCommon.ExtendToStructure` on Revit 2015 to 2023,
  experimentally (RE-284, #1158).** Whether a wall's top is joined to a roof
  or floor is read from its native record (a `JoinToRoofGStep` with
  `m_baseOrTop` 1), as on Revit 2024 and 2025. Against the same project
  saved by a later release it is equal on all 1,983 walls both files hold,
  over 24 pairs of Autodesk's sample projects of 2015 to 2022 and their 2023
  copies, with both values present (124 of the 2015 `rac_advanced`'s 180 walls
  are joined). Revit's own exports agree on the 114 walls of BIMcollab's
  Revit 2015 example and the 4 of modelo_bim, which are all not joined; the
  joined value is measured against an export only on 2024 and 2025. A wall's
  height, top cut and side quantities are still read from 2024 on.

- **A wall's `GrossFootprintArea` (#1046, RE-292).** A wall nothing joins
  to or cuts, whose body is its record's box cut back at its joins and that
  is more than three times as long as it is thick, carries `GrossFootprintArea`,
  its length times its width, beside the side areas and volumes it already
  gets. Revit's export writes it on the walls it draws as an extrusion, and
  it is Revit's on all 48 such walls of the BIM4LCA office as built, 47 at
  building permit and 91 of the R2025 models', none different and none that
  Revit's export leaves out (the one wall as long as it is thick, which it
  gives none, is not written): the office goes from 28,162 to 28,210 of the
  39,388 quantities equal as built (Measure run 41183025562). Walls joined to roofs
  or floors get one from Revit on 78 of 85 as built and nothing in the
  record tells which, so they stay unwritten.

- **A roof-joined wall's `Height` (#1046, RE-293).** A wall joined to roofs,
  floors or footings that lie clear of it, which gets `GrossSideArea`,
  `NetSideArea`, `GrossVolume` and `NetVolume` from its record's box (RE-270,
  RE-290), now also carries `Qto_WallBaseQuantities.Height`, the body's:
  Revit's export writes it on exactly those walls, and rvt-rs's extrusion is
  as tall as Revit's solid on every one. 47 more walls of the BIM4LCA
  office as built and 58 at building permit carry it, Revit's on every
  one, none different and none that Revit's export leaves out: the office
  goes from 28,278 to 28,492 of the 39,388 quantities equal as built
  (Measure run 41183229365).

- **Walls' `IsExternal` and the parameters named for a standard property
  on Revit 2015 to 2023, experimentally (RE-283, #1158).** A wall's type
  Function (`BasicWallType.m_function`) is read from its native records on
  those releases, and the project and shared parameters named `IsExternal`,
  `<set>.IsExternal` or for a property of one of IFC4's sets are read from the
  saved parameters, so a wall's `IsExternal` and those properties are written
  as on 2024 and 2025. The Function is equal to the same project's later
  copy on 1,982 of 1,983 walls over 24 pairs of Autodesk's samples (the other
  is an edit between the saves). Revit's own exports agree on the 114 walls of
  BIMcollab's Revit 2015 example (`IsExternal` and `LoadBearing`, which two
  parameters on its walls give) and the 4 walls of modelo_bim, a Revit 2023
  file (`IsExternal`, from the Function), none different. Doors, windows and
  slabs get none: their Function is not read on these releases.

### Changed

- **`FieldType::ElementIdRef` is a class reference and nothing more
  (#1101, the maintainer, 2026-10-30).** A schema field's by-value reference
  is the type word `0e 00 00 00` and a `u16` naming the referenced class,
  six bytes; where its bit 15 is set the class is defined inline after it.
  The variant's `sub` read the two bytes after those six, which belong to
  whatever follows: the next field's name length on about 1,900 fields of a
  2024 to 2026 schema, the pad before an inline definition on about 240,
  and a class's GUID-list count on about 600. `sub` is gone from
  `ElementIdRef` and from `rvt-schema --json`, `referenced_tag` no longer
  keeps bit 15, and every reference to the root `ElementId` class reads as
  `ElementId`, where that used to depend on the next field.
  `FieldType::encode` writes the six bytes. Code that names `sub` stops
  compiling: drop it. No export changes (Measure run 41102077364, every
  reference model, the R2025 models, Autodesk's samples and the family
  corpus); the walker's schema scan finds fewer low-confidence candidates,
  which the default export already left out, so the diagnostics count fewer
  suppressed ones (1,138 to 91 `HostObjAttr` on Autodesk's 2027 MEP sample,
  on 39 files in all).

- **A pipe's `InvertElevation` is the height above the model's zero, and a
  Revit 2024 vertical pipe has one (W7, RE-157).** Revit's export writes the
  lower end's height in the model's frame, not above the pipe's storey: the
  BIM4LCA office's 12 vertical pipes stand on a storey 55.5 m up and carry
  55.32 m. RE1's pipes are all on the storey at zero, which hid the
  difference. rvt-rs no longer takes the storey's elevation off, and a
  vertical pipe's invert, its lower end, no longer waits for an inner
  diameter that Revit 2024 pipes do not have read, so the office's 12 pipes
  as built and at building permit have Revit's value where they had none.

- **The property census counts curtain members' `IsExternal` apart, as the
  exporter's own output (W7, RE-269).** The exporter that made the BIM4LCA
  office's exports gives a curtain mullion or panel its curtain wall's
  `IsExternal` and then caches each member type as not external, so every
  later wall's members of that type read false: a value the model does not
  hold, which rvt-rs does not write (the maintainer, 2026-10-30).
  `tools/re/psets_vs_ifc.py` no longer counts those values as missing, nor
  the mullion types' own: 10,267 on the office as built (10,003 mullions,
  260 panels, 4 mullion types) and 10,304 at building permit are listed
  apart, with the other values Revit's export gets wrong (RE-235 §5). It
  counts them only where the export's own type objects hold the value, so
  RE1 Architecture's 12 members, exported by a later exporter, are still
  scored. On the office as built the census goes from 12,672 missing of
  57,773 to 2,405 of 47,506 with `--revit-set-placement`, and from 13,638
  to 3,371 without it; equal (45,045 and 44,079) and different (56) do not
  move, and no export changes.

- **The property census counts what a project's own export-time property set
  file places apart, as out of W7's scope (RE-239).** The maintainer ruled
  those values out of scope on 2026-10-26; `tools/re/psets_vs_ifc.py` now
  lists them apart instead of as missing: in an IFC4 Reference View export,
  the glazing set on elements that are not doors or windows, and the space
  occupancy set. On the BIM4LCA office as built that is 1,981 values (1,907
  at permit); with `--revit-set-placement` the census's missing values go
  from 2,381 to 455 and its different ones from 56 to 1, and equal is
  unchanged on every model. No export changes.

- **`psets_vs_ifc` scores each opening against the opening it corresponds to
  (W7).** Revit's export writes several openings with one `Tag`, the
  ElementId of the element they belong to: a door drawn as an opening, filled
  by its door, and, in each further wall or door that element cuts, an
  opening with no filling that holds `Pset_OpeningElementCommon.Reference`
  and `FireExit`. Keyed by `Tag` alone, the census scored those values
  against rvt-rs's filled opening, or against a curtain wall that shares the
  Tag, and listed them as missing: on the BIM4LCA office as built, 140
  `Reference` values and 31 `FireExit` values that sit on openings rvt-rs
  does not write (a curtain wall's 69 cuts, 39 fire hose cabinets' and 31
  door openings' cuts in further walls). An opening is now keyed by whether
  something fills it, and the values on openings rvt-rs does not write are
  counted apart, not dropped.

- **Python draws doors and windows from their saved meshes, as `rvt-ifc`
  does.** Since 0.11.0 `rvt-ifc` and `rvt-gltf` draw the doors and windows
  of Revit 2023 to 2027 files from the meshes Revit saved for them, where all
  of a mesh decodes; Python's `write_ifc()` and `rvt_to_ifc()` still drew
  their bounding boxes, so the same file gave a different IFC (found checking
  the 0.11.0 wheel against the CLI). They now draw the same bodies, and
  `export_diagnostics()`, `export_diagnostics_json()` and
  `rvt_to_ifc_diagnostics()` describe that export. `saved_meshes=False` on
  each keeps the boxes, as `rvt-ifc --no-saved-meshes` does; a
  `RuntimeWarning` says when the saved graphics could not be read.
  `rvt::ifc::saved_meshes::default_scope` gives the default to Rust callers.

- **The quantity census keys an opening by whether something fills it, as
  the property census does (#1161).** `tools/re/qtos_vs_ifc.py` keyed every
  opening by its `Tag` alone, so the further cuts Revit's export writes for
  a door or a curtain wall were scored against rvt-rs's door opening. Each
  opening now carries the classes of what fills it, so a door's opening is
  still known for RE-235 §5's swapped `Width` and `Height`, and the
  quantities on openings rvt-rs does not write are counted apart (#1165).
  On the BIM4LCA office as built that line lists 527 quantities (some of
  them were hidden before, a later opening of the same `Tag` overwriting an
  earlier one); missing goes from 11,700 to 11,367, the swapped door-opening
  `Width` and `Height` set aside from 191 to 187 each (the 4 others are a
  door's further cuts), and equal and different are unchanged on every
  model. No export changes.

- **The viewer draws doors and windows from their saved meshes, with a
  toggle.** Since 0.11.0 `rvt-ifc` and `rvt-gltf` draw the doors and windows of
  Revit 2023 to 2027 files from the meshes Revit saved for them, where all of a
  mesh decodes; the browser viewer still drew their bounding boxes, so the
  model on screen, its glTF and its IFC differed from the command line's for
  the same file. It now draws the same bodies, and File status has a "Draw
  doors and windows from saved meshes" option, on by default; changing it
  reopens the open file, and `?savedMeshes=0` in the address starts with it
  off. `openRvtBytesWithDiagnosticsModeAndOpenLimits` takes `savedMeshes` for
  it. The cost, measured in Playwright (Chromium on a 4-CPU runner, one open
  each, `tests/saved-meshes-measure.spec.ts`, which Deploy viewer runs and
  uploads): Core Interior (32 MiB, 138 doors and windows, all drawn from
  their meshes) loads in 25.6 s against 24.5 s with boxes, and the wasm
  memory after the load is 324 MiB either way; Einhoven and RE1
  Architecture load 0.1 s and 0.3 s slower with the same memory, and the three
  RE1 models with no doors or windows read nothing extra.

### Fixed

- **A partition over 512 MiB no longer stops every reader of native records
  ([#1075](https://github.com/DrunkOnJava/rvt-rs/issues/1075)).** The readers
  of native records (types, materials, stairs, phases, saved parameters and
  others) kept a stream cap of their own, 512 MiB, that `--large-model` did
  not raise. A model with a larger partition got none of their values, not
  only that partition's, and the diagnostics did not say so. They now read under the file's own
  limits: `--large-model` raises theirs too, and a partition over the limit is
  named in the diagnostics and skipped while the rest of the file is read.
  An element whose current record sits in that partition is left out rather
  than read from an older copy in another partition. At
  the default limits they now stop at 256 MiB, as the rest of the reader
  does, where they read up to 512 MiB before. Not yet measured on a model
  over the limits: a contributor's model with a 1.33 GB partition (#1117) is
  the first such file.

- **A wall's top is cut at the floor that covers it (RE-267, #1067).** A wall
  whose top join or Join Geometry pair names a slab, roof or footing is drawn
  up to that element's underside when the element's outline covers the whole
  of the wall's plan, clear of its holes and openings, as Revit's export draws
  it: on the BIM4LCA office 64 of the 65 tops Revit cuts as built and 70 of 70
  at building permit are now Revit's, no other wall's base or top moves, and
  the exports of Core Interior, RE1 Architecture and the R2025 models are
  byte-identical (Measure run 41092512565). A wall that a floor covers in part
  keeps its full height, where Revit steps the top (29 office walls as built
  and 26 at permit), and so does the one wall as built whose plan lies
  0.0243 ft² outside the floor and which Revit cuts all the same. A cut wall
  carries `BodyHeightSource` `wall_top_cut_at_covering_floor` and
  `TopCoverElementId`, the element it is cut at, and no `Height`, which
  Revit's export omits on 2 of them as built and 4 at permit.

### Removed

- **`rvt-capabilities` no longer lists relation domains, and
  `ifc::NotYetImplemented` is gone (the maintainer, 2026-10-30).** The
  capability manifest's `relation_domains` section and its
  `relations.typed_domains` row described the typed-relation module that
  0.11.0 removed (#1131): the row claimed a registry and "SCC/condensation/
  quarantine stubs exist", which was no longer true. The section leaves
  `rvt-capabilities` (text and `--json`) and
  `docs/schemas/capability-manifest.schema.json`, whose `schema_version` is
  now 2 (a consumer that reads version 1 sees the section is gone), and
  `rvt::capability::RelationDomain`, `RelationDomainRegistry`,
  `RelationDomainEntry` and `DomainStatus` leave the API. Nothing replaces
  them: no code read or wrote a relation domain. `ifc::NotYetImplemented`,
  an error no exporter returned, goes with them. Two remaining rows of the
  manifest say in plain words what they meant: nothing reads the compound
  ArcWall variant's openings, and remapping Extensible Storage ElementIds
  needs Revit API output the project does not have.

- **A floor's and a covering's `Width` (#1046, RE-294).** Revit 2024's
  exporter (24.0.5) writes `Qto_SlabBaseQuantities.Width` on a compound
  floor and `Qto_CoveringBaseQuantities.Width` on a ceiling and on a wall
  it exports as a covering; the exporters that wrote two held-out models (a
  Revit 2025 and a Revit 2026 file) write none on them, so rvt-rs wrote
  values Revit's export omits there. Where Revit's exporters disagree on a
  value, rvt-rs leaves it out, and these are no longer written, including
  the ceilings' `Width` of 0.11.0 (RE-250): the BIM4LCA office loses 113
  quantities that were equal as built and 89 at building permit (they
  become missing, none different), and the held-out models go from 2 and 3
  values Revit's export omits to none (Measure run 41204218380).

- **Public API nothing in the workspace used is removed, as planned before
  1.0 (the maintainer, 2026-10-30).** No exporter, CLI, binding, test,
  example, benchmark or fuzz target called these, so 1.0 doesn't promise
  them. Most were convenience wrappers over fields that stay public:
  - `Floor::is_depressed`, `Door::is_flipped`, `Window::sill_height_inches`,
    `Wall::is_unconnected` and `WallType::width_inches`: read the field
    they wrapped.
  - `WallKind::is_basic` and `WallFunction::to_ifc_predefined`: match on the
    enum.
  - `FloorBoundaryLoop::area_sqft` and `to_curve_loop`: the loop's points
    stay public, so compute the area from them.
  - `Extrusion::t_shape`, `l_shape`, `u_shape`, `rectangle_hollow`,
    `circle_hollow` and `arbitrary_with_voids`: build the `ProfileDef`
    variant directly.
  - `MaterialLayerSet::total_thickness_feet`: sum the layers.
  - `ifc::category_map::is_ifc_member`, `EvidenceTier::meets_verified_gate`,
    `InflateShortfall::is_complete` and `RoundTripReport::is_typed_clean`.
  - `RevitFile::preview_png_untrimmed`, `preview_raw` and
    `transmission_data_probe`: `preview_png` remains, and
    `transmission_data::probe_transmission_data` is what the last one
    wrapped.
  - `WalkerControl::cancel_token`: keep a clone of the `CancelToken` you
    pass to `with_cancel`.
  - `SchemaTable::tagged_ancestor_map`, `walker::encode_adocument_fields`
    and `ADocumentInstance::elem_table_pointer`.

### Research

- **Why Revit's export writes `IsExternal` true on 528 curtain members
  (RE-269, #35).** The value is not in the model: revit-ifc's
  `IsElementExternal` gives a curtain mullion or panel its curtain wall's
  value, and the exporter that made the BIM4LCA office's exports (24.0.5.432)
  also caches each member type as not external once the first wall that uses
  it is done, so every later wall's members of that type read false. Replaying
  that cache over the walls in ElementId order is Revit's value on all 10,263
  members and panels of the office as built (498 mullions and 30 panels true)
  and all 10,300 at building permit, none different; the host rule is Revit's
  on 1,651 and 1,691. rvt-rs still writes no `IsExternal` on curtain mullions
  and panels: the exporter version is not in the model, and exporters from
  24.2.20 on do not cache types. A probe lists each member's type, host and
  the host's Function, and `tools/re/member_is_external_cache_vs_ifc.py`
  scores both rules and prints the export's exporter version.

- **Reading an ElementId in a 4-byte `Identifier` in every native reader
  moves three Revit 2023 exports no export checks (RE-282, #1158).** Revit
  2015 to 2023 save `Identifier` as a 4-byte `m_id`, 2024 and later as an
  `m_id64`, and every native reader but the Element base and the phase order
  reads only the second. Reading both shapes everywhere leaves 91 of 94
  exports byte-identical on Measure's unfiltered run (all 30 samples of 2015
  to 2022, BIMcollab, the held-out models, all 25 samples of 2024 to 2027,
  the R2025 models, the offices, Core Interior and RE1) and moves Einhoven
  and two of Autodesk's 2023 samples: 36 and 30 connector ports, a wall
  layer set and material associations appear that nothing measures on
  Revit 2023. So each reader is
  admitted to 2015 to 2022 on its own measurement, reading the ids it needs
  itself, and the other readers stay silent on 2023.

- **The native record framing holds on Revit 2015 to 2022 (RE-280, #1158).**
  On Autodesk's 30 sample projects of those releases and BIMcollab's
  Revit 2015 example, every one of 27,762 record groups frames under the
  layout Revit 2023 uses (a `u32` id, a word, the length, the body and the
  length again), the five global streams parse (the extensible-storage
  catalogue on 22 of 31), and 98.8 to 100 per cent of the declared
  elements' object graphs decode, with the owner id agreeing on every one.
  Before Revit 2018 a group's declared body size counts each record's
  trailing length word, on all 12,005 groups of 2015 to 2017. Against the
  same project's 2023 copy, every ElementId both files hold has the same
  class on 23 pairs, and every saved parameter of 2021 and 2022 has the same
  value. Nothing is admitted yet: `native_element::decode_base` refuses
  `Identifier` as 2015 to 2023 write it (a 4-byte `m_id`), which also keeps
  the phase filter from reading Revit 2023, and the extensible-storage
  catalogue needs `m_specTypeId`, which 2021 introduced.
  `probes/examples/probe_re280_native_framing_pre2023.rs` is the measurement.

- **Three BIM4LCA office properties the file does not decide (W7, RE-277).**
  Columns' `Pset_ColumnCommon.Slope` (73 of 279 as built, 101 of 345 at
  permit, all 0.0) comes from the last geometry object Revit's exporter sees
  for the column, and the file does not fix it: six columns have a native
  record identical in both files and Revit gives them the property only at
  permit, and twelve identical columns on one level split 5 to 7 by whether a
  join is user-set. The building's `Reference` (`Project Information`) is a
  string the file does not hold. The door openings' 140 `Reference` and 31
  `FireExit` values are on the openings Revit writes where a door cuts a
  further wall, which rvt-rs does not write. None is written. The probe
  `probe_w7_office_gaps` lists each Level's, beam's, column's and pipe's
  native record for the office and the R2025 models.

- **What Revit's wall areas and volumes are (RE-270, #1046).** In revit-ifc
  24.1 and 24.2 (the office's exporter) a wall's `GrossSideArea` is the
  largest vertical face's outer boundary and `NetSideArea` the largest face
  with its holes out, the solid's volume is `NetVolume`, and `GrossVolume` is
  the wall type's width times the `GrossSideArea`. Worked out of Revit's own
  bodies they give 435 and 430 of the office's 579 walls as built (grouping
  the faces by normal, as 25.2 does, gives 270 and 276) and all 138 of the
  R2025 models'; `GrossFootprintArea` is `Length` times `Width` on all 279
  walls that carry it. Revit writes the four on exactly the walls it writes
  a `Height` on, and `NetVolume` on every wall, 28 more as built.
  `tools/re/wall_quantities_vs_ifc.py` is the measurement.

- **Revit cuts a wall's top only where the floor covers it (RE-267, #1067).**
  The 5 walls per stage that RE-258 found Revit leaves whole are cut over the
  part of their plan the floor covers and left at full height over the rest,
  so the highest point does not move: on the BIM4LCA office the area of
  Revit's body at the wall's full height equals the wall's plan outside the
  floor on 79 of 83 walls as built and 83 of 87 at permit. A rule that cuts a
  top where the floor named by its top join or a cut pair covers the wall's
  whole plan gives 64 of the 65 cut tops as built and 70 of 70 at permit,
  and cuts no wall Revit leaves whole; one wall's 0.085 ft stub past the
  floor's edge is cut by Revit and unexplained, and a part-covered wall
  needs a stepped top, which this export does not draw.
  `tools/re/wall_top_cover_vs_ifc.py` and
  `probes/examples/probe_1067_native_dump.rs` are the measurement.

## [0.11.0] — 2026-10-30

0.11.0 stops a model too large for the reader's limits from exporting
empty without a word, reads projects that kept an older release's schema,
and leaves out what Revit's own export leaves out: elements that do not
exist in the export phase, and property values Revit does not write. It
also removes, in one breaking change, the stable API that nothing used.

A stream over the reader's resource limits is now named in the diagnostics,
and `rvt-ifc` and `rvt-gltf` exit with status 3 instead of writing an empty
model; `--large-model` reads streams up to 2 GiB, and the viewer offers the
same retry (#1075). A project saved by Revit 2024 to 2027 that kept the
schema of the release it passed through is read as that release, where it
read as zero elements (#421). Neither fix is measured yet on the files that
showed the problem, which are private. #1075 is measured by lowering the
limits below the streams of the reference files (Core Interior's 17 MB
partition against an 8 MiB limit), and no model over the default limits has
been read with `--large-model`. #421 is measured only on the reference and
sample files, none of which keeps an older schema. Doors and windows are
drawn from the meshes Revit saved for them where the whole mesh decodes,
windows whose family cuts a solid get Revit's opening, a one-storey project
gets its storey, and walls nothing joins to or cuts carry Revit's `Height`.
Revit 2027 joins the family corpus. Revit 2008 to 2013 files now open, and
2008 to 2014 projects export their elements experimentally, outside the 1.0
range.

puzzbobb wrote the phase filter (#1074), the reader for files that kept an
older schema (#1081) and the Revit 2008 to 2014 readers (#1076, #1077), and
n3ur0m4n `rvt-gltf --progress` (#1073). Thank you.

### Upgrading from 0.10

- Rust code that names a removed public item stops compiling: "Removed"
  below lists each one and what to use instead. The decoders become
  `SimpleDecoder::new("<class>")`, and `ifc::from_decoded::slab_extrusion`
  takes the slab's thickness in feet (`Option<f64>`).
- `rvt-ifc` and `rvt-gltf` exit with status 3 and write no model when a
  stream is over a resource limit. Pass `--large-model` (or
  `--max-stream-bytes` / `--max-inflate-bytes`) to read it, or
  `--allow-partial` to write what was read. Python's `write_ifc()` and
  `rvt_to_ifc()` raise `ValueError` in that case unless
  `allow_partial=True`.
- Doors and windows are drawn from their saved meshes by default;
  `--no-saved-meshes` draws them as their boxes, as 0.10 did.
- An export no longer writes `Pset_DoorWindowGlazingType.IsWired`,
  `Pset_BuildingCommon.NumberOfStoreys` where a storey holds no element,
  or `Reference` outside the 34 sets a Revit export confirms, and its
  `IfcOwnerHistory` says `.NOCHANGE.`.
- A consumer of the schema-table JSON sees `Flagged` fields on Revit 2027.
- The format probes run as `cargo run -p rvt-probes --example <name>`.

### Added

- **A family's types and type parameters, from its PartAtom type table.**
  `PartAtom::family_types()` reads the table a family's `PartAtom` stream
  holds: each type's name, and each type parameter's name, display name,
  kind (built-in, the family's own or shared), parameter type, units and value as
  Revit printed it. `rvt-info` shows it (`family_types` in `--json`). It is
  Revit's text at save, not a reading of the family's own records.

- **Revit 2008 to 2010 projects export their elements, and Revit 2008 to
  2012 projects their storeys, as experimental and outside the 1.0 range
  (RE-260).** Revit 2009 and
  2010 frame their element records as 2011 does, and Revit 2008 four bytes
  shorter; each is read with its release's schema tags. Materials of Revit
  2008 to 2011, whose names no later rule found, are named, and the Levels
  of Revit 2008 to 2012, whose elevations follow a framed `Plane`
  reference, become storeys. On local files a 2009 wall and its 8 doors
  export on two storeys and a 2008 causeway as one generic model. The files
  are closed source and measured locally only, with no Revit export or
  same-model copy to check them against. The 1.0 release covers 2015 to
  2027; these releases stay experimental outside it.

- **Revit 2008 to 2013 files open, and Revit 2011 to 2014 projects export
  their elements, as experimental and outside the 1.0 range (RE-259).** No file written before 2014
  opened: `BasicFileInfo` was read only for a year from 2014 on, and the
  schema grammar expected a GUID list that schemas before 2014 do not
  write. Now every local 2008 to 2013 file opens and reports its release;
  2008 and 2009, which state none, get it from their build date. Revit 2011
  to 2014 frame their element records as 2015 does and are read with each
  release's schema tags. Two layouts differ: a material's shading frame
  holds two more ElementIds on 2011 to 2013, and some 2011 to 2013 value
  blocks hold no type's map. On a local copy of Autodesk's 2013
  `rac_advanced` sample, every ElementId shared with its 2015 copy keeps its
  class and 62,071 of 62,153 fields agree, the rest in the files' own
  bytes; its export has the 2015 copy's building elements, class for class,
  but for a re-divided curtain wall. The pre-2015 files are closed source
  and measured locally only, with no Revit export of them. The 1.0
  release covers 2015 to 2027; these releases stay experimental outside it.

- **Revit 2027 in the family corpus (#1092, RE-264).** Autodesk's 2027
  sample family joins the twelve-release corpus, so PartAtom, previews,
  `BasicFileInfo` and the schema are measured on 2027 as on 2016 to 2026,
  and `streams::partition_for_year(2027)` is `Partitions/70`. Its schema
  lists 325 fields no earlier release lists, each with byte 2 of its type
  header set to 0x02, 0x12 or 0x22; `FieldType::Flagged` decodes them to
  their base type (77 bool, 79 reference containers, 59 ElementId
  references and so on) and keeps the flag. What the flag means for their
  instance bytes is not measured, so readers treat them as unknown, as
  before.
  Breaking for a consumer of the schema-table JSON
  (`docs/schemas/schema-table.schema.json`): a 2027 schema's field can now
  be `Flagged`, where it was `Unknown`.

- **`rvt-gltf --progress`.** The conversion prints its progress to stderr
  in the `progress: <stage> <done>/<total>` lines of `rvt-elements
  --progress`, and `WalkerControl` gains two stages for saved graphics:
  `saved-graphics` (reading them, 0 then 1/1) and `saved-meshes` (one line
  per element meshed). A cancelled token stops the export with
  `Error::Cancelled`. The .glb is byte-identical with and without the flag.

- **A per-release table in `docs/support-matrix.json`.** `releases` lists
  Revit 2015 to 2027, oldest first. For each release it names the row that
  rates its projects' element records (the row `rvt::capability` already
  cites) and the rows measured on its family files (none for 2015 and 2027,
  which the family corpus does not have). Statuses stay in the capabilities.
  `rvt-capabilities --matrix` prints the table, and
  `tests/support_matrix.rs` pins it to the crate and the family corpus.

- **The viewer retries a model its limits stopped, with the large-model limits
  (#1075).** A stream over the reader's resource limits was named in the File
  status panel ("Part of the file not read") but the browser could only point
  at `rvt-ifc --large-model`. The panel now offers "Retry with large-model
  limits", which reads the same file again with the stream and inflate limits
  `rvt-ifc --large-model` uses (streams up to 2 GiB), and says a browser tab
  can use at most 4 GiB of memory, so a very large model can still fail to
  open. It comes from one new wasm binding,
  `openRvtBytesWithDiagnosticsModeAndOpenLimits(bytes, mode, limits)`, where
  `limits` is `{ largeModel?, maxStreamBytes?, maxInflateBytes? }` or `null`
  for the defaults; `openRvtBytesWithDiagnosticsMode` is unchanged. The viewer
  also reads `?maxStreamBytes=` and `?maxInflateBytes=` from its address for
  the first open, which is how the browser test reaches the path on a 20 KiB
  fixture with `?maxStreamBytes=200`.

- **A wall's `Qto_WallBaseQuantities.Height` (#1016, RE-266).** Written as
  the depth of the wall's body where its native record names no roof or
  floor joined to it, no element cutting it and no instance's cutout: Revit's
  `Height` on all 81 such walls of the BIM4LCA office as built, 70 at permit
  and 93 of puzzbobb's R2025 walls, none different and none Revit's export
  leaves out. Every other wall still gets no `Height`, as its body may be
  cut (#1067).

### Changed

- **A Revit 2023 file's partition ArcWalls are scanned once per export, not up
  to four times.** The production walk, the partition recovery and the IFC
  export each ran the same scan over every `Partitions` stream. The file now
  keeps the scan for its release and limits. The exports are unchanged.

- **The crates.io package carries only what builds the crate.** It held
  the whole repository (`cargo package --list` gave 1,252 files: 286
  research probes, 258 RE reports, the tooling and the maintainers'
  backlog), 3.6 MB compressed for 0.10.0 of crates.io's 10 MB limit. It now
  packs 185 files, 1.0 MiB compressed: the source, the support matrix and
  tag-drift table the source embeds, the README and the licence files. CI
  builds the packaged crate on every pull request.

- **`Pset_DoorWindowGlazingType.IsWired` is no longer written.** Every door
  and window of the BIM4LCA office holds 0 for it, which rvt-rs wrote as No
  (494 as built, 490 at permit), and Revit's export writes it on none
  (RE-194 §4). No property value Revit's export omits is written, as for
  quantities: W7 now says so.

- **`Pset_BuildingCommon.NumberOfStoreys` only where it cannot be wrong
  (RE-263, #1097).** Revit's exporter counts the storeys that contain an
  element up to IFC 26.0 and every storey from IFC 26.1, and a `.rvt` does not
  say which will export it. rvt-rs now writes the count only where every
  storey contains an element, so both give the same number; elsewhere the
  property is left out (the maintainer, 2026-10-29).

- **The format probes are a crate of their own, `rvt-probes`.** The 244
  `probe_*.rs` programs behind the RE reports moved from `examples/` to
  `probes/examples/`. They are no longer compiled by every `rvt` build,
  clippy run and MSRV check; CI's clippy job checks them once. Run one with
  `cargo run -p rvt-probes --example probe_re70_wall_join_lists -- <file>`,
  and Measure's `probe` input takes the same names as before. The user
  examples (`basics_*`) and the earlier research tools stay in `examples/`.

- **Doors and windows are drawn from the meshes Revit saved for them, by
  default.** `rvt-ifc` and `rvt-gltf` now draw every door and window whose
  body would be its record's bounding box from its saved mesh, on Revit
  2023 to 2027, where the whole mesh decodes: every vertex within 0.001 ft
  of Revit's export on all 132 of Core Interior's doors and 6 windows and
  all 269 of the BIM4LCA office's windows (Measure run 40932136360). A door
  whose saved graphics do not decode whole keeps its box (221 of the
  office's 269 doors). `--no-saved-meshes` keeps every box; `--saved-meshes`
  still draws every other family from its mesh too.
  `saved_meshes::attach_scoped_with_control` with `SavedMeshScope` does the
  same from Rust; Python's `write_ifc()` and the viewer are unchanged.

- **`Reference` only in the sets a Revit export confirms (RE-239 §7).**
  rvt-rs writes an element's type name as `Reference` in the 34 IFC4
  standard sets where a measured Revit export writes it with the same value,
  and in no other set until one does (the maintainer, 2026-10-29). Shading
  devices, alarms, footings,
  reinforcing bars, transport elements and ramp flights get none for now.

- **The viewer's first screen leads with what it can read.** A support
  grid shows, for each Revit release from 2015 to 2027, whether its
  projects open with typed elements or export experimentally and whether
  its family files have verified metadata and schema or are not measured.
  The grid is drawn from `docs/support-matrix.json`. Below it, the drop
  target sits beside a compact demo list. Core Interior and Einhoven show
  captures of the viewer drawing them, and every other demo shows a
  neutral glyph. While the export buttons are disabled, the header says
  why. File status rows that are waiting show neutral rings instead of
  amber dots. The workflow and the profile notes are collapsed rows, and
  the profile notes move to the right panel once a file is open. The
  panel's own text no longer says that typed elements come only from
  Revit 2024 and 2025.

### Fixed

- **`rvt-ifc-compare` reads each file in its own length unit.** It
  compared storey elevations and the bounding box as the numbers each file
  writes, so rvt-rs's metres against Revit's feet or millimetres differed
  by 3.28 or 1,000 times on every storey. Both sides are now converted to
  metres by the file's own `LENGTHUNIT` (`compare::length_unit_scale_to_metres`),
  and RE1's storeys come out the same as Revit's.

- **Openings of windows whose family cuts a solid are cut as Revit cuts
  them (RE-225, #227).** A window set in a wall whose symbol cuts a solid
  (`m_complexCutGeoms`) and no outline had its opening cut to the wall's
  thickness and its box's height; Revit's export cuts it to the solid,
  deeper across the wall and lower at the bottom (on the BIM4LCA office's
  windows 0.328 ft deeper each side and 0.059 ft lower). The opening is now
  as deep as the solid's extent across the wall and as tall as its height,
  down from the body's top: all 269 of the office's windows as built and 265
  at permit within 0.001 ft of Revit's openings. A door's opening keeps its
  box: on the office the solid of a door's symbol brings 4 doors' openings
  closer to Revit's and none within 0.001 ft. No door or window of Core
  Interior, RE1 or puzzbobb's Revit 2025 models cuts a solid.

- **The owner history meets IFC4's `CorrectChangeAction` rule.** Every
  export wrote `IfcOwnerHistory` with `ChangeAction` `.ADDED.` and no
  `LastModifiedDate`, which IFC4's WHERE rule forbids (IfcOpenShell's
  EXPRESS rule check found it on every reference export). It is now
  `.NOCHANGE.` with no last-modifying user or application, as Revit's own
  export writes it.

- **Elements that do not exist in the export phase are left out (#328,
  RE-184).** A project's `AllProjectPhases` record lists its phases in
  order, and with no phase chosen Revit's export shows the last: an element
  is written when it was created in that phase or before it and not
  demolished in it or before it, as Revit 2025's exports of two models made
  for this show. On Snowdon Towers, whose phases run Legends, Existing, New
  Construction, the six slabs created and demolished in Legends are no
  longer written, and nothing else changes. The diagnostics count what is
  left out as `element_record_not_in_export_phase`. Rooms are not checked:
  Revit's exporter skips a room by its own phase parameter, which no model
  here shows to be its created phase, so a room created in an earlier phase
  is still written. A read of the phases that fails on a Revit 2023 to 2027
  file is a warning in the diagnostics, and the export then writes every
  element.

- **The IfcProject description no longer names an internal decoder.** On
  every file not saved by Revit 2023 it read "Revit 2024 export; ArcWall
  standard decoder skipped: Revit 2024 is outside supported versions
  [2023]", a note about one of rvt-rs's readers that told a user of the IFC
  nothing about the model. It is now "Revit 2024 export" and the family's id
  where there is one.
  `rvt-ifc --placeholder`'s description no longer names internal work
  phases; it says the file holds the project only.

- **The crate's, the wheel's and the API docs' summaries say 2016 to
  2027.** They named 2016 to 2026 as the releases whose metadata,
  previews and schema rvt-rs reads; the 2027 sample family has been
  measured with the rest since #1092, as `docs/status.md` and the support
  matrix say.

- **A model with a stream over a resource limit no longer exports empty
  without a word (#1075).** A partition larger than 256 MiB, a compressed
  member that inflates past 256 MiB, or a partition that inflates past
  1 GiB in total used to be skipped by every reader, so a large client
  model came out with 0 elements, 0 storeys and exit status 0. Each such
  stream is now recorded (`RevitFile::unread_streams`), listed in the
  export diagnostics (`stream_not_read_resource_limit`, with a warning per
  stream), and `rvt-ifc` and `rvt-gltf` write no model and exit with
  status 3 unless `--allow-partial`. `--large-model` (streams up to 2 GiB,
  8 GiB inflated), `--max-stream-bytes` and `--max-inflate-bytes` raise the
  limits. Partition streams and the walker's `Global/Latest` now inflate
  under the file's `OpenLimits` instead of the defaults, so Python's
  `max_inflate_bytes` reaches them; a stream's inflated total is
  `max_inflate_bytes` or 1 GiB, whichever is larger. Python's `write_ifc()`
  and `rvt_to_ifc()` raise `ValueError` while a stream is unread unless
  `allow_partial=True`, and `RevitFile.unread_streams()` lists them. Not
  measured on the reporter's files, which are private.

- **Exports with saved meshes skip a pass over the file that nothing
  used.** After reading doors' and windows' saved meshes (on by default),
  the saved-graphics reader made a second pass over every element's
  graphics to resolve render materials into two fields that no exporter,
  binding or probe read. That pass, and the unused saved-graphics glTF
  writer that was their only reader, are gone. Exports are byte-identical
  on all 32 reference models, and `rvt-ifc` takes 24% to 52% less time on
  those with saved door and window meshes: 3.5 s instead of 5.8 s on Core
  Interior, 17.9 s instead of 24.7 s on the BIM4LCA office as built
  (medians of three rounds on one runner).

- **A file that kept an older release's schema reads as that release
  (#421).** A project saved by Revit 2024 to 2027 can carry the schema, and
  so the element-record marker, of the release it passed through: in a
  field report, 25 of 49 such project files carried the marker of 2022,
  2023, 2024 or 2025, and all 25 read as zero elements and zero Levels,
  because the marker did not match the saving release's. The readers now
  take the release the file's schema belongs to (`schema_release`: a 2024
  to 2027 marker, or a 2015 to 2023 release's class tags) when it is older
  than the saving release (`RevitFile::layout_release`). A schema that
  matches no measured release still reads nothing. Metadata, reports and
  the export's GlobalIds keep the saving release; the diagnostics say when
  a file is read as another release than the one that saved it, and give
  the support status of the release it is read as. On Autodesk's 58 sample
  projects of 2015 to 2027 each schema is its own release's and every
  export is byte-identical (Measure run 40928760469). Not yet measured on
  an affected file. A file whose `Global/ElemTable` fits neither record
  size still reads no element records, and the diagnostics now say so;
  `probe_421_schema_marker` prints the table's fit as counts only.

- **A one-storey project gets its storey.** The Level reader asked for at
  least two Levels before writing any storey, so a project with one Level
  exported every element on no storey: R2025 `01_Walls`' five walls, which
  Revit's export puts on `Level 1`. On Revit 2024 to 2027, one Level is now
  a storey set when the broader Level-id scan finds no other Level, so a
  Level the full recovery misses still keeps the export from guessing.
  Releases before 2024 keep asking for two: Revit 2023's Einhoven model
  reads one Level while its walls stand at two or more base elevations.

### Removed

- **Three research modules nothing used are removed.**
  `rvt::research::class_tag_map` (a 2016 to 2026 tag lookup from
  `docs/data/tag-drift-2016-2026.csv`; `formats::SchemaClasses` reads each
  file's own tags), `compound_framing` (a marker tokenizer for the RE-15
  compound ArcWall study) and `relations` (typed-relation domains its own
  documentation called architecture stubs, wired into nothing). They were
  behind `unstable-research`, outside the semver promise. The CSV stays for
  `examples/tag_drift_svg.rs`.

- **Stable API that nothing in the workspace used (the maintainer,
  2026-10-30).** One breaking change before 1.0 removes the public items
  that no exporter, binary, Python binding, viewer call, test, example or
  fuzz target called. Code that names any of them stops compiling and the
  next release is a minor one; the entries below list each removal and what
  to use instead. Nothing the exporter, the CLIs or the Python package
  write changes.

- **`elements`: one `SimpleDecoder` per class, and 46 unused typed views
  gone.** Fourteen decoder structs repeated `SimpleDecoder` line for line
  (check the schema entry's class name, then walk its fields):
  `LevelDecoder`, `CategoryDecoder`, `SubcategoryDecoder`,
  `ReferencePlaneDecoder`, `WallDecoder`, `WallTypeDecoder`,
  `FloorDecoder`, `FloorTypeDecoder`, `RoofDecoder`, `RoofTypeDecoder`,
  `CeilingDecoder`, `CeilingTypeDecoder`, `DoorDecoder` and
  `WindowDecoder`. Use `SimpleDecoder::new("Wall")` and so on;
  `all_decoders()` registers the same 81 classes, `decoder_for_class`
  finds the same ones, and a wrong schema entry fails with the same
  `SchemaMismatch`. The typed views nothing called are removed with their
  modules (`annotations`, `category`, `ceiling`, `circulation`,
  `curtain_wall`, `drafting`, `family`, `foundation_and_furnishings`,
  `generic`, `grid`, `mep`, `project_organization`, `reference_planes`,
  `reference_points`, `roof`, `structural`): `Category`, `Subcategory`,
  `ReferencePlane`, `Roof`, `RoofType`, `Ceiling`, `CeilingType`, `Column`,
  `Beam`, `Stair`, `StairType`, `Railing`, `RailingType`, `CurtainWall`,
  `CurtainGrid`, `CurtainMullion`, `CurtainPanel`, `Grid`, `GridType`,
  `Tag`, `TextNote`, `Dimension`, `View`, `Sheet`, `Schedule`, `Symbol`,
  `FamilyInstance`, `StructuralFoundation`, `Furnishing`, `Rebar`,
  `GenericModel`, `Mass`, `MepInstance`, `Phase`, `DesignOption`,
  `Workset`, `Revision`, `BasePoint`, `SurveyPoint` and `ProjectPosition`,
  with their enums. `FloorType`, `FloorFunction`, `FillPattern`,
  `LinePattern`, `LineStyle`, `ParameterElement`, `SharedParameter`,
  `ParameterValueSource`, `ParameterBundle`, `effective_value` and
  `merge_effective` go too. To read a class's fields, take them from
  `DecodedElement::fields` as the views did; `Level`, `Wall`, `WallType`,
  `Floor`, `Door`, `Window`, `Zone`, `Material` and `ParameterValue`
  stay, as the exporter and the typed JSON use them.

- **`ifc::from_decoded` keeps the bridge `build_ifc_model` needs.** The
  module lost 48 public items nothing called: the roof, ceiling, column,
  beam, stair, door and window extrusion builders,
  `floor_extrusion_from_boundary` and its quantities,
  `polygon_perimeter_feet`, `storeys_from_levels`, `materials_from_revit`,
  `door_property_set`, `stair_property_set`, `column_property_set`,
  `beam_property_set` and the rest. It keeps
  `build_ifc_model`, `ElementInput`, `BuilderOptions`, `wall_extrusion`,
  `slab_extrusion`, `wall_segment_length_feet`,
  `wall_segment_angle_radians`, `polygon_area_sqft`, `wall_property_set`
  and `window_property_set`. Build an `Extrusion` or a `Storey` directly
  where a removed helper made one. `slab_extrusion` takes the slab's
  thickness in feet, `Option<f64>`, where it took `Option<&FloorType>`.
  `ifc::entity_type_histogram` is removed with them.

- **`ifc::measure` and `ifc::annotation` are removed.** Their distance,
  angle and area helpers and their markup layer had no caller in the
  crate, the binaries, the bindings, the viewer or the tests.

- **The viewer-state helpers behind the unused wasm bindings.**
  `ClippingPlane`, every method of `SectionBox`, `ViewMode` (its
  `default_section_box`, `is_orthographic` and `label`) and
  `CameraState` (`eye`, `orbit`, `pan`, `zoom`, `focus_on` and
  `frame_bbox`), `CategoryFilter`'s `new`, `hide`, `show`, `is_hidden`
  and `apply`, `SceneNode`'s `descendants_count`, `find_by_name` and
  `flatten`, `Schedule::filter_by_ifc_type` and `Schedule::to_csv`, and
  `share::encode_to_fragment`. The data types stay, because `ViewerState`
  carries them and `share::decode_from_fragment`, which the fuzz target
  and the tests call, reads them back. For a CSV schedule use
  `ifc::schedule_csv`; the viewer drives its own camera.

- **`geometry` keeps `Point3`, `Vector3`, `Curve`, `CurveLoop` and
  `recovery`.** `Solid`, `Face`, `BooleanOp`, `Mesh`, `PointCloud`,
  `BoundingBox`, `Transform3`, `Point3::ORIGIN` and the `Vector3` axis
  constants are removed: only the recovery of wall curves, floor loops,
  opening hosts and levels used the module.

- **Eleven wasm bindings the viewer never calls.** `openRvtBytes`,
  `openRvtBytesWithLimits`, `openRvtBytesWithDiagnostics`,
  `openRvtBytesWithDiagnosticsAndLimits`,
  `openRvtBytesWithDiagnosticsModeAndLimits`, `applyCategoryFilter`,
  `encodeToFragment`, `decodeFromFragment`, `cameraEye`,
  `defaultSectionBoxForView` and `defaultSheetOptions`. The 11 that
  remain are the ones `viewer/src` calls, and `docs/viewer-api.md` lists
  them. Call `openRvtBytesWithDiagnosticsMode(bytes, mode)` for a model
  and its diagnostics at an export quality mode, and pass `null` as
  `renderPlanSvg`'s options for the defaults. The walker's scan limits are
  no longer settable from JavaScript.

- **`elem_table::link_partition_element_ids` (`unstable-research`).** A
  wrapper nothing called; use
  `research::partition_scanner::link_elem_table_to_partitions`.

### Research

- **Revit's "Export to IFC" parameter does not explain the single elements
  rvt-rs writes that Revit's export leaves out (RE-262, #792).** Neither
  `IFC_EXPORT_ELEMENT` nor `IFC_EXPORT_ELEMENT_TYPE` is saved, in a parameter set or a family parameter slot, on RE1
  Plumbing's tank 442378 or the BIM4LCA office permit model's column
  1927742, or on any element of those models, RE1 Architecture or Core
  Interior; 24 office instances as built carry it, all with value 1.

- **Two bits of the element record's flag word (RE-261, #223).** Checked
  against Revit's exports, bit 12 of `+0x46` is exactly a room Revit exports
  as a space (302 of 302 rooms on Core Interior, RE1 and the R2025 models)
  and a wall with an end joined (8 of 8 on RE1, the only export that writes
  wall joins). That confirms two rows of puzzbobb's bit table. Bit 7 reads as
  a type record, as RE-128 found; their `WallSweep` row for it is not
  testable on these models, and the rest of the table needs member values or
  one-change saves. Nothing reads the word yet.

- **The storey count Revit's exporter writes (RE-263, #1097).** Up to IFC
  26.0 Revit's exporter writes `Pset_BuildingCommon.NumberOfStoreys` as the
  number of storeys that contain an element, and from IFC 26.1 as the number
  of storeys (24 of 24 and 5 of 5 reference exports). rvt-rs writes the
  number of storeys, IFC4's definition, only where every storey contains
  an element, where the two rules agree.

- **Where Revit cuts a wall's top and base (RE-258, #1067).** On the BIM4LCA
  office, rvt-rs's walls differ from Revit's at one end only: 65 tops and 20
  bases as built. Revit cuts a top at the underside of a floor the wall's
  top join names, which gives 63 of the 65, but that rule also cuts 5
  walls Revit leaves whole, and nothing read tells them apart, so no wall
  is cut yet. `tools/re/wall_heights_vs_ifc.py` measures every wall's base
  and top against Revit's export: Core Interior's 360 and puzzbobb's 117
  R2025 walls all match.

- **What a wall's `Height` is (RE-257, #1016).** Revit's export writes the
  vertical span of its own body: rvt-rs's wall body is that tall on 501 of
  the BIM4LCA office's 579 walls as built and all 117 of puzzbobb's R2025
  walls, but taller on the office's other 78, which Revit draws cut (#1067).
  Neither the wall's saved height (-1001101) nor any single field of its
  record tells those apart; RE-266 narrows it to the walls nothing joins to
  or cuts.

## [0.10.0] — 2026-10-27

0.10.0 writes more of the base quantities Revit's own IFC export writes,
each new one measured against that export with none different.

Walls carry their `Width` however they are drawn, and their `Length` where
their curve, cut back at the walls joined at its ends, is as long as the
curve (RE-252). Straight curtain walls carry their curve's `Length`, and
beams drawn along their location line its length. Floors drawn as one
extrusion carry their `GrossArea` (RE-249), and, where their type has one
layer, their `Depth` and `GrossVolume`. Ceilings carry their layers'
`Width` (RE-250). Hollow-section columns joined to nothing carry their
`CrossSectionArea`, `OuterSurfaceArea` and `GrossVolume` (RE-247). Spaces
whose room record holds its solid's faces carry the room's `Height`
(RE-255). A stair railing whose path is one line along its stair's runs
carries its `Length` (RE-254). On the BIM4LCA office as built that makes
21,065 of the quantities Revit's export writes (20,157 in 0.9.0), one
different (a mullion whose `Span` differs) and none that Revit's export
leaves out.

`rvt-ifc --diagnostics` lists each wall's native curve length and the walls
whose native record names the type rvt-rs gives them, for the quantity
census; the IFC file does not change. `rvt-elements` and Python list a
wall's, floor's, ceiling's or roof's type parameters after its own
(RE-256).

puzzbobb's pull request reads those type parameters (#1065). Thank you.

### Upgrading from 0.9

- No code change is needed: the stable API breaks nothing
  (`DecodedExportDiagnostics` gains two fields; it is `#[non_exhaustive]`).
- An export carries more base quantities; `rvt-ifc --no-base-quantities`
  still leaves them all out.
- `rvt-elements` and `decoded_elements()` list more entries for walls,
  floors, ceilings and roofs, all at level `type`; no entry changes.

### Added

- **A beam's `Length` (#1016).** A beam drawn along its location line
  (RE-49) carries `Qto_BeamBaseQuantities.Length`, that line's length, as
  Revit's export writes it: all 8 of the BIM4LCA office's beams at each
  stage, none different. Its areas and volumes are left out, as the box it
  is drawn as is not its section.

- **Ceilings' `Width` (#1016, RE-250).** A compound ceiling whose body is as
  deep as its type's layers carries `Qto_CoveringBaseQuantities.Width`, that
  thickness, as Revit's export writes it: 59 of the BIM4LCA office's
  ceilings as built and 55 at permit, each Revit's value, none different.
  Its `GrossArea` is left out: its sketch's area is Revit's on most ceilings
  but about 1% larger on 11.

- **A hollow-section column's area, surface and volume (#1016, RE-247).** A
  column joined to nothing and drawn as its type's hollow section carries
  `Qto_ColumnBaseQuantities.CrossSectionArea`, `OuterSurfaceArea` and
  `GrossVolume` from that section, its rounded corners counted as arcs:
  Revit's values on all 79 such columns of the BIM4LCA office as built and
  55 at permit, none different. Its `NetVolume` is left out, as Revit's
  export writes the smaller volume Revit computes, and a column drawn as its
  record's box gets none of the three, as a round column is drawn as one too.

- **Curtain walls' `Length` (#1016).** A straight curtain wall carries
  `Qto_CurtainWallQuantities.Length`, its curve's length, which Revit's
  export writes from the wall's location curve: 213 of the BIM4LCA office's
  220 curtain walls as built and at permit, each Revit's value, none
  different. A curved one gets none.

- **Floors' `Depth` and `GrossVolume` (#1016).** A floor drawn as one
  extrusion whose type has one layer, as deep as the extrusion, carries
  `Qto_SlabBaseQuantities.Depth`, the extrusion's length, and `GrossVolume`,
  its profile's area times that length, as Revit's export writes them: 13 of
  the BIM4LCA office's floors as built and at permit, each Revit's value,
  none different. A layered floor gets neither: Revit's export gives one
  layer's depth there. Its `Perimeter` is left out, as Revit's export writes
  it in feet, unscaled, in a millimetre file; the census counts it apart on
  each slab where its value, read as feet, is Revit's own outline (13 of the
  office's floors at each stage).

- **Floors' `GrossArea` (#1016, RE-249).** A floor drawn as one extrusion
  carries `Qto_SlabBaseQuantities.GrossArea`, the area of the profile it
  extrudes, its outline less its holes, as Revit's export writes it: 31 of
  the BIM4LCA office's slabs as built and at permit, each Revit's value,
  none different. A floor of several loops, drawn as several elements, is
  left out.

- **A space's `Height` (#1016, RE-255).** A space whose room's native record
  holds its stored solid's faces carries `Qto_SpaceBaseQuantities.Height`,
  the room's `RoomElem.m_height`, as Revit's export writes it: 234 of the
  BIM4LCA office's 254 at each stage, every room of puzzbobb's R2025 models
  and all 13 of the held-out Revit 2026 house's, none different. The other 20 office rooms, whose records hold no
  faces, get none: Revit's `Height` is another value on all of them.

- **A stair railing's `Length` (#1016, RE-254).** A railing that belongs to a
  stair and whose path is one line along the stair's runs carries
  `Qto_RailingBaseQuantities.Length`: the line, climbing at the stair's riser
  height over its tread depth for as long as the longest of those runs
  climbs and level for the rest, as Revit's export writes it. 4 of the
  BIM4LCA office's railings at each stage and both railings of each of
  puzzbobb's R2025 stair models 13 to 16, none different. A railing over a
  stair well's landings, on a sloped floor, or level, gets none yet.

- **A wall's, floor's, ceiling's and roof's type parameters in
  `rvt-elements` and Python (#35, RE-256).** Each lists its type's saved
  parameters after its own, as a family instance does: its native record
  names its type (`m_WallAttributesId`, `m_floorAttributesId`,
  `m_ceilingAttributesId`, `m_roofAttributesId`). On Snowdon Towers every
  such type value Revit's export writes is Revit's: the type name on 1,363
  of 1,380 elements (not the 17 stair landings), the assembly code 1,362 of
  1,362, the type mark 889 of 889 and the walls' fire rating 683 of 683.

- **A wall's curve length in the export diagnostics (#1046, RE-252).**
  `rvt-ifc --diagnostics` lists each exported wall's native curve length
  (`decoded.wall_curve_lengths_feet`, by ElementId); the IFC file does not
  change. `tools/re/qtos_vs_ifc.py` compares it with the cut-back curve
  Revit's export draws as the wall's `Axis`, and counts a missing `Length`
  apart where the two differ, since Revit's export writes either: 461 of the
  BIM4LCA office's walls as built and 388 at permit. It does so only on a
  model where Revit's `Length` is one of the two on every wall, so no curve
  was read wrong.

- **Walls' `Length` (#1046, RE-252).** A wall carries
  `Qto_WallBaseQuantities.Length` where its curve, cut back where its body
  meets the walls joined at its ends, is as long as the curve. Revit's
  export writes either length, by how it draws the wall, so only there is
  the value never in doubt: 88 of the BIM4LCA office's walls as built and 81
  at permit, each Revit's value, none different. The cut-back ends are read
  twice, from the wall's native record and from its join reaches, and taken
  only where the two agree.

- **Every wall's `Width` (#1016).** A wall the partition reader does not draw
  from its centreline (a slanted wall, or one whose line its data does not
  place) carries `Qto_WallBaseQuantities.Width` too, its type's layers'
  thickness, which Revit's export writes as the wall's `Width` whatever its
  orientation: now all 607 of the BIM4LCA office's walls as built and all
  589 at permit, each Revit's value, and 31 more door and window openings'
  `Depth` with them.

- **Walls whose saved record states their type, in the export diagnostics
  (#1048, RE-253).** `rvt-ifc --diagnostics` lists the walls whose native
  record names the type the export gives them (`decoded.walls_type_confirmed`).
  `tools/re/qtos_vs_ifc.py` counts a wall's differing `Width` apart, where
  Revit's export gives it another type, only on such a wall, so a wall whose
  type rvt-rs picks wrong stays scored.

### Research

- **What a curtain mullion's `NetSurfaceArea` is (RE-248, #1016).** Revit's
  export sums the faces of the mullion's Revit solid: its section's perimeter
  times its length plus 0, 1 or 2 ends, the ends that are not joined to
  another mullion. Listers and a ranking of the mullion records find nothing
  in the model that says which ends those are, so the value stays unwritten
  on all 10,003 of the BIM4LCA office's mullions; the next signal is the
  curtain wall's grid and its type's join condition.
  The curtain wall type's join condition (`m_autoJoinCond`) was read on
  2026-10-27: 0 on every mullion of the office, so it does not say either.

- **Where a railing's path is, and what its `Length` is (RE-254, #1016).** A
  railing's native record names its path's sketch, whose lines and arcs are
  the path in plan. Revit's export writes the path's length in 3D, along the
  stair or sloped floor the railing stands on: the plan sum is Revit's
  `Length` on 4 of the BIM4LCA office's 41 railings, and nothing read tells
  those apart, so no railing gets one yet.

- **Stair-well railings' `Length` (RE-254, #1016).** On the BIM4LCA office's
  20 stair-well railings Revit's `Length` is the run-count climb of their
  three-run stairs, less the railing's placement offset on the inner
  railings and plus it on the outer ones, to 1e-13 ft, the sign following
  which side of the railing its run lies on against the way it turns. How
  the offset enters is not understood, so none of them gets a `Length` yet.

- **What a space's volume is not (RE-255, #1016).** rvt-rs's own body of a
  space, its stored solid's outline times its depth, is Revit's `NetVolume`
  on 167 of the BIM4LCA office's 278 rooms and every space quantity on the
  R2025 rooms, but nothing in a room's record tells the office rooms it
  holds on, so no space volume or area is written.

- **Whether a wall keeps Revit's wall quantities (RE-251, #1016).** A probe of
  every number in each wall's native record and saved parameters finds none
  of Revit's wall areas or volumes on the BIM4LCA office: Revit computes them
  when it exports. A wall's `Height` is its saved parameter -1001101 on 496
  of 579 walls, but nothing read tells the other 83 apart, so it stays
  unwritten.

- **Which type a wall's records name (RE-253, #1048).** On the held-out
  model where one wall's `Width` differs from Revit's, every reading of the
  saved file (the wall's reference list, its native record, its type's data
  copies, the distance between its record's faces) gives it the 200 mm type
  rvt-rs picks, while Revit's export, made hours after the last save, gives
  it a 300 mm type the file also holds: most likely an unsaved change.
  Count-only probes and a scorer of the export's time against the save's
  run on held-out models.

## [0.9.0] — 2026-10-26

0.9.0 writes more of the property sets and base quantities Revit's own IFC
export writes, and every number below is measured against that export.

Base quantities are new. Curtain mullions' `Length` and, where Revit draws
the mullion as an extrusion of its section, its `NetVolume`; doors' and
windows' `Width`, `Height` and `Area`; walls' `Width` and the `Depth` of the
openings their doors and windows cut; stair flights' `Length` and
`NetVolume`; pipes' cross-section and outer surface; and the `Length` of
columns joined to nothing are written as Revit writes them: 20,157 of the
quantities Revit's export writes on the BIM4LCA office as built, one
different (a mullion whose `Span` differs) and none that Revit's export
leaves out. They are on by default, as in each of Revit's built-in IFC4
export setups, and `rvt-ifc --no-base-quantities` leaves them out (RE-242).

Elements carry more of Revit's properties: curtain mullions' `Span`, walls'
`ExtendToStructure`, railings' `Height`, slabs' `PitchAngle` where their top
is known to be level, stair flights' treads, tread lengths and nosing on
Revit 2024 and 2025, and the `Reference` of stairs, runs, landings, openings
and furniture. Curtain walls, pipes, wall sweeps and ramps take the type
their native record names, and doors and windows are set in the wall their
native record names. A project or shared parameter named for a standard
property (`FireExit`) fills it where IFC4's set applies.

`rvt-ifc --revit-set-placement` writes properties where Revit's exporter
does and IFC4's definitions do not, such as a room's
`Pset_SpaceCommon.FloorCovering` (RE-235); the default output follows
buildingSMART's definitions. A value Revit's export writes that is known to
be wrong (a stair's `WalkingLineOffset`, a curtain mullion's
`CrossSectionArea`, a door opening's swapped `Width` and `Height`) is left
out either way. `--saved-meshes` keeps an element's box where its saved mesh
is only part of its body. Exports of Revit 2023 to 2027 files are faster:
the native records are found through one index per file.

puzzbobb's pull request takes each 2024 to 2027 file's record marker from its
own schema (#1033). Thank you.

### Upgrading from 0.8

- No code change is needed: the stable API breaks nothing (`SavedMeshReport`
  gains a field; it is `#[non_exhaustive]`).
- An export now carries more property values and base quantities. For an
  export without base quantities, as from a Revit setup with "Export base
  quantities" off, pass `rvt-ifc --no-base-quantities`.

### Added

- **Parameters named for a standard property alone.** A project or shared
  parameter named for a standard property set's property alone (`FireExit`)
  now fills that property in each set that applies, where no
  `<set>.<property>` parameter does, as Revit's export reads them: the
  BIM4LCA office's `Pset_DoorCommon.FireExit` on all 266 doors, where none
  was written before (RE-227).

- **Base quantities from a stair flight's, pipe's and column's body (#1016,
  RE-247).** Each stair flight drawn from its sketch carries
  `Qto_StairFlightBaseQuantities.Length`, the span of its riser lines, and
  `NetVolume`; each pipe whose cylinder is read carries its
  `GrossCrossSectionArea` and `OuterSurfaceArea`; and each column joined to
  nothing carries its `Length`, each as Revit's export writes it. On the
  BIM4LCA office the quantities equal to Revit's go from 19,972 to 20,157 as
  built, none different and none that Revit's export leaves out. A joined
  column's `Length` and a curtain panel's `Width` are left out: Revit's
  export gives none on 18 meshed columns and 3 panels that nothing in the
  model tells apart.

- **Curtain walls' types.** A curtain wall type whose serialised data gives no
  name takes the one its native record holds, so each curtain wall is named
  `Curtain Wall:<type>:<ElementId>`, typed, and carries its type's name as its
  `Reference`, as Revit's export does: all 220 curtain walls of the BIM4LCA
  office as built and at building permit, where none was before (RE-230).

- **Doors and windows carry their base quantities (#1016).**
  `Qto_DoorBaseQuantities` and `Qto_WindowBaseQuantities` hold `Width`,
  `Height` and `Area`, the size revit-ifc reads from the door's or window's
  Width and Height parameters. They are written where the symbol that draws
  the element and its type agree on that size: on the BIM4LCA office as
  built that is 196 of 269 doors and all 269 windows, each value as Revit
  writes it, and none different.

- **Openings face-hosted doors and windows cut.** A door or window hosted on
  a face of another element, such as a roof hatch, cuts the opening its
  symbol's solid makes in that element, as Revit's export writes it: on the
  BIM4LCA office, roof 2991742's four 600 mm hatch openings, each at Revit's
  place, through the roof's thickness, with Revit's name and GlobalId
  (RE-225, #824).

- **Curtain mullions carry `Qto_MemberBaseQuantities.Length` (#1016).**
  It is the mullion's `Span` (RE-226), which Revit's export writes as its
  length on all 10,003 of the BIM4LCA office's mullions: 9,984 of them as
  built, and 10,012 of 10,031 at building permit, are now written as Revit
  writes them, the first of the office's 50,580 base quantities after the
  pipes' lengths.

- **Curtain mullions' `NetVolume` (#1016, RE-243).** A mullion whose type
  has its own rectangular or square section, standing on a straight curtain
  wall whose profile is not edited, carries `Qto_MemberBaseQuantities.NetVolume`:
  its section times its length, the volume of the extrusion Revit's export
  draws it as. On the BIM4LCA office that is 7,795 of 10,003 mullions as
  built and 7,823 of 10,031 at building permit, each Revit's value. Mullions
  on curved curtain walls or ones with an edited profile, some of which
  Revit draws as cut meshes, get none.

- **Curtain mullions' `Span`.** Each curtain mullion carries
  `Pset_MemberCommon.Span`, its length along its own axis, as Revit's export
  writes it: Revit's value on 9,984 of the BIM4LCA office's 10,003 mullions as
  built and 10,012 of 10,031 at building permit, where none was written
  before (RE-226).

- **Pipes, wall sweeps and ramps take the type their own record names
  (RE-240, #35).** Where neither RE-130's stored pipe-type layout nor RE-69's
  reference list types one, a pipe's `RbsPipeCurve.m_idType`, a wall sweep's
  `WallSweep.m_typeId` or a ramp's `Ramp.m_rampAttrId` names its type, and the
  type's record its name. On both BIM4LCA office files the 12 pipes, 9 wall
  sweeps and 5 ramps that had neither now carry Revit's Name, ObjectType and
  type object, and 52 more of Revit's property values (their types'
  `Reference`) are written, none different.

- **`rvt-ifc --no-base-quantities` (#1016).** Leaves out every base
  quantity (the `Qto_` sets: lengths, widths, heights and areas), as
  Revit's exporter does when its export setup's "Export base quantities" is
  off. They stay on by default, as in each of Revit's built-in IFC4 setups
  (RE-242). The quantity census counts the quantities rvt-rs writes apart
  where Revit's reference export writes none, instead of as values Revit
  does not write.

- **Openings' `Depth` (#1016, RE-244).** The opening a door or window cuts
  in a wall carries `Qto_OpeningElementBaseQuantities.Depth`, the wall's
  thickness, where the symbol that draws it cuts an outline, as Revit's
  export writes it: 184 of the BIM4LCA office's openings as built and 159 at
  permit, each Revit's value. Their `Width` and `Height`, which Revit's
  export swaps on every door opening (its `Width` is the opening's height),
  are left out.

- **Openings' `Reference`.** An opening of its host's own geometry, a floor's
  sketch void or a hatch's cut in a roof, carries its host's type name as
  `Pset_OpeningElementCommon.Reference`, as Revit's export writes it: 28
  openings on the BIM4LCA office, none different (RE-237).

- **Railings' `Height`.** Each railing's `Pset_RailingCommon` carries its
  `Height`, its type's top rail height, as Revit's export writes it: on all
  41 railings of the BIM4LCA office as built and at building permit, RE1
  Architecture's one and both railings of each R2025 stair model, where none
  was written before (RE-231).

- **`rvt-ifc --revit-set-placement`.** Writes properties where Revit's own
  exporter writes them and IFC4's definitions do not: a text parameter named
  `<set>.<property>` for a property the set's IFC4 definition lacks (a room's
  `Pset_SpaceCommon.FloorCovering`), and the `Reference` of a set Revit
  matches to the element by the start of its entity's name
  (`Pset_SpaceHeaterTypeCommon` on a space). On the BIM4LCA office 961 more of
  Revit's values, none different. The default output still follows
  buildingSMART's IFC4 definitions (RE-235).

- **Slabs' `PitchAngle`.** A slab whose top is known to be level, a stair
  landing or a slab whose body is as deep as its type's layers, carries
  `Pset_SlabCommon.PitchAngle` 0, as Revit's export writes it: 54 of the 62
  slabs Revit's export gives it on the BIM4LCA office, none different; the
  eight sloped ones are left out (RE-238).

- **Revit 2025 stairs' and flights' dimensions.** On Revit 2025 projects each
  stair's `Pset_StairCommon` and each flight's `Pset_StairFlightCommon` carry
  their risers, riser height, tread lengths, number of treads and nosing
  length, from the stair's and run's native records: Revit's values on the
  five R2025 stair models, none different, where none was written before
  (RE-236).

- **Stair flights' treads, tread lengths, nosing and run type.** On Revit
  2024 projects each flight's `Pset_StairFlightCommon` carries its number of
  treads (its risers, one fewer when it begins and ends with a riser),
  `TreadLengthAtOffset` and `TreadLengthAtInnerSide` (the stair's tread
  depth), its run type's nosing length and the run type's name as its
  `Reference`, each landing its landing type's name as its `Reference`, and
  each stair's `Pset_StairCommon` its `TreadLengthAtOffset` and the nosing
  length its runs share. On the BIM4LCA office these are Revit's on 34 or 35
  of the 35 flights rvt-rs writes, 22 of 22 landings and 14 of 14 stairs,
  none different. `WalkingLineOffset` is not written: Revit's export gives it
  as half the run's width in feet read as millimetres.

- **Stairs' `Reference`.** A stair named as Revit names it (RE-65) carries its
  stair type's name as its `Pset_StairCommon.Reference`, as its runs and
  landings carry theirs: all 16 of the BIM4LCA office's as built and at
  building permit and each R2025 stair model's one, where 2 of the office's
  had it before.

- **Walls' `ExtendToStructure`.** Each wall's `Pset_WallCommon` carries
  `ExtendToStructure`, true where its native record joins its top to a roof
  or floor, as Revit's export writes it: Revit's value on all 607 walls of the
  BIM4LCA office as built, 589 at building permit and RE1 Architecture's 7,
  where none was written before (RE-228).

- **Walls carry `Qto_WallBaseQuantities.Width` (#1016).** It is the
  wall type's thickness, the Width parameter revit-ifc reads: 602 of the
  BIM4LCA office's 607 walls as built, and 519 of 589 at building permit,
  each as Revit writes it. A wall's `Length` is not written yet: Revit's is
  its Length parameter, not its location line's length.

### Changed

- **Faster exports of Revit 2023 to 2027 files (#1032).** Each read of the
  native records walked every partition twice; the first one now records
  where every record is, and the later ones decode only the partitions
  holding the records they need. On one runner, the BIM4LCA office exports
  7 to 11% faster, Core Interior 14%, Autodesk's 2024 samples 2 to 8%, with
  every export byte for byte as before.

- **The element-record marker of Revit 2024 to 2027 files comes from each
  file's own schema (#421, puzzbobb, #1033).** Every scan that needs it (element names, stair
  names, type records, Level records) takes the `[Outline][0xFF x4]
  [ElementParents]` marker from the file's `Formats/Latest` (RE-80), and the
  release's measured constant (`bbox_marker`) is the cross-check: a file
  whose schema gives another marker is not scanned, one whose schema does
  not parse keeps the constant. On all 84 2024 to 2027 files measured (5,
  68, 4 and 7) the two agree, and the marker's `ElementParents` tag less 12
  is the schema's `ElementHeader` tag. On 2016 to 2018 that gap is 9, not
  12, so `record_prologue_constant` holds only from 2019
  (`tests/schema_bbox_marker.rs`).

- **One parse of the schema per file (#1032).** A dozen of the partition
  readers an export runs each read and parsed `Formats/Latest` again for a
  few class tags; the file handle now keeps the parsed classes for all of
  them, with every export byte for byte as before.

### Fixed

- **Furniture's `Reference`.** buildingSMART's IFC4 definitions list
  `Reference` in `Pset_FurnitureTypeCommon` with no type, as they do in three
  other sets, and the property-set table left it out. The table generator now
  gives such a property the type most sets give it (`IfcIdentifier`), and
  lists every other property a set defines, so each piece of furniture
  carries its type's name there, as Revit's export does: all 98 of the
  BIM4LCA office's as built and at building permit.

- **Curtain mullions' `NetVolume` in any language (#1016, RE-246).** Which
  mullions have a rectangular section is now read from the class of their
  type's section object rather than from their family's English name, so a
  model made in another language gets the same `NetVolume`: Autodesk's MEP
  sample projects, whose rectangular mullion family is named in German, now
  carry it. Every export of the reference models is unchanged.

- **Doors and windows are set in the wall their native record names
  (RE-245).** Where a door's or window's reference list names several walls,
  the nearest one was taken as its host, which on the BIM4LCA office put 11
  doors in a wall next to Revit's. On Revit 2024 and 2025 the host is now
  the wall the element's current native record names: all 463 of the
  office's doors and windows set in a wall are in Revit's, their 11 openings
  move to within 0.001 ft of Revit's, and 7 more door types carry Revit's
  GlobalId.

- **`--saved-meshes` keeps the box where a saved mesh is partial.** An element
  some face or branch of whose saved graphics does not decode keeps its record
  box instead of a mesh that is only part of its body, and `rvt-ifc` counts
  them. On the BIM4LCA office 357 such meshes, none of them Revit's body, keep
  their box, and 1,294 of the 1,352 bodies drawn from a saved mesh are Revit's,
  against 1,294 of 1,709 before (RE-224).

### Research

- **Which columns Revit's export gives a `Slope` (RE-233, #35).** A probe of
  columns' native records, and `tools/re/record_scalars_vs_ifc.py` now ranks
  record fields against whether Revit writes a property at all. No field
  tells the 73 of the BIM4LCA office's 279 columns Revit writes
  `Pset_ColumnCommon.Slope` on: revit-ifc takes it from the last object of
  each column's exported geometry.

- **Values a project's own property set file places are out of 1.0's
  scope (RE-239, #35).** On the BIM4LCA office, Revit's export writes
  `Pset_DoorWindowGlazingType` on walls, curtain walls and coverings and
  takes rooms' `OccupancyType` from their `RoomTypeCode` parameter, neither of
  which revit-ifc's own definitions do: the export's user-defined property set
  file chose them, and the model does not store that file. A probe and
  `tools/re/mapped_parameters_vs_psets.py` trace each value to the element's
  saved parameter; W7 of the 1.0 criteria now leaves these values out and
  RE-239 lists them per reference model (the maintainer's decision,
  2026-10-26).

- **Stored symbol lengths against Revit's door and window sizes (RE-229,
  #810).** A probe and scorer, `examples/probe_re229_symbol_lengths.rs` and
  `tools/re/symbol_lengths_vs_ifc.py`, compare every length a door's or
  window's symbol stores with the sizes Revit's export writes. The width is a
  family's own parameter on the BIM4LCA office and a built-in one on Core
  Interior and RE1, and no stored length is the width of the office's two
  doors #810 still leaves unsized.

## [0.8.0] — 2026-10-25

0.8.0 draws more of a Revit model as Revit's own IFC export does, and every
number below is measured against that export.

Revit 2025 stair runs are drawn as their treads and risers, read by member
name from their native records. Monolithic runs with upright risers,
riserless runs and spiral runs are drawn as their steps or treads, arcs and
all. A monolithic landing on Revit 2024 is its slab, and a landing that is
one tread is that tread. Each stair and flight carries the `PredefinedType`
Revit gives it, and Revit 2025 stairs carry Revit's names and type objects.

Walls end where Revit's do at T joints, read from each wall's native record,
and a layered wall's core runs through the other wall's finishes to its core
(RE-222, RE-223): on the BIM4LCA office 482 of 607 walls end where Revit's
do, against 412 in 0.7.0. Doors and windows whose symbols cut no outline
take their own extents as their overall size. Square hollow-section columns
are drawn as their section, rounded corners and all (RE-221). Revit 2024
pipes are cylinders, and where a file stores its project's, site's,
building's and storeys' GlobalIds, the export writes them (RE-220).

`rvt-ifc --revit-property-sets` writes each element's saved parameters as
Revit's "Export Revit property sets" does (RE-210). `--anonymize` on
`rvt-ifc --diagnostics` and `rvt-inspect --json` replaces every name the
output takes from the project with a placeholder, so the files can be
posted publicly; the viewer's diagnostics download is anonymized.

puzzbobb's pull request writes Revit's property sets (#934), and their
R2025 stair, roof and opening models measured most of this release. Thank
you.

### Upgrading from 0.7

- **Element ids are an `rvt::ElementId` (#873, #923).** The ids the element
  structs carry (`level_id`, `host_id`, `type_id`, `symbol_id` and the rest)
  are `Option<rvt::ElementId>` where they were `Option<u32>`. `ElementId`
  wraps the `i64` Revit's API uses and serialises as the bare number, so the
  CLIs' JSON and the Python values are the same integers. `ElementId::from`
  takes a `u32`, `get` gives the value. The wire-level views
  (`walker::DecodedElement::id`, `walker::InstanceField::ElementId`,
  `elem_table::ElemRecord`) keep the `u32` the file stores.
- **`IfcModel` gains `type_property_sets` (#934).** Code that builds an
  `IfcModel` with a struct literal needs `type_property_sets:
  Default::default()` or `..Default::default()`.
- **`ifc::RevitGlobalIds` gains `project`, `site` and `building` (#878).**
  Code that builds a `RevitGlobalIds` with a struct literal needs the new
  fields or `..Default::default()`.

The CLIs, the viewer and the Python package need nothing.

### Added

- **`--anonymize` for diagnostics you post publicly.** `rvt-ifc --diagnostics`
  and `rvt-inspect --json` name things the project names: the file and
  project names, the levels, a sample of the materials and of skipped
  elements, and, in `rvt-inspect`'s report, the folder path Revit stored for
  the file, of which only a `\Users\<name>\` part and Autodesk build and
  project-id folders were replaced. With
  `--anonymize` each is replaced with a placeholder (`<redacted>.rvt`,
  `Level 1`, `Material 1`, `element-<ElementId>`), the "Export to IFC As"
  values a warning lists are dropped, and counts, releases and ElementIds
  stay. The browser viewer's Download
  diagnostics now gives the anonymized file, named `rvt-diagnostics.json`.
  `ExportDiagnostics::anonymize` and `redact::anonymize_file_name` do it
  for library callers.

- **A door or window whose symbol cuts no outline carries its overall size
  (#810, #844).** Such a door or window was written with `$`, or with its
  type's Width and Height, which on the BIM4LCA office are 10 mm short of
  Revit's. Its `OverallWidth` and `OverallHeight` are now its own extents:
  along its axis and, where it is set in a wall or a curtain wall, through
  its height, or else along its own Y axis, as Revit's export measures a
  family no wall hosts, such as a roof hatch. On the office all 269 windows
  and 266 of 269 doors are now Revit's (none and 187 before), and so is RE1
  Architecture's curtain-wall door.

- **A stair flight's PredefinedType is STRAIGHT or SPIRAL, as Revit's
  export writes it (#323).** Every flight was `.NOTDEFINED.` or unset. A
  run's treads name its shape in their native records (`m_runType`, 0 for a
  straight run, 2 for a spiral one), and the flight and its type object now
  take it, on Revit 2024 and 2025. On the BIM4LCA office all 35 flights of
  each file have Revit's predefined type, and so do the flights of the 5
  R2025 stair models.

- **A column whose type is a square hollow section is drawn as it (W6,
  RE-221).** Such a column was a solid box. Its section's outside, wall
  thickness and corner radius are now read from its family's own
  parameters, and it is drawn as Revit's export draws it: an
  `IfcArbitraryProfileDefWithVoids` of two `IfcIndexedPolyCurve`s, each four
  lines and four quarter arcs, extruded through its height (`BodySource`
  `family_type_hollow_section`). On the BIM4LCA office all 79 such columns
  as built and 55 at building permit are Revit's body, every vertex within
  0.001 ft. The IFC model gains `ProfileDef::IndexedPolyCurveWithVoids` and
  `IndexedCurve` for it.

- **A monolithic stair landing is drawn as its slab (#824).** Each landing
  was its record box, which spans its stair's storey, 12.5 to 14.2 ft tall.
  On Revit 2024 a monolithic landing is now its slab: the outline the lines
  at its top in its own data make, from its top down its landing type's
  thickness. On the BIM4LCA office all 21 monolithic landings have Revit's
  top and outline, and 60 of its 63 slabs now have Revit's outline, against
  40. Where a flight meets a landing, Revit's underside follows the flight's
  soffit and rvt-rs draws it flat, so the slabs are 100.4% to 104.7% of
  Revit's volume. The body says so: `BodySource` `partition_landing_slab`.

- **A monolithic stair run with upright risers is drawn as its steps, its
  underside ending where Revit's does (#357).** Monolithic runs were drawn
  only with slanted risers, their underside cut flat at their base. With no
  nosing an upright riser runs up its riser line as a slanted one does, so
  such a run is now drawn the same way. The underside is cut at the run's
  extension below its base where it starts at its stair's base, and runs
  on to its first riser where it starts on a landing. The flight of
  puzzbobb's R2025 model 16 has Revit's volume, 14.75 ft3 where its box was
  237, and so do 34 of the 35 flights of the BIM4LCA office, a Revit 2024
  project, where every one was its box.

- **A Revit 2024 pipe is drawn as its cylinder (#869).** Pipe cylinders and
  the ports at their ends were read on Revit 2025 only. On the BIM4LCA
  office, a Revit 2024 project, all 12 pipes on each of its two files are
  now Revit's cylinder, with its radius, axis ends and length, where they
  were boxes, and their 24 ports sit at their ends with Revit's axis. A
  2024 pipe's inner diameter is not read, so it writes no `InvertElevation`
  rather than one from its axis alone. `pipe_bodies_vs_ifc` compares each
  file in its own site frame and reads a profile Revit writes as four arcs.

- **A Revit 2024 family instance's ports are placed too (#869).** Placing a
  port no duct or pipe joins at its connector, through the instance's
  transform, ran on Revit 2025 only, the release the RE1 models measured.
  The BIM4LCA office, a Revit 2024 project with Revit's IFC4 export, is the
  oracle for 2024: all 43 of its such ports are now within 0.01 ft of
  Revit's, with Revit's axis, where none was placed. `port_places_vs_ifc`
  now compares each file in its own site frame, since Revit writes the
  office's site at its shared coordinates; the RE1 scores do not change.

- **`rvt-ifc --revit-property-sets` writes Revit's own property sets (#35,
  RE-210).** With the option, an export writes each element's named saved
  parameters (RE-207) as Revit's exporter does with "Export Revit property
  sets": one set per parameter group, named for the group (`Dimensions`,
  `Identity Data`, `Materials and Finishes`, ..., `Other` for none), and the
  type's parameters in such sets on its type object, or on its elements with
  `(Type)` added where it has none. Following revit-ifc's `Release_25.x.x`
  branch, the IFC group is left out, a Yes/No is a Boolean, an integer with
  a spec an integer, a non-empty text a text, a length, pipe or duct size,
  area, volume, angle or number its measure, an air flow, HVAC or piping
  pressure, apparent or electrical power, electrical potential or current
  its measure in SI units (whose units the export then declares), a
  material its name and a family type `<family> : <type>`. Built-in
  parameters, enumerations and other specs are left out and counted.
  Against Revit 2025's export of Autodesk's Snowdon Towers HVAC with
  "Export Revit property sets" (measured locally, not committed), all
  28,906 properties written (counted per element) are in the set of the
  same name with Revit's value (`tools/re/revit_property_sets_vs_ifc.py`);
  Revit writes 79,907 more, nearly all built-in parameters. On Snowdon
  Architectural, 58,346 of 58,733 are; Revit names the sets of 13 of its
  type objects with `(Type)`, which follows from the order of its export. On the BIM4LCA office
  as built, `FireExit`, written in `Identity Data`, equals Revit's
  `Pset_DoorCommon` and `Pset_WindowCommon.FireExit` on 498 of 498 doors and
  windows. Without the option the export is unchanged.

- **A stair run with treads and no risers is drawn as its separate
  treads (#357).** Such a run was its record box. Each tread is now its own
  solid, from its riser line to the next, its top at its riser's height and
  its type's tread thickness deep. The flight of puzzbobb's R2025 model 15
  is Revit's: 9 treads, 4.45 ft3 where its box was 356, every vertex within
  0.00001 ft. A riserless type with a nosing, shaped by its nosing profile
  family, keeps its box.

- **A spiral stair run with treads and no risers is drawn as its treads
  (#357).** Such a run was its record box. Each tread's native record holds
  the spiral's centre, its two radii and the angles of its riser lines, and
  each tread is now written as Revit writes it: an extrusion of a profile
  with two arcs about the centre and two lines, the nosing carrying it on
  under the tread above, as an `IfcIndexedPolyCurve`. On the BIM4LCA
  office the spiral flight's 18 treads have Revit's volume, 4.540 ft3 where
  the box was 1,307, and its height. A monolithic spiral run keeps its box.

- **Revit 2025 stairs, flights and stringers are named and typed as Revit's
  export names them (#323).** RE-65 read a stair's and its parts' names
  from byte offsets measured on Revit 2024, so on Revit 2025 they had no
  name and no type object. They now come from the native records: the stair
  type's construction (`Assembled Stair`, `Cast-In-Place Stair`), each
  type's name, the run type's monolithic flag and the stair type's support
  style. On puzzbobb's five R2025 stair models every stair, flight and
  stringer has Revit's `Name` and `ObjectType`, and a type object with
  Revit's GlobalId.

- **A stair's PredefinedType is the one Revit's export gives it, from its
  runs and landings (#323).** Every stair was `.NOTDEFINED.` or unset. A
  stair with a spiral run is now `SPIRAL_STAIR`, one straight run
  `STRAIGHT_RUN_STAIR`, two runs and a landing `TWO_STRAIGHT_RUN_STAIR`,
  `QUARTER_TURN_STAIR` or `HALF_TURN_STAIR` as the second turns, and three
  or four runs each square to the one before `DOUBLE_RETURN_STAIR` or
  `THREE_QUARTER_TURN_STAIR`, on Revit 2024 and 2025. On the BIM4LCA office
  all 16 stairs of each file have Revit's predefined type, where 2 did, and
  so do the 5 R2025 stair models; ten more of the office's stair type
  objects take Revit's GlobalId with it.

- **A Revit 2025 stair run is drawn as its treads and risers (#357).**
  Revit 2024's byte anchors for a run's riser height, tread depth and ends
  are not found on Revit 2025, so every 2025 flight was its record box. A
  run's sketch, riser height and ends are now read by member name from its
  native records, with `StairsRun.m_endWithRiser` as puzzbobb found it, and
  its run type from its own element record, as on 2024. On puzzbobb's R2025
  stair models 13 and 14 each flight's volume is Revit's, 4.49 and
  4.43 ft3 where the box was 317, with every vertex of Revit's body on
  rvt-rs's, and the flight carries its type's name. The riserless,
  monolithic and spiral runs keep their box.

- **The project, site, building and storeys take the GlobalIds the file
  stores (#878, RE-220).** rvt-rs generated the project's, site's and
  building's GlobalIds on every file. Revit's export takes them from the
  project information's stored IfcGUID values where it has them, and so
  does rvt-rs now: on BIMcollab's Revit 2015 example and both BIM4LCA office
  files all three are Revit's, where none were. On the 2015 example the five
  storeys take their stored values too, Revit's, where none matched. A file
  that stores none keeps generated ones.

- **A landing that is one tread is drawn as it (#824).** A stair landing
  that is not monolithic, has a tread and takes it from its runs was its
  record box, the height of its storey. It is now its native outline loop,
  arc and all, from its top down its runs' tread thickness, written as an
  `IfcIndexedPolyCurve` extrusion as Revit writes it. The BIM4LCA office's
  landing beside its spiral stair now has Revit's body, volume and every
  vertex.

### Changed

- **Breaking: element ids are an `rvt::ElementId` (#873).** The ids the
  element structs carry (`level_id`, `host_id`, `type_id`, `symbol_id` and
  the rest, 95 fields), `OpeningHostRelationship::host_element_id` and the
  arc-wall record's ids are `Option<rvt::ElementId>` where they were
  `Option<u32>`. `ElementId` wraps the `i64` the Revit API's
  `ElementId.Value` is, and serialises as the bare number, so the JSON the
  CLIs print and the Python values are the same integers. `ElementId::from`
  takes a `u32`, `from_raw` a file's 64-bit slot (0 and values from
  `u32::MAX` up name no element), `get` gives the value and `as_u32` the
  stored form. The wire-level views keep the `u32` the file stores:
  `walker::DecodedElement::id`, `walker::InstanceField::ElementId` and
  `elem_table::ElemRecord`; `DecodedElement::element_id()` gives the typed
  id.

- **`ifc::RevitGlobalIds` gains `project`, `site` and `building` (#878).**
  They carry the GlobalIds a file stores for the project, site and
  building (RE-220). Code that builds a `RevitGlobalIds` with a struct
  literal needs the new fields or `..Default::default()`; they deserialise
  as `None` when absent.

### Fixed

- **An instance whose data copies disagree takes its current copy's
  transform (RE-216, #227).** A family instance can have its element data
  in more than one partition. Where the copies' transforms differed, the
  instance had no transform at all, so its turned box, facing, opening and
  ports fell back to what needs none. It now takes the copy in the partition
  its current record is in, as the native reader picks records (RE-145). On
  Core Interior 8 stacked doors had two copies an inch apart across their
  host wall; the current one places Revit's opening exactly, and all 132
  doors' openings are now within 0.001 ft of Revit's, where 8 were within
  0.26 ft. On the BIM4LCA office at building permit 10 more door types take
  Revit's GlobalId, since their flip comes from the transform. No other
  measured model's export changes.

- **A slab edited after its first save keeps Revit's outline (#824,
  RE-216).** Where a sketch line's ends, a sketch's curve list or the slab's
  own record box differ between partitions, the copy in the partition the
  element's current record is in is taken, where such a sketch was dropped
  and its slab drawn as its box. On the BIM4LCA office at building permit
  two slabs take Revit's outline, holes and all, and with the
  tread landing (#824) 62 of 63 slabs now have it, as on the as-built file.

- **A sketch with a quarter arc keeps its arc (#824, RE-96).** A quarter
  arc's ends are two corners of its sketch line's box, so the box solve
  closed the sketch with the arc's chord, and the arc was lost. Such a
  sketch is now read from its lines' own curves. The BIM4LCA office's roof
  1461758 has Revit's outline and area, its two 9.6 ft corner arcs
  included, where it was 52.5 ft² short. 61 of the office's 63 slab
  outlines now match Revit's (59 at building permit), against 60 and 58.

- **A wall stops where Revit's does at a T joint, and a wall with no
  orientation in its partition data is drawn from its centreline (W6,
  RE-222).** Where another wall meets a wall's end with its middle, the
  wall's native record says so, and the body now stops half that wall's
  thickness short, as Revit's does. A wall whose partition data gives a line
  but no orientation, such as 13 of the BIM4LCA office's 17 exterior walls
  of type US-1, takes its native record's orientation, so it is drawn from
  its centreline and in its layers instead of its record box. On the office
  as built 428 of 607 walls now end where Revit's do (412 before), and 22
  more walls carry Revit's layer set; at building permit 377 of 589 (374)
  and 32 more. Core Interior, RE1 and the R2025 models are unchanged.

- **At a T joint a layered wall's core runs through the other wall's
  finishes, as Revit's does (W6, RE-223).** A wall type's core is read from
  its native compound structure, and where a layered wall meets another
  wall's side its core now stops at that wall's core rather than its face.
  On the BIM4LCA office as built 482 of 607 walls end where Revit's do (428
  before), and 422 of 589 at building permit (377). Core Interior, RE1 and
  the R2025 models are unchanged.

- **`writer::write_with_patches` is documented, with an example CI runs
  (#872).** It had no doc comment, so docs.rs showed it with no description,
  no errors and no example. It now says what it writes and when it refuses,
  and shows a patch written and read back.

### Research

- **Saved display meshes scored by shape (RE-78, W6).** A new scorer,
  `tools/re/mesh_bodies_vs_ifc.py`, compares each body `--saved-meshes`
  draws with Revit's body for the same element, by volume and by how far
  each body's vertices lie from the other's surface. Every vertex is within
  0.001 ft on all 138 of Core Interior's doors and windows, on 1,294 of 1,709
  bodies scored on the BIM4LCA office (all 269 windows) and on 148 of 214 on
  RE1's Revit 2025 models. A mesh that is not exact is part of the body, and
  nothing in the file yet tells it apart, so the option stays opt-in.

## [0.7.0] — 2026-10-24

0.7.0 draws more of a building the way Revit's own export does, and settles
the API that 1.0 will keep. Gable, hip and slope-arrow roofs are drawn as
their planes, Revit 2025 roofs slope (RE-199), and a Revit 2025 roof by
extrusion is its sketch's section (RE-211). Revit 2025's tapered walls are
tapered (RE-213). On Revit 2024 and 2025, a door's or window's overall size
is the outline its symbol cuts in its host where it cuts one, and so is its
opening where its type gives none (RE-215). Ports are placed at a pipe's
ends and at a duct's along a model axis, where they join a placed port and,
on Revit 2025, at their connector through the family instance's transform;
HVAC and piping ports carry Revit's flow direction, name and description
(RE-212). Family instances, mullions, columns and walls carry Revit's
materials, read from their native records (RE-200 to RE-206), and
`rvt-elements` and Python list each element's saved parameters on Revit 2023
to 2027 files (RE-207). `rvt-ifc` honours `SOURCE_DATE_EPOCH`, and CI checks
that every writer gives the same bytes twice.

The crate's API is now 27 stable modules, the ones `RELEASE.md` says 1.0
keeps stable. The research readers moved to `rvt::research`, behind the
`unstable-research` feature. `Error` and the stable modules' output types
are `#[non_exhaustive]`, so a later release can add to them without breaking
your code, and CI's semver checks compare the Rust API, the Python stubs,
the wasm bindings and the JSON schemas with the base of every pull request.
Code that uses the crate has to change (below). The CLIs and the viewer need
nothing, and the Python package changes one exception.

puzzbobb's pull requests draw roofs as their planes (#870), carry Revit's
materials (#886), list each element's saved parameters (#887) and draw
Revit 2025's tapered walls (#935). n3ur0m4n's reads saved graphics faster,
with the same output (#882). Thank you.

### Upgrading from 0.6

- **The research readers need `unstable-research` (#873).** The partition,
  native and element-record readers are under `rvt::research`:
  `rvt::partition_element_records` is now
  `rvt::research::partition_element_records`. The `RevitFile` accessors that
  return their types (`element_names`, `design_options`,
  `second_prologue_ids`, `partition_data_objects`), the linkers
  `link_arcwall_element_ids` and `link_partition_element_ids`, and
  `elements::arc_wall::decode_candidate` and `from_partition_arc_wall` need
  the feature too. `EvidenceTier` is in `rvt::capability`, and
  `relations::RelationDomain::for_edge_kind` is now
  `relations::domain_for_edge_kind`.
- **A `match` needs a wildcard arm.** `Error` and the stable modules' 62
  public enums are `#[non_exhaustive]`. Code outside the crate can no longer
  build the 135 structs that only rvt-rs produces with a struct literal.
- **Errors say what failed (#898, #899).** `Error::Cfb` is now
  `Cfb { op, path, source }`, and its `io::Error` is the error's source. A
  file, buffer or stream over its limit is `Error::TooLarge`. The failures
  that were reported as `BasicFileInfo` or `Cfb` without being about either
  are now `SchemaMismatch`, `NoDecoder`, `MissingClass`, `Malformed` or
  `NotLocated`. In Python, `RevitFile.read_adocument()` raises `ValueError`,
  where it raised `OSError`, for a file whose schema declares no `ADocument`.
- **One decoder type (#894).** The 67 unit decoder types (`mep::DuctDecoder`,
  `generic::MassDecoder`, ...) are replaced by
  `elements::SimpleDecoder::new("<class>")`. `all_decoders()` and
  `decoder_for_class` return the same classes as before.
- **The property set builders return a `Vec` (#827).** `from_decoded`'s
  builders return IFC4's set and an `Rvt...` set, and
  `ElementInput::property_set` is now `property_sets`. `RvtDocExporter`'s
  exports do not use these builders.
- **`ifc::pset_validate` is removed (#831).**
- **The element walk fails closed (#852).** `walker::iter_elements` no
  longer yields a `Global/Latest` row that has no ElementId, and it skips a
  typed schema-path row its decoder rejects.
- **Exports change.** An export names 0.7.0 in its IFC header and in its
  `IfcApplication`. The geometry, materials, ports and properties below are
  new, so the same file exported by 0.6 and by 0.7.0 gives different IFCs.

### Added

- **A Revit 2025 roof by extrusion has its shape (RE-211, #883).** Its
  sketch is a B-spline in a vertical plane, now read: its degree, weighted
  control points and knots. That curve is the roof's top. The type's
  thickness, square to it, gives the underside, both ends are cut plumb, and
  the section runs across the record box, which it must reproduce. On
  `11_RoofByExtrusion` in puzzbobb's Revit 2025 models every vertex of
  Revit's body is within 0.001 ft of rvt-rs's (0.000124 ft at most), and the
  volume within 0.03%, where it was the 7,665 ft³ record box. That makes 19
  of the 19 roofs in those models. A roof by extrusion on another release,
  or one whose sketch holds an arc, keeps its box. `roof_bodies_vs_ifc` also
  measures Revit's vertices against rvt-rs's surface.

- **A door's or window's opening is the outline its symbol cuts (RE-215,
  #227).** On Revit 2024 and 2025, where its type gives no opening, a door
  or window cuts its host along the outline the symbol that draws it cuts,
  placed by the instance's transform (RE-87) and through the wall. 124 of
  Core Interior's 132 doors, its 6 windows and RE1 Architecture's 4 doors
  now cut Revit's opening within 0.001 ft; they were up to 0.26 ft off.
  Core Interior's other 8 doors have no transform read and keep their box.

- **`rvt-elements` and Python's `decoded_elements()` list each element's
  saved parameters (#35, RE-207).** On Revit 2023 to 2027 files, an element's
  `parameters` now also hold the values its native record saves: its typed
  parameter sets, the family parameters of its own `FamilyParams`, and the
  native fields read as built-in parameters, each named by the file's own
  definition (family, shared, project and global parameters) with its
  storage type, spec, group and value. A family instance's type parameters
  follow its own, marked `"level": "type"`; a family's instance parameter is
  taken only from the instance. Lengths, areas, volumes and angles also get
  their SI value, a Yes/No value is a boolean, and a value naming a material
  or a family type also gets that element's name. Built-in parameters keep
  their negative id and no name. A family type parameter (`ParamDefFamType`,
  a window's Muntin Pattern or trim) is now read as the ElementId it holds.
  On the BIM4LCA office (Revit 2024), every parameter named `<set>.<property>`
  holds the value Revit's export writes for that property on the element:
  7,606 of 7,606 as built and 7,592 of 7,592 at building permit; on
  puzzbobb's R2025 models a window's `Pset_WindowCommon.FireRating` is its
  type's `Fire Rating`, 18 of 18. `tools/re/element_parameters_vs_ifc.py`
  scores either output against Revit's export. Nothing new is written to IFC.
  The library gives the same entries through
  `elements::parameters::saved_parameters_lossy`. By puzzbobb (#887).

- **On Revit 2025, every family-instance port is placed, joined or not
  (#869).** A port no duct or pipe joins now stands at its connector: the
  point and axis its connector entry gives in the symbol that draws the
  instance (RE-215), through the instance's transform (RE-87). On Revit 2025
  all 53 such ports of RE1 Electrical, 29 of Mechanical and 18 of Plumbing
  are within 0.01 ft of Revit's, each with Revit's axis; they had no
  placement. The type's symbol is not enough: fittings sized per instance
  have symbols of their own, and it put 11 of those ports up to 0.81 ft off.
  Ports joined to a duct or pipe stay at their partner's point (#922).

- **The viewer's whole journey is tested on the six redistributable
  reference models.** A browser test opens Einhoven, Core Interior and the
  four RE1 models, inspects an element, downloads the schedule, the IFC, the
  GLB, the plan SVG and the diagnostics, and checks them against each other:
  the inspected element is in the schedule and in the IFC under its
  ElementId, the schedule lists exactly the elements the diagnostics count by
  type, and on Einhoven and Core Interior the export has the counts
  `tests/project_count_fixtures.rs` pins for the CLI's export of the same
  file. Deploy viewer fetches the RE1 models for it. The Einhoven demo card
  gave 24 walls and 42 materials; it now gives the 26 walls, 9 curtain
  panels, 2 windows and 36 materials the viewer shows.

- **A door's and window's OverallWidth and OverallHeight are the outline the
  symbol that draws it cuts in its host (RE-215, #844).** A family instance
  names its type and, in its `InstanceInfo`, the symbol that draws it. Where
  the instance's own parameters change the family's geometry, that is a
  symbol of its own. Revit's exporter sizes a door or window from that
  symbol, and the extents of the outline it cuts are Revit's `OverallWidth`
  and `OverallHeight` on every door and window measured that has one: Core
  Interior's 132 doors and 6 windows, RE1 Architecture's 4 doors, and each
  R2025 model's door and 3 windows. Core Interior's windows, whose type is
  1010 mm square, had `$`; they now have Revit's 1500 by 2700 mm. On the
  BIM4LCA office, the 187 of its 269 doors that cut an outline have Revit's
  sizes; its 269 windows and other 82 doors are drawn by symbols that cut
  none and keep what they had (#810). Revit 2024 and 2025 only, and loops of
  straight lines only. RE1's curtain-wall door has no outline and keeps `$`.

- **`rvt-ifc` honours `SOURCE_DATE_EPOCH`, and CI checks every writer is
  deterministic (#873).** The time in an IFC's header and owner history was
  the export's wall-clock time, so two exports of the same file differed.
  With `SOURCE_DATE_EPOCH` set (Unix seconds, the reproducible-builds
  convention) `rvt-ifc` records that time instead, and refuses a value that
  is not a number. A CLI test runs `rvt-ifc`, `rvt-gltf`, `rvt-sheet` and
  both `rvt-schedule` schedules twice on the same file and requires the same
  bytes, on the tier-1 synthetic project and on Einhoven where the project
  corpus is present. The `rvt-ifc` help no longer calls the export
  document-level only.

- **Every port has Revit's flow direction, name and description (RE-212,
  #834).** A family instance's HVAC or piping connector takes the Flow
  Direction its family sets in the connector's own parameters, or, where the
  family leaves it bidirectional, the direction Revit calculated for the
  instance's connector. A duct's or pipe's connector takes its calculated
  direction. A bidirectional connector joined to one with a direction takes
  the opposite one. On RE1 Plumbing, Mechanical and Electrical that is every
  one of Revit's 463 ports, where rvt-rs wrote `NOTDEFINED` on all 408 HVAC
  and piping ports before. Ports are now named `InPort_`, `OutPort_` or
  `Port_` by their direction and described `Flow`, as Revit names them, and
  a sink placed at a pipe's end points into it. `SystemType` is still not
  written.

- **Each GitHub release carries the CLI archives' build provenance as a
  file (#891).** The release workflow already attested the archives; the
  attestation bundle is now also attached as `rvt-rs-vX.Y.Z.intoto.jsonl`,
  so it can be checked offline and OpenSSF Scorecard's Signed-Releases check
  finds it. `gh attestation verify <archive> -R DrunkOnJava/rvt-rs` works as
  before.

- **Three examples show the library end to end (#872).**
  `cargo run --example basics_inspect -- model.rvt` reads what a file is,
  `basics_elements` lists its building elements by IFC type with their
  ElementIds,
  and `basics_export` writes IFC4, glTF, a CSV schedule and the export
  diagnostics. Each runs on the synthetic project in `corpus/tier1/` without
  an argument, and CI runs all three on it and on Einhoven, a real
  Revit 2023 project (MIT), where they must list and export elements. `examples/README.md` tells them apart
  from the research probes, and the README links them.

- **Gable, hip and slope-arrow roofs are drawn as their planes, and Revit
  2025 roofs slope (#356, RE-199).** A footprint roof with more than one
  edge that defines a slope is drawn as its edges' planes, each rising from
  its edge at its angle and raised by the edge's plate offset, and each held
  to its edge's wedge, so hips and the valley of an L-shaped hip come out as
  Revit draws them; the eave is set by the shallowest edge. A roof sloped by
  a slope arrow rises along the arrow, set by "Height at Tail" or by
  "Slope". A Revit 2025 roof's rafter cut (plumb, two-cut plumb or two-cut
  square) and fascia depth are read, and a two-cut is cut level at its eave.
  On Revit 2025 the edge slope block, each sketch line's segment and a
  Sketch's curve list are now read: those readers refused 2025, so every
  2025 roof was a level block and edited sketches lost their outline. As for
  sheds since RE-56, a body is drawn only where it reproduces the roof's
  record box height, and a roof stays a level block otherwise. On puzzbobb's
  Revit 2025 models (`R2025_Models.zip`, Measure `-f r2025=true`, run
  39602093604), 18 of the 19 roofs Revit's exports hold now have Revit's
  body, every vertex within 0.001 ft of Revit's and the volume within 1e-4
  of it, where main had none; the 19th is a roof by extrusion. Measure
  scores every roof body against Revit's export with
  `tools/re/roof_bodies_vs_ifc.py`. On Autodesk's sample projects, which
  have no Revit export, one roof of rac_advanced is drawn along its slope
  arrow in both its 2024 and 2025 saves, and the 2025 readers draw 29 more
  elements of rac_advanced 2025, 4 of rac_basic 2025 and 1 of rst_basic 2025
  from their sketches rather than as their boxes. Every other 2024 sample,
  the reference models and the BIM4LCA office export as before; on one
  runner, rac_advanced 2025 exports in 1.08 times main's time and rac_basic
  2025 in 1.06. Not drawn: roofs by extrusion (#883), hip roofs over a T or
  U footprint (#884), and a square two-cut with a fascia; a Revit 2024 roof
  with several defining edges takes the same path, but none is measured
  (#885). By puzzbobb (#870).

- **Every output says when a file's release is read experimentally (#871).**
  The export diagnostics carry `input.release_support`: the
  `docs/support-matrix.json` row that rates the release's element records,
  and that row's status (`verified`, `partial`, `experimental`, or
  `unsupported` where none are read). An experimental release (Revit 2015 to
  2022, 2026 and 2027) adds a warning. `rvt-inspect` reports the same in
  `file.release_support`, prints it beside the version, and classifies an
  experimental release's export as `experimental_release`. The viewer's file
  status shows "Experimental Revit release". Before, nothing but `rvt-doc`
  told a Revit 2021 export apart from a Revit 2024 one.

- **A pipe's ports are placed (#847).** Each `IfcDistributionPort` of a
  pipe is placed at the end of the pipe its connector entry keys name: the
  pipe itself with that connector's index, or the element and connector it
  is joined to (RE-131, RE-138). Its Z axis points out of the pipe, or into
  it for a `SINK` port, as Revit's export writes them. On the RE1 Plumbing
  model all 124 of the ports Revit writes on pipes are within 0.01 ft of
  Revit's, and with RE-212's flow directions each has Revit's axis. A duct
  along one of the model's axes has its ports placed the same way, at the
  centres of its box's faces across its length: on RE1 Mechanical all 62 of
  the ports Revit writes on ducts and pipes are within 0.01 ft of Revit's,
  each with Revit's axis. A port joined to a placed duct or pipe port, a
  family instance's included, is placed at that port's point with its axis,
  as Revit's export places every port joined to a segment's (#869): all 118
  such ports of RE1 Plumbing and all 59 of RE1 Mechanical are within
  0.01 ft of Revit's, where none was placed before. Their Z axes, taken from
  the segment port's, are Revit's on 108 and 53 and opposite on the other
  10 and 6, where Revit's export turns the axis from the segment port's. On
  Revit 2025 every other family-instance port is placed at its connector
  too (#869), and the no-geometry modes write none.

- **Every public type implements `Debug`, as the Rust API Guidelines ask (C-DEBUG).** The 14 element
  decoders, the three exporters, `corpus::Sample` and `RevitFile` did not.
  `RevitFile`'s shows the limits it was opened with and how many streams it
  has inflated, not the file's bytes. The crate now warns on a public type
  without one (`missing_debug_implementations`), which CI's `-D warnings`
  makes an error.

### Changed

- **Changelog entries are fragments under `changelog.d/` (#938).** Each
  pull request writes its entry as `changelog.d/<section>/<name>.md`, and a
  release step (`tools/release/changelog.py assemble`) moves them into this
  file. Every pull request used to add its entry at the top of the same
  section here, so each one that landed left the others in conflict with
  `main`. CI checks that every fragment is in a section and starts an item.

- **The user pages say what rvt-rs does now (#907).** `docs/README.md`
  indexes `docs/` as user pages, contributor pages and the research record.
  The user pages lost their backlog ids and planning labels, and the claims
  those labels sat on are corrected. `validation-evidence.md` described an
  `ifc-smoke` job that does not exist and unit tests that no longer run; it
  now describes the IfcOpenShell job's checks, the cross-witness verdict and
  Measure. `compatibility.md`, `demos.md` and `benchmarks.md` said
  per-element geometry was not implemented; they now say where it is exact
  and where a body is a bounding box. The API doc check also reads every
  user page the index lists.

- **The Python guide's examples run in CI (#872).** The three runnable
  blocks of `docs/python.md` run in order against the synthetic project file
  `corpus/tier1/architectural-2024`, through the built wheel
  (`tests/python/test_docs_python.py`). The 18 blocks that show a signature
  or the shape of a result are fenced `text`, so the page no longer presents
  them as code to run.

- **Saved graphics are read faster, with the same output (#881, #882).**
  An export with `--saved-meshes` caches pointer lookups, skips a
  parameter pass the saved scene never used, and meshes elements in
  parallel (one after the other in the browser viewer). On one runner, the
  six reference models export in 0.64 to 0.94 of the time, and the IFC is
  byte-identical (Measure run 39956807932). Thanks to n3ur0m4n. CI now
  exports Einhoven and Core Interior with their saved meshes and validates
  both with IfcOpenShell (#908): 11 of 11 and 138 of 138 bounding-box
  bodies come from saved graphics, with 0 validation errors.

- **The API's entry points have examples that run (#872).** The crate
  root's three examples and new ones on `RevitFile::open`,
  `metadata::read_metadata`, `walker::iter_elements`,
  `RvtDocExporter::export_with_diagnostics`, `write_step`,
  `gltf::model_to_glb` and `schedule_csv::elements_csv` open the synthetic
  project file the repository ships, `corpus/tier1/architectural-2024`,
  and CI now runs every documentation example (`cargo test --doc`), which
  it never did. Those entry points document their errors (`# Errors`), and
  `summarize_strict`'s doc no longer says a missing PartAtom is a failure:
  it is not. The crate root's example used the deprecated `summarize`.

- **Breaking: `Error` is `#[non_exhaustive]`, and says what failed.** A
  `match` on it now needs a wildcard arm, so later variants are not breaking
  changes. Failures that were reported as `Error::BasicFileInfo` or
  `Error::Cfb` although they had nothing to do with either now have their
  own variants: `SchemaMismatch { decoder, schema }` for a typed decoder
  given another class's schema (was "...Decoder received wrong schema"),
  `NoDecoder` for a class with no typed decoder, `MissingClass` for a
  schema with no `ADocument`, and `Malformed { what, detail }` for an
  ArcWall or opening-index record (a wrong class tag among them), an
  `ADocument` or a `Global/ElemTable` that does not have the shape its
  reader expects. Every variant is
  documented, and `NotLocated` is new for a record that cannot be located
  with confidence, as `read_adocument_strict` reports an earlier release's
  `ADocument`. The other variants and their messages are unchanged. In
  Python, `RevitFile.read_adocument()` raises `ValueError`, where it raised
  `OSError`, for a file whose schema declares no `ADocument`.

- **Breaking: the stable API's output types are `#[non_exhaustive]`.** All
  62 public enums of the stable modules and the 135 public structs that
  only rvt-rs produces, so a release can add a variant or a field without
  breaking code that uses them. A `match` on one of these enums needs a
  wildcard arm, and these structs can no longer be built with a struct
  literal outside the crate. The structs callers build stay as they were:
  the options and limits (`OpenLimits`, `WalkerLimits`, `CsvOptions`,
  `StepOptions`, ...), the bridge's inputs (`ElementInput`,
  `BuilderOptions`, `Storey`, `MaterialInfo`, `StreamPatch`,
  `ExportContentPolicy`, `corpus::Sample`, `parse_mode::Warning`),
  `IfcModel` and the parts it is assembled from (materials, layer sets and
  element layers, opening cuts, GlobalIds), the glTF document records
  `write_glb` writes, the geometry primitives (`Point3`, `CurveLoop`,
  `Mesh`, ...), the viewer's state (`CameraState`, `ClippingPlane`,
  `SectionBox`, `ViewerState`, `CategoryFilter`), the schedule rows
  `Schedule::from_rows` takes, what the encoders write
  (`BasicFileInfo`, `PartAtom`), and the few output types the tests
  construct (`DecodedElement`, `Wall`, `ClassEntry`, ...).

- **The API docs are written for someone outside the project.** Backlog
  and task ids (`B55`, `VW1-13`, `SEC-12`) and work-plan labels ("Lane C",
  "Phase 4d", "the attack plan") are gone from the API docs: the crate's
  and the CLIs' rustdoc, the Python docstrings and type stubs, and the
  Python guide; where one was a citation it is kept in a `// Tracker:`
  comment beside the item. The Python docs no longer say only `scaffold`
  export succeeds until geometry recovery lands: that is true of a file
  with no recovered elements, such as the synthetic fixtures. The crate
  root says what `RE-NN` and `#NNN` refer to, and CI checks that every RE
  finding the docs cite has a report and that no planning label comes
  back. The `ifc` module's overview, which still described per-element
  export as pending, says what the exporter writes today, and
  `ifc::from_decoded` no longer claims to attach no geometry, materials or
  property sets.

- **Breaking: `Error::Cfb` keeps the I/O error it reports (#899).** It was a
  string. It is now `Cfb { op, path, source }`: what was being done, the
  stream or file, and the `io::Error`, which `Error::source()` returns, so a
  caller can tell a permission error from a full disk. A file, buffer or
  stream over its limit (`OpenLimits`, the identity-stream cap) is the new
  `Error::TooLarge { what, size, limit }`, a destination path without a
  file name is `Error::Io` with `InvalidInput`, and a stream too short for
  its framing is `Error::Malformed`. All three were `Cfb` before.

- **Breaking: the research readers moved to `rvt::research`, behind the
  `unstable-research` feature (#873).** The crate's API is now 27 stable
  modules (reading and identity, typed elements and the element walk,
  export, and the writer), which `RELEASE.md` names under "What 1.0 keeps
  stable". The other 67 modules, the partition, native and element-record
  readers behind the format findings and the Phase 1 contract stubs, change
  whenever a finding does and are outside that promise. To keep using one,
  enable the feature and add `research::` to its path:
  `rvt::partition_element_records` is now
  `rvt::research::partition_element_records`. The `RevitFile` accessors that
  return research types (`element_names`, `design_options`,
  `second_prologue_ids`, `partition_data_objects`), the ElemTable linkers
  `link_arcwall_element_ids` and `link_partition_element_ids`, and
  `elements::arc_wall::decode_candidate` and `from_partition_arc_wall` need
  the feature too. `EvidenceTier` and the relation-domain registry the
  capability manifest reports are now in `rvt::capability`, and
  `relations::RelationDomain::for_edge_kind` is
  `relations::domain_for_edge_kind`. `rvt::cli` is hidden from the docs.
  Nothing changes for the CLIs, the Python package or the viewer, and the
  crate builds the same code either way: the feature only makes the research
  modules public. docs.rs shows them, marked as needing the feature.

- **Breaking: one `elements::SimpleDecoder` replaces the 67 decoder types
  the element modules generated.** The classes the schema walk decodes
  field by field (`mep::DuctDecoder`, `generic::MassDecoder`,
  `annotations::TextNoteDecoder` and 64 more) were each a unit struct
  stamped out by one of 15 identical `simple_decoder!` macros. They are now
  registered as `SimpleDecoder::new("<class>")`, which decodes exactly what
  the type of that name did; `elements::all_decoders()` and
  `decoder_for_class` return the same classes in the same order. A wrong
  schema's error is `Error::SchemaMismatch`, which reads `<class> decoder
  received the schema of <schema>`.
  `decoder_for_class` returns a `&'static dyn ElementDecoder` from a
  registry built once, so a lookup no longer builds all 81 decoders. The
  `cargo generate` element template registers through `SimpleDecoder`
  and no longer generates unit tests.

- **What 1.0 requires is written down.** `docs/release-1.0-criteria.md`
  lists each requirement with its bar, the value measured now, and a status
  of passed, failed or not measured, linked to its evidence. 1.0 supports
  every Revit release from 2015 to 2027 and ships with no limitations.
  `RELEASE.md` does not tag it while any requirement has not passed. It
  replaces the milestone's completion percentage as the release decision.

- **`from_decoded`'s property set builders write a standard set's name
  only for the properties IFC4 declares in it (#827).** `wall_property_set`
  put Revit's base and top offsets, unconnected height, structural usage
  and location line in `Pset_WallCommon`, and its door, window, column,
  beam and stair siblings did the same, though IFC reserves the `Pset_`
  prefix and each standard set's properties for buildingSMART's
  definitions. Each builder now returns a `Vec<PropertySet>`: IFC4's set
  with what it declares (`Pset_StairCommon.NumberOfRiser`, `RiserHeight`
  and `TreadLength`, `Pset_BeamCommon.Span`, `Pset_ColumnCommon.LoadBearing`,
  with the measure types IFC4 gives them), and a set of rvt-rs's naming
  (`RvtWall`, `RvtDoor`, `RvtWindow`, `RvtStair`, `RvtBeam`, `RvtColumn`)
  with the rest. `ElementInput::property_set` became `property_sets`; the
  first set is the element's own and each further one is written as an
  `IfcEntity::ElementPropertySet`. `floor_base_quantities` no longer writes
  a `Depth` and `GrossVolume` from a thickness of 1 ft where the floor type
  gives none, and the diagnostic exporter's `Pset_RvtRsDiagnosticCandidate`
  is now `RvtRsDiagnosticCandidate`. `RvtDocExporter`'s exports do not use
  these builders and do not change. The builders and `ElementInput` are
  public, so code that calls them needs the new types.

- **The crate's and the wheel's descriptions name the experimental releases.**
  crates.io and PyPI show 0.6.0's and 0.6.1's as reading typed elements from
  Revit 2023 to 2025 projects with 2026 experimental; 2015 to 2022 and 2027 are
  experimental too since 0.6.0 (RE-179 to RE-181).

### Fixed

- **Family instances, mullions, columns and walls carry Revit's materials (#355).**
  Read from the native records instead of byte offsets and scans
  (RE-201 to RE-206):
  - a mullion or curtain panel takes its type's `m_renderStyleId`;
    RE-166's fixed offset moved on 10 of Snowdon Towers' 13 mullion types
    (RE-201);
  - an off-centre column takes the layer of the wall it was first joined
    to on its side, or the exterior layer where it stands out past both
    faces; rendered content (RPC, planting) is `<Unnamed>` (RE-202,
    RE-205);
  - a type's materials are its `m_geomTag2MaterialId` read by name, not
    a nested component's; a material's name is its own `Material.m_name`
    (RE-203);
  - `<Unnamed>` only where the family's own document shows geometry with
    no material, and a material only hidden forms have, nested families'
    included, is left out (RE-204, RE-205, RE-206);
  - a layer that takes its category's material takes the one the
    category's graphics styles name where the object styles are unset or
    not read, as on every Revit 2025 file (RE-206).
  On the R2025 models every wall now has Revit's "Default Wall", 117 in
  21 models, none before. RE1 Architecture 71 of 71 and RE1 Electrical 49
  of 49 elements have Revit's set (70 and 47 before). Locally on Snowdon
  Towers 5,134 of the 5,134 elements rvt-rs gives materials have Revit's
  set (3,689 after RE-200). Core Interior exports as before. The material
  names and category styles are memoised, and Snowdon's export takes
  23 s against 20 s before. By puzzbobb (#886).

- **A family instance takes the materials its own parameters name (#355).**
  Where a family type's geometry-material map holds no material, #580
  gave its columns and products `<Unnamed>`. Where the instance's own
  material parameters (`FamilyInstance.m_pInstParams`) name materials,
  Revit's export gives it those instead, and now so does rvt-rs (RE-200).
  On the R2025 models 12 to 17, the 9 window trims and muntins of each now
  have Revit's material set: all 13 elements rvt-rs gives materials have
  Revit's set, 4 before.
  Locally on Snowdon Towers, 232 more elements have Revit's set and none
  fewer. Core Interior, the RE1 models and the other R2025 models export
  as before. By puzzbobb (#886).

- **Revit 2025's tapered walls are drawn tapered (RE-213, #358).** RE-86
  read a wall type's face angles only on Revit 2024. Revit 2025 frames them
  with twelve zero bytes after the frame's `u32 2`, where 2024 has eight, so
  on a 2025 file every tapered wall fell back to its record box, up to
  9.3 ft wider than the wall. On Snowdon Towers saved by Revit 2025, 19 of the
  20 walls Revit tapers are now within 0.001 ft of Revit's body across the
  wall at five heights (none within 0.1 ft before). 24 walls are closer to
  Revit's in the GLB and none is further, and the 20 windows in them take
  RE-89's cut. Every other 2025 file measured exports the same bytes.

- **The viewer's downloaded diagnostics keep their per-type counts, and its
  Shapes row appears.** The bindings hand a Rust map over as a JavaScript
  `Map`, which `JSON.stringify` writes as `{}`. So `exported.by_ifc_type`,
  `body_sources` and the other maps were empty in the diagnostics the viewer
  showed and saved. The status panel's Shapes row, which says how many
  bodies are bounding boxes (#409), summed an empty list and never appeared.
  The viewer now turns the diagnostics' maps into objects when it receives
  them, and `docs/viewer-api.md` no longer says results are plain JSON.

- **A column record holding a NaN no longer panics the export.** The
  column-cut grid sorted its edges with `partial_cmp(..).expect("finite")`,
  so a NaN read from a damaged record box stopped the whole export. A
  column or cutter with a non-finite coordinate now gets no cut box, and the
  grid sorts with `f64::total_cmp`. 40 other `expect` and `unwrap` calls on file
  bytes and fixed arrays are gone: those reads go through `first_chunk` or
  the crate's checked little-endian reader instead.

- **The PyPI page's links work, and it is written for Python users (#905).**
  PyPI showed the repository README, whose links are relative and resolve
  under `pypi.org/project/rvt/`, so none reached its document. The page is
  now `rvt-py/README.md`: install, a short example, what to expect, and
  absolute links to the Python guide, the support table and the changelog.
  That file said PyPI shipped 0.1.2 and the crate was not on crates.io; it
  names no version now. The package's Documentation link opens the Python
  guide. CI reads the built wheel's description, checks that it is that
  file with absolute links only, and runs its example.

- **A mass whose "Export to IFC As" names an entity is written as it (#864).**
  rvt-rs read no mass instances. The BIM4LCA office's two `VOLUME` masses
  carry `IfcSpace`, and Revit's export writes them as `IfcSpace`. They now
  are, as the 42 family instances of #763 are, and both office files' 308
  `IfcSpace` pair with Revit's by GlobalId. A mass with no such value is
  still not written, as Revit's exporter does not write the mass category
  by default.

- **A family instance whose "Export to IFC As" is `IfcSpace` is an
  `IfcSpace` (#763).** The BIM4LCA office's 42 `Turning Free Space`
  instances carry `IfcSpace` with `USERDEFINED` as their own predefined
  type, and Revit's export writes them as `IfcSpace .USERDEFINED.`, typed by
  an `IfcSpaceType` named after the family type, with `Pset_SpaceCommon`'s
  `Reference` that type's name. rvt-rs wrote them as proxies contained in
  the room around them. They are now spaces, each typed by an
  `IfcSpaceType` and holding no furniture of its own. A room, area or
  space keeps its own mapping. One difference from Revit's export
  remains: Revit's places these spaces in no spatial structure, which
  IFC4 rule `IfcSpatialStructureElement.WR41` does not allow, so rvt-rs
  aggregates each under its storey, as it does rooms.

- **A ceiling of separate loops is one `IfcCovering` (#823).** An element
  whose sketch has separate loops was written as one entity per piece,
  named `<name>:2`, `<name>:3` and so on (#331), as Revit's export writes
  floors. Revit's export writes a ceiling as one `IfcCovering` whose body
  holds every piece, as on the BIM4LCA office's compound ceilings (Measure
  run 39240535083). A ceiling's pieces are now further items of its
  `Body` representation, one `IfcExtrudedAreaSolid` each. Floors keep
  their pieces. The pieces stay in the `IfcModel` (`IfcModel::body_pieces`
  names them), so the glTF export and the viewer still draw them.

- **The schema path's typed decoders fail closed (#852).** A `Global/Latest`
  candidate of the seven `MVP_TYPED_CLASSES` became a `schema_typed` row,
  at confidence 0.9, whatever its bytes held: each decoder rejected only a
  wrong schema name. It is now skipped when a declared field did not read
  as its schema type or its class's typed view reads nothing from it. The
  typed path also takes the caller's `WalkerLimits`, through a new
  `ElementDecoder::decode_with_limits` (its default decodes the bytes cut
  to the cap), which every registered decoder implements by passing the
  limits to `decode_instance_with_limits`, and
  `elements::decode_typed_with_limits`. On the 76 files
  Measure reads, no candidate of the seven classes reaches the production
  score, so no export changes (Measure run 39364060825).

- **`walker::iter_elements` yields no `Global/Latest` row without an
  ElementId (#852).** Its documentation said a candidate with no ElementId
  is yielded only by the diagnostic path, but the production threshold
  yielded one too, undeduplicated and joined to nothing. It is now left
  out unless the caller asks for a score below
  `PRODUCTION_ELEMENT_MIN_SCORE`. On the 76 files Measure reads, every
  production row comes from a partition record and the schema path yields
  none, so no export changes (Measure run 39367864281).

### Removed

- **`ifc::pset_validate` and `docs/ifc/pset-mapping-examples.yaml` (#831).**
  The module classified property set names against a hand-written list of
  seven sets, so it called every other IFC4 set rvt-rs writes, such as
  `Pset_SpaceCommon` and `Pset_EnvironmentalImpactIndicators`, unknown, and
  both still said typed doors were unsupported (RE-19), though doors are
  typed from their element records' `BuiltInCategory` and exported as
  `IfcDoor`. Nothing in the crate, its binaries, the Python bindings or the
  viewer called it. The exporter takes the IFC4 sets it writes from
  `ifc::standard_property_sets` (#821), and Measure checks every export's
  standard sets against buildingSMART's definitions (#826). The module was
  public, so code that used it stops compiling, and the next release is a
  minor one.

### Research

- **Revit 2015's element GlobalIds are not in the file (RE-208, #878).**
  On BIMcollab's 2015 example, every element GlobalId in Revit's export has
  an episode prefix of its own (241 in 241, where Core Interior's 854 fall
  in 5), so RE-48's history rule cannot rebuild them. A stored IfcGUID
  parameter holds the last 21 characters of 229 of the 241; their first
  character, the top two bits, differs every time, and nothing tested in the
  file sets it. The project, site, building and five storeys are stored
  exactly, which is the lead for the storeys' GlobalIds.

- **Revit 2015 is scored against Revit 2015's own export (RE-198, #421).**
  BIMcollab's example structural model, a Revit 2015 project, comes with
  Revit 2015's IFC2X3 export of it. Measure fetches both by hash with
  `-f bimcollab=true` and never stores them. It is the first 2015 to 2022
  file scored against an export of its own release, and it was measured
  before any fix, so it is held out. All 241 of the export's elements come
  out under Revit's ElementId, in Revit's class or its IFC4 equivalent, with
  wall faces and layer sets as Revit's. Three air terminals Revit's export leaves out are written, 16
  footings sit on no storey (#879), and none of Revit's GlobalIds is rebuilt
  on 2015 (#878). 2015 stays experimental.

## [0.6.1] — 2026-10-19

A patch release of 0.6.0. On a 32-bit target, such as the WebAssembly the
browser viewer runs, an export could panic when a size read from a
partition wrapped around (#836); the viewer, built from `main`, has had the
fix since it landed there. crates.io and PyPI show the README packed into
each version, and 0.6.0's support table said more than was measured: it
marked opening Revit 2027, 2015 and 2014-and-earlier projects, with their
previews and schema, as supported, and said every element's `IsExternal` is
written. 0.6.1's says what is measured. 0.6.1 is built from 0.6.0, not from
`main`, which has changes for the next minor release.

### Upgrading from 0.6.0

Nothing to change. An export names 0.6.1 in its IFC header's `FILE_NAME`
and in its `IfcApplication`. Otherwise, on a 64-bit target, Measure's scores
and diagnostics of the reference models, Autodesk's sample projects and the
family corpus are the same as 0.6.0's.

### Fixed

- **The one-pass data-object scan checks the end it computes (#836).**
  `partition_room_parameters::data_objects` tries a header at every offset
  of a partition and added the size it reads there to the offset unchecked.
  On a 32-bit target a size near 4 GiB from arbitrary bytes wrapped the end
  around, and a wrapped end below 4 made its next read panic, which in the
  browser viewer aborts the export. The i686 job found it on every family
  release. A 64-bit build cannot overflow there, so its output is unchanged.
  The job then found `partition_type_materials`' map reader multiplying a
  count read from arbitrary bytes by 12 before checking it against its
  64-entry limit; the count is now checked first.
- **The README's support table says what is measured.** 0.6.0's marked
  opening Revit 2027 and 2015 projects, with their previews and schema, as
  supported: they open on Autodesk's sample projects, where previews and
  PartAtom are not measured. It marked Revit 2014 and earlier projects as
  supported too: no documented 2014 project is measured, and a public 2014
  family opens and its schema reads (RE-147). It said every element's
  `IsExternal` is written (RE-193): curtain mullions and panels get none,
  as the 0.6.0 notes below now say too. The rows for 2015 to 2022 and 2027
  say that no Revit export of those files was measured, rather than that
  there is none to measure against, and the summary of what rvt-rs reads
  names the releases their elements are measured against: Revit 2019, 2023
  and 2026 copies of the same sample projects.

### Changed

- **CI runs the real-file, CLI, contract and hardening checks on 32-bit
  i686 with overflow checks on (#836).** A sum or product of lengths read
  from a file that cannot overflow a 64-bit `usize` can still wrap on a
  32-bit target and pass a bounds check it should fail: #835 found one on
  wasm32, where the browser viewer runs, and every other CI job builds for
  64 bits. In the new job, which `CI passed` requires, such a wrap panics in
  the fuzz regressions and the family-corpus checks instead.

## [0.6.0] — 2026-10-19

rvt-rs writes IFC4, and 0.6.0 makes its property sets IFC4's. 0.5 took some
of them from Revit's exports of the RE1 models, which are IFC2X3; every
standard property set rvt-rs now writes matches buildingSMART's published
IFC4 property set definitions on all 89 exports Measure checks, by rvt-rs's
own check of them (#822, #826). buildingSMART's Validation Service has not
been run on them, and quantity sets are not checked. More of what
Revit's exporter writes is written: parameters named for any IFC4 property
set's property (RE-194, #817), `IsExternal` on every class but curtain
mullions and panels (RE-193), a type's Fire Rating, a door's overall size,
Revit 2024 Areas as spaces (RE-191), and each port's type and, where
Revit's exporter gives it one, its flow direction (RE-197). Projects saved
by Revit 2015 to 2022 and by Revit 2027 export their elements, as
experimental (RE-179 to RE-181).

puzzbobb's pull requests read Revit 2015 to 2022 and 2027 projects (#740,
#742) and found where a wall's function and roof attachment come from
(RE-185); Erry91's gives each door its overall size (#788); MadsHolten's
fixes a panic in the browser viewer (#835). Thank you.

### Upgrading from 0.5

- **Property sets moved to where IFC4 has them (#822).** An element's
  `Reference` is in `Pset_EnvironmentalImpactIndicators` and each IFC4 set
  that applies to it and declares `Reference`, no longer in
  `Pset_QuantityTakeOff` or `Pset_DistributionFlowElementCommon`. Storeys and
  spaces lose `Pset_AirSideSystemInformation` and `Pset_ProductRequirements`.
  A pipe's or duct's `Length` is in `Qto_PipeSegmentBaseQuantities` or
  `Qto_DuctSegmentBaseQuantities`, an `IfcElementQuantity`, and a pipe's
  `InvertElevation` in `Pset_PipeSegmentOccurrence`. A room's Floor Finish is
  in `Pset_SpaceCoveringRequirements`, a proxy's Serial Number in
  `Pset_ManufacturerOccurrence` only, and a pipe fitting's nominal diameter
  in `RvtElementRecordGeometry.NominalDiameter`, one length.
- **Public API.** `ifc::entities::PropertySet::name_sets` is removed and
  `storey_sets` takes no name; a `PropertySet` named `Qto_` is written as an
  `IfcElementQuantity`; `ifc::export_content::TypedProductionAppend` gains
  `computed_inverts`; `ifc::entities::IfcEntity::Port` gains
  `predefined_type` and `flow_direction`; and
  `native_connectors::symbol_port_indices` returns `(index, domain)` pairs.
- **Older and newer releases export elements.** A Revit 2015 to 2022 or 2027
  project, which exported no building element, now exports them, as
  experimental: measured against the same models saved by a neighbouring
  release, not against a Revit export of them.

### Added

- **A port is typed by its connector's domain, and an electrical port is
  `SOURCEANDSINK` (#828, RE-197).** Revit's IFC4 export types each port by
  its connector's domain, `DUCT`, `CABLE` or `PIPE`, and writes every
  connector but an HVAC or piping one as bidirectional (revit-ifc
  `ConnectorExporter`). rvt-rs wrote no `PredefinedType` and `NOTDEFINED` for
  every direction. On the BIM4LCA office all 67 ports now have Revit's
  `PredefinedType` (none before) and 43 its `FlowDirection`; all 53 of RE1
  Electrical's ports are `SOURCEANDSINK`, as in Revit's export, and no port
  takes a direction Revit's export does not give it. The direction rule is
  measured on electrical connectors only; for others outside HVAC and piping
  it rests on revit-ifc's rule. An electrical connector whose description
  sets `Flow=Source` or `Flow=Sink` is exported with that direction by Revit;
  rvt-rs does not read the description (#834), so it writes such a port
  `SOURCEANDSINK`. An HVAC or piping connector's direction, a
  duct's or pipe's included, is the family's and is not read, so it stays
  `NOTDEFINED` (the office's 24 pipe ends are `SOURCEANDSINK` in Revit's
  export); `SystemType` is not written. `ifc::entities::IfcEntity::Port` gains
  `predefined_type` and `flow_direction`, which the writer writes only as a
  value of IFC4's enumeration, and `tools/re/ports_vs_ifc.py` compares the
  three attributes on the ports both exports hold.
- **A type's Fire Rating fills an element's IFC4 `FireRating` (RE-77,
  RE-194).** Revit's exporter reads a parameter named for the property
  (`Pset_WallCommon.FireRating`) first and the type's Fire Rating after it.
  rvt-rs read the type's Fire Rating (RE-77) but wrote it only in its own
  set; it now also goes in each IFC4 standard set declaring `FireRating`
  that applies to the element, wherever no such parameter supplies one. On
  the BIM4LCA office `Pset_WallCommon.FireRating` goes from 71 to 97 of
  Revit's 97 values (26 walls carry their type's), on RE1 Architecture from
  0 to 7 of 7; none differs, and where a parameter and the type disagree
  (4 elements) Revit's export has the parameter's, as rvt-rs does.
- **A parameter named for any IFC4 property set's property is written (#817).**
  The sets RE-194 writes such parameters in are now IFC4's 278 property sets
  that apply to an entity, generated by `tools/ci/gen_standard_property_sets.py`
  from buildingSMART's own list of IFC4's definitions (the one its validation
  service checks rule PSE001 against) and IFC4's entity hierarchy, instead of
  21 sets listed by hand; a set scoped to a predefined type applies only with
  it, and `IfcInteger` properties are read too. CI checks the table is the
  generator's output. No measured model has parameters for the new sets, so
  no export changes.
- **Parameters named for a standard property set's property are written in
  that set (RE-194).** Revit's exporter reads a project or shared parameter
  named `<set>.<property>`, such as `Pset_WallCommon.FireRating` or
  `Pset_WindowCommon.ThermalTransmittance`, before anything else for that
  property. rvt-rs now reads each such parameter of an IFC4 set's property
  from the element and writes it there, where IFC4 applies the set to the
  entity written. Booleans, labels and text, counts, lengths and thermal
  transmittance are read; a thermal transmittance is written as
  `IfcThermalTransmittanceMeasure`, with its unit declared. The parameter's
  value replaces rvt-rs's own. On the BIM4LCA office, as built, property
  values equal to Revit's go from 14,531 to 18,374. The 7 stair riser values
  that differed are now Revit's, and 55 `OccupancyNumber` values differ only
  in type: IFC4's `IfcCountMeasure` against the export's `IfcInteger`. What
  that export takes from its user-defined property set file, such as a
  glazing set on walls, is not written.
- **`IsExternal` as Revit's exporter sets it, on every class but curtain
  mullions and panels (RE-193).**
  Revit's export reads a Yes/No parameter named for the property set first
  (`Pset_WallCommon.IsExternal` on a wall), then calculates the value from a
  parameter named `IsExternal`, the type's Function, a roof's category, and
  otherwise writes not external. rvt-rs now does the same for walls, curtain
  walls, doors, windows, slabs, roofs, columns, beams, coverings, railings,
  ramps, stairs, proxies and spaces, with a wall's Function from its type's
  native record (RE-185). On the BIM4LCA office, as built, the `IsExternal`
  values equal to Revit's go from 266 to 2,110 and those that differ from 29
  to none; on the reference and R2025 models from 13 to 245, none differing.
  Curtain mullions and panels get no value, nor do windows whose type's
  Function is not read: Revit's export does not follow their curtain wall or
  host wall.
- **Revit 2024 Areas export as `IfcSpace` (#763, RE-191).** An Area's
  partition record carries no bounding-box marker, so the element-record
  scan never read one. A placed Area now exports as `IfcSpace .SPACE.` under
  Revit's GlobalId: its number as `Name`, its name as `LongName`, in its
  Level's storey, typed by an `IfcSpaceType` of its own and with
  `Pset_SpaceCommon.Reference`, as Revit's export writes it. It has no body:
  the Area's boundary is not decoded. On the BIM4LCA office, as built and at
  building permit, rvt-rs now writes all 30 of the area spaces in Revit's
  export, with number, name and GlobalId as Revit's. The permit file's two
  Areas that are not placed are left out, as Revit leaves them out. Revit
  2024 only.
- **Revit 2015 to 2018 projects export their elements, as experimental
  (RE-181).** Revit 2015 to 2018 frame their element records as 2019 does,
  so the 32-bit readers read them with each release's own schema tags. Three
  layouts differ: a Level record's placement kind, a wall's flip, which
  follows its location line with no word as on 2019 and 2020, and a
  material's shading frame, which holds two pattern slots where 2019 holds
  four. A type's element-data name is looked for in its own data only, past
  any one-unit entry. There is no Revit export of a 2015 to 2018 file to
  measure against; on Autodesk's 2016 to 2018 `rac_basic`, `rst_basic` and
  `rac_advanced` every ElementId shared with the 2019 copy has its class,
  2018 `rac_basic` and `rac_advanced` agree with it on every field, and the
  exports have the 2019 copies' building elements, class for class, and pass
  IfcOpenShell's schema validation. Autodesk's six 2015 sample projects
  (#801) frame their records the same way and carry the 2015 tags; on five
  of them every ElementId shared with the 2019 copy has its class, and the
  sixth, `rst_basic`, is another model. Before, a 2015 to 2018 project exported
  no building element. Beyond RE-180's, it changes no 2019 to 2027 export.
  Revit 2014 is not read: no documented 2014 file backs it. By puzzbobb
  (#742).
- **Revit 2019 to 2022 projects export their elements, as experimental
  (RE-180).** Revit 2019 to 2022 frame their element records as 2023 does,
  so the 2023 readers read them, each constant taken as the release's own tag
  of the same schema class: element data, materials, the category material
  frame and layers. Revit 2019 and 2020 write a wall's flip with no word
  before it, and in 32-bit data a one-id list reads as a one-unit name, which
  is no longer taken. There is no Revit export of a 2019 to 2022 file to
  measure against; on Autodesk's 2019 to 2022 `rac_basic`, `rst_basic` and
  `rac_advanced` every ElementId shared with the 2023 copy has its class. The
  `rac_basic` and `rst_basic` exports, and the 2021 and 2022 `rac_advanced`
  ones, have 2023's building elements, class for class (the 2019 and 2020
  `rac_advanced` are an earlier edit of the model), and every export passes
  IfcOpenShell's schema validation. Before, a 2019 to 2022 project exported
  no building element. Every 2024 to 2027 export is byte-identical. Of the
  2023 samples only `rac_advanced` changes: the one-unit name rule gives its
  export 2 more entities and 2 more property values. By puzzbobb (#742).
- **Revit 2027 projects export their elements, as experimental (RE-179).**
  The partition readers take their 2027 constants from the file's schema, as
  RE-124 found every release's to be: the record marker, Level elevations,
  element data, layers and materials. Layer records, wall join lists and
  mullion type materials keep their 2026 layouts. Revit 2027 writes most
  element frames without an ElementId at `+0x00`, type symbols among them, so
  from 2027 a symbol frame takes its enclosing record's id as an instance
  already does (RE-35). There is no Revit 2027 IFC export to measure against;
  on Autodesk's five 2027 sample projects the elements equal those of the
  same models saved by Revit 2026 (20,818 of 20,818 ElementIds, every class
  the same, 363,072 of 363,256 fields, measured by Measure's paired probe,
  #799), and the six 2027 exports pass
  IfcOpenShell's schema validation. Before, a 2027 project exported no
  building element. Every 2023 to 2026 export is byte-identical. By puzzbobb
  (#740).
- **A door's `OverallWidth` / `OverallHeight` from its type (#759, RE-187).**
  A door type's Width and Height are Revit's `OverallWidth` and
  `OverallHeight` on every door where they are read, so a door with no
  opening read now writes them instead of `$`: all 132 of Core Interior's
  doors and 4 of RE1 Architecture's 5, each equal to Revit's (none before).
  A door cut at its rough size keeps that size (RE-94). Windows keep failing
  closed: Core Interior's type size is not Revit's. Checked by
  `tests/door_overall_sizes.rs` and scored by
  `tools/re/door_window_sizes_vs_ifc.py`. By Erry91 (#788).

### Changed

- **`ifc::entities::PropertySet::name_sets` is removed, and `storey_sets`
  takes no name (#822).** The two sets `name_sets` built are IFC2X3's, and a
  storey's sets no longer include them.
- **Measure checks each export's standard property sets against IFC4's
  definitions (#822).** `tools/ci/ifc4_property_sets.py` counts, for every
  `Pset_` set an export relates to an object or a type, what buildingSMART's
  IFC4 definitions (the list its validation service checks rule PSE001
  against, pinned by #817's generator) do not allow: sets IFC4 does not
  define, properties a set does not have, sets on entities IFC4 does not
  apply them to, and values of another kind or type. Measure appends it to
  every export's `schema.txt`. On the 89 measured exports of `main` before
  #822's fixes it found IFC2X3's `Pset_QuantityTakeOff`,
  `Pset_DistributionFlowElementCommon`, `Pset_ProductRequirements` and
  flow-segment sets, `Pset_AirSideSystemInformation` on storeys, and IFC2X3's
  `NominalDiameter`, `Length` and `SerialNumber` in three IFC4 sets that do
  not have them, and no value of another type.
- **CI and Measure run IfcOpenShell 0.9.0.post1 (#773).** Five of Measure's
  scorers let go of an IFC file while they still used its elements, which
  IfcOpenShell 0.9 does not allow (`room_names_vs_ifc` crashed under 0.9.0);
  they now keep it open. The geometry scorers also print why IfcOpenShell
  cannot mesh an element, which they used to count without a reason. On
  main's commit Measure's scores are byte-identical under 0.8.5 and
  0.9.0.post1 (runs 39153682389 and 39153684706; only the export timings
  differ), walls included, which 0.9.0 could not mesh. So the pin moves, and
  Dependabot proposes IfcOpenShell's minor releases again.
- **CI installs Rust with rustup (#765).** Every workflow's toolchain step
  now runs `rustup toolchain install` itself, with the components and targets
  its job needs, instead of the third-party `dtolnay/rust-toolchain` action.
  That action's `stable` and `nightly` branches are force-pushed, so its pins
  went stale and zizmor flagged them, and GitHub's runners already ship
  rustup.

### Fixed

- **`rvt-inspect` and `rvt-corpus doctor` take Revit 2015 and 2027 files
  (#838).** Both reported such a file as an unsupported version before
  looking at its export, though rvt-rs now reads its element records,
  experimentally (RE-179, RE-181), as it reads 2019 to 2022, which both
  already took. Their range is now 2015 to 2027.
- **The browser viewer no longer panics on a string field whose length is
  corrupt (#835).** On wasm32, where `usize` is 32 bits, a UTF-16 string
  field's end, `4 + 2 × length`, could wrap past its bounds check when the
  length was corrupt, and slicing then panicked, ending the whole export; a
  64-bit build cannot wrap there. The end is computed with checked
  arithmetic, and such a field is read as raw bytes, as one too short already
  is. Found and fixed by MadsHolten, on a private Revit 2024 project.
- **A pipe's or duct's length and a pipe's invert are where IFC4 puts them
  (#822).** RE-131, RE-134 and RE-157 wrote them in IFC2X3's
  `Pset_FlowSegmentPipeSegment` and `Pset_FlowSegmentDuctSegment` (and a
  duct's length also in `Pset_DuctSegmentTypeCommon`), as Revit's IFC2X3
  exports of the RE1 models do. IFC4 has neither flow-segment set, and its
  `Pset_DuctSegmentTypeCommon` has no `Length`. The length is now in the
  segment's base quantities, `Qto_PipeSegmentBaseQuantities` or
  `Qto_DuctSegmentBaseQuantities`, as an `IfcElementQuantity`, and a pipe's
  invert in `Pset_PipeSegmentOccurrence`, where Revit's IFC4 export of the
  BIM4LCA office writes them, on an element written as a pipe or duct segment
  only, not one its Export to IFC As writes as another entity. A pipe's own
  parameter named `Pset_PipeSegmentOccurrence.InvertElevation` is written as
  it is, with no storey elevation taken off. A `PropertySet` named `Qto_` is
  now written as an `IfcElementQuantity` of IFC4 simple quantities. A
  building-element proxy's Serial Number is no longer also written in
  `Pset_PrecastConcreteElementGeneral`, which in IFC4 has no `SerialNumber`;
  `Pset_ManufacturerOccurrence` holds it.
- **A pipe fitting's nominal diameter is no longer written in
  `Pset_PipeFittingTypeCommon` (#822).** RE-165 wrote it there as Revit's
  IFC2X3 exports of the RE1 models do, a list of one length. IFC4's
  `Pset_PipeFittingTypeCommon` has no `NominalDiameter`, and Revit's IFC4
  Reference View export writes none of the port sets that hold one: revit-ifc
  certifies none of them for that view, and its export of the BIM4LCA office
  writes no property set on any of its 67 ports. The measured value is now in
  rvt-rs's own `RvtElementRecordGeometry` set, as one length.
- **An element's `Reference` is in IFC4's property sets, not IFC2X3's
  (#822).** RE-154 wrote the type's name in `Pset_QuantityTakeOff` on every
  element with a type and in `Pset_DistributionFlowElementCommon` on MEP
  elements, as Revit's exports of the RE1 models do. Those exports are
  IFC2X3, and IFC4 has neither set. The name now goes in every IFC4 set,
  from #817's table, that applies to the element's entity and declares
  `Reference`: `Pset_EnvironmentalImpactIndicators` on every element, the
  entity's common set, the reinforcement sets of walls, slabs, columns and
  beams, and a distribution element's own type set
  (`Pset_SanitaryTerminalTypeCommon` and so on), as Revit's IFC4 export of
  the BIM4LCA office writes them. Columns, windows, beams and shading
  devices had no `Reference` before, and MEP elements had it only in the
  IFC2X3 set. On the office as built, values equal to Revit's go from 18,502
  to 32,053 (from 18,501 to 32,225 at building permit) and values Revit does
  not write from 13,257 to 621; every new value Revit's export holds is
  equal. Against Revit's IFC2X3 exports of the RE1 models every `Reference`
  value is still Revit's, each in the IFC4 set that holds it.
- **Storeys and spaces carry no IFC2X3 name sets (#822).**
  `Pset_AirSideSystemInformation` and `Pset_ProductRequirements` holding a
  storey's or space's name, as Revit's IFC2X3 export of the RE1 models writes
  them, are no longer written: IFC4 has no `Pset_ProductRequirements` and
  applies `Pset_AirSideSystemInformation`, the information of the air-side
  system serving a space, to spaces and zones only, and Revit's IFC4 export
  of the office writes neither.
- **Each piece of an element written in pieces carries the element's
  property sets (#816).** An element whose sketch has separate loops is one
  entity per piece (#331), and every set it gets after its body went on the
  first piece only, among them `Reference`, `IsExternal`, `Serial Number`,
  the level and stair sets and the parameters named for a standard set.
  Revit's export gives each piece all of them, as its export of Snowdon
  Towers' architectural model shows on its 19 elements in pieces.
- **A room's Floor Finish is in `Pset_SpaceCoveringRequirements`, where IFC4
  puts a space's coverings.** RE-153 wrote it as `Pset_SpaceCommon.FloorCovering`,
  following Revit's export of RE1 Architecture, which is IFC2X3 (where that
  set holds the coverings), not IFC4 as RE-153 recorded. IFC4's
  `Pset_SpaceCommon` has no covering properties, so the IFC4 file rvt-rs
  writes carried one its standard set does not define. `psets_vs_ifc`
  reads an IFC2X3 reference's space coverings under their IFC4 set.
- **A room with no number is named, typed and referred to as Revit's export
  does it (RE-196).** A room's number and name were read only as two
  non-empty texts, so a room with no number got neither: its space was
  `Room-<ElementId>` with no `LongName`, no `IfcSpaceType` and no
  `Reference`. It is now `<name>:<ElementId>` with its name as `LongName`,
  typed and referred to by the same name, as Revit's exporter falls back
  to. An empty number or name also counts as the latest copy's value (#800):
  at building permit the BIM4LCA office's rooms had kept older copies'
  numbers and Finnish names where the current copy has none. The office's
  rooms all have Revit's number and name now, as built and at permit, 264
  of 264 (221 before), and the text parameters read the same way, so an
  emptied Floor Finish or Serial Number writes nothing.
- **A stair's `IsExternal` joins its `Pset_StairCommon` (RE-194).** RE-193
  wrote it in a second set of that name, beside the riser and tread
  dimensions RE-47 writes, where Revit's export writes one set. On Autodesk's
  2024 sample projects 34 stairs now carry one set fewer, with the same
  values.
- **An element's "Export to IFC As" of `IfcStair` or `IfcDoor` is honoured
  (#792).** On both BIM4LCA office files two generic models carry their own
  `IfcStair` and a curtain panel its own `IfcDoor` with `DOOR`, and Revit's
  export writes them as `IfcStair .NOTDEFINED.` and `IfcDoor .DOOR.`. rvt-rs
  did not act on either value and wrote a proxy and a plate.
- **A wall sweep in a Revit 2024 file stays an `IfcBuildingElementProxy`,
  whatever its "Export to IFC As" says (#792, RE-192).** On both BIM4LCA
  office files 9 wall sweeps carry their own `IfcCovering` (predefined type
  `ROOFING`), and Revit's export writes all 9 as proxies. rvt-rs followed the
  value and wrote coverings. This is measured on that one project, not read
  from the exporter's source, whose wall sweep path takes the entity from the
  same lookup as other elements; other releases keep following the value.
- **A family's or type's name comes from its latest copy, so its elements
  keep their type (#807).** The family and type name index and the system
  family type names dropped a name whose copies in different partitions
  differed, and the elements joined through it got no type. That follows
  #800's rule (RE-145). On the BIM4LCA office's building-permit file, where
  every family and type has a Finnish-named older copy and an English-named
  newer one, rvt-rs typed none of 345 columns, 52 doors, 43 light fixtures and
  20 appliances, and no 4 walls and 8 slabs. It now types the same elements
  as on the as-built file. Revit's names now match on all of those
  columns, doors and light fixtures, and the permit file gets materials on
  403 more elements and 67 ports, Revit's count. The other files export as
  before.
- **A room's name and its other parameters come from its latest copy
  (#800).** Where several partitions hold copies of a room's data that
  differ, the number and name readers (RE-117) and the text and integer
  parameter readers (RE-153, RE-158) gave the room nothing. They now take the
  copy in the latest partition, the one RE-145 found current wherever an
  element's records differ, and give nothing only when copies within that
  partition disagree. The BIM4LCA office's building-permit file holds an
  older copy of every room and area with Finnish names and a newer one with
  English names, and Revit's export uses the English ones. There, rooms
  named as in Revit's export go from 1 to 191 of 234, as on the as-built
  file, and each named room gets its `IfcSpaceType`. The as-built file, the
  six reference models and the samples export as before.
- **Fixtures and appliances follow their "Export to IFC As" (#792,
  RE-190).** Revit's export writes an element as the entity its type's
  "Export Type to IFC As" names: waste, fire suppression, sanitary and air
  terminals and electric appliances on the BIM4LCA office. rvt-rs now does
  too, with the type's predefined type where it is an IFC4 enumerator of the
  entity. Waste and fire suppression terminals get their type entities and
  ports, and a room's space contains them as Revit's export does. On the
  as-built file 185 elements now have Revit's class (78 waste terminals, 51
  electric appliances, 40 fire suppression terminals, 15 wash-hand basins
  and an air terminal), and the type entity is Revit's on 12,355 of 12,358
  elements (was 12,170). On the permit file the count is 165, because rvt-rs
  types none of 20 appliances there.
- **An export value belongs to the data object that holds it, on Revit
  2024 and later (#792, RE-190).** RE-45 gave an entry to the last
  element-data header within 0x800 bytes. On the office's permit file that
  gave `IfcSpace` to doors, curtain walls, sketch lines and five drains,
  whose types' `IfcWasteTerminalType` it hid, and the drains' type was
  written twice under one GlobalId (four schema-validation findings, now
  none). Revit 2023 and earlier keep the header rule. The six reference
  models export as before.
- **A pipe or duct type name needs the `u32 0` before its length (RE-179).**
  RE-130's scan matched an element's reference lists on Autodesk's 2027
  `rme_basic` and named 33 pipes `였縿괜쭃`. All six type names of RE1
  Plumbing and Mechanical carry the zero word; the false match does not.
- **A curtain wall in a Revit 2024 file is `IfcCurtainWall`, whatever its
  "Export to IFC As" says (#792, RE-188).** Revit 2024's exporter never reads
  the value on a curtain wall: revit-ifc 24.1.0 to 24.3.40 create every
  curtain wall as one. rvt-rs followed the value, and wrote the 124 BIM4LCA
  office curtain walls set to `IfcShadingDevice` as shading devices, where
  Revit's export of both files has curtain walls. In 2025 and 2026 files,
  whose exporters read the value from 25.2.0 on, a curtain wall keeps
  following it. That half is read from the exporter's source and not
  measured on a Revit export. The export diagnostics also warn with every
  "Export to IFC As" value the export does not apply, counted by value and
  class, which `category_map`'s documentation had already promised.
- **An ElemTable record's first id is no longer declared where it differs
  from its second (#760, RE-189).** On Autodesk's Snowdon Towers 2024
  samples, 84 records (Architectural) and 27 (Structural) carry two ids: the
  element's ElementId and the id it was created with. Nothing in either
  project names a first id that is not also some element's current id, so
  `elem_table::declared_ids` now declares each record's current id only,
  dropping 84 and 20 ids that named no element. Measured on the 49 Autodesk
  samples Measure reads and CI's 6 reference models, exports and `rvt-info`
  are unchanged; only the two Snowdon models' `decoded_element_fraction`
  rises, as its denominator shrinks.
- **Export diagnostics report the file name the model was saved under, not
  its folders (#782).** Their `input.original_path` kept the whole path with
  only the Windows account folder redacted, so client and project folder
  names reached a file people attach to bug reports. It is now the file name,
  as the export's name has been since 0.5.1: RE1 Architecture's reads
  `RE1.rvt`.

### Research

- **RE-194: where a parameter named for a property set's property keeps its
  value, and where Revit's export writes it.** A Yes/No or integer is the
  `u32` after the parameter's ElementId in the element's own data object, a
  text the `u32 n · UTF-16` after it, and a number the `f64` just before it.
  Revit's export of the BIM4LCA office writes every such value on the element
  itself, never only on its type. Probe `probe_re194_pset_parameters`,
  scorer `pset_parameters_vs_ifc`. `psets_vs_ifc` now also counts the values
  only rvt-rs writes, by set and property, and Measure gives each reference
  model's export the schema check a sample's gets, up to 120 MB: the office
  files' thermal transmittance unit is a writer path nothing else reaches.
- **Measure hashes each sample's IFC export.** A sample's export was checked
  for schema findings and then dropped, so a change that moved only its
  names or properties showed nowhere in a run. Each sample now gets the
  `ifc.sha256` a model gets, with the export time ignored. The compare job
  lists the samples with the models as identical or differing, or as having
  no base hash when the base ran before this change.
- **RE-185: where a wall's `IsExternal` and `ExtendToStructure` come from
  (#35).** A wall's `IsExternal` is its type's native `WallType.m_function`
  (0 interior, 1 exterior), and its `ExtendToStructure` is true exactly where
  its record holds a `JoinToRoofGStep`. These agree with Revit's IFC4 export
  on 8 walls of RE1 Architecture and 117 of puzzbobb's Revit 2025 models,
  and Measure reproduces them with `-f r2025=true`. A window's `FireRating`
  is a family-type text entry that RE-77's scan does not read. Probe
  `examples/probe_re185_wall_function.rs`; nothing is exported from these
  yet. By puzzbobb.
- **Measure scores puzzbobb's Revit 2025 models (#766, #356).** With
  `-f r2025=true` it fetches `R2025_Models.zip` by its pinned hash
  (`research/r2025-models.tsv`): 23 projects that each isolate one feature
  (roofs, slope arrows, rooms, stairs, wall joins). Measure scores the 22 that
  come with Revit's IFC4 export against it, as it scores the reference
  models. Their author released them under Apache-2.0 on #356; they are
  fetched for the run and never stored.
- **The class-set scorer says why an element is in another class (#792).**
  `tools/re/class_sets_vs_ifc.py` prints, for each group in another class
  than Revit's, Revit's category, PredefinedType and type object and the
  RE-45 value rvt-rs read. A new probe scorer,
  `tools/re/export_parameters_vs_ifc.py`, compares every "Export to IFC As"
  value the RE-45 probe reads with Revit's export. The RE-45 probe prints
  each entry's owner under both rules. `ports_vs_ifc` prints its first ten
  schema-validation findings.
- **Autodesk's Revit 2015 sample projects are pinned for Measure (#742).**
  Autodesk keeps all six on its download host under `Docs/`, where the 2016
  to 2027 samples are under `Docs/InProd/`, so a 2015 reading no longer rests
  on files only a contributor holds. The 2019 MEP and `rst_advanced` projects
  are pinned with them, so each 2015 project has a 2019 copy to compare
  with (`-f probe_pair_release=2019`): nine more files, 58 in all.
- **Measure compares each sample with the same project saved by another
  release (#740, #742).** With `-f probe_pair_release=<year>`, a two-file
  probe also runs on every sample of another release and the same project
  saved by that year, the sample first, into the sample's `probe-pair.txt`;
  `-f sample_years="<first> <last>"` reads only the samples of those
  releases. The comparisons of Autodesk's 2027 samples with their 2026
  copies, and of earlier releases with their 2023 and 2019 copies, then run
  on the runners instead of on one machine.
- **Measure scores the BIM4LCA office against Revit's exports (#763).** With
  `-f nordic=true` it reads the concrete office as built and at building
  permit, with Revit's IFC export of each, by byte range
  (`research/bim4lca-measure.tsv`), and scores them like the reference
  models. A new scorer, `tools/re/class_sets_vs_ifc.py`, runs on every model:
  per IFC class of Revit's export, how many of its elements rvt-rs writes as
  the same class, as another class or not at all, paired by Tag (spaces by
  GlobalId), with the missing and the extra ones listed by name, type and
  storey.
- **The BIM4LCA models are read in CI (#766).** The tier-two job fetches the
  four Revit 2024 models and the Revit IFC export its targets read from VTT's
  1.58 GB `ARCH.zip` by byte range (`tools/ci/fetch-bim4lca.py`), each
  checked against `research/bim4lca-models.tsv`, so the targets that measure
  #751's level parameters, #752's shared elevations, #753's column bottoms
  and #757's cut columns on them run on every pull request; they skipped.

## [0.5.1] — 2026-10-18

A patch release of 0.5.0. An export is named by the model's file name
instead of the path the model was saved at, which could carry the author's
account and folder names (#782). crates.io and PyPI show the README packed
into each version, and 0.5.0's still called 0.4.0 the latest release;
0.5.1's names the release it ships in.

### Upgrading from 0.5.0

- **A file without a PartAtom title, as most projects are, exports under its
  file name** (#782): the `IfcProject`'s Name, the IFC header's `FILE_NAME`,
  the glTF scene root and the diagnostics' `project_name` were its original
  path, folders included.

### Fixed

- **An export names the model by its file name, not the path it was saved
  at (#782).** A file without a PartAtom title, as most projects are, was
  named after its full original path, which can hold the author's Windows
  account and client folder names. That name went into the IFC header's
  `FILE_NAME`, the `IfcProject`'s Name, the glTF scene root and the viewer's
  project node, and the export diagnostics' `project_name`, which skipped the
  redaction their `original_path` gets. It is now the file name without its
  extension: RE1 Architecture's export is named `RE1`.
- **The README names the release it ships in.** The release pull request now
  moves the README's release section before the tag (`RELEASE.md`, step 8),
  since a published version's README can never be edited; the channel checks
  still follow in the post-release pull request.

### Changed

- **serde_json 1.0.154** (#776), a patch update in the lockfile the CLI
  archives, the wheels and the container image are built from.

### Research

- **Autodesk's Snowdon Towers 2024 models are pinned for Measure (#766).**
  `-f samples=true` now also reads the sample's Architectural, Structural,
  HVAC, Plumbing and Electrical models from Autodesk's download host, each
  pinned by sha256 and size and never stored, so a regression like #736
  shows in the run's head-against-base diff. On `main` they export 6,056,
  1,282, 2,596, 6,253 and 1,576 elements (run 39119698158).

## [0.5.0] — 2026-10-18

Revit 2026 projects now export typed elements, experimentally, measured on
one project. MEP models gain ports, port-to-port connections and systems,
pipes are drawn as the cylinders Revit draws, and Autodesk's 2024 MEP
sample is walked in 10 seconds instead of 17 minutes. Exports carry more of
what Revit's own export writes: each element's levels, its category, family,
type and phase, and more of the common property sets. Rooms sit in the
spatial tree as IFC4 requires, and Autodesk's Snowdon Towers samples export
their elements again. Family instances can be drawn from the meshes Revit
saved for display, on Revit 2024 to 2026 files (`--saved-meshes`, opt-in).
Every figure is measured against Revit's own export of the same model.

This is the first release with pull requests from outside contributors:
helenkwok, pinterbanget and Persus1337. rosejn's saved-graphics reader is
behind `--saved-meshes`, and format research from STE1200, jakobhirn-bit
and puzzbobb is behind several entries below. Thank you.

### Upgrading from 0.4

- **The minimum supported Rust version is 1.87,** up from 1.85.
- **The WebAssembly bindings are the `rvt-wasm` crate.** `rvt` has no `wasm`
  feature and builds as a plain `rlib`; build the browser package with
  `wasm-pack build rvt-wasm --target web --out-dir ../viewer/pkg --out-name rvt`.
- **Each `IfcSpace` is aggregated into its storey** through `IfcRelAggregates`
  (or into the building where no storey is known), and is no longer listed in
  the storey's `IfcRelContainedInSpatialStructure` (#744). A reader that looked
  for rooms among a storey's contained elements reads the aggregation instead.
- **Exports carry more entities:** an `IfcSpaceType` for each room, ports
  nested in their elements, `IfcSystem`s, and the property sets above. Counts
  keyed on 0.4's output change.
- **Type objects take Revit's GlobalIds** (RE-167): a door's or window's type
  is keyed by its original symbol and its flip, and a family instance's type
  takes its original symbol's GlobalId, so type GlobalIds differ from 0.4's.
- **`elem_table::ElemTableHeader::record_count` is a `u32`** (#764): the count
  the table states, which can exceed 65,535.

### Added

- **Release binaries carry their dependency list (C10).** Each binary of a
  GitHub release is built with `cargo auditable`, which embeds the crates
  and versions it was built from, so `cargo audit bin` and other scanners
  can check a downloaded binary against the RustSec advisories.
- **Revit's level parameters, in a `Constraints` property set (RE-172 to
  RE-176, #750).** Read from each element's association record: a wall's
  Base Constraint and Top Constraint, a column's Base Level and Level, a
  railing's, roof's or stair's Base Level, a duct's or pipe's Reference
  Level, and any other element's Level, with Revit's names for each class.
  A value is written only when the id is a Level rvt-rs recovered, on Revit
  2024. Beams and braces whose association record holds no Level take the
  highest Level their record lists (RE-175), and stairs their Base and Top
  Level from the record or its lists (RE-174, RE-176). helenkwok measured
  every written value against Revit's IFC4 export of seven Autodesk 2024
  samples and the BIM4LCA models, and none differs (#751).
- **Category, family and type, and Phase Created in Revit's own property
  sets (RE-177, #754).** An element's `Other` set carries its Category (its
  `BuiltInCategory` by the English name Revit's export writes), Family,
  Family and Type, Type and Type Id, and its `Phasing` set its Phase
  Created; a type object carries its Category and Family Name in `Other`
  and its Type Name in `Identity Data`. Phase names are read from the
  file's `ProjectPhase` frames. helenkwok measured them against Revit's
  IFC4 export of seven Autodesk 2024 samples, with no wrong value (#755).
- **Every join of an MEP model is read from its family instances' connectors
  (B86).** A family instance's native `Connector` objects list the elements
  joined to each connector (RE-171), so the export writes those joins
  wherever the native record path reads the file (Revit 2023 to 2027), joins
  to equipment written as a proxy included. RE1 Mechanical writes all 73 of
  Revit's joins and RE1 Plumbing all 126, and Autodesk's MEP samples write
  about 2,600 each on 2024, 2025 and 2026 alike. Checked by
  `tests/mep_connections.rs`.
- **Every port is nested in its element (B84).** Ports are related to their
  elements with an `IfcRelNests`, one per element, as Revit's own IFC4 export
  writes them; no `IfcRelConnectsPortToElement` is written. Checked by
  `tests/port_nesting.rs`.
- **A family instance written as a proxy has its ports (B83).** Each port
  is nested in its element with an `IfcRelNests`, as Revit's own IFC4 export
  nests ports, since IFC4 ties a port to a distribution element only. RE1
  Electrical's 53 and RE1 Mechanical's 150 ports are all written.
- **Each room is typed by an `IfcSpaceType` of its own (B78).** Revit's export
  of Core Interior types every `IfcSpace`: the type is named
  `<room name> <room number>:<ElementId>`, its Tag is the room's ElementId,
  its PredefinedType NOTDEFINED, and its GlobalId the room's with bytes 10
  and 11 XOR 2048 (revit-ifc `InstanceAsType`). rvt-rs now writes the same 116
  types and relations. Checked by `tests/space_types.rs`.
- **A duct's `Pset_DuctSegmentTypeCommon.Shape` (B43).** Written as Revit's
  export writes it, an `IfcPropertyEnumeratedValue` of `RECTANGULAR`, `ROUND`
  or `FLATOVAL` from the duct type's sweep profile, through a new
  `PropertyValue::Enumerated`; the viewer's property panel shows it as text.
  RE1 Mechanical's 25 ducts are all rectangular, so `ROUND` and `FLATOVAL`
  rest on IFC's enumeration alone. Checked by `tests/duct_segment_shape.rs`.
- **A duct takes the system family its type's shape names (B59).** A rigid
  duct type's native record holds its sweep profile
  (`AbsSysRectSweepProfile`, `AbsSysCircSweepProfile` or
  `AbsSysOvalSweepProfile`), and Revit names the duct's system family by it.
  RE1 Mechanical's ducts are now named `Rectangular Duct:<type>:<id>`, with
  that ObjectType and an `IfcDuctSegmentType`, as in Revit's export
  (`native_duct_shapes`, Revit 2023 to 2027). Checked by `tests/duct_names.rs`.
- **A family instance has a port for each of its symbol's connectors (RE-169,
  B54, #528).** A FamilySymbol's native record lists its connectors
  (`m_arrConnectorData`); each family instance written as a distribution
  element now has a port per connector, joined or not, the cable tray and
  conduit ones aside, as in Revit's export (`native_connectors`). Electrical's
  proxies keep none: IFC4 ties a port only to a distribution element. Checked
  by `tests/family_instance_ports.rs` on RE1 Electrical, Plumbing and
  Mechanical.
- **`--saved-meshes` reads Revit 2025 and 2026 (B77).** On RE1's four 2025
  models 207 of 214 replaced bodies are within 0.01 ft of Revit's box
  (Mechanical 68 of 68, Plumbing 41 of 41, Electrical 47 of 49, Architecture
  51 of 56), and all ten 2025 and 2026 samples export with the flag, so it
  now reads every release the native record path reads.
- **A wall's edited elevation profile cuts an opening (RE-151, B55, #227).**
  Wall 55840's sketch on Core Interior is a loop of 11 lines in its vertical
  plane; the lines that leave the loop's bounding rectangle cut the region
  Revit's export writes as opening 55859 voiding the wall. rvt-rs now writes
  it, in the wall's plane and frame, extruded through its thickness: 63 of
  Revit's 63 unfilled openings, the wall's within 0.001 ft of Revit's box.
  Checked by `tests/wall_profile_openings.rs`.
- **The native record path reads Revit 2025 and 2026 (B46, #328).** Its
  framing, record-count and body-size checks hold on all nine 2025 files
  (RE1 and Autodesk's samples) and all five 2026 samples, so
  `native_document::extract` now admits 2023 to 2027, and with it each
  element's `m_createdPhaseId` and `m_demolishedPhaseId`. Checked by
  `tests/native_phases.rs` on the four RE1 models.
- **A railing whose list names no type takes the one its own data object
  names (B45).** RE1 Architecture's railing 462556 holds its type 446543 in
  its `BaseRailing` data object and is written with Revit's `IfcRailingType`
  `Railing:-`, its Tag and GlobalId, and `Reference` `-`. The type's name is
  read from its `StairsRailingAttr` object, ended by a `BalusterPattern`
  frame; the tags RE-66 measured on 2024 are class ordinals that move every
  release, so they are now read from the file's schema by name. A typed
  railing is now written with its `IfcRailingType`. Checked by
  `tests/railing_type.rs`.
- **Every duct and pipe has a port at each end (#528, B54).** Revit's export
  writes connectors 0 and 1 of 92 of RE1's 94 ducts and pipes as
  `IfcDistributionPort`s, joined or not; rvt-rs wrote a port only for a join
  it read. Each duct and pipe exported as a distribution element now has both
  (RE1 Mechanical 62 of 62, Plumbing 122 of 122). Checked by
  `tests/duct_and_pipe_ports.rs`.
- **Door and window types take Revit's hashed GlobalId (RE-167, B72).** Revit
  keys a door's or window's type by the MD5 of its original symbol's GlobalId
  with the door's flip (`Sub-element:Flipped: True|False`, and
  ` InAssembly: False` from revit-ifc 25.4), the flip from the door's box
  against its host wall's axis (revit-ifc `DoorWindowInfo`). A door type is
  written once per flip. Checked by `tests/door_type_global_ids.rs` on RE1 and
  Core Interior.
- **A family instance's type takes its original symbol's GlobalId (RE-167,
  B60, B73).** The original symbol is the `u32` 20 bytes before the end of the
  instance's `GElement` data object (a FamilySymbol, SysMullionFamSym or
  SysPanelFamSym); Revit writes one type object per original symbol, keeping
  the symbol's Tag. An instance that uses its own geometry takes a sub-element
  GlobalId of itself (revit-ifc `InstanceAsType`), and a type written under
  several entities keeps its own GlobalId once, the others hashed with the
  entity and predefined type. Checked by `tests/type_global_ids.rs`.
- **A door or window in a curtain grid is one of its curtain wall's parts
  (B64).** As in Revit's export, with no opening and no host wall. Checked by
  `tests/curtain_wall_doors.rs`.
- **Curtain wall mullions and panels take their type's material (RE-166,
  B66).** A mullion type's material at byte 135 of its data object, a panel
  type's at 139, Revit 2024 to 2026. Checked by `tests/curtain_part_materials.rs`.
- **A pipe fitting's `NominalDiameter` (RE-165, B44, #35).** From its
  connector records, or on a fitting with none (a cap) its smallest
  parameter pair, as an `IfcPropertyListValue` (`PropertyValue::List`); 42 of
  42 on RE1 Plumbing. Checked by `tests/fitting_nominal_diameter.rs`.
- **A room's furniture, fixtures and equipment are contained in its space
  (B63).** A family instance written as furniture, a sanitary terminal or a
  proxy, hosted by nothing, is contained in the `IfcSpace` whose outline holds
  it on its storey; 39 of 39 on RE1 Architecture. Checked by
  `tests/space_containment.rs`.
- **MEP systems as `IfcSystem` (RE-162, B33, #528).** Each `RbsHvacSystem`,
  `RbsPipingSystem` or `RbsElectricalSystem` data object, grouping the members
  that hold its id and serving the building; all 26 of RE1's systems with
  Revit's names and members. Checked by `tests/mep_systems.rs`.
- **Reference on equipment named by its panel name, and a pipe's type from its
  curve object (B29, #35).**
- **A ceiling's `Pset_CoveringCommon.Finish` (B41, #35):** its finish layers'
  materials, each followed by `;`. Checked by `tests/covering_finish.rs`.
- **A door's and a slab's `IsExternal`, from its type's Function (RE-158, B28,
  #35).** Checked by `tests/is_external.rs`.
- **A pipe's `InvertElevation` (RE-157, B26, #35):** a horizontal pipe's axis
  less half its inner diameter, read from its own curve object, or a vertical
  pipe's lower end, above its storey. Checked by `tests/invert_elevation.rs`.
- **The shared parameter Serial Number (RE-156, #35)** in
  `Pset_ManufacturerOccurrence.SerialNumber`, and on proxies in
  `Pset_PrecastConcreteElementGeneral`. Checked by `tests/serial_number.rs`.
- **Storeys, the building and spaces carry Revit's spatial property sets
  (#35).** Checked by `tests/spatial_property_sets.rs`.
- **A duct's size, type, `Length` and `Reference` from its curve object
  (RE-134, #35, #96), and a pipe's `Length` (#35),** in the flow-segment sets
  and `Pset_DuctSegmentTypeCommon`. Checked by `tests/duct_fields.rs`,
  `tests/segment_length.rs` and `tests/duct_segment_type_common.rs`.
- **Python: `part_atom_json_strict` (#502).** Names a missing or malformed
  PartAtom instead of returning nothing.
- **Six CLIs run end to end on real files (B53).** `rvt-elem-table`,
  `rvt-history`, `rvt-analyze`, `rvt-diff`, `rvt-ifc-compare` and `rvt-write`,
  in `tests/cli_real_files.rs`.
- **The `Reference` of Revit's common property sets (RE-154, #35).** Revit's
  export gives every element `Pset_QuantityTakeOff.Reference`, its entity's
  common set's `Reference` (`Pset_WallCommon`, `Pset_DoorCommon`,
  `Pset_DistributionFlowElementCommon` ...), and on walls and slabs the
  reinforcement-pitch set's, each the type's name; no stored parameter holds
  it. The IFC export now writes those sets on each element named
  `Family:Type:ElementId`, for the entities measured on the RE1 exports.
  Checked by the new real-file target `tests/common_reference.rs` against all
  four RE1 exports.
- **A room's `Pset_SpaceCommon`, with its Floor Finish (RE-153, #35).** Revit's
  export gives each room a `Pset_SpaceCommon` with `Reference`
  (`IfcIdentifier`, its name and number) and `FloorCovering` (`IfcLabel`, its
  Floor Finish). A room's Floor Finish is the text parameter -1006903, joined
  to the room through the Adler-32-verified data object that holds it; on RE1
  Architecture it is Revit's `FloorCovering` on 11 of 11 rooms. The IFC export
  now writes that set on every room. Checked by the new real-file target
  `tests/space_common.rs` against RE1 Architecture's export.
  `partition_room_parameters::scan_room_text_parameter` and
  `enclosing_data_object`, and `PropertyValue::Label` and `::Identifier`.
- **A floor's sketch holes are openings voiding it (RE-151, #227).** Revit's
  export writes each hole in a floor's or shading device's sketch twice: as a
  void of the slab's profile, which rvt-rs already wrote, and as an
  `IfcOpeningElement` voiding the slab, with the hole's outline through the
  slab's thickness, placed in the slab's frame, named `Family:Type:<type id>`
  and tagged with the lowest ElementId of the sketch lines on the hole. The
  IFC export now writes those openings: Core Interior's 42 slab and 20
  shading-device openings, all within 0.001 ft of Revit's (Measure). Checked
  by the new real-file target `tests/unfilled_openings.rs`. The 63rd, a hole
  in wall 55840's elevation profile, is not read.
  `element_record_plan_profiles::tag_voids`, `PlanProfile::void_openings`
  and `IfcEntity::VoidOpening`.
- **Family instances whose type draws with no material take Revit's
  `<Unnamed>`, and columns walls are joined to take the walls' material
  (#355).** A family type's geometry-material map (RE-82, puzzbobb's
  `FamilySymbol.m_geomTag2MaterialId`) whose every value is unset read as no
  map, and its instances got no material. Revit's export gives them
  `<Unnamed>`: 107 of Core Interior's 256 columns and 150 of the 152 RE1
  fittings, terminals and fixtures it relates a material to. Such a type is
  now recognised by the first map in its value block, distinct geometry-tag
  keys and only unset values (RE-149), and its columns and products take
  `<Unnamed>`. A column the record's reference list joins to overlapping
  walls takes those walls' material instead, as Revit gives Core Interior's
  other 149 columns "Default Wall" (the module that cuts columns by those
  walls already found the same 149/107 split). Doors, windows, walls and
  slabs are unchanged. Checked by the new real-file target
  `tests/element_materials.rs`: all 256 of Core Interior's columns have
  Revit's material set. `element_record_column_cuts::column_joined_walls`,
  `partition_type_materials::unset_material_types` and `UNNAMED_MATERIAL`.
- **Ports for the joins of duct and pipe fittings (RE-141, #528).** A
  fitting's joins are written in the same list form as a pipe's, with the
  fitting first, and are read wherever they are in a partition. The IFC export
  now writes an `IfcDistributionPort` for each connector of a joined pair of
  fittings, ducts and pipes, fitting to fitting included: against Revit's own
  export of the RE1 models, 66 of 73 of its port-to-port connections on
  Mechanical (53 before) and 120 of 126 on Plumbing (112 before), none and one
  that Revit does not hold, no schema finding, and no other model's output
  changes. `partition_connector_pairs::scan_fitting_pairs`, and the joins type
  `ConnectorJoins` now holds any number of connectors; the field names are
  built by `connector_element_field` and `connector_index_field` where
  `CONNECTOR_ELEMENT_FIELDS` and `CONNECTOR_INDEX_FIELDS` were.
  `reports/element-framing/RE-141-fitting-joins.md`.
- **Ports and port-to-port connections for ducts and pipes (RE-138,
  #528).** A duct's or pipe's object lists its connectors' joins after its
  `RbsCurveConnectorManager` anchor, in two blocks that store the curve's own
  connector index differently (inverted in the earlier). rvt-rs reads them
  and writes an `IfcDistributionPort` for each connector, an
  `IfcRelConnectsPortToElement` for each port and an `IfcRelConnectsPorts` for
  each join whose two elements it exports as distribution elements (IFC4
  allows a port on no other, so equipment written as a building element proxy
  has none): 53 of the 59 connections of Revit's export that involve a duct or
  pipe on RE1 Mechanical, 112 of 118 on Plumbing, with every port tied to the
  element its name gives, one connection written that Revit's export lacks and
  no finding in IfcOpenShell's schema validation. The ports have no flow
  direction (Revit's names, `InPort_` and `OutPort_`, follow the system's
  flow, which the lists do not carry), so they are named
  `Port_<ElementId>_<index>`; fitting-to-fitting and equipment connections are
  not read. `examples/probe_re138_connector_pairs.rs`,
  `tools/re/ports_vs_ifc.py`.
- **Pipes as the cylinders Revit draws (RE-131, #96).** A pipe's two ends
  are connector entries in its partition (an element, a connector index,
  the `u32` 1 and a point of three `f64`), keyed by the pipe itself or by
  the element connected at that end, and its record box gives the outside
  radius: every axis that is not nearly the
  pipe's own must give the same radius, and the box rebuilt from the
  cylinder must be the record box on all six sides, or the pipe keeps its
  box. On RE1 Mechanical and Plumbing (Revit 2025), against Revit's own
  export: 69 of 69 pipes are circle-profile extrusions whose radius, axis
  ends and length are Revit's to 0.00001 ft. `main` wrote every one as a
  box, and the right axis for 23 of the 69. No other model's IFC
  changes. The entry layout and the radius derivation are jakobhirn-bit's
  (Discussion #112), measured by them on Snowdon Towers' plumbing sample.
  Revit 2025 only. Every RE1 pipe runs along a principal axis, so sloped
  pipes rest on the rebuild proof, not on an oracle; ducts and fittings keep
  their boxes.
- **Family instances in their real shape, opt-in (RE-78, #255, #227).**
  `rvt-ifc --saved-meshes` and `rvt-gltf --saved-meshes` replace each
  bounding-box body with the mesh Revit saved for the element's display,
  read by rosejn's native saved-graphics reader from #255 (extracted with
  their authorship). Measured against Revit's own export, per element world
  box: on Core Interior every door, window and shading device (158 of 158)
  is within 0.01 ft, where the box made every door 2.92 ft too deep; on
  Snowdon Towers 3,332 of the 3,736 replaced bodies are, up from 1,138
  for their boxes, including 131 of 132 doors, 289 of 388 light fixtures and
  all 110 columns. Saved meshes are written as `IfcTriangulatedFaceSet`
  (Snowdon's IFC is 111 MB with them, 12 MB without) and take 37 s on
  Snowdon, so they stay opt-in. Windows are the exception (2 of 68 on
  Snowdon): Revit's export draws parts the saved Fine-detail mesh does not.
  Stairs and curtain walls keep the bodies of their parts. Revit 2024 only:
  the reader declines 2025 files, which then export with their boxes.
- **Pipes named as Revit names them (RE-130, #96).** A pipe's type is in
  its record's reference list, but pipe and duct types have no name entry,
  so no type was found. Their name follows the type's ElementId at a fixed
  place. Pipes now export as `Pipe Types:<type>:<ElementId>` with their
  type object: RE1 Mechanical 6 of 6 and Plumbing 62 of 63 (none before),
  and Plumbing's elements are typed as Revit types them on 122 of 123 (60
  before). Ducts are not named yet: their family follows their type's
  shape.
- **Joined-wall lists decide the L corners the join lists leave open
  (RE-127, #238).** Beside RE-70's join lists, a wall's data lists every
  wall it is joined to, with a run of values per wall. Where the join
  lists do not say which of two walls runs through, the one whose entry
  for the other is a run of consecutive values stops at the other's face,
  and the other ends at the corner. On Core Interior that settles the last
  9 ends RE-26 cut the wrong way: wall bodies are 360 of 360 exact against
  Revit's export (351 before). No other score and no other reference
  model's IFC changes.
- **Revit 2026 wall join lists (RE-126, #421).** They are laid out as on
  2025, so 2026 walls are now joined by them. Against the house's
  CoordinationView export, which draws each wall as one box, 85 of 118
  wall ends are where Revit puts them (27 of 48 before, on the 24 walls
  that had a centreline).
- **Material names kept after parameter entries (RE-125, #355).** A
  material whose name field holds shared parameters (a manufacturer's
  product, supplier, URL, classification) keeps its name after them,
  closed by eight zero bytes, an ElementId and `0xff`×8. Read as a last
  resort, it names every material on the Revit 2026 house (148 of 148,
  from 116), Core Interior (86 of 86, from 78) and Snowdon Towers (220 of
  220). On the house, layer names are Revit's on 183 layers (13 before)
  and 65 of 67 elements have Revit's material set (5 of 19). Element
  material sets on Core Interior and Snowdon are unchanged; their
  material lists gain the names.
- **Revit 2026 projects export typed elements (RE-124, #421).** Every
  per-release constant rvt-rs reads is the tag of a class in the file's own
  schema, and on a Revit 2026 project those tags give the 2026 values:
  the record marker (`Outline`, `ElementParents`), Level elevations
  (`Plane`), element data (`CellList`), layers (`VerticalRegionsStructure`,
  whose 2026 records are 41 bytes) and materials (`Material`,
  `PatternHelper`, `PhysicalParamSet`). On the flowbim.ee house against
  Revit 26.4's own IFC4X3 export: 113 of its 115 elements, none outside
  it, with Revit's GlobalIds; its 4 storeys with Revit's names and
  elevations; 13 of 13 rooms with Revit's number and name; 13 of 13 doors
  and 15 of 16 windows in their host walls; walls named `Family:Type` as
  Revit names them, and 26 layer sets as thick as Revit's. Experimental:
  one model. Wall joins, roof slopes, stairs and IFC export overrides are
  not read on 2026.

### Changed

- **revit-ifc is an accepted clean-room source for what Revit's own export
  writes (#778).** `CLEANROOM.md` accepts Autodesk's open-source IFC exporter
  (LGPL v2) for how the export forms an identifier and which entity,
  relationship or value it writes, never for the file format. Contributors
  cite the file and class, write code from a description of the behaviour
  rather than copying or translating it, and check every finding against
  Revit's own export. The type GlobalIds of RE-167 rest on it, and #778
  audits each current citation against the rule.
- **The integration tests are one test binary, `integration` (C6).** Every
  `tests/*.rs` file is a module of `tests/integration/main.rs`, and its tests
  are named `<file>::<test>`: run one file with `cargo test --test
  integration -- <file>::` where `cargo test --test <file>` ran it before.
  Building them went from 118 s of build units for 77 binaries to 13 s for
  one.
- **The WebAssembly bindings are their own crate, `rvt-wasm` (C6).** `rvt`
  builds as a plain `rlib` and has no `wasm` feature; build the viewer's
  package with `wasm-pack build rvt-wasm --target web --out-dir ../viewer/pkg
  --out-name rvt`. Native builds no longer link a shared library they never
  use.
- **MEP models export several times faster (B80).** With the IFC
  byte-identical on all six reference models and 36 samples:
  - family-instance ports and duct shapes read object graphs without the
    parameter-definition pass (`native_document::extract_graphs`);
  - each partition stream's data objects are scanned once per file
    (`RevitFile::partition_data_objects`), and MEP systems, pipe diameters
    and fitting sizes read that scan instead of searching every stream once
    per element;
  - a data object's Adler-32 is summed in place with SIMD (`simd-adler32`),
    as a stream's false headers claimed gigabytes that were copied before
    being rejected.
  - a family instance's ports are read in one native pass, the symbols by id
    and every family by class (B79);
  - each native extraction reuses the schema, element index and increments
    the file has already read, and the partition members the export has
    already inflated (B82).
  - a partition stream's room solids are found in one pass rather than one
    search per room (B85).
  - a curve's connector entries are searched for only after its anchor, not
    across its whole partition (B87).
  Measured step by step inside one export, the 2025 `rme_advanced` sample's
  ports step went from 6.1 s to 1.7 s, its MEP systems from 4.6 s to 0.06 s,
  its curve and fitting sizes from 14.7 s to 1.0 s, and its first data-object
  scan from 3.2 s to 0.4 s.
- **Measure records each `rvt-ifc` export's wall time, takes `rvt-ifc`
  flags, and can time both refs on one runner (B81).** Each model's and
  sample's `rvt-ifc.log` ends in `milliseconds <n>`, and the summary's table
  names each side's CPU: the base and head jobs run on different runners,
  whose speed alone moves those times by up to about 1.7x. The `timing` input
  builds both refs on one runner and exports every file with each in turn.
  The `ifc_flags` input passes flags such as `--saved-meshes`, scored by
  `saved_meshes_vs_ifc`.
- **An export walks the file once (B71).** `RevitFile` keeps the production
  walk, and the export's diagnostics reuse it instead of walking again; IFC
  and diagnostics are byte-identical on all six reference models.
- **MEP models export in seconds, not half an hour (B70).** MEP system
  membership and pipe type names are found in one pass per partition stream
  (`partition_room_parameters::data_objects`) instead of a search per id; a
  walk of Autodesk's 2024 MEP sample went from 1,037 s to 9.8 s, with the IFC
  byte-identical on all six reference models.
- **The slim witness verdict runs whenever the full one ran (B27),** so a
  pinned-count change regenerates both observations in one CI cycle.
- **The minimum supported Rust version is 1.87, up from 1.85.** quick-xml 0.42
  needs 1.86 and earcut 0.4.10 and later call `is_multiple_of`, stabilised in
  1.87, and both updates were being held back for it (earcut's bound in
  `Cargo.toml` said so). They are taken: the PartAtom reader now reads
  quick-xml 0.42's `&str` names, text and entity references, and earcut is at
  0.4.11. `rvt` 0.4.0 on crates.io declares 1.85. `docs/maintaining.md` says
  how the MSRV moves, and the MSRV job in CI now builds on 1.87 (its check name
  changes with it).
- **The MSRV job is a required check.** A pull request that needs a newer Rust
  than `rust-version` can no longer merge past it: Dependabot's earcut 0.4.11
  update failed the job while GitHub still called the pull request mergeable.
  The check is named `build + real files / ubuntu-latest / msrv`, without the
  version, so the ruleset in `.github/rulesets/main-protection.json` does not
  change when the MSRV does.
- **Element records are scanned with the marker the file's own schema
  gives (#421).** The marker is `Outline`'s tag, `0xFF`×4 and
  `ElementParents`' tag in the file's own `Formats/Latest` (RE-80), and
  the record header carries `ElementHeader`'s tag. On every 2024 and 2025
  file measured (11, with STE1200's rows for 2014 to 2026 in Discussion
  #112) the schema's marker is the release's measured constant, so the
  constant is now confirmed from the file: a file whose schema gives
  another marker is not scanned, rather than scanned with the wrong one.
  Which releases are read is unchanged, and every 2024 and 2025 export
  is byte-identical.
- **`ROADMAP.md` and the contribution map match the milestones, the
  support matrix and the tracker.** The roadmap's position table and
  milestones were the RE-19 era's: they now carry the matrix's status for
  each area, the three releases so far (0.2.0, 0.3.0, 0.4.0) and what the
  open milestones are for. The contribution map's open remainders, dated
  2026-09-19, are the tracker's 24 open issues by milestone, with their own
  titles. Documentation only.
- **The repository is run as `docs/maintaining.md` describes.** `main`
  requires a pull request and resolved conversations (ADR-005, now applied),
  release tags `v*` are protected by a second ruleset, and both rulesets are
  kept as code in `.github/rulesets/`. CodeQL default setup is on (workflows,
  the viewer's TypeScript and JavaScript, the Python bindings), the repository
  requires full-SHA pins for actions, and there is a `SUPPORT.md`. Dependabot
  groups minor and patch updates, the bug and feature forms apply the project's
  own `type:bug` and `type:feature` labels, and CI keeps its smoke wheels for
  seven days and a pull request's evidence bundle for fourteen. The 0.4.0
  milestone is closed with its release, and the geometry slices still open moved
  to 0.4.x. Nothing in the crate, the CLIs or the viewer changes.
- **`tools/maintainer/audit.py` checks the repository against
  `docs/maintaining.md`.** It is read-only and prints one line per check: every
  open issue's labels, milestones against releases, stray branches, the Actions
  cache, the state of `main`'s workflows, open security alerts, the live
  rulesets against `.github/rulesets/`, the repository settings, the labels the
  forms and Dependabot apply, that Dependabot watches every lockfile,
  Dockerfile and composite action (the witness crate's lockfile is pinned on
  purpose), and replies owed to pull requests and discussions. The state of
  `main` is read per workflow, from its push and scheduled runs, out of the
  check suites of its last 100 commits through GraphQL: GitHub's REST runs list
  is sometimes served from a stale index (one URL returned three different
  snapshots minutes apart on 2026-09-30, the freshest six days old, and later
  two differently shaped queries agreed on one stale answer), which made the
  check report a Fuzz failure that a green scheduled run had long superseded.
  The PyPI environments now accept only release refs.
- **`tools/maintainer/clean_caches.py` deletes the Actions caches nobody can
  use again.** The caches of pull requests that are no longer open, of deleted
  branches, and the superseded generations of `main`'s rust-cache families (a
  new lockfile saves a cache under a new key and the old ones are never read
  again; the newest two are kept). It is a dry run unless given `--delete`. On
  2026-09-30 the cache stood at 9.1 GB of a 10 GB limit, and 4.9 GB of it was
  that: 0.45 GB from closed pull requests, 4.49 GB from `main`'s old
  generations.

### Fixed

- **`ElemTableHeader::record_count` is the `u32` the table states (#745).** It
  kept the low 16 bits of the count at `0x02`, on the claim that no table
  holds 65,536 records, but a Nordic LCA office project's states 72,798.
  `rvt-elem-table` then reported 7,262 declared records, and the implicit
  fallback layout parsed only that many. The field is now a `u32`, and both
  layouts take their record limit from the same count.
- **Each room is aggregated into its storey (#743).** Every `IfcSpace` was
  listed in its storey's `IfcRelContainedInSpatialStructure`, which in IFC4
  holds elements, not spatial elements, so no space had a parent in the
  spatial tree (buildingSMART rules SPS002 and SPS007) and a reader walking
  the tree found no storey for a room or anything in it. Spaces are now
  parts of their storey, or of the building where no storey is known,
  through `IfcRelAggregates` (#744, helenkwok).
- **A Level that is not a building storey may share a storey's elevation
  (#746).** The storey check dropped every Level when two shared an
  elevation, so the BIM4LCA timber houses, whose reference level sits at
  `Level_01`'s elevation, exported one storey at 0 ft. Only storeys are now
  held to distinct elevations; those files get Revit's eight storeys (#752,
  helenkwok).
- **A column whose record box starts at z = 0 takes its bottom from its
  geometry node (RE-173, #746).** Some columns' records hold a box that
  starts at exactly 0 while the column starts higher. The instance's
  geometry node holds the true z-range; where every copy of it agrees, the
  column takes that bottom (131 of 135 columns of Autodesk's 2024 MEP
  sample and all 54 of a BIM4LCA house, measured by helenkwok, #753).
- **A column that a joined slab or roof cuts is exported as its cut body
  (RE-178, #756).** The column's geometry node also holds its extent after
  geometry joins; where every copy agrees, the IFC body is the cut one while
  the placement and the column's own extent stay. 272 of 276 cut columns on
  the Autodesk 2024 samples and BIM4LCA files then equal Revit's body,
  none regressing (#757, helenkwok).
- **`--saved-meshes` reads the saved graphics of documents with a large
  increment table (RE-78).** `Global/DocumentIncrementTable` decoded under a
  100,000-value budget, which two BIM4LCA houses exceed (109,543 and
  106,289 values), so no family instance took its saved mesh there. The
  table's budget is now the larger of that and its byte length (#758,
  Persus1337; reproduced by helenkwok).
- **Snowdon Towers exports its elements again (#736).** The record
  size `elem_table::detect_layout` picks is the one at which the stated
  records fit from `0x06` with ids rising. It tested the first id, but on
  Autodesk's Snowdon Towers samples that id differs from the second on 27
  (Structural) and 84 (Architectural) records and falls back below its
  predecessor 2 and 3 times. Neither size passed, so the table was read in
  the 12-byte fallback layout. Every element record then lacked its
  ElementId (RE-30), and the export wrote 0 of Structural's 1,282 elements
  and 1 of Architectural's 6,056 (since #559). The test now reads the second
  id, the ElementId that partition records, owners and Revit's own export
  carry (RE-41). It rises over every table measured, and on Snowdon every
  owner is one of the second ids, while 13 and 42 owners are none of the
  first ids. Every other file reads the same layout as before.
  `tests/elem_table_frame.rs` checks rising ElementIds and owners on the
  second id, and reads the Snowdon samples when they are in
  `RVT_PROJECT_CORPUS_DIR` (#737, pinterbanget).
- **The viewer's wasm network-import audit can fail (VW1-21).** It grepped
  `wasm-objdump` output for `"fetch"` and the like with the quotes, which
  wabt never prints for an import (`src/binary-reader-objdump.cc:1607`), so
  it passed whatever the wasm imported. `viewer/scripts/audit-wasm-imports.mjs`
  reads each import with `WebAssembly.Module.imports()`, and the body of its
  shim in the glue, where a constructor such as `new WebSocket` shows; it
  fails on a network primitive, on an import whose shim it cannot read, and
  when it misses its own canary. The deployed viewer imports none of them,
  and the deploy no longer installs wabt.
- **Revit 2023 records read their container and Levels their Building Story.**
  These are 32-bit forms. Measured on Autodesk's 2023 sample projects, joined on
  ElementId with the 2024 copy of each:
  - A 2023 element record's container reference is the `u32` 22 bytes before
    its marker, not the `u64` at 2024's place relative to the marker.
    - The `u32` agrees with 2024's container on all 1,201, 5,858 and 8,901
      shared records of `rac_basic`, `rst_basic` and `rac_advanced`.
    - The `u64` disagrees on 9, 4,183 and 95, every container member among them.
    - The Level reader takes the same field. So `rac_basic`'s Level 800333,
      a container member, no longer fails the storey-set check, and its six
      Levels are read.
  - A 2023 Level's Building Story frame is `ff ff ff ff 00 01`, an `f64`, a
    `u32` ElementId and the setting, where 2024 writes the `u64` forms.
    Every Level of the three projects now gives the setting the 2024 copy gives it.
  - Storey agreement with the 2024 copy: 15.5 to 81.7 per cent (`rac_basic`),
    91.9 to 95.2 (`rst_basic`), 90.0 to 97.8 (`rac_advanced`). `rac_basic`'s
    one export element that 2024 does not export, a container member, is gone.
  - Exports of 2024 files are unchanged.
  - Probe: `examples/probe_32bit_container.rs` (#738, pinterbanget).
- **An export without geometry keeps its elements' types and names (B74).**
  `--mode typed-no-geometry` read the record property set, which carries an
  element's family and type, only with its geometry, so it typed no element
  and wrote no type object. It now writes the same `ObjectType` and type
  objects as the default export (RE1 and Core Interior, every typed element),
  a room's number and name, and the Level an element's record names, without
  bodies or placements. Checked by `tests/typed_no_geometry_types.rs`.
- **`rvt-write` refuses a patch it cannot write readably (B69).** Revit stores
  `Formats/Latest`, `Global/Latest` and the partitions in 65,249-byte pages
  ending in a checksum rvt-rs cannot compute; a patch reaching a full page is
  now refused instead of written unreadable, and every patched stream is
  written as a whole gzip member, with the CRC32 and ISIZE trailer Revit's
  own streams carry.
- **Export diagnostics count the Revit parameter values the export writes
  (B49).** The parameters claim and `parameter_value_count` now see the
  `Pset_` sets, which they read as zero before.
- **`probe_elem_table_ownership` takes its files from the command line (#501).**
- **The native path reads Revit 2018's schema to its end (#421, #154).**
  `schema_registry::parse` stopped at byte 351,466 of the 2018 catalog
  (`schema name byte budget`), on Autodesk's 2018 family and 2018
  `rac_basic`. A composite member that is not itself an array, over a fixed
  array of plain class references, writes those references once more after
  the nested descriptor: 2018's `SiteSurface.m_facets` (`0x500d` over
  `0x100e`), the only member of that shape in any catalog from 2016 to 2027,
  as puzzbobb found. The parser now reads them. Checked by the new real-file
  target `tests/schema_registry_catalogs.rs`: every family catalog of 2016 to
  2026 is read to its end and gives the same classes as
  `formats::schema_classes`.
- **The declared ElementIds include the ElemTable's first record (RE-140, #152,
  #421).** The frames `elem_table::parse_records` reads start 24 bytes into
  their records, so none holds the table's first record, which starts at
  `0x06`. On all 38 Autodesk sample projects and families of 2016 to 2027
  measured, the one id that record declares was missing from every reader's
  declared set although it has a record in the partitions' chain (a strict
  walk of the chain covers every id read from `0x06` on all 38, and holds no
  id the table does not declare on 37 of them; the 2021 `rac_advanced` sample
  has 6).
  Reported by puzzbobb (#421). Superseded by the entry below:
  `parse_records` now returns that record itself.
- **`Global/ElemTable` is read from `0x06`, as the table it is (#152).** After
  the `u16` tag and the `u32` record count at `0x02`, the records start at
  `0x06` and close with their owner, 28 bytes through Revit 2023 and 40 from
  2024 (STE1200's frame; puzzbobb measured it on 30 files from 2008 to 2027).
  `elem_table::parse_records` read frames from `0x1E`, 24 bytes into each
  record, so it never returned the first record, and on every 2024 and later
  file and every family it returned a last one read from the bytes after the
  table: `(0, 0)` on the 2024 to 2027 projects, `(18, 0)` on the 2016 family,
  declaring an id 0 that no record holds. On 28-byte projects it returned one
  record too few (Einhoven: 2,614 of 2,615). It now returns exactly the stated
  count, ids rising over the whole table, with each record's own owner, so a
  family's first record is owned by element 17. The record size is the one at
  which the stated records fit from `0x06` with rising ids, which also reads
  the 2008 to 2012 projects whose marker scan took an 84-byte stride (no such
  file is in the corpora here). `ElemTableLayout::start` is `0x06` and
  `marker_offset` the owner's offset (24 or 28); `ElemRecord::previous_ids` is
  always `None` (the first record is `records[0]`);
  `RECORD_EPISODE_OFFSET` and `RECORD_EPISODE_OFFSET_28` are both 8, the same
  bytes as before counted from the record's start, so every GlobalId is
  unchanged. Id 0 is an ordinary id (Einhoven's and Core Interior's
  `AllProjectPhases`). Checked by the new real-file target
  `tests/elem_table_frame.rs`.
- **The native record path keeps elements whose route finds no record
  (RE-145, #548).** `native_document::extract` takes an element's current
  record from the partition its stored revision routes to, and skips records
  elsewhere as historical. On Einhoven 6 elements route to `Partitions/1` and
  on Core Interior 10 to `Partitions/48`, partitions that hold none of their
  records, so the native path emitted 2,609 of Einhoven's 2,615 elements and
  26,415 of Core Interior's 26,425. Such an element now takes its copy in the
  latest partition that holds one, which is the copy the route itself picks
  wherever it finds one (569 of 571 differing pairs on Core Interior, 887 of
  887 on Autodesk's 2021 `rac_advanced`). The summary counts them in
  `current_records_outside_route`. Checked by the new real-file target
  `tests/native_current_records.rs`.
- **The native path walks the partitions of Revit 2016 and 2017 (#421).**
  `native_segments::walk` required a `SignatureMarker` class, which the 2016
  and 2017 schemas do not have and their partitions never write, and it took
  `m_continuationBits` to be 4 to 7, where it is `(n << 2) | flags` with `n` = 1
  from 2018 and 100 or more before (puzzbobb). The class is now required only
  where a marker is met, and only the continuation flags are read from the
  word. Checked by the new real-file target `tests/native_partitions_walk.rs`:
  every partition of every family of 2016 to 2026 walks to its end. The
  native record path still admits only 2023, 2024 and 2027.
- **Walls whose data sets a second flag after the flip (RE-126).** A
  wall's flip is one byte, and the byte after it is a second flag that is
  0 or 1. It was read as one `u32` 0 or 1, so a wall setting the flag lost
  its exterior side, its layers and its centreline body: 35 of the Revit
  2026 house's 59 walls and 4 of Core Interior's 360. On the house, layer
  sets go from 26 to 61, all as thick as Revit's with its names, and layer
  names are Revit's on 320 layers (183 before). Core Interior writes 407
  layer sets (403), all as thick as Revit's. Its wall bodies, and every
  RE1, Einhoven and Snowdon Towers output, are unchanged.
- **A type whose material map cannot be read no longer drops its doors
  and windows (RE-124, RE-84).** RE-84 exports a door or window whose type
  has no geometry-material map as its opening alone. Two cases made every
  type look map-less: a file whose material names are not read, and a map
  with unset entries (a part with no material of its own), which the 2026
  house's manufacturer doors and windows have. Neither now counts as "no
  map": without material names nothing is decided, and a map with unset
  entries shows the type draws geometry, though its materials are not
  taken as the element's (taken as its set, they gave 46 more wrong sets
  than right on Snowdon Towers). Every 2023 to 2025 file is byte-identical.
- **No invented storeys (RE-124).** A file whose Levels are not read got
  storeys anyway: Level-like strings from its partitions at elevation 0, or
  a placeholder `Level 1`. On a Revit 2026 house they were `Ground floor`,
  `Level 1` and `Roof`, none of its four Levels, and every family file had
  `Level 1` and `Roof` though a family has no Levels. Such a file now has no
  `IfcBuildingStorey` (IFC4 needs none), and its elements are contained in
  the `IfcBuilding`. Every project whose Levels are read, 2023 to 2025, is
  unchanged: Core Interior, Snowdon Towers, the RE1 models, Einhoven and
  the 255ribeiro projects keep the same storeys.

### Research

- **RE-169: a family instance's ports are its symbol's connectors (#528).**
  On RE1 Electrical, Plumbing and Mechanical every family instance's ports in
  Revit's export are its FamilySymbol's connector indices, less the cable tray
  and conduit ones. RE1 Plumbing's tank 442378, which Revit's export leaves
  out, is in New Construction, demolished in no phase, in the main model and
  unflagged; why Revit leaves it out stays open.
- **RE-168: an element record's leading reference slot is its phase (#228).**
  ElementId 3 is the `ProjectPhase` "New Construction" in Core Interior and
  RE1's MEP models; where a file's phases are 1 and 3 the leading slot is one
  of them on all but a handful of records.
- **Autodesk's MEP sample projects, 2024 to 2027, are pinned for Measure
  (B68):** ducts of every shape, fittings, open connectors and systems, the
  evidence RE1 lacks.
- **RE-163: a keyed block is listed when the family at the top of its chain is
  declared (#421)**, both ways, on all 42 files measured.
- **RE-162: an MEP system is an element of its own, and its members hold its
  id (#528).**
- **RE-161: the leading 3 of a reference list is ElementId 3 (#228)**, not a
  workset or a parameter.
- **RE-160: an element's phases are not parameter entries in its data object
  (#328)** but its native fields `m_createdPhaseId` and `m_demolishedPhaseId`.
- **RE-159: a pipe's `ConnectionType` is not stored on the pipe or its type
  (#35).**
- **RE-158: a door's and a floor's `IsExternal` is its type's Function (#35).**
- **RE-157: a pipe's inner and outer diameters are in its own curve object
  (#35).**
- **RE-156: Serial Number is a shared parameter's text entry in the element's
  own data object (#35).**
- **RE-155: an element record's reference list has no fixed slot for its type
  or Level (#228).**
- **Scorers for Measure:** `psets_vs_ifc` scores every property Revit writes,
  numbers within a relative 1e-5 and enumerated and list values too;
  `reference_property_sets` lists the sets of Revit's own export;
  `type_objects_vs_ifc` accepts an IFC4 type entity for Revit's IFC2X3 one.
- **The ElemTable's invariants hold on a 2014 family and every file of 2016 to
  2027 (RE-147, #152).** With the table read from `0x06`, all 54 files measured
  read their stated record count, their ids rise, every owner a record names
  is an id of the table, and no record owns itself or sits in an owner loop:
  the Stertil 2014 family STE1200 pointed to (28-byte records, first record
  1274 with an unset owner), the eleven Autodesk families, the 36 sample
  projects of 2016 to 2027 and the six reference models. The record size
  changes once, between 2023 and 2024. The family is pinned in
  `research/public-families.tsv` and `tools/fetch-public-families.sh` fetches
  it for a Measure run with `families` set, never stored.
  `reports/element-framing/RE-147-elemtable-on-2014-to-2027.md`.
- **The Q4 word is the grandparent's tag, and `ClassEntry::tag` is the base's
  (RE-146, #154).** On every class `formats::parse_schema` tags, on 53 files of
  2016 to 2027 (the families, Autodesk's sample projects and the six reference
  models), the tag it keeps is the base class's tag (never the class's own),
  and its `ancestor_tag` is the grandparent's, as puzzbobb reported. That
  answers RE-137's open question: the Q4 addendum looked a grandparent's tag up
  among base tags. `ClassEntry`'s documentation says so now; the readers that
  use `ClassEntry::tag` as the class's own tag (the schema-directed walker,
  `tagged_ancestor`, `rvt-analyze`) are left for a follow-up.
  `reports/element-framing/RE-146-q4-word-is-the-grandparent.md`.
- **The positions that carry the header tag and are not a record (RE-144,
  #152).** Steffen asked whether RE1 Architecture's 1,707 such positions come in
  repeated groups, as his scanner's false hits do on Core Interior. They are
  mostly single: 1,431 distinct (id, size) pairs, and 32 pairs seen three or
  more times cover 268 of them, their commonest size words being made of `0xFF`
  bytes. Core Interior's 277 do repeat (26 pairs cover 210 of them) and
  Einhoven's 20 hardly (one pair of three). The probe now prints the grouping.
  `reports/element-framing/RE-144-header-tag-false-hits.md`.
- **Where an element's copies differ, the native route takes the latest
  partition's (RE-145, #548).** On 1,518 of 1,522 differing ids across Core
  Interior, Einhoven, the RE1 models and Autodesk's 2019 and 2021 samples
  (887 of 887 on 2021 `rac_advanced`, as puzzbobb found),
  `native_index::route_episode` sends the element to its latest partition's
  copy. It also sends 6 elements of Einhoven and 10 of Core Interior to
  partitions that hold none of their records, which the native path then
  skipped (fixed in #565). `examples/probe_re145_current_copy_routing.rs`,
  `reports/element-framing/RE-145-current-copy-routing.md`.
- **The later partition's copy is the newer one on Autodesk's 2021
  `rac_advanced` sample (RE-143, #548).** The project has two partitions and
  4,037 ids with a record in both. Of the 750 whose two records carry the
  element marker and have different boxes, the 2020 release's single record of
  the element agrees with the copy in `Partitions/7` for all 750 and the 2022
  release's with the copy in `Partitions/8` for all 750 (to 0.001 ft): the
  file holds two revisions of the element, and the later partition's is the
  newer. The other multi-partition samples repeat only records without a box,
  and Core Interior's own 571 ids are not decided by this one file; #548 stays
  open. `tools/re/copy_vs_neighbours.py` repeats it from a Measure run.
- **A keyed block belongs to the loaded family whose GUID keys it (RE-148,
  #421).** On every Autodesk sample project of 2019 to 2027 the keys of the
  partition blocks after a `ContentMarker` are in `Global/ContentDocuments`
  (163 of 163 on `rac_basic`, 52 of 52 on `rst_basic`, 121 of 121 on
  `rac_advanced`, as puzzbobb found), and on every block of every file
  measured the marker's count is the block's channel-101 records less the one
  with id -1. RE1 Electrical's 60 keys of 288 are not found in the stream.
  That answers RE-135's question of which family a record after the chain
  belongs to. `native_segments::GroupSource` gains `content_element_count`.
  `reports/element-framing/RE-148-content-keys.md`.
- **Autodesk's sample projects of 2016 to 2027, and puzzbobb's measurements
  (RE-140, #421, #152, #548).** Autodesk publishes its `rac_basic`, `rst_basic`
  and `rac_advanced` sample projects for every release from 2016 to 2027, 36
  files, on the host the sample families come from; the Measure workflow's new
  `samples` input fetches them by pinned hash
  (`research/autodesk-sample-projects.tsv`, never stored). Every measurement
  puzzbobb posted on #421 that they can test reproduces: a strict walk of the
  partition chain (the header tag, and a trailer that repeats the size) holds
  a record for every id the ElemTable declares on all 47 files measured (36
  projects, 11 families), the tags are RE-133's and 2027's is 1540, the
  trailer's flag word is `0` or `0x01000000` except one `0x01001269` in each
  of the eight structural projects of 2016 to 2023, and every element marker
  of 2016 to 2023 has its 32-bit header 52 bytes before it and a well-formed
  box after. The same model read in 2016 to 2022 agrees with 2024's reading
  of it: class names on every id both hold, categories on 99.9 to 100 per
  cent and boxes on 89.6 to 100 per cent of the records with a marker (the
  box differences are likelier the model's own edits than a reading error,
  and were not examined element by element). Revit 2027's records hold the
  marker where 2024's do, 22 bytes further for each parameter entry. No
  release is admitted: there is no Revit export of any of them.
  `reports/element-framing/RE-140-sample-projects-2016-2027.md`.
- **What differs between the copies of a record in several partitions
  (RE-139, #548).** On Core Interior, of the 571 ids whose copies differ, the
  bounding boxes of 171 walls, 96 family instances, 111 rooms and 12 floors
  differ by up to a foot (the wall examples by exactly one inch): they are
  different revisions of the element, held in the main partition and one
  other (`Partitions/46` with `59` for 377 ids, with `55` for 12). The other
  models' boxes are equal or differ by less than 1e-9 ft. All 171 walls whose
  boxes differ are exported, and all 360 exported walls match Revit's faces
  and ends to 0.001 ft, but the exported body comes from the type's layers
  and the axis, so that does not say the other copy would give the same.
  Which copy is current is open. `examples/probe_re135_undeclared_records.rs`.
- **The Q4 addendum's "ancestor class" does not reproduce (RE-137, #154).**
  The reconnaissance document read the `u16` before a class's field count as
  a reference to an ancestor distinct from the parent, with nine classes
  resolving. Read with the page-stripped schema grammar (`schema_classes`), on
  17 files of 2016 to 2026, none of the nine has the base the addendum named
  (`HostObjAttr` has base `Symbol`, what the addendum called its parent), and
  every base reference of every class resolves (3,023 of 3,023 on a 2024
  file). The addendum, the README and the field documentation now say so; what
  the legacy word is stays open. `examples/probe_re137_q4_base_classes.rs`.
- **The pre-2024 envelope, the ElemTable's record lengths and the repeated
  ids (RE-136, #152, #153, #421, #548).** Steffen's `[u32 id][u32 size][u16
  tag][u16 entries]` envelope reads every Autodesk family of 2016 to 2023
  (seven releases not measured before): one record per declared id, none left
  over, every class resolved. The ElemTable's record is 28 bytes on all of 2016
  to 2023 and 40 on 2024 to 2026, on 17 files. The scan chains 99.46 per cent
  of candidates on Core Interior and 99.62 per cent on Einhoven, Steffen's
  99.5 to 99.8 per cent. The ids that open two chain records on Core Interior
  (1,444) and the RE1 models are copies of one element's record in different
  partitions, never two in one stream, always the same class; 873 of the 1,444
  are byte-identical and the rest differ (236 walls, 184 family instances, 111
  rooms), with Revit's GlobalId on all 854 exported elements regardless.
  `examples/probe_re135_undeclared_records.rs`.
- **The records after a project's chain are the loaded families' own
  documents (RE-135, #152, #421).** Steffen's reading of Einhoven's walls 2921
  and 3637, which RE-132 could not explain (Discussion #112): on six project
  files and 187,421 records after their chains, none has an id declared in
  `Global/ElemTable`, none shares an id with a chain record, and none repeats.
  Both Einhoven walls and Core Interior's three undeclared walls sit in a
  family's template views (`DBDrawing`, `Viewport`, a wall type, the `SWall`,
  `Viewer`, `DBViewSection`), the class mix is a standalone family's, and
  `Global/ContentDocuments` has 6 and 46 section headers on the two files, his
  counts. Two corrections: the chain's end, not the declaration, separates the
  two (737 of Core Interior's chain records have an undeclared id), and bit
  `0x10` of the flag word is not a host-stub marker (Core Interior's three
  walls carry `0x929`; the bit is on 1,421 of RE1 Architecture's chain records
  and on 1,177 of a standalone family's). 1,444 ids on Core Interior open two
  records in the chain, which RE-136 explains: they are copies of one
  element's record in different partitions (#548).
  `examples/probe_re135_undeclared_records.rs`.
- **A duct's or pipe's type, width and height are in its own object
  (RE-134, #96).** The fields that follow the `RbsCurveConnectorManager`
  anchor RE-131 reads a size after are the `RbsCurve` schema's own, in its
  order: width or diameter at `+8`, height at `+16`, the type's ElementId at
  `+64`. On RE1 Mechanical and Plumbing, against Revit's own IFC, that id is
  Revit's type `Tag` for all 94 ducts and pipes, which closes RE-129's
  negative (the link is in the element's object, not its record), a duct's
  width and height are two of Revit's extrusion dimensions on 25 of 25 and
  its size text is width by height in millimetres on 25 of 25. rvt-rs's duct
  bodies already have Revit's world box on 25 of 25
  (`tools/re/duct_bodies_vs_ifc.py`, now in every Measure run). The round
  versus rectangular switch, which names a duct's family, is not found: all
  25 ducts are rectangular and no redistributable model has a round or an
  oval one, so ducts keep their `Duct-<ElementId>` names.
  `examples/probe_re134_duct_connectors.rs`.
- **jakobhirn-bit's class numbers are the tags rvt-rs reads, on eleven
  releases (RE-133, #154).** His table gives ten classes (`ElementHeader`,
  `Family`, `FamilyInstance`, `FamilySymbol`, `ContentMarker`,
  `ElementParents`, `FamilyInstancePatternHelper`, `InstanceInfo`,
  `RbsPipeCurve`, `RbsCurveConnectorManager`) for Revit 2024 and 2026
  (Discussion #112): on the runners all 20 values equal the definition
  ordinals `Formats/Latest` gives. The same ten are measured on the 11
  Autodesk family files, 2016 to 2026, and each MIT reference model carries
  its release's tags. `examples/probe_re133_class_ordinals.rs`; the Measure
  workflow runs a probe on the family corpus too with `-f families=true`.
- **STE1200's `element_header` vectors reproduce (RE-132, #152, #421).**
  Steffen's eight records of Einhoven (Revit 2023) and Core Interior (Revit
  2024), each with its offset, ElementId, class, category, flags and box
  (Discussion #112): on the runners, 6 of 8 agree with what rvt-rs reads on
  every field, at the same offset on both envelopes, and his flags are the
  `u32` at `+0x46` that RE-128 reads. The other two, Einhoven's walls 2921
  and 3637, are not declared in `Global/ElemTable`, and rvt-rs does not read
  them. `examples/probe_re132_slik_vectors.rs`.
- **Revit 2026 element records (RE-124, #421).** On the public flowbim.ee
  house (Revit 2026, with Revit's own IFC4X3 export), the records carry the
  marker RE-32 predicted (`61 01 ff ff ff ff f1 05`, from the file's own
  schema), and the 2024/2025 decode with the RE-21 instance rule finds 88
  placed instances, every one in Revit's export, of every class there.
  Revit 2026 is not admitted yet: in a trial, doors and windows came out as
  bare openings, storeys were not the model's, and rooms were unnamed.
  `examples/probe_re124_revit_2026_records.rs` and
  `tools/re/instances_vs_ifc_tags.py` measure it.

## [0.4.0] — 2026-09-28

Revit 2023 projects now export typed elements, as 2024 and 2025 projects
do: storeys, types, layers, materials, GlobalIds, wall joins and beam cuts,
measured against Revit's own exports of two 2023 projects. On every
supported release, rooms take their real outline and Revit's number and
name, doors and windows cut the opening their type specifies, turned
family instances are drawn turned, steel members carry their I section,
and every typed element is related to an IFC type object of its Revit
type. Every figure is measured against Revit's own export of the same
model; the reports under `reports/element-framing/` hold the inputs.

### Upgrading from 0.3

- **`IfcSpace.Name` is the room number** (RE-117). It was
  `Room-<ElementId>`. The room name stays in `LongName`, and the ElementId,
  which an `IfcSpace` has no `Tag` for, is the new `ElementId` property.
  Anything that read a room's ElementId from its IFC `Name` reads that
  property instead. A room without a number keeps the `Room-<ElementId>`
  Name; one without a name now has no `LongName` (it repeated the Name).
  glTF node names, the viewer and the room schedule are unchanged.
- **Only building-story Levels are storeys** (RE-118). A Level whose
  Building Story setting is off is no longer written as an
  `IfcBuildingStorey`, as in Revit's export. Of the models measured, only
  Snowdon Towers' structural model changes: its Top of Footing and TOS
  Levels are not storeys, and its 12 building stories now are.
- **System family names follow the file's saved locale** (RE-123). On a
  file saved as `PTB`, walls, floors and roofs are `Parede básica`, `Piso`
  and `Telhado básico` in `Name`, `ObjectType` and material layer set
  names, where 0.3 wrote `Basic Wall`, `Floor` and `Basic Roof`. Files
  saved in other locales are unchanged.
- **`ElemRecord::owner_id` values change** (#152). Each record now reports
  its own owner; 0.3 reported the next record's. This reaches
  `rvt-elem-table --json` and Python's `elem_table_records()`. The IFC
  export does not use it.
- **Rust API:** public structs gained fields, among them
  `ifc::IfcModel::element_type_ids` and
  `partition_level_records::PartitionLevel::building_story`. Code that
  builds them with struct literals needs the new fields or
  `..Default::default()` where the type has one.
- **Element bodies and IFC entity counts change** wherever geometry is
  newly decoded (openings, turned instances, rooms, sketched outlines,
  steel sections) and where type objects are added. GlobalIds of existing
  elements, rooms and storeys are unchanged.

### Added

#### Revit 2023 projects

- **Revit 2023 projects export their elements (RE-81, #421).** Walls, doors,
  windows, floors, roofs, columns, beams, foundations, rooms and the other
  recovered categories come from their 2023 element records, typed, under
  Revit's own ElementIds, each drawn as its bounding box. Against Revit's
  IFC4 exports of two 2023 projects: 37 of 37 elements and 37 of 45, with
  none outside either export; the 8 missing are window trims nested in the
  windows, left out because Revit keeps some nested components (frames,
  cuts) and drops others (mullion patterns) by a rule not yet decoded.
  Names, types, storeys, joins, opening hosts, design options, parameters
  and IFC export overrides are not read for 2023, and the export
  diagnostics say so.

- **Revit 2023 element records are located (RE-81, #421).** A 2023 record
  sits behind the schema-derived marker with its ElementId at marker -52 and
  its category at -38; the instance rule finds every element of Revit's own
  export of two 2023 projects. A probe and a report; the exporter does not
  read 2023 records yet.

- **Revit 2023 Levels and storeys (RE-107, #421).** Revit 2023 projects
  export with their real Levels as storeys, and each element on the
  storey of the Level its record names, by the same rules as 2024's. The
  elevation marker's `u16` is the schema's `Plane` tag on every release.
  A Level's newest partition holds its current name and elevation.
  - Exemplo_data: 2 of 2 storeys and 37 of 37 elements as Revit's.
  - modelo_bim: 4 of 4 storeys and 29 of 37 elements as Revit's.
  - Every 2024 and 2025 file is byte-identical.

- **Revit 2023 elements take Revit's own GlobalIds (RE-108, #421).** A
  2023 `Global/ElemTable` record holds the element's creating episode at
  `+0x0c`, where 2024's 40-byte record has it at `+0x18`. Every element,
  room and storey rvt-rs exports from the two 2023 projects with a Revit
  export now carries the GlobalId Revit gives it: Exemplo_data 37, 9 and
  2, modelo_bim 37 and 4 (none before). Every 2024 and 2025 file is
  byte-identical.

- **Revit 2023 family instances carry their family and type names
  (RE-109, #421).** A 2023 name entry is `01 00 00 00 · u32 id · u32 n ·
  UTF-16`, with no category after it as 2024's has. An instance's type is
  the one same-category type record its list names that has a name, and
  its family is the one named id its type's record references with no
  record of its own. Every door, window, column, beam and footing of the
  two 2023 projects with a Revit export now has Revit's own Name and
  ObjectType: 19 of 19 on Exemplo_data and 31 of 31 on modelo_bim.
  System-family types (walls, floors, roofs) are named since RE-111. Every
  2024 and 2025 file is byte-identical.

- **Revit 2023 walls, floors and roofs carry their type (RE-111, #421).**
  A 2023 system-family type has no record and no name entry, but its
  element data opens with `ff ff ff ff c0 02 01 00 00 00` and a `u32`
  ElementId and names it as 2024's does. Of the named ids an element's
  record names, its type is the one named by records of the fewest
  categories, a strict subset of every other's. Every element Revit's
  export types on the two 2023 projects now carries Revit's type name:
  Exemplo_data 37 of 37 and modelo_bim 37 of 37 (19 and 31 before). The
  floors are typed by an `IfcSlabType` with Revit's `Tag` and GlobalId;
  walls and roofs keep their category's name, since their system family
  follows from their type's layers, which 2023 does not read. Every 2024
  and 2025 file is byte-identical.

- **Revit 2023 wall, floor and roof types read their layers, and walls and
  roofs are typed (RE-112, #421).** A 2023 host type's layers follow its
  name as on 2024, framed by `ff ff ff ff 6f 10` on a wall type, in 29-byte
  records: width, function, material and deck profile as `u32` ids. All
  five host types of the two 2023 projects read Revit's own layer widths,
  their materials named as Revit names them (the concrete masonry, the
  cast-in-place concrete and the metal deck). With layers read, walls are
  Basic Walls and roofs Basic Roofs (RE-63), so every element Revit's
  export types is now typed on both 2023 projects: Exemplo_data 37 of 37
  and modelo_bim 37 of 37, each with Revit's type `Tag`, and the walls,
  floors and roofs with Revit's type GlobalId. A structural deck layer
  may have no width, as a membrane may. Every 2024 and 2025 file is
  byte-identical.

- **Revit 2023 family instances carry their materials (RE-113, #421,
  #355).** A 2023 family type's value block is `[owner u32][28 x ff][3 x
  00]` and its first map `u32 n · n × (u32 key · u32 material)` names the
  materials its geometry uses, as RE-82's does on 2024. A 2023 material is
  found by its class tag `0x09fb` 0x27 bytes past its `u32` id, and named
  by its element data or, inside a family's data, by a `-1001203`
  parameter entry. 2023 doors, windows, columns and beams are now written
  with their type's `IfcMaterialConstituentSet`, where no 2023 element had
  a material before: Revit's own set on 17 of Exemplo_data's 18 and all 27
  of modelo_bim's. A type whose map names a material with no name read gets
  none. Every 2024 and 2025 file is byte-identical.

- **Revit 2023 walls and floors carry their material layer sets (RE-114,
  #421).** A 2023 wall's data holds its location line and its orientation
  as 2024's does, read now through one element-data layout per release (a
  `u32` id on 2023). 2023 walls, floors and roofs take RE-53's and RE-57's
  layers, named by RE-113's materials, and are written with an
  `IfcMaterialLayerSetUsage`: modelo_bim's 4 walls with Revit's layer and
  thickness, and its floor with its concrete layer (Revit also writes the
  zero-width metal deck). Walls keep their record body, since 2023 wall
  joins are not read, and layers that take their category's material stay
  unnamed on 2023. Every 2024 and 2025 file is byte-identical.

- **Revit 2023 object styles give each category its material (RE-115,
  #421).** A 2023 category's object-styles entry is `i32 category · ff ff
  ff ff · u32 1|2 · u32 1 · ff ff ff ff 3f 01 · 8 bytes · i32 -3000010 ·
  u32 material`. Walls', floors' and roofs' layers that take their
  category's material take that one, as RE-91's wall layers do on 2024,
  and floors', roofs' and ceilings' now do so on every release. Exemplo_data's
  17 walls and its floor carry Revit's own `Material pré-definido de
  parede` and `Material pré-definido de piso`: 35 of its 37 elements now
  have Revit's material set (18 before). Every 2024 and 2025 file is
  byte-identical.

- **Revit 2023 materials carry their shading colours and names (RE-116,
  #421, #355).** A 2023 material keeps its transparency, 0.5, four
  eight-byte pattern slots (`u32` id, `u32` COLORREF), its shading colour
  and its shininess, where 2024's slots are four bytes, and ends its name
  before `ff ff ff ff eb 0b` or in its own name field, as RE-58's 2024
  names do. The 2023 export's material list is now the file's own
  materials, named and coloured (modelo_bim 108, Exemplo_data 114,
  Einhoven 36), instead of 146, 155 and 42 partition display strings; their
  colours and transparency are
  Revit's on 20 of the 21 materials Revit's exports style, none different,
  and Exemplo_data's roller door gains Revit's material (36 of its 37
  elements with Revit's material set). The GLB draws 2023 elements in
  them. Every 2024 and 2025 file is byte-identical.

- **Revit 2023 doors and windows cut their host wall (#421, RE-85).** The
  2023 reference lists carry the host as 2024's do. On the two 2023
  projects measured, all 30 doors and windows take the wall Revit's filled
  opening voids (none had a host before), and 15 of the 30 openings equal
  Revit's within 0.01 ft.

- **Revit 2023 rooms take their outline too (RE-102).** Revit 2023 stores
  the room's solid as 2024 does, with four `ff` bytes before its tag. All
  9 of Exemplo_data's rooms now match Revit's own area and outline (6
  before). `plan_profiles_vs_ifc.py --match centroid` pairs rooms by
  position where their GlobalIds are not Revit's. Every other local file
  is byte-identical.

- **Revit 2023 beams sit on the storey of the Level at their top (RE-119,
  #421).** A structural framing element's record does not give the Level
  Revit's export puts it on: four beams landed on the storey below it and
  four foundation beams on no storey. Revit's export puts each beam on the
  Level at its top: four beams spanning 3.2 to 4.0 m on the 4.0 m Level,
  four foundation beams spanning -0.8 to 0.0 m on the 0.0 m Level. A 2023
  beam whose record box's top is exactly one Level's elevation is now
  bound to that Level, and modelo_bim has all 37 of its elements in
  Revit's storey (29 before). Revit 2024 and 2025 are unchanged; the rule
  is unmeasured there.

- **Revit 2023 walls are cut back by the walls they join (RE-120, #421).**
  A 2023 wall's record box is the untrimmed wall, as on 2024: at an L or
  T joint it runs to the other wall's centreline, where Revit's export
  stops it at that wall's face. A 2023 record's reference list names the
  walls it joins, as 2024's does, so 2024's join solver (RE-26, RE-29)
  runs on the 2023 records unchanged. Exemplo_data's 17 walls and
  modelo_bim's 4 now have Revit's box exactly (2 of 17 and 4 of 4
  before). Einhoven, with no export, has 2 walls trimmed.

- **Revit 2023 beams stop at the columns they frame into (RE-122,
  #421).** A 2023 beam's record box runs to the centres of its end
  columns; Revit's export stops it at their faces. The beam's reference
  list names the column each end lies in, so an end inside exactly one
  named column is cut back to that column's near face. modelo_bim's 8
  beams now have Revit's box exactly (0 before). On Revit 2024 the same
  columns are named (Snowdon Towers' structural model), but most steel
  beams stop a stored join cutback short of the face, and when Revit
  applies it is not decoded, so 2024 beams are unchanged; the report
  records where the cutbacks are stored.

#### Every supported release

- **Elements are typed by IFC type objects of their Revit type (RE-110).**
  Every element rvt-rs names `Family:Type` and whose type it reads is now
  typed through `IfcRelDefinesByType` by one IFC4 type object per Revit
  type (`IfcWallType`, `IfcDoorType`, `IfcColumnType` and the rest), as
  Revit's own export types it. Each type object is named `Family:Type`,
  with the type's ElementId as its `Tag` and the GlobalId the type's
  UniqueId gives. Before, no type object was written. Measured per `Tag`
  against Revit's exports: Core Interior 854 of 854 elements typed with
  Revit's type name, `Tag` and entity; Snowdon Towers 5,803 of 5,812;
  RE1 Architecture 72 of 73; on Revit 2023, every family instance of
  Exemplo_data (19) and modelo_bim (31). Revit's exporter often writes
  several type objects for one type, such as one per column, with
  GlobalIds of its own, so type GlobalIds are Revit's on 438 of Core
  Interior's 854. Railings, openings and spaces are not typed, nor are
  2023 walls and roofs, whose system family is not read. Both Core
  Interior witness observations are refreshed.

- **Rooms carry their numbers and names on every release (RE-117, #90,
  #421).** A room's number and name are the parameter entries `-1006901
  (ROOM_NUMBER) · u32 n · UTF-16` and `-1006900 (ROOM_NAME) · u32 m ·
  UTF-16` just past its data object, with ids as wide as the release's
  ElementIds. Read from there, every room carries Revit's number and name:
  Core Interior 116 of 116 (unchanged), Snowdon Towers 54 of 54 against
  its VIM export (RE-29's parameter block named 1, wrongly), RE1
  Architecture 11 of 11 and Exemplo_data 9 of 9 (neither named before).
  RE-29's block still gives a room's Level, and its number and name where
  no entries are read.

- **Rooms take their real outline (RE-101, #90).** A room's partition
  stores the room's solid as planar faces, and the walls among them close
  into its outline, columns and shafts left out as voids. Rooms export
  with that outline instead of their bounding box:
  - Core Interior: all 116 at Revit's own area and outline (76 before).
  - RE1 Architecture: all 11 keep theirs.
  - Snowdon Towers: 45 of 54 at the area its VIM export gives (10 before).
    The 7 whose solids do not close at the floor keep their box.

  `rvt-schedule --schedule rooms` gains an area column, filled for rooms
  whose outline was read. `plan_profiles_vs_ifc.py` also measures each
  outline against the other file's, as Revit splits edges at every
  bounding element. Every other local file is byte-identical.

- **Family instances carry their materials (RE-82, #355).** A family
  type's parameters hold a map of the materials its geometry is drawn in;
  each instance now exports them as an `IfcMaterialConstituentSet`, the way
  Revit's own export writes doors, windows, furniture and fixtures. Elements
  whose materials equal Revit's: Snowdon Towers 2,533 (from 1,202), Core
  Interior 220 (from 88), RE1 Architecture 58 (from 15). The differences are
  families with a nested component (counter tops with an appliance, Core
  Interior's windows) and planting Revit writes as `<Unnamed>`.

- **Family instances are drawn in their material's colour (#355).** In the
  GLB and so the browser viewer, a family instance whose type draws in
  exactly one material (RE-82) takes that material's colour when the colour
  was read, instead of its category's. Checked end to end against Revit's
  own IFC4 styles: all 290 such instances on Snowdon Towers and all 38 on
  RE1 Architecture render Revit's material and colour; Core Interior has
  none. Instances drawn in several materials keep their category's colour.

- **Walls whose layer takes its category's material get it (#355,
  RE-91).** The document's object styles hold each category's material.
  Where the Walls entry is set, a wall layer that takes its category's
  material takes that one. On Core Interior all 360 walls now carry
  Revit's material ("Default Wall" on 356, none before): 315 as a
  one-layer layer set and 45 as the material itself. Its layer sets go
  from 88 to 403, all on Revit's bodies within 0.001 ft. Snowdon Towers
  and the MIT house leave the entry unset and are unchanged; so are RE1
  and Einhoven.

- **Walls their data does not place carry their type's materials (#355,
  RE-88).** A wall with no orientation, line or arc in its data still
  takes its type's layers for its materials, with no exterior side, so it
  is neither drawn in layers nor given a layer set. On Snowdon Towers 95
  of the 111 walls that had no material now have Revit's association (72
  before), none a different one. On Core Interior 4 walls get Revit's
  "Concrete"; its other 356 walls' layers take their category's material,
  which Revit writes "Default Wall" and rvt-rs does not read.

- **Walls whose body is not a layer set carry their layers' materials
  (#355, RE-88).** Revit's export associates a wall's materials by its
  type's layers: one layer gives its `IfcMaterial`, several an
  `IfcMaterialConstituentSet` of the layers, exterior first, a material's
  k-th occurrence named " (k)". That is Revit's association on every
  Snowdon wall rvt-rs writes a layer set for (143 of 143, 823 of 824).
  Tapered, curved, cut and profiled walls, which got no material, now get
  it where their layers are read: on Snowdon Towers 72 of the 111, all
  equal to Revit's (57 constituent sets, 15 materials), none different.
  In the GLB a wall with one material takes its colour. Every other local
  file exports byte-identical IFC.

- **Steel beams are drawn as their I section (RE-103, #94).** A framing
  type stores its section's width, depth, web and flange thickness and
  its centroid, and a centroid at the centre both ways marks an I, not a
  channel. 377 of Snowdon Towers' structural beams now export as an
  `IfcIShapeProfileDef` along their line (none before), placed on the
  box's bottom where the beam is set down from its line. On all 271 of
  them its VIM export can score, Revit's own mesh is that I in that place.
  Every other local file is byte-identical.

- **Steel columns are drawn as their I section (RE-104, #94).** A
  structural column's flanges run along its instance's X axis. All 40 of
  Snowdon Towers' steel columns now export as their type's
  `IfcIShapeProfileDef` turned to it, where the turned section fills the
  record box. On all 24 its VIM export can score, Revit's own mesh is that
  I at that turn.
  - The type-value readers now take a block's owner from every ElementId
    `Global/ElemTable` declares, secondary ids included (RE-41). That reads
    W8x31's section, and changes no exported value elsewhere.
  - Every other local file is byte-identical.

- **Steel members carry their section as a material profile (RE-105,
  #94).** The 417 beams and columns drawn as their I section are written
  with an `IfcMaterialProfileSetUsage`, one set per type pairing its
  material with that I, instead of a constituent set.
  - `IfcMaterialProfile` now writes the real profile where the model has
    one, rather than a 1 × 1 m rectangle stand-in.
  - `rvt-schedule` names these members' material (Steel).
  - The GLB and every other local file are byte-identical.

- **Windows cut Revit's opening, from their type (#227, RE-93).** A
  type's length parameters are read from its value block. Revit's export
  cuts a window's opening its type's Width by Height, centred on the
  window's origin, from its type's Default Sill Height above it. A window
  whose type stores all three takes that opening where its box holds it,
  and its `OverallWidth` / `OverallHeight`. On Snowdon Towers 56 of 68
  windows now match Revit's opening within 0.001 ft (none did); the
  Double-Hung windows, whose types store no Default Sill Height, keep
  their box opening. Every other local file is byte-identical.

- **Doors Revit cuts at their rough size get that opening (#227, RE-94).**
  Revit's export cuts a door at its type's Rough Width by Rough Height
  exactly where the door's body is Rough Height tall. Such a door now takes
  that opening, centred on its origin, and its `OverallWidth` /
  `OverallHeight`. On Snowdon Towers 57 of 126 doors: 54 now match
  Revit's opening within 0.001 ft (none did), and the 3 in tapered walls
  match along the wall and in height. Doors cut to their frame, and
  opening-only doors, keep their box opening. Every other local file is
  byte-identical.

- **Slab outlines with curved edges (RE-96).** A sketch line's curve
  can be an arc, stored in the layout a curved wall's is (RE-75) but
  unmarked, so the line's record box decides whether it reads as an arc.
  Arcs are drawn as chords within 0.0005 ft, and an outline with an arc is
  kept only where it spans its element's record box. On Snowdon Towers 17
  more slabs carry their outline (149 of 199), and 87 match Revit's area
  within 0.1% (73 before). Snowdon Towers structural gains one; every
  other local file is byte-identical.

- **Edited sketches take their lines from the sketch itself (RE-97).** A
  Sketch element names the element it sketches and lists its curves. Where
  an edit left a line's owner reference naming another element, the list
  is now the sketch's membership, and such an outline is kept only where
  it spans its element's record box. On Snowdon Towers 176 of 199 slabs
  and all 20 roofs carry their outline (149 and 13 before); 102 slabs
  match Revit's area within 0.1% (89 before) and 10 roofs (7 before).
  Snowdon Towers structural gains three; every other local file is
  byte-identical.

- **Ceilings carry their sketch's outline (RE-98).** Ceilings take the
  outline their sketch closes, as roofs do, instead of their record box.
  66 of Snowdon Towers' 68 ceilings (50 before) and all 6 of RE1
  Architecture's (5 before) now match Revit's outline within 0.1% of its
  area. `plan_profiles_vs_ifc.py` measures each element's top surface, so
  Revit's stacked layer solids count once.

- **Shaft openings cut the floors and roofs they pass through (RE-99).**
  A shaft opening's sketch is read, leaving out the diagonals Revit draws
  across it in plan, and its outline becomes a void in each one-piece
  floor, roof or ceiling within its height that holds it strictly inside.
  On Snowdon Towers 7 elements take a shaft void, all closer to Revit's
  area, two of them now exact. A shaft on an element's edge is not cut
  yet. Every other local file is byte-identical.

- **Round shafts (RE-100).** A full-circle sketch line is read, so a round
  shaft opening cuts its void too. On Snowdon Towers four more slabs match
  Revit's area within 0.1% (154), and a roof becomes exact. Every other
  local file is byte-identical.

- **Core Interior's 20 shading-device plates carry their outline (#233,
  RE-95).** Each owns one zero-length sketch line: a record whose box is a
  point, with no line of its own, on an end of two of the others. It is
  now left out, and the other lines' recorded ends close each plate into
  Revit's outline exactly (none did). Two slabs in Snowdon Towers
  structural and one in the MIT house (2024) close the same way; no Revit
  export of those files is held. Every other local file is byte-identical.

- **Stair runs ending with a tread, and monolithic runs, are drawn as
  their steps (#357, RE-92).** A run's data holds Revit's "End with
  Riser" setting. It is off on exactly the three Snowdon Towers runs that
  have one riser line more than risers, so those are drawn with their
  last tread running to the last riser line. Monolithic runs are drawn as
  one cast body, its underside the type's structural depth below the
  steps and cut off by the floor. 38 of Snowdon's 43 flights are drawn
  as their steps (34 before), and the 4 added equal Revit's own geometry
  but for one nosing edge. Every other local file is byte-identical.

- **Held-out validation (#408).** Autodesk's Snowdon Towers Plumbing, HVAC
  and Electrical samples, never used to develop the decoder, were exported
  once and compared with the VIM export of the sample
  (`tools/re/held_out_vs_vim.py`,
  `reports/validation/held-out-snowdon-mep-2026-09-27.md`): 10,385
  exported elements in the VIM, every one in the expected category; every
  instance both hold exported; 8,766 storeys and 10,385 GlobalIds equal.

- **Research: a door's record box does not give its body (RE-121,
  negative, #227).** The box reaches past Revit's door body across the
  wall on one side, by up to 0.9 m. Reflecting its near side about the
  wall's centreline gives Revit's body exactly where the body is
  centred: Core Interior 132 of 132, RE1 4 of 4, the 2023 projects 9 of
  11. On Snowdon Towers, whose door frames sit at a wall face, it gives
  0 of 126, and nothing read tells the cases apart, so doors keep their
  record box. Windows fail the same test (modelo_bim 10 of 10, Core Interior 0 of 6, Snowdon 0 of 68). `tools/re/door_bodies_vs_ifc.py` measures both.

### Changed

- **`IfcSpace.Name` is the room number, as in Revit's export (RE-117).**
  A space was named `Room-<ElementId>`, with the room name in
  `LongName`. Revit's exporter writes the number in `Name` and the name
  in `LongName`, and now so does rvt-rs; the ElementId, which an
  `IfcSpace` has no `Tag` for, is the new `ElementId` property. Every
  room's `Name` and `LongName` equal Revit's on Core Interior (116),
  RE1 Architecture (11) and Exemplo_data (9), and Snowdon Towers' 54
  against its VIM export. A room without a number keeps the
  `Room-<ElementId>` Name, and one without a name has no `LongName`
  (it repeated the `Name` before). GlobalIds are unchanged. **Upgrade:**
  anything that read a room's ElementId out of its IFC `Name` reads the
  `ElementId` property instead. glTF node names, the viewer and the
  room schedule are unchanged.

- **Only a Level that is a building story becomes a storey (RE-118,
  #219).** A Level keeps Revit's Building Story setting past its name: the
  first `ff × 8 · 00 01`, an `f64`, a `u64` ElementId, then 1 or 0. It is
  exact against the "Building Story" parameter of Snowdon Towers' VIM
  export on all 37 architectural and structural Levels, and 1 on Core
  Interior's and RE1's, each a storey of Revit's IFC. A pad byte of 1
  before the name length no longer refuses a Level. Snowdon Towers'
  structural model, whose Levels were refused, now has its 12 building
  stories as storeys, their names and elevations the VIM's, with 208 of
  the 210 elements on them in their storey; its Top of Footing and TOS
  Levels are not storeys. Every other model's IFC is byte-identical.

- **System families are named in the file's saved locale (RE-123).**
  A system family's name is not stored in a Revit file: Revit's export
  writes it in the language the file was saved in. On the four files
  saved as `PTB` (Exemplo_data and modelo_bim, 2023; Projeto1 and
  teste_export_2025, 2025) Revit names walls `Parede básica`, floors
  `Piso` and roofs `Telhado básico`, where rvt-rs wrote `Basic Wall`,
  `Floor` and `Basic Roof`. Elements and material layer sets now carry
  those names on `PTB` files: 32 of 32 elements' family names as Revit's
  (0 before). Other families and locales keep the English name;
  `ENU` files are byte-identical.

- **Turned family instances are drawn turned (RE-87).** A family
  instance's data holds its transform, a 3 x 3 rotation stored row by row
  and its origin. A door, window, piece of furniture or other family
  instance turned off the model's axes is now the rectangle at its angle
  whose box is its record box, and a door's or window's opening turns with
  it (`BodySource` `partition_family_instance_turned_box`). On Snowdon
  Towers all 181 such instances are closer to Revit's body along their
  own axes (median 1.47 ft off before, 0.48 ft now; 55 within 0.01 ft,
  none before), and the openings of the 27 turned doors and windows are
  Revit's width along them (none before). Instances square to the model's
  axes, and Core Interior, RE1, Einhoven and the MIT house, are unchanged.

- **Fixtures hosted on ceilings and walls are turned too (#227, RE-90).**
  RE-87 turned only instances whose Z axis is the model's. One with any
  vertical axis takes its plan direction from its first flat axis, which
  agrees with Revit's placement on every such family instance on Snowdon
  Towers. 58 more instances are drawn turned (44 proxies, 14 light
  fixtures): 44 closer to Revit's body, none further. Every other local
  file is byte-identical.

- **Turned and tilted curtain mullions and panels are drawn along their
  own axes (RE-106).** The box along an instance's transform axes whose
  axis-aligned box is its record box follows from three linear equations.
  1,019 of Snowdon Towers' mullions and panels turned or tilted off the
  model's axes now export as that box instead of their record box. On
  every one, Revit's own mesh has its extents and centre along those axes
  (to 0.0000 ft). Every other local file is byte-identical.

- **Tapered walls are drawn with their leaning face (#358, RE-86).** A
  wall type stores three face angles after a fixed frame, and a wall
  whose orientation word is 2 is tapered by them: its exterior face leans
  by the type's angle, wider at the base, its interior face is vertical,
  and at the top it is its type's thickness about its line. Each such wall
  is now that cross-section extruded along its line (`BodySource`
  `partition_wall_tapered_section`) instead of its record box. On Snowdon
  Towers, 19 of the 20 walls Revit tapers match its body to 0.001 ft across
  the wall at five heights and the 20th to 0.052 ft (on main none was
  within 0.1 ft, the worst 9.3 ft off). In the GLB, 24 walls score better
  against Revit's bodies and none worse. Every other local file exports
  byte-identical IFC. Slanted walls (word 0) keep their box.

- **Windows in tapered walls get Revit's opening (#227, RE-89).** Revit
  cuts a window's opening in a tapered wall as a vertical box the host
  type's thickness deep, centred on the window's own origin. rvt-rs cut
  none there (the wall's base is wider than RE-83's limit); now all 20
  such windows on Snowdon Towers have Revit's opening across the wall
  (0.77 and 0.13 ft off before). A turned family instance reports its
  origin (`InstanceOriginX`, `InstanceOriginY`). Doors, which Revit cuts
  flush with the exterior face, keep their box.

- **Openings are as deep as the wall they cut (#227, RE-83).** A door's or
  window's `IfcOpeningElement` took the element's whole box, standing up to
  4.9 ft out of the wall. Its depth now comes from the host wall's plan
  outline, measured along the element's own axes; width and height stay the
  element's box. Against Revit's export
  (`tools/re/openings_vs_ifc.py`): Core Interior 138 of 138 within 0.25 ft
  (none before), Snowdon Towers 72 of 110 within 0.26 ft (none before). In
  walls that are not straight extrusions the opening stays the element's
  box. Three Snowdon hosts differ from Revit's (#439); the docs no longer say
  every host pairs.

- **Doors and windows whose type draws no geometry export as their opening
  (#309, RE-84).** Families that only cut their host ("Door-Opening",
  "Schematic Opening Cut", "CasedOpening") have a type value block with no
  geometry-material map. Revit's export writes each as an
  `IfcOpeningElement` voiding the host, Tagged with the element's
  ElementId, and no door or window; rvt-rs now does the same wherever it
  binds the host. On Snowdon Towers, Core Interior and RE1 Architecture,
  48 of 48 such elements and 343 of 343 others follow the rule in Revit's
  export; 36 Snowdon and 1 RE1 openings are now written this way with
  Revit's Tag and host. The 11 whose host is not bound yet stay doors and
  windows (#439). Openings are not drawn in the GLB or plan SVG and are not
  counted as building elements in the export diagnostics.

- **Doors and windows find their host wall anywhere in their reference
  list (#439, RE-85).** RE-23 took the slot just before the record's own
  ElementId. The list is in ascending ElementId order, so a host with a
  larger id than its door comes after it, and families that list their
  type put it between. The host is now the nearest exported wall the list
  names, below the record's own id first, skipping curtain walls. On
  Snowdon Towers, 192 of the 194 doors and windows Revit fills an opening
  for take its host (107 before; the other 2 take another wall Revit also
  cuts), none takes a wall Revit does not cut, and the 11 opening-cut doors
  and windows left over by RE-84 are now openings. Core Interior and RE1
  are unchanged.

- **The viewer is built with pnpm.** `viewer/package.json` pins
  `packageManager` to pnpm 12.6.0 and `viewer/pnpm-lock.yaml` replaces
  `package-lock.json`, imported from it with the same resolved versions.
  CI, the viewer deploy, the release workflow, `tools/check-local.sh
  --viewer`, the Cursor Cloud installer and the docs install with
  `pnpm install --frozen-lockfile`. The dependency audit runs
  `pnpm audit --audit-level high` with the same retry and NOT MEASURED
  handling.

- **CI and the local gate no longer run unit tests.** Unit tests are banned
  in this repository. `tools/ci/test-targets.txt` classes every
  `tests/*.rs` target as real-file, CLI, contract or unit, and
  `tools/ci/verify-real-files.sh` runs all but the unit ones (failing on an
  unclassified target). The CI build job (now `build + real files`), the
  tier-two corpus job and `tools/check-local.sh` use it; the library,
  binary and doc unit suites, the synthetic IFC and tier-one jobs and the
  pytest suite no longer run, and the Python wheel job smoke-tests the
  installed wheel on Revit files instead. The test sources are kept.

- **The install guide and status page describe v0.3.0 as released** (#405),
  each channel checked from its published artifacts.

- **`rvt` 0.3.0 is on crates.io and docs.rs** (#406). The install guide
  documents `cargo install rvt --locked` and links the API documentation.

### Fixed

- **`ElemRecord::owner_id` names the owner of the right record (#152,
  RE-31 §6).** `Global/ElemTable`'s records start at `0x06` and end with
  their owner field, so the field that opens the frame rvt-rs parses
  closes the record before it; `owner_id` paired every owner with the
  next element. Now the element's own partition reference list names the
  owner on 21,072 of 21,074 records on Core Interior (86.6 % before) and
  1,574 of 1,574 on Snowdon Towers (92.4 %), and no record names itself
  (304 and 1,342 did). Reported, with the self-ownership test that
  settles it, by STE1200. `rvt-elem-table` and Python's
  `elem_table_records()` report the corrected owners; nothing in the IFC
  export used the field.

- **The element schedule names family instances' materials.**
  `rvt-schedule`'s material column now falls back to an element's material
  constituent set (RE-82, RE-88), listing each material once in the IFC's
  order, and matches the material the IFC associates. Before, it was empty
  for every element written with a constituent set. Rows with a material:
  - Core Interior 448 → 586;
  - Snowdon Towers 1,273 → 2,703, structural 482 → 1,138;
  - RE1 Architecture 15 → 58.

  No other column changes.

## [0.3.0] — 2026-09-27

Trustworthy Revit 2024 and 2025 export: generated GlobalIds stable between
exports, the SI units the values are written in, the whole schema, each
element's Revit class and type parameters, MEP devices, and a count of the
bodies that are still boxes.

### Changed

- **The viewer shows the version it was built from** (#405), under the drop
  zone, from its package version, which now follows the crate's (it was
  0.1.0).
- **Three per-release constants are class tags in the file's own schema
  (RE-80).** The number opening `Global/ElemTable` is the tag of the class
  `ElemTable`; the element-record marker is the tags of `Outline` and
  `ElementParents` with four `0xFF` bytes between them; the record-chain
  header constant is the tag of `ElementHeader`. Measured on every release
  from 2016 to 2026, which shows the marker a reader needs for any release
  comes from the schema, not a table. Documentation and a probe; the export
  is unchanged. Credit: jakobhirn-bit (Discussion #112).

- **The crate's own documentation says what it does today.** The docs.rs
  front page had called IFC export a scaffold and the schema "100% classified";
  it now states the supported profile and shows three runnable entry points
  (read a file's identity, list a project's elements with the export
  diagnostics, write IFC4 and glTF). The crates.io description and
  `documentation` link (docs.rs) are updated to match (#406).

- **GlobalIds rvt-rs generates no longer shift when the model changes
  (#400).** Entities Revit's export gives no GlobalId (openings, the
  further pieces of a slab, site, building, relationships, property sets)
  were numbered by their position in the STEP file, so drawing one more
  profile point renumbered everything after it: RE-75's curved walls alone
  changed the GlobalIds of all 147 openings and 6 slabs on Snowdon Towers,
  and 12,409 of the file's 20,149 GlobalIds. Each is now a SHA-256 of a
  fixed namespace, the document's `Unique Document GUID` and the entity's
  own identity (an element's ElementId and which piece of it it is, an
  opening's filling element, a relationship's participants), in IFC's
  22-character form. Exported before and after RE-75, Snowdon Towers now
  keeps all 20,149. Revit's own GlobalIds are unchanged. **One-time
  change:** every generated GlobalId differs from the previous release's
  (on Snowdon Towers, the 155 products without a Revit GlobalId and every
  relationship and property set), so anything keyed by those needs one
  re-import. The `0rvtrs` prefix is gone.
- **IFC length properties no longer end in "Feet".** Their values were
  always `IfcLengthMeasure` / `IfcPositiveLengthMeasure` in the file's
  length unit, metres, so a name ending in "Feet" misread them by a factor
  of 3.28. `ThicknessFeet` is now `Thickness`, `JoinReachStartFeet` /
  `JoinReachEndFeet` are `JoinReachStart` / `JoinReachEnd`, `JoinTrimStartFeet`
  / `JoinTrimEndFeet` are `JoinTrimStart` / `JoinTrimEnd`, `AxisLengthFeet`,
  `SectionWidthFeet` and `SectionDepthFeet` are `AxisLength`, `SectionWidth`
  and `SectionDepth`, and `TypeSectionWidthFeet` / `TypeSectionDepthFeet` are
  `TypeSectionWidth` / `TypeSectionDepth`. The values are unchanged. Anything
  that reads these properties by name needs the new names.
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

- **Exports say how much of the model is a stand-in box (#409).** The
  diagnostics sidecar counts exported bodies by how they were derived
  (`body_sources`) and how many are only the element record's bounding box
  (`bounding_box_bodies`); `rvt-ifc` prints it and the viewer's file status
  has a Shapes row. Core Interior: 274 of 970 (its doors, windows, rooms and
  rotated plates); Snowdon Towers: 4,770 of 6,081; RE1 Architecture: 75 of
  85. Each element's own `BodySource` property already said so; now the
  total does too.
- **Electrical and mechanical devices and equipment (RE-79, #96).** Six more
  categories export from their element records: lighting devices (switches,
  sensors), fire alarm devices (as `IfcAlarm`), data devices (as
  `IfcElectricAppliance`), electrical fixtures (receptacles, junction boxes),
  electrical equipment (panels) and mechanical equipment, the last four as
  proxies, following Revit's own export. A symbol nested in a switch or
  receptacle is an element record of its own that Revit's export leaves out;
  it and its parent reference each other and it has the larger ElementId,
  so rvt-rs leaves it out too (21 of 21). RE1 Electrical now exports every
  element Revit's export holds, 49, and nothing else (it was 12), each with
  Revit's GlobalId; RE1 Mechanical adds its air handler. Electrical
  equipment, which Revit names by its Panel Name, is named
  `ElectricalEquipment-<ElementId>` until that parameter is read. Core
  Interior and Snowdon Towers are unchanged.
- **Every element carries its Revit class (RE-76, #154, #223).** The schema
  is now read by its grammar, which gives each class its serialization tag:
  its definition ordinal in the file's own `Formats/Latest`. An element
  record names its class by that tag at `+0x4a`, so each record-backed
  element's property set gets `RevitClass`: `SWall`, `ArcWall` and
  `FaceWall` for walls, `Floor`, `RoomElem`, `ProfileRoof`, `StairsElement`,
  `FamilyInstance` and so on. Every exported element on Core Interior,
  RE1 Architecture and Snowdon Towers resolves (970, 85 and 6,081), and
  Snowdon's 32 `ArcWall` records are exactly the 32 walls RE-75 found
  storing an arc. `formats::schema_classes` and `RevitFile::schema_classes`
  expose the classes. Credit: STE1200 and jakobhirn-bit (Discussion #112),
  rosejn (#255).
- **Type Mark, Description and Fire Rating (RE-77, #35).** Each element
  now carries its type's Type Mark, Description and Fire Rating, read from
  the type's parameter value block. On Snowdon Towers every value rvt-rs
  writes agrees with Revit's: Type Mark on 1,257 of the 1,276 elements the
  VIM export can check (the other 19 are empty in that later edition),
  Description 70 of 70, and Fire Rating 696 of 696 against Revit's own
  IFC4 export; RE1 Architecture's 7 fire ratings too. Instance parameters
  and numeric values are not read yet.
- **A reference model pack (#407).** `docs/reference-models.md` lists every
  model rvt-rs's claims are measured on, with its licence, Revit release, how
  its reference export was made and what a run should show, and explains the
  export-setting differences (view definition, schema, coordinates) that are
  not decoding errors. The twelve redistributable files (magnetar Core
  Interior and Einhoven, the four MIT RE1 models and their exports) are
  recorded with their origin URLs in `research/witness-registry.json`, and
  `tools/fetch-reference-models.sh` downloads them and checks every hash.
  Files with no licence stay local only and are listed, never fetched. The
  page also gives five tiny models anyone with Revit can make for open
  questions (hip and gable roofs, wall joins, rooms, openings), and how to
  report a problem with a model you cannot share.

- **Curved walls are drawn along their arc (RE-75, #358).** A wall's data
  stores its location arc in the record rvt-rs read as a line, and the word
  before the record says which it is. rvt-rs read an arc's axes as a line,
  rejected it and drew the wall's box. Each curved wall is now the ring
  sector its arc and its type's thickness make. On Snowdon Towers the 24
  curved walls Revit draws with an arc axis have Revit's centre and radius
  within 0.001 ft, and their faces are Revit's on 23 of them (0 before).
  Their footprints overlap Revit's by a median 0.95, from 0.03. Their ends
  are the arc's own, not yet trimmed at joins, and are all within 0.625 ft
  of Revit's.
- **Walls that meet at an angle end on slanted lines, as Revit draws them
  (RE-74, #358).** RE-70's butt joins, RE-71's layered corners and RE-73's
  T joints now apply where two walls meet at 45 to 135 degrees, not only
  at right angles, and each layer ends where its two edges meet the other
  wall's face or layer boundary. On Snowdon Towers 34 of 102 angled walls
  have both of Revit's ends, from 18, and 689 of 976 axis-parallel walls,
  from 675. Every angled L join drawn is right along every layer edge, and
  18 of 22 angled T joints. One wall gets worse: its core now ends where
  Revit's does, but Revit runs its interior finish past the end. At sharp
  corners and shallow bends, outside 45 to 135 degrees, Revit draws
  something else, so those joints keep their ends.
- **Plumbing fixtures whose record names no Level sit on Revit's storey
  (RE-68, #369).** They take the Level the objects their record names
  carry, as Revit's export does, even where their base elevation points
  elsewhere: Snowdon Towers' floor sinks sit 0.57 ft below that Level and
  its faucet 3.2 ft above. All 4 are now on Revit's storey. On Snowdon,
  5,949 elements are on Revit's storey, 1 is on no storey and 1 on another.
  Other classes still need that Level to agree with their base, because for
  light fixtures, slab edges and wall sweeps it alone is wrong on 73 of 214.
- **A wall that ends part way along another stops against it layer by layer,
  as Revit draws T joints (RE-73, #358).** The other wall runs through.
  Counted from the face it meets, each layer of the stopping wall passes the
  other wall's layers of lower priority, so a core passes a finish, and
  stops at the first of equal or higher priority; a single-layer wall stops
  at the face. On Snowdon Towers 280 of 313 measured T ends are right in
  every layer, 675 of 976 axis-parallel walls now have both of Revit's ends,
  from 516, and 4,290 of 5,634 layer ends match Revit's body, from 3,925.
  Each such end names the wall it stops against in the IFC and the viewer's
  element panel, 202 more on Core Interior, whose bodies already stopped
  there. Revit draws some T joints as a clean stop at the face instead,
  which nothing read predicts, and 13 Snowdon walls lose an end they had.
  RE1's export (Revit 2025) writes each wall as one rectangle, so no layered
  end shows in it: against it 3 of 7 walls keep both ends, from 6.
- **Every curtain panel and mullion sits in the curtain wall Revit puts it
  in (RE-72, #370).** A panel names the curtain grid it lies on, and the
  grid's own data names its curtain wall: that settles the panels that name
  several walls and a nested curtain wall that took the wrong parent. Panels
  that hold a basic wall export as `IfcCurtainWall` with their own body, as
  Revit exports them. On Snowdon Towers all 2,075 curtain parts Revit
  aggregates now have Revit's whole (16 had none and one another), and all
  60 curtain walls match Revit's, bodiless or not.
- **Layered walls end layer by layer at their corners, as Revit draws them
  (RE-71, #358).** At a butt join between two layered walls, the layers
  nest like concentric Ls: counted from the outside of the corner, each
  layer of the wall that runs through reaches the outer edge of the other
  wall's matching layer, and each layer of the wall that stops, its inner
  edge. Where both walls show the same layers in the same order and share
  their base and top, the wall is drawn as that staircase, as an
  `IfcArbitraryClosedProfileDef` and one GLB band per layer. On Snowdon
  Towers 516 of 976 axis-parallel walls now have both of Revit's ends, from
  346, and 3,925 of 5,634 layer ends match Revit's body, from 3,078. Revit
  draws a few such joints as a clean butt instead, which nothing read
  predicts: 16 layer ends on Snowdon get worse. So does one RE1 wall's
  end, but RE1's export writes each wall as one rectangle and cannot show
  layered ends (RE-73).
- **A butt join's wall runs through, and the other stops, as Revit draws
  them (RE-70, #358).** A wall's own data lists the walls it runs through
  at a join; where two walls' centrelines end at one point, that wall
  reaches the other's far face and the other stops at its near face. The
  lists never contradict a joint Revit's own export draws cleanly: 212 of
  212 decided ends on Snowdon Towers, and 1,110 axis-parallel ends against
  the record box on five 2024 and 2025 files. Walls take these ends where
  both have a single layer, since Revit cleans layered joins layer by
  layer. On Snowdon Towers 9 walls improve and none gets worse, and 346 of
  976 axis-parallel walls have both of Revit's ends, from 338. The IFC
  reports each such end as `JoinReachStart` / `JoinReachEnd`.
- **A wall names the walls it butt-joins.** Every end the join lists
  decide, layered walls included, carries `JoinStartWallElementId` /
  `JoinEndWallElementId` and `JoinStartRunsThrough` / `JoinEndRunsThrough`
  in the IFC: 185 of Core Interior's 360 walls. The viewer's element panel
  lists them as "Start: runs through" or "End: stops at" with the other
  wall, a link that selects it.
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
  - The property set gains `AxisLength`, `SectionWidth` and
    `SectionDepth`, and `BodySource` reads `partition_beam_axis`.
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

- **The IFC header names the rvt-rs version that wrote it.** `FILE_NAME`'s
  originating system and `IfcApplication.Version` said `0.1.x` whatever the
  version; they now carry the crate's version.

- **The IFC's units are the units its values are written in (#403).** The
  writer writes every length in metres, but a model with no recovered units
  (every synthetic fixture and family file, and any `IfcModel` a caller
  builds) declared millimetres, so IFC viewers drew it 1,000 times too small:
  IfcOpenShell read the synthetic fixture's 20 ft wall as 6 mm long. Real
  project exports declared Revit's display units for area, volume, angle and
  mass (square inches, cubic inches, degrees and pounds on Snowdon Towers;
  "centi square metres" on Projeto1) over values written in square metres,
  cubic metres, radians and kilograms. No value in those exports used them
  yet, so their geometry was unaffected. The unit assignment is now always
  metre, square metre, cubic metre, radian, kilogram and second; Revit's
  display units stay in the export diagnostics
  (`recovered_unit_identifiers`), which describe the source model.
- **The whole embedded schema is read (#410).** `Formats/Latest` is stored in
  checksum pages like the database streams, but it was inflated without the
  page strip, so it drifted after about 140 KB while still decoding without
  an error, and schema parsing stopped at 64 KB to keep the drift out. With
  the strip, every file of one Revit release inflates to the byte-identical
  schema, family or project, and rvt-rs reads all of it: 4,126 classes and
  11,562 fields on Revit 2024 instead of 395 and 1,114, and 122,107 fields
  across the 11 sample releases instead of 13,570. 9 to 12 field encodings
  per release are not classified yet and the coverage gate lists them. The
  earlier "100% field classification" covered the first 64 KB only. No IFC
  output changes: exports of Core Interior, the four RE1 models, Einhoven and
  Snowdon Towers are identical. RE-20's note that `Level` is absent from the
  schema was an artifact of the 64 KB scan. Credit: STE1200's checksum-page
  finding and schema grammar (Discussion #112).

- **Empty curtain panels are left out, as Revit's export leaves them out
  (#309).** A curtain panel of the "Empty System Panel" family fills a grid
  cell with nothing, and Revit never exports one; rvt-rs wrote each as a
  plate box. On Snowdon Towers the 24 are gone and counted in the export
  diagnostics as `element_record_empty_curtain_panel`. The family is
  recognised by its English name, so files saved in another language keep
  them. The panel type's unset material is checked as well: an empty-family
  type with a material is reported.
- The viewer's "Not ready" row no longer lists empty curtain panels among
  the elements rvt-rs exports and Revit's export leaves out. It names the
  ones that remain: opening-cut families and curtain panels with no
  resolved type.
- The viewer's Parameters status row says in plain words that Revit's
  parameter table is not read yet (#35), instead of "AProperty* host joins
  pending".
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
