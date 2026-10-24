- **The viewer's whole journey is tested on the six redistributable
  reference models.** A browser test opens Einhoven, Core Interior and the
  four RE1 models, inspects an element, downloads the schedule, the IFC, the
  GLB, the plan SVG and the diagnostics, and checks them against each other:
  the inspected element is in the schedule and in the IFC under its
  ElementId, the schedule lists exactly the elements the diagnostics count by
  type, and on Einhoven and Core Interior the export has the counts
  `tests/project_count_fixtures.rs` pins for the CLI's export of the same
  file. Deploy viewer fetches the RE1 models for it. The Einhoven demo card
  gave 24 walls and 42 materials; it now gives the 26 walls, 9 curtain
  panels, 2 windows and 36 materials the viewer shows.
