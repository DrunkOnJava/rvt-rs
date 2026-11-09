- **A curtain panel of Revit 2015 to 2023 takes the material of its category's
  Object Styles where the category holds one, and its type's otherwise (#1593,
  RE-377).** Revit's export gives the 67 plates of a Revit 2018 project the
  material of the Curtain Panels category's two graphics styles, not the panel
  type's `m_renderStyleId`, which is another material on all 67; on every other
  model with plates the category holds none and Revit gives the type's. The panels
  of those releases had no material since RE-367; they have Revit's on that
  project and, where the category holds none, the type's as on 2024 and later.
  Revit 2024 and later are unchanged: no model of those releases has a Curtain
  Panels category that holds a material, so which Revit's exporter prefers there is
  not measured.
