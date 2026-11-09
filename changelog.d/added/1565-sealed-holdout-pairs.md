- **Fresh held-out pairs for the 1.0 check are sealed, and CI keeps them so
  (#1565).** W5 counts a held-out pair only when its first measurement meets
  W4, W6 and W7, so a pair that arrives first is sealed until the final 1.0
  check. `tools/ci/seal-holdout-pair.sh` downloads a pair, hashes and counts it,
  deletes it and adds its row to `research/sealed-holdout-models.tsv`, reading
  nothing else from it. `tools/ci/check_sealed_holdout.py`, a step of the PII
  guard job, fails on a malformed row, on any other tracked file holding a
  sealed URL or hash, and on any workflow or script naming the registry. Its
  failure-first tests run beside it, and each of eight mutations of the check
  fails them. The registry is empty; the final check's Measure input is added
  when W4, W6 and W7 pass on the reference models.
