- **The saved-graphics pass builds only the graph fields meshing reads, which
  takes 3% to 7% off a default export's peak memory (#1694).** The pass decoded
  every field of every graph object of a graphics record, and the dhat run put
  77% to 87% of its peak in those decoded nodes. It now keeps the 66 fields its
  readers use (`SAVED_GRAPHICS_FIELDS`, through `Options::graph_fields`), reading
  and checking the rest as before. On Autodesk's samples of Revit 2023 and 2024,
  the BIM4LCA files and the reference models the export's peak reads 0.93 to 1.00
  times the previous build's on one runner (Snowdon Towers Architectural 1,395 to
  1,351 MiB, `rac_basic` 2023 361 to 347, the offices 1,301 to 1,250 and 1,435 to
  1,376), the pass's own peak 6% to 10% lower, and 125 of 125 exports are
  byte-identical. Export time is within 3% on 28 of 29 files (`rac_advanced` 2023
  reads 1.05). The decoded nodes are still most of the pass's peak: a pointer
  value is a three-entry map, about 730 bytes (#1694).
