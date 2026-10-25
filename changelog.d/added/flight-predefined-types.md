- **A stair flight's PredefinedType is STRAIGHT or SPIRAL, as Revit's
  export writes it (#323).** Every flight was `.NOTDEFINED.` or unset. A
  run's treads name its shape in their native records (`m_runType`, 0 for a
  straight run, 2 for a spiral one), and the flight and its type object now
  take it, on Revit 2024 and 2025. On the BIM4LCA office all 35 flights of
  each file have Revit's predefined type, and so do the flights of the 5
  R2025 stair models. A stair's own predefined type (Revit's
  `DOUBLE_RETURN_STAIR` and the like) is not read yet.
