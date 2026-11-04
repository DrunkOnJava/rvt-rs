- **Value sources for room outlines, column sections and pipe axes (#1368).**
  Under `--element-provenance` a room whose outline is read from its stored
  solid lists the header of each solid that gave it (`room_solid_faces`,
  `parts`, in the room's own partition), where it had only the frame for its
  box; a column drawn as its type's hollow or I section lists the type's
  parameter entries it took the section from (`type_hollow_section`,
  `type_i_section`: the width, height, wall and corner radius, or the I
  section's six values, each the first copy of an entry every copy of which
  holds the same value); and a pipe drawn from its connector entries lists
  the two entries that are its ends (`pipe_connectors`). The readers still
  without a source are listed in `docs/export-diagnostics.md`. These sources add 1.2% to
  the BIM4LCA office's table (6,131,584 to 6,202,130 bytes of compact JSON) and
  2.0% to Core Interior's (718,828 to 733,326; Measure run 41890994176).
  Nothing is noted unless the table is asked for, so the default export and its
  diagnostics are unchanged.
