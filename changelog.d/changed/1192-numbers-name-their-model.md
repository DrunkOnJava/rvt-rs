- **Every number on the README and the supported profile now names the model
  it was measured on, and CI fails one that does not (#1192).** A new step of
  the PII guard job, `tools/ci/measured_numbers.py`, reads both pages and
  fails a number whose clause names no model of the new "Model index" table
  in `docs/reference-models.md`, a number on a `local only` or `not shared`
  model whose clause does not say so, and an index row that does not say who
  can fetch its model (`redistributable`, `fetched by Measure`, `local only`
  or `not shared`). Every digit counts except what its spelling marks as an
  identifier (an issue, a report, a hash, a date, a Revit release, a Measure
  run id, a tool's version), and a number that is not a measurement carries
  `<!-- w10: reason -->`, which fails without a reason or with nothing to
  excuse. On the pages as they were at `f009976e`, 51 clauses failed it (39
  with no model, 12 on a local-only model that did not say so; the check run
  on them with the final index); each now names its model,
  and the Snowdon Towers, 255ribeiro, flowbim, pre-2015 and client-model
  figures say "local only" or "not shared". The one figure that traced to no
  committed report, Snowdon Towers' 1,030 of 1,078 walls with faces exactly
  Revit's, is from a local run recorded in #446 on 2026-09-27 and is labelled
  as not re-run since. The README and the profile say how to read the
  figures, `docs/reference-models.md` gives the one Measure dispatch that
  re-runs them on the models anyone can fetch, with what it does not cover,
  and `docs/maintaining.md` says how a new number keeps to the rule. Measured
  by `--refusals` (a copy of the pages with a bare number, an unflagged
  local-only model, a marker without a reason, a marker with nothing to
  excuse and an index row with no fetchability, each refused), by the pure
  judgement's failure-first tests (fifteen mutations of the script each fail
  them) and by the check passing on the pages. Not measured: that the
  re-run command reproduces the pages' figures, which nobody has compared,
  and whether each figure belongs to the model in its clause.
