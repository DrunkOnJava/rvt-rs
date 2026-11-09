- **Fresh held-out pairs for the 1.0 check are sealed in a registry that no
  committed workflow or script may name (#1565).** W5 counts a held-out pair
  only when its first measurement meets W4, W6 and W7, so a pair that arrives
  first is sealed until the final 1.0 check. `tools/ci/seal-holdout-pair.sh`
  downloads a pair, hashes and counts it, deletes it and adds its row to
  `research/sealed-holdout-models.tsv`. It refuses, from the first 64 KiB of
  each file alone, a project that is not an OLE2 file and an IFC that is not
  that release's own IFC4 or IFC4X3 export, and it refuses a row the check
  would fail. `tools/ci/check_sealed_holdout.py`, a step of the PII guard job,
  fails on a malformed row, on any other tracked file holding a sealed URL or
  hash, and on any committed file that is not Markdown and names the registry.
  It cannot see a path built at run time or a run dispatched from an
  unreviewed branch. Its failure-first tests run beside it, and each of 16
  mutations of the check fails them. The registry is empty; the final check's
  Measure input is added when W4, W6 and W7 pass on the reference models.
