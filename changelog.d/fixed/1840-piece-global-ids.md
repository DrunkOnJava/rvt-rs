- **An element's further pieces keep their GlobalIds from one save to the
  next (#1840).** A layered floor's parts, a slab's further pieces and a
  curtain wall's parts share their element's Tag, and Revit gives only the
  first a GlobalId. The others were derived from the document's
  `Unique Document GUID`, which can change between two saves of one project,
  so on a Revit 2027 structural model every part's GlobalId moved on every
  save (#1829). They are now derived from their element's Revit GlobalId,
  which the file keeps. Each such piece's GlobalId changes once, in this
  release; the element's own does not.
