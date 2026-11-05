- **A roof drawn as its record box no longer cuts the top of the walls under it,
  unless the box is exactly the roof's layers (#1471, RE-351).** Autodesk's
  `rac_basic` draws 16 walls up to their record boxes on its 2025 save and cut
  at 5.7265 m on its 2026 save, up to 3.1865 m lower, from the same record box,
  plan, joins and top-join element (Measure run 42063046730 at 3149973f, #1454's
  exports, read with a script; not yet measured on this head). The element is
  roof 243274, a roof by extrusion: the 2025 reader draws it as a solid, which
  the wall-top pass leaves out, and the 2026 reader, which reads no roof by
  extrusion, draws it as its record box, which the pass took for a roof with a
  level underside though the box's underside is its lowest point. The pass now
  takes a roof only where `attach_slab_layers` gave it its layers, which it does
  where they add up to the box's height; a slab or footing is cut at as before.
  Not yet measured on this head: which walls of the other releases and models
  change.
