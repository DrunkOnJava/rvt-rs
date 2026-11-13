- **Revit 2015 to 2023 walls get the openings their rectangular wall
  openings cut (RE-456).** The steps of a wall's record that name what cuts
  it save that element's ElementId in 4 bytes before Revit 2024, and they
  were read only in 2024's 8-byte form, so on 2015 to 2023 no rectangular
  wall opening, cabinet, door in a second wall or curtain wall cut an
  opening. Both forms are read now, on Revit 2015 to 2027. A rectangular
  opening of 2015 to 2018, which saves its rectangle only in the wall's
  face, is placed along the wall's location line. Autodesk's `rac_basic`
  gains its two rectangular openings and `rac_advanced` its one on every
  save from 2015 to 2023, each equal to the same project's 2024 save, and
  BIMcollab's Revit 2015 structural example gains 18, the 15 of them that
  Revit's own export of it holds each equal to that export's.
