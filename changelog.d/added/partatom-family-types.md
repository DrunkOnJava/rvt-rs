- **A family's types and type parameters, from its PartAtom type table.**
  `PartAtom::family_types()` reads the table a family's `PartAtom` stream
  holds: each type's name, and each type parameter's name, display name,
  kind (built-in or the family's own), parameter type, units and value as
  Revit printed it. `rvt-info` shows it (`family_types` in `--json`). It is
  Revit's text at save, not a reading of the family's own records.
