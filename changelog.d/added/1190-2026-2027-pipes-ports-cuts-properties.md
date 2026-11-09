- **Revit 2026 and 2027 projects place a pipe by RE-131's connector entries
  first, as 2025 projects do (RE-370 already drew every pipe as its cylinder
  from its record), place family instances' ports, cut walls to their doors' and windows' outlines, and write
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
  that is not its Width times its Height, and a stair's, its flights' and its
  stringers' `ObjectType` is not written on 2026 and 2027, where Revit's export
  writes no `Family:Type` there (the `ObjectType` of every other class differs on
  those releases as well, #1472); their Width, Height and the stairs' names and
  types are written and equal. No
  held-out model holds a pipe, a duct or a port. In Measure run 42545885153
  (this change against `main` at 1c088750, unfiltered), on the 2026 and 2027
  saves of `rme_advanced` and `rme_basic` every body is the 2025 save's (6,605
  and 6,650; 36 walls differed on `main`, uncut by their doors and windows),
  every pipe (488 and 491) is the 2025 save's cylinder, as it already was since
  RE-370, and the placed ports are the 2025 save's, 6,967 and 7,054 of them
  against 1,944 and 1,956 on `main`. `rac_advanced`'s 2027 save compares its 83 doors and
  24 windows and leaves none out. Only 2026 and 2027 exports move (15 of 118),
  and on the held-out 2026 and 2027 pairs every value added is equal, with
  "different" and the values Revit does not write unchanged.
