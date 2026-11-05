- **A probe reads which way the sketch lines of a floor's void run and whether
  that tells which line Revit tags the void with (RE-352, #1408).**
  `probe_re352_void_loop_directions` reads the loop of every unfilled opening in
  Revit's export whose host is not a wall, twice: from the order its lines were
  drawn in (their ElementIds) and from the direction each line is stored in. It
  prints the line each reading says Revit tags, beside the tag, for the void
  3147725 of the BIM4LCA office, the one `void_tag` gets wrong, and for every
  other void. The export is unchanged. Not yet measured on this head.
