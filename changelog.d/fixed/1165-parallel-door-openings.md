- **A door that also cuts a second wall running along its own now gets that
  opening (#1165, RE-471).** Revit's export cuts it ten feet deep across the two
  walls, as wide and as tall as the door's own opening, centred on the face of
  the door's own opening on the side the door's local Y points away from. That
  holds on all 82 such openings of the BIM4LCA models and all 24 of Snowdon Towers
  Architectural. rvt-rs counted these steps as
  `door_second_wall_parallel_to_door_wall` and wrote nothing. It now writes them,
  and keeps the reason for a door whose Y runs along its wall. Not yet measured
  on this head.
