- **Where an area boundary line lies against the wall it is picked from, and
  that the file stores it (#1261, RE-332).** The 127 area boundary lines of
  Autodesk's `rac_advanced` were compared with the record boxes of the walls
  their reference lists name: 61 name a basic wall, 50 a curtain wall, 16
  none. A line on an interior wall lies on its centre line, one on an exterior
  or concrete wall half the wall's thickness off it (the outer face), a
  curtain wall's 0.041 ft off its axis, and two lines are drawn on a face for
  one stretch and on the centre line for another. Joined line by line with the
  offsets the file stores for each line (`AreaMeasureCurveData`, RE-333), the
  position is that offset on all 127 (71 lines have one), and the step of the
  two compound lines is stored at the same place on both. A first reading of
  this, that nothing in the file says where such a line steps from face to
  centre, was wrong, and the rule it drew two Areas with is not in rvt-rs:
  the plan topology of RE-333 draws all 25 Areas from the stored offsets.
  `tools/re/wall_lines_vs_offsets.py` joins the two probes' output and fails
  on any line whose position and offsets disagree. Report:
  `reports/element-framing/RE-332-wall-hosted-lines.md`.
