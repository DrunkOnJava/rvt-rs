- **`rvt-ifc --no-base-quantities` (#1016).** Leaves out every base
  quantity (the `Qto_` sets: lengths, widths, heights and areas), as
  Revit's exporter does when its export setup's "Export base quantities" is
  off. They stay on by default, as in each of Revit's built-in IFC4 setups
  (RE-242). The quantity census counts the quantities rvt-rs writes apart
  where Revit's reference export writes none, instead of as values Revit
  does not write.
