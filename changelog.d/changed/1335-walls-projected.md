- **The read of the wall records no longer builds the values of the fields it
  ignores (#1335).** The read that takes each wall's type and whether its top
  is joined to a roof or floor decoded every `SWall` and `ArcWall` graph in
  full to read two fields of it. It reads and checks the same bytes, with the
  same refusals, and builds only the fields it keeps. No export changes. The
  figures are in the pull request.
