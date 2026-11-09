- **Revit's body keeps a profile wall's top line on to the wall's end, measured
  at its stub ends (RE-383, #1473).** On the BIM4LCA office's two files the
  highest point of Revit's body at the end of the 10 stubs of 9 walls is the
  top edge of the wall's sketch loop carried on to the end, to 0.0001 ft, and
  rvt-rs's is Revit's to 0.0000 ft (RE-358's rule); on 4 stub ends of 3 walls of
  Snowdon Towers Architectural, checked against Revit's own export locally, the
  same. No byte of the file states the rule: its inputs are the sketch's lines
  and the record's box. The report names what no file with a Revit export can
  test (a stretch longer than the wall is thick, an arc, an angled wall).
