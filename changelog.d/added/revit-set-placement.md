- **`rvt-ifc --revit-set-placement`.** Writes properties where Revit's own
  exporter writes them and IFC4's definitions do not: a text parameter named
  `<set>.<property>` for a property the set's IFC4 definition lacks (a room's
  `Pset_SpaceCommon.FloorCovering`), and the `Reference` of a set Revit
  matches to the element by the start of its entity's name
  (`Pset_SpaceHeaterTypeCommon` on a space). On the BIM4LCA office 961 more of
  Revit's values, none different. The default output still follows
  buildingSMART's IFC4 definitions (RE-235).
