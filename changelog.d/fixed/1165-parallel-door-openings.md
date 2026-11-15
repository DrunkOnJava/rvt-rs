- **A door that also cuts a second wall running along its own now gets that
  opening, where the two walls span the same heights (#1165, RE-471).** Revit's
  export cuts it ten feet deep across the two walls, as wide and as tall as the
  door's own opening, centred on the face of the door's own opening on the side
  the door's local Y points away from. rvt-rs counted these steps as
  `door_second_wall_parallel_to_door_wall` and wrote nothing. It now writes the
  opening where the second wall's base and top are the door's own wall's: of the
  BIM4LCA models' openings that lie within their wall's height, Revit writes
  every one in a wall of the door's wall's heights (23 of 23) and not the one in
  a wall of other heights. Of the openings that stand above or below a second
  wall of other heights, Revit writes 42 on the BIM4LCA models and 1 on Snowdon
  Towers Architectural but leaves out 4 and 3, and nothing read tells them apart,
  so those steps still write nothing and are counted, as are doors that face
  along their walls.
