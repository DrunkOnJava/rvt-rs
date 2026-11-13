- **An element's further pieces keep their GlobalIds from one save to the
  next (#1840).** A layered floor's parts, a slab's further pieces and a
  curtain wall's parts share their element's Tag, and Revit gives only the
  first a GlobalId. The others were derived from the document's
  `Unique Document GUID`, which can change between two saves of one project,
  so on a Revit 2027 structural model every part's GlobalId moved on every
  save (#1829). They are now derived from their element's Revit GlobalId,
  which the file keeps. The element's own GlobalId does not change.
  **Upgrade note for clash and issue-tracking (BCF) users:** each such
  further piece's GlobalId changes once, in this release. A tool that
  follows elements by GlobalId sees those pieces removed and added again
  between an export made with the previous release and one made with this
  one; from then on they hold. With each piece go the GlobalIds of its
  material association and its property sets. On Autodesk's MEP samples
  that is 25 pieces a file, on Snowdon Towers Architectural 25, and on the
  BIM4LCA concrete housing as built 3; a model with no layered floor, slab
  or curtain wall split into parts has none.
