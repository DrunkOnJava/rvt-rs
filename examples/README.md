# Examples

**Using the library.** Start with these. Each runs as-is on the synthetic
project file the repository ships, which has no element records, so it
lists and exports none; CI runs all three on it and on `Revit_IFC5_Einhoven.rvt`
(Revit 2023, from magnetar-io/revit-test-datasets, MIT), where they do:

- [`basics_inspect.rs`](basics_inspect.rs): what a file is (release, build,
  title, last save, schema).
- [`basics_elements.rs`](basics_elements.rs): its building elements, by IFC
  type, with ElementIds.
- [`basics_export.rs`](basics_export.rs): IFC4, glTF, a CSV schedule and the
  export diagnostics.

```sh
cargo run --example basics_inspect -- path/to/model.rvt
```

**Research.** Everything else here supports a format finding: the
`probe_*.rs` programs each prove one fact against a corpus (their module doc
says which, and how to check it) and back a report under
`reports/element-framing/`; the others are earlier research tools. They use
`rvt::research`, the unstable readers behind the `unstable-research`
feature, which examples get automatically. Expect their APIs and output to
change.
