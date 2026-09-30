# Getting help with rvt-rs

rvt-rs is pre-1.0 and has one maintainer, so help is best effort and there
is no guaranteed response time. Where to go depends on what you need.

## Start here

- [`README.md`](../README.md) says what rvt-rs reads and writes, with the
  measured tables.
- [`docs/status.md`](../docs/status.md) and the
  [supported profile](../docs/supported-profile.md) say what is decoded, what is
  not, and which levels a file can reach. Most "does it support X" questions are
  answered there.
- [`docs/install.md`](../docs/install.md) for installing the CLIs, the Python
  package or the viewer.

## Questions, ideas and show-and-tell

Open a thread in
[GitHub Discussions](https://github.com/DrunkOnJava/rvt-rs/discussions):
Q&A for how-to questions, Ideas for proposals, Show and tell for what you
built. Issues are for work that is ready to be done, so a question opened as an
issue will be moved to a discussion.

## Bugs and requests

Use an [issue form](https://github.com/DrunkOnJava/rvt-rs/issues/new/choose).
A useful bug report has the rvt-rs version (`rvt-info --version` or
`python -c "import rvt; print(rvt.__version__)"`), the Revit release and file
type, the command you ran, and what you expected. Say whether the file can be
shared. **Do not attach a model you do not have the right to share, and do not
upload confidential projects**: rvt-rs runs locally, so a description and the
output of `rvt-inspect` or the `rvt-ifc --diagnostics` sidecar are usually
enough, and a maintainer will tell you if a file is needed.

## Security

Do not open a public issue. Use
[private vulnerability reporting](https://github.com/DrunkOnJava/rvt-rs/security/advisories/new);
[`SECURITY.md`](../SECURITY.md) says what is in scope.

## Not supported here

Revit itself, Autodesk licensing, converting a private model for you, and
anything that needs Autodesk's SDK, source or documents: rvt-rs is a clean-room
project ([`CLEANROOM.md`](../CLEANROOM.md)).

## Contributing

[`CONTRIBUTING.md`](../CONTRIBUTING.md) has a ten-minute path to a first pull
request, and [`docs/contribution-map.md`](../docs/contribution-map.md) lists the
open work by milestone.
