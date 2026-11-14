- **An element whose IFC export parameters differ between the partitions of a
  Revit 2024 or later file takes its current copy's value, as Revit's export
  writes it (#1713).** The scan dropped a key an element set to two different
  values, but the partition the element's stored revision routes to (else the
  latest that holds the key, as every other read of a copied record does) holds
  the edited value: the BIM4LCA permit files change a wall's or a wall type's
  "IFC Predefined Type" after the first save, so 40 walls and 42
  family-instance spaces of three of them read no value where Revit's export
  writes one. The same rule moves 8 roofs of the timber office to `IfcSlab`
  and one wall of the concrete housing as built to `IfcCovering`, as their
  latest "Export to IFC As" says. A roof exported as `IfcSlab` keeps its saved
  mesh like any roof. A current copy that disagrees with itself still leaves
  the key out.
