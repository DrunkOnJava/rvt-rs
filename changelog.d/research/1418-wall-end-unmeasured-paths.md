- **The wall-end rule's unexercised paths are stated, with what each export
  reaches (#1418).** RE-337 now has a section for the paths no model
  exercised before RE-348.
  - A joined wall drawn as a solid is counted, as `joined_wall_not_read`.
    Depending on the order of the record's list, it lands in
    `exported.wall_end_refusals` (and the decisions log) or in
    `exported.wall_end_reads_unmarked`.
  - A wall in the way that the rule does not read is still found by
    nothing, and the end would be drawn through it.
  - `base_cut_lost_on_extended_plan` and `finish_wall_ends` are reachable
    and have no instance.

  On the 111 exports of Measure run 42567326561 (`main` at `83417fde`):
  - 15 of the 93 openly licensed exports are Revit 2024 saves, the only
    release the rule reads;
  - the rule draws, keeps or counts an end on 3 of them: the BIM4LCA
    office's two files and Snowdon Towers Architectural;
  - `joined_wall_not_read` and `base_cut_lost_on_extended_plan` appear on
    none;
  - all 8 walls whose end is drawn have their base cut, so
    `finish_wall_ends` makes nothing.

  No synthetic fixture can reach the pass: it reads Revit 2024 element
  records that `gen-fixture` does not write. `docs/export-diagnostics.md`
  now says which joined walls `joined_wall_not_read` covers, and that the
  decisions log leaves out the unmarked reads.
