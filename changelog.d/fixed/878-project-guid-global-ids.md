- **A Revit 2017 to 2027 file that stores none gets the project, building
  and site GlobalIds Revit's export gives them, and a Revit 2017 to 2022 file
  the storeys', from the project's own GUID; a history with an entry closed by
  another byte than `0x28` is read (#878, #1929, RE-459).** Revit's exporter
  makes the project's, the building's and the site's GlobalIds from one GUID
  with a fake ElementId (-16, -15, -14) XORed into its last 32 bits, and a
  storey's from it with the Level's ElementId before Revit 2023. RE-220 could
  not find that GUID, and files that stored none kept generated ids. It is in
  the file: the header of `Global/History` holds it four times, at `0x16`,
  `0x26`, `0x36` and `0x56`, and it is not an entry of the history. On the 67
  models of Measure that have a Revit export, the four copies are equal on all
  of them (268 of 268), and the project, building and one site are Revit's on
  63 of the 64 files of 2017 to 2027 whose export is of their own project; the
  storeys are Revit's on 69 of 69 on 11 files of 2017 to 2022, and on none of
  229 from 2023, where each Level's own episode gives them as before (Core
  Interior, 15 of 15). Where Revit's export is of another project than the
  file (two `rac_advanced` saves of 2015 and 2016, whose header holds another
  GUID than the saves that export came from) nothing is measured, and BIMcollab's
  Revit 2015 example stores its ids as strings, which win as they did. One
  held-out Revit 2018 project follows no rule found: neither the header GUID,
  nor a stored string, nor an episode gives its ids, and it is not claimed.
  The rule is read only on Revit 2017 and later, because no file of 2015 or
  2016 can test it. Alongside it, `EpisodeHistory::parse` read none of the
  1,314 elements of the Revit 2018 Vitoria "Before" model because four of its
  1,127 entries are closed by `0x1e`, not `0x28`: it reads them now, and the
  elements of that model Revit's export holds are Revit's on RESULT_PENDING.
  Every project, building, site and storey GlobalId of a file the rule applies
  to changes once, with the next GlobalId change; RESULT_PENDING of the 146 exports of an
  unfiltered Measure get Revit's project, building and site where they had
  none (`reports/element-framing/RE-459-episode-global-ids-2015-2023.md`).
