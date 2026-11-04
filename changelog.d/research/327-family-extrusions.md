- **A family's extrusions rebuild from their own sketch curves (#156, RE-327;
  Discussion #112).** An `ExtrusionElem`'s helper names its sketch; the
  `CurveElem` records that name it each hold one `GLine` or `GArc` in the
  family's coordinates; chained into loops, sorted into outlines and holes,
  placed in their `SketchPlane`'s frame (the columns of `m_3x3`, `m_flipZ`
  reversing the normal) and swept between the extrusion's two `Double`
  parameters (-1001800 and -1001801), they reproduce the mesh Revit saved
  for the form: the 24 extrusions of the family corpus (2 on each of 12
  releases, 2016 to 2027) within 0.001 ft, and 6,093 of the 6,172
  extrusions of Autodesk's 58 sample projects (Revit 2015 to 2027) whose
  saved mesh decodes whole, by volume (1%), bounding box, vertices and
  surface (0.01 ft). The other 79 share one signature (a down-facing plane,
  not flipped, whose extrusion sets no `m_alwaysRefPlaneNorm`), the solid
  on the wrong side of the plane, and the reader refuses them.
  `native_document` refuses every family file, so the new
  `native_form_geometry` reads a family file's own groups; `VarSketch` is
  refused on 2015 to 2017 (`VarParam::m_pSubst`, RE-323) and the rebuild
  does not read it. Not reproduced: STE1200's 85 of 86, a Revit 2014 file
  outside the native releases. Report:
  `reports/element-framing/RE-327-family-extrusions.md`.
