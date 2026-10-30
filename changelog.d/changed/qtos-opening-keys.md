- **The quantity census keys an opening by whether something fills it, as
  the property census does (#1161).** `tools/re/qtos_vs_ifc.py` keyed every
  opening by its `Tag` alone, so the further cuts Revit's export writes for
  a door or a curtain wall were scored against rvt-rs's door opening. Each
  opening now carries the classes of what fills it, so a door's opening is
  still known for RE-235 §5's swapped `Width` and `Height`, and the
  quantities on openings rvt-rs does not write are counted apart (#1165).
  On the BIM4LCA office as built that line lists 527 quantities (some of
  them were hidden before, a later opening of the same `Tag` overwriting an
  earlier one); missing goes from 11,700 to 11,367, the swapped door-opening
  `Width` and `Height` set aside from 191 to 187 each (the 4 others are a
  door's further cuts), and equal and different are unchanged on every
  model. No export changes.
