- **A beam that keeps its record box: which test of `beam_body` declines
  its one location line (#1247, RE-328).** `partition_beam_axes::beam_check`
  names the first test a line fails (`BeamReject`: an end outside the record
  box, a section that does not close, a line too steep, and the rest), and
  `beam_body` is `beam_check(..).ok()`, unchanged. `probe_re328_beam_rejections`
  sorts every structural-framing element a file's partition reader recovers
  but does not run along a line, by that test, with counts only, for the
  reporter's private Revit 2024 project; `probe_re328_beam_meshes` scores the
  candidate bodies against each beam's own saved mesh. Report:
  `reports/element-framing/RE-328-beam-rejections.md`.
