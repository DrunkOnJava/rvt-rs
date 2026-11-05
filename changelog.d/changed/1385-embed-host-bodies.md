- **A wall that embeds a curtain wall is drawn from the mesh Revit saved for
  it where Revit raised its base (#1385, RE-340).** RE-335 found that Revit
  draws such a wall less the curtain wall's prism and less whatever cuts its
  end stubs, and that no record read says which walls keep the stubs. The
  wall's own saved graphics do: on the BIM4LCA office, as built and at
  building permit, the lowest vertex of the saved mesh of each of the 113
  walls that embed a curtain wall is Revit's lowest point to within 0.001 ft.
  Fourteen stand 8.2021 or 9.514 ft above their base, the curtain wall's
  height, and 12 of those 14 meshes are Revit's body to 0.02 ft³ (the other
  two, one wall at each date, are 1.0% under Revit's IFC tessellation of the
  wall and equal to Revit's own `NetVolume` and `GrossSideArea` of it).
  `rvt-ifc`, `rvt-gltf`, the Python package and the viewer now draw a wall
  that embeds a curtain wall from its saved mesh when the mesh starts at least
  0.5 ft above the wall's base and decodes whole, on Revit 2024 to 2027, and
  leave the wall as it was otherwise: a wall that keeps its stubs (2103576 as
  built, 2177158) is unchanged, and so is a wall with a door, window or other
  opening hosted in it (the void its own curtain wall cuts does not count), a
  Tag shared with another entity or a base cut at a floor (#1384).
  - The base of all 14 walls Revit raises, 6 as built and 8 at permit, is now
    Revit's with the curtain walls' openings (#1165) written (Measure run
    41973316830 at 3ad0762f): walls whose base and top are Revit's
    rise from 598 to 604 of 607 as built and from 577 to 585 of 589 at permit,
    and no other wall's row moves.
  - The 0.5 ft threshold was fitted and checked on those same two files (113
    host states, 10 distinct raised walls); the held-out models and
    Autodesk's samples gave no host to score.
  - The glTF draws the six hosts as built, which have two or more layers, in
    one material, not in their layers; the hosts of one layer keep their
    layer's colour (#1405). The IFC keeps every host's layer set.
  - `--no-saved-meshes` keeps the walls' records. A file with no curtain wall
    reads no wall record for this, and the exports of Core Interior and RE1
    Architecture are byte-identical.
