- **Revit 2026 and 2027 walls, and the curtain members, plates and curtain
  walls on them, get the `IsExternal` the same project's 2025 save gets
  (#1860).** A wall's type Function was read only on the releases measured
  through 2025, so on a 2026 or 2027 save every wall, and every curtain part
  that takes its `IsExternal` from its wall, was written without it: 4,643
  elements of Autodesk's `rac_advanced` sample. It is now read on 2026 and
  2027 too, measured against each sample's 2025 save element by element.
  On the 2027 save of `rac_advanced`, 79 of its 3,192 curtain members still
  get none (#1865).
