- **A door's opening in a second wall now gets the `Width`, `Height` and `Depth`
  Revit's export writes for it (#1165, RE-338).** `Qto_OpeningElementBaseQuantities`
  holds them on the openings rvt-rs writes for a door that cuts a wall besides the
  one it is hosted in: `Width` the wall's thickness, `Height` the opening's height
  and `Depth` its extent along the wall, the wall's section Revit's export sweeps
  the door's width into. A door's opening in a wall that runs along the door's own,
  which Revit draws ten feet across about a plane nothing read gives, is still
  left out.
