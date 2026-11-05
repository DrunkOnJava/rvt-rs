- **A wall that embeds a curtain wall is drawn from the mesh Revit saved for
  it where Revit raised its base (#1385, RE-338).** RE-335 found that Revit
  draws such a wall less the curtain wall's prism and less whatever cuts its
  end stubs, and that no record read says which walls keep the stubs. The
  wall's own saved graphics do: on the BIM4LCA office, as built and at
  building permit, the lowest vertex of the saved mesh of each of the 113
  walls that embed a curtain wall is Revit's lowest point to within 0.001 ft.
  Fourteen stand 8.2021 or 9.514 ft above their base, the curtain wall's
  height, and 12 of those 14 meshes are Revit's body to 0.02 ft³ (the other
  two, one wall at each date, are 1.0% under it). `rvt-ifc`, `rvt-gltf`, the
  Python package and the viewer now draw a wall that embeds a curtain wall
  from its saved mesh when the mesh starts at least 0.5 ft above the wall's
  base and decodes whole, on Revit 2023 to 2027, and leave the wall as it was
  otherwise: a wall that keeps its stubs (2103576 as built, 2177158) is
  unchanged, and so is a wall with an opening hosted in it, a Tag shared with
  another entity or a base cut at a floor (#1384). The base of all 14 walls
  Revit raises, 6 as built and 8 at permit, is now Revit's (Measure run
  41945943795): walls whose base and top are Revit's rise from 598 to 604 of
  607 as built and from 577 to 585 of 589 at permit, and no other wall's row
  moves. `--no-saved-meshes` keeps the walls' records. A file with no curtain
  wall reads no wall record for this, and the exports of Core Interior and
  RE1 Architecture are byte-identical.
