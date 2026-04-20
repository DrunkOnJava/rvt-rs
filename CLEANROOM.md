# Clean-room policy

rvt-rs is a clean-room implementation of a reader for the Autodesk
Revit on-disk format. Every byte-level finding in the repo is derived
from **public** sources. This document states the policy in
enforceable terms so contributors and auditors can verify.

## Accepted sources

Any of these are fine for contributors to use when adding new
findings or decoders:

1. **Public byte observations.** Opening a Revit file in a hex
   editor and documenting what the bytes look like. The file itself
   can come from anywhere — Autodesk's public sample library
   (phi-ag/rvt corpus), files the contributor created themselves,
   or files a user has explicitly authorized sharing. The
   observations are independent of any Autodesk source code.
2. **Publicly-shipped symbol tables.** `RevitAPI.dll` is distributed
   by Autodesk as part of the publicly-downloadable Revit SDK /
   NuGet package. Its **exported C++ symbol list** (names + mangled
   type signatures) is a legitimate public reference. We use it
   to cross-check that class names we extract from
   `Formats/Latest` match the names Autodesk's own library uses.
   This is the same basis researchers have used for openly-published
   RVT research since at least 2014.
3. **Previously-published reverse-engineering work.** Apache Tika's
   Revit metadata extractor, chuongmep/revit-extractor (Python),
   the phi-ag/rvt project, and academic papers on Revit forensics
   are all public sources whose findings can be referenced.
   Contributors should cite the source in the PR description.
4. **IFC / buildingSMART specifications.** Freely published by
   buildingSMART International. Fair to use for IFC export
   mapping decisions.

## Forbidden sources

Do not use these. Findings derived from them will be rejected at PR
review and, if merged accidentally, removed in a subsequent commit
with the originating contributor's history preserved.

1. **Autodesk SDK source code.** Revit SDK headers / samples
   beyond the publicly-documented API surface are off limits.
   Using `RevitAPI.dll`'s **symbol exports** is fine (§Accepted
   sources); reading its **disassembly** is not.
2. **Open Design Alliance (ODA) SDK source.** The ODA
   `drwBimRv` / `drwDgnDb` / equivalent internals are
   commercial-licensed. Do not copy, paraphrase, or translate
   them.
3. **Leaked internal documentation.** Format specifications
   obtained through unauthorized means (breaches, disclosures,
   internal Autodesk documents not published on public channels)
   must not be used.
4. **NDA material.** If any contributor has signed an NDA with
   Autodesk, ODA, or a related party that covers Revit internals,
   they cannot contribute findings that their NDA covers. They
   remain welcome to contribute to non-NDA-covered areas (tests,
   CI, docs, tooling that doesn't require NDA knowledge).
5. **Decompiled internals beyond public symbol exports.** IDA /
   Ghidra output from `RevitAPI.dll` is only permissible at the
   level of "what functions does it export and with what
   signatures" — the data that appears in the DLL's public export
   table. Reconstructed pseudocode of internal functions is not
   accepted.

## Contributor declaration

When opening a non-trivial PR that adds format-level findings, the
contributor confirms the following in the PR description:

> "The changes in this PR are based on [list of sources from
> §Accepted sources]. I have not used Autodesk SDK source, ODA SDK
> source, leaked internal documentation, NDA material, or
> decompiled internal Revit implementation code beyond public
> symbol exports."

A one-sentence equivalent is fine. The point is that the finding's
provenance is on record.

For routine PRs (docs, tests, refactors, tooling), this declaration
isn't required — it applies specifically to format-RE findings
(§Q-addenda in the recon report, new `FieldType` variants, walker
improvements, element decoder additions).

## Handling suspicious contributions

If a reviewer has reason to believe a PR's findings come from a
forbidden source, the reviewer:

1. Asks the contributor privately (not in the public PR) to
   confirm the source. Respects that contributors may have
   legitimate NDA constraints preventing disclosure of specific
   details.
2. If source cannot be confirmed as an §Accepted one, the PR is
   declined and the finding does not land. The contributor is not
   blocked from future work on non-RE parts of the project.
3. If a finding later turns out to have come from a forbidden
   source after merging, the commit is reverted, the affected
   code paths are re-derived clean-room by a different
   contributor, and the project posts a correction note.

## Why this matters

- Trust with the Autodesk / ODA / buildingSMART ecosystem. A
  project that scrupulously respects IP boundaries is more
  credible and less likely to invite litigation threats.
- Downstream users can ship rvt-rs in commercial products
  (Apache-2 license + clean-room provenance = no viral-license
  risk + no IP-contamination risk).
- Contributors who want to work on the project without signing an
  NDA get a clear path.

## Legal posture

rvt-rs is intended as a clean-room interoperability implementation.
It does not use Autodesk/ODA SDK sources, leaked documentation, or
decompiled proprietary implementation code. **Users with specific
legal constraints should evaluate the project with their own
counsel.** This document describes process and discipline; it is
not legal advice.
