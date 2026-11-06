- **A probe and a scorer for what separates RE1 Plumbing's water closet tank
  from the plumbing fixtures Revit's export holds (#1496, RE-363).**
  `probe_re363_plumbing_fixtures` prints, for every plumbing fixture a file
  holds, the record, native, family-form and saved-graphics readings that
  candidate rules would use, and `tools/re/tank_rules_vs_ifc.py` counts, for
  each rule, the fixtures in Revit's export it would drop. Not yet measured on
  this head.
