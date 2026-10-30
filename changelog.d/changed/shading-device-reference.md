- **`Reference` only in the sets a Revit export confirms (RE-239 §7).**
  rvt-rs writes an element's type name as `Reference` in the 34 IFC4
  standard sets where a measured Revit export writes it with the same value,
  and in no other set until one does (the maintainer, 2026-10-29). Shading
  devices, alarms, footings,
  reinforcing bars, transport elements and ramp flights get none for now.
