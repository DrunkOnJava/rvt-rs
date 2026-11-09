- **A roof drawn as its record box no longer cuts the top of the walls under it,
  unless the box is exactly the roof's layers and its outline is read (#1471,
  RE-351).** Autodesk's `rac_basic` draws 16 walls up to their record boxes on
  its 2025 save and cut at 5.7265 m on its 2026 and 2027 saves (15 of them on
  2024), up to 3.1865 m lower, from the same record box, base and top Levels,
  joins and top-join element on all 13 of its saves (Measure run 42106436497 at
  b2b5716e). The element is roof 243274, a roof by extrusion on every save: the
  2025 reader draws it as a solid, which the wall-top pass leaves out, and the
  other readers, which read no roof by extrusion, draw it as its record box,
  which the pass took for a roof with a level underside though the box's
  underside is its lowest point. The pass now takes a roof only where
  `attach_slab_layers` gave it its layers, which it does where they add up to
  the box's height (the proof that the underside is level), and where its plan
  outline is read (an L-shaped roof drawn over its box's rectangle would cover a
  wall it does not reach); a slab or footing is cut at as before. **This is a
  smaller error, not Revit's body:** Revit's walls attach to the roof's sloped
  underside, and the box-height prism restored here is still not it (the second
  read of #1476 estimates about 5.1 m3 over across the 16 against about 13.5 m3
  under with the flat cut; not reproduced). Wall 506386 loses the five values
  the roof's box let it carry, `Height` and its net and gross volume and side
  area, and keeps `Length` and `Width` as the 2025 save does (the 2025 and 2026
  exports of Measure run 42063046730). Only roof 243274 exercises the roof-cover
  path anywhere, and no wall is cut by a plate roof, so the branch that keeps a
  plate roof as a cutter is unmeasured. In the unfiltered Measure run
  42554270370 (head 584c8a89 against `main` at fc29acbf, 118 exports) four
  exports move: `rac_basic` 2024, 2026 and 2027, and the held-out AA house
  (Revit 2026), whose walls' heights its counts did not score. The other 114,
  the BIM4LCA office's among them, are byte for byte `main`'s. On the 2026 and
  2027 saves the 16 walls are the 2025 save's bodies, and only roof 243274,
  still a record box, differs among 441 bodies. So the same-project check
  pins no wall any more (#1471, the exception of 2026-11-05 closed), and
  held-out models now keep counts of the walls at Revit's base and top.
