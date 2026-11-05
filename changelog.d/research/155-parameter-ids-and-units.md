- **mahach666's findings on the parameter store, its ids and units, and on
  sketch to solid (RvtIO, Discussion #1241) are split by what they rest on,
  and the counts they asked for are answered (#155, #156, RE-344).** Of their
  findings, the record shapes of the typed sets and family slots, the
  `-1001800` and `-1001801` extrusion extents, the sketch-to-loops stages and
  the stored bounding box were already measured (RE-207, RE-327, RE-32), and
  RE-310 had reached their result on the two millimetre conversions
  independently; the unit factors beyond the converted specs, the host areas
  and the volume rules rest on the Revit API and are recorded as reported, not
  measured. `probe_re344_parameter_ids_and_units` counts the sign rule of
  parameter ids, six conversions of the stored millimetre lengths on Revit
  2015 to 2027, the extents of a family's forms and the GUIDs in a family's
  `PartAtom`; its figures are not yet measured on this head. Against the
  counts in RvtIO's table, rvt-rs's `main` (5988d839, Measure run 42028541274)
  differs on `rac_basic` by 5 to 11 building elements (3 Furniture Systems, a
  Roof, a Mass, and from 2023 on 4 mullions and 2 curtain panels), on
  `rst_basic` by the 346 reinforcing bars and 16 element assemblies it does
  not read, and on `rac_advanced` by 192 curtain panels, a door, an assembly
  and 2 Masses (and 25 Areas on the 2021 save); `rac_basic`'s 14 rooms come
  back with #1435.
