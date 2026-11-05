- **The 25 Areas of Autodesk's `rac_advanced` 2021 save are written, from
  Revit's plan topology alone, and say `area_plan_topology_unchecked`
  (#1434).** The 2021 save holds both states of the model: 16 area boundary
  lines have two copies in two partitions that differ in box (0.0246 ft),
  each naming the same Level and scheme at the same height. A group of
  lines with such a conflict is no longer unreadable: the lines'
  arrangement is not read (which copy is current is not known from the
  partitions), and the Area is drawn from Revit's plan topology, which reads
  the current copy of each line and is checked, as everywhere, against the
  area Revit saves for the loop and the Area's point; an Area may name a line
  whose copies conflict in place. A copy that names another Level or scheme
  or lies at another height, a design option, a stray height, a missing line
  and a line off the Level still make the group unreadable. The 2021 save's
  25 regions equal the 2022 save's in vertex count, holes, arcs and area
  (25 of 25), and differ from the 2020 save's on exactly the 7 Areas where
  the 2020 and 2022 saves differ (0.23 to 2.77 ft2), so the current copy is
  the later state; the only Revit export of the project shows the earlier one,
  which equals 18 of the 25 outlines and does not show the other 7, so the
  Areas say `area_plan_topology_unchecked` and not `area_plan_topology`
  (Griffin's decision for this save; the 2022 to 2024 saves keep
  `area_plan_topology` by his release rule). A same-project gate holds the 2021 save to
  the 2022 save, Area by Area, in GlobalId, number, name, storey, outline
  source, body area, top surface, height and lowest vertex. Of the models of
  a Measure run of 117 exports, one moves, this one; the held-out models are
  byte for byte as before, so none of them exercises the change, and no
  public file has a design-option area line, so that refusal is kept by
  construction and shown only by the helpers' tests. Report:
  `reports/element-framing/RE-333-area-plan-topology.md`, section 10.
