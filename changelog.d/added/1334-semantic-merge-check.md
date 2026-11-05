- **Measure says what moved in an export's meaning, and Jev says whether the
  pull request states it (#1334).** A Measure run with a base now diffs the
  two sides' MCP dumps for each openly licensed model (Core Interior,
  Einhoven, RE1, the BIM4LCA office, the Revit 2025 models) with
  `tools/re/semantic_diff.py`: elements added, removed and changed with the
  value before and after, byte offsets that moved for unrelated reasons
  dropped, order ignored, values compared exactly, and identical changes
  collapsed by their whole (path, before, after) so a bulk change is one line
  and a different value is never hidden in it. The diffs are the
  `semantic-diff` artifact; a model that is not openly licensed has no dump
  and the compare job fails if one holds one. The new Semantic check workflow
  (`workflow_run` on Measure, run from main in the Actions environment `jev`
  that holds the key) gives Jev `jev-1.13.0` the diff in chunks of at most
  16,000 characters beside the pull request's own `## Summary` (found through
  the API from the measured head commit, quoted as untrusted data) and asks,
  blind (the outputs are labelled A and B at random per chunk, with no model
  name, id or direction), whether each difference is stated by the described
  change, implied by it, unstated, or only rounding. It writes the check run
  `semantic diff (advisory)` on the measured commit: pass (success), warn
  (neutral), fail (failure: a move the Summary does not state) or NOT MEASURED
  (no key, a fork, no pull request, no Summary where the diff has findings,
  HTTP 402, a time-out), with each finding and its probability. It is not a
  required check and posts no pull request comment. A pass means the dumps
  agree, not that the exports are identical: the dump samples five elements
  per entity. `docs/maintaining.md` says how an agent fixes a fail (change the
  code, or state the change in the Summary) and answers a warn. Three
  failure-first tests of the diff (0.35 against 0.3500001, a reorder with moved
  offsets, a bulk collapse that hides a different value) run in the PII guard
  job.
