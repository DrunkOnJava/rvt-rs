- **A standalone family of Revit 2014 is read from its own records, as
  2015 to 2027 are (RE-489, #1189).** `native_form_geometry::open` took
  releases 2015 to 2027 only; it now takes 2014 to 2027 (`FAMILY_RELEASES`).
  On the three Revit 2014 families of DynamoRevit's test suite every plain
  group frames, and the 12 extrusions they hold all rebuild; the 8 with a
  complete saved mesh match it under the RE-327 probe's tests (volume,
  bounding box, vertices and surface samples), the other 4 holding a void
  or no mesh. Revit 2013 stays refused: its schema does not parse within the
  field budget. Not measured: any Revit 2014 family with a revolution, a
  sweep or a blend, and any 2014 project (the project readers keep
  `NATIVE_RELEASES`).
