- **Measure scores what the MCP server answers (#1327).** A scripted client
  with no language model (`tools/re/mcp_client.py`) starts the built
  `rvt-rs-mcp`, opens each model Measure exports and asks a fixed query set:
  the summary, the levels, every element's row, the totals per IFC entity,
  Revit class and level, the rooms with their areas, `get_element` for up to
  five elements of each entity, the diagnostics and an `export_ifc`.
  `tools/re/mcp_answers_vs_ifc.py` checks the answers against the same run's
  IFC export (the elements and spaces by GlobalId with their entity, Name,
  LongName, Tag, predefined type and storey; the storeys and the elements each
  holds; the rooms' number, name, storey and plan area; the property and
  quantity sets, trust markers, type object, placement and host of the sampled
  elements; the diagnostics against `diagnostics.json`; the server's export
  against `ifc.sha256`), where a difference ends its output with `exit 1`,
  and against Revit's export with the pairing and tolerances of the class
  sets, storeys, space profiles, property sets and quantities scorers. It is
  run by name (`-f scorers=mcp_answers_vs_ifc`) or with the rest of the suite,
  builds the server only then, keeps the lines starting `mcp answers:` of the
  held-out models, and writes each model's answers as a deterministic dump
  (`mcp-dump.json`, sorted, floats at six decimals, no handle or path) for the
  semantic merge check (#1334). Measure runs 41949014871 (Einhoven, Core
  Interior, RE1), 41949018446 (the BIM4LCA office as built and at permit),
  41949021964 (the 24 Revit 2025 models) and 41949025326 (the held-out, spent,
  BIMcollab and rac_advanced models), all of head 1895e771 with `base=none`,
  found the head server equal to the export in the protocol, the counts, the
  rooms' areas, the sampled elements' property and quantity sets, trust
  markers, bodies, placements and hosts, the diagnostics and the export hash
  (the office's 13,021 rows took 44 s), and not equal in four things; 57 of
  the 58 models fail, and only a blank Revit 2025 project passes. A space's
  `Name` in `list_elements` and `get_element` is the model's
  `Room-<ElementId>` where the IFC has the room number (or `<name>:<ElementId>`).
  `element_type` lacks the `Other` and `Identity Data` sets the writer gives
  every type object (all 6 reference models, 23 of the 24 Revit 2025 models,
  the office) and the type objects made for rooms, and names a type for
  elements the writer gives none (Einhoven's four ArcWalls). Parts of a stair
  or railing have no level where the IFC puts them on their whole's storey (5
  of the Revit 2025 models, 4 of the held-out ones, and 599 of the office's
  12,974 rows, whose levels' counts differ in 8 of 10). The office lists 47
  `IfcCovering` rows (158, against the export's 111) that the export does
  not write. The checks added after those runs (a space's long name, an
  absent placeholder name, the type object's entity and name) are not yet
  measured on a run.
