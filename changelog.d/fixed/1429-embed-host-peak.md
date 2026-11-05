- **The export's peak memory is back to what it was before the walls that
  embed a curtain wall were drawn from their saved meshes (#1429).** #1401
  (RE-340) reads the graphics of those walls in the saved-graphics pass, and
  the pass kept every owner's decoded graph until it returned, so the 42 hosts
  of the BIM4LCA office as built (41 at permit; about 1.6 MiB each, drawn from
  their mesh or not) stayed under every later batch of door and window
  symbols: 66 MiB of live
  heap, and 76 MiB of resident memory at the pass's peak (917 to 993 MiB as
  built, 979 to 1,057 MiB at building permit). The pass now frees an owner's
  graph with its batch, unless another owner names it, and meshes the owners
  that name no symbol before it reads any symbol; the symbol batches and the
  elements are the ones they were. `rvt-ifc`'s peak on the office is 914 MiB
  as built and 971 MiB at permit (the parent of #1401: 916 and 979 MiB), and
  Snowdon Towers architectural's 1,062 MiB, against 1,108 MiB on `main`
  before this change. `probes/examples/probe_1429_export_peak.rs` profiles
  the heap at the export's peak by call site.
