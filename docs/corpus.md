# Corpus Guide

rvt-rs needs redistributable Revit files to prove support across real-world
projects. This guide defines what can be accepted and how corpus health is
checked.

## Acceptance Rules

A corpus file can be committed or referenced publicly only when all of these are
true:

- The submitter has the right to share the file publicly.
- The license is recorded with an SPDX identifier or a clear custom permission
  statement.
- The file contains no confidential client data, personal data, credentials, or
  private project paths that cannot be redistributed.
- The source URL, original path, SHA256, file size, Revit release, and file type
  are recorded.

Files that cannot meet those rules may still be useful as private bug
reproducers, but they do not belong in the public corpus.

## Suggested Metadata

Create a sibling metadata file named `<fixture>.license.json`:

```json
{
  "source_url": "https://github.com/example/repo/path/to/model.rvt",
  "source_repo": "example/repo",
  "source_path": "path/to/model.rvt",
  "license": "MIT",
  "sha256": "lowercase-hex-sha256",
  "bytes": 123456,
  "revit_release": "2024",
  "file_type": ".rvt",
  "redistribution": "public",
  "notes": "Small architectural sample with walls, doors, windows, and two levels."
}
```

## Known-Count Targets

When a submitter can provide counts, prefer this shape in the issue:

```json
{
  "levels": 2,
  "walls": 24,
  "floors": 3,
  "doors": 8,
  "windows": 14,
  "rooms": 6
}
```

Known counts are not required for submission, but they are the fastest path to a
regression fixture because they give CI a concrete oracle.

Project count manifests live under
[`tests/fixtures/project-counts/`](../tests/fixtures/project-counts/). Each
manifest must mark every target category as `known`, `known_gap`,
`decoder_baseline`, or `unknown`; unknown counts require a reason so missing
source data is visible during review.

## Local Health Checks

Inventory an existing corpus directory:

```bash
tools/corpus-health.sh path/to/corpus
```

Fetch permissive candidate repositories, then inventory them:

```bash
tools/fetch-corpus.sh _corpus_candidates
tools/corpus-health.sh _corpus_candidates
```

Run the smoke test directly:

```bash
RVT_PROJECT_CORPUS_DIR=path/to/corpus cargo test --test project_corpus_smoke -- --nocapture
RVT_PROJECT_CORPUS_DIR=path/to/corpus cargo test --test project_count_fixtures -- --nocapture
```

The test suite skips gracefully when corpus paths are absent. CI should use
explicit corpus jobs once redistributable fixtures are committed or available
through a stable download path.
