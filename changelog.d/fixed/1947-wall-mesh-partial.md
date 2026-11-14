- **A wall's quantities are no longer read from a saved mesh that decoded only
  in part (#1947).** A beam's and a column's mesh were already refused when any
  of their solids failed to decode. A wall's was read whenever what did decode
  was one closed piece, which a wall of two solids with one undecoded would be.
  No measured model holds such a wall: 61 of 61 exports are unchanged.
