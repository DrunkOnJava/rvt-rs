- **Revit 2026 and 2027 projects draw pipes as their cylinders, place family
  instances' ports, cut walls to their doors' and windows' outlines, and write
  `IsExternal`, railing heights and lengths, stair names, rooms' height and net
  volume and the pipe and duct quantities, as 2025 projects do (#1190,
  RE-346).** Ten readers read the 2025 records these releases write; their
  release lists gain 2026 and 2027. **None of it is scored against a Revit
  export of a 2026 or 2027 Autodesk sample**; it is held to the same sample
  saved by Revit 2025 (`tools/re/bodies_across_saves.py`, which Measure now
  fails on a difference on `rme_advanced` and `rme_basic` too), and that cannot
  catch an error the 2025 save shares. Revit's own exports of the held-out 2026
  and 2027 projects disagreed in two places, so a door's or window's Area is
  not written on 2027, where Revit's export gives it a value
  that is not its Width times its Height, and a stair's family and type are not
  read on 2026 and 2027, where Revit's export writes no `Family:Type` object
  type; their Width, Height and the stairs' names are written and equal. No
  held-out model holds a pipe, a duct or a port. puzzbobb's local figures, not
  a Measure run: on `rme_advanced` and `rme_basic` 2026 and 2027 the bodies that
  differ from the 2025 save fall from 490 and 493 to 0, and every port of the
  four Autodesk samples that hold one, 14,095 in all, is the 2025 save's.
  Every 2015 to 2025 file exports as before.
