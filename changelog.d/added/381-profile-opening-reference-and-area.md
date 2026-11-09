- **A wall's profile opening carries the wall's type name as its `Reference` and
  its profile's area, as Revit's export writes them (RE-381).** The opening of a
  wall's edited elevation profile (RE-380) now has the wall's `Reference` in
  `Pset_OpeningElementCommon` and `Pset_EnvironmentalImpactIndicators`, and none
  where the wall has none, and, where the wall has base quantities, its `Area`
  in `Qto_OpeningElementBaseQuantities`, the area of the opening's profile in
  the wall's plane, and no other quantity. Revit's export writes exactly these
  on the BIM4LCA terrain files' opening (Reference `MS-2`, Area 7.216427 m2)
  and the Reference pair on all 13 of Snowdon Towers' pocket openings (local
  only); Core Interior's export has neither, as its wall has no Reference and
  its setup writes no base quantities. Measured on one opening for the Area,
  which is why it is the profile's area and nothing more.
