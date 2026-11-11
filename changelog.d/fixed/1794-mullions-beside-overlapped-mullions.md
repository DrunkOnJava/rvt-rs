- **A curtain mullion beside an overlapped mullion is read like any other, but for two kinds (#1794, RE-441).**
  The rule that tells which end faces of a curtain mullion Revit's solid keeps
  left a mullion out whenever one of its ends lay within 6 cm of a mullion that
  was left out or overlapped (another lying over it). Applied to the 314
  mullions of the BIM4LCA concrete office as built and the 280 of the timber
  office whose only unread neighbours are overlapped ones, it gives Revit's
  count on 312 and 272, the others having no value or being of two kinds that
  stay unread: a mullion with a mullion across centred on an end, and an end
  with no partner where an overlapped mullion's axis lies on the end's plane (a
  vertical standing on the bar's face). The concrete office as built and at
  permit gain 353 `NetSurfaceArea` values and the timber office 320 and 316,
  every one equal to Revit's, none different; 13 exports with no Revit export
  to meet gain 562 more (`rac_advanced`, `rme_advanced`, `rme_basic` and
  Snowdon Towers Architectural). The neighbours of a mullion left out stay
  unread.
