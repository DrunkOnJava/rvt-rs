- **`tools/ci/seal-holdout-pair.sh` keeps one unopened copy of each sealed pair
  in a private repository, which the final 1.0 check will fall back to where a
  contributor's link no longer serves the file whose hash the row holds
  (#1565).** It uploads both files as `<sha256>.rvt` and `<sha256>.ifc`,
  checks each asset's digest against the hash it is named by, and only then
  deletes the local copies and adds the row, so no pair is listed without a
  copy. It refuses to upload unless that repository is private: the files
  embed Autodesk's template families, which no contributor's licence covers,
  so the copies are never made public or redistributed.
  `tools/ci/check_sealed_holdout.py` now also fails on any committed file
  that is not Markdown and names that repository. The final check is still
  to come, and its pull request decides which credential it reads the copies
  with.
