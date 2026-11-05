- **Measure scores what the MCP server answers (#1327).** A scripted client
  with no language model (`tools/re/mcp_client.py`) starts the built
  `rvt-rs-mcp`, opens each model Measure exports and asks a fixed query set:
  the summary, the levels, every element's row, the totals per IFC entity,
  Revit class and level, the rooms with their areas, `get_element` for up to
  five elements of each entity, the diagnostics and an `export_ifc`.
  `tools/re/mcp_answers_vs_ifc.py` checks the answers against the same run's
  IFC export (the elements and spaces by GlobalId with their entity, name, Tag,
  predefined type and storey; the storeys and the elements each holds; the
  rooms' number, name, storey and plan area; the property and quantity sets,
  trust markers, type object, placement and host of the sampled elements; the
  diagnostics against `diagnostics.json`; the server's export against
  `ifc.sha256`), where a difference ends its output with `exit 1`, and against
  Revit's export with the pairing and tolerances of the class sets, storeys,
  space profiles, property sets and quantities scorers. It is run by name
  (`-f scorers=mcp_answers_vs_ifc`) or with the rest of the suite, builds the
  server only then, keeps the lines starting `mcp answers:` of the held-out
  models, and writes each model's answers as a deterministic dump
  (`mcp-dump.json`, sorted, floats at six decimals, no handle or path) for the
  semantic merge check (#1334). Measured on the v0.14.0 binaries on Einhoven,
  Core Interior and the four RE1 models: the protocol, the counts, the levels,
  the rooms' areas and the sampled elements' property sets, trust markers,
  placement and host equal the export; three disagreements are reported (see
  the pull request). Not yet measured on this head or in Measure.
