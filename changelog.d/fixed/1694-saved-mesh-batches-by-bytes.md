- **The saved-mesh pass cuts its batches by the body bytes of the symbols they
  read, which takes the peak memory of the default export down on the files
  where the old batches overshot (#1694).** A batch was sized from the graph
  objects per symbol of the one before it, and the symbols sort by id with the
  largest last, so the batch that met them held several times its budget. One
  walk now reads the body bytes of every symbol an owner names without building
  a graph, and the batches are cut at 2,000,000 bytes (about the 20,000 graph
  objects of before). The export's peak memory falls from 875 to 889 MiB to 360
  to 392 on Autodesk's `rac_basic` of 2023 to 2027, from 934 to 1,001 to 724 to
  792 on `rme_advanced` and `rme_basic`, from 1,790 to 1,396 on Snowdon Towers
  Architectural and from 1,439 and 1,550 to 1,301 and 1,436 on the BIM4LCA
  office as built and at permit. No export changes (126 of 126 byte-identical
  to `main`'s) and no timed file takes more than 2% longer (Measure run
  42977559276). A file whose symbols come to one or two batches under the
  budget decodes more of them at once than the old first batch of 8 symbols
  did, and peaks a little higher: Snowdon Towers HVAC 366 to 410 MiB, RE1
  Architecture 160 to 186. `--no-saved-meshes` still runs no pass.
