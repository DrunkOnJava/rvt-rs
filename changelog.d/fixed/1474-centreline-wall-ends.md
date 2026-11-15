- **The wall end rule now reads walls drawn along their location line
  (#1474).** A wall with an edited elevation profile, drawn along its location
  line (`partition_wall_centreline`), stops short of or reaches past its line's
  end by the join reach. The end rule matched each end to the line's end through
  a record box's join trim, which such a wall does not have, so it never read
  their ends and marked nothing. They now take the reach as their trim. The BIM4LCA
  office's walls 2056705 and 2056770 as built are the case the issue names. Not
  yet measured on this head.
