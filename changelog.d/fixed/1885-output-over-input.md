- **`rvt-ifc` refuses to write over its input, and `rvt-gltf`, `rvt-sheet`
  and `rvt-schedule` refuse however the input is named (#1885).**
  `rvt-ifc model.rvt -o model.rvt` replaced the Revit file with the IFC, with
  no check. The other three compared the output path with the input path as
  typed, so `./model.rvt`, a symbolic link to it and (on Unix) a hard link got
  through. All four now ask whether the two are one file before they export,
  and say `refusing to overwrite the input`. `rvt-ifc --observation` also
  wrote its IFC straight to the output's name, so a killed export left a
  truncated `.ifc` there; it is written beside the output and renamed when
  whole, as the export without `--observation` has been.
