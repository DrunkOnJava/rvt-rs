- **The wheels and the sdist on a release page are now in `SHA256SUMS` and the
  build-provenance attestation, so `gh attestation verify <file> -R
  DrunkOnJava/rvt-rs` works for each of them and the one attested `SHA256SUMS`
  checks every file on the page but the provenance bundle (#1466).** Up to
  v0.15.0 both covered the CLI archives only, though the page carried seven
  wheels and the sdist (the files PyPI gets). `release-binaries.yml`'s
  `checksums` job now hashes every file the run holds for the page (the
  archives, and for a release the `wheel-*` and `sdist` artifacts of the
  publish run), a new `attest` job checks the page against `SHA256SUMS` (every
  line has its file with that hash, every file a line) before it attests
  `dist/*`, and `upload` attaches the signed page as one artifact, holding
  `contents: write` and no longer the OIDC token. A pull request or weekly run
  builds no wheels, and its `SHA256SUMS` lists the archives alone. A
  `workflow_dispatch` input, `python`, builds one wheel and the sdist so the
  same jobs can be run before a tag (RELEASE.md): nothing is attached or
  pushed, but the attestation it signs is real. `tools/ci/check-workflows.py`
  fails when the checksums, attest and upload jobs stop describing the same
  files, and on a job-level `if` above them: a skipped job skips every job
  that needs it, however far down, which skipped `image`, `attest` and
  `upload` in the first design's pull request run (42134335447). Measured by
  Release binaries run 42137536209 (a dispatch with `python`, head 872f6b6f):
  the downloaded `release-page` artifact passes `sha256sum -c SHA256SUMS`,
  `gh attestation verify -R DrunkOnJava/rvt-rs` passes on its wheel, sdist,
  `SHA256SUMS` and a Linux and the Windows archive, the one attestation's
  eight subjects are the seven `SHA256SUMS` lines and `SHA256SUMS` itself,
  `docker/stage.sh` passes in the run's image job and on the page directory
  and on the two Linux archives with `SHA256SUMS` alone (the lines of absent
  files skipped), and the image was neither pushed nor attested; and by the
  pull request run 42137542924 at the same head (no wheels: `SHA256SUMS` lists
  the five archives, the image job's `stage.sh` and smoke test pass). Not yet
  measured: a tag push, the only run of the `upload` job, of the seven-wheel
  set (the rehearsal builds one) and of the image push after its two-step
  download.
