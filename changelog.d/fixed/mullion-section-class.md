- **Curtain mullions' `NetVolume` in any language (#1016, RE-246).** Which
  mullions have a rectangular section is now read from the class of their
  type's section object rather than from their family's English name, so a
  model made in another language gets the same `NetVolume`: Autodesk's MEP
  sample projects, whose rectangular mullion family is named in German, now
  carry it. Every export of the reference models is unchanged.
