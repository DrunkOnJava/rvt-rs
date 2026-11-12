- **A decoded graph's fields take a sixth of the memory, and a default
  export reads 6% to 25% faster (#1694).** The decoder now builds every
  object's fields as `rvt::research::graph_value::Fv`, which keeps an
  object's values in one slice and its field names once for all the objects
  of a class, where each field was a map entry with a key string of its own
  (a node costs 32 to 34 bytes, not 172 to 215). Nothing an export holds
  changes: 145 of 145 exports are byte-identical to `main`'s, with the same
  diagnostics and censuses (Measure run 43144800763). On one runner, three
  rounds each (run 43144804167), a default export takes 0.75 to 0.94 of the
  time (the 29 files together 0.83) and peaks at 0.50 to 1.00 of the memory:
  Snowdon Towers Architectural 1,406 to 1,114 MiB, the BIM4LCA office as
  built 1,219 to 945 and at permit 1,363 to 1,034, Core Interior 434 to 327,
  `rac_basic` 2023 348 to 250 and `rst_basic` 2023 320 to 159. The four
  largest files fall 21% to 24%, not the 30% the rows asked for; what is
  left of their peak is the open work of #1694.
