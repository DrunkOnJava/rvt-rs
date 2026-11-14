- **A part of an aggregate is no longer counted as written outside any storey
  (#1842).** `unsupported_geometry_missing_level` counted every element whose own
  storey is empty, so the flights and landings of a stair, which reach their
  storey through the stair as in Revit's export, were counted as storeyless: 28
  of the 79 elements a Revit 2027 structural model reported. The count now holds
  only the elements, and the wholes, that are in no storey. The IFC file does not
  change.
