- **A roof drawn as its record box no longer cuts the top of the walls under it,
  unless the box is exactly the roof's layers (#1471, RE-351).** Autodesk's
  `rac_basic` draws 16 walls up to their record boxes on its 2025 save and cut
  at 5.7265 m on its 2026 and 2027 saves (15 of them on 2024), up to 3.1865 m
  lower, from the same record box, base and top Levels, joins and top-join
  element on all 13 of its saves (Measure run 42106436497 at b2b5716e). The
  element is roof 243274, a roof by extrusion on every save: the 2025 reader
  draws it as a solid, which the wall-top pass leaves out, and the other
  readers, which read no roof by extrusion, draw it as its record box, which
  the pass took for a roof with a level underside though the box's underside is
  its lowest point. The pass now takes a roof only where `attach_slab_layers`
  gave it its layers, which it does where they add up to the box's height; a
  slab or footing is cut at as before. Not yet measured on this head: the walls
  after the change, and which walls of the other models change.
