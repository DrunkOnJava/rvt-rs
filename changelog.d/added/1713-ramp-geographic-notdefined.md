- **A ramp or a geographic element with no predefined type of its own or its
  type's is `NOTDEFINED` on Revit 2024, as Revit's export writes it (#1713).**
  Revit's export gives `NOTDEFINED` to the one ramp of each BIM4LCA housing
  file and to 3 of the 23 geographic elements of each of its two terrain files
  (6 and 6 in all), where rvt-rs left them unset. No measured file of another
  release and no held-out pair holds either in both exports, so no other
  release takes the default.
