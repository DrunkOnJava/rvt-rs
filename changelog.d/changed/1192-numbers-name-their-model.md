- **Every number on the README and the supported profile now names the model
  it was measured on, and CI fails one that does not (#1192).** A new step of
  the PII guard job, `tools/ci/measured_numbers.py`, reads both pages and
  fails a number with no model of the new "Model index" table in
  `docs/reference-models.md` within 300 characters of it in its own clause
  (nothing inside parentheses ends a clause), a number on a `local only` or
  `not shared` model whose clause does not say so beside that model's name,
  and an index row that does not say who can fetch its model
  (`redistributable`, `fetched by Measure`, `local only` or `not shared`).
  Every numeral counts, in a heading as in a cell, except what its spelling
  marks as an identifier (an issue, a report, a hash, a date, a Revit
  release, a Measure run id, a tool's version), and a number that is not a
  measurement carries `<!-- w10: reason -->`, which fails without a reason or
  with nothing to excuse. On the pages as they were at `f009976e`, 51 clauses
  failed the first form of the check (39 with no model, 12 on a local-only
  model that did not say so); each now names its model, and the Snowdon
  Towers, 255ribeiro, flowbim, pre-2015 and client-model figures say "local
  only" or "not shared". The one figure that traced to no committed report,
  Snowdon Towers' 1,030 of 1,078 walls with faces exactly Revit's, is from a
  local run recorded in #446 on 2026-09-27 and is labelled as not re-run
  since. The saved-mesh figures that had gone stale (the BIM4LCA office's
  1,294 of 1,709 and RE1's 207 and 148 of 214, from October) are now Measure
  run 41850769925's at `e024f03c`: the office 1,554 of 1,650 as built and
  1,638 of 1,738 at building permit, with 265 of 269 doors and all 269
  windows exact as built, RE1 148 of 228 exact and 228 of 228 within 0.01 ft
  of Revit's box, and Core Interior 138 of 138 (RE-78's 158 included 20 shading
  devices, which run 41850769925 does not score). The README and the
  profile say how to read the figures, `docs/reference-models.md` gives the
  one Measure dispatch that re-runs them on the models anyone can fetch, with
  what it does not cover, and `docs/maintaining.md` says how a new number
  keeps to the rule and that a pull request which moves a figure edits the
  page. Measured by `--refusals` (a copy of the pages with a bare number, a
  number far from its model, a number in a heading, an unflagged local-only
  model, a local-only word beside another model's name, a marker without a
  reason, a marker with nothing to excuse and an index row with no
  fetchability, each refused), by the pure judgement's failure-first tests
  (twenty-two mutations of the script each fail them) and by the check
  passing on the pages. Not measured: that the re-run command reproduces the
  pages' figures, which nobody has compared, and whether each figure belongs
  to the model in its clause.
