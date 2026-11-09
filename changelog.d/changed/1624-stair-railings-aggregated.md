- **A railing that belongs to a stair is aggregated under it, as Revit's export
  aggregates it (#1624).** Revit's own IFC4 exports put each railing hosted on a
  stair among the parts of its `IfcStair`, beside its runs and stringers: 76 of
  Snowdon Towers' 142 railings (Revit 2024, local only) and all 10 of the R2025
  stair models' 13 to 17 (Revit 2025). rvt-rs wrote them standalone, so the body
  of the stair, which has none of its own and is the sum of its parts, was short
  by its railings: 2.74 ft of height and 0.72 ft3 of volume on R2025 model 13. A
  railing joins the stair its record names (`BaseRailing.m_stairsId`, the field
  RE-254 reads a stair railing's length from) on Revit 2024 to 2027, where a run,
  landing or stringer already makes that stair a whole; a stair no part names
  keeps its own body. A railing whose record names a component of the stair
  (`m_stairsComponentId`, a run) stays a plain element, as in Revit's export
  (RE-390): one of the BIM4LCA office's 26 stair railings, Tag 2598405, which
  Revit's export puts on Level_01 while its stair is in the Basement.
