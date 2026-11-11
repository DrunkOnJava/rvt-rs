- **A Revit 2027 save is written without a `PredefinedType` on its beams,
  columns, plates and coverings as well (#1716).** The exporter's public source
  writes an occurrence's `PredefinedType` only where the element has no type
  object, and the creators of those four classes call that helper as the
  creators of the ten classes #1692 measured do; this is taken from the source
  and not from a measured export, since no Revit 2027 export holds one of them,
  and a curtain wall, a footing, a ramp, a shading device and the MEP terminals
  keep their value.
