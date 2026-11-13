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
  `work_budget_reached` skipped item counting each reader's stops, a
  `Work budget reached:` warning per reader. The export is still written:
  `rvt-ifc` and `rvt-gltf` print a warning naming each reader, Python's
  `write_ifc` gives a `RuntimeWarning`, and the MCP server's exports list
  it in their notes. The memo of element-data header positions is held under 64 MiB, and the memo
  of inflated partitions under 16 GiB. No real file of the measured corpus
  changes.
