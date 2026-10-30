# Changelog

All notable changes will be documented here. This project follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/) and
[semver](https://semver.org/spec/v2.0.0.html). Changes on `main` since the
last release are also in [`changelog.d/`](changelog.d/README.md), as
fragments a release moves into this file.

## [Unreleased]

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
