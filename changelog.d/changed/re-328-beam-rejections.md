- **A level beam whose record box is longer than its location line is drawn
  as that box along the line (#1247, RE-328).** `beam_body` declines a
  line whose section does not close; on Autodesk's 2024 samples that is
  every beam it declines with one line, bar 2 Snowdon beams whose line
  leaves the box by up to 100 ft. 34 of the 52 are level, have no
  Cross-Section Rotation and have a box longer than their line by at most a
  foot (17 Snowdon, 9 rst_basic, 8 rac_advanced; RE-49 took 15 of Snowdon's
  for beams with oblique end cuts, and their saved mesh says otherwise).
  They are now the box's extents along the line, `BodySource`
  `partition_beam_box_along_axis`, instead of the axis-aligned record box.
  Read it as an envelope: its direction and section come from the line and
  the box (the section across the line is the beam's own saved mesh's on all
  25 whose mesh decodes whole; the mesh is Revit's tessellation, not an IFC
  export), and **its length is the box's, not the beam's**: the mesh's ends
  are the body's on 16 of the 25 and end inside it on 9, by more than
  0.01 ft and at most 1 ft, so it can run past the beam at its ends (the
  record box does on 21). The line must be level and within 25 degrees of a
  model axis, the only angles measured.
  The 9 without a decodable mesh are on a model axis, where the body is their
  record box. It gets no `Qto_BeamBaseQuantities.Length`, `Span`, `Slope` or
  `Roll`, and no I section; `AxisLength` is the line's length and
  `BodyLength` the body's. The beams already drawn along their line, and
  the BIM4LCA office, are unchanged. 16 sloped, rolled beams and the 2
  Snowdon beams keep their box. New: `beam_check` and `BeamReject` in
  `rvt::research::partition_beam_axes` name the test that declines a line
  (`beam_body` is unchanged), and `probe_re328_beam_rejections` sorts a
  file's beams by it, counts only. Report:
  `reports/element-framing/RE-328-beam-rejections.md`.
