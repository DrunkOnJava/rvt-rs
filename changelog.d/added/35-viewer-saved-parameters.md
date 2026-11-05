- **The viewer's info panel lists an element's saved parameters (#35, RE-207).**
  Selecting an element shows, under "Saved parameters", the same entries
  `rvt-elements` and Python's `decoded_elements()` list for it: instance
  parameters, then the type parameters it takes from its type, each with its
  name, its value as saved and its unit where the entry has one (`3 ft · 0.9144 m`,
  `Yes`, a material's or family type's name beside its ElementId). A built-in
  parameter has no name in the file, so it is listed by its id behind a fold
  that says why (#255), and what the read could not list (family slots no
  definition resolves, an instance parameter whose value is its family's
  default) is counted in a line. The panel says so when the file is of a
  release outside Revit 2015 to 2027, when the element has no saved
  parameters, when the element has no ElementId in the export and when a read
  failed, with "Try again"; it shows that it is reading while it does.
  `viewer/tests/parameters-row.spec.ts` holds the entries the panel shows
  for an element to the ones `rvt-elements` prints for the same ElementId,
  whole, on Einhoven and Core Interior. It is a new binding, `savedParameters`
  (`docs/viewer-api.md`), and it does not read at the open: the export, and so
  every IFC, GLB and diagnostics output, is unchanged. An element is read when
  it is selected, in a worker of its own that reads the `File` the page
  already holds and ends with its answer, so nothing of the file stays
  resident beyond what the page held before, the open's worker is not held up
  by a read, and a read that fails leaves the rest of the panel where it
  was. A file whose estimated memory, with what the open left, passes the
  ceiling the page already judges an open by is read when asked, with a
  button. Measured in the viewer tests (run 42103334445, head f56b671a, CI
  runner, WASM without LTO): on Einhoven every one of the 37 tree elements
  was selected and the panel held, for the 28 with parameters, exactly the
  369 entries `rvt-elements` prints (332 instance, 37 type, 8 named), and
  showed the empty state for the 7 with none; a read took a median of 532 ms
  (most 740 ms) in an instance of 17 MiB, against 23 MiB for the open's
  worker. On Core Interior four elements of four categories match
  `rvt-elements` too; a read took 2.9 to 3.7 s in 275 MiB, against 326 MiB
  and 29.0 s for the open. On the BIM4LCA office (62 MiB) a selection read on
  its own, with no button: 8 entries in 7.5 s in 943 MiB, against 910 MiB and
  126.5 s for the open. A read thus takes about what the open's worker did
  (0.74 to 1.04 times it) while it runs, and the ten-times-size estimate the
  page gates on understates it on the office (616 MiB estimated). That office
  is not compared with `rvt-elements`.
- **File Status's Parameters row stops saying the rest are still to be
  decoded (#35).** It said so while `rvt-elements` and Python already listed
  every element's saved parameters (RE-207). It now counts the values written
  to the IFC property sets and points to the info panel. The panel's "Not
  recovered" note for an element with no property set now names "IFC property
  sets", not "Revit parameters", which it lists itself.
