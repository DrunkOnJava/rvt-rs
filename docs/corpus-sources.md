# Expanding the test corpus

The shipped integration tests run against the phi-ag/rvt sample family
(11 releases × ~400 KB `.rfa`). That corpus is **deliberately narrow**
— family files are simpler than real project models and miss several
edge cases we want confidence on.

## What's still missing

- **`.rvt` project files** (20-200 MB range). Real architecture /
  engineering projects with hundreds of levels, sheets, views,
  elements. Tests: does `rvt-analyze` still finish in seconds? Are
  there tag values we haven't seen? Does Layer 4c.2 classification
  coverage hold at 84%?
- **`.rte` project templates** (~1-20 MB). Tests the template-file
  code paths.
- **`.rft` family templates** (~100 KB - 2 MB). Tests families not
  yet populated with content.
- **Large-project stress test**. Find a publicly-redistributable file
  ≥ 100 MB to verify streaming-read + memory behaviour.

## Where to find them (lawful, redistributable sources)

1. **Autodesk's own sample projects.** Bundled with every Revit
   installation under `C:\ProgramData\Autodesk\RVT <year>\Samples\`.
   Redistribution rights unclear; verify per-file license before
   committing to the repo.

2. **NIBS / buildingSMART sample datasets.** Some are public,
   redistributable, and specifically curated for IFC-interop testing.
   Check buildingSMART's sample-file library and NIBS's COBie test
   sets.

3. **GitHub repos with legitimately-published .rvt files.**
   Search: `language:N/A extension:rvt size:>10000000`. Vet
   license + provenance before adding any to our corpus.

4. **Client-donated files.** If a real contractor / architect
   offers a representative project file under a permissive license,
   that's the highest-signal test input. Redact all personal
   info before committing.

5. **Programmatically-generated.** A script that builds a
   representative project via Revit's API during a one-off authoring
   session, stored in the repo alongside its generator. Not automatic
   but repeatable.

## Committed-in-repo criteria

- Cumulative corpus size < 500 MB (Git LFS quota).
- Every file has a clear, redistributable license (CC-BY, Apache-2,
  MIT, or explicit Autodesk permission).
- Every file is redacted of creator paths / personal names before
  commit (see `src/redact.rs` for the helpers).
- Every file has a matching entry in `samples/PROVENANCE.md` naming
  source, license, sha256, and date added.

## Alternative: probabilistic corpus

Rather than one large test file, generate a probabilistic corpus:
for each feature the reader should handle, find the smallest RFA/RVT
that exercises it, and check in just that file. Faster to clone,
easier to keep redactable, and gives better test-isolation.

Suggested probabilistic targets:

- 1× multi-category family (doors, windows, furniture)
- 1× template file (`.rft`)
- 1× project template (`.rte`)
- 1× multi-level project with phasing
- 1× project with linked models
- 1× project with extensive parameter customisation
- 1× "broken" / minimal-viable file (for error-handling tests)

This is the recommended track once public adoption starts — early
adopters' broken files become the test corpus organically.
