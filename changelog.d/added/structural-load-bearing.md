- **`LoadBearing` on structural columns and framing (#1158, RE-285).**
  Revit's exporter calls a structural column or structural framing element
  load bearing by its category alone. rvt-rs now writes that `LoadBearing`
  in its common property set on every release, unless a parameter named for
  it says otherwise, and on Revit 2015 to 2023, experimentally, the
  element's `IsExternal`, not external, as the calculation's last step gives
  it. Against Revit's exports: BIMcollab's Revit 2015 example 91 of 91 beams
  and 15 of 15 columns for each property, and modelo_bim (a Revit 2023 file,
  Revit 26.2's exporter) 8 of 8 beams and columns for `IsExternal` and 9 of 9
  for `LoadBearing` (the ninth on the type objects RE-286 writes); on the
  BIM4LCA office as built one column with no such parameter gains Revit's
  value. None different, and no new value that Revit's export leaves out.
