# Changelog fragments

Each pull request writes its changelog entry here instead of in
`CHANGELOG.md`, as `changelog.d/<section>/<name>.md`. `<section>` is one of
`added`, `changed`, `fixed`, `removed`, `security` and `research`, and
`<name>` is anything that keeps the file apart from other pull requests',
such as its branch name. When every pull request added its entry at the top
of the same `CHANGELOG.md` section, each one that landed left the others in
conflict with `main` (#938).

A fragment is the entry exactly as it will read in the changelog: one or
more items starting `- **`, wrapped at about 76 columns, with the same
measured, user-facing wording as the entries already in `CHANGELOG.md`.

    python3 tools/release/changelog.py check      # CI runs this on every pull request
    python3 tools/release/changelog.py assemble   # a release step (RELEASE.md)

`check` also fails on an item a released section of `CHANGELOG.md` already
has: an entry a branch carried over from before a release belongs to the
release that shipped it.

`assemble` moves every fragment into `CHANGELOG.md`'s `[Unreleased]`, at
the top of its section, and deletes the fragments. Until a release runs it,
the changes on `main` since the last release are in `CHANGELOG.md`'s
`[Unreleased]` (those from before 2026-10-24) and here.
