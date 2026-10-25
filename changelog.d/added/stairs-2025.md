- **A Revit 2025 stair run is drawn as its treads and risers (#357).**
  Revit 2024's byte anchors for a run's riser height, tread depth and ends
  are not found on Revit 2025, so every 2025 flight was its record box. A
  run's sketch, riser height and ends are now read by member name from its
  native records, with `StairsRun.m_endWithRiser` as puzzbobb found it, and
  its run type from its own element record, as on 2024. On puzzbobb's R2025
  stair models 13 and 14 each flight's volume is Revit's, 4.49 and
  4.43 ft3 where the box was 317, with every vertex of Revit's body on
  rvt-rs's, and the flight carries its type's name. The riserless,
  monolithic and spiral runs keep their box.
