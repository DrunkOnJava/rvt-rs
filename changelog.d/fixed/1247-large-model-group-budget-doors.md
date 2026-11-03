- **`--large-model` now raises the budget of the reads of native records,
  and a door or window is written as its opening alone only where its type's
  record was read.** Each native read took a fixed 256 MiB budget for one
  reassembled group, which `--large-model` did not raise. On a user's 438 MB
  Revit 2024 project a group over it stopped the read of the door types and
  of the saved graphics, even with `--large-model`. That project's 73 doors
  then fell back to the type-parameter scan, which RE-299 showed misses
  maps, and were written as their openings alone (#1247). The default
  budget now follows the file's inflate limit, so `--large-model` raises it,
  and is never more than the partition's own inflated bytes. On Revit 2024
  to 2027, a door or window is written as its opening alone only where its
  type's native record was read with an empty geometry map; a type the read
  did not reach is not taken to draw nothing. Earlier releases keep their
  rule.
