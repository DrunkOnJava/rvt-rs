- **Revit 2025 stairs, flights and stringers are named and typed as Revit's
  export names them (#323).** RE-65 read a stair's and its parts' names
  from byte offsets measured on Revit 2024, so on Revit 2025 they had no
  name and no type object. They now come from the native records: the stair
  type's construction (`Assembled Stair`, `Cast-In-Place Stair`), each
  type's name, the run type's monolithic flag and the stair type's support
  style. On puzzbobb's five R2025 stair models every stair, flight and
  stringer has Revit's `Name` and `ObjectType`, and a type object with
  Revit's GlobalId.
