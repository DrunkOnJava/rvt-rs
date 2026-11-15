- **A wall end is no longer drawn through a wall the end rule does not read
  (#1583).** A wall with an edited elevation profile now keeps the end its
  record gives where such a wall, one the record does not join there, stands
  where the end would be drawn. It is marked `unread_wall_stands_in_the_added_end`.
  That covers a wall drawn as a solid other than its plan's prism, one whose base
  join names elements, and one whose ElementId several entities hold. Before
  this, the end was drawn through it with no marker and no count. The wall is
  found from the plan and the heights of the body the export draws for it, and
  one whose body cannot be bounded is taken to stand there (RE-337).
