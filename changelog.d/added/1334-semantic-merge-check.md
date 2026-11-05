- **Measure says what moved in an export's meaning, and Jev scores it
  (#1334).** A Measure run with a base now diffs the two sides' MCP dumps
  for each openly licensed model (Core Interior, Einhoven, RE1, the BIM4LCA
  office, the Revit 2025 models) with `tools/re/semantic_diff.py`: elements
  added, removed and changed with the value before and after, byte offsets
  that moved for unrelated reasons dropped, order ignored, values compared
  exactly, and identical changes collapsed by their whole (path, before,
  after) so a bulk change is one line and a different value is never hidden
  in it. The diffs are the `semantic-diff` artifact; a model that is not
  openly licensed has no dump and the compare job fails if one holds one.
  The new Semantic check workflow (`workflow_run` on Measure, run from main
  in the Actions environment `jev` that holds the key) scores the artifact
  with Jev `jev-1.13.0` in chunks of at most 16,000 characters, blind (the
  descriptions are labelled A and B at random per chunk, with no model name,
  id or direction), and writes the check run `semantic diff (advisory)` on the
  measured commit: pass, warn, fail or NOT MEASURED (no key, a fork, HTTP 402,
  a time-out, a diff with no model compared), with each finding and its
  probability in the summary. It is not a required check and posts no pull
  request comment; `docs/maintaining.md` says how an agent treats a warn and a
  fail. Three failure-first tests of the diff (0.35 against 0.3500001, a
  reorder with moved offsets, a bulk collapse that hides a different value)
  run in the PII guard job.
