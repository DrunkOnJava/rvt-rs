- **A release tag that is not signed, not on `main` or a hotfix branch, out
  of step with a version file, or on a commit whose `CI passed` is red or
  that CI never ran on now fails in seconds, before any wheel builds, and a
  release that failed part way can be re-run (#1457).** A `preflight` job in
  `publish.yml`, which every other job needs, refuses a tag that is
  lightweight or whose signature GitHub does not report as verified, whose
  commit is on neither `main` nor a `hotfix/*` branch, or that disagrees
  with any of `rvt`, `rvt-py`, `rvt-rs-mcp` and `rvt-wasm`'s versions, the
  `=X.Y.Z` pins on `rvt`, `viewer/package.json`, `CITATION.cff`, both lock
  files or the `CHANGELOG.md` section (`tools/ci/release_preflight.py`; the
  same version check runs on every pull request), and waits for `CI passed`
  on the tagged commit through the new `wait-for-check` action, which Deploy
  viewer waits through too. A commit whose every file CI's push filter skips
  is refused as a release tag, since a docs-only commit after a red one
  would otherwise ship the red commit's tree. The crates.io steps skip a
  version crates.io already has, PyPI skips files it has from the second
  attempt on, releases run one at a time so two tags cannot race for the
  container image's `latest`, and the docs.rs preview uses CI's pinned
  nightly and banner check where it floated on `nightly`. The Intel macOS
  CLI is now run on the arm64 runner under Rosetta before it ships, and
  `SHA256SUMS` is attested with the archives, so verifying that one file
  checks every archive; the wheels and the sdist on the release page are in
  neither (#1466). Measured by the pure judgements' failure-first tests
  (each comparison mutated in a scratch copy fails them), by running the
  check read-only against v0.15.0 and v0.14.0 (it passes) and against
  missing, mismatched and off-branch cases (it refuses each), by `wait.sh`
  on real commits, by `publish.yml` dispatch run 42072957401 (the version
  check refusing a scratch branch in 14 s, with the other jobs skipped) and
  by Release binaries run 42072884597 at head 0fbd4e38 (the Intel build's
  smoke under Rosetta, which the image already had). Not yet measured: a
  tag push, which is the only run of the signature, branch and CI checks and
  of the crates.io state check on a hosted runner.
