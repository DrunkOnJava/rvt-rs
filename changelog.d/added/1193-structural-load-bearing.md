- **A wall, a floor and a stair get the `LoadBearing` Revit's exporter calculates for them (#1193, RE-429).**
  `Pset_WallCommon.LoadBearing` is true when the Structural box of the wall's record is set and its
  usage is Bearing, and false otherwise; `Pset_SlabCommon.LoadBearing` is whether the floor's Structural
  box (`FLOOR_PARAM_IS_STRUCTURAL`) is set, and a stair whose box is unchecked gets false. They are
  written on Revit 2024 and later; a parameter named for the property still replaces the value. They
  read equal to Revit's on 4,006 of the 4,253 walls the censuses hold (the BIM4LCA models, the R2025
  models and RE1; from 2,372), 294 of 313 slabs (from 171) and all 168 stairs (from 52), none different,
  and on the 138 walls of the held-out models of those releases. The 247 walls left are an IFC2X3 pair of
  2018, counted apart. Autodesk's samples of Revit 2024 to 2027 and Core Interior, which no Revit export
  scores, gain the property on their walls and slabs (6,041 elements across the 60 exports Measure keeps
  that move, each in the set it already held); no element is added or removed. A stair with a checked
  box, a wall whose usage is Shear or Combined, and walls, floors and stairs before Revit 2024 are left
  without: no model holds the first two, and the older models show only the false state. Timed on 26 files of Revit 2024
  (Measure run 43040952174), the change reads 0.96 to 1.04 of `main`'s time and 0.95 to 1.00 of its
  peak memory.
