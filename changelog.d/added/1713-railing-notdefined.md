- **A railing with no predefined type of its own or its type's is `NOTDEFINED`
  on Revit 2024 to 2026, as Revit's export writes it (#1713).** Revit's export
  gives every such railing `NOTDEFINED`: the 8 of the BIM4LCA office files, the
  10 of the R2025 models 13 to 17 and the 2 of each held-out pair of 2024 to
  2026; rvt-rs left them unset. The 2027 export writes no value on a railing and
  a 2022 file's export parameters are not read, so neither release takes the
  default.
