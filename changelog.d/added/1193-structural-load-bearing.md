- **A wall, a floor and a stair get the `LoadBearing` Revit's exporter calculates for them (#1193, RE-429).**
  `Pset_WallCommon.LoadBearing` is true when the Structural box of the wall's record is set and its
  usage is Bearing, and false otherwise (on every release whose wall records Measure holds);
  `Pset_SlabCommon.LoadBearing` is whether the floor's Structural box (`FLOOR_PARAM_IS_STRUCTURAL`) is
  set, and a stair whose box is unchecked gets false, on Revit 2024 and later. A parameter named for the
  property still replaces the value. They read equal to Revit's on all 4,253 walls the censuses hold
  (the BIM4LCA models, the R2025 models, RE1 and an IFC2X3 pair counted apart; from 2,372), 294 of 313
  slabs (from 171) and all 168 stairs (from 52), none different, and on the 554 walls of the held-out models. Autodesk's samples and
  Core Interior, which no Revit export scores, gain the property on their walls and slabs (8,659
  elements across the 95 exports Measure keeps, each in the set it already held); no element is added
  or removed. A stair with a checked box, a wall whose usage is Shear or Combined, and floors and stairs
  before Revit 2024 are left without: no model holds the first two, and the third reads another layout.
