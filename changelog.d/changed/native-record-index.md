- **Faster exports of Revit 2023 to 2027 files (#1032).** Each read of the
  native records walked every partition twice; the first one now records
  where every record is, and the later ones decode only the partitions
  holding the records they need. On one runner, the BIM4LCA office exports
  7 to 11% faster, Core Interior 14%, Autodesk's 2024 samples 2 to 8%, with
  every export byte for byte as before.
