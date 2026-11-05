- **A release tag that is not signed, not on `main` or a hotfix branch, out
  of step with a version file, or on a commit whose `CI passed` is red now
  fails in seconds, before any wheel builds, and a release that failed part
  way can be re-run (#1457).** A `preflight` job in `publish.yml`, which
  every other job needs, refuses a tag that is lightweight or whose
  signature GitHub does not report as verified, whose commit is on neither
  `main` nor a `hotfix/*` branch, or that disagrees with any of `rvt`,
  `rvt-py`, `rvt-rs-mcp` and `rvt-wasm`'s versions, the `=X.Y.Z` pins on
  `rvt`, `viewer/package.json`, `CITATION.cff`, both lock files or the
  `CHANGELOG.md` section (`tools/ci/release_preflight.py`; the same version
  check runs on every pull request), and waits for `CI passed` on the
  tagged commit through the new `wait-for-check` action, which Deploy viewer
  waits through too. The crates.io steps skip a version crates.io already
  has, PyPI skips files it has from the second attempt on, releases run one
  at a time so two tags cannot race for the container image's `latest`, and
  the docs.rs preview uses CI's pinned nightly and banner check where it
  floated on `nightly`. The Intel macOS CLI is now run on the arm64 runner
  under Rosetta before it ships, and `SHA256SUMS` is attested with the
  archives, so verifying that one file checks every archive. Measured only
  by the pure judgements' failure-first tests and by running the check
  read-only against v0.15.0 (it passes) and against missing, mismatched and
  off-branch cases (it refuses each); not yet measured in a workflow run on
  this head.
