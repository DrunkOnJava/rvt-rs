- **Openings, property sets, quantities and the relationships between
  entities keep their GlobalIds from one save to the next (#1854).** Each
  GlobalId the IFC writer derives was seeded from the document's
  `Unique Document GUID`, which changes on every save of every Autodesk
  sample measured, so between two saves of one project every opening,
  property set, quantity set, material association, containment, void,
  fill, aggregation, type relationship and port took a new GlobalId while
  the elements kept theirs. Each is now seeded from the GlobalId of the
  entity it belongs to: an opening from its host's, a property set from its
  element's, a relationship from the entity it starts from. It holds wherever
  that entity's does, and two documents share it only where they share that
  entity's. No project-level identity could take the document's place: the
  first episode and the ProjectInfo GlobalId are the template's, one value
  on 64 of the 102 models measured. What belongs to no entity still moves
  between saves: the project, site, building and storeys where Revit gives
  them no GlobalId, classifications, materials, MEP systems and types Revit
  gives no GlobalId.
  **Upgrade note for clash and issue-tracking (BCF) users:** every such
  derived GlobalId changes once, in this release, as the element pieces of
  #1840 do; from then on they hold.
