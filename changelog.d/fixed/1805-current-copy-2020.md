- **A wall, floor, roof or ceiling type whose layer copies disagree takes its
  current copy on Revit 2020 too (#1805, RE-466).** It was dropped there, and
  its elements had no layer set. On the one Revit 2020 file measured with such
  a type, the 5 floors' layer sets are now within 0.001 ft of Revit's, with
  Revit's material on every layer that has a width; a layer with no material
  of its own takes its category's (RE-464). On 4 of them, Revit's export also
  writes 0-wide membrane layers, which rvt-rs leaves out as Revit's Snowdon
  export does (RE-53). No Autodesk sample of 2015 to 2023 has such a type, so
  the other releases before 2024 still drop it.
