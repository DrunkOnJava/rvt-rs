# Probes

Each program here proves one fact about the Revit file format against a
corpus. Its module documentation states the fact, and how to check it. The
`probe_re*.rs` probes back the numbered reports under
[`reports/element-framing/`](../reports/element-framing/); the others back
issues or earlier findings.

They are the examples of the `rvt-probes` crate, a workspace member that is
never published. Before it was split out, the `rvt` package compiled all of
them in every build and CI check.

```sh
cargo run --profile ci -p rvt-probes --example probe_re70_wall_join_lists -- path/to/model.rvt
```

Probes use `rvt::research`, the unstable readers behind the
`unstable-research` feature, so their APIs and output change with each
finding. CI's clippy job checks that every probe still compiles.

Measure runs a probe on each reference model and can score its output:

```sh
gh workflow run measure.yml --ref main -f ref=<sha> -f base=none \
  -f probe=probe_re70_wall_join_lists -f probe_scorer=wall_join_lists_vs_ifc
```

A new probe is one self-contained file, as
[`CONTRIBUTING.md`](../CONTRIBUTING.md#reverse-engineering-findings) describes.
It reads corpus paths from its arguments or the `RVT_*` environment variables,
and never hardcodes them.
