- **Exports read walls' native records once instead of twice, and skip
  per-record hashes no export uses.** Measured on GitHub's runners against
  0.12.x on every reference model and Autodesk's samples, with byte-identical
  output: Revit 2015 to 2023 exports take 4% to 5% less time, 2024 3%, 2025
  3%, 2026 and 2027 1% to 2%, the reference and BIM4LCA models 4%. Peak
  memory is unchanged within 1 MiB except on Autodesk's 2015 rme basic
  sample, 361 to 386 MiB, which is not explained (#1237).
