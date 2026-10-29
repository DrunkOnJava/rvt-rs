- **A one-storey project gets its storey.** The Level reader asked for at
  least two Levels before writing any storey, so a project with one Level
  exported every element on no storey: R2025 `01_Walls`' five walls, which
  Revit's export puts on `Level 1`. One Level is now a storey set when the
  broader Level-id scan finds no other Level, so a Level the full recovery
  misses still keeps the export from guessing.
