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
