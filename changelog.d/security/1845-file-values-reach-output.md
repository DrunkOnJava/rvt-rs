- **A crafted file can no longer write its own text into an export's STEP,
  fail a whole IFC export over one number, or write NaN or infinity into
  the glTF or plan SVG (#1845).** The comment after an export's header held
  the file's PartAtom id unchanged, so a `*/` in it ended the comment and
  the rest became part of the file; it is now kept inside the comment. A
  stored number that is not finite, or larger than 1e30 (twenty orders past
  any model's coordinates), reached the export, where the IFC writer
  refused the whole file. It is now read as absent wherever a stored
  number is decoded (`le_bytes`, which every partition scanner reads
  through, the walker and the native graphs), so no sum or difference made
  from it becomes infinite. A sum of stair counts that would overflow
  leaves the property out. The glTF and the plan SVG, which had no check of
  their own, now draw an element whose transform or outline is not finite
  as no geometry. No real file of the measured corpus changes.
