- **Value sources for roofs drawn from their slopes or from an extrusion
  (#1368).** Under `--element-provenance` a roof drawn from its slopes lists
  the slope block of each sketch line the slope was made from
  (`roof_slope_edges`: the defining edges, or the slope arrow and the boundary
  lines it rises from), those lines' own data (`roof_line_data`), its fascia
  depth and rafter cut where they were read (`roof_fascia_depth`,
  `roof_rafter_cut`) and the layers of its type that give its thickness
  (`roof_type_layers`), beside the outline and the box it already listed. A
  roof drawn from an extrusion lists the first frame of each of its sketch
  lines (`sketch_lines`), their data and its type's layers. Each owner map is
  built where the roof reader already resolves its sketch. CI's Corpus health
  now fetches puzzbobb's Revit 2025 models so that
  `tests/value_sources_offsets.rs` reads these sources and every other part
  against the models whose roofs are drawn this way. Area boundary lines are
  the one body still without a source (family extrusions have theirs from the
  entry for them). Nothing is noted unless the table is asked for, so the
  default export and its diagnostics are unchanged.
