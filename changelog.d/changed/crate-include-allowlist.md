- **The crates.io package carries only what builds the crate.** It held
  the whole repository (`cargo package --list` gave 1,252 files: 286
  research probes, 258 RE reports, the tooling and the maintainers'
  backlog), 3.6 MB compressed for 0.10.0 of crates.io's 10 MB limit. It now
  packs 185 files, 1.0 MiB compressed: the source, the support matrix and
  tag-drift table the source embeds, the README and the licence files. CI
  builds the packaged crate on every pull request.
