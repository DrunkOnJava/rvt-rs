- **Walls' `Pset_WallCommon.ExtendToStructure` on Revit 2015 to 2023,
  experimentally (RE-284, #1158).** Whether a wall's top is joined to a roof
  or floor is read from its native record (a `JoinToRoofGStep` with
  `m_baseOrTop` 1), as on Revit 2024 and 2025. Against the same project
  saved by a later release it is equal on all 1,983 walls both files hold,
  over 24 pairs of Autodesk's sample projects of 2015 to 2022 and their 2023
  copies, with both values present (124 of the 2015 `rac_advanced`'s 180 walls
  are joined). Revit's own exports agree on the 114 walls of BIMcollab's
  Revit 2015 example and the 4 of modelo_bim, which are all not joined; the
  joined value is measured against an export only on 2024 and 2025. A wall's
  height, top cut and side quantities are still read from 2024 on.
