- **The beam-end probe cuts a beam at the section the export draws for each
  column its list names, and on the BIM4LCA models that is the column's box
  (RE-455, #1839).** A fourth kind beside `column`, `wall` and `beam` holds
  the beam at the column's own section (a rectangle, a circle or a closed
  outline, turned by the element's rotation and placed at its location),
  along the beam's centre line, where the column kind uses the record box. On
  the ten BIM4LCA models it reads the section of 1,307 of 1,441 columns and
  gives the column kind's bounds on every beam it cuts: Revit's length within
  1 mm on the one beam Revit shortens and it reaches, and a cut where Revit
  leaves the beam on 2 beams, as the box does. The columns there are boxes, so
  that is a check that the kind does no harm, not a test of it; the test is a
  model whose boxes run past Revit's beams, such as the reporter's. Nothing
  ships.
