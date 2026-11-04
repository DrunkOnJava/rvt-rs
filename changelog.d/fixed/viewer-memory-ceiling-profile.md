- **A sketch's value-source notes cost nothing without
  `--element-provenance`, and the viewer's stop-in-the-silent-stretch test
  measures its cap (#1374).** The two notes `solve_sketch_plan_profiles`
  kept for the value sources, the set of elements whose outline came from
  their lines' own ends and the set of lines whose copies disagreed, were
  built on every open. Both are now built only while the handle records
  value sources. The viewer test that stops Core Interior's open between
  the last partition stream and the saved graphics no longer hard-codes a
  310 MiB cap: where the wasm memory grows between two reports moved with
  #1370 (16.3 MiB of it now falls inside the saved meshes, with the same
  325.4 MiB peak; the viewer measurement of Deploy viewer run 41870855196
  at f1f80d9e against run 41885589164 at 0ca59127), so the test opens the
  model once at the real ceiling and puts its cap between the memory at the
  first report that reads "Building the model" and the memory at the first
  report after those.
