- **The saved-mesh pass cuts its batches by the body bytes of the symbols they
  read, so its peak memory no longer depends on where the largest symbols sort
  (#1694).** A batch was sized from the graph objects per symbol of the one
  before it, and the symbols sort by id with the largest last, so on Autodesk's
  samples of Revit 2023 and 2024 the peak batch held 3 to 7 times its 20,000
  graph objects (`rac_basic` 99,580, `rac_advanced` 123,965, Snowdon Towers
  Structural 135,746) and set the export's peak memory. One walk now reads the
  body bytes of every symbol an owner names without building a graph, and the
  batches are cut at 2,000,000 bytes. No export changes. `--no-saved-meshes`
  still runs no pass.
