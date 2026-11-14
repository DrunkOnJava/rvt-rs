- **An element whose IFC export parameters differ between the partitions of a
  Revit 2024 or later file takes the latest partition's value, as Revit's
  export writes it (#1713).** The scan dropped a key an element set to two
  different values, but a later partition holds the edited, current copy: the
  BIM4LCA permit files change a wall's or a wall type's "IFC Predefined Type"
  after the first save, and 40 walls and 42 family-instance spaces of three of
  them read no value where Revit's export writes one. A latest copy that
  disagrees with itself still leaves the key out.
