- **The export diagnostics count the doors and windows whose type record was
  not read.** Since #1271, a door or window whose type's record a read did
  not reach (a large file over its limits) is written as a door or window
  drawn as its record box, not as its opening alone. Its category comes from
  its own record, but whether it draws geometry was never read. A warning
  now gives their number, since one that is a cased opening in Revit is its
  opening alone in Revit's export (#1247).
