- **Revit 2025's tapered walls are drawn tapered (RE-213, #358).** RE-86
  read a wall type's face angles only on Revit 2024. Revit 2025 frames them
  with twelve zero bytes after the frame's `u32 2`, where 2024 has eight, so
  on a 2025 file every tapered wall fell back to its record box, up to
  9.3 ft wider than the wall. On Snowdon Towers saved by Revit 2025, 19 of the
  20 walls Revit tapers are now within 0.001 ft of Revit's body across the
  wall at five heights (none within 0.1 ft before). 24 walls are closer to
  Revit's in the GLB and none is further, and the 20 windows in them take
  RE-89's cut. Every other 2025 file measured exports the same bytes.
