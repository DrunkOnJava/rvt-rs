- **A wall whose floor covers all of its plan but a stub inside another wall is
  cut whole, as Revit cuts it (#1067, RE-375).** The BIM4LCA office's wall
  3004978 as built, whose roof leaves a 0.0853 ft stub of its end inside the
  wall it abuts, kept its full height: 9.24 ft over Revit's top and 14.94 ft³
  of body over. It is now cut at the roof's underside over its whole plan, as
  Revit's body is. It carries `TopCoverStub` (`inside_another_wall`) and is
  counted in `exported.wall_top_stubs_cut`. The rule rests on that one wall:
  what a floor leaves of a wall must be under 0.05 ft² and wholly inside the
  plans of other walls, and of the 225 pieces of Revit's wall plans outside a
  floor that RE-375 reads in five buildings the stub is the only one that is,
  while Revit keeps the wall's own height over most others.
