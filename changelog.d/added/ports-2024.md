- **A Revit 2024 family instance's ports are placed too (#869).** Placing a
  port no duct or pipe joins at its connector, through the instance's
  transform, ran on Revit 2025 only, the release the RE1 models measured.
  The BIM4LCA office, a Revit 2024 project with Revit's IFC4 export, is the
  oracle for 2024: all 43 of its such ports are now within 0.01 ft of
  Revit's, with Revit's axis, where none was placed. `port_places_vs_ifc`
  now compares each file in its own site frame, since Revit writes the
  office's site at its shared coordinates; the RE1 scores do not change.
