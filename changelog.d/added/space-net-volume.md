- **Spaces' `NetVolume` (#1016, RE-296).** Revit's export writes a space's
  `Qto_SpaceBaseQuantities.NetVolume` as the volume of the room's closed
  shell. Where the room's stored solid is a prism of its outline, every
  vertical face running from the floor of its record box to the ceiling and
  no face slanted, that volume is the outline's area, its holes taken off,
  times the box's height, and rvt-rs writes it. On the BIM4LCA office that
  is 167 of the 278 values Revit's export writes as built and 178 at
  building permit, and both rooms of each of puzzbobb's six R2025 models
  with rooms, none different and none that Revit's export leaves out. The
  volume of a room whose ceiling steps (54 as built), of the mapped
  accessibility volumes and of the rooms whose outline does not close is not
  written; nor is `GrossVolume`, which Revit's export writes on 88 of the
  167 prisms only and which nothing in the file tells from the 79 it leaves
  out.
