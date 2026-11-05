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
  hash were equal on every model. The two server faults are fixed in #1406;
  on its head the scorer passes 6 of 6 reference models and 19 of 24 Revit
  2025 models (runs 41951524664, 41951554180). The scorer's later changes
  (this round's report guard, verdicts and sampling) are measured by the
  pull request's own run on its final head.
