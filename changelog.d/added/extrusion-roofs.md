- **A Revit 2025 roof by extrusion has its shape (RE-211, #883).** Its
  sketch is a B-spline in a vertical plane, now read: its degree, weighted
  control points and knots. That curve is the roof's top. The type's
  thickness, square to it, gives the underside, both ends are cut plumb, and
  the section runs across the record box, which it must reproduce. On
  `11_RoofByExtrusion` in puzzbobb's Revit 2025 models every vertex of
  Revit's body is within 0.001 ft of rvt-rs's (0.000124 ft at most), and the
  volume within 0.03%, where it was the 7,665 ft³ record box. That makes 19
  of the 19 roofs in those models. A roof by extrusion on another release,
  or one whose sketch holds an arc, keeps its box. `roof_bodies_vs_ifc` also
  measures Revit's vertices against rvt-rs's surface.
