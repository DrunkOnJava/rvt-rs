- **A wall whose saved mesh decoded only in part takes no quantities from it
  (#1947).** A beam's and a column's mesh were already refused when any of their
  solids failed to decode, but a wall's was read whenever what did decode was one
  closed piece, so a wall of two solids could take one solid's volume and areas.
