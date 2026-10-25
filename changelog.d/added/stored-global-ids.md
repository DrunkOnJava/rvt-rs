- **The project, site, building and storeys take the GlobalIds the file
  stores (#878, RE-220).** rvt-rs generated the project's, site's and
  building's GlobalIds on every file. Revit's export takes them from the
  project information's stored IfcGUID values where it has them, and so
  does rvt-rs now: on BIMcollab's Revit 2015 example and both BIM4LCA office
  files all three are Revit's, where none were. On the 2015 example the five
  storeys take their stored values too, Revit's, where none matched. A file
  that stores none keeps generated ones.
