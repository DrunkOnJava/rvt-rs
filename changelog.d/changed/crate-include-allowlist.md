- **The crates.io package carries only what builds the crate.** It held
  the whole repository: 1,252 files, 12 MiB before compression (3.6 MB of
  crates.io's 10 MB limit for 0.10.0), with 286 research probes, 258 RE
  reports, the tooling and the maintainers' backlog. It now holds 186:
  the source, the support matrix and tag-drift table the source embeds,
  the README and the licence files. CI builds the packaged crate on every
  pull request.
