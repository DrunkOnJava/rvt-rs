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
  offline after it has loaded does all three. The first page load is heavier by
  the size of that script (not yet measured on this head). A read has taken 0.74 to 1.04
  times the open's wasm memory, rising with the file, and nothing above 910 MiB
  has been measured, so a selection is read on its own only while the wasm
  memory the page holds is at most 1 GiB; above that the panel asks first,
  saying that clicking may close the tab, and after a read has run out of
  memory, or its worker has gone without a word, that file is read only when
  asked. The page keeps each element's answer, up to 64 elements with the least
  recently selected going first, until a file is opened again, so an element
  selected a second time is not read again. Not yet measured on this head: all
  figures.
- **File Status's Parameters row stops saying the rest are still to be
  decoded (#35).** It said so while `rvt-elements` and Python already listed
  every element's saved parameters (RE-207). It now counts the values written
  to the IFC property sets and points to the info panel. The panel's "Not
  recovered" note for an element with no property set now names "IFC property
  sets", not "Revit parameters", which it lists itself.
