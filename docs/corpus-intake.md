# Corpus intake checklist

Short checklist for accepting Revit files into rvt-rs validation work.
Motivated by the contributor conversation in
[Discussion #112](https://github.com/DrunkOnJava/rvt-rs/discussions/112)
([@STE1200](https://github.com/STE1200)).

**Do not solicit files a contributor is unsure they can share.** If rights,
confidentiality, or redistribution are unclear, stop. Prefer a local probe
note or an authorized private hand-off over pressure to attach bytes.

For metadata shape and health commands, see [`docs/corpus.md`](corpus.md).
For lawful public sources, see [`docs/corpus-sources.md`](corpus-sources.md).

## Three intake lanes

| Lane | Where bytes live | What maintainers may do | Required before use |
|------|------------------|-------------------------|---------------------|
| **Public redistributable** | In-repo (`corpus/tier1/`), or a public URL cited in a license sidecar | Commit, redistribute with the project, run in CI | Rights confirmed; SPDX/custom license; no confidential content; sidecar complete |
| **Authorized private** | Maintainer-only storage; never committed | Reproduce bugs, measure coverage, write public summaries without leaking file contents | Explicit authorization; retention/use limits recorded; no public attachment |
| **Local probes** | Contributor or maintainer machine only | Share probe output, offsets, counts, and falsifiers in issues/PRs | No private paths or customer identifiers in public text; do not upload the file |

Autodesk-owned installer samples are **not** redistributed by this project
(see [`SECURITY.md`](../SECURITY.md)). Point CI at external corpora with
`RVT_SAMPLES_DIR` / `RVT_PROJECT_CORPUS_DIR` when needed.

## License sidecar (public lane)

Every public fixture needs a sibling `<name>.license.json` (see
[`docs/corpus.md`](corpus.md)). Minimum fields:

- `source_url` / `source_repo` / `source_path` (as applicable)
- `license` (SPDX id or clear custom permission statement)
- `sha256`, `bytes`
- `revit_release`, `file_type`
- `redistribution` (`public` for this lane)
- `notes` (why the file is useful)

Optional known-count oracles speed regression fixtures; they are not required
to open an intake issue. Use the
[Corpus submission](https://github.com/DrunkOnJava/rvt-rs/issues/new?template=corpus_submission.yml)
form for public offers.

## Lane selection guide

1. **Unsure about rights or confidentiality?** Do not submit. Say so in the
   discussion or issue and stop — maintainers will not ask for the file.
2. **Clearly redistributable and scrubbed?** Public lane + sidecar.
3. **Authorized for research but not redistribution?** Authorized private —
   arrange a private channel; never paste private corpus into the tracker.
4. **Useful only on your machine?** Local probes — paste commands, digests,
   counts, and evidence tables, not the binary.

## Maintainer checklist (before merge or private use)

- [ ] Lane chosen explicitly; public lane never used for private material
- [ ] Contributor affirmed rights (public) or authorization (private)
- [ ] Sidecar present and SHA256 matches bytes (public)
- [ ] No credentials, client names, or unretractable paths in public text
- [ ] Expected counts / probe oracles recorded when available
- [ ] Credit recorded when the offer came from a named contributor discussion
