- **Rooms on Revit 2026 and 2027 files take their outline, not their box.**
  The reader of a room's stored solid ran on Revit 2008 to 2025 only, so on
  a 2026 or 2027 file every room was its record box, and an L-shaped room's
  box covered its neighbours: Autodesk's `rac_basic` Entry Hall was 72.8 m²
  where its 2025 save gives 30.3 m². Every room of Autodesk's four sample
  projects with rooms (257) now gives the same verdict and area in its 2026
  and 2027 saves as in 2025 (#1260, reported by inevitable-co-ai).
