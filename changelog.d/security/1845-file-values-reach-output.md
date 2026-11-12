- **A crafted file can no longer write its own text into an export's STEP,
  or a NaN or infinity into the IFC, glTF or plan SVG (#1845).** The comment
  after an export's header held the file's PartAtom id unchanged, so a `*/`
  in it ended the comment and the rest became part of the file; it is now
  kept inside the comment. A stored number that is not finite, or larger
  than 1e30 (twenty orders past any model's coordinates), is now read as
  absent where the walker, the native graphs and the element records decode
  it, so no sum or difference made from it becomes infinite. A sum of stair
  counts that would overflow leaves the property out. No real file of the
  measured corpus changes.
