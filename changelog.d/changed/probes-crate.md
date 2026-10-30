- **The format probes are a crate of their own, `rvt-probes`.** The 244
  `probe_*.rs` programs behind the RE reports moved from `examples/` to
  `probes/examples/`. They are no longer compiled by every `rvt` build,
  clippy run and MSRV check; CI's clippy job checks them once. Run one with
  `cargo run -p rvt-probes --example probe_re70_wall_join_lists -- <file>`,
  and Measure's `probe` input takes the same names as before. The user
  examples (`basics_*`) and the earlier research tools stay in `examples/`.
