- **Measure says what moved in an export's meaning, and Jev says whether the
  pull request states it (#1334).** A Measure run with a base now diffs the
  two sides' MCP dumps for each openly licensed model (Core Interior,
  Einhoven, RE1, the BIM4LCA office, the Revit 2025 models) with
  `tools/re/semantic_diff.py`: elements added, removed and changed with the
  value before and after, byte offsets that moved for unrelated reasons
  dropped, order ignored, values compared to the dump's six decimals, and
  identical changes collapsed by their whole (path, before, after) so a bulk
  change is one line and a different value is never hidden in it. The diffs
  are the `semantic-diff` artifact; a model that is not openly licensed has no
  dump and the compare job fails if one holds one. The new Semantic check
  workflow (`workflow_run` on Measure, run from main in the Actions
  environment `jev` that holds the key) scores only a Measure run dispatched
  in this repository, gives Jev `jev-1.13.0` the diff in chunks of at most
  16,000 characters beside the pull request's own `## Summary` and `##
  Testing` (found through the API from the measured commit, HTML element tags
  and invisible characters removed, quoted as untrusted data, all of it shown in
  the check's summary) and asks, blind (the outputs are labelled A and B at random per
  chunk, with no model name, pull request number or direction), whether each
  difference is stated specifically, follows from a specific change, is
  covered only by a general description, is unstated, or is only rounding. A
  general description counts as unstated, so "improves how walls are
  exported" covers no change. It writes the check run `semantic diff
  (advisory)` on the commit Measure measured: pass (success), warn (neutral),
  fail (failure: a move the Summary and Testing do not state specifically) or
  NOT MEASURED (no key, a pull request from a fork, no pull request for the
  commit even for an empty diff, a check written on a commit Measure did not
  measure, no Summary where the diff has findings, HTTP 402, a time-out, an
  answer that does not add up), with each finding and its probability. The
  measured commit and the base are written by the compare job from the plan
  job's refs, not by the measured code, and a run whose base is not on main
  writes no check. Only
  a run of every open model writes a check, and a pass needs all 34 compared
  in full: a model whose dump is missing, a diff that failed, a dropped group,
  sampled property sets that one dump alone holds too often (none held by
  both while either holds any, or more than a tenth of them) or an unscored
  chunk makes it NOT MEASURED at best, and a run with a
  `models` filter writes none, so a narrower run's pass cannot stand beside a
  fuller run's fail. It is not a required check and posts no pull request
  comment. A pass means the dumps agree, not that the exports are identical:
  the dump samples five elements per entity. `docs/maintaining.md` says how
  an agent fixes a fail (change the code, or state the change specifically
  in the Summary) and answers a warn, and gives the command for a complete
  run. Failure-first tests of the diff and of the verdict logic (0.35
  against 0.3500001, a reorder with moved offsets, a bulk collapse that hides
  a different value, a gap that reads as a pass, an answer that does not add
  up, a chunk over the cap in non-ASCII text) run in the PII guard job.
