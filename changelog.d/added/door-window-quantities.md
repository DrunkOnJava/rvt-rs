- **Doors and windows carry their base quantities (#1016).**
  `Qto_DoorBaseQuantities` and `Qto_WindowBaseQuantities` hold `Width`,
  `Height` and `Area`, the size revit-ifc reads from the door's or window's
  Width and Height parameters. They are written where the symbol that draws
  the element and its type agree on that size: on the BIM4LCA office as
  built that is 196 of 269 doors and all 269 windows, each value as Revit
  writes it, and none different.
