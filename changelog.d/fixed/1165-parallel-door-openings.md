- **A door that also cuts a second wall running along its own now gets that
  opening, where the two walls span the same heights (#1165, RE-471).** Revit's
  export cuts it ten feet deep across the two walls, as wide and as tall as the
  door's own opening, centred on the face of the door's own opening on the side
  the door's local Y points away from. rvt-rs counted these steps as
  `door_second_wall_parallel_to_door_wall` and wrote nothing. It now writes the
  opening where the second wall's base and top are the door's own wall's: on the
  BIM4LCA models Revit writes the opening in every such wall (23 of 23) and not
  in the one second wall of other heights. Steps whose opening would stand above
  or below the second wall, whose second wall spans other heights, or whose door
  faces along its walls still write nothing and are counted.
