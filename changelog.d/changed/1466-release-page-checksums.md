- **The wheels and the sdist on a release page are now in `SHA256SUMS` and the
  build-provenance attestation, so `gh attestation verify <file> -R
  DrunkOnJava/rvt-rs --source-ref refs/tags/<tag>` works for each of them and
  the one attested `SHA256SUMS` lists every file on the page but itself and the
  provenance bundle (#1466).** Up to v0.15.0 both covered the CLI archives
  only, though the page carried seven wheels and the sdist (the files PyPI
  gets). `release-binaries.yml`'s `checksums` job now hashes every file the run
  holds for the page (the archives, and for a release exactly seven wheels and
  one sdist, the `wheel-*` and `sdist` artifacts of the publish run), a new
  `attest` job checks the page against `SHA256SUMS` (every line has its file
  with that hash, every file a line) before it attests `dist/*`, and `upload`
  attaches the signed page as one artifact, holding `contents: write` and no
  longer the OIDC token. A final `page` job fails a release whose page is not on
  it (fifteen files), so a skipped `upload` goes red. A pull request or weekly
  run builds no wheels, and its `SHA256SUMS` lists the archives alone. A
  `workflow_dispatch` input, `python`, builds one wheel and the sdist so the
  same jobs can be run before a tag (RELEASE.md): nothing is attached or
  pushed, but the attestation it signs is real, which is why the documented
  `gh attestation verify` commands name the tag's ref. `RELEASE.md` also says
  how to recover a release whose page chain failed. `tools/ci/check-workflows.py`
  fails when the checksums, attest, upload and page jobs stop describing the
  same files, when a tag run could go green with no page (`upload` needing more
  than `attest`, `attest` or `upload` not running on `inputs.upload`, a job
  between them, a job-level `if` above them: a skipped job skips every job that
  needs it, however far down, which skipped `image`, `attest` and `upload` in
  the first design's pull request run 42134335447), and when the wheel and
  archive counts leave publish.yml's `build-wheels` matrix and the `build`
  matrix behind. Measured by Release binaries run 42137536209 (a dispatch with
  `python`, head 872f6b6f): the downloaded `release-page` artifact passes
  `sha256sum -c SHA256SUMS`, `gh attestation verify -R DrunkOnJava/rvt-rs`
  passes on its wheel, sdist, `SHA256SUMS` and a Linux and the Windows archive,
  the one attestation's eight subjects are the seven `SHA256SUMS` lines and
  `SHA256SUMS` itself, `docker/stage.sh` passes in the run's image job and on
  the page directory and on the two Linux archives with `SHA256SUMS` alone
  (the lines of absent files skipped), and the image was neither pushed nor
  attested; by the pull request run 42137542924 at the same head (no wheels:
  `SHA256SUMS` lists the five archives, the image job's `stage.sh` and smoke
  test pass); and by `--source-ref refs/tags/v0.15.0` passing on v0.15.0's
  macOS arm64 archive and `--source-ref refs/tags/v9.9.9` refusing it. The
  exact wheel count and the `page` comparison were run on stand-in files and
  against v0.15.0's 15 release assets with `gh release view`, not in CI. Not yet
  measured: a tag push, the only run of the `upload` and `page` jobs, of the
  seven-wheel count (the rehearsal builds one) and of the image push after its
  two-step download.
