- **The viewer no longer calls a Revit 2008 to 2014 file an unsupported
  version.** Its file status warned "Unsupported Revit version" for any file
  before Revit 2015, though rvt-rs reads 2008 to 2014 element records
  experimentally since RE-259 and RE-260 (#1076, #1077); it now warns only
  outside 2008 to 2027, as `rvt-inspect` decides. `rvt-inspect`'s
  `unsupported_revit_version` message, and the docs that quote it, name the
  same 2008 to 2027 range.
