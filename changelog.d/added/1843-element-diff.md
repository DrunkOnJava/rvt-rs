- **`rvt-diff --elements` compares two saves of a model element by element
  (#1843).** Each file is exported as `rvt-ifc` exports it, and the elements
  are keyed by the GlobalId the export writes. It lists the elements added,
  removed and changed, and for each changed one the fields that differ:
  class, name, storey, type, predefined type, material, where the body is
  placed, a digest of the body, and each property the export writes. `--json`
  writes the whole comparison (`docs/schemas/rvt-diff-elements.schema.json`),
  and `--redact` scrubs names and text values. The library is
  `rvt::ifc::element_diff`. What rvt-rs does not read is not compared, and a
  change in how rvt-rs reads a file shows as a change, so compare two saves
  with one build. Suggested by HWT-25 (#1829), who compared two saves of a
  Revit 2027 model by hand.
