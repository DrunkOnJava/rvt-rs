- **The round faces of a saved mesh that meet along an edge hold fewer
  triangles (#2111, RE-496).** #2107 closed the cracks between two cylinder
  faces by giving their shared edge points four times finer than the faces'
  own 0.001 ft chord; the edge now takes them at half that chord. The cracks
  stay closed on the 77 files read, and 48 to 100 per cent of the triangles
  #2107 added go, by file: on Autodesk's `rme_basic` the 2024 export goes from
  112.7 to 108.0 MiB (106.0 before #2107). On 147 groups of faces the farthest
  a round face stands off its true cylinder rises by at most 0.0003 ft, still
  within 0.001 ft.
