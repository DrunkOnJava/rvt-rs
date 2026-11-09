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
