- **The export's peak memory is back to what it was before the walls that
  embed a curtain wall were drawn from their saved meshes (#1429).** #1401
  (RE-340) reads the graphics of those walls in the saved-graphics pass, and
  the pass kept every owner's decoded graph until it returned, so the 42 hosts
  of the BIM4LCA office as built (41 at permit; about 1.6 MiB each, drawn from
  their mesh or not) stayed under every later batch of door and window
  symbols: 66 MiB of live heap, and 76 MiB of resident memory at the pass's
  peak (917 to 993 MiB as built, 979 to 1,057 MiB at building permit). The
  pass now frees an owner's graph with its batch, unless another owner names
  it (a shared symbol that names one already freed has it read again), and
  meshes the owners that name no symbol of their own in a batch of their own
  before it reads any symbol. The batches of symbols are the ones they were,
  except that where an owner names only other owners the later ones can differ
  in size; the elements are the ones they were. `rvt-ifc`'s peak on the office
  is 914 MiB as built and 971 MiB at permit (the parent of #1401: 916 and 979
  MiB), and Snowdon Towers architectural's 1,062 MiB, against 1,108 MiB on
  `main` before this change. A file with no such wall is not changed: Core
  Interior, the RE1 models and
  Einhoven are at 1.00 of the parent's peak. The time does not move: 0.98 to
  1.02 of `main`'s across the 68 files timed. The IFC does not move: the 111
  files an unfiltered Measure exports (34 models, 19 held-out and spent
  models, 58 Autodesk samples) are byte-identical to `main`'s.
  `probes/examples/probe_1429_export_peak.rs` profiles the heap at the
  export's peak by call site.
